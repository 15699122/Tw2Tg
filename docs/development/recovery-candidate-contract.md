# Attempt-bound business recovery candidate

Owner: Cross-platform Owner. Implements approved output Plan §5.6, option A
confirmed 2026-10-17. This does not enlarge the immutable recovery manifest.

## Field necessity

| Fields | Recovery consumer and necessity |
|---|---|
| schema_version | Explicit supported business representation: Storage v1 and Desktop normalized v2; unknown versions rejected |
| tweet_id | Match manifest and attempt-bound Tweet row; reject another identity |
| url, tweet_type, author, created_at | Restore the exact normalized merged business record; the initial Tweet row is not an attempt-bound replacement for these values |
| text | Restore both Tweet text and merged metadata in the final transaction |
| reply_to, quoted_tweet (including its normalized business fields) | Restore relationship columns and the merged record without re-fetching mutable network data |
| media | Restore merged metadata and cross-check the immutable manifest; duplication is required by the existing business DTO representation |
| archived_at | Restore the frozen archive timestamp rather than sampling restart time |

`finalize_archive_recovery_v2` persists the full normalized record into
`merged_metadata_json`. Removing these business fields would change committed
business semantics, not just remove a redundant recovery field. Optional user
exports are not an authoritative recovery source. Existing submission rows cannot
reconstruct all attempt-specific values. No raw Sidecar object is retained.

## Boundary and compatibility

Storage's private `recovery_candidate` module explicitly allowlists root, author,
quoted-Tweet and media fields. Future portable DTO fields fail closed until their
recovery necessity is reviewed. Encoding retains the existing JSON byte layout
and digest algorithm; safe existing PREPARED candidates require no rewrite or
migration. Unknown fields/versions fail closed, including on old stored rows;
they are not silently dropped. Integrity and attempt checks remain mandatory.

Structured Tweet URLs must be empty or HTTPS X/Twitter status URLs matching the
associated numeric Tweet ID. Query strings, fragments, userinfo, transport hosts
and unexpected paths are refused, not persisted or echoed in errors. This rule
also applies to quoted-Tweet URLs. Unsupported old URLs cause an explicit error
without deleting or rewriting their recovery record. Free-form authored text is
business content, not interpreted as transport configuration or scanned for
arbitrary URL-like prose. Request headers, cookies, credentials and raw transport
payloads have no allowed structural fields.

## Lifecycle

PREPARED retains the candidate across failures. Business facts/events, COMMITTED
and candidate deletion share one transaction. Rollback retains the exact
candidate. COMMITTED replay verifies durable business/files read-only and does
not require a candidate. Never time-expire a PREPARED candidate: recovery may
still require it. Deletion is logical SQLite cleanup, not secure erasure.

Old COMMITTED candidate rows are deliberately not cleaned by replay. A separate
controlled cleanup task must validate committed facts before deleting redundant
rows. Historical cleanup is not claimed complete by this change.