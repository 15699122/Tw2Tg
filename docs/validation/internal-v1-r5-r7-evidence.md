# InternalV1 R5–R7 Linux integration evidence

Date: 2026-11-08. Owner: Cross-platform Owner. Entry source:
`f183e0f5d77ab99d681f10cb70912edd9ed33c63`, branch
`cross-platform/automatic-pairing-reconcile-20261002`.
Implementation-source commit: `3e97f4398e9c5803b5beb5bdf0c533fac9d62a72`.
The subsequent documentation-only handoff identifies this revision explicitly. Tests below were run on
the dirty implementation tree, not falsely attributed to an unchanged entry HEAD.

## Scope and expected invariants

R5 recovery, runtime/control and session extraction; restoring one lost traits
doc comment; narrowing fallback visibility to the original executor subtree;
R6 keep-together review; R7 integration and stale R4 static-wiring-test repair.
No IPC name/payload, storage schema, recovery authority, enrollment or ADR-015
lifecycle order changes. Existing tests remain in the parent; their bodies and
assertions were not moved or weakened merely to reduce its line count.

## Equivalence review and corrections

| ID / target | Result | Method and evidence |
|---|---|---|
| R5-FACTORY-EQUIV, all moved declarations/attributes/bodies and parent residual | PASS | Exact complete-region comparison of Git `4c208585 → f183e0f5`, not selected brace blocks. Includes decoder, both inherent impls, fallback struct/impl and moved test. Only exact added test imports/visibility and reviewed parent preamble changes accepted. Decoder and parent-lifecycle mutation controls rejected. `r5-strict-equivalence.log`, audit exit 0. |
| R5-RECOVERY-EQUIV, original recovery behavior | PASS | Complete report types/impl plus both Linux replay entry points compared to `f183e0f5`; every residual parent byte matches before subsequent slices. Only three `pub(super)` bridges; integrity-bypass mutation rejected. Same log, audit exit 0. |
| R5-RUNTIME-SESSION-EQUIV, lifecycle bodies and untouched residual | PASS | Four complete runtime regions and complete session region compared after exact visibility changes and rustfmt. Residual parent declarations, persistence, coordinator and tests byte-identical. `r5-runtime-session-equiv.log`, audit exit 0. |
| R5-TRAITS-EQUIV, complete contract/in-memory declarations | PASS | Four traits, two structs, **two** complete impls, preserving strings/attributes/comments. Exact three `pub(super)` fields and explanatory comment accepted. Lost `JobDatabaseFactory` doc restored. Unconditional compatibility exports checked. `r5-traits-strict3.log`, audit exit 0. |
| TG-RESTORATION-BODIES, Telegram production restoration | PASS (production items) | Source `370b10eb^` against current unchanged Telegram sources: 154 complete declarations/impls, 139 exact and 15 limited to reviewed visibility/docs/rustfmt changes; no body differences. Full impl identity avoids losing same-named methods. Includes `upload_watchdog`, formerly omitted by name-only audit. `telegram-complete-items2.log`, audit exit 0. This is not a test-body relocation proof; Telegram's 54 executed tests separately pass in workspace regression. |

The previous traits count of “3 impls” was incorrect: both original and extracted
module have two relevant impls. Earlier factory/traits selected-block audits and
their broad PASS claims are superseded, not deleted. They stripped comments,
omitted items or accepted arbitrary differences. No product defect was inferred
from a tooling failure. Initial recovery/runtime/session compiler and test-compiler
failures from missing imports/private sibling-test access are retained; corrected
with explicit imports and executor-subtree visibility, not behavior changes.

## Executed validation

All logs are retained under `.tw2tg-tmp/agent-scratch/build-logs/`.
Cargo jobs use one shared target serially. The Python harness records raw child
return codes and refuses zero-selected-test success; delegated execution records
the real `timeout → systemd-run --wait --pipe → cargo` exit, not grep's status.
The `r5-*-summary.json` files include source HEAD, dirty inventory and SHA-256 of
each executor source; the implementation commit is the durable source identity.

