//! Telegram outbox and bot-isolated `file_id` cache persistence (plan
//! TG-04 / TG-05), implemented on top of the additive `0007` migration.
//!
//! Timestamps are stored as text and compared lexicographically, so callers
//! must pass one consistently formatted, monotonically ordered UTC form
//! (the convention the existing job and send-state rows already use).

use rusqlite::{OptionalExtension, Row, params};

use crate::{Database, StorageError};

use xarchive_telegram::{
    CachedFileId, FileCacheKey, FileIdCacheStore, MediaKind, NewOutboxEntry, OutboxEntry,
    OutboxState, SendStateError, TelegramOutboxStore, UNKNOWN_REASON_LEASE_EXPIRED,
};

const OUTBOX_COLUMNS: &str = "id, bot_identity, chat_id, message_thread_id, idempotency_key, \
     request_fingerprint, message_kind, config_version, plan_version, plan_order, tweet_id, \
     media_reference, content_sha256, state, attempt_count, claim_token, claim_expires_at, \
     request_started, next_retry_at, telegram_message_id, results_json, last_error_code, \
     last_error_message, unknown_reason, created_at, updated_at";

fn outbox_error(error: rusqlite::Error) -> SendStateError {
    SendStateError::Store(error.to_string())
}

fn media_kind_text(kind: MediaKind) -> Result<String, SendStateError> {
    serde_json::to_value(kind)
        .ok()
        .and_then(|value| value.as_str().map(str::to_owned))
        .ok_or_else(|| SendStateError::Store("invalid media kind".to_owned()))
}

fn outbox_from_row(row: &Row<'_>) -> Result<OutboxEntry, SendStateError> {
    // Column order is fixed by OUTBOX_COLUMNS. A mapping or decode error is a
    // store fault, never a silently defaulted row.
    macro_rules! field {
        ($index:expr) => {
            row.get($index)
                .map_err(|error| SendStateError::Store(error.to_string()))?
        };
    }
    let state_value: String = field!(13);
    let state = OutboxState::parse(&state_value)
        .ok_or_else(|| SendStateError::Store(format!("invalid outbox state: {state_value}")))?;
    let request_started: i64 = field!(17);
    let attempt_count: i64 = field!(14);
    Ok(OutboxEntry {
        id: field!(0),
        bot_identity: field!(1),
        chat_id: field!(2),
        message_thread_id: field!(3),
        idempotency_key: field!(4),
        request_fingerprint: field!(5),
        message_kind: field!(6),
        config_version: field!(7),
        plan_version: field!(8),
        plan_order: field!(9),
        tweet_id: field!(10),
        media_reference: field!(11),
        content_sha256: field!(12),
        state,
        attempt_count: u32::try_from(attempt_count).unwrap_or(0),
        claim_token: field!(15),
        claim_expires_at: field!(16),
        request_started: request_started != 0,
        next_retry_at: field!(18),
        telegram_message_id: field!(19),
        results_json: field!(20),
        last_error_code: field!(21),
        last_error_message: field!(22),
        unknown_reason: field!(23),
        created_at: field!(24),
        updated_at: field!(25),
    })
}

impl Database {
    /// The row a claim token currently owns, if any.
    fn claimed_entry(&self, claim_token: &str) -> Result<Option<OutboxEntry>, SendStateError> {
        let sql = format!("SELECT {OUTBOX_COLUMNS} FROM telegram_outbox WHERE claim_token = ?1");
        let mut statement = self.connection.prepare(&sql).map_err(outbox_error)?;
        let mut rows = statement
            .query(params![claim_token])
            .map_err(outbox_error)?;
        let Some(row) = rows.next().map_err(outbox_error)? else {
            return Ok(None);
        };
        outbox_from_row(row).map(Some)
    }

    /// Applies a transition to exactly the row the caller still owns. The
    /// statement binds `?1` to `claim_token` itself; every later write must
    /// therefore guard on it, so an expired claim can never overwrite the
    /// facts recorded by the worker that reclaimed the entry.
    fn update_claimed(
        &self,
        claim_token: &str,
        statement_sql: &str,
        trailing: &[&dyn rusqlite::ToSql],
    ) -> Result<(), SendStateError> {
        let mut parameters: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(trailing.len() + 1);
        parameters.push(&claim_token);
        parameters.extend_from_slice(trailing);
        let changed = self
            .connection
            .execute(statement_sql, rusqlite::params_from_iter(parameters))
            .map_err(outbox_error)?;
        if changed == 0 {
            return Err(SendStateError::StaleClaim);
        }
        Ok(())
    }

