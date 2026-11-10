//! Persisted payload restore and media resolution.
//!
//! Split out of `telegram_send.rs` without changing behaviour or the public
//! path: the parent re-exports these items, so `crate::telegram_send::<name>`
//! keeps resolving.
//!
//! The rules this module enforces:
//!
//! * a queued payload is rebuilt from its immutable version-1 snapshot only,
//!   never from current settings, so a configuration edit cannot redirect work
//!   that is already queued;
//! * a snapshot media path is accepted only when it names a recorded media fact
//!   of the same tweet and the resolved file matches the recorded digest and
//!   size, which rejects both traversal and a stale archive directory;
//! * an expired claim returns to the retry queue only if its request never
//!   started; otherwise it becomes UNKNOWN and waits for a human decision.

use xarchive_storage::Database;

use xarchive_telegram::{OutboxEntry, SendPayload, TelegramOutboxStore};

/// Rebuild a send payload exclusively from the immutable persisted snapshot.
/// Current settings are intentionally not consulted, preventing config edits
/// from redirecting queued work.
pub fn restore_payload_from_entry(entry: &OutboxEntry) -> Option<SendPayload> {
    if entry.payload_schema_version != Some(1) {
        return None;
    }
    serde_json::from_str(entry.payload_json.as_deref()?).ok()
}

/// Resolve persisted archive-relative media references against the tweet's
/// recorded archive directory. FileStore rejects traversal and existing links.
pub fn resolve_archived_payload(
    database: &Database,
    entry: &OutboxEntry,
    files: &xarchive_storage::FileStore,
) -> Option<SendPayload> {
    let mut payload = restore_payload_from_entry(entry)?;
    let SendPayload::Media { items, .. } = &mut payload else {
        return Some(payload);
    };
    let tweet_id = entry.tweet_id?;
    let external_id = database.tweet_external_id(tweet_id).ok()??;
    let facts = database.tweet_archive_facts(&external_id).ok()??;
    // The committed archive directory is the only authority for the on-disk
    // location; the snapshot only supplies the archive-relative selection key.
    let archive_directory = files.archive_path(&facts.archive_directory).ok()?;
    for item in items {
        // A snapshot path is accepted only when it names a recorded media fact
        // of this tweet. Reconstructing the archive-root-relative path from the
        // committed directory rejects both traversal and a stale directory.
        let fact = facts.media.iter().find(|fact| {
            std::path::Path::new(&facts.archive_directory).join(&fact.relative_path)
                == item.file_path
        })?;
        let resolved = files.archive_path(&item.file_path).ok()?;
        if !resolved.starts_with(&archive_directory) || !resolved.is_file() {
            return None;
        }
        if let Some(expected) = fact.sha256.as_deref()
            && xarchive_storage::FileStore::sha256(&resolved)
                .ok()?
                .as_str()
                != expected
        {
            return None;
        }
        if let Some(expected) = item.content_sha256.as_deref()
            && xarchive_storage::FileStore::sha256(&resolved)
                .ok()?
                .as_str()
                != expected
        {
            return None;
        }
        if let Some(expected) = fact.size_bytes
            && std::fs::metadata(&resolved).ok()?.len() != expected
        {
            return None;
        }
        item.file_path = resolved;
    }
    Some(payload)
}

/// Resolve claims left behind by a crashed worker.
///
/// A claim that expired before its request started returns to the retry queue;
/// one that expired after the request started becomes `UNKNOWN` and waits for
/// human review (plan TG-04).
pub fn recover_expired_claims(database: &Database, now: &str) -> Result<u64, String> {
    database
        .recover_outbox_claims(now)
        .map_err(|error| error.to_string())
}
