# Current Platform Handoff

## Download history and dashboard continuation — 2026-10-06

State: `CROSS_PLATFORM_IN_PROGRESS`. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`. Source commit:
`6d77b6b`; handoff commit: this entry's commit. Current Owner:
Cross-platform Owner. No formal Windows handoff is made by this in-progress
record. The working tree is committed and pushed so the current progress is
available on GitHub; this does not mean the feature is complete.

### Implemented in this increment

- Added migration `0014_download_task_metrics.sql`, typed download metric and
  paginated-job models, paginated storage queries, metric persistence with
  stale-attempt fencing, and storage tests for pagination and fencing.
- Added Tauri commands for paginated task history and per-job download metrics.
- Wired the Downloads page with 20-item pagination, archive-directory controls,
  persisted-metric display, and navigation from dashboard task summaries;
  removed the storage panel from Settings while preserving its other controls.
- Runtime executor instrumentation is **not implemented**. UI must treat
  unavailable historical or runtime metrics as unavailable; do not infer
  duration or speed from unrelated task timestamps.

### Verification on the pre-handoff working tree

- `cargo test -p xarchive-storage --lib --no-fail-fast`: PASS, 76 tests.
- `cargo check -p xarchive-desktop --all-targets`: PASS.
- `npm run check --workspace desktop`: PASS, with the existing Tauri API
  mixed static/dynamic import warning.
- `npm test --workspace desktop`: FAIL; a UI wiring assertion expects
  `job.last_error_message` to remain in `shared.jsx`. This failure is
  unresolved and must be investigated; do not describe the frontend suite as
  passing.
- `cargo fmt --all -- --check`: FAIL due to formatting differences.
- `git diff --check`: PASS.
- Full regression was not run. Windows native GUI, directory picker, and
  download backend validation were not run and remain
  `WINDOWS_VERIFICATION_PENDING`.

### Remaining work and ownership

1. Cross-platform Owner: repair/review the failing UI assertion and Rust
   formatting, then rerun the affected checks and record results.
2. Cross-platform Owner with platform-owner review:
   `CROSS_PLATFORM_CHANGE_REQUIRED` before changing cross-module executor
   contracts for attempt instrumentation. Instrument gallery-dl and aria2
   attempts consistently and persist trustworthy start/finish, bytes, duration,
   and attempt count; never infer these values from job timestamps.
3. Cross-platform Owner: add/verify UI tests for pagination, empty/loading/error
   states, archive-location uniqueness, dashboard summaries, and navigation.
4. Confirm SQLite navigation remains available after removing the Settings
   storage panel; preserve database/log paths and existing archived files.
5. Windows Platform Owner: validate native GUI/navigation, directory picker,
   and real download backends against a fresh artifact built from the eventual
   handoff commit. These checks are not claimed PASS by this Linux batch.

Next Owner remains Cross-platform Owner for the listed implementation and
shared-contract review. Formal Windows handoff follows after the cross-platform
batch and required Linux verification are complete.

## Cross-platform batch — fd7d834 integrated + 4 non-Windows fixes — 2026-10-06

State: `READY_FOR_WINDOWS`. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`. Merge revision:
`6afb351` (merge of `fd7d834ceaae2fa72bd3eb7d17987d0e02622cf6` into
`5c3efb79d2c7ffe3c84fe1f33821cd94b6c77a49`); this handoff commit on top adds
the cross-platform follow-ups below. Current Owner: Cross-platform Owner.
Next Owner: **Windows Platform Owner**.

### Integrated Windows return (fd7d834, no conflicts)

- Merged `origin/codex/windows-validation-5c3efb79` (`fd7d834`) with `--no-ff`.
  Brings Windows-only PAC adapter repair `44e60e3`, Full `2b98952` build +
  validation docs, and user manual feedback with two scoped FAILs.
- `44e60e3` review: touches only
  `desktop/src-tauri/src/system_proxy_resolver.rs` (a `#[cfg(windows)]`
  module). No shared abstraction, schema, protocol, helper, or dependency
  changed — no `CROSS_PLATFORM_REVIEW_REQUIRED` approval needed for it.
- Requested reconcile per `docs/development/cross-platform-validation.md`
  §16 remains not applicable: that file is navigation-only and has no §16;
  ownership/Git-handoff/validation-policy sources apply instead
  (`git-platform-handoff.md` §16 revision template used).

### Cross-platform follow-ups completed (no Windows environment needed)

All three feedback items triaged; narrow Linux-owned repairs implemented:

1. **WQ-FULL-STATUS-01** (sidebar/panel disagreement + stale port):
   `browser_connection` combines live WebSocket socket state with a 30s
   Named Pipe freshness window, while the panel shows `websocket_*` live
   state — the same snapshot can legitimately disagree after bootstrap
   activity ages out. `save_network_settings`/`set_use_aria2` rebuild the
   executor + transport (possibly a new port) without refreshing the panel.
   Fixes: status copy now names both sources and points at `websocket_*`
   (`commands.rs`); both save handlers call `refreshExtension()` after
   success (`main.jsx`); wiring regression added (`ui-wiring.test.mjs`).
   Full semantic unification (single canonical source) is deferred as
   `CROSS_PLATFORM_CHANGE_REQUIRED` design — needs Windows input on which
   source is authoritative for bootstrap-only activity.
2. **WebSocket close-within-3s FAIL** (`CROSS_PLATFORM_REVIEW_REQUIRED`):
   `handle_websocket_connection` broke out of the loop on `Message::Close`
   without replying; tungstenite queues the close echo on `read` and only
   sends it on the next `read`/`flush`. The peer therefore never saw a
   graceful close. Fix: reply + flush before break, per RFC 6455 §7.1.2.
   Regression: `authenticated_connection_routes_a_browser_request` now
   asserts the server echoes `Message::Close`.
3. **WQ-DL-01/02 cancel attribution**: `production.rs::download_v2` used one
   `"archive download cancelled"` string for both an executor-token trip
   (settings-change rebuild counts) and a worker `Cancelled` event, so triage
   could not separate them. Split into `"interrupted by executor
   cancellation"` (our shutdown) vs `"cancelled by worker"` (worker decided).

### Linux verification on the merged + fixed tree (new-tree evidence only)

- Environment: Linux WSL2 Ubuntu 26.04.1, Rust 1.98.0, Node v26.7.0.
- `cargo test -p xarchive-desktop --lib`: 251/251 PASS (incl. 13/13
  websocket_transport with the new close-echo assertion).
- `cargo test -p xarchive-protocol --lib`: 28/28 PASS.
- `desktop` Node suite: 215/215 PASS (incl. new save-refresh wiring test).
- `node scripts/docs-audit.mjs`: PASS; `git diff --check`: PASS.
- Windows-native build/package/GUI/DPI/keyboard, Full workspace regression,
  installers/releases, real external services: NOT_RUN (incremental scope;
  old-tree results not promoted). `44e60e3`'s 6 extra module tests are
  `#[cfg(windows)]`-gated native fixtures and cannot run on Linux; the 251
  vs 257 delta is expected, not a regression.

### Windows work required (all BLOCKED/NOT_RUN items, none claimed PASS)

- Rebuild Full from this handoff revision; revalidate WQ-FULL-STATUS-01 with
  the new refresh behavior (dedicated browser profile, exact rebuilt Full;
  connect, wait >30s, compare browser/sidebar/WebSocket panel; change
  settings with no active job; verify port + refreshed UI).
- Re-run the controlled WebSocket close probe against the fixed transport;
  confirm close completes within 3s.
- Reproduce WQ-DL-01/02 per mode with the split cancel messages; record job
  ID, mode, worker stderr/event code, cancellation source, final state.
- Remaining: isolated Registry/PAC/WPAD actual-egress M13 (WQ-PROXY-15/16,
  incl. loopback implicit-bypass risk); full DPI/keyboard/busy matrix
  (M10/P6, WQ-PROXY-11/17, WQ-DL-06); fresh Full/real-download/browser/
  Telegram acceptance; Credential Manager ignored test; MSI/NSIS/signing
  NOT_APPLICABLE (portable Full only). Release remains blocked by the
  user-observed FAILs until repaired + revalidated.

---

## Full 2b98952 user manual return — 2026-10-06

[Exact manual results and diagnosis](../validation/windows-full-2b98952-feedback.md): user reports first automatic browser connection/task creation PASS scoped; re-detect/sidebar/current-port consistency FAIL; gallery-dl and aria2 download completion FAIL user-observed. Running Full executable hash verified. Restart/reconnect and cause-isolated download reproduction remain NOT_RUN; existing historical and scoped build results preserved.

Next Owner **Cross-platform Owner** for status-source semantics and configuration-change/executor/transport lifecycle policy (`CROSS_PLATFORM_CHANGE_REQUIRED`, narrow repairs may instead require review). Windows retains native reproduction and exact-build revalidation. State `WINDOWS_VERIFICATION_PENDING`; release blocked by these failures. Documentation input `a104000`; build source `2b98952`, implementation `44e60e3`, Linux handoff `5c3efb79`, return branch `codex/windows-validation-5c3efb79`; handoff revision is the commit containing this entry. Tracked tree clean after commit; local untracked/user data preserved.


## Full package continuation — 2026-10-06

State: `WINDOWS_VERIFICATION_PENDING`. Current Owner Windows Platform Owner; next **Cross-platform Owner** for Git integration/shared WebSocket close review. Source/build/validation `2b98952aeaa717899ebcd13c09f058a28e998e73`; implementation `44e60e369b5c57d3ed66b46fb610dc906a45780f`; return branch `codex/windows-validation-5c3efb79`. Documentation handoff is the commit containing this entry; tracked tree clean after it, local untracked data/artifacts preserved.

Canonical fresh Full portable ZIP built, SHA `06ab9d7973a413f2bbd28d7c81dd83d24daa2e77384f751714c7a4f7affa1e11`; dev candidate, not release or MSI/NSIS. Rust 512 PASS/1 ignored, Node 214+52 PASS, Python 61 PASS; package/ZIP integrity and real bundled component/startup/Sidecar/bootstrap/auth/current-size GUI/download keyboard/restart subsets passed. Controlled WebSocket close-within-3s FAIL retained; investigate shared close/drain behavior under `CROSS_PLATFORM_REVIEW_REQUIRED`, no shared code changed. Real archive/browser automatic reconnect/Telegram/PAC/DPI and remaining lifecycle acceptance NOT_RUN with prerequisites in [exact Full results](../validation/windows-full-2b98952-results.md). [Install/manual continuation](../validation/windows-full-2b98952-manual.md). Windows retains manual/native acceptance after reconciliation.

---

The previous return below identifies the implementation ancestry; this Full continuation is the active artifact state.

## Windows 5c3efb79 return — 2026-10-06

State: `WINDOWS_VERIFICATION_PENDING`. Current Owner: Windows Platform Owner.
Next Owner: **Cross-platform Owner** for Git integration/review; Windows retains native acceptance.

- Input branch: `cross-platform/automatic-pairing-reconcile-20261002`.
- Cross-platform input/handoff: `5c3efb79d2c7ffe3c84fe1f33821cd94b6c77a49` (includes `344ca10`, reviewed `8f7fde6` / `8e0169a`). Fetch preceded checkout; no direct sync.
- Return branch: `codex/windows-validation-5c3efb79`.
- Windows implementation/validation: `44e60e369b5c57d3ed66b46fb610dc906a45780f`.
- Documentation handoff: the commit containing this entry / return branch HEAD. Tracked tree clean after commit; existing local untracked dependencies/data/artifacts preserved.

Windows-only adapter now distinguishes configured/failed PAC and WPAD from loaded scripts, applies scheme-specific environment precedence and uses explicit WinHTTP PAC execution. Shared predicate/contracts/dependencies unchanged.

Results: Desktop module 257 PASS / 1 ignored; native WinHTTP PAC fixtures and client/child boundaries PASS scoped; fresh debug native build/startup/Settings/proxy summary PASS scoped. Initial module FAIL retained with Python prerequisite and loopback assertion details. WinHTTP implicit loopback DIRECT despite throwing PAC remains an integration risk; fixture PASS does not close WQ-PROXY-15/16.

Fresh executable SHA-256: `660388A7F828E771B8B8B851A238177F3C6CA6210A9CF7852D5616278C4174E8` (debug, not Full).

[Exact results, capability and manual queue](../validation/windows-5c3efb79-results.md).
Current queue: [windows-queue.md](../validation/windows-queue.md).
History: [windows-validation-history.md](../validation/windows-validation-history.md).

