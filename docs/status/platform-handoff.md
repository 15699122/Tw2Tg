# Current Platform Handoff

This file contains only the current handoff batch, following the template in ../development/git-platform-handoff.md. Historical Windows results belong in ../validation/windows-validation-history.md.

## Batch

- Task: Reconcile Windows WQ-U12 round; pin shared Windows Named Pipe endpoint contract (`\\.\pipe\xarchive-v1`) in `xarchive-protocol`; Native Host Windows default-endpoint fallback; document E5 state.
- Branch: feature/u7-desktop-production-integration
- Current owner: Cross-platform Owner -> Windows Platform Owner
- Current state: READY_FOR_WINDOWS

## Revisions

- Cross-platform input revision: `59c8221`
- Cross-platform handoff revision: `c40f312` (this record follows it in the next commit)
- Windows input revision: PENDING (Windows records actual checkout; use branch tip >= `c40f312`)
- Windows implementation revision: PENDING
- Windows validation revision: PENDING

## Cross-platform Work Completed

- Fixed shared endpoint contract in `xarchive-protocol`: `WINDOWS_PIPE_ENDPOINT = \\.\pipe\xarchive-v1`; `PIPE_ENDPOINT_ENV` migrated there (native-host re-exports it; Desktop no longer hardcodes the string); contract pin test `pins_shared_transport_endpoint_contract`.
- Native Host Windows client falls back to the shared default pipe name when `XARCHIVE_PIPE_ENDPOINT` is unset; the variable is now a diagnostic override only. Unix behavior unchanged (env still required; portable-root socket path).
- `desktop/src-tauri/src/transport.rs` docs now state the Windows backend (roadmap E5, Windows Owner) must listen on `WINDOWS_PIPE_ENDPOINT`.
- `docs/development/roadmap.md` E5 progress row added (`LINUX_CONTRACT_DONE / WINDOWS_WORK_PENDING`).
- Validation (minimal necessary; full suite not run - small contract + wiring change, no schema/migration/architecture impact): `cargo test -p xarchive-protocol -p xarchive-native-host` 17+8 PASS (Targeted); `cargo check -p xarchive-native-host --target x86_64-pc-windows-msvc` PASS (compile-only proof of the `cfg(windows)` branch); `cargo test -p xarchive-desktop --lib` 87 PASS (Module, covers all transport tests); `cargo fmt --check` clean.
- Reconciliation: no new Windows commits since `ad220a8`; WQ-U12-01 `WINDOWS_PASS` (package boundary only), WQ-U12-02/03/04 `WINDOWS_BLOCKED`. Root cause confirmed: Desktop transport server exists only `cfg(unix)`; setting `XARCHIVE_PIPE_ENDPOINT` alone never creates a server. No pending `CROSS_PLATFORM_CHANGE_REQUIRED` / `CROSS_PLATFORM_REVIEW_REQUIRED`.

## Windows Work Required

- E5: implement Desktop Windows Named Pipe server listening on `xarchive_protocol::WINDOWS_PIPE_ENDPOINT` by default; per-connection read/respond using the existing 4-byte LE length + JSON framing (reuse `xarchive_native_host` framing helpers), same `BrowserRequest`/`BrowserResponse` contract as the Unix adapter; ACL restricted to the current interactive user; multi-connection handling; stop accepting + clean up on exit; reconnect/error behavior.
- Wire listener start-up on the Windows runtime path (currently only the Unix branch exists in `runtime.rs`).
- E6: HKCU per-user Native Host registration (Edge/Chrome), manifest, allow-list. E7: connection-status observable facts.
- Windows runtime/ACL/reconnect verification of the `cfg(windows)` fallback (cross-target compile-only was verified from Linux).
- Mark `CROSS_PLATFORM_*` only if the shared contract itself must change.

## Windows Validation Required

- Rebind to the new handoff revision and clear when the E5 listener lands: `WQ-P0-04`, `EXT-W-NATIVE-04`, `WQ-NATIVE-20260920-03`, `WQ-P1-02`, `WQ-U12-02/03/04`, `MANUAL-WIN-IPC-01` (currently `WINDOWS_BLOCKED` / `WINDOWS_VERIFICATION_PENDING`).
- Verify: default pipe name connects with no env var; env-var diagnostic override still honored; `NATIVE_PIPE_UNAVAILABLE`/`NATIVE_PIPE_ERROR` transitions; ACL limited to current user; worker/Extension identity/framing facts remain valid (package-boundary PASS must not be reported as Native Messaging end-to-end PASS).

## Expected Behavior

- Desktop (Windows) listens on `\\.\pipe\xarchive-v1`; Native Host reaches the same default with no environment wiring; `XARCHIVE_PIPE_ENDPOINT` overrides only for diagnostics.
- Unix Desktop/Native Host behavior unchanged: socket at `portable_root/cache/xarchive-v1.sock`; Native Host still requires the env var.

## Known Risks

- The `cfg(windows)` fallback branch is proven by cross-target `cargo check` only; no Windows runtime evidence yet.
- Historical `windows-queue.md` entries cite env-var-only behavior (`NATIVE_PIPE_UNAVAILABLE` when unset); that branch now applies to Unix only - the queue remains valid as history.
- Edge-launched Native Host inherits Edge's environment: the default pipe name is the reliable path; the env override must not be required in production.

## Windows Results

Implementation:
- pending

PASS:
- pending

FAIL:
- pending

BLOCKED:
- pending

Manual validation required:
- pending

## Cross-platform Follow-up

CROSS_PLATFORM_CHANGE_REQUIRED:
- none

CROSS_PLATFORM_REVIEW_REQUIRED:
- none

## Next Owner

Owner: Windows Platform Owner

Required actions:
- fetch remote; confirm working tree clean; update to branch tip (>= `c40f312`); verify revision against this file;
- run the Windows batch in "Windows Work Required" and the queued validations above;
- record input/implementation/validation revisions, commit, push; return ownership only if `CROSS_PLATFORM_*` follow-up exists.

Update this file for the active batch only; move completed outcomes to the history document.
