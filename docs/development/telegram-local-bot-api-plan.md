# Telegram Local Bot API Plan

### 2026-10-04 continuation: completion-time production capture

Production archive execution now reads persisted non-secret Telegram configuration
after extraction/transfer completes rather than using the context-creation snapshot.
Missing or invalid configuration fails closed. The verified active bot identity and
portable metadata/media facts become a PREPARED journal intent, finalized through
ArchiveService and materialized idempotently after local commit. Recovery does not
resample settings. New outbox rows receive matching active-generation authorization
inside the materialization transaction; replay never grants existing historical rows.
Regression coverage includes changed target/revision, disabling, malformed/missing
configuration, new-row authority and post-rotation replay without reauthorization.

Shared Tauri entry points now expose task-state labels, captured-candidate resume,
bounded stop, non-secret settings save and queued/retry cancellation. Settings save
invalidates capability and owns revision advancement; it stops the old worker but
does not fabricate a provider or restart without one. UNKNOWN remains review-only.
Production credential/provider/startup/restart wiring and frontend integration remain
IN_PROGRESS, as do broader chain fault tests. This is not a complete automatic-send
product or a Windows/real-service PASS. Current evidence and remaining work are in
the latest checkpoint of `../status/platform-handoff.md`.

<!-- Cross-session receipt: see docs/status/platform-handoff.md, 2026-10-04. -->

Heartbeat checkpoint (2026-10-04): clock-aware Desktop attempts are guarded by
a 60-second Tokio timer that extends the 300-second claim. Timer readiness is
checked before attempt completion; explicit loss or storage error stops the
driver without committing its result/cache. Desktop 199/199 and strict
all-target Clippy PASS. Existing short-request tests do not exercise a timer
tick; accelerated heartbeat/fault tests and bounded storage-error retry remain
pending. Dropping the driver does not prove a blocking request was unsent.

Monotonic lease checkpoint (2026-10-04): renewal rejects deadlines earlier
than the existing lease or not later than now; equal deadlines remain valid.
Storage 71/71 tests and Storage/Desktop strict all-target Clippy PASS. The test
process was externally suspended by terminal job control and resumed with
SIGCONT, rather than changing assertions or rerunning a second copy.

Lease checkpoint (2026-10-04): request-start and every claimed terminal/retry
write now require an unexpired lease, even before the recovery scanner runs.
Regression assertions reject SENT, retry, UNKNOWN, failure, cancellation and
request-start writes from an expired owner. Storage 71/71 and Desktop 199/199
tests PASS; strict all-target Clippy PASS. Runtime heartbeat and fresh-clock
integration remain pending; this storage fence alone does not make uploads safe.

Caption checkpoint (2026-10-04): new captured intents use planner version 3.
Short metadata merges into the first media caption only when that caption is
absent; existing captions are preserved and long metadata remains separate.
Versions 1 and 2 retain their earlier layout. Desktop 199/199 tests and strict
all-target Clippy PASS. Production capture/sender/commands remain unfinished;
this is not Windows rendering or real-send acceptance.

Planner compatibility checkpoint (2026-10-04): newly captured intents use plan
version 2 (bounded text splitting). Version 1 keeps its original single-message
text plan for deterministic historical replay. Both direct and journal planning
reject unknown versions before writing outbox rows. Desktop 198/198 tests and
strict all-target Clippy PASS; caption-first layout remains pending.

Current credential increment (2026-10-04): Desktop verified-credential activation
now captures rotation policy and eligible row IDs atomically with activation.
The default wrapper selects automatic; the explicit policy entry supports confirm.
Confirmation replay preserves its original candidate set even after settings change.
Storage 70/70 and Desktop 197/197 tests PASS; strict all-target Clippy PASS.
This helper is not yet wired to production Tauri credential commands. Production
archive capture, sender heartbeat/scheduling, command/UI integration and subsystem
fault tests remain unfinished; no Windows or real Telegram result is claimed.

Planner increment (2026-10-04): archive metadata longer than the existing text
limit is split into ordered units using the shared splitter. Single-unit keys
remain unchanged; multi-unit keys include their index. A Unicode regression
verifies complete text preservation, bounded chunks and idempotent replay.
Desktop 198/198 tests and strict all-target Clippy PASS. Caption-first layout
and planner-version compatibility for historical oversized intents still need
completion before production archive capture is enabled.

Owner: Cross-platform Owner (shared contract, transport, persistence, planner, caching, cross-platform tests, shared docs); Windows Platform Owner (Windows Credential Manager adapter, Data Protection, packaged Local Bot API Server deployment, real-account send, Windows GUI acceptance).

Status: `IN_PROGRESS (resumed 2026-10-03; Linux round 2026-10-04)` — TG-00 is committed; TG-01
(endpoint contract, config contract and transport wiring), TG-02 (async streaming upload
transport), TG-03, TG-04 (outbox, atomic claim, recovery), TG-05 (bot-isolated `file_id`
cache) and the TG-06 shared business model exist as shared code in `xarchive-telegram`,
`xarchive-storage` and the Desktop config/send core. At `1f14cea`, Linux recorded Telegram
48/48, storage 54/54 and Desktop 176/176; on the 2026-10-03 working tree Linux recorded
Telegram 48/48, storage 60/60 and Desktop 189/189 with clippy clean. On 2026-10-04 the retained
working-tree P0 changes passed Telegram 50/50, storage 60/60, Desktop 191/191, strict targeted
Clippy, frontend tests/check/build, Sidecar 54/54 and docs audit. Windows results are recorded
separately below.
**This implementation batch has resumed.** Production archive enqueue/recovery, runtime scheduling,
UI, Windows credentials and real send remain outstanding. The remaining production runtime,
archive enqueue, recovery, Tauri commands and task projection are cross-platform development;
they must not be treated as Windows-only verification. No release ships or accepts this
Telegram scope yet. Only subsections explicitly marked `IMPLEMENTED` describe completed work;
resuming development does not retroactively change any published release.

### Current round boundary — 2026-10-04

The Linux round completed bounded shared-layer checks and the credential rotation configuration
contract, but **did not complete all Plan work that is independent of Windows**. On 2026-10-04 the
cross-platform owner added migration `0009` and a durable archive-intent recovery journal with
idempotence and state-transition tests. This is a recovery foundation, not the completed archive/outbox
coordination protocol: production enqueue, intent-to-outbox recovery, runtime scheduling and UI remain
unimplemented. The following
remain implementation work owned by the Cross-platform Owner, not BLOCKED Windows validation:

1. Define stable bot identity versus credential generation, token replacement failure semantics,
   and persisted queue authorization for `automatic` versus `confirm` rotation.
2. Complete fault testing for every archive-intent/filesystem/SQLite crash boundary, then implement
   idempotent ARCHIVED-to-outbox materialization and QUEUED transition. Do not claim these systems
   share an atomic transaction; production enqueue must wait for this full recovery protocol.