    /// Outbox rows of one bot for a Tweet, in plan order: the data source for
    /// the task send-state projection (plan TG-06). Rows of other bots are
    /// never mixed in.
    pub fn list_outbox_for_tweet(
        &self,
        bot_identity: &str,
        tweet_id: i64,
    ) -> Result<Vec<OutboxEntry>, StorageError> {
        let sql = format!(
            "SELECT {OUTBOX_COLUMNS} FROM telegram_outbox \
             WHERE bot_identity = ?1 AND tweet_id = ?2 \
             ORDER BY plan_order ASC, id ASC"
        );
        let mut statement = self.connection.prepare(&sql)?;
        let mut rows = statement.query(params![bot_identity, tweet_id])?;
        let mut entries = Vec::new();
        while let Some(row) = rows.next()? {
            entries.push(
                outbox_from_row(row)
                    .map_err(|error| StorageError::InvalidState(error.to_string()))?,
            );
        }
        Ok(entries)
    }
}

impl TelegramOutboxStore for Database {
    fn enqueue_outbox(&self, entry: NewOutboxEntry) -> Result<i64, SendStateError> {
        self.connection
            .execute(
                "INSERT INTO telegram_outbox \
                     (bot_identity, chat_id, message_thread_id, idempotency_key, \
                      request_fingerprint, message_kind, config_version, plan_version, \
                      plan_order, tweet_id, media_reference, content_sha256, state, \
                      attempt_count, request_started, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'QUEUED', 0, 0, ?13, ?13) \
                 ON CONFLICT(chat_id, idempotency_key) DO NOTHING",
                params![
                    entry.bot_identity,
                    entry.chat_id,
                    entry.message_thread_id,
                    entry.idempotency_key,
                    entry.request_fingerprint,
                    entry.message_kind,
                    entry.config_version,
                    entry.plan_version,
                    entry.plan_order,
                    entry.tweet_id,
                    entry.media_reference,
                    entry.content_sha256,
                    entry.created_at,
                ],
            )
            .map_err(outbox_error)?;
        // Idempotent: an existing row keeps its id, state and history.
        self.connection
            .query_row(
                "SELECT id FROM telegram_outbox WHERE chat_id = ?1 AND idempotency_key = ?2",
                params![entry.chat_id, entry.idempotency_key],
                |row| row.get(0),
            )
            .map_err(outbox_error)
    }

    fn claim_due_outbox(
        &self,
        bot_identity: &str,
        claim_token: &str,
        now: &str,
        lease_until: &str,
    ) -> Result<Option<OutboxEntry>, SendStateError> {
        // One atomic UPDATE: the sub-select picks the oldest due entry and
        // the state guard keeps it to a single winner, so two workers racing
        // the same row can never both claim it.
        let changed = self
            .connection
            .execute(
                "UPDATE telegram_outbox \
                 SET state = 'IN_FLIGHT', claim_token = ?2, claim_expires_at = ?4, \
                     request_started = 0, attempt_count = attempt_count + 1, updated_at = ?3 \
                 WHERE id = ( \
                     SELECT id FROM telegram_outbox \
                     WHERE bot_identity = ?1 \
                       AND state IN ('QUEUED', 'RETRY_WAIT') \
                       AND (next_retry_at IS NULL OR next_retry_at <= ?3) \
                     ORDER BY plan_order ASC, id ASC \
                     LIMIT 1 \
                 ) \
                 AND state IN ('QUEUED', 'RETRY_WAIT')",
                params![bot_identity, claim_token, now, lease_until],
            )
            .map_err(outbox_error)?;
        if changed == 0 {
            return Ok(None);
        }
        self.claimed_entry(claim_token)
    }

    fn claim_outbox(
        &self,
        bot_identity: &str,
        idempotency_key: &str,
        claim_token: &str,
        now: &str,
        lease_until: &str,
    ) -> Result<Option<OutboxEntry>, SendStateError> {
        let changed = self
            .connection
            .execute(
                "UPDATE telegram_outbox \
                 SET state = 'IN_FLIGHT', claim_token = ?3, claim_expires_at = ?5, \
                     request_started = 0, attempt_count = attempt_count + 1, updated_at = ?4 \
                 WHERE id = ( \
                     SELECT id FROM telegram_outbox \
                     WHERE bot_identity = ?1 AND idempotency_key = ?2 \
                       AND state IN ('QUEUED', 'RETRY_WAIT', 'UNKNOWN') \
                     ORDER BY id ASC LIMIT 1 \
                 ) \
                 AND state IN ('QUEUED', 'RETRY_WAIT', 'UNKNOWN')",
                params![bot_identity, idempotency_key, claim_token, now, lease_until],
            )
            .map_err(outbox_error)?;
        if changed == 0 {
            return Ok(None);
        }
        self.claimed_entry(claim_token)
    }

