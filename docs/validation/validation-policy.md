# Unified Validation Policy

Validation is risk-based and incremental. Select the smallest scope that reasonably covers the current change: Targeted -> Module -> Subsystem -> Full.

Before testing, inspect the Git diff, changed modules, dependency/call relationships, API or schema changes, platform behavior, and reusable previous results. Prioritize affected unit tests, regression tests, affected integration tests, then module-level tests and relevant formatter/lint/type/build checks.

Full regression is normally reserved for releases, major architecture or core changes, migrations, dependency overhauls, large cross-module changes, security-sensitive changes, or uncertain blast radius. If Full is not run, record: Full regression not run for this change set, and explain why.

## Deferred platform validation

For each Windows Release or pre-Release, asset completeness, nonempty files, tag/source parity and manifest/SHA-256 integrity remain blocking upload checks. Windows WDIO runs in a separate job against the executable uploaded by the **same build run**, with its run ID, source SHA and executable SHA-256 verified before execution. WDIO FAIL/BLOCKED alone does not block publishing; preserve the actual result, environment, driver/WebView2 details, logs and diagnostics artifact. Missing input or a failed identity check is NOT_RUN (not PASS); a missing or mismatched release asset is a blocking packaging failure. A green build or publication is not Windows GUI acceptance. If the validation job was not started, record NOT_RUN and arrange a Windows Owner follow-up. Never reuse a previous version's PASS.

Windows work that is not a hard prerequisite is accumulated as WINDOWS_WORK_PENDING or WINDOWS_VERIFICATION_PENDING. Use WINDOWS_BLOCKING only when continuing development would be unreliable without Windows behavior.

Previous PASS results may be reused only when related code, dependencies, platform contracts, and environment requirements remain unchanged. Otherwise record REVALIDATION_REQUIRED.

## Result states

Every applicable item uses PASS, FAIL, BLOCKED, NOT_RUN, or NOT_APPLICABLE. Every non-PASS result includes its reason, prerequisite, evidence, and follow-up. Do not promote a build, mock, startup, or smoke test into GUI, native integration, real account, installer, signing, or external-service acceptance.
## Separating implementation from execution

Implementation status, verification result, deferral reason and ownership state are four different things and are recorded separately:

| Dimension | Values |
|---|---|
| Implementation | `PLANNED`, `IN_PROGRESS`, `IMPLEMENTED` |
| Verification | `PASS`, `FAIL`, `BLOCKED`, `NOT_RUN`, `NOT_APPLICABLE` |
| Deferral reason | e.g. `IMPLEMENTATION_NOT_READY`, missing capability, missing account |
| Handoff | states defined in [`../development/platform-ownership.md`](../development/platform-ownership.md) |

Rules:

- An unimplemented feature is `PLANNED`. It is never `FAIL` and never `PASS`.
- Implemented but deliberately not executed this round is `NOT_RUN` with the specific reason. `NOT_APPLICABLE` means the scope itself does not apply.
- **Skipping is not passing.** Every non-PASS result carries a reason, an evidence location and a follow-up.
- A check that fails and then passes on retry records both facts. The result is not silently rewritten to `PASS`; stability is recorded as intermittent with a follow-up observation.

## Execution capability

Before declaring a check BLOCKED or NOT_RUN, determine what the current environment can actually do: OS and execution environment, shell and sandbox permissions, GUI and automation availability, MCP and network reachability, and the artifact identity the evidence must bind to.

Prefer the least invasive permitted path, and use an alternative permitted path when it yields equivalent evidence for the same target. Do not weaken product behaviour, assertions or acceptance criteria to accommodate an unavailable tool. An alternative may be recorded as PASS only when it proves the same acceptance target; a different check is a different check.

Capability never changes ownership: obtaining Windows evidence through an approved channel does not transfer Windows implementation ownership, and the ability to edit shared code does not grant authority over a shared contract.

## Tooling failure

A tooling, sandbox, automation, MCP, network or environment failure is not by itself a reason to change production code. Before changing product code in response to a failure, determine whether it reproduces independently of the tooling, separate a product defect from an environment or tool defect, record tooling failures separately from product results, and modify product code only when evidence indicates a product-level defect.

## Computer Use fallback

Run GUI and Computer Use checks after non-GUI checks. If automation is unavailable, allow limited retry, then mark the check BLOCKED with blocker COMPUTER_USE_UNAVAILABLE; continue independent checks and add a Manual Windows Validation Queue item. A GUI automation failure is not evidence of a product failure. An equivalent CLI/API/log/filesystem/process check may be PASS only when it proves the same target.

Manual items record ID, name, related change, status, blocker, purpose, prerequisites, numbered steps, expected result, evidence, result, and notes.
## Required evidence fields

Every validation record, whether automated or manual, carries these fields. A result without its evidence field is not a record.

| Field | Requirement |
|---|---|
| `id` | Stable identifier, reused across rounds |
| `target` | What behaviour is being proven |
| `priority` | P0 / P1 / P2 |
| `owner` | Cross-platform or Windows Platform |
| `implementation` | `PLANNED` / `IN_PROGRESS` / `IMPLEMENTED` |
| `status` | `PASS` / `FAIL` / `BLOCKED` / `NOT_RUN` / `NOT_APPLICABLE` |
| `defer_reason` | Required for every non-PASS status |
| `source_sha` | Exact revision the result binds to |
| `build_origin` | Workflow run, build ID or local build command |
| `artifact_sha256` | Hash of the exact executable or archive tested |
| `platform` | Target OS and architecture |
| `environment` | Host OS, display scaling, profile/VM, browser |
| `tool_versions` | Driver, WebView2, Node, Rust, Python, CLI |
| `method` | `automated`, `gui-automated`, `integration`, `smoke`, `static`, `manual`, `delegated` |
| `prerequisites` | Accounts, fixtures, registry state, services |
| `steps` | Numbered, reproducible |
| `expected` | Expected result, written before execution |
| `evidence` | Command and exit code, logs, screenshot, artifact path |
| `blocks_development` | Whether it blocks further cross-platform work |
| `blocks_release` | Whether it blocks publication |
| `revalidation` | Conditions under which this result may be reused, else `REVALIDATION_REQUIRED` |
| `follow_up` | Next action and owner |

`method` and `platform` are separate fields: delegation describes who executed the check, not what kind of check it was, and an automated method on a Windows host is still a Windows result.

A `PASS` may be reused only when the related code, dependencies, platform contracts and environment requirements are unchanged.

Each round records source revision, change scope, selected and skipped tests, selection reasons, results, deferred tests, and manual requirements. Windows-specific failures are handled by the Windows Owner; shared-contract or shared-behavior findings are CROSS_PLATFORM_CHANGE_REQUIRED and return to Linux with reproduction, evidence, root-cause hypothesis, affected modules, impact, and recommended change.