3. Wire archive intent creation/reconciliation, one sender loop, live lease renewal, startup
   recovery and single-instance scheduling.
4. Wire Tauri settings/credential/send commands and archive-vs-send task projection; add command,
   recovery, concurrency and failure tests.

**Batch A recovery corrections (2026-10-04):** Recovery after rename now reads final-directory
metadata without creating staging, rejects simultaneous staging/final directories, checks persisted
intent identity, job/tweet identity, metadata schema, text presence and media size/digest. Repeated
intent preparation ignores lifecycle state/update timestamps; PREPARED-to-SKIPPED is rejected rather
than violating the existing SQL constraint. The regression simulates rename before SQLite archive
completion and rejects damaged metadata. Batch A remains IN_PROGRESS: exhaustive crash injection,
database reopen/concurrent recovery, journal identity/version design, durable manual-review semantics
and archive-result/send-error isolation still need implementation and tests. No production send is enabled.

**Atomic outbox plan increment (2026-10-04):** Desktop now converts every planned send before
writing and uses a SQLite transaction for batch materialization. A conflict in any entry rolls
back new entries; replay returns the same row identities. Storage 63/63 and Desktop 194/194,
strict storage/Desktop all-target Clippy PASS. This does not yet atomically advance the journal,
wire a production caller, implement credential authorization or start a sender scheduler.

**Durable recovery test increment (2026-10-04):** Separate tests now cover staging-before-rename
and final-after-rename recovery using a disk SQLite database closed and reopened before recovery.
Each closes and reopens the database again to verify persisted ARCHIVED progress. Storage 64/64
PASS. This is restart evidence, not power-loss durability, concurrent recovery, or the complete
fault matrix; remaining shared development and Windows acceptance are still outstanding.

**Journal/outbox transaction increment (2026-10-04):** Storage exposes
`enqueue_archived_intent`, which validates journal state and entry tweet/bot/chat/topic/config/plan
identity, inserts entries and advances ARCHIVED to QUEUED in one transaction. Regression assertions
cover a mismatched second entry rolling back the first and preserving ARCHIVED, successful commit,
and idempotent replay. Storage 64/64 and strict all-target Clippy PASS. The API is not yet called
by Desktop; complete snapshot derivation and plan completeness validation remain caller obligations
and need dedicated integration tests before production use.

**Persisted planner increment (2026-10-04):** Desktop `queue_persisted_archive_sends` loads
the journal snapshot, rejects unsupported plan versions/invalid media or missing target facts,
derives all entries through the shared planner, and calls the journal/outbox transaction API.
Regression covers PREPARED refusal without rows, ARCHIVED commit, topic/config/target preservation
and QUEUED replay. Desktop 195/195 and strict storage/Desktop all-target Clippy PASS. This is an
explicit coordination API, not yet production archive/startup wiring or a sender scheduler.

**Snapshot rejection tests (2026-10-04):** Targeted persisted planner tests 2/2 PASS,
including malformed JSON, wrong media JSON shape and unsupported plan version. Each failure
leaves no outbox rows and preserves ARCHIVED plus its previous timestamp. Desktop strict
all-target Clippy PASS. The full Desktop suite was not rerun for this test-only increment.

These items cannot be safely declared complete by the current Linux checks. Once implemented and
cross-platform-tested, Windows-only acceptance is a separate phase and is summarized in §8 and
`windows-queue.md` / `windows-manual-steps.md` §K.

**Latest incremental Linux evidence (same round):** The Desktop sender now consumes an immutable
`ArchiveSendIntent` when planning outbox rows, and a fake-server regression verifies a text payload
retains its persisted `message_thread_id`. Shared token replace/delete orchestration has targeted
tests; it does not implement the Windows adapter or queue rotation authorization. Latest affected
checks: Telegram 50/50, storage 60/60, Desktop lib 194/194; strict three-crate Clippy, fmt, diff
check and docs audit PASS. Full Rust workspace regression and Node/Sidecar reruns were not selected
for this Rust/doc-only increment. Windows and real Telegram acceptance remain NOT_RUN.

**Current-state authority:** `docs/development/status.md` owns the concise implemented-capability
summary; this Plan owns requirements, work-package phase and implementation boundaries;
`docs/status/platform-handoff.md` owns only the active batch/revisions/next Owner;
`docs/validation/windows-queue.md` owns acceptance-item states; and
`docs/validation/windows-manual-steps.md` owns executable procedures, not a second status ledger.
Windows evidence remains bound to the exact source and artifact recorded in validation history.

## 1. Why this document exists

Telegram send was paused on 2026-10-01 and excluded from the `v0.2.1-pre1` release scope ([`status.md`](status.md), [`../release/notes/v0.2.1-pre1.md`](../release/notes/v0.2.1-pre1.md)). The Owner resumed Telegram development on 2026-10-03 for a future release. This plan describes how the **already existing** `xarchive-telegram` contract crate is completed and wired into the Desktop runtime. The prior release scope and its claims remain unchanged.

This is **not** a new uploader. It is the completion of an existing contract plus its production integration.

### 1.1 What already exists (do not rebuild)

| Area | Existing implementation |
|---|---|
| Secret abstraction | `SecretStore` trait, `MemorySecretStore`, `BotToken` (redacted `Debug`/`Display`) |
| Request models | `SendMessageRequest`, `SendPhotoRequest`, `SendVideoRequest`, `SendMediaGroupRequest`, `MediaGroupItem` |
| Transport | `ReqwestTelegramTransport` (blocking reqwest 0.13.4, Rustls, JSON) |
| Formatter | `format_metadata`, `split_text`, `split_for_telegram`, `media_groups` |
| Send state | `SendState`, `SendStateStore` trait, `SentSendRecord`, `PendingSendRecord`, idempotent send helper |
| Endpoint contract (TG-01, partial) | `IMPLEMENTED` — `EndpointMode` (`cloud`/`local`), validated `TelegramEndpoint` (cloud: HTTPS; local: explicit loopback HTTP with port), mode-specific typed errors, no userinfo/query/fragment/path, `method_url()` keeps the token out of logs; unit-tested acceptance/rejection matrix |
| Media constants and send plan (TG-03, partial) | `IMPLEMENTED` — album bounds (`TELEGRAM_ALBUM_MIN_ITEMS = 2`), caption limit, cloud/local upload ceilings as capability ceilings, `MediaKind`/`UploadMode`/`MediaGroupSend`, MIME-then-extension `classify_media()`, deterministic `stable_upload_file_name()`, trailing-single promotion in `media_send_plan()`; unit-tested boundary matrix |
| Secret-store failure contract (TG-01, partial) | `IMPLEMENTED` — `SecretStoreError::Unavailable`/`AccessDenied` carry only the failure description, never the secret; unit-tested redaction |
| Persistence | `telegram_send_attempts` table + `Database` implementation of `SendStateStore` |
| Desktop config | `telegram_timeout_seconds` in `NetworkConfig` |