    fn mark_request_started(&self, claim_token: &str, now: &str) -> Result<(), SendStateError> {
        self.update_claimed(
            claim_token,
            "UPDATE telegram_outbox SET request_started = 1, updated_at = ?2 \
             WHERE claim_token = ?1 AND state = 'IN_FLIGHT'",
            &[&now],
        )
    }

    fn record_outbox_sent(
        &self,
        claim_token: &str,
        telegram_message_id: &str,
        results_json: Option<&str>,
        now: &str,
    ) -> Result<(), SendStateError> {
        self.update_claimed(
            claim_token,
            "UPDATE telegram_outbox \
             SET state = 'SENT', telegram_message_id = ?2, results_json = ?3, \
                 claim_token = NULL, claim_expires_at = NULL, next_retry_at = NULL, \
                 last_error_code = NULL, last_error_message = NULL, unknown_reason = NULL, \
                 updated_at = ?4 \
             WHERE claim_token = ?1 AND state = 'IN_FLIGHT'",
            &[&telegram_message_id, &results_json, &now],
        )
    }

    fn record_outbox_retry(
        &self,
        claim_token: &str,
        next_retry_at: &str,
        error_code: Option<i64>,
        error_message: &str,
        now: &str,
    ) -> Result<(), SendStateError> {
        self.update_claimed(
            claim_token,
            "UPDATE telegram_outbox \
             SET state = 'RETRY_WAIT', next_retry_at = ?2, last_error_code = ?3, \
                 last_error_message = ?4, claim_token = NULL, claim_expires_at = NULL, \
                 updated_at = ?5 \
             WHERE claim_token = ?1 AND state = 'IN_FLIGHT'",
            &[&next_retry_at, &error_code, &error_message, &now],
        )
    }

    fn record_outbox_unknown(
        &self,
        claim_token: &str,
        reason: &str,
        error_code: Option<i64>,
        error_message: &str,
        now: &str,
    ) -> Result<(), SendStateError> {
        self.update_claimed(
            claim_token,
            "UPDATE telegram_outbox \
             SET state = 'UNKNOWN', unknown_reason = ?2, last_error_code = ?3, \
                 last_error_message = ?4, claim_token = NULL, claim_expires_at = NULL, \
                 next_retry_at = NULL, updated_at = ?5 \
             WHERE claim_token = ?1 AND state = 'IN_FLIGHT'",
            &[&reason, &error_code, &error_message, &now],
        )
    }

    fn record_outbox_failed(
        &self,
        claim_token: &str,
        error_code: Option<i64>,
        error_message: &str,
        now: &str,
    ) -> Result<(), SendStateError> {
        self.update_claimed(
            claim_token,
            "UPDATE telegram_outbox \
             SET state = 'FAILED_PERMANENT', last_error_code = ?2, last_error_message = ?3, \
                 claim_token = NULL, claim_expires_at = NULL, next_retry_at = NULL, \
                 updated_at = ?4 \
             WHERE claim_token = ?1 AND state = 'IN_FLIGHT'",
            &[&error_code, &error_message, &now],
        )
    }

    fn cancel_outbox(
        &self,
        bot_identity: &str,
        idempotency_key: &str,
        now: &str,
    ) -> Result<bool, SendStateError> {
        // Only a not-yet-sent entry can be cancelled cleanly; a claimed entry
        // that may already be on the wire must go through the UNKNOWN path.
        let changed = self
            .connection
            .execute(
                "UPDATE telegram_outbox \
                 SET state = 'CANCELLED', claim_token = NULL, claim_expires_at = NULL, \
                     next_retry_at = NULL, updated_at = ?3 \
                 WHERE bot_identity = ?1 AND idempotency_key = ?2 \
                   AND state IN ('QUEUED', 'RETRY_WAIT')",
                params![bot_identity, idempotency_key, now],
            )
            .map_err(outbox_error)?;
        Ok(changed > 0)
    }

    fn record_outbox_cancelled(&self, claim_token: &str, now: &str) -> Result<(), SendStateError> {
        self.update_claimed(
            claim_token,
            "UPDATE telegram_outbox \
             SET state = 'CANCELLED', claim_token = NULL, claim_expires_at = NULL, \
                 next_retry_at = NULL, updated_at = ?2 \
             WHERE claim_token = ?1 AND state = 'IN_FLIGHT'",
            &[&now],
        )
    }