Cross-platform follow-up: integrate/review Windows adapter; decide/document implicit-bypass acceptance. A shared contract change, if needed, requires `CROSS_PLATFORM_CHANGE_REQUIRED`; no shared change occurred here.
Windows follow-up: isolated Registry/PAC/WPAD actual-egress M13; remaining DPI/keyboard/busy matrix; fresh Full/real-download/browser/external-service acceptance. All remain NOT_RUN/pending with explicit prerequisites in the batch record. No whole-row Windows PASS or WINDOWS_BLOCKING claim.

Requested cross-platform-validation §16 is absent (navigation-only); used linked ownership/validation rules and git-platform-handoff §16 revision template.

---

Earlier checkpoints below are historical; only the current batch above defines active ownership/status.

## Windows 130affaf validation return

State: `CROSS_PLATFORM_REVIEW_REQUIRED`; full platform acceptance remains `WINDOWS_VERIFICATION_PENDING`.
Input branch `cross-platform/automatic-pairing-reconcile-20261002`, exact input/handoff `130affafdff512934d7c135b3eb61a2344d27b10`.
Return branch `codex/windows-validation-130affaf`. Shared fixture repair `8f7fde61b4fd2a16c9f96444990cf8fcb57302a2`; shared Toggle Enter repair/final build source `8e0169a83313eec98b57a793d71254689c4fcc1d`. Return revision is the documentation commit containing this entry.
Current Owner at execution/return: Windows Platform Owner; next Owner: Cross-platform Owner for Git reconciliation and review. Windows retains remaining native/Full/manual acceptance. Tracked tree clean before Git alignment; untracked local Windows data preserved; no direct sync.

Native proxy 37/37, core proxy 27/27, Python protocol/entrypoint 27/27 and supervisor 11/11 PASS. Rust protocol at input FAIL 27/28: shared JSONL loop still used a POSIX path; narrow fixture instantiation repair -> 28/28 PASS. Frontend targeted 79/79 at input, 80/80 after Enter repair. Fresh Windows build/startup and independent packaged worker five-capability/download_started probe PASS scoped; Full package/actual download completion NOT_RUN.

New download page navigation, Windows aria2 control presentation and Space persistence PASS on initial 8f7fde6 artifact; Enter FAIL against P6. 8e0169a final artifact retains true after restart and Enter switches/persists false PASS scoped. Remaining Settings navigation/GUI checks BLOCKED / COMPUTER_USE_UNAVAILABLE after two activation failures; full DPI/busy/keyboard/real aria2 acceptance remains open.
Final Desktop SHA256 `3A8CC8AFD3EEA82259F001B62754E01FB97B4C950BDFE7F745AABCE86D77EC46`; worker SHA256 `2098750E0F61A8BD1122FDD1CB22DE8CF5EA4EAE8BD378ED1A54C99E9CB15993`. Detailed commands, identities, original failures, environment and manual queue: [batch record](../validation/windows-130affaf-results.md), local evidence `validation-artifacts/windows-130affaf/`.

CROSS_PLATFORM_REVIEW_REQUIRED: integrate/review both small shared repairs. Native PAC static finding: adapter tests config.pac.is_some(), but official failed download yields pac=None while retaining pac_url/ErrorDownload, so the false boolean defeats the new fail-closed guard; WPAD states/evaluation distinction need adapter-level review. Actual failed-PAC/egress test remains NOT_RUN; do not promote helper PASS to runtime acceptance. Prior timeout/ACL/reconnect/advisory items remain open at original identities. Next Windows checks use reviewed source, controlled isolated PAC/WPAD fixtures and P6/M recipes.

---


## Download settings page and proxy UI continuation — 2026-10-05

State: `READY_FOR_WINDOWS`. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`, on top of handoff
`fa46a53`. Working tree is committed; there is no uncommitted state. Current
Owner: Cross-platform Owner. Next Owner: **Windows Platform Owner**.

### Cross-platform implementation

- Completed the settings render repair and UI polish carried in the working tree:
  proxy diagnosis form now uses the shared form wrapper, long proxy values wrap
  safely, explanatory status is not presented as a successful route, backend
  coverage clauses are separated, and the Bootstrap panel has local spacing.
- Added the dedicated `内容下载` page and sidebar navigation. The existing
  `use_aria2` setting and Tauri command remain the source of truth; aria2 manager
  controls are shown only on Windows, while other platforms receive an explicit
  explanation. Settings retains the Sidecar gallery-dl path controls.
- Added regressions for page render/platform branches and sidebar/prop wiring.
  `icon.jsx` was not changed: the proposed Extension path was not geometrically
  verified and must not be inferred correct.

### Linux verification (this batch, on the merged and committed tree)

- Desktop targeted render/wiring/state tests: 79/79 PASS.
- Desktop Node suite: 213/213 PASS.
- Desktop Vite production build: PASS; existing mixed static/dynamic Tauri API
  chunk warning remains informational.
- `cargo test -p xarchive-desktop --lib`: 251/251 PASS.
- `node scripts/docs-audit.mjs` and `git diff --check`: PASS.
- Windows-native build, packaged artifact validation, WebView2 GUI/DPI/keyboard,
  persistence across Windows restart, and real aria2/process behavior: **NOT_RUN**.

Environment: Linux WSL2 (Ubuntu), Linux Rust target `x86_64-unknown-linux-gnu`;
Windows PowerShell interop is present, but no Windows Tauri artifact or project
WDIO/Tauri driver process was available to bind to this batch's source. This does
not establish Windows product failure. No Windows execution is claimed PASS. Follow
`WQ-DL-01..08`; WQ-DL-06 and its reproducible steps in
[`../validation/windows-manual-steps.md`](../validation/windows-manual-steps.md)
now explicitly cover navigation, aria2 platform UI, DPI, keyboard and persistence.

### Next owner / handoff

Windows Platform Owner: build from the committed source SHA of this handoff and
execute the Windows queue, beginning with WQ-DL-06 UI/navigation acceptance and
the applicable runtime rows. No `WINDOWS_BLOCKING` is identified; remaining
checks are deferred Windows-owned validation and do not block independent
cross-platform work.

---

## Bypass wildcard parity and non-Windows completion — 2026-10-05

State: `READY_FOR_WINDOWS`. Branch: `cross-platform/automatic-pairing-reconcile-20261002`,
on top of the reconciled 6cf4cc8 handoff `0fec592`. Current Owner: Cross-platform Owner.
Next Owner: **Windows Platform Owner**.

One further product defect was found and fixed while completing the Plan's
non-Windows scope. `ProxyBypass::matches` documented `*.domain` support, but
`matches_host` only stripped a leading dot, so `*.example.com` degraded to a
literal comparison and **silently matched nothing**: a user who typed the
wildcard spelling into `NO_PROXY` or a bypass list got traffic proxied that they
believed was excluded, with no error anywhere. The two spellings are now
distinct rather than normalized — `.domain` covers the apex and its subdomains,
`*.domain` covers subdomains only — and the doc comment was corrected, since it
also claimed a bare hostname matches its subdomains, which the code never did.

The Plan's remaining completion criteria were audited rather than assumed. Both
validators use the host's own `is_absolute`; credential redaction is applied on
every proxy-bearing surface and aria2 logs no proxy value; `Manual` reaches the
child boundary through the `Set` path and `Direct` through `ClearProxyEnvironment`.

Linux validation: `cargo test --workspace` (21 targets, xarchive-core proxy 27),
workspace all-target Clippy with `-D warnings`, `cargo fmt --check`, the Sidecar
pytest suite, the Desktop Node suite, the Vite build, the docs audit and
`git diff --check` all PASS. The wildcard tests are mutation-checked: disabling
the `*.` branch fails them.

`WQ-PROXY-18` and manual step M15 cover the wildcard behavior, which needs a real
Windows bypass list. Nothing in this entry was validated on Windows.

## Cross-platform reconciliation of the 6cf4cc8 return — 2026-10-05

State: `READY_FOR_WINDOWS`. Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
Input/handoff `6cf4cc8`; reconciled return `d201deb2f69974025285b8068797ba31b2c714dd` from
`codex/windows-validation-6cf4cc8`, merged through Git. This entry is the handoff revision;
Windows must build from the exact SHA of the commit containing it.
Current Owner: Cross-platform Owner. Next Owner: **Windows Platform Owner** for native
runtime, manual and Full acceptance. Platform acceptance remains
`WINDOWS_VERIFICATION_PENDING`; nothing in the returned record was upgraded.

Both returned repairs are integrated and reviewed:

- `dc5ef1f` — the Windows adapter compile repair. Accepted: these are the compile
  errors Linux could not detect, and the Windows evidence is the only source for
  them. The module still cannot be compiled on this host.
- `297a237` — the shared Settings render repair. Accepted, and it fixes a **real defect
  introduced by the proxy batch**: `proxySystem`, `proxyDiagnoseUrl` and
  `setProxyDiagnoseUrl` were never passed from `main.jsx`, so the page could not render
  the resolver summary or hold the diagnosis URL. Its React render regression test is
  kept. This also shows the Linux gap that let it through: only pure `ui-state.js`
  helpers were covered, never an actual render.

Follow-ups closed here:

- **Platform-valid protocol fixtures** (`CROSS_PLATFORM_REVIEW_REQUIRED`). Both validators
  use the host's own `is_absolute`; the hard-coded POSIX `/tmp/job-1` was the fixture's
  defect, not the validator's. Rust and Python now derive a host-absolute staging
  directory. Verified directly that `/tmp/job-1` is *not* absolute under
  `PureWindowsPath` while the new fixture is, and that a relative path is still rejected
  on both flavours. No validator relaxed. A positive "host-absolute is accepted"
  assertion was added, because a validator rejecting everything would have satisfied every
  original assertion.
- **Fail-closed PAC fall-through** (`CROSS_PLATFORM_CHANGE_REQUIRED`). The official
  resolver falls through to `DIRECT` when a script cannot be obtained or evaluated; that
  is indistinguishable from a deliberate `DIRECT` and would send corporate traffic out
  unproxied. The adapter now consults the PAC state and refuses that case. The rule is a
  platform-independent function so it is Linux-testable, and an unrecognized future state
  name is refused rather than assumed safe. **This is the one behavior change in the
  batch and it is shared, not Windows-only.**
- **Capability copy.** `system_proxy_supported()` no longer hard-codes `false` and its
  note no longer claims the native resolver is still to come. The settings test asserts
  the platform invariant instead of the stale literal.
- **WQ-PROXY-08 wording.** It demanded per-URL child routing while `WQ-PROXY-12`
  deliberately refuses a PAC child; it now scopes to static/environment/manual and
  cross-references the refusal.

Linux validation for this reconciliation: `cargo test --workspace` (21 targets ok,
Desktop 251, protocol 28), workspace all-target Clippy with `-D warnings`, `cargo fmt
--check`, the Sidecar pytest suite (61), the Desktop Node suite (209), the Vite build,
the documentation audit and `git diff --check` all PASS. The PAC refusal rule was
mutation-checked: reverting the allow-list fails its tests.

Not performed and not claimed: any Windows check. `WQ-PROXY-01`..`17`, the protocol
fixture rerun and M0..M14 all remain `WINDOWS_VERIFICATION_PENDING`, including the
controlled failing-PAC run the new rule needs. Prior extraction-timeout, confirmation
ACL and reconnect items stay open at their original artifact identities.

## Windows 6cf4cc8 validation return — 2026-10-05

State: `CROSS_PLATFORM_REVIEW_REQUIRED`; platform acceptance remains `WINDOWS_VERIFICATION_PENDING`.
Branch: `codex/windows-validation-6cf4cc8`. Input/handoff `6cf4cc8da25e0925c555cba2aaeb7667ea353414`.
Windows implementation repair `dc5ef1f2503dbbcadcdae5bec6bb169480e2e25b`; shared UI repair/validation build source `297a237aa1f91339c63870f93b1bb2e350b9c2a0`.
Return revision: Git documentation commit containing this entry. Current Owner at return: Windows Platform Owner; next Owner: Cross-platform Owner for Git integration/review and shared fixture/policy/copy follow-up. Windows retains native/manual acceptance.
Tracked tree clean before alignment; existing untracked Windows data retained. Git fetch and exact checkout completed; no direct sync. Old optional-download uncommitted wording below is superseded by committed 3f9cd1f in this handoff.

Input native compile FAIL (three Windows adapter compile errors), repaired on Windows. Native proxy 33/33, core proxy 24/24, production integrity 6/6, supervisor 11/11 after explicit Python path PASS. Rust protocol 27/28 and Python protocol 20/21: Linux absolute-path fixtures FAIL on native Windows; no validators weakened. Frontend targeted 75/75 PASS after Settings missing proxy props repair. Actual React test reproduces proxySystem ReferenceError before fix and passes afterward. Fresh native build and scoped Settings/expanded proxy GUI PASS at 297a237; full DPI/keyboard/persistence NOT_RUN. Fresh independent packaged worker five-capability/download_started PASS scoped; actual download completion/Full package NOT_RUN.

Desktop SHA256 `995930EE5B49345D2CBBC2C5CF28825EE1154F854E166E06595886F85909D56A`. Full evidence, earlier artifact hashes, tooling failures, commands and manual prerequisites: [batch result](../validation/windows-6cf4cc8-results.md). Local evidence `validation-artifacts/windows-6cf4cc8/`.

Cross-platform follow-up: review shared UI repair, make protocol fixtures platform-valid; review PAC/WPAD failure fall-through DIRECT against fail-closed policy; reconcile legacy PAC unsupported copy and per-URL child queue wording with deliberate PAC child refusal. Static policy concern is not a runtime PAC FAIL. Prior archive timeout/confirm ACL/reconnect/advisories remain open at their original identities. No full-regression, actual proxy-route, real-download or Telegram acceptance claimed. Manual continuation: current proxy M recipes and download queue, with fresh reviewed Full artifact, controlled proxy/VM fixtures and authorized external services.

---


## System proxy Batch B — ordered per-URL resolution and the Windows OS resolver — 2026-10-05

State: `READY_FOR_WINDOWS`. Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
Source revision `f57c4b2` is the previous handoff; this batch is the commit
`fbe3116b4fc1e64233a2185ae3ef2d1e8da60f1e` on top of `3f9cd1f`. Owner:
**Cross-platform Owner (Linux)**; the next owner is the **Windows Platform
Owner**. Windows must build from this exact commit.

### Scope

- `crates/xarchive-core/src/proxy.rs` gains the ordered contract:
  `ProxyCandidate` (`Http` / `Socks` / `Direct`), `ProxySource`, and
  `ProxyResolution` with a `reason` that keeps a failure cause instead of
  degrading it to a generic message. An empty candidate list is a policy
  failure, never a direct route. `ProxyBypass` implements the usual `no_proxy`
  matching rules.
- `desktop/src-tauri/src/system_proxy_resolver.rs` (new, `#[cfg(windows)]`) wraps
  `microsoft/os-proxy-resolver`, pinned to commit `796b027c9361bb407f2a8d9d79c56b2dc4a42ee2`
  because the crate is **not published on crates.io**. A backend-less build
  (`default-features = false`) is used so PAC/WPAD evaluation is delegated to
  WinHTTP and no embedded PAC engine or `unsafe` code enters the build.
