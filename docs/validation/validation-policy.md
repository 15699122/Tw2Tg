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

## Computer Use fallback

Run GUI and Computer Use checks after non-GUI checks. If automation is unavailable, allow limited retry, then mark the check BLOCKED with blocker COMPUTER_USE_UNAVAILABLE; continue independent checks and add a Manual Windows Validation Queue item. A GUI automation failure is not evidence of a product failure. An equivalent CLI/API/log/filesystem/process check may be PASS only when it proves the same target.

Manual items record ID, name, related change, status, blocker, purpose, prerequisites, numbered steps, expected result, evidence, result, and notes.

Each round records source revision, change scope, selected and skipped tests, selection reasons, results, deferred tests, and manual requirements. Windows-specific failures are handled by the Windows Owner; shared-contract or shared-behavior findings are CROSS_PLATFORM_CHANGE_REQUIRED and return to Linux with reproduction, evidence, root-cause hypothesis, affected modules, impact, and recommended change.