Sources: `crates/xarchive-telegram/src/lib.rs`, `crates/xarchive-storage/src/database/telegram.rs`, `crates/xarchive-storage/migrations/0002_telegram_send_state.sql`, `desktop/src-tauri/src/config.rs`.

### 1.2 What is missing

- Config, endpoint, outbox, retry-classification, `file_id` cache, send plans and
  progress-projection contracts are implemented and unit-tested, including the attempt driver
  that turns a claimed entry into a durable transition. What is still missing is the Desktop
  runtime that enqueues after an archive, schedules the claim loop and renders the settings
  page, plus the Windows credential adapter.
- No real send has ever run: there is no authorised bot/target evidence, no Local Bot API
  Server deployment, and no Unigram receiving-side evidence (`WQ-TG-*` all `NOT_RUN`).

## 2. Target architecture

```text
Browser / Desktop
       │
       ▼
Local archive completes; files verified and committed
       │
       ▼
SQLite Telegram Outbox   (persistent work list, not a second database)
       │
       ▼
Telegram Sender
  ├─ send plan, rate limiting, recovery
  ├─ file_id cache
  ├─ streaming multipart
  └─ later: controlled server-local path
       │
       ▼
Cloud HTTPS  or  Local loopback HTTP
       │
       ▼
Telegram
       │
       ▼
Unigram (user's Windows receiving client)
```

"Outbox" means rows in the existing SQLite database that describe pending sends. It is not a new service or datastore.

### 2.1 Telegram must not block local archiving

The current state-machine document places `TG_METADATA_SENDING` **before** download ([`../architecture/job-state-machine.md`](../architecture/job-state-machine.md)). This plan changes that contract:

- Local archive success stands on its own.
- Telegram delivery state is displayed separately.
- Telegram unavailability does not block archiving.
- A Telegram-only retry does not trigger a re-download.
- Automatic sending is **off** until the user explicitly enables it.

This is a **shared state-machine change** and is part of TG-00, not a silent implementation detail. The first version does not send metadata before media is archived; if that is wanted later it becomes a separate send policy.

### 2.2 Unigram is the receiving client, not a sending dependency