- The environment resolver now honors `NO_PROXY` and separates the HTTP and HTTPS
  variables per URL instead of taking the first variable for every destination.
- `ProxyHttpClient` caches per candidate and per resolver generation, so a system
  proxy change invalidates cached clients. `candidates_for` returns the ordered
  list; the aria2 release download walks it because that GET is verified against
  the release digest. Telegram takes the first candidate only, because retrying a
  send through another proxy could duplicate it.
- The settings page reports the resolver, the PAC state, the static and bypass
  values, and the ordered candidate list, all redacted, and accepts a target URL
  for route diagnosis.
- `reqwest` gained the `socks` feature so a PAC result naming SOCKS is usable
  rather than a reason to refuse the policy.
- **Child-process coverage is now enforced, not assumed.** gallery-dl and aria2
  read proxy environment variables once, so they cannot evaluate a per-URL PAC or
  WPAD policy. Before the Sidecar starts, the executor asks the resolver which
  policy is active: an environment or static system policy is handed over as an
  inherited environment, while a PAC/WPAD policy (or an unresolved one) is
  **refused with `PROXY_POLICY_NOT_APPLICABLE`**. Without this the child would
  have silently connected direct, bypassing the policy. The settings page and the
  route diagnostic both report the answer as `child_coverage`.

### Validation performed here

`cargo test --workspace`, workspace all-target Clippy with `-D warnings`,
`cargo fmt --all --check`, the Desktop Node suite, the Desktop Vite build, the
documentation audit, and `git diff --check` all pass. See the commit message for
the counts.

### Explicitly not validated here

The Windows adapter **was not compiled**: only `x86_64-unknown-linux-gnu` is
installed, and the official resolver cannot build on a non-Windows target without
a PAC engine. The PAC/WPAD behaviour, the per-URL routes of the real transports,
the DPI and keyboard matrix, and the credential-leak sweep are all
`WINDOWS_VERIFICATION_PENDING`; see the Batch B table in
`docs/validation/windows-queue.md` and the manual steps.

## Optional download mode (`use_aria2`) — 2026-10-05

State: `READY_FOR_WINDOWS`. Branch: `cross-platform/automatic-pairing-reconcile-20261002`,
based on `16beade`. This batch is uncommitted in the Linux working tree; no commit
has been created yet, so the branch head is unchanged. Owner: **Cross-platform
Owner (Linux)**; after the commit and push the next owner is the **Windows Platform
Owner**.

### Scope

`use_aria2` becomes an optional configuration flag on `DownloadConfig` (default
`false`). When it is off, gallery-dl performs the media byte download itself and
aria2 never starts; when it is on, the existing extraction → `MediaTransferPlan` →
aria2 path is used unchanged. The two backends are never mixed within one Job.

This is a shared cross-platform protocol change (`CROSS_PLATFORM_CHANGE_REQUIRED`
applies to the contract; Windows retains native runtime validation):

- v2 `SidecarV2Command` gained an optional `staging_dir` field that only the new
  `download` command may carry, and `validate` requires it to be a non-empty
  absolute path; any other command carrying it is rejected.
- New `DownloadStarted` / `DownloadCompleted` events and the
  `DownloadedMediaFile` / `DownloadResult` payloads. `download_completed` must
  carry `download`; the payload may not ride on any other event.
- `download_media` is now a **required** capability (5 total). A worker that does
  not advertise it fails the handshake; there is no fallback path.
- Rust, Python `worker_v2`/`extraction`/`protocol_v2`, both JSON Schemas, the
  shared fixtures, and the supervisor handshake fixture were updated together.

### Rust behavior

`execute_v2_archive` routes on the flag: `true` → `execute_aria2_archive`
(unchanged), `false` → `execute_gallery_dl_archive`. The latter sends `download`,
waits for `download_completed`, cross-checks the tweet identity, and then
`verify_downloaded_files` re-anchors every reported relative path under the job
staging directory, rejects non-regular files, and compares the on-disk length to
the reported size before producing `DownloadFile`s. An extraction that lists media
but produced no file is an error rather than a metadata-only archive. When the flag
is off, `runtime::executor_config` resolves `aria2_program` to `None`, so the aria2
process path is unreachable.

### Python behavior

`build_download_command` runs gallery-dl for real (`--directory`, deterministic
`{num:>02}.{extension}`, `--write-info-json`) and defensively refuses
`--skip-download`/`--dump-json`/`--resolve-json`/`--simulate`.
`DownloadRunner` empties the staging directory first, runs the child with the
existing cancellation/timeout handling, recovers the extraction result from the
`info.json` gallery-dl wrote, pairs sorted media files with the extracted media
indexes, removes the metadata before reporting, and validates the result before
emission.

### Frontend

`AppStatus` exposes `use_aria2`; a new `set_use_aria2` command persists the choice
and rebuilds the executor configuration. The Settings page gained a
"媒体传输方式" section with a toggle.

### Linux validation (all executed on this tree)

- `cargo test --workspace`: 21 test targets, 0 failures (desktop 234,
  `xarchive-protocol` 28).
- `cargo clippy --workspace --all-targets -- -D warnings`: 0 issues.
- `cargo fmt --all -- --check`: clean.
- Sidecar `pytest`: 61/61 (includes new `download` command validation, download
  result validation, command-shape, runner, and worker-event tests).
- Sidecar `compileall`: clean.
- Desktop Node tests: 203/203. Extension Node tests: 52/52.
- Desktop `vite build`: success. `scripts/docs-audit.mjs`: PASS.
- `git diff --check`: clean.

### Windows-only work (not performed here)

Eight queue rows `WQ-DL-01..WQ-DL-08` were added to
[`../validation/windows-queue.md`](../validation/windows-queue.md) with manual
steps in [`../validation/windows-manual-steps.md`](../validation/windows-manual-steps.md).
They cover the packaged-worker handshake, a real gallery-dl download and archive,
proving aria2 does not start, cancel/timeout process-tree behavior, Windows
staging containment and reparse protection, Settings rendering/persistence across
DPI, the aria2 path regression, and restart recovery. **None may be marked PASS from
Linux evidence, from the command shape, or from the unit/fixture tests.**

### Next steps for the Windows Platform Owner

1. Commit and push this batch; build fresh artifacts from that exact commit.
2. Execute `WQ-DL-01` first: a stale packaged worker without `download_media` now
   fails the handshake, so verify the shipped worker advertises all five
   capabilities before attempting a download.
3. Run `WQ-DL-02`/`WQ-DL-03` together, then the remaining rows in order.
4. Record redacted worker logs only; keep cookies and tokens out of shared evidence.

---

## Cross-platform reconciliation and handoff — 2026-10-05

State: `READY_FOR_WINDOWS`. Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
Reconciled Windows return: `0aa8d14be1f3d4d3e5aac0886def850e9536f1b7`, based on
the prior UI source/handoff `ca25e5379302431a3130436896735b8be2bb4744`. This handoff
commit records Linux reconciliation and validation; no uncommitted state is included.
Current Owner after push: **Windows Platform Owner**. No shared production code was
changed in this reconciliation.

### Reconciled results and triage

- The return is documentation-only. Windows evidence is scoped: targeted UI tests
  47/47 and a fresh native `--no-bundle` build passed; current-size/maximized visual
  inspection, Sidecar Space/Enter, Extension navigation and refresh independence
  passed. It does **not** pass the full DPI, narrow-window, focus/hidden-Tab,
  busy/error or backend-effect matrix.
- The separate fresh Full artifact has package inventory/component-startup PASS
  only. On it, automatic browser connection, task creation, Telegram authentication
  and explicit Channel test receipt were reported PASS; the real archive attempt
  failed with `DOWNLOAD_TIMEOUT`, and a confirmation dialog reported ACL errors
  with action-unknown. No completed archive is evidenced.
- The Owner reports standalone use of the same packaged gallery-dl binary succeeded
  for the same URL and produced a readable image. This narrows investigation to
  invocation context, but does not prove whether arguments, working directory,
  environment/configuration or child-process handling caused the timeout. No
  task-scoped gallery stderr/progress was provided. Do not infer a shared defect or
  modify production behavior without independent reproduction.
- Automatic browser pairing and a real archive attempt in this Full package are
  **not the same acceptance** as the UI build. Keep the scoped PASS, archive FAIL,
  and queue-level Telegram delivery state distinct; in particular, an explicit
  Channel test receipt does not prove automatic archive/media delivery.
- Reconnect after Desktop restart required manual Extension “Reconnect”; manual
  reconnect passed, automatic recovery was observed failed without a measured
  recovery deadline. A stale displayed/actual port mismatch was observed separately
  and does not establish concurrent corruption.
- Route shared extraction diagnostics and confirmation call/capability review under
  `CROSS_PLATFORM_REVIEW_REQUIRED`; Windows Owner retains native reproduction and
  platform-specific queue execution. No `CROSS_PLATFORM_CHANGE_REQUIRED` is
  established.

### Linux reconciliation

The Windows return merge is documentation-only and preserves the original evidence
and artifact identities. Linux targeted UI regression, Desktop build, docs audit,
and diff hygiene are run on the reconciled tree and recorded with the final commit.
No Windows GUI/build/runtime result is claimed by this Linux batch.

### Next Windows handoff