| ID / target | Status | Command / executed result / evidence |
|---|---|---|
| R5-DESKTOP, affected module regressions | PASS | `cargo test -p xarchive-desktop --lib -- --test-threads=1`: 365 passed, 13 ignored, Cargo exit 0. `r5-session-verified2-desktop.log`. |
| R5-CLAIM-FAILURE, failed factory releases lifecycle resources | PASS | Executed `factory_failure_finalizes_the_claim_and_releases_admission_and_scopes` in the above run. Asserts durable FAILED decision, attempt 1, failure event, peer Job/destination reacquisition and runner readmission. |
| R5-DELEGATED, production finalization/recovery/refusal/drain/replacement | PASS | Five exact tests, one executed test per command, all child exits 0. `r5-final-delegated-summary.json` and its five raw logs. |
| R7-WORKSPACE, integration regression | PASS | `cargo test --workspace --no-fail-fast -- --test-threads=1`: 852 passed, 24 ignored across 32 target results, Cargo exit 0. `r7-workspace.log`. Includes Storage 178, Telegram 54 and Workflow 49. |
| R7-SIDECAR, protocol/cancellation/extraction regression | PASS | `.venv/bin/python -m pytest sidecar/tests -q`: 101 passed, exit 0. `r7-sidecar.log`. |
| R7-FRONTEND-BUILD, frontend build plus extension syntax | PASS | `npm run check`: exit 0. `r7-frontend.log`. No Windows executable/GUI acceptance inferred. |
| R7-FRONTEND-TESTS, static wiring and JS regressions | PASS after repair | `npm test`: Desktop 230 and Extension 52 passed, exit 0. `r7-frontend-tests3.log`. Earlier run failed five obsolete `commands.rs`/old command-path lookups; intermediate repair still pointed archive directory at settings. Both failures retained in `r7-frontend-tests.log` and `r7-frontend-tests2.log`. Final tests inspect settings/logs/archive/jobs responsibility modules and exact `commands::telegram::<name>` registrations; all existing assertions retained. |
| R7-IPC, unchanged registration surface | PASS | 78 declarations, 78 unique handler entries, identical names. `continuation-ipc.log`, audit exit 0. |
| R7-CHECKER-SELFTEST, governance tooling | PASS | `node --test scripts/check-architecture.test.mjs`: 12 passed, exit 0. `continuation-architecture-selftest.log`. |

Final tracked-set gates: fmt, docs audit (153 tracked Markdown, no dead links),
staged diff check and architecture checker all exit 0. Before reseal the checker
reported 7 baseline-only findings (5 missing extracted modules and 2 line deltas).
The one authorized R7 reseal covers 189 files; the post-reseal checker reports
44 JS/TS nodes, 22 relative edges and 0 findings. The residual executor exception
was reviewed for its roughly 8900 unchanged regression-test lines, not silently
removed or used to skip modularization. Evidence: `r7-final-checks.json` and
`r7-architecture-before-reseal.log`. The existing two library warnings
(`InMemoryCommitRecoveryFactsProvider` unused import and `record_recovery_results`
dead code) remain; no new warning is accepted silently.

## Deferred work and ownership

- Windows native drain evidence, process-tree containment/shutdown, NTFS
  no-replace/coordination, MSVC integration and GUI/packaging are Windows-owned,
  implementation pending and verification NOT_RUN until the formal Git handoff.
  No Windows checks were executed here. They do not block Linux refactoring.
- V1-02c/d have no reachable legitimate permit-bearing mismatch constructor;
  retain reachability analysis and negative fences, not forged permits or a
  fabricated production PASS.
- aria2 remains user-deferred. Its earlier attempted fixture failed because the
  executable/prerequisite was absent. No real aria2 acceptance PASS is claimed;
  the failed attempt is not rewritten as “never attempted”.
- Existing ignored tests retain their own platform/fixture prerequisites. The five
  explicitly executed delegated checks above do not promote every ignored test.

Follow-up and exact revision: [current handoff](../status/platform-handoff.md).
R6 review: [responsibility record](../architecture/modules/remaining-capability-review.md).
## Committed-source confirmation

On `3e97f4398e9c5803b5beb5bdf0c533fac9d62a72`, with no tracked source changes,
Desktop 365 passed / 13 ignored and the five exact delegated checks were re-run;
fmt, lib check and diff also exit 0. JSON summaries
`r7-committed-source-summary.json` and `r7-committed-delegated-summary.json`
bind all executor source hashes and the retained untracked backup inventory.
The following handoff commit changes documentation only; reuse is valid because
source, dependencies and runtime assumptions are unchanged.