Unigram ([`unigramdev/unigram`](https://github.com/unigramdev/unigram)) is the Owner's Windows Telegram client. It receives and renders what XArchive sends; it is **not** the Local Bot API Server and is not called by XArchive.

Boundaries:

- XArchive must send whether or not Unigram is running.
- XArchive never reads Unigram credentials, caches, or its local message database.
- The user is never asked to log a Telegram user account into XArchive.
- Unigram's notification center, storage and download settings are **not** evidence of Telegram delivery.
- Unigram is the **primary Windows receiving-side acceptance target**, not a runtime dependency.

### 2.3 Unigram is a schedule input, not a blocker

Media display issues reported against Unigram (see §5) shape acceptance and file naming. They must not silently change the send protocol, trigger transcoding, or trigger automatic re-sends. A client-side rendering problem is not a send failure.

## 3. Scope of the first version

### 3.1 In scope

- Single bot, one default target, optional topic/`message_thread_id`.
- Cloud or external Local Bot API Server.
- Text, photo, video, and file (`sendDocument`) fallback.
- Media groups with per-item results.
- Streaming upload, cancellation, layered timeouts.
- SQLite persistence, `UNKNOWN` results, manual re-send.
- Bot-isolated `file_id` cache.
- Windows Credential Manager adapter.
- Settings page, send status, diagnostics.
- Automatic send after archive, only once explicitly enabled.

### 3.2 Out of scope for the first version

- Receiving updates, commands, callback queries.
- User-account login and any TDLib Rust binding.
- Automatic Docker/WSL2 installation.
- Desktop-managed Bot API Server lifecycle.
- Automatic transcoding or splitting of oversized files.
- Paid broadcasts.
- Arbitrary URL media input.
- Automatic remote message deletion.
- Guaranteed remote exactly-once delivery.
- Server-local path upload enabled by default.

## 4. Verified upstream references

Verified 2026-10-01 via the GitHub API unless stated otherwise. "Not archived" does **not** mean "recently released" or "safe to pin"; each row records what was actually checked.

| Project | Checked state | How this plan uses it |
|---|---|---|
| `tdlib/telegram-bot-api` | Not archived; `master`; latest commit `e3e9dd8` (2026-08-25); BSL-1.0 | Authoritative protocol and server baseline |
| `teloxide/teloxide` | Not archived; MIT; recent push 2026-09-15 | Reference for endpoint/request/error modelling only; no framework dependency |
| `aiogram/telegram-bot-api` | Not archived; **no license detected via API**; recent push 2025-04-15 | Container/deployment reference only; **not** "actively maintained" on this evidence |
| `bots-house/docker-telegram-bot-api` | Not archived; MIT; recent push 2024-07-16 | Secondary container reference |
| `avbor/docker-telegram-bot-api` | Not archived; MIT; recent push 2026-08-28 | Reference for a patched server variant when a proxy is required |
| `unigramdev/unigram` | Not archived; `develop`; recent push 2026-09-24; GPL-3.0 | Receiving-client acceptance target and file-naming input |

### 4.1 Facts that shape the design

- The official server's `--local` mode is documented to raise the upload limit to 2000 MB, allow unlimited downloads, and accept server-local paths / `file://` URIs. **Treat 2000 MB as a server capability ceiling, not a per-media-type guarantee**, and do not write "2 GiB verified" before it is measured.
- The official server listens on HTTP port `8081` by default; cross-machine HTTPS requires separate TLS termination.
- Bot API media rules differ per method: photo size limits, text/caption lengths, album item counts, and `file_id` bot-scoping and type-scoping are **separate** from the cloud 50 MB upload limit. Do not apply one rule to another.
- `reqwest` is already pinned at `0.13.4` in `crates/xarchive-telegram/Cargo.toml`. Do not downgrade to `0.12` to copy example code.
- `tokio` and `tokio-util` exist in `Cargo.lock` **only as transitive dependencies**. They are not approved direct dependencies of `xarchive-telegram` until that is recorded.

### 4.2 Issue #816 is not evidence of a confirmed local-path defect

The upstream report about `file://` returning "invalid file HTTP URL specified" has maintainer replies pointing at URI form, missing `--local`, and internal `inputFileRemote` vs `inputFileLocal` selection ([`tdlib/telegram-bot-api#816`](https://github.com/tdlib/telegram-bot-api/issues/816), read 2026-10-01).

So the reason to defer server-local paths is **path-mapping, security and version-specific verification cost**, not an assumed upstream bug. The issue stays as a regression scenario source.

## 5. Unigram receiving-side references

Checked 2026-10-01. **An open issue does not prove the behavior exists in the Owner's installed version, and a closed issue does not prove every scenario passes.** These are test-scenario sources.

| Issue | State | Planning effect |
|---|---|---|
| [#3244](https://github.com/unigramdev/unigram/issues/3244) bulk download overwrites same-named files | open (reported on 11.8; mostly file messages) | Add stable, distinguishable upload file names; add bulk-download test. Not a fix to Unigram's downloader |
| [#3453](https://github.com/unigramdev/unigram/issues/3453) multi-quality video stalls on HDR | open (WebView2/HLS path) | Video acceptance adds sustained playback, seek, quality switch, and applicable-hardware recording |
| [#3299](https://github.com/unigramdev/unigram/issues/3299) NVIDIA RTX video enhancement black screen | open (upstream `blocked`) | Record GPU, driver, HDR and enhancement state; separate file vs client-rendering causes |
| [#2490](https://github.com/unigramdev/unigram/issues/2490) gallery broken for threads with an album root | open (older report) | Test normal albums and comment threads separately; do not disable all albums |
| [#3306](https://github.com/unigramdev/unigram/issues/3306) HEVC MP4 upload hangs | closed (Unigram's own upload path) | Does **not** justify changing XArchive's send protocol; keep an HEVC receive test |
| [#3292](https://github.com/unigramdev/unigram/issues/3292) crash downloading with insufficient disk space | closed (receiving client) | Add receiving-side disk pre-check; client save failure is not a send failure |

### 5.1 Diagnostic process for "sent successfully but Unigram looks wrong"

1. Confirm the Bot API success result and chat/message id.
2. Confirm Unigram is viewing the correct account and target.
3. Check whether the media is still downloading and whether auto-download is off.
4. Check disk space, format and any reported error.
5. If needed, compare the **same message** in another client, or download the file and compare it.
6. Only then classify as project, server, client, or environment.

**Never classify an external defect from a single upstream issue.**

Unigram's own diagnostics documentation describes enabling TDLib logs and warns against sending logs to untrusted parties ([`Documentation/TDLib-logs.md`](https://github.com/UnigramDev/Unigram/blob/develop/Documentation/TDLib-logs.md), read 2026-10-01). Therefore: collect logs only with user consent, minimise what is captured, never auto-package the whole `LocalState`, and never upload account logs or session material to a public issue.

XArchive will not change Unigram's WebView2 configuration or treat an XArchive WebView2 setting as a fix for a Unigram playback problem.

### 5.2 Licences

Unigram is GPL-3.0. This plan reuses **behaviour and test scenarios** only. No player or downloader code is copied. If code reuse is ever wanted, it needs a separate licence review before merge.

## 6. Work packages

Complexity: S / M / L / XL. Owner is the primary implementer; validation ownership follows §8.

### TG-00 — Scope restoration, contract and source freeze (Cross-platform, M)

Status: `IMPLEMENTED` — this document exists, the scope is re-opened, and the queue
rows `WQ-TG-001`…`WQ-TG-UNI-08` exist as `NOT_RUN`.

1. Create this plan; distinguish paused history from the re-opened scope.
2. Freeze the supported Bot API methods and server version for this batch.
3. Approve "archive and send are decoupled" (§2.1).
4. Define config, send-state, error-classification and event contracts.
5. Register external sources and licences (§4, §5, and [`../references/external-sources.md`](../references/external-sources.md)).

Done when: no remaining "Telegram must succeed before download" ambiguity in docs or code; first-version scope, acceptance targets and next Owner are explicit; no published release record is rewritten to claim Telegram support.

### TG-01 — Configuration, credentials and endpoint safety (Cross-platform design; Windows credential adapter; M)

Status: `PARTIAL` — the shared contract is implemented and unit-tested in
`xarchive-telegram`, and the transport now consumes it through
`ReqwestTelegramTransport::with_api_endpoint()`; the Desktop config contract includes a
rotation-resume policy field, but production rotation behavior is not implemented.
Runtime/UI wiring and the Windows credential adapter are still `PLANNED`.

Desktop configuration implemented (`desktop/src-tauri/src/config.rs`): `TelegramConfig`
carries `enabled`, `endpoint_mode`, `api_base`, `chat_id`, optional `message_thread_id`,
`auto_send_on_archive`, upload mode, connect/upload-processing timeouts, the last verified
capability record and a `revision`. `AppConfig::validate()` enforces the endpoint policy for
the enabled case; a document written before these keys existed loads as **disabled**
(`#[serde(default)]`). There is no token field: presence is read from the `SecretStore`
(`bot_token_present()`), the frontend projection (`TelegramSettings`) carries only a flag,
and a settings change advances `revision` while dropping the capability record so queued
items cannot be silently redirected. The `credential_rotation_resume_policy` field is only a
configuration contract; it does not yet track credential generations, activate tokens, or
authorize/resume queued plans. The Windows Credential Manager adapter and settings UI are still
Batch B. Shared secret orchestration now exposes `replace_bot_token()` and `delete_bot_token()`
over the `SecretStore` abstraction: replacement validates the candidate and obtains a nonempty
stable bot identity before mutating the store, so failed verification leaves the prior credential
intact. Atomic replacement on store failure remains the platform adapter's responsibility. This
is a cross-platform policy seam, not a Windows secret-store implementation; the identity is not
persisted as a credential generation and no queued work is resumed here.

Non-sensitive configuration to add (the endpoint half of this list already has a shared-contract implementation — see below):

- `enabled`
- `endpoint_mode` (`cloud` / `local`)
- `api_base`
- `chat_id`
- optional `message_thread_id`
- `auto_send_on_archive` (default false)
- request/connect/upload-processing timeouts
- upload mode
- last-verified server capability record

Sensitive configuration:

- Bot Token stored through the existing `SecretStore` abstraction.
- Config file, SQLite and frontend state hold only a reference or a "present" flag.
- The frontend never reads the full token back.
- Extend `SecretStoreError` with real variants (store unavailable, access denied).
- API ID / API Hash belong to the external server deployment and are **not** stored by Desktop in the first version.

Endpoint policy:

- Cloud: HTTPS.
- Local: first version accepts only an explicit **loopback HTTP** address.
- Reject URLs carrying credentials, query, fragment, or arbitrary extra path.
- Disable automatic redirects so a token-bearing request cannot be forwarded to another origin.
- Local requests go **direct**, not by "the environment happens to bypass the proxy".
- Reject any other unsupported cleartext remote address.
- Any verified capability record is invalidated on endpoint/bot/config change.

Acceptance: the token never reaches logs, error URLs, task events, the database, or a diagnostics export; older configs without these keys load as disabled.

Shared-contract implementation already landed (`crates/xarchive-telegram/src/lib.rs`,
40/40 unit tests PASS):

- `SecretStoreError::Unavailable`/`AccessDenied` describe the failure without carrying the secret.
- `EndpointMode` (`cloud`/`local`) with strict `parse()`; unknown strings return `None`.
- Validated `TelegramEndpoint::parse()`: cloud requires HTTPS; local requires HTTP on an
  explicit loopback host with a port; both reject userinfo, query, fragment, and any path
  other than `/`; a trailing slash is normalised.
- `TelegramError::EndpointNotSecure`/`EndpointNotLoopback`/`EndpointMalformed` so callers
  can distinguish policy violations from malformed input.
- `TelegramEndpoint::method_url()` assembles `/bot<token>/<method>` at the last moment, so
  the token never lives in a stored endpoint string.
- The endpoint contract now has a production consumer: `with_api_endpoint()` accepts a
  validated `TelegramEndpoint` (cloud HTTPS; local loopback HTTP always pinned direct — a
  proxy is never applied to loopback). The legacy raw-string constructors keep the old
  HTTPS/test-only rule, so `with_endpoint("http://…")` still fails with `InvalidEndpoint`.

### TG-02 — Production transport and streaming upload (Cross-platform, L)

Status: `IMPLEMENTED (shared crate layer)` — the transport API below exists and is unit
tested (crate suite 48/48 PASS). Its production caller is the Desktop send service
(`desktop/src-tauri/src/telegram_send.rs`, TG-06); real-send and large-file evidence still
need a running Local Bot API Server on Windows (`WQ-TG-*`).

- Keep reqwest `0.13.4`; do not downgrade.
- Introduce a **localised async transport for uploads** rather than converting the whole executor to async: the Tauri runtime drives the send task; async file reads with bounded buffering; explicit cancellation and timeouts; never hold the database or `RuntimeState` lock across a network `await`.
- Register `tokio`/`tokio-util` as **direct, feature-scoped** dependencies of the crate before use.
- Photos and videos both use a streaming file source.
- Prefer a length-known multipart part when the length is known.
- Re-open and re-verify the file if the request body must be rebuilt.
- Never read a whole video into memory.
- Bound the response size.
- Extract message / media / album results from the success response; do not only check the HTTP status.

Layered timeouts: connect timeout; request-body stall; server-processing wait; optional overall deadline; user cancellation. The existing 30-second control-request timeout must not govern large sends.

Progress semantics: `queued → checking file → uploading to Bot API → awaiting Telegram result → confirmed`. "Uploaded 100%" is never shown as "message sent". Server-local-path mode shows stages, not a fabricated percentage.

Shared implementation landed (`crates/xarchive-telegram/src/lib.rs`):

- `ReqwestTelegramTransport::send_upload()` — a localised async upload: the file is
  re-opened and re-verified against `expected_size`, then streamed from disk in 64 KiB
  pull-based chunks (`TrackedUploadStream`) into a length-known multipart part; a whole
  video is never read into memory. Photos/videos/documents select the method and field
  through `MediaKind`.
- Layered timeouts: `UploadTimeouts` (connect / body-stall / server-processing / optional
  overall deadline) raced against the send by `upload_watchdog`, each classified into its
  own `TelegramError` variant; the 30 s control-request timeout never governs uploads.
- Cancellation: `tokio_util::sync::CancellationToken` settles as `UploadCancelled`; a
  request that may already have been sent is never auto-retried — the caller reports
  `UNKNOWN` instead (TG-04 semantics).
- Progress: `UploadStage` (`Queued → CheckingFile → Uploading → AwaitingResult →
  Confirmed`); `Confirmed` is emitted only after a parsed `ok: true`, so "uploaded" is
  never shown as "sent".
- Response bound: `TELEGRAM_MAX_RESPONSE_BYTES` (1 MiB) applies to the async upload path
  and the blocking control path; `ok: true` is checked rather than only the HTTP status,
  and `result_message_ids()` / `result_file_ids()` extract message, media and album
  results.
- Endpoint policy: redirects disabled everywhere (the token lives in the URL); the upload
  client is built per send with its own connect timeout and the control path's proxy
  policy; local endpoints go direct.
- Albums: `send_media_group_attempt()` streams a whole `sendMediaGroup` as one
  `multipart/form-data` request. Every file part is length-known and pull-based, so an
  album of videos is never buffered whole; an item whose `file_id` is already cached for
  this bot is sent as that value instead of being uploaded again. All items are opened
  and verified **before** any bytes move, so a later unreadable file cannot leave a
  half-sent album. `AlbumProgress` aggregates per-item progress and only reports the body
  finished when the last item ended; `album_watchdog()` applies the same layered
  deadlines as a single-file body. An item count outside 2–10 is refused before a request
  is built.

Still open for TG-02: real-send and large-file evidence (`WQ-TG-*`), which need a running
Local Bot API Server on Windows. The Desktop send service now consumes this transport
(TG-06).

### TG-03 — Formatting, media classification and send planning (Cross-platform, M)

Status: `PARTIAL` — the classification, naming, album-bound, and send-plan helpers are
implemented and unit-tested; the planner has no production caller yet.

- One Tweet yields a stable send plan.
- Metadata and media keep their association.
- Long text is split while preserving order.
- First version sends plain text; avoid extra rich-text escaping complexity.
- Photo/video that does not meet display rules can be sent as a file, per user policy.
- Never silently compress, transcode or delete originals.
- Over-limit means: keep the local archive and show a clear error.

Media-group rule: the album item range is 2–10, so the current `media_groups()` chunking is **not** directly sendable — a trailing group of one must use a single-media method instead.

Display vs original-file modes:

| Mode | Purpose | Behaviour |
|---|---|---|
| **Display (default)** | Browse photos/videos directly in Unigram | Photo/video/album methods; no byte-identical download promise |
| **Original file (optional)** | Preserve and verify original content | File message, explicit name, download-then-SHA-256 acceptance |

Stable upload file name (to reduce #3244 collisions):

```text
x_<tweet_id>_<media_index>_<content_hash_prefix>.<extension>
```

Rules: never a generic `video.mp4`; never expose an absolute local path; never rename the archived file; deterministic handling of length/illegal characters/collisions.

Shared-contract implementation already landed (`crates/xarchive-telegram/src/lib.rs`,
40/40 unit tests PASS): `TELEGRAM_ALBUM_MIN_ITEMS = 2` pins the album lower bound;
`MediaKind` (`photo`/`video`/`document`), `UploadMode` (`display`/`original_file`), and
`MediaGroupSend` (`Album`/`Single`) model the display-vs-original-file distinction;
`classify_media()` lets the declared MIME win with the extension as fallback and treats
`image/gif` as a non-photo; `stable_upload_file_name()` emits
`x_<tweet>_<index>_<12-hex>.<ext>` with a `.bin` fallback for a missing extension;
`media_send_plan()` promotes any trailing group of one into `Single`, so the plan never
emits a one-item "album". `TELEGRAM_CAPTION_LIMIT` (1024) and the cloud/local upload
ceilings are recorded as capability ceilings, not per-media-type guarantees. The planner
has no production caller yet.

Video compatibility: first-round samples use H.264/AAC MP4 as a **baseline to verify**, not a guarantee. HEVC/10-bit/HDR and other containers form an extended set. `supports_streaming` follows the real media, not a blanket `true`. A client black screen never triggers transcoding, re-send or overwrite.

### TG-04 — Persistence, dedup, rate limiting and recovery (Cross-platform, L — highest risk)

Status: `IMPLEMENTED (shared layer)` — migration `0007_telegram_outbox.sql`, the
`TelegramOutboxStore` contract, its SQLite implementation, the retry/decision rules and the
attempt driver are unit-tested (`xarchive-storage` 54/54, `xarchive-telegram` 46/46 PASS).
The Desktop runtime that schedules those drivers is still unwired.

Why the existing shape is insufficient: dedup reads a `SENT` row and then separately writes `PENDING`, which does not stop two workers, does not isolate bots, cannot express "Telegram accepted but the client lost the response", and cannot store multi-message album results.

Proposed states (names to be aligned with existing job/event conventions):

```text
QUEUED → IN_FLIGHT → SENT
                  ├─ RETRY_WAIT
                  ├─ FAILED_PERMANENT
                  └─ UNKNOWN
QUEUED → CANCELLED
```

Add an **additive migration** (never edit `0002`): bot identity, target/topic, archive/media reference, plan version and order, request fingerprint, atomic claim/lease, attempt and next-retry, confirmed message and media results, redacted error, and the reason a result is `UNKNOWN`. Legacy rows without bot identity are excluded from a new bot's cache and from automatic re-send.

Retry policy:

| Situation | Policy |
|---|---|
| Connection failed before the request was sent | Bounded backoff |
| Explicit 429 | Read and persist the server `retry_after` |
| Auth/permission error | Stop; require correction |
| Explicit media-parameter error | Correct, or a deterministic fallback |
| Request may have been accepted but the response was lost | `UNKNOWN`; **no automatic re-send by default** |
| User cancels after the request was sent | Record cancel intent and uncertainty; never promise remote undo |

The Bot API error `parameters` carry `retry_after` and migration targets; the transport must retain them instead of collapsing to "HTTP status 429".

**Acceptance principle: local idempotency does not equal remote exactly-once.** For `UNKNOWN`, offer "keep for review" and "re-send after confirming a possible duplicate", and never present a generic retry as a no-duplicate guarantee.

Shared implementation landed:

- Additive migration `0007_telegram_outbox.sql` (0002 untouched): `telegram_outbox` with
  bot identity, target/topic, archive/media reference, plan position, `config_version`,
  request fingerprint, claim/lease, `request_started` fence, attempt + `next_retry_at`,
  confirmed message and per-item results, redacted error and the `UNKNOWN` reason.
- `TelegramOutboxStore`: idempotent `enqueue_outbox`, atomic `claim_due_outbox` /
  `claim_outbox`, claim-fenced writes that fail with `StaleClaim`, `cancel_outbox` (only
  before the send), `recover_outbox_claims` and `list_due_outbox`. Two workers cannot claim
  the same row, and a worker whose lease expired cannot overwrite newer facts.
- Retry policy: `classify_send_failure()` maps the error plus the transport's own request
  progress onto `SendFailure` (retry / permanent / media-correction / unknown / cancelled),
  honouring the Bot API `parameters.retry_after` (retained verbatim on `TelegramError::Api`,
  which also fixes non-2xx JSON errors previously collapsed to "HTTP status"), and
  `decide_outbox_transition()` maps that onto the next durable transition with bounded
  backoff (5 s doubled per attempt, capped at 30 min).
- Recovery: an expired claim whose request never started returns to `RETRY_WAIT`; one whose
  request started becomes `UNKNOWN` with `claim_lease_expired` and is never re-sent
  automatically. `UNKNOWN` rows are excluded from `list_due_outbox`; only a deliberate
  `claim_outbox` after review can re-send them.
- Legacy rows without bot identity are invisible to the outbox and the cache, and are never
  auto-resent.
- Attempt driver `run_claimed_attempt()`: fences with `mark_request_started`, awaits the
  transport, then persists exactly what the shared classification decided (retry with the
  scheduled timestamp, `UNKNOWN` with its reason, `FAILED_PERMANENT`, clean `CANCELLED`, or a
  media-correction row). A lost claim returns `RunAttemptError::StaleClaim` **before** the
  request is sent, so a worker that lost its lease cannot send or overwrite newer facts. The
  transport side (`send_upload_attempt()`) reports the confirmed message/file ids on success
  and derives the request progress from its own body tracker on failure.

### TG-05 — `file_id` cache and file consistency (Cross-platform, M)

Status: `IMPLEMENTED (shared layer)` — key type, record type, `FileIdCacheStore` (including
`delete_file_id()` for the one sanctioned eviction), its SQLite implementation, the
conservative invalidation predicate and the representation-version constant are unit
tested. The Desktop reuse path is wired in `telegram_send.rs`: a cached photo or video is
sent by identifier with no upload body, and `MediaCorrection` evicts the entry so the next
attempt uploads the original file. Still unwired: the settings UI and the Windows secret
store.

Cache key: `bot identity + SHA-256 + media kind + representation version`. Store `file_id`, `file_unique_id`, size and confirmation time. `file_unique_id` is identification only, never a send parameter.

Rules: prefer the hash already computed during archiving; re-check the file before sending; write the cache only after a confirmed success; store album results per item; keep or drop a cache entry based on bot identity after a token change; fall back to the original file **only** on an explicit invalid-file-id error; never treat a permission or network failure as cache invalidation; never let a file change or retry break request length/content consistency. When cached reuse conflicts with an expected file name, choose either to keep the original name or to re-upload — do not silently promise both.

Two consequences of the Bot API that the Desktop path follows: `file_id` is scoped to the
bot that uploaded it, which is why the cache key starts with the bot identity; and a
cached **document** has no JSON control method, so a cached document is re-uploaded rather
than sent by identifier.

### TG-06 — Desktop integration and settings (Cross-platform business; Windows native/GUI; L)

Status: `PARTIAL` — shared business contracts and send helpers are implemented; production
runtime integration is not. The shared business model includes `TelegramConfig` /
`TelegramSettings` (frontend projection with a presence flag only), the config revision that
binds queued items, `outbox_projection()` + `SendProjection::label()` wording that never
claims receipt/read state, the official deep-link rule (`message_link()`), the send planner
(`plan_media_sends()` / `plan_text_send()` with a stable `idempotency_key` and a content
`fingerprint()`), and `Database::list_outbox_for_tweet()` as a data source for a future task
send-state projection. The Desktop send helper core is implemented as well
(`desktop/src-tauri/src/telegram_send.rs`): `auto_send_enabled()` gates automatic sending on
both switches **and** a target; `queue_archive_sends()` accepts facts for a finished archive
and queues the plan idempotently; `run_due_sends()` executes one bounded due batch when called;
`recover_expired_claims()` recovers expired claims when called. These functions are not yet
called by the production archive commit path or Desktop startup/runtime. Still outstanding:
the archive enqueue call site, Tauri commands/configuration bridge, runtime claim-loop and
startup recovery wiring, task-status UI, Windows Credential Manager/native integration, and
deployment/real-send acceptance. The shared helper layer is implemented; this does not mean
all non-Windows product integration is complete.

**Resumed-work checkpoint (2026-10-03):** TG-06 is active again. The current branch contains
Scheme B outbox identity/idempotency and durable payload snapshot — the
`CROSS_PLATFORM_CHANGE_REQUIRED` decisions below — but the production runtime is still not wired.
**Scheme B contract decisions (resolved `CROSS_PLATFORM_CHANGE_REQUIRED`):** migration
`0008_telegram_outbox_snapshot.sql` rebuilds `telegram_outbox` so send identity is
`UNIQUE(bot_identity, target_scope, idempotency_key)`, where `target_scope` is
`<chat_id>:<topic-or-none>`; two bots or two topics can therefore never collide on one key.
`enqueue_outbox()` is idempotent only when the stored `request_fingerprint`, payload schema and
payload JSON all match; a same-key different-plan enqueue returns
`SendStateError::IdempotencyConflict` instead of silently reusing a prior row. Each row carries a
versioned, immutable `payload_json` snapshot (`payload_schema_version = 1`) of the planned
`SendPayload`, whose media references are archive-root-relative paths that must stay inside the
Tweet's committed archive directory; an unsafe or machine-local path is refused with
`SendStateError::InvalidPayloadPath`. Legacy rows keep their state but have a NULL snapshot, which
explicitly means they cannot be resumed until manually replanned. `restore_payload_from_entry()`
rebuilds a payload only from the snapshot, never from current settings; `resolve_archived_payload()`
then maps each reference back to a recorded media fact (internal outbox row id -> external Tweet
id -> archive facts) and resolves it through `FileStore`, rejecting traversal, symlink/escape, a
stale directory, or a size/digest change. This closes the earlier gap in which a row stored too
little content to send and resolving it from the current config could redirect queued work.
Still outstanding for TG-06 (Scheme B remainder): stable bot identity versus credential
generation policy, recoverable archive-complete/enqueue consistency, production archive enqueue,
lease-heartbeat caller, startup claim recovery, Tauri commands/settings bridge, task-status UI and a
single-instance sender loop. These are cross-platform product-development tasks, not Windows-only
validation. This round did not add an archive-intent schema: filesystem rename and SQLite updates
cannot share one transaction, and production enqueue must wait for a tested recovery/reconciliation
protocol rather than claim false atomicity. Windows Credential Manager, native runtime/packaging,
Local Bot API deployment, controlled real-send and Unigram acceptance remain Windows/real-
environment work. Do not describe the planned helpers as a production send path.

**Second checkpoint (2026-10-03): plan-order dependencies and claim lease renewal.** Two of the
Scheme B remainder items are now implemented in the shared layer.

- *Plan-order dependencies.* `claim_due_outbox()` and `list_due_outbox()` skip a row whose earlier
  `plan_order` sibling of the same `tweet_id` group — same `bot_identity`, same `target_scope` — is
  not `SENT`. The gate is expressed as `NOT EXISTS (… p.plan_order < c.plan_order AND p.state <>
  'SENT')`, so a unit is claimable only when every earlier planned unit of that archive has been
  sent. Rows with `tweet_id IS NULL` have no archive to sequence against and stay independent.
  Because a `FAILED_PERMANENT`/`UNKNOWN`/`CANCELLED` predecessor is not `SENT`, the rest of that
  archive is held back rather than being sent out of plan order; releasing it is a deliberate
  manual decision, not an automatic consequence of one unit failing. This makes plan order a
  property of the durable queue instead of a property of whichever loop happens to run.
- *Claim lease renewal.* `TelegramOutboxStore::renew_outbox_claim(claim_token, lease_until, now)`
  extends the lease of an `IN_FLIGHT` row. It returns `false` — never an error — when the caller no
  longer owns the claim, so a worker that lost its lease can detect it and simply drop its write
  instead of overwriting the reclaiming worker's row. The UPDATE is guarded by `claim_token`, by
  `state = 'IN_FLIGHT'`, and by `claim_expires_at > now`: a lease that already elapsed belongs to
  the recovery path, so its former owner cannot resurrect it. This is the store half of the
  heartbeat; a send path still has to call it while an upload runs, and that call site is listed as
  outstanding above.

New shared tests: `outbox_plan_order_holds_back_the_rest_of_an_archive_until_its_predecessor_is_sent`,
`outbox_unit_that_cannot_be_sent_pauses_the_units_that_follow_it`,
`outbox_rows_without_a_tweet_are_independent_and_still_claim_in_plan_order`,
`outbox_lease_renewal_is_owner_only_and_moves_the_recovery_deadline`, and
`outbox_expired_lease_cannot_be_resurrected_by_its_former_owner`. Note that `tweet_id` is a
foreign key to `tweets`, so these tests seed a real tweet row rather than an arbitrary id.

A queued row can be reconstructed for sending from its own `payload_json` snapshot alone
(`restore_payload_from_entry()` + `resolve_archived_payload()`); `run_due_sends()` still takes a
`resolve` callback so focused transport tests can inject payloads, and an unresolvable payload is
recorded as a plan needing correction rather than being silently skipped.

Settings section (bottom of the settings page, per the existing layout): enable Telegram; bot token write/replace/delete; target chat/topic; cloud/local; local address; test auth; check target; send test message after explicit confirmation; auto-send default off; advanced timeouts; verified server capability. **Do not show an unimplemented "auto select local path" option.**

Task UI shows local archive state and Telegram send state separately: transfer stage, wait/failure reason, re-send, cancel, `UNKNOWN` review. A config change affects only new items; queued items bind to a config version so an edit cannot silently redirect them.

Suggested task wording — `Telegram: send confirmed` / `client display: not judged by the send service`. Do not show "Unigram received/read" unless a separate reliable, authorised implementation exists.

Optional low-coupling message location: when a valid message link exists, offer "open Telegram message" and "copy link"; otherwise keep target and message id. Build links per Telegram's official deep-link rules, never by concatenating unvalidated input. First version does **not** guarantee forcing Unigram to open; it opens via the system association, and Windows acceptance records the real behaviour. Do not change the user's default apps or hardcode an app package/EXE path.

Cloud↔Local switch uses an explicit migration wizard, not a hidden `logOut` inside "test connection". Upstream notes that a switch requires logging out of the cloud first and that returning to the cloud has a 10-minute lockout; the Local instance also has its own `close` semantics.

### TG-07 — External server deployment and Windows acceptance (Windows, L)

First version ships version-constrained deployment instructions and does **not** manage the server: fixed official-server version; optional Docker/WSL2; host port published to loopback only; persistent directory; disk budget and log rotation; private API-hash injection; restart/upgrade/backup/rollback steps; container logs are not automatically public diagnostics.

Verify the two legs separately: `Desktop → Local API` and `Local API → Telegram`. A working Windows system proxy does not prove the second leg.

Windows credential and runtime scenarios: Credential Manager create/read/replace/delete; no plaintext fallback when unavailable; portability after moving the portable directory; sleep/offline/exit/restart; Unicode, long paths, file locks; UI focus/keyboard/narrow-window/DPI; isolated account and controlled target.

### TG-08 — Server-local path optimisation (later batch; cross-platform path contract; Windows/Docker mapping; L)

Only after TG-01–07 are stable. Preconditions: confirmed server version; verified `--local` behaviour; clear shared-directory boundary; server can actually read the file; file retained until the result is confirmed; defined failure/fallback semantics.

Security: model Windows and container paths separately; no string-replacing directories; defend against `..`, symlink and junction/reparse escape; mount the share read-only where possible; encode spaces, non-ASCII, `#` and `%`; never expose the whole user directory; **never** use a mechanical "over 50 MB → local path" threshold. Validate value with real CPU/memory/disk/time measurement, not by assuming one fewer HTTP hop.

## 7. Test and acceptance matrix

| Layer | What must be tested | Done when |
|---|---|---|
| Contract | endpoint, redaction, config migration, request/result | no credential in any failure path |
| Transport | multipart, length, slow response, disconnect, non-JSON, 429 | error classification and cancellation are exact |
| Storage | bot isolation, claim race, migration, legacy rows | no double claim; no reuse of a legacy result |
| Recovery | crash before/at/after the request, before the DB write | `UNKNOWN` never auto-resends |
| Media | single, album, file fallback, text boundaries | order and message mapping correct |
| Performance | large-file streaming | memory does not grow with file size |
| Windows | credentials, file semantics, GUI, restart | real platform evidence |
| Telegram E2E | text, photo, video, album, cached reuse | the target really receives it and the DB matches |
| Large file | >50 MB, near the ceiling, over-limit reject | bound to server version and real file evidence |
| Security | redirects, proxy, logs, directory escape | no credential or unauthorised file leaves |

### 7.1 Receiving-side acceptance (Unigram), three separate layers

| Layer | Acceptance | What it cannot substitute for |
|---|---|---|
| **Send** | Bot API result, message id, DB record | playing in the client |
| **Display** | text, album, thumbnails, playback, link | original-file integrity |
| **File** | download, file name, size, original hash | normal display of every media kind |

A check that passes in another client does not automatically make a Unigram check pass. Where hardware scenarios cannot be run, mark `NOT_RUN` or note the environment — never infer HDR/GPU behavior from an ordinary display test.

Queue identifiers: `WQ-TG-001` … `WQ-TG-009` (credentials, network, text, media, recovery, cache, large file, GUI, security) plus receiving checks `WQ-TG-UNI-01` … `WQ-TG-UNI-08` (version/environment, text, 1/2/10/11-item media, video playback, applicable HDR/GPU, file download/naming, large-file manual download, deep-link behaviour).

Linux ordering: `Targeted → Module → Subsystem`; escalate only when storage migration, executor and send interaction are all touched. Real-send tests use a dedicated bot/target authorised by the Owner; ordinary CI carries no production token and triggers no public send.

## 8. Batches and handoff

### Batch A — shared capability (Cross-platform)

TG-00 … TG-05 plus the shared business part of TG-06. Deliverables: config and state contracts; Local endpoint; transport; formatter/planner; outbox, cache and recovery; mock and migration tests; the Windows manual queue. Then one Git handoff.

### Batch B — Windows integration and real send (Windows)

TG-06 platform part plus TG-07. Deliverables: Credential Manager; UI and native lifecycle; external-server deployment verification; controlled-account real send; large-file, cancel, recovery and diagnostics.

### Batch C — optional optimisation

TG-08. Never a precondition for the first-version send path.

Shared contract/state-machine defects use `CROSS_PLATFORM_CHANGE_REQUIRED`; small changes that preserve an abstraction use `CROSS_PLATFORM_REVIEW_REQUIRED`.

Next development Owner: **Cross-platform Owner**. Real Windows send is the completion gate, but nothing requires it to block Batch A.

## 9. Documentation this plan touches

When implementing, update:

- [`../architecture/job-state-machine.md`](../architecture/job-state-machine.md) — decouple Telegram from the download path.
- [`../architecture/data-model.md`](../architecture/data-model.md) — `telegram_send_attempts` extension and the new migration.
- [`../architecture/runtime-flow.md`](../architecture/runtime-flow.md) — Telegram send flow.
- [`status.md`](status.md) — Telegram row returns to active development.
- [`../references/external-sources.md`](../references/external-sources.md) — new entries.
- [`../validation/windows-queue.md`](../validation/windows-queue.md) — `WQ-TG-*` items.
- [`../status/platform-handoff.md`](../status/platform-handoff.md) — current batch.

### 9.1 Definition of "first version usable"

1. Real target receives text, photo, video and an album.
2. Confirmed message results are fully persisted.
3. Restart and `UNKNOWN` handling pass.
4. Credential and diagnostics redaction pass.
5. Telegram failure never damages the local archive.
6. Local large-file ability has version-bound real evidence.
7. Release notes separate verified, unverified and unsupported scope.

## Windows validation — 2026-10-02 / 1f14cea

Tested shared implementation `1f14cea6859dc1c0ecec164509579cfe4eb15f1a` on native Windows/MSVC via Git checkout. Telegram 48/48 and storage 51/51 PASS (3 Unix-only tests excluded); Desktop lib 175/175 PASS after explicitly binding native Python 3.12.14. First Desktop run 172/175 and Cargo sandbox/network failures are preserved as environment diagnostics in [Windows history](../validation/windows-validation-history.md), with commands, artifact hashes and evidence limits. No source or assertions changed. fmt/docs audit PASS.

This completes Windows module validation of the delivered shared layer, not TG-06/TG-07 product acceptance. `WQ-TG-001`–`009` and `WQ-TG-UNI-01`–`08` remain NOT_RUN/IMPLEMENTATION_NOT_READY; [manual steps §K](../validation/windows-manual-steps.md) retain prerequisites and criteria. Full-workspace/packaging/GUI/real-send regression not run because the selected affected subsystem covers this shared delivery and the production send entry point does not exist.

Next Owner Cross-platform Owner for the already planned runtime/Tauri/settings/task wiring, followed by a runnable Git handoff to Windows for native credentials and real send. No new CROSS_PLATFORM_CHANGE_REQUIRED/CROSS_PLATFORM_REVIEW_REQUIRED or WINDOWS_VERIFICATION_BLOCKING. Validation-record revision is the commit containing this section on `codex/windows-validation-1f14cea`.