Use this exact handoff commit and build fresh artifacts from it. Priority follow-up:
capture task-scoped redacted extractor arguments/stderr/progress and compare the
standalone versus application invocation; reproduce the confirmation-dialog ACL
failure and inspect the relevant capability path; verify successful archive/output
integrity, duplicate handling, automatic post-restart reconnect with timestamps,
and automatic Telegram archive/media delivery where authorized prerequisites exist.
Continue the remaining 100/125/150/200% DPI, narrow/wide, keyboard/focus/hidden-Tab,
busy/error/backend-effect settings matrix. Do not expose cookies, tokens, or other
credentials in shared evidence. Exact instructions and queue IDs are in
[`windows-full-ca25e53-manual.md`](../validation/windows-full-ca25e53-manual.md),
[`windows-manual-steps.md`](../validation/windows-manual-steps.md), and
[`windows-queue.md`](../validation/windows-queue.md).

---

## Windows ca25e53 visual validation return — 2026-10-05

State: WINDOWS_VERIFICATION_PENDING. Branch: codex/windows-validation-ca25e53.
Input/handoff/validation source: ca25e5379302431a3130436896735b8be2bb4744;
Cross-platform source: 4f119ca76694d4ec1063435d88bd33d66d9fae00.
Return revision: the Git documentation commit containing this entry.
Current Owner: Windows Platform Owner; next Owner: Windows Platform Owner for
remaining GUI acceptance; Cross-platform Owner reconciles this Git result and
continues existing advisory work. No production code or shared contract changed.
Tracked tree clean before checkout; local untracked dependencies/data retained.

Fresh Full follow-up: dist-portable/XArchive-full-validation-ca25e53-20261005; Desktop SHA256 37344cb7777152af1baf76eec85480e432e43ceaf44baae8ee31f8c598fe1b5e. Package inventory and component startup PASS scoped, real integrations NOT_RUN. See [Full manual recipe](../validation/windows-full-ca25e53-manual.md). This distinct Full build does not inherit the earlier GUI artifact PASS.

Scope: Targeted presentation tests and fresh native Desktop build/rendering.
PASS: node --test desktop/test/ui-wiring.test.mjs desktop/test/telegram-render.test.mjs
47/47; node desktop/scripts/build-tauri.mjs --no-bundle exit 0, including Vite
production build. Existing mixed static/dynamic Tauri import warning and MSVC
linker informational warnings retained. Full regression not run: only React/CSS
presentation/copy changed; Sidecar, shared Rust, lockfiles and workflows unchanged.

Artifact: target/release/xarchive-desktop.exe; SHA-256 d9457efe8154f165650d1bd18a49844aed9d7d19a37f8d6d6e12350980e67ccc.
Native environment: Windows 10.0.29680 x64, Node 24.19.0, Cargo 1.98.0;
local dev release channel. This is a fresh --no-bundle executable, not a Full package.
Evidence: validation-artifacts/windows-ca25e53/targeted-tests.log, native-build.log,
evidence.json, gui-disclosure-observation.txt, extension-refresh.png (local),
and inline Computer Use screenshots.

Scoped GUI PASS: current 1082x790 and maximized 1280x752 logical window captures;
concise collapsed summaries, shared Bootstrap/Telegram header spacing and
monochrome icon treatment, Extension puzzle icon; Sidecar and Extension expanded
status omit duplicate icons; detailed Sidecar/Telegram help remains in expanded
content; mouse expansion/collapse and Sidecar Space expand/Enter collapse;
Extension sidebar opens/expands/scrolls to matching panel; refresh leaves it open.
SQLite navigation also observed. Multiple Telegram/storage/Extension panels stayed
open independently. No credential/settings save or real message send performed.
DPI was not independently queried/changed (Owner previously reported 200%); these
window captures do not prove the full 100/125/150/200% matrix or a narrow breakpoint.

NOT_RUN/manual: full DPI/narrow-width matrix, all-panel Enter/Space and hidden Tab
order, focus visibility/navigation focus, fake-token masking/busy/error states,
and independent backend-effect checks for folding. Prior older-artifact GUI results
are not promoted to this changed presentation. Automation reported document-level
focus only, so rendered toggle behavior does not prove full focus acceptance.
Tooling: initial UIA Extension click landed on SQLite after maximize; first
coordinate attempt selected a 13x13 auxiliary screenshot and was rejected.
Recovered using the returned 1280x752 main screenshot; intended Extension jump
and refresh then succeeded. No product defect established or acceptance weakened.

Inherited remaining queue: cancel/shutdown fake-descendant subtree cleanup,
isolated upload=false release transfer/rehearsal and default-branch CodeQL, real
Telegram/Channel/archive acceptance. Deferred as unrelated to this presentation
change and existing Owner local/CI scope. Their prior states/evidence are retained.
No new CROSS_PLATFORM_CHANGE_REQUIRED or CROSS_PLATFORM_REVIEW_REQUIRED.
Cross-platform follow-up: reconcile this documentation return through Git and
continue unresolved Linux glib/WDIO advisory work; no shared implementation fix
requested by this batch.


---

## Cross-platform reconciliation — Windows return 52d8bc5 (2026-10-05)

Reconciled `origin/codex/windows-validation-aa8dabd` at
`52d8bc5a6c996d986c43e90cc574f81b8edf767c` into
`cross-platform/automatic-pairing-reconcile-20261002`. Input handoff:
`30bc96d1c5f69e0337c5928547ead131a6ea5aee` (containing
`aa8dabda5be8050c68ff472eb1090fca160a9082`); Windows implementation:
`39f54e5011b626827ee38b3ab06fadaf316b51b6`. Integration commit: `b937b5b`.
The Windows branch shared the earlier `aa8dabd` parent and did not contain the
later `30bc96d` handoff wording correction; merge preserved both histories.
No shared production implementation/contract change was made and no
`CROSS_PLATFORM_CHANGE_REQUIRED` is open.

### Reconciled Windows results (bounded to exact evidence)

- Release workflow hardening in `39f54e5`: trusted digests gate external asset
  execution/extraction; build/validation jobs have read-only contents and
  publication has a separate write-scoped job; inventory and hashes are rechecked
  before publishing, with no clobber. Isolated Windows Actions run 37255273960
  and native asset checks are recorded in
  [Windows history](../validation/windows-validation-history.md). This is not
  full production release acceptance.
- Sidecar packaged checks cover bounded burst/backpressure, matching cancel, full
  shutdown, EOF and worker exit PASS. Fake-descendant startup/subtree reclamation
  remains NOT_RUN because the fixture marker did not appear; no product defect
  was established.
- Settings GUI evidence is scoped PASS only for current-size mouse disclosure/form,
  hidden collapsed form, and SQLite/Sidecar navigation. Full DPI/keyboard/focus/
  Extension/busy-error/backend-effect matrix remains NOT_RUN.
- Windows native builds and module tests passed on exact `aa8dabd` implementation
  artifacts. Linux did not re-run or claim Windows evidence. Exact artifact hashes
  and test/tool details remain in Windows validation history.

### Remaining queue and ownership

