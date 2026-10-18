//! F3 record import transcript and accepted-set commits.
//!
//! The frozen F3 contract requires conflicting duplicates to *reject the
//! affected records*, and it requires the accepted set committed *in one
//! transaction*. One transaction is all-or-nothing: any commit error rolls
//! back the entire accepted set, so mixed Accept/conflict inputs cannot leave
//! a partial durable set. The defense is execution order, not a reopened
//! contract: verdicts are rechecked after the transactional guard (and after
//! the maintenance `drain`, done by the caller before invoking this module),
//! commits follow, and a mid-commit conflict poisons the whole transaction.
//!
//! Write mapping reuses the existing production row paths and semantics, one
//! accepted record at a time: author identity through the same
//! `users`/`user_names` rows as `Database::upsert_user`, tweet identity and
//! payload through the same columns as `insert_tweet_for_user` /
//! `update_tweet_metadata`. The module creates no Job rows, emits no events,
//! touches no `archive_recovery` journal, and never resets an existing
//! `tweets.archived_at` from an unbounded import: `archived_at` is written
//! exactly as the record's own value. No multi-row aggregate is computed; every
//! per-record write re-checks the same unique keys as the non-transactional
//! helpers it reuses. Mixed accepted/conflicting inputs abort the transaction,
//! never persist only the accepted prefix.

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use xarchive_core::{ArchiveMetadata, DownloadRecord};

use crate::{Database, StorageError};

pub use crate::record_evidence::{
    RecordEvidenceVerdict, preview_archive_record, preview_download_record,
};

/// One accepted record: index into the caller's `records` slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedRecord {
    /// Index into the `records` slice passed to `import_accepted_records`.
    pub index: usize,
}

/// One commit failure, with the accepted-set index that triggered it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordImportCommitFailure {
    pub index: usize,
    pub reason: String,
}

/// Outcome of one transactional import.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum RecordImportOutcome {
    /// Every accepted record committed; identical ones counted as no-ops.
    Committed { accepted: usize, identical: usize },
    /// A conflicting duplicate existed in the input: nothing was committed.
    Conflicts { identities: Vec<String> },
    /// A commit error rolled back the entire accepted set.
    RolledBack {
        failures: Vec<RecordImportCommitFailure>,
    },
}

/// Probe mirroring `record_evidence`'s contract:
/// `(is_regular_file, size_bytes, sha256_hex)` or a reason.
pub type EvidenceProbe<R> = R;

/// Transcript equality for one record: identical payload under the same
/// identity is an idempotent no-op; a byte-different payload under the same
/// identity is a conflict. `Ok(true)` = identical, `Ok(false)` = absent.
fn transcript_disposition(
    existing_merged_json: Option<&str>,
    incoming_merged_json: &str,
) -> Result<bool, RecordImportOutcome> {
    match existing_merged_json {
        Some(existing) if existing == incoming_merged_json => Ok(true),
        Some(_) => Err(RecordImportOutcome::Conflicts {
            identities: vec![String::from("see accepted-set index")],
        }),
        None => Ok(false),
    }
}

// __IMPORT_TX_ANCHOR__

