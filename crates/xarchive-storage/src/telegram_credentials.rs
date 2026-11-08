//! Telegram credential generations: rotation, resume authorization and the
//! claim fence.
//!
//! A credential generation fences which worker may claim and record sends. The
//! methods here are the only writers of that fence, and they run inside the
//! same SQLite transactions the callers already used before this module
//! existed; the SQL, ordering and error mapping are unchanged.
//!
//! No in-memory state is held: every decision is read from and written to the
//! database, so a restarted worker observes exactly the persisted fence.

use rusqlite::OptionalExtension;

use crate::{Database, StorageError};

impl Database {
    /// Fence credential verification and claiming in one SQLite write transaction.
    /// A stale worker gets no work, even if the replacement authorized the row.
    pub fn claim_due_outbox_for_generation(
        &self,
        generation: i64,
        bot_identity: &str,
        claim_token: &str,
        now: &str,
        lease_until: &str,
    ) -> Result<Option<xarchive_telegram::OutboxEntry>, xarchive_telegram::SendStateError> {
        use xarchive_telegram::{SendStateError, TelegramOutboxStore};
        let transaction = self
            .connection
            .unchecked_transaction()
            .map_err(|_| SendStateError::Store("claim transaction unavailable".into()))?;
        // Acquire the writer lock before reading the active generation. A deferred
        // read alone would permit a competing activation between read and claim.
        transaction
            .execute(
                "UPDATE telegram_credential_generations SET status = status WHERE generation = ?1",
                [generation],
            )
            .map_err(|_| SendStateError::Store("credential fence unavailable".into()))?;
        let matches: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM telegram_credential_generations WHERE generation = ?1 AND bot_identity = ?2 AND status = 'ACTIVE')",
            rusqlite::params![generation, bot_identity], |row| row.get(0),
        ).map_err(|_| SendStateError::Store("credential fence unavailable".into()))?;
        if !matches {
            return Ok(None);
        }
        let claimed = self.claim_due_outbox(bot_identity, claim_token, now, lease_until)?;
        if claimed.is_some() {
            transaction.execute(
                "UPDATE telegram_outbox SET claim_generation = ?2 WHERE claim_token = ?1 AND state = 'IN_FLIGHT'",
                rusqlite::params![claim_token, generation],
            ).map_err(|_| SendStateError::Store("claim generation binding failed".into()))?;
        }
        transaction
            .commit()
            .map_err(|_| SendStateError::Store("claim commit failed".into()))?;
        Ok(claimed)
    }

    /// Capture a rotation decision once. Replays must match the original facts;
    /// changing settings does not retroactively change an existing decision.
    pub fn record_telegram_rotation_decision(
        &self,
        generation: i64,
        policy: &str,
        candidate_ids: &[i64],
        now: &str,
    ) -> Result<(), StorageError> {
        if !matches!(policy, "automatic" | "confirm") {
            return Err(StorageError::InvalidState("invalid rotation policy".into()));
        }
        let json = serde_json::to_string(candidate_ids)?;
        let transaction = self.connection.unchecked_transaction()?;
        transaction.execute(
            "INSERT INTO telegram_rotation_decisions(generation, policy, candidate_ids_json, created_at) VALUES (?1, ?2, ?3, ?4) ON CONFLICT DO NOTHING",
            rusqlite::params![generation, policy, json, now],
        )?;
        let existing: (String, String) = transaction.query_row(
            "SELECT policy, candidate_ids_json FROM telegram_rotation_decisions WHERE generation = ?1",
            [generation], |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        if existing != (policy.to_owned(), json) {
            return Err(StorageError::InvalidState(
                "rotation decision conflict".into(),
            ));
        }
        if policy == "automatic" {
            let bot: String = transaction.query_row(
                "SELECT bot_identity FROM telegram_credential_generations WHERE generation = ?1 AND status = 'ACTIVE'",
                [generation], |row| row.get(0),
            )?;
            for id in candidate_ids {
                let eligible: bool = transaction.query_row(
                    "SELECT EXISTS(SELECT 1 FROM telegram_outbox WHERE id = ?1 AND bot_identity = ?2 AND state IN ('QUEUED', 'RETRY_WAIT') AND payload_json IS NOT NULL AND payload_schema_version = 1)",
                    rusqlite::params![id, bot], |row| row.get(0),
                )?;
                if !eligible {
                    return Err(StorageError::InvalidState(
                        "rotation candidate no longer eligible".into(),
                    ));
                }
                transaction.execute(
                    "INSERT INTO telegram_resume_authorizations(generation, outbox_id, granted_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
                    rusqlite::params![generation, id, now],
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn telegram_rotation_decision(
        &self,
        generation: i64,
    ) -> Result<Option<(String, Vec<i64>)>, StorageError> {
        let record: Option<(String, String)> = self.connection.query_row(
            "SELECT policy, candidate_ids_json FROM telegram_rotation_decisions WHERE generation = ?1",
            [generation], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;
        record
            .map(|(policy, json)| Ok((policy, serde_json::from_str(&json)?)))
            .transpose()
    }

    /// Snapshot rows eligible for rotation; future rows are never included.
    pub fn telegram_resume_candidates(&self, generation: i64) -> Result<Vec<i64>, StorageError> {
        let mut statement = self.connection.prepare(
            "SELECT o.id FROM telegram_outbox o JOIN telegram_credential_generations g ON g.bot_identity = o.bot_identity WHERE g.generation = ?1 AND g.status = 'ACTIVE' AND o.state IN ('QUEUED', 'RETRY_WAIT') AND o.payload_json IS NOT NULL AND o.payload_schema_version = 1 ORDER BY o.id",
        )?;
        Ok(statement
            .query_map([generation], |row| row.get(0))?
            .collect::<Result<Vec<_>, _>>()?)
    }

    /// Grant only explicitly selected recoverable rows to the current generation.
    /// Any invalid selection rolls back the entire grant; UNKNOWN is never eligible.
    pub fn authorize_telegram_resume(
        &self,
        generation: i64,
        outbox_ids: &[i64],
        now: &str,
    ) -> Result<(), StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        let bot: Option<String> = transaction.query_row(
            "SELECT bot_identity FROM telegram_credential_generations WHERE generation = ?1 AND status = 'ACTIVE'",
            [generation], |row| row.get(0),
        ).optional()?;
        let bot =
            bot.ok_or_else(|| StorageError::InvalidState("active credential changed".into()))?;
        for id in outbox_ids {
            let eligible: bool = transaction.query_row(
                "SELECT EXISTS(SELECT 1 FROM telegram_outbox WHERE id = ?1 AND bot_identity = ?2 AND state IN ('QUEUED', 'RETRY_WAIT') AND payload_json IS NOT NULL AND payload_schema_version = 1)",
                rusqlite::params![id, bot], |row| row.get(0),
            )?;
            if !eligible {
                return Err(StorageError::InvalidState(
                    "outbox row is not eligible for resume".into(),
                ));
            }
            transaction.execute(
                "INSERT INTO telegram_resume_authorizations(generation, outbox_id, granted_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
                rusqlite::params![generation, id, now],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    /// Explicit operator review of one uncertain outcome. This is not an
    /// automatic retry: the caller must acknowledge possible duplication.
    /// Preserve unknown_reason as audit evidence until a subsequent result.
    pub fn review_telegram_unknown_for_resend(
        &self,
        generation: i64,
        outbox_id: i64,
        confirmed: bool,
        now: &str,
    ) -> Result<(), StorageError> {
        if !confirmed {
            return Err(StorageError::InvalidState(
                "duplicate-risk confirmation required".into(),
            ));
        }
        let transaction = self.connection.unchecked_transaction()?;
        let changed = transaction.execute(
            "UPDATE telegram_outbox SET state = 'QUEUED', next_retry_at = NULL, updated_at = ?3 \
             WHERE id = ?2 AND state = 'UNKNOWN' AND payload_schema_version = 1 AND payload_json IS NOT NULL \
             AND EXISTS(SELECT 1 FROM telegram_credential_generations g WHERE g.generation = ?1 AND g.status = 'ACTIVE' AND g.bot_identity = telegram_outbox.bot_identity)",
            rusqlite::params![generation, outbox_id, now],
        )?;
        if changed != 1 {
            return Err(StorageError::InvalidState(
                "review target or credential changed".into(),
            ));
        }
        transaction.execute(
            "INSERT INTO telegram_resume_authorizations(generation, outbox_id, granted_at) VALUES (?1, ?2, ?3) ON CONFLICT DO NOTHING",
            rusqlite::params![generation, outbox_id, now],
        )?;
        transaction.commit()?;
        Ok(())
    }

    /// Revoke sending authority before deleting a native secret. A failed
    /// secret deletion leaves an inactive orphan, never an active credential.
    pub fn retire_telegram_credential_generation(&self, expected: i64) -> Result<(), StorageError> {
        let changed = self.connection.execute(
            "UPDATE telegram_credential_generations SET status = 'RETIRED' WHERE generation = ?1 AND status = 'ACTIVE'",
            [expected],
        )?;
        if changed != 1 {
            return Err(StorageError::InvalidState(
                "active credential changed".into(),
            ));
        }
        Ok(())
    }

    pub fn telegram_resume_authorized(
        &self,
        generation: i64,
        outbox_id: i64,
    ) -> Result<bool, StorageError> {
        Ok(self.connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM telegram_resume_authorizations a JOIN telegram_credential_generations g ON g.generation = a.generation JOIN telegram_outbox o ON o.id = a.outbox_id WHERE a.generation = ?1 AND a.outbox_id = ?2 AND g.status = 'ACTIVE' AND g.bot_identity = o.bot_identity AND o.state IN ('QUEUED', 'RETRY_WAIT'))",
            rusqlite::params![generation, outbox_id], |row| row.get(0),
        )?)
    }

    /// Register an already verified identity and an opaque SecretStore key.
    /// Registration never activates a credential or authorizes queued sends.
    pub fn prepare_telegram_credential_generation(
        &self,
        bot_identity: &str,
        secret_reference: &str,
        now: &str,
    ) -> Result<i64, StorageError> {
        let id = bot_identity
            .strip_prefix("telegram-bot:")
            .and_then(|value| value.parse::<i64>().ok());
        if !id.is_some_and(|id| id > 0) || secret_reference.trim().is_empty() {
            return Err(StorageError::InvalidState(
                "invalid credential identity or reference".into(),
            ));
        }
        self.connection.execute(
            "INSERT INTO telegram_credential_generations(bot_identity, secret_reference, status, created_at) VALUES (?1, ?2, 'CANDIDATE', ?3)",
            rusqlite::params![bot_identity, secret_reference, now],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    /// Compare-and-set activation prevents a stale verification from replacing
    /// a newer active credential. This does not itself grant queue authority.
    pub fn activate_telegram_credential_generation(
        &self,
        candidate: i64,
        expected_active: Option<i64>,
    ) -> Result<(), StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        let active: Option<i64> = transaction
            .query_row(
                "SELECT generation FROM telegram_credential_generations WHERE status = 'ACTIVE'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if active != expected_active {
            return Err(StorageError::InvalidState(
                "credential generation changed".into(),
            ));
        }
        let ready: bool = transaction.query_row(
            "SELECT EXISTS(SELECT 1 FROM telegram_credential_generations WHERE generation = ?1 AND status = 'CANDIDATE')",
            [candidate], |row| row.get(0),
        )?;
        if !ready {
            return Err(StorageError::InvalidState(
                "credential candidate unavailable".into(),
            ));
        }
        transaction.execute(
            "UPDATE telegram_credential_generations SET status = 'RETIRED' WHERE status = 'ACTIVE'",
            [],
        )?;
        transaction.execute(
            "UPDATE telegram_credential_generations SET status = 'ACTIVE' WHERE generation = ?1",
            [candidate],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn active_telegram_credential_generation(
        &self,
    ) -> Result<Option<(i64, String, String)>, StorageError> {
        Ok(self.connection.query_row(
            "SELECT generation, bot_identity, secret_reference FROM telegram_credential_generations WHERE status = 'ACTIVE'",
            [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).optional()?)
    }

    /// Atomically activate a candidate and capture the rotation policy and
    /// eligible queue. SecretStore writes must already have succeeded.
    pub fn activate_telegram_credential_with_resume(
        &self,
        candidate: i64,
        expected_active: Option<i64>,
        policy: &str,
        now: &str,
    ) -> Result<Vec<i64>, StorageError> {
        if !matches!(policy, "automatic" | "confirm") {
            return Err(StorageError::InvalidState("invalid rotation policy".into()));
        }
        let tx = self.connection.unchecked_transaction()?;
        let previous: Option<(i64, String)> = tx.query_row(
            "SELECT generation, bot_identity FROM telegram_credential_generations WHERE status = 'ACTIVE'",
            [], |row| Ok((row.get(0)?, row.get(1)?)),
        ).optional()?;
        if previous.as_ref().map(|value| value.0) != expected_active {
            return Err(StorageError::InvalidState(
                "credential generation changed".into(),
            ));
        }
        let bot: String = tx.query_row(
            "SELECT bot_identity FROM telegram_credential_generations WHERE generation = ?1 AND status = 'CANDIDATE'",
            [candidate], |row| row.get(0),
        )?;
        let mut ids = Vec::<i64>::new();
        if previous.as_ref().is_some_and(|value| value.1 == bot) {
            let mut statement = tx.prepare(
                "SELECT id FROM telegram_outbox WHERE bot_identity = ?1 AND state IN ('QUEUED', 'RETRY_WAIT') AND payload_json IS NOT NULL AND payload_schema_version = 1 ORDER BY id",
            )?;
            ids = statement
                .query_map([&bot], |row| row.get(0))?
                .collect::<Result<Vec<_>, _>>()?;
        }
        tx.execute(
            "UPDATE telegram_credential_generations SET status = 'RETIRED' WHERE status = 'ACTIVE'",
            [],
        )?;
        tx.execute(
            "UPDATE telegram_credential_generations SET status = 'ACTIVE' WHERE generation = ?1",
            [candidate],
        )?;
        tx.execute(
            "INSERT INTO telegram_rotation_decisions(generation, policy, candidate_ids_json, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![candidate, policy, serde_json::to_string(&ids)?, now],
        )?;
        if policy == "automatic" {
            for id in &ids {
                tx.execute(
                    "INSERT INTO telegram_resume_authorizations(generation, outbox_id, granted_at) VALUES (?1, ?2, ?3)",
                    rusqlite::params![candidate, id, now],
                )?;
            }
        }
        tx.commit()?;
        Ok(ids)
    }
}
