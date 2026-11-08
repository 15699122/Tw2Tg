//! Bot-isolated `file_id` cache contract (plan TG-05).
//!
//! This module owns the cache key shape, the confirmed-value record and the
//! invalidation rule. It performs no I/O and holds no connection; the persistence
//! implementation lives in the storage layer.
//!
//! The crate root re-exports every public item, so call sites are unchanged.

use serde::{Deserialize, Serialize};

use crate::contract::TelegramError;
use crate::delivery::SendStateError;
use crate::formatting::MediaKind;

// TG-05 — bot-isolated `file_id` cache contract
// ---------------------------------------------------------------------------

/// Representation version baked into every cache key. Bump it when the way
/// media is rendered or sent changes in a way that makes an older `file_id`
/// representation unusable for the new expectations.
pub const TELEGRAM_FILE_CACHE_VERSION: u32 = 1;

/// Cache key: `bot identity + SHA-256 + media kind + representation
/// version` (plan TG-05). `file_unique_id` is never part of the key — it is
/// identification only and never a send parameter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileCacheKey {
    pub bot_identity: String,
    pub content_sha256: String,
    pub media_kind: MediaKind,
    pub representation_version: u32,
}

/// A confirmed cached upload result. Written only after the Bot API
/// confirmed the send (plan TG-05).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedFileId {
    pub file_id: String,
    pub file_unique_id: String,
    pub file_size: u64,
    pub confirmed_at: String,
}

/// Persistence contract for the bot-isolated `file_id` cache.
pub trait FileIdCacheStore {
    /// Look up a file id for this exact bot and representation. A miss must
    /// fall back to uploading the original file, never to another bot's id.
    fn lookup_file_id(&self, key: &FileCacheKey) -> Result<Option<CachedFileId>, SendStateError>;

    /// Store a confirmed result. The caller only invokes this after a parsed
    /// `ok: true` response (plan TG-05).
    fn store_file_id(&self, key: &FileCacheKey, value: &CachedFileId)
    -> Result<(), SendStateError>;

    /// Remove one entry, used when Telegram explicitly reports the identifier
    /// as unusable so the next attempt uploads the original file again
    /// (plan TG-05). No other failure may call this.
    fn delete_file_id(&self, key: &FileCacheKey) -> Result<u64, SendStateError>;

    /// Drop every entry of exactly one bot identity (the chosen policy when
    /// the operator decides to purge after a token change). Entries of other
    /// identities are never touched.
    fn drop_file_cache_for_bot(&self, bot_identity: &str) -> Result<u64, SendStateError>;
}

/// True only for Telegram's explicit "this `file_id` is no longer usable"
/// answers. Permission, network or generic server failures must never be
/// treated as cache invalidation (plan TG-05): the cache entry stays until
/// Telegram itself says the identifier is dead.
pub fn is_invalid_file_id_error(error: &TelegramError) -> bool {
    let TelegramError::Api {
        code: 400 | 404,
        description,
        ..
    } = error
    else {
        return false;
    };
    let description = description.to_ascii_lowercase();
    [
        "invalid file id",
        "wrong file identifier",
        "file reference expired",
        "file reference empty",
    ]
    .iter()
    .any(|needle| description.contains(needle))
}

// ---------------------------------------------------------------------------