impl Database {
    /// Commit one pre-filtered accepted set in a single SQLite transaction.
    ///
    /// The caller must have run `preview_download_record` (evidence) and the
    /// maintenance `drain` before invoking this. `accepted` indexes into
    /// `records`. `archive_directory` is the caller-selected local archive
    /// root as a string; it is stored verbatim, never derived here.
    ///
    /// Per record: identical merged payload under the same tweet id is an
    /// idempotent no-op; a byte-different payload under the same tweet id is a
    /// conflict that aborts the whole transaction; an absent tweet id is
    /// committed through the existing user/tweet/metadata row paths. Any
    /// commit error rolls back the entire accepted set.
    pub fn import_accepted_records(
        &self,
        records: &[DownloadRecord],
        accepted: &[AcceptedRecord],
        now: &str,
        archive_directory: &str,
    ) -> Result<RecordImportOutcome, StorageError> {
        let transaction = self.connection.unchecked_transaction()?;
        let mut accepted_count = 0usize;
        let mut identical_count = 0usize;
        for item in accepted {
            let record = records.get(item.index).ok_or_else(|| {
                StorageError::InvalidMetadata(format!(
                    "accepted-set index {} is out of range",
                    item.index
                ))
            })?;
            let incoming = serde_json::to_string(&record.metadata)?;
            let existing: Option<String> = transaction
                .query_row(
                    "SELECT merged_metadata_json FROM tweets WHERE tweet_id = ?1",
                    params![record.metadata.tweet_id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(StorageError::from)?;
            match transcript_disposition(existing.as_deref(), &incoming) {
                Ok(true) => {
                    identical_count += 1;
                    continue;
                }
                Ok(false) => {}
                Err(conflicts) => {
                    // Do not commit: dropping `transaction` rolls everything back.
                    let mut identities = Vec::new();
                    identities.push(format!(
                        "{} / {}",
                        record.account_handle, record.metadata.tweet_id
                    ));
                    let _ = conflicts;
                    return Ok(RecordImportOutcome::Conflicts { identities });
                }
            }
            if let Err(error) = Self::commit_imported_record(
                &transaction,
                record,
                &incoming,
                now,
                archive_directory,
            ) {
                let reason = error.to_string();
                return Ok(RecordImportOutcome::RolledBack {
                    failures: vec![RecordImportCommitFailure {
                        index: item.index,
                        reason,
                    }],
                });
            }
            accepted_count += 1;
        }
        transaction.commit()?;
        Ok(RecordImportOutcome::Committed {
            accepted: accepted_count,
            identical: identical_count,
        })
    }

    /// Write one absent-tweet record through the existing production row
    /// paths, inside the caller's transaction. Author identity reuses the
    /// `users`/`user_names` rows with source `record-import`; tweet identity
    /// and payload reuse `insert_tweet_for_user`'s upsert columns and
    /// `update_tweet_metadata`'s merged-payload write. `incoming` is the
    /// already-serialized canonical metadata; it is parsed back only to reuse
    /// `update_tweet_metadata`'s signature, never reinterpreted.
    pub(crate) fn commit_imported_record(
        transaction: &rusqlite::Transaction<'_>,
        record: &DownloadRecord,
        incoming: &str,
        now: &str,
        archive_directory: &str,
    ) -> Result<(), StorageError> {
        let user_row_id = match record
            .metadata
            .author
            .user_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(user_id) => Some(Self::transactional_upsert_user(
                transaction,
                user_id,
                record.metadata.author.username.as_deref(),
                record.metadata.author.display_name.as_deref(),
                now,
            )?),
            None => None,
        };
        let metadata: ArchiveMetadata = serde_json::from_str(incoming)?;
        // Same columns as `insert_tweet_for_user`'s upsert. Insert first so an
        // absent tweet id is created, then overwrite the merged payload.
        transaction.execute(
            "INSERT INTO tweets (tweet_id, canonical_url, tweet_type, text, user_id, created_at, updated_at)\n             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)\n             ON CONFLICT(tweet_id) DO UPDATE SET canonical_url = excluded.canonical_url, user_id = COALESCE(excluded.user_id, tweets.user_id), updated_at = excluded.updated_at",
            params![
                metadata.tweet_id,
                metadata.url,
                metadata.tweet_type,
                metadata.text,
                user_row_id,
                now
            ],
        )?;
        let (tweet_row_id,): (i64,) = transaction.query_row(
            "SELECT id FROM tweets WHERE tweet_id = ?1",
            params![metadata.tweet_id],
            |row| Ok((row.get(0)?,)),
        )?;
        // Same merged-payload write as `update_tweet_metadata`.
        let quoted_tweet_id = metadata
            .quoted_tweet
            .as_ref()
            .and_then(|quoted| xarchive_core::TweetId::new(quoted.tweet_id.clone()).ok())
            .map(|id| id.as_str().to_owned());
        let reply_to_tweet_id = metadata
            .reply_to
            .as_ref()
            .and_then(|reply_to| xarchive_core::TweetId::new(reply_to.clone()).ok())
            .map(|id| id.as_str().to_owned());
        transaction.execute(
            "UPDATE tweets SET text = ?1, merged_metadata_json = ?2, archive_directory = ?3, archived_at = ?4, reply_to_tweet_id = ?5, quoted_tweet_id = ?6, updated_at = ?4 WHERE id = ?7",
            params![
                metadata.text,
                incoming,
                archive_directory,
                metadata.archived_at,
                reply_to_tweet_id,
                quoted_tweet_id,
                tweet_row_id
            ],
        )?;
        Ok(())
    }

    /// `Database::upsert_user` against a live transaction, with identical SQL
    /// and identical name-history semantics (`user_names` source for imports is
    /// `record-import`; an unchanged latest name writes nothing).
    fn transactional_upsert_user(
        transaction: &rusqlite::Transaction<'_>,
        user_id: &str,
        username: Option<&str>,
        display_name: Option<&str>,
        now: &str,
    ) -> Result<i64, StorageError> {
        if user_id.trim().is_empty() {
            return Err(StorageError::InvalidMetadata(
                "user_id must not be empty".into(),
            ));
        }
        let directory = xarchive_core::stable_user_directory_name(username, display_name, user_id);
        transaction.execute(
            "INSERT INTO users (x_user_id, stable_directory_name, first_seen_at, last_seen_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?3, ?3, ?3) ON CONFLICT(x_user_id) DO UPDATE SET stable_directory_name = excluded.stable_directory_name, last_seen_at = excluded.last_seen_at, updated_at = excluded.updated_at",
            params![user_id, directory, now],
        )?;
        let row_id: i64 = transaction.query_row(
            "SELECT id FROM users WHERE x_user_id = ?1",
            params![user_id],
            |row| row.get(0),
        )?;
        if let Some(username) = username.filter(|value| !value.trim().is_empty()) {
            let unchanged: bool = transaction
                .query_row(
                    "SELECT username, display_name FROM user_names WHERE user_id = ?1 ORDER BY observed_at DESC, id DESC LIMIT 1",
                    params![row_id],
                    |row| {
                        let latest_name: String = row.get(0)?;
                        let latest_display: Option<String> = row.get(1)?;
                        Ok(latest_name == username && latest_display.as_deref() == display_name)
                    },
                )
                .optional()
                .map_err(StorageError::from)?
                .unwrap_or(false);
            if !unchanged {
                transaction.execute(
                    "INSERT INTO user_names (user_id, username, display_name, source, observed_at) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![row_id, username, display_name, "record-import", now],
                )?;
            }
        }
        Ok(row_id)
    }
}

#[path = "record_import/tests.rs"]
#[cfg(test)]
mod record_import_tests;