    fn recover_outbox_claims(&self, now: &str) -> Result<u64, SendStateError> {
        // Crash recovery (plan TG-04): a claim that expired before the
        // network attempt is retried; one that expired after it started is
        // UNKNOWN and waits for review — never an automatic re-send.
        let resolved = self
            .connection
            .execute(
                &format!(
                    "UPDATE telegram_outbox \
                 SET state = CASE WHEN request_started = 0 THEN 'RETRY_WAIT' ELSE 'UNKNOWN' END, \
                     next_retry_at = CASE WHEN request_started = 0 THEN ?1 ELSE NULL END, \
                     unknown_reason = CASE WHEN request_started = 0 THEN NULL \
                                           ELSE '{UNKNOWN_REASON_LEASE_EXPIRED}' END, \
                     claim_token = NULL, claim_expires_at = NULL, updated_at = ?1 \
                 WHERE state = 'IN_FLIGHT' \
                   AND claim_expires_at IS NOT NULL AND claim_expires_at <= ?1"
                ),
                params![now],
            )
            .map_err(outbox_error)?;
        Ok(resolved as u64)
    }

    fn list_due_outbox(
        &self,
        bot_identity: &str,
        now: &str,
    ) -> Result<Vec<OutboxEntry>, SendStateError> {
        let sql = format!(
            "SELECT {OUTBOX_COLUMNS} FROM telegram_outbox \
             WHERE bot_identity = ?1 \
               AND state IN ('QUEUED', 'RETRY_WAIT') \
               AND (next_retry_at IS NULL OR next_retry_at <= ?2) \
               AND (claim_token IS NULL OR claim_expires_at IS NULL OR claim_expires_at <= ?2) \
             ORDER BY plan_order ASC, id ASC"
        );
        let mut statement = self.connection.prepare(&sql).map_err(outbox_error)?;
        let mut rows = statement
            .query(params![bot_identity, now])
            .map_err(outbox_error)?;
        let mut entries = Vec::new();
        while let Some(row) = rows.next().map_err(outbox_error)? {
            entries.push(outbox_from_row(row)?);
        }
        Ok(entries)
    }
}

impl FileIdCacheStore for Database {
    fn lookup_file_id(&self, key: &FileCacheKey) -> Result<Option<CachedFileId>, SendStateError> {
        // Bot identity is part of the lookup, so one bot can never reuse
        // another bot's file id (plan TG-05).
        self.connection
            .query_row(
                "SELECT file_id, file_unique_id, file_size, confirmed_at FROM telegram_file_cache \
                 WHERE bot_identity = ?1 AND content_sha256 = ?2 AND media_kind = ?3 \
                   AND representation_version = ?4",
                params![
                    key.bot_identity,
                    key.content_sha256,
                    media_kind_text(key.media_kind)?,
                    key.representation_version
                ],
                |row| {
                    Ok(CachedFileId {
                        file_id: row.get(0)?,
                        file_unique_id: row.get(1)?,
                        file_size: row.get::<_, i64>(2)?.max(0) as u64,
                        confirmed_at: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(outbox_error)
    }

    fn store_file_id(
        &self,
        key: &FileCacheKey,
        value: &CachedFileId,
    ) -> Result<(), SendStateError> {
        // Only reached after a confirmed success; a newer confirmation
        // replaces the older one.
        self.connection
            .execute(
                "INSERT INTO telegram_file_cache \
                     (bot_identity, content_sha256, media_kind, representation_version, \
                      file_id, file_unique_id, file_size, confirmed_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                 ON CONFLICT(bot_identity, content_sha256, media_kind, representation_version) \
                 DO UPDATE SET file_id = excluded.file_id, \
                               file_unique_id = excluded.file_unique_id, \
                               file_size = excluded.file_size, \
                               confirmed_at = excluded.confirmed_at",
                params![
                    key.bot_identity,
                    key.content_sha256,
                    media_kind_text(key.media_kind)?,
                    key.representation_version,
                    value.file_id,
                    value.file_unique_id,
                    i64::try_from(value.file_size)
                        .map_err(|_| SendStateError::Store("file size out of range".to_owned()))?,
                    value.confirmed_at,
                ],
            )
            .map_err(outbox_error)?;
        Ok(())
    }

    fn drop_file_cache_for_bot(&self, bot_identity: &str) -> Result<u64, SendStateError> {
        // Exactly one bot identity; other bots keep their entries.
        let removed = self
            .connection
            .execute(
                "DELETE FROM telegram_file_cache WHERE bot_identity = ?1",
                params![bot_identity],
            )
            .map_err(outbox_error)?;
        Ok(removed as u64)
    }

    fn delete_file_id(&self, key: &FileCacheKey) -> Result<u64, SendStateError> {
        let removed = self
            .connection
            .execute(
                "DELETE FROM telegram_file_cache \
                 WHERE bot_identity = ?1 AND content_sha256 = ?2 AND media_kind = ?3 \
                   AND representation_version = ?4",
                params![
                    key.bot_identity,
                    key.content_sha256,
                    media_kind_text(key.media_kind)?,
                    key.representation_version
                ],
            )
            .map_err(outbox_error)?;
        Ok(removed as u64)
    }
}