Current Owner at Windows return: Windows Platform Owner. The return is now
integrated by the Cross-platform Owner; current Owner for this handoff is Windows
Platform Owner. Continue `WQ-SEC-SIDECAR-QUEUE-01`
subtree-cleanup fixture; `WQ-SEC-RELEASE-PERMISSIONS-01` isolated production-like
`upload=false` artifact transfer/rehearsal and default-branch CodeQL rescan;
`WQ-SETTINGS-PANELS-01` remaining DPI/keyboard/accessibility/side-effect acceptance.
Real Telegram/channel and archive acceptance remain NOT_RUN per recorded scope.
Linux advisory follow-up for `glib` and WDIO dependencies remains open and is not
resolved by this return. See [Windows queue](../validation/windows-queue.md) and
[manual steps §N](../validation/windows-manual-steps.md#n-current-continuation-follow-up--2026-10-05).

Linux reconciliation checks on the merged tree: Sidecar protocol tests 15/15
PASS; supervisor tests 11/11 PASS; Desktop Node tests 199/199 PASS after updating
the release-workflow wiring assertion; Desktop Vite build PASS (existing mixed
static/dynamic Tauri API chunk warning); docs audit PASS; `git diff --check` PASS.
Windows-only build, GUI, process-tree behavior and production Actions rehearsal
were not run on Linux.

Handoff revision: tracked in Git history on `cross-platform/automatic-pairing-reconcile-20261002`; pushed with a clean working tree. Current Owner after push: Windows Platform Owner, starting from the branch tip.

---

## Windows security implementation and native follow-up — 2026-10-05

This continuation supersedes the earlier aa8dabd pending implementation summary.
Branch: codex/windows-validation-aa8dabd. Input return: 3c77a1341bca3ffe7cd51b3b21622f6081b68e8d.
Implementation: 39f54e5011b626827ee38b3ab06fadaf316b51b6. Return/handoff revision:
the documentation commit containing this entry. State: WINDOWS_VERIFICATION_PENDING.
Current Owner at Windows return: Windows Platform Owner. This return is now
integrated by the Cross-platform Owner; Windows Owner performs remaining platform
acceptance. No shared
production code or contract changed, and no new shared-defect marker is asserted.
Tracked worktree clean at start; existing untracked local data preserved.

Implementation: windows-release.yml now defaults to contents:read; checkout does
not persist credentials; the build reads only the public Extension ID variable;
publication is a separate contents:write job requiring successful build. All
seven verified files are copied into a flat staging directory before upload;
publication rechecks source/tag, inventory, sizes, manifest and SHA256SUMS. Existing
asset names fail before upload; clobber was removed. Dispatch upload defaults to
false. Existing tag/source parity and independent non-blocking WDIO remain intact.
No Release was created, overwritten, or dispatched during this continuation.

External gallery-dl 2026.09.20 and aria2 1.37.0 are pinned in
 desktop/scripts/windows-external-assets.json. gallery digest was independently
read from the official upstream GitHub Release API; aria digest retains the
existing reviewed Windows aria2.rs pin. Verification rejects missing/invalid pins,
missing/empty files and altered bytes before execution/extraction. Pin changes
require independent source review; downloaded bytes never establish their own pin.

PASS: local release/security module tests 9/9; all 31 workflow PowerShell scripts
parsed; both real upstream downloads matched fixed pins; after verification,
gallery-dl --version returned 1.32.13:2026.09.20 and aria2 verified extraction and
--version returned 1.37.0. Isolated read-only Windows Actions PASS:
https://github.com/15699122/Tw2Tg/actions/runs/37255273960 , exact implementation
39f54e5, including negative integrity/policy tests and real asset downloads.
Full production build-to-publish transfer/rehearsal and default-branch CodeQL
rescan remain NOT_RUN; this isolated run is not production release acceptance.

Packaged Sidecar controls PASS on the unchanged aa8dabd worker: 256 metadata
commands (16 KiB each), producer blocked in all cases; matching extraction cancel
CANCELLED / exit 0 in 0.359s; full shutdown INTERRUPTED / exit 0 in 0.313s;
EOF / exit 0 in 1.782s, without synthesizing a shutdown event. Source queue remains
32, read count 33 while busy. EOF fixture marker PID 92060 was observed exited.
Cancel/shutdown fake descendant startup/whole-tree reclamation is NOT_RUN:
marker was not observed before controls; worker exit does not prove subtree cleanup.
Evidence/probes: validation-artifacts/windows-aa8dabd/queue-probe.json and
queue_probe_controls.py; no network/media accessed. Earlier probe timeout included
a fixture shutdown command missing required job_id; fixed, not a product defect.
Original pre-control child-marker waits and independent source diagnosis remain
recorded as fixture limitations, not a shared implementation failure.

Scoped native GUI PASS on unchanged aa8dabd Desktop hash
CA02189CFAB3CBEED5E017010A85B5DB6D2CD895BD91BEC208BEE7281E49A8FA:
current-size collapsed panels; mouse Telegram expand/collapse and form layout;
unconfigured credential checks disabled; hidden form absent from accessibility
text after collapse; SQLite opens storage and Sidecar opens its own section.
Computer Use produced actual snapshots. Current scaling was Owner-reported 200%,
not independently measured or changed. Keyboard focus automation did not establish
web control focus, so keyboard/hidden Tab acceptance remains NOT_RUN. Other DPI,
narrow/wide matrix, Extension jump/refresh independence, masking with entered fake
credential, busy/error and backend-side-effect checks remain NOT_RUN. No settings
or credentials saved; no Telegram send. Worker SHA-256 remains
11CFE182B2C1389F5813466D7B69747AC940F61C8E237EDA2F7016749D3902BE.

Owner explicitly selected local/CI scope for this continuation; real Telegram and
Channel validation remain in the manual queue. Cross-platform follow-up: integrate
Windows-owned workflow/tooling and review evidence; continue existing Linux advisory
work. Windows follow-up: production upload=false rehearsal/CodeQL evidence, fake
child reclamation and the remaining settings GUI matrix. Existing WQ-SEC-PERMS-01
closure and old icon/keyboard artifact results are preserved at their original identity.


## Windows aa8dabd validation return — 2026-10-05

State: WINDOWS_VERIFICATION_PENDING. Current/next Owner: Windows Platform Owner.
Branch: codex/windows-validation-aa8dabd. Input/handoff/validation revision:
aa8dabda5be8050c68ff472eb1090fca160a9082; Sidecar implementation:
2da530c2165125649ca47d4996908464dd32a696; settings implementation:
94fbfd403f8053d5e5584001b7e242e5321ec456. Exact target fetched from origin
and checked out through Git. Return revision is the commit containing this entry.
No production implementation changes in this return. Tracked tree was clean at
checkout; Windows-local untracked dependencies/artifacts were preserved. Tauri
build produced only a non-semantic Cargo.toml line-ending change, restored before commit.

PASS on native Windows 10.0.29680.0, Python 3.12.14, Rust/Cargo 1.98,
Node 24.19: protocol pytest 15/15; supervisor cargo tests 11/11;
Desktop Node tests 197/197 (elevated rerun); fresh PyInstaller worker build;
fresh Tauri optimized --no-bundle build. Independent instrumented queue probe:
capacity/queued 32, reader consumed 33 while extraction blocked, clean EOF after
release. This source probe is not packaged-process acceptance.

Worker SHA-256: 11CFE182B2C1389F5813466D7B69747AC940F61C8E237EDA2F7016749D3902BE.
Desktop SHA-256: CA02189CFAB3CBEED5E017010A85B5DB6D2CD895BD91BEC208BEE7281E49A8FA.
Desktop path: target/release/xarchive-desktop.exe. Worker path:
validation-artifacts/windows-aa8dabd/worker-dist/xarchive-downloader/xarchive-downloader.exe.
Logs and local probes: validation-artifacts/windows-aa8dabd/ (local artifacts,
not committed). Commands: pytest sidecar/tests/test_protocol_v2.py;
cargo test -p xarchive-sidecar-supervisor --locked;
node --test desktop/test/*.test.mjs; PyInstaller sidecar/pyinstaller/xarchive-downloader.spec;
node desktop/scripts/build-tauri.mjs --no-bundle.

Tooling limitations: initial sandbox Node killTree attempt was denied; elevated
rerun passed all 197. Packaged queue probe emitted extraction_started but its
controlled fake child marker never appeared (3-second attempts and a bounded
10-second reset-environment retry). Packaged cancel/full shutdown/EOF/child-exit
acceptance is NOT_RUN: fixture startup blocker, no product defect established.
Later shell channel lost valid cwd/FileSystem access; node filesystem/process
channel remained available and was used for reading evidence and Git write-back.

Scoped static review FAIL against release-hardening criteria: windows-release.yml
runs gallery-dl before trusted digest verification and expands aria2 before such
verification; workflow contents:write is inherited by build and publication is
in the build job. WQ-SEC-RELEASE-ASSET-01 and WQ-SEC-RELEASE-PERMISSIONS-01 remain
implementation pending; isolated Actions and negative digest tests NOT_RUN.
No release dispatched. WQ-SEC-PERMS-01 remains closed.

New settings native GUI/DPI/accessibility matrix NOT_RUN on this artifact;
previous Full-package screenshots/keyboard results do not close this changed UI.
Real Telegram/channel and real archive/duplicate tests remain NOT_RUN; bot/channel
creation reported by Owner is not credential/service acceptance.

Next Windows Owner: repair/independently verify fake fixture, complete packaged
queue cases; implement trusted asset pins and separate read-only build/publish
jobs, then isolated Actions; execute new settings matrix. Cross-platform follow-up:
review this shared queue source evidence and retain unresolved Linux glib/WDIO
advisory work; no new shared-contract defect or shared-code change claimed.



---

## Security remediation — Linux batch handed off (2026-10-05)

State: `READY_FOR_WINDOWS` for Windows-owned follow-up. Branch:
`cross-platform/automatic-pairing-reconcile-20261002`. Source revision before this
handoff commit:
`2da530c2165125649ca47d4996908464dd32a696`. This follow-up state correction is
committed and pushed as `aa8dabda5be8050c68ff472eb1090fca160a9082`; working tree
is clean. Current Owner: Windows Platform Owner.

Linux shared change: `sidecar/src/xarchive_downloader/worker_v2.py` now bounds
the stdin command queue to 32, backpressures the reader and stops it when the EOF
sentinel is drained. Regression coverage is in
`sidecar/tests/test_protocol_v2.py`. No protocol schema/version change. This is a
small shared implementation change preserving the abstraction:
`CROSS_PLATFORM_REVIEW_REQUIRED`.

Linux checks: initial focused regression reproduced that EOF drained all 128
metadata events but did not emit a protocol `shutdown` event. EOF is distinct
from an explicit shutdown command, so the regression now asserts complete drain
and clean EOF termination without expecting a shutdown event. Final focused
regression 1/1, `test_protocol_v2.py` 15/15, and complete `sidecar/tests` 55/55
PASS. The earlier `55/55` draft claim is superseded by these rerun results.
`cargo test -p xarchive-sidecar-supervisor --locked` 11/11; `cargo fmt --all --
--check`, `git diff --check`, and docs audit PASS. Network-isolated pytest attempt
was a tooling failure because bubblewrap omitted a writable `/tmp`; no product
conclusion was drawn from it. `cargo audit --no-fetch` used the local advisory DB
and is not a fresh online audit. Online `npm audit` on this worktree reported 20
HIGH entries including the existing `extract-zip` path and additional current
dependency findings; no Node dependencies were changed. See the security plan for
scope and caveats.

Windows work: execute `WQ-SEC-SIDECAR-QUEUE-01` against the exact pushed SHA and
follow manual steps §M.3 for bounded stdin burst/backpressure, cancel, shutdown
while full, EOF and process exit. `WQ-SEC-RELEASE-ASSET-01` and
`WQ-SEC-RELEASE-PERMISSIONS-01` require Windows Owner implementation/review and
isolated Actions validation; steps are in §M.1–M.2. These are `NOT_RUN` here, not
product failures or PASS. Existing `WQ-SEC-PERMS-01` is already closed; do not
reopen it. No release job or Windows workflow was modified in this Linux batch.

Open advisory results: `glib 0.18.5` remains Linux-target reachable through GTK3;
Windows target excludes it. No compatible locked upgrade was found. `extract-zip
2.0.1` remains in the WDIO tree with no patch indicated by the current npm audit;
do not infer GitHub alert state from local audit. Linux optimized release/GUI and
Windows runtime/Actions checks remain pending. Details in
`docs/development/security-remediation-plan.md`.

## Desktop settings visual batch — 2026-10-04

State: `READY_FOR_WINDOWS`. Branch: `cross-platform/automatic-pairing-reconcile-20261002`. Source revision: `c8f63c91ab0f8915e9a61713e2af65d98d7ab5df`; implementation revision: `94fbfd403f8053d5e5584001b7e242e5321ec456`; final corrected handoff revision: recorded below. Uncommitted state: none at handoff. Current/next Owner: Windows Platform Owner.

Implementing the screenshot-requested settings panels, Telegram presentation/icon, corrected Sidecar JSONL explanation and Extension footer spacing. Shared scope: `desktop/src/main.jsx`, `desktop/src/pages/settings-page.jsx`, `desktop/src/components/telegram-settings.jsx`, new `desktop/src/components/settings-section.jsx`, shared icon/button/CSS, and Desktop tests. No Rust, persistence, secret-store, Telegram command or protocol behavior is changed.

The initial resumption run found `desktop/test/ui-wiring.test.mjs` failed to parse; that stale assertion/syntax issue has been fixed. Current Linux validation: Desktop Node tests 197/197 PASS; `npm run check` PASS with the existing mixed static/dynamic `@tauri-apps/api/core.js` chunk warning; documentation audit PASS (96 Markdown files; zero dead links/unindexed docs); `git diff --check` clean. Windows validation is `NOT_RUN` in this Linux environment and is required on the exact handoff build: WebView2 rendered layout at 100/125/150/200% DPI and narrow/wide sizes; keyboard disclosure/focus and hidden-control tab order; SQLite/Sidecar/Extension navigation; Extension separator gap; Telegram form, credential masking, busy/error states; refresh action independence; and no backend effect from folding. UI evidence only; unrelated `WQ-TG-*` external-service acceptance remains governed by the existing queue. The Windows Platform Owner should start from the final correction handoff revision recorded below (implementation: `94fbfd403f8053d5e5584001b7e242e5321ec456`).

## Revision correction — 2026-10-04

The settings implementation is `94fbfd403f8053d5e5584001b7e242e5321ec456`. The earlier handoff documentation commit `fba84c070100403de6669208cc76c3206a491ccc` contained an incorrect implementation SHA reference. The correction is committed and pushed as `4a858eaae8185ff967f1ea4d7bd0faa230d0d7cf`; use `4a858eaae8185ff967f1ea4d7bd0faa230d0d7cf` as the handoff revision for checkout.

## Cross-platform reconciliation of the Windows f57c4b2 return — 2026-10-04

State: `WINDOWS_VERIFICATION_PENDING` after reconcile; next handoff state
`READY_FOR_WINDOWS`. Current and next Owner: **Windows Platform Owner**.
Reviewed incoming branch `codex/windows-validation-f57c4b2`, target
`3734f87c32dd3ab9eaabfec76f55cbdd16503019` (Windows implementation/build
`b5dd8c95f2685e8d91adf2bb9c24f7d7caf50012`) against the formal handoff `f57c4b2`
and source `542dc8ebd0bed69ea66afd575f8e8bdadb52fdff`. Reconciled into the
Cross-platform branch as merge `8e87e19`; this record's own commit is the next
handoff revision and supersedes the Windows local return section below.

### Reconciled result

- Integration was clean and stayed inside the Windows boundary. No shared crate,
  storage migration, schema or cross-platform contract changed: `crates/` and the
  shared Desktop Telegram modules are byte-identical to the handoff source.
- The native provider is a Windows-only module behind
  `#[cfg(windows)]`, depends on a pinned Windows-only `keyring = "=3.6.3"`
  (`windows-native`, explicit `WinCredential`), sanitizes native errors, maps
  `NoEntry` to absence, and installs through the existing production provider
  seam at startup. There is no default, mock or plaintext fallback. This matches
  the shared contract, so this is canonical integration, not
  `CROSS_PLATFORM_CHANGE_REQUIRED`.
- The Windows-only dependency and its licence obligation were registered in
  `docs/references/external-sources.md` and `THIRD_PARTY_NOTICES.md`. Reviewed and
  accepted: MIT selected, redistribution notice recorded, final release
  SBOM/notice inclusion still open.
- Product versus tooling classification is correct. Schannel fetch failure,
  incremental-cache finalization access denial, denied OS CIM query and linker
  informational warnings are recorded as tooling/environment conditions, and no
  assertion or product behavior was weakened. I found no product defect.
- Windows evidence is honestly bounded and is **not** promoted: native synthetic
  credential CRUD, module/mock Telegram and storage tests, UI tests/build, a
  native development-channel build and a scoped read-only settings smoke are
  `PASS`. Real sends, large files, crash recovery, proxy integration,
  WQ-TG-008 GUI acceptance, `WQ-TG-UNI-*`, the Scheme B integrated-source
  remainder and Full packaging remain `NOT_RUN`/`PLANNED`. Earlier pairing
  failures are not closed by this return.

### Linux validation on the reconciled tree

Run by the Cross-platform Owner after the merge, not carried over from the
pre-merge tree: `cargo build --workspace` PASS; `cargo test --workspace` PASS
(Desktop 231, Storage 75, Telegram 54, plus other workspace suites, 0 failures);
strict workspace all-target Clippy PASS; Node Desktop 195 PASS; docs audit PASS
(96 tracked Markdown); `git diff --check` clean. `cargo tree` confirms `keyring`
is not linked on Linux. Logs: `/tmp/tg-recon-build.log`,
`/tmp/tg-recon-test.log`, `/tmp/tg-recon-clippy.log`, `/tmp/tg-recon-node.log`,
`/tmp/tg-recon-docs.log`. Full regression was not escalated: the change set is a
Windows-only adapter plus documentation and the merged shared tree is
byte-identical to the tree that already passed the workspace suite.

### Next Windows work and blocking prerequisites

1. `WQ-TG-001` full acceptance — isolated-account GUI credential rotation,
   restart, presence-only projection and config/SQLite/log/diagnostic-export
   leakage matrix. This is the immediate next executable step and needs only
   Windows capability.
2. User-supplied prerequisites before any real-service queue can run: an isolated
   test bot token, a controlled chat/topic, and `api_id`/`api_hash` for Local Bot
   API Server deployment (`WQ-TG-007`, currently `PLANNED`). Until these exist,
   `WQ-TG-002`–`006`, `009` and `WQ-TG-UNI-*` cannot be executed by anyone.
3. `WQ-TG-008` full GUI matrix and the Scheme B integrated-source acceptance
   remain deferred to a subsequent Windows batch.

No cross-platform defect is open, so nothing blocks Windows from continuing, and
no `CROSS_PLATFORM_CHANGE_REQUIRED` or `CROSS_PLATFORM_REVIEW_REQUIRED` is
outstanding. Reuse of the Windows `PASS` results requires the native adapter,
pinned dependency, runtime setup and account capabilities to be unchanged;
otherwise record `REVALIDATION_REQUIRED`.

## Windows local return — 2026-10-04 / f57c4b2

State: `WINDOWS_VERIFICATION_PENDING`. Current and next Owner: **Windows Platform Owner**.
Source `542dc8ebd0bed69ea66afd575f8e8bdadb52fdff`; incoming documentation handoff
`f57c4b2090f02df3ba55c58a660e2eee143a9660`; Windows implementation/tested build
`b5dd8c95f2685e8d91adf2bb9c24f7d7caf50012` on `codex/windows-validation-f57c4b2`.
Return revision is the Git commit containing this section. No tracked uncommitted
state is required; pre-existing local caches, tools and validation artifacts remain untracked.
This current return supersedes the READY_FOR_WINDOWS state below.

Windows-native Credential Manager adapter and production provider injection are
implemented. Synthetic native CRUD, sanitized errors, Telegram/storage modules,
Desktop Telegram tests, UI tests/build and native build pass. Computer Use opened
the rebuilt application and settings showed credential service available, token
not saved and sender stopped. This is a scoped smoke, not real-send acceptance.
See [history](../validation/windows-validation-history.md#2026-10-04--telegram-windows-local-return-f57c4b2).

User confirmed no prepared bot/chat/api_id/api_hash and requested local validation
only. Local Bot API deployment remains PLANNED; real sends, large files, recovery,
proxy integration and Unigram receiving acceptance remain NOT_RUN. An open Unigram
window was detected, but no version/account/receiver matrix was executed.
Scheme B integrated-source acceptance is NOT_RUN this round: the minimum scope
selected was Telegram credential integration; earlier pairing FAIL/NOT_RUN is not closed.

Cross-platform follow-up: no shared contract change or product defect found.
Cross-platform Owner should reconcile the Windows commit through Git and review
the Windows-only pinned dependency/license record; this is canonical integration,
not `CROSS_PLATFORM_CHANGE_REQUIRED`. Next executable work is Windows GUI credential
presence/restart/leakage checks, then deployment and real-service queue after the
user prepares an isolated bot and target. Full regression/package publication was
not run: no release is being produced and the scoped tests cover this native adapter.

## Formal handoff to Windows Platform Owner — 2026-10-04

State: `READY_FOR_WINDOWS`. Ownership transfers from Cross-platform Owner to
Windows Platform Owner for this batch. This supersedes every
`CROSS_PLATFORM_IN_PROGRESS` checkpoint below; they remain as the development
history that produced this handoff, not as the current state.

| Field | Value |
|---|---|
| Batch | Telegram TG-06 shared integration, Scheme B remainder, plus the Windows Telegram acceptance queue |
| Branch | `cross-platform/automatic-pairing-reconcile-20261002` |
| Handoff source commit | `542dc8ebd0bed69ea66afd575f8e8bdadb52fdff` |
| Handoff commit | the commit containing this table; read its exact SHA from Git history |
| Parent baseline | `f4ec469f0ca3d657b9fe1b954a224bdb433e210d` |
| Uncommitted state | none — the Linux working tree was clean at handoff, and no uncommitted state is required |
| Previous owner | Cross-platform Owner (shared contracts, storage, transport, Desktop business logic, shared docs) |
| Current owner | **Windows Platform Owner** |

### Cross-platform work completed

- Completion-time Telegram configuration sampling in the archive commit path,
  with fail-closed behaviour on missing or invalid configuration.
- Immutable archive-intent capture, a durable journal, and idempotent
  ARCHIVED-to-outbox materialization with atomic new-row authorization.
- Credential rotation decisions and claim-generation fencing
  (migrations `0012_telegram_rotation_decisions.sql` and
  `0013_telegram_claim_generation.sql`), verified bot identity, and exact-row
  resume authorization for both `automatic` and `confirm` policy.
- Cooperative sender worker with claim heartbeat, startup recovery, and
  start/stop/settings-restart lifecycle; it never fabricates a credential
  provider.
- Registered Tauri commands: settings projection, credential replace/delete with
  revocation before native cleanup, rotation-candidate confirmation, explicit
  reviewed `UNKNOWN` resend, task/job state projection with live upload stages,
  bounded cancellation and sender stop.
- Explicit never-retried Cloud/Local endpoint-migration control path plus a
  durable `migration_pending` latch that only that command clears.
- Shared test coverage including concurrent materialization, restart-then-single
  send, permanent-failure isolation from the local archive, provider lifecycle,
  and mutation-checked bounded storage-error handling.

### Changed shared modules

`crates/xarchive-telegram`, `crates/xarchive-storage` (including migrations),
`desktop/src-tauri` (`archive`, `commands`, `config`, `executor`, `runtime`,
`telegram_send`, new `telegram_worker`, new `telegram_control`), the React
settings/task surfaces, and shared documentation.

### Windows work required before dependent checks can run

1. `WQ-TG-001` — implement the Windows Credential Manager adapter and inject it
   through the provider seam. It must be the only production `SecretStore`; no
   plaintext or in-memory fallback may be introduced.
2. `WQ-TG-007` — deploy a version-pinned Local Bot API Server bound to loopback.

### Windows validation required

Run the queue in [`../validation/windows-queue.md`](../validation/windows-queue.md)
and manual steps §K in
[`windows-manual-steps.md`](../validation/windows-manual-steps.md). Start from
the exact source SHA above: fetch, verify a clean Windows working tree, check
out that revision, and record Windows build/architecture/tool versions plus the
rebuilt artifact SHA-256. Linux workspace, Clippy, Node and Sidecar results are
shared evidence only and are not Windows PASS. `WQ-TG-002`–`006`, `008`, `009`
and `WQ-TG-UNI-01`–`08` remain `NOT_RUN` until executed.

### Known risks and expected behavior

- `UNKNOWN` is never resent automatically; re-send requires a deliberate
  reviewed action and may produce a duplicate.
- After an endpoint-mode migration, sending stays disabled until explicitly
  enabled. A failed or unknown migration outcome keeps it paused; do not blindly
  retry.
- Loopback endpoints always connect direct, never through a configured proxy;
  cloud endpoints honour the configured proxy.
- A confirmed send means only that the Bot API answered. It never implies the
  client received, displayed or read the message.

### Deferred GUI and manual items

Keyboard traversal, 100/125/150% DPI, narrow-window layout, cancel/retry/
`UNKNOWN` review interactions, Credential Manager presence checks, and the
Unigram receiving-side matrix. See manual steps §K for the numbered procedures.

### Cross-platform follow-up

None outstanding for this batch. Any shared-contract, schema or cross-platform
behavior finding on Windows returns to the Cross-platform Owner as
`CROSS_PLATFORM_CHANGE_REQUIRED`; a small fix that preserves an existing
abstraction uses `CROSS_PLATFORM_REVIEW_REQUIRED`.

## Documentation reconciliation and proxy-policy regression — 2026-10-04

- Owner/state: Cross-platform Owner / `CROSS_PLATFORM_IN_PROGRESS`. Parent revision
  `f4ec469f0ca3d657b9fe1b954a224bdb433e210d`. This increment is a development
  checkpoint, not a formal handoff, release approval or ownership transfer.
- Closed the Linux/cross-platform work items that remained after `f4ec469`. The
  shared Telegram production path was already complete; what remained was
  documentation that still described it as missing, which would have made the
  Windows Owner misread `IMPLEMENTATION_NOT_READY` blockers as absent features.
- Reconciled stale claims against the code: `status.md` capability matrix and
  Telegram capability line, `architecture/overview.md` Native Host and Telegram
  entries (Windows Named Pipe server, ACL and browser install wiring are
  implemented; only real Windows registration and browser acceptance are
  outstanding), and the Telegram Plan continuation plus round-boundary sections.
  The round-boundary text is preserved as the state at that point, with an
  explicit note that its four items were subsequently delivered.
- `windows-queue.md` TG table now carries an explicit Implementation column.
  `WQ-TG-002`–`006`, `008`, `009` are `IMPLEMENTED`; `WQ-TG-001` and `WQ-TG-007`
  are shared-`IMPLEMENTED` with the Windows-specific adapter/server deployment
  still `PLANNED`. Every TG row remains `NOT_RUN`, but the deferral reason is now
  the missing Windows/real-environment capability instead of "no production
  entry point". `WQ-TG-UNI-*` rows are unchanged.
- Renamed manual steps §K from "no production send entry point" to "shared
  production path ready, awaiting Windows/real-environment acceptance", updated
  its prerequisites to name only the two genuinely Windows-blocked items, and
  repointed the queue's §K anchor at the new heading.
- Reviewed the outstanding migration proxy question and found the behavior
  correct rather than defective: `with_api_endpoint` pins `no_proxy` for
  `EndpointMode::Local` and keeps the resolved proxy for cloud, so a Cloud→Local
  migration keeps the proxy on the cloud leg and bypasses it on the loopback leg.
  Added `migration_source_and_target_apply_endpoint_specific_proxy_policy` to lock
  that contract in, including that proxy credentials never reach `Debug` output.
- Heartbeat coverage was re-checked rather than assumed missing: accelerated-tick
  renewal and heartbeat-loss tests already exist in `telegram_send.rs`. The Plan's
  "accelerated heartbeat/fault tests and bounded storage-error retry remain
  pending" wording is now stale and was corrected.
- Added the two missing storage-error tests. `heartbeat_storage_error_stops_the_
  attempt_and_is_not_retried` removes the outbox table so the renewal fails with
  a real SQL error, and requires the driver to end on the first failure rather
  than retry the renewal. `an_unusable_file_cache_stops_the_row_without_claiming_
  it_sent` removes the `file_id` cache table and requires the media row to be
  deferred with no request reaching the Bot API.
- Both new tests were mutation-checked rather than trusted: making the renewal
  return `Poll::Pending` instead of the store error fails the first test by
  timeout, and downgrading the cache read to `unwrap_or(None)` fails the second
  test with the raw SQL error. An earlier draft of the second test asserted a
  batch-level abort after a confirmed send; that premise was wrong, because the
  unreachable cache stops the row before the request. The test was rewritten to
  assert the behavior the code actually and correctly guarantees, instead of
  changing product behavior to fit the test.
- No product code changed in this increment; both tests lock in existing
  behavior.
- Final combined results on this tree: `cargo test --workspace` PASS (Desktop
  231, Telegram 54, plus other workspace suites); workspace all-target Clippy
  with `-D warnings` PASS. Logs: `/tmp/tg-final4-workspace.log`,
  `/tmp/tg-final4-clippy.log`.
- Windows Credential Manager, native runtime/packaging, packaged Local Bot API
  deployment, GUI acceptance, controlled real-account send and Unigram acceptance
  remain `NOT_RUN` and are Windows Platform Owner work. No Windows PASS is
  claimed. No shared Telegram implementation or test scope is known to remain;
  the next step is a formal handoff naming an exact SHA.

## Shared-development closure checkpoint — 2026-10-04

- Owner/state: Cross-platform Owner / `CROSS_PLATFORM_IN_PROGRESS`. Branch
  `cross-platform/automatic-pairing-reconcile-20261002`; parent revision
  `48312827a898e3854cb4ff17371987d9a16ed705`. This increment is a development
  checkpoint, not a formal handoff, release approval or ownership transfer. Its
  exact SHA is the commit carrying this section; read it from Git history rather
  than from this text.
- Closed the remaining non-Windows TG-06 scope: an explicit never-retried
  Cloud/Local endpoint-migration control path (`telegram_control.rs`), a durable
  `migration_pending` latch that only that command clears, and live upload-stage
  projection on every send path.
- The migration flow persists the pause before any control request, requires
  explicit confirmation, uses `logOut` for a Cloud source and `deleteWebhook`
  then `close` for a Local source, verifies the target bot identity, and leaves
  sending paused on any failure or unknown outcome. It cannot be bypassed by an
  ordinary settings save or by sender startup.
- Real defect found and fixed while testing: text-only sends reported no progress
  stage, so the task projection was silently empty for text archives. The text
  path now reports queued/awaiting/confirmed. Byte-level `Uploading` progress is
  still only emitted when a body is actually streamed.
- Tests added: migration latch blocking automatic sending and the batch runner;
  the latch surviving a config round trip; the settings projection exposing the
  pause without secret material; reported stage ordering; frontend command wiring
  per surface, and confirmation-gated migration.
- Final combined results on this tree: `cargo test --workspace` PASS (Desktop
  229, Storage 75, Telegram 53); workspace all-target Clippy with `-D warnings`
  PASS; Node Desktop 195 and Extension 52 PASS; frontend build, `cargo fmt
  --check`, `git diff --check` and docs audit (96 tracked Markdown) PASS. Logs:
  `/tmp/tg-final2-workspace.log`, `/tmp/tg-final2-clippy.log`,
  `/tmp/tg-final2-node.log`, `/tmp/tg-final2-ext.log`, `/tmp/tg-final2-build.log`,
  `/tmp/tg-final2-docs.log`.
- Documentation reconciled: the Plan, the capability status paragraphs and the
  Windows queue no longer describe production enqueue, commands or UI as missing.
- Windows Credential Manager, native runtime/packaging, packaged Local Bot API
  deployment, GUI acceptance, controlled real-account send and Unigram acceptance
  remain `NOT_RUN` and are Windows Platform Owner work. No Windows PASS is
  claimed. The next step is a formal handoff naming an exact SHA.

## GitHub progress synchronization — 2026-10-04

- Owner/state: Cross-platform Owner / `CROSS_PLATFORM_IN_PROGRESS`.
- Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
- Source baseline: `fece4381a1bce47c8b7aaebd342b19fb15165afc`.
- Synchronization revision: the commit containing this section; obtain its exact
  SHA from Git history. This commits all retained Telegram implementation,
  migrations 0012/0013, worker, UI, tests and checkpoint documentation.
- This is a progress checkpoint, not `READY_FOR_WINDOWS`, release approval or
  ownership transfer. Shared remaining work listed below still applies.
- Earlier uncommitted-state statements describe their historical checkpoints;
  after this synchronization, verify local cleanliness and remote branch SHA.
- Validation is the completed combined-tree evidence listed below; this
  synchronization changes documentation only and claims no new Windows results.

## Goal continuation — 2026-10-04 (provider and UI integration)

- Current Owner: Cross-platform Owner; state `CROSS_PLATFORM_IN_PROGRESS`.
  Branch `cross-platform/automatic-pairing-reconcile-20261002`, source HEAD
  `fece4381a1bce47c8b7aaebd342b19fb15165afc`. Changes remain uncommitted;
  this is not a formal handoff and no commit or push was performed.
- Shared provider injection, startup/settings sender restart, sanitized batch
  errors, native deletion revocation-before-cleanup, random secret references,
  credential presence and task projections are wired. React settings and job
  surfaces are mounted. Production has no MemorySecretStore/plaintext fallback;
  Windows Owner must inject the native provider.
- Sender batches reconcile completed journals and recover expired claims before
  sending. Original-file mode captures document media kinds at completion.
  Tests cover concurrent materialization, disk reopen followed by one send,
  permanent API failure preserving local archive, injected lifecycle restart,
  missing provider and native-delete failure revoking authority.
- Final completed checks: Rust workspace PASS (Desktop 220, Storage 75,
  Telegram 53); strict workspace all-target Clippy PASS; Node Desktop 193 and
  Extension 52 PASS; Sidecar 54 PASS; frontend build, format, diff check and docs
  audit PASS. Logs: `/tmp/tg-reviewed-workspace.log`,
  `/tmp/tg-reviewed-clippy.log`, `/tmp/tg-reviewed-node.log`,
  `/tmp/tg-reviewed-build.log`, `/tmp/tg-goal-sidecar.log`,
  `/tmp/tg-goal-docs.log`.
- Reviewed UNKNOWN resend is now an explicit selected-row, active-generation
  transaction gated by duplicate-risk acknowledgement; automatic claiming of
  UNKNOWN remains forbidden. A regression proves missing confirmation, stale
  generation and repeated review are rejected.
- Remaining shared work is NOT a Windows blocker: complete explicit
  Cloud/Local migration wizard, diagnostic/control-request
  fault tests, progress/reason projection, and final Plan/document reconciliation.
  Current endpoint-mode changes with active credentials are rejected rather
  than silently performing logOut. GetChat/logOut/close contract variants exist;
  no completed migration flow is claimed.
- Windows credentials, native GUI, external Local Server, real Telegram and
  Unigram acceptance remain NOT_RUN. Queue and manual steps remain the authority;
  do not promote existing IMPLEMENTATION_NOT_READY entries to PASS.

## Continuation checkpoint — 2026-10-04 (completion-time capture)

- Owner/state: Cross-platform Owner / `CROSS_PLATFORM_IN_PROGRESS`; NOT a formal Windows handoff. Branch remains `cross-platform/automatic-pairing-reconcile-20261002`, HEAD `fece4381a1bce47c8b7aaebd342b19fb15165afc`; all implementation remains uncommitted, including migrations 0012/0013 and `telegram_worker.rs`. No restore, commit, push or ownership transfer occurred.
- This checkpoint supersedes the receipt's production `telegram_intent: None` statement: production now samples persisted Telegram configuration after extraction/transfer completion, captures verified active bot identity and immutable archive facts, journals the intent through ArchiveService, and attempts idempotent materialization after local commit. Missing/invalid configuration fails closed. Startup recovery remains journal-only. Error-return contexts preserve the sampling source.
- Atomic materialization now grants only newly inserted rows to the currently active matching bot generation in the same transaction. Existing rows receive no grant on replay; rotation confirmation remains exact-row and UNKNOWN remains excluded. Full bot/target/key uniqueness is used for detecting existing rows.
- Registered Tauri commands: task-state projection, captured-candidate resume confirmation, bounded sender stop, non-secret settings save (server-owned revision/capability), and clean queued/retry cancellation. Saving settings stops the old worker before persisting; it does NOT restart a sender without a native provider.
- New regressions cover completion-time target/revision changes, disable/malformed/missing configuration, initial new-row authority and no reauthorization on post-rotation replay.
- Validation evidence: `/tmp/tg-final-workspace.log` (`cargo test --workspace`, successful completed run: Desktop 216, Storage 74, Telegram 53 plus other workspace tests); `/tmp/tg-combined-clippy.log` (strict all-target Clippy for Desktop/Storage/Telegram). Initial compilation errors from incorrect method signatures and initial Clippy test-module placement failure were corrected before the successful runs. Final checks are rerun after the error-context sampling-source preservation change; consult the completed logs rather than terminal integration status.
- Capability detected this continuation: WSL2 Linux x86_64, Linux-only Rust target, no Wine executable; DISPLAY/Wayland exported. No Windows artifact or native credential/real Telegram acceptance was executed. Terminal completion reporting failed intermittently; completed logs/process inspection were used instead of duplicating still-running tests.
- Remaining non-Windows scope is NOT complete: production provider injection/startup/settings restart and credential commands/presence projection, frontend settings/task integration, and expanded end-to-end crash/rotation/heartbeat tests remain. Capture failures currently fail closed without a dedicated Telegram diagnostic projection and need follow-up. Do not label this READY_FOR_WINDOWS or complete TG-06.
- Windows queue remains `NOT_RUN`; see `../validation/windows-queue.md` and manual steps §K. Native Credential Manager adapter, Windows runtime/GUI/packaging and real-service/Unigram acceptance belong to Windows Owner. No plaintext or in-memory production credential fallback was introduced.

## Cross-session receipt — 2026-10-04

This receipt supersedes stale present-tense statements below; older increments remain historical evidence, not validation of the entire current working tree. This is a local cross-session transfer, NOT a formal Windows handoff.

- Owner: Cross-platform Owner. Branch: `cross-platform/automatic-pairing-reconcile-20261002`. Committed baseline/local tracking revision: `fece4381a1bce47c8b7aaebd342b19fb15165afc`. No remote fetch was performed for this receipt.
- Uncommitted implementation includes rotation decisions (migration 0012), claim-generation fencing (0013), planner versions 1/2/3, real-time Clock, heartbeat, cooperative sender worker, RuntimeState ownership, archive metadata capture helper and executor Telegram configuration snapshots. Preserve all modified and untracked files; do not restore entire files to the baseline.
- Code truth: `archive.rs` still passes `telegram_intent: None`. Runtime has `start_telegram_sender`, but production credential-provider/startup/settings commands are not connected. Existing helpers are not a usable automatic-send product.
- Next: decide and test configuration capture timing (executor/context snapshot currently differs from the approved completion-time sampling); connect immutable intent capture, atomic materialization and exact-row authorization for new plans; then connect provider/startup/settings lifecycle, Tauri business commands and state projection. Complete crash/rotation/stop integration tests before platform delivery.
- PASS (existing local logs, not newly rerun): Desktop library 214/214 and Desktop strict all-target Clippy in `/tmp/tg-executor-snapshot.log` and `/tmp/tg-executor-snapshot-clippy.log`. Historical Storage 74/74 and Telegram 53/53 are older incremental results and require revalidation against the final combined tree.
- NOT_RUN: final combined subsystem/full-workspace regression, production archive-to-send integration, Windows/native GUI/Credential Manager, real Telegram/Local Server/Unigram acceptance. Shared implementation gaps are NOT Windows blockers.
- Tool diagnostic: terminal completion reporting intermittently fails; read completed logs and inspect processes rather than repeatedly restarting tests. Do not treat that tooling failure as a product defect.
- Windows execution remains in `../validation/windows-queue.md` and manual steps §K of `../validation/windows-manual-steps.md`. Implementation-not-ready checks remain NOT_RUN; missing platform capabilities may be BLOCKED only after detection. No Windows PASS or release approval is claimed.


Status: `CROSS_PLATFORM_IN_PROGRESS` — 2026-10-04 development checkpoint. This batch adds archive-intent recovery and atomic outbox materialization, verified bot identity, credential generations and exact-row resume grants. Runtime startup coordinates persisted intents; production capture, policy-driven authorization and sender execution remain incomplete. Windows acceptance remains `NOT_RUN`. Current Owner: Cross-platform Owner. No Windows or real-send PASS is claimed; this checkpoint is not a formal Windows handoff.

- Checkpoint branch: `cross-platform/automatic-pairing-reconcile-20261002`.
- Parent/source baseline: `ddc533658860d28b60e50e86e8e1ce4687f02e39`.
- Checkpoint revision: the Git commit containing this update; identify its exact SHA from Git history.
- Working-tree scope: all listed Telegram implementation and documentation changes are included in this checkpoint; post-commit cleanliness is verified separately during synchronization.
- Latest validation: Telegram 52/52, Storage 68/68, Desktop 197/197; relevant strict all-target Clippy, format and whitespace checks PASS. No full workspace or Windows acceptance is claimed.

## Active Telegram TG-06 resumed batch — 2026-10-04

- Post-checkpoint policy increment (uncommitted, based on fece438): candidate
  selection is an exact eligible-row snapshot for the ACTIVE generation.
  Same-bot automatic policy grants that snapshot; confirmation policy returns
  candidates without granting; different-bot identity does not resume old rows.
  Storage 68/68, Desktop 197/197 and strict all-target Clippy PASS. Production
  credential commands, durable rotation decision recovery and sender remain incomplete.

- Claim authorization increment: automatic and keyed manual claim SQL now requires
  an exact grant from the ACTIVE generation for telegram-bot:<id> identities.
  Stable-bot UNKNOWN rows cannot be claimed through the generic manual API.
  Legacy identities retain their historical behavior pending explicit migration;
  they are not converted or granted production authority here. Storage 68/68,
  Desktop 197/197 and strict all-target Clippy PASS. Policy-driven grants and
  production sender integration remain incomplete.

- Resume authorization storage: migration 0011 persists grants for exact outbox ids
  and active credential generations. Grants reject other bots, UNKNOWN and unavailable
  snapshots; invalid selections roll back all grants. New rows are not implicitly
  authorized and retired generations cannot grant authority. Storage 67/67 and strict
  storage/Desktop all-target Clippy PASS. These APIs are not yet enforced by production
  claim SQL or invoked by automatic/confirmation credential commands.

- Candidate activation coordination: verified credentials are stored under isolated
  SecretStore references before database CAS activation. Stale activation cleans up
  its inactive candidate without overwriting the active secret. Cross-connection
  stale-generation regression and disk reopen tests PASS (Storage 66/66); candidate
  coordination regression PASS (Desktop 197/197). Strict all-target Clippy PASS.
  This helper is not yet a production credential command or queue resume authorization.

- Credential persistence increment: migration 0010 stores verified bot identity and
  opaque SecretStore references with CANDIDATE/ACTIVE/RETIRED generations, never tokens.
  Transactional compare-and-set activation rejects stale verification; a partial unique
  index enforces one active generation. Disk reopen regression PASS. Storage 65/65,
  Desktop 196/196 and strict storage/Desktop all-target Clippy PASS. SecretStore/SQLite
  activation coordination and outbox resume authorization are still not connected.

- Verified identity increment: shared transport now supports non-publishing getMe;
  verification accepts only a positive numeric bot id with is_bot=true and produces
  telegram-bot:<id>, independent of token rotation. Credential replacement returns
  the verified identity after successful secret storage instead of discarding it.
  Telegram 52/52 and Desktop 196/196 PASS; strict all-target Clippy PASS.
  Durable generation activation and queue authorization remain unimplemented.

- Runtime continuation: PREPARED archive file recovery now runs before executor startup
  recovery, then ARCHIVED intents are materialized without network requests. Individual
  failures are isolated and retained for review. Desktop 196/196 PASS; strict all-target
  Clippy PASS. The media planner now splits contiguous document-only and photo/video runs
  without reordering (Telegram 51/51 and Desktop send tests 13/13 PASS). No production
  intent capture, credential authorization or sender loop is claimed by this increment.

- Latest incremental evidence: disk journal recovery covers staging and renamed final across
  database reopen (Storage 64/64). Desktop persisted-snapshot planner now uses the atomic
  journal/outbox API and preserves captured bot/chat/topic/config facts (Desktop 195/195).
  Strict storage/Desktop all-target Clippy PASS. Production wiring, authorization, scheduler,
  commands/UI and full integration remain incomplete; Windows checks are not executed.

- Goal continuation: complete-plan outbox insertion is now transactional; Desktop prepares all
  entries before insertion. Conflict rollback and replay regression PASS. Storage 63/63,
  Desktop 194/194 and strict storage/Desktop all-target Clippy PASS. Shared implementation is
  still incomplete (journal advancement, credentials/authorization, production enqueue, scheduler,
  commands/UI); no Windows validation or formal handoff occurred.

- Batch A continuation: fixed final-directory recovery after rename, prevented accidental staging creation,
  added persisted intent/job/tweet and media-integrity validation, and corrected immutable-intent comparison.
  Regression covers rename before archive database completion and damaged metadata. Batch A remains
  CROSS_PLATFORM_IN_PROGRESS; the exhaustive failure/reopen/concurrency matrix and journal identity design
  remain pending. All changes are local and uncommitted; no ownership transfer or Windows evidence.

- Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
- Source commit: `ddc533658860d28b60e50e86e8e1ce4687f02e39` (branch tip at round start; all current working-tree edits remain uncommitted).
- Handoff commit: none — this is **not** a Windows handoff; status stays `CROSS_PLATFORM_IN_PROGRESS`.
- Uncommitted state: retained edits in `crates/xarchive-telegram/src/lib.rs`; `desktop/src-tauri/src/config.rs` (rotation policy plus token replacement/deletion orchestration and tests); `desktop/src-tauri/src/telegram_send.rs` (topic propagation, immutable `ArchiveSendIntent` planner input and topic regression); Telegram Plan/handoff/queue/manual updates; indexes and new `docs/user-guide/telegram.md`. These changes remain uncommitted; nothing was committed or pushed.
- Current Owner: Cross-platform Owner. PowerShell interop capability was probed; no Windows validation or platform evidence was produced.
- Audited boundary: production archive completion is in `/home/shiraishi/VSCode Workspace/Tw2Tg/desktop/src-tauri/src/archive.rs`; `queue_archive_sends()` still has no production caller. `renew_outbox_claim()` has no live sender caller; runtime has no Telegram scheduler. `RuntimeState` has no injected SecretStore or Telegram commands. `complete_sidecar_archive()` and the filesystem rename/SQLite writes are multi-step and have no tested archive/outbox reconciliation protocol. Remaining non-Windows work is itemized in the Telegram Plan's current-round boundary.
- 2026-10-04 continuation audit: migration `0009` and the PREPARED/ARCHIVED journal plus staging-recovery operation are implemented and covered by two storage tests. The database+filesystem protocol is not complete: crash-boundary injection, missing-stage/manual-review resolution, ARCHIVED-to-outbox materialization, and production callers remain pending. This continuation's final checks: Storage 62/62; Desktop library 194/194; storage+desktop strict all-target Clippy, format, diff check, docs audit PASS. Telegram suite 50/50 is prior same-day evidence and was not rerun in this storage increment.
- Implementation this round: added `TelegramConfig::replace_bot_token()` and `delete_bot_token()` over the existing SecretStore abstraction; candidate verification happens before secret mutation and the token is never included in config or UI projection. Added tests for failed verification retaining the prior token, verified replacement, and deletion. Preserved the existing topic propagation work in the sender and added an end-to-end fake-server regression asserting text posts include the persisted topic. These helpers do not implement credential generations, atomic platform-store failure guarantees, queue authorization, Tauri commands or a Windows adapter.
- Continuation: storage migration `0009_telegram_archive_intents.sql` records immutable archive/target planning facts before archive rename when an intent is supplied. `ArchiveService` can recover the PREPARED journal from staging and advance it to ARCHIVED; CAS transitions and immutable-fact conflict behavior are tested. Production archive callers still pass no Telegram intent; there is no ARCHIVED-to-outbox recovery runner, scheduler, heartbeat, or settings/send UI. Linux checks: Storage 62/62, Desktop all-target compile check, fmt and diff check PASS. This is a journal foundation only, not proof of all crash boundaries or production coordination.
- Validation in this round: WSL2/Linux x86_64; starting source `ddc533658860d28b60e50e86e8e1ce4687f02e39`; working tree remains uncommitted. PASS: `cargo test -p xarchive-telegram` 50/50; `cargo test -p xarchive-storage` 60/60; `cargo test -p xarchive-desktop --lib` 194/194; strict Clippy for the three crates; `cargo fmt --all -- --check`; `git diff --check`; docs audit (95 tracked Markdown). Node/Sidecar checks were from the retained 2026-10-04 baseline, not rerun for this incremental Rust/doc change. Full workspace regression not run. Windows native build/GUI/Credential Manager/Local Bot API/real-send/Unigram were not run; queue remains `NOT_RUN`, with manual procedures in §K. No commit/push/handoff was created.

- Formal Windows test source: `9920566ef1df115effc3ab5df5d9a12fd42210ed` (includes `5ff0a22` Windows integration).
- Handoff documentation branch: `cross-platform/automatic-pairing-reconcile-20261002`. This commit records the documentation checkpoint authorizing Windows retest of the formal source.
- Incoming source reconciled in this batch: `6060f5a2d554d32fa8400f7f5084319b19fabeb1`; Windows return `ebdc44db5ae17ef4189532d06dbb6f6a22a629ae` on `codex/windows-validation-6060f5a`.
- Earlier Windows implementation `d65a01bfe2c6f93a071322bfef154fd947b0b70b` and evidence `30fc57588065b73677473d411368219eda3313a7` are based on `0c74087cf26d7120dfe6bbabb7c66b37b879fe3e`, not on the tested source. Integration commit `5ff0a22` carries identical `crates/` + `desktop/src-tauri/` code to d65a01b (verified by empty diff on those trees); d65a01b Windows PASS remains bound to its own source/artifact and is not claimed for the integrated source.

Exact `6060f5a` Windows evidence: targeted package/UI Node tests 27/27 PASS; Native Host release build and canonical Full build PASS; Extension directory inventory PASS scoped (13 files including helper); generated Full `installation.files` omitted the shipped helper (FAIL scoped); packaged Host returned `BOOTSTRAP_NOT_IMPLEMENTED`. Automatic pairing, UI reproduction, E2E and lifecycle were NOT_RUN / IMPLEMENTATION_NOT_READY on that source. Build success is not GUI or integration acceptance.

CROSS_PLATFORM_CHANGE_REQUIRED was addressed by integrating the Windows bootstrap through Git; Windows validation of that integration remains pending. Full installation inventory correction is committed in the formal test source above. `CROSS_PLATFORM_REVIEW_REQUIRED` for the small inventory correction has been reviewed and targeted Linux validation is complete. Shared stop-aware WebSocket source review and Linux targeted tests are complete; this is not Winsock validation. Shared UI authoritative state / automatic-options behavior remain follow-ups; preserve historical UI FAIL until reproduced on the exact Windows artifact with redacted runtime fields.

Prior Linux validation for the integrated source and this documentation reconciliation: Desktop Node workspace tests, Extension tests/check/build, Cargo format, protocol/Host tests, WebSocket targeted tests, docs audit and whitespace checks PASS (docs audit and targeted tests re-run in this batch). Full regression was not run; no Windows checks are claimed for this follow-up.

After formal handoff, Windows Owner fetches the pushed exact SHA, rebuilds an identified Full artifact, then performs registration, sidebar/detail + runtime field capture, 5s/45s idle, status query, authorized archive/duplicate/recovery, worker/restart/sleep, profiles/ACL, install/upgrade and redaction checks. This Linux session did not have Windows GUI/browser/Registry or Windows Rust-target capability: those checks are blocked here, not product failures. Manual steps are consolidated in [Windows manual steps §L](../validation/windows-manual-steps.md#l-browser-automatic-pairing--current-integrated-source-handoff-pending); queue IDs and prerequisites are in [current queue](../validation/windows-queue.md). Prior failures and automation blockers remain bound to their original artifacts.

Commands, exact-source Windows receipt and artifact hashes: [Windows validation history](../validation/windows-validation-history.md#2026-10-02--exact-6060f5a-windows-validation-return). Manual steps and current queue are linked above.

### Full manual evidence update — 2026-10-05

On the ca25e53 Full artifact above, Owner manual evidence establishes PASS for initial automatic browser connection, task creation, Telegram authentication and explicit Channel test receipt. Real download FAIL: gallery-dl extraction DOWNLOAD_TIMEOUT; confirmation dialog ACL errors also observed, triggering action unknown. Entire integration acceptance remains pending. See the latest Full manual integration record in Windows history and queue. Remaining archive/duplicate/automatic Telegram media and lifecycle checks stay NOT_RUN. Next Owner: Cross-platform Owner for CROSS_PLATFORM_REVIEW_REQUIRED extraction diagnostics and confirm authorization review; Windows Platform Owner retains native reproduction and manual acceptance. This documentation commit is the return revision; no production code changed.

### Reconnect and standalone extractor follow-up — 2026-10-05

Owner confirmed standalone gallery-dl succeeds with the same Full-bundled executable and authorized X URL, while XArchive remains timed out. This narrows but does not establish the invocation-context cause. Empty `_staging` confirms directory setup only. Owner also observed no automatic Extension reconnection after Desktop restart; clicking Reconnect restored authenticated/connected without entering a token. Timed recovery remains FAIL observed; manual reconnect PASS. Captures show stale form port versus active port in one session; investigate as a separate display issue, without assuming simultaneous-state corruption. See Windows history/queue for exact evidence. Next: Cross-platform Owner reviews extraction diagnostics and confirm ACL path; Windows Platform Owner reproduces and completes manual acceptance.
