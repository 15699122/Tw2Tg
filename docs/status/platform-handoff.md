# Current Platform Handoff

Status: `CROSS_PLATFORM_IN_PROGRESS` — Telegram TG-06 resumed; current batch is limited to state reconciliation and a shared-contract blocker review. No Telegram production code has been wired. Current Owner: Cross-platform Owner; next action: resolve the shared outbox identity/idempotency and durable payload/config-snapshot contract before production enqueue or scheduling. Windows remains the owner of Credential Manager, Local Bot API Server deployment, GUI, packaging and real-account acceptance; no Windows handoff is implied by this status update.

## Active Telegram TG-06 resumed batch — 2026-10-03

- Branch: `cross-platform/automatic-pairing-reconcile-20261002`.
- Source commit: `808fe31c7d359f1d1311c89526ed9a8caa6a2f6c`.
- Handoff commit: none; working tree contains uncommitted documentation updates.
- Uncommitted state: README, development status, Telegram plan and Windows manual steps only; no production code changed.
- Current Owner: Cross-platform Owner. No Windows execution or platform evidence was produced.
- Completed: Telegram development status reconciled as resumed on 2026-10-03, preserving the historical `v0.2.1-pre1` exclusion; TG-06 Plan now records the blocker as `CROSS_PLATFORM_CHANGE_REQUIRED`.
- Blocking shared contract: current `telegram_outbox` uniqueness is `(chat_id, idempotency_key)` rather than bot-scoped, and durable rows do not capture enough planned content/config to reconstruct pending work safely after restart or a settings edit. Production enqueue/claim-loop must not use current settings as a substitute snapshot.
- Next: Linux Cross-platform Owner decides/additively implements bot-scoped idempotency and durable payload/config snapshot/reconstruction semantics with migration and regression tests; then wire archive enqueue, token-secret boundary, Tauri commands, bounded claim scheduling and task projection. Windows credential adapter and platform acceptance follow through formal Git handoff.
- Validation: `git diff --check` PASS; `node scripts/docs-audit.mjs` PASS. Rust tests not run because this batch changed documentation only and production wiring is gated on the shared-contract decision.

- Formal Windows test source: `9920566ef1df115effc3ab5df5d9a12fd42210ed` (includes `5ff0a22` Windows integration).
- Handoff documentation branch: `cross-platform/automatic-pairing-reconcile-20261002`. This commit records the documentation checkpoint authorizing Windows retest of the formal source.
- Incoming source reconciled in this batch: `6060f5a2d554d32fa8400f7f5084319b19fabeb1`; Windows return `ebdc44db5ae17ef4189532d06dbb6f6a22a629ae` on `codex/windows-validation-6060f5a`.
- Earlier Windows implementation `d65a01bfe2c6f93a071322bfef154fd947b0b70b` and evidence `30fc57588065b73677473d411368219eda3313a7` are based on `0c74087cf26d7120dfe6bbabb7c66b37b879fe3e`, not on the tested source. Integration commit `5ff0a22` carries identical `crates/` + `desktop/src-tauri/` code to d65a01b (verified by empty diff on those trees); d65a01b Windows PASS remains bound to its own source/artifact and is not claimed for the integrated source.

Exact `6060f5a` Windows evidence: targeted package/UI Node tests 27/27 PASS; Native Host release build and canonical Full build PASS; Extension directory inventory PASS scoped (13 files including helper); generated Full `installation.files` omitted the shipped helper (FAIL scoped); packaged Host returned `BOOTSTRAP_NOT_IMPLEMENTED`. Automatic pairing, UI reproduction, E2E and lifecycle were NOT_RUN / IMPLEMENTATION_NOT_READY on that source. Build success is not GUI or integration acceptance.

CROSS_PLATFORM_CHANGE_REQUIRED was addressed by integrating the Windows bootstrap through Git; Windows validation of that integration remains pending. Full installation inventory correction is committed in the formal test source above. `CROSS_PLATFORM_REVIEW_REQUIRED` for the small inventory correction has been reviewed and targeted Linux validation is complete. Shared stop-aware WebSocket source review and Linux targeted tests are complete; this is not Winsock validation. Shared UI authoritative state / automatic-options behavior remain follow-ups; preserve historical UI FAIL until reproduced on the exact Windows artifact with redacted runtime fields.

Prior Linux validation for the integrated source and this documentation reconciliation: Desktop Node workspace tests, Extension tests/check/build, Cargo format, protocol/Host tests, WebSocket targeted tests, docs audit and whitespace checks PASS (docs audit and targeted tests re-run in this batch). Full regression was not run; no Windows checks are claimed for this follow-up.

After formal handoff, Windows Owner fetches the pushed exact SHA, rebuilds an identified Full artifact, then performs registration, sidebar/detail + runtime field capture, 5s/45s idle, status query, authorized archive/duplicate/recovery, worker/restart/sleep, profiles/ACL, install/upgrade and redaction checks. This Linux session did not have Windows GUI/browser/Registry or Windows Rust-target capability: those checks are blocked here, not product failures. Manual steps are consolidated in [Windows manual steps §L](../validation/windows-manual-steps.md#l-browser-automatic-pairing--current-integrated-source-handoff-pending); queue IDs and prerequisites are in [current queue](../validation/windows-queue.md). Prior failures and automation blockers remain bound to their original artifacts.

Commands, exact-source Windows receipt and artifact hashes: [Windows validation history](../validation/windows-validation-history.md#2026-10-02--exact-6060f5a-windows-validation-return). Manual steps and current queue are linked above.
