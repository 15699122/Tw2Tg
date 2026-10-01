# Current Platform Handoff

## Current batch: Windows published-asset validation (2026-10-01)

- Git input/handoff `bd40402d8d3a455911216e199233d66156d67657`; branch `codex/windows-validation-v020`, tracked tree clean before work. Product implementation/tag `7910033c9bdb2c649383ee9ddc7af063b258c8c7` (v0.2.0); validation revision is the commit containing this record. No code change or filesystem synchronization.
- Published seven assets downloaded; manifest/source/hash/size and four archive tests/extractions PASS. Actual native Computer Use observes Full/Core/standalone Dashboard/version; Full directory initialization, Sidecar ready/stop/worker exit and config restart PASS. Windows Native Pipe targeted tests 2/2 PASS. No full regression or old failed WDIO recipe rerun.
- G4 PARTIAL (completed task persistence still unexecuted), G5 BLOCKED prerequisite / real archive NOT RUN, G7 PARTIAL (other-user ACL/junction/Unicode/long paths NOT RUN). G6 NOT_APPLICABLE for advertised scope; Telegram send NOT RUN. Historical release WDIO FAIL preserved; no new product FAIL. Final app/worker/driver/listener inventory empty.
- Evidence: `../validation/windows-validation-history.md` (2026-10-01); current manual queue: `../validation/windows-queue.md`. Local Full: `validation-artifacts/windows-v020-20261001/extracted/full`. CI Native Host absolute path needs supported registration after extraction, not installed acceptance.
- No CROSS_PLATFORM_CHANGE_REQUIRED / CROSS_PLATFORM_REVIEW_REQUIRED and no WINDOWS_VERIFICATION_BLOCKING. Next Owner: **Windows Platform Owner**, completing M-CAND-02 (including completed-task restart) and remaining M-CAND-04. State `WINDOWS_VALIDATION_PENDING`; Git reconciliation may occur later without a platform implementation handoff.

## Previous batch: v0.2.0 released under owner authorization (2026-09-30)

- **`v0.2.0` is published.** Tag `v0.2.0` -> `7910033c9bdb2c649383ee9ddc7af063b258c8c7`; Release https://github.com/15699122/Tw2Tg/releases/tag/v0.2.0 . `dev` and `main` were already identical at that revision before the release, so no extra synchronization was required. Actions run `36705896154`.
- **Build job PASS; seven assets uploaded and non-empty.** The exe SHA-256 was independently re-verified on Linux by downloading the asset: measured `fb193309f4f9e073c2b7784a1a410bc0eb1f608c02382b07c2d20ee45a45d6d4`, matching `SHA256SUMS-v0.2.0.txt`, the release manifest and the build identity. The manifest `source_sha` equals the tag commit and the Release target. **This closes gate G3**, which had been `NOT RUN`.
- **Independent WDIO job FAIL (non-blocking in the workflow).** Identity check and preflight both succeeded and the identity exe hash matches the published asset; session creation failed three times with `session not created: DevToolsActivePort file doesn't exist`, so the Dashboard assertion was never reached. This is the previously recorded `tauri-driver` / WebView2 driver environment defect, not a regression from this code. No process or port was left behind. Evidence: artifact `XArchive-v0.2.0-wdio-diagnostics-36705896154`.
- **Authorization and its conditions.** The Owner explicitly authorized publishing with the gates not fully closed (option B), on the conditions that Telegram must not be advertised as implemented and that the notes must present this as a developer version prior to v1.0 while claiming only documented, verified items. The published notes comply: they list the verified set, state the unverified scope explicitly, and mark Telegram as not verified and not available.
- **Still unverified after the release: G4 GUI (`WINDOWS_BLOCKED`, `COMPUTER_USE_UNAVAILABLE`), G5 extension pairing and a completed archive (`WINDOWS_BLOCKED`), G6 Telegram (`NOT RUN`), G7 filesystem and transport semantics (`NOT RUN`).** Publishing the tag does not close any of these. The WDIO failure is independent evidence that GUI acceptance is still missing.
- Record: `../releases/v0.2.0.md`. Next Owner: **Windows Platform Owner**, to run the M-CAND-01..04 manual acceptance queue against `v0.2.0` and close G4, G5 and G7. State: `READY_FOR_WINDOWS`.

## Previous batch: Windows results integrated, candidate frozen (2026-09-30)

- Handoff revision: this record is introduced by the documentation commit immediately preceding the final documentation commit; both are on `dev` and fast-forwarded to `main`, and the **final `dev`/`main` revision is the handoff revision** reported in the batch summary. A commit cannot contain its own SHA, so this line deliberately names the record rather than a hash. **Freeze candidate for validation: `fe3feee7e1f6089b370d64c32f484dbe1fd438b5`** (the code tree; the two documentation commits after it change no product code). Git-only exchange; no direct filesystem synchronization; existing Windows local artifacts, caches and credentials preserved.
- Input was the previous handoff `5f952d0091644775ec633a9e62a1696fe71fd471`, with Windows revisions `951453c` (workflow/build), `4d1d914` (fixture) and `fe3feee` (validation/handoff) on `codex/windows-validation-5f952d0`. All three are now integrated by fast-forward with no merge commit.
- `CROSS_PLATFORM_REVIEW_REQUIRED` is **discharged**. The reviewed change is the shared test fixture in `desktop/test/wdio-config.test.mjs`: it writes a zero-byte placeholder so the Windows-only `existsSync` guard in `desktop/wdio.conf.mjs` is satisfied while the test only imports configuration. Linux evidence: the test passes without the fixture because the guard is behind `process.platform === "win32"`, and a direct probe of that guard shows it would throw `EdgeDriver executable not found` with no placeholder and proceed with one. The fixture is required on Windows, does not weaken production validation, and the placeholder is never spawned. No shared contract, API, schema or data model changed. No `CROSS_PLATFORM_CHANGE_REQUIRED` was raised by the Windows batch.
- Code scanning alert #2 (`actions/missing-workflow-permissions`) is **closed**: the workflow now declares `permissions: contents: read` and `persist-credentials: false`, was executed on Windows as run `36700506149` (SUCCESS), and the default-branch rescan marked the alert `fixed` at 2026-09-30T10:40:33Z with `dismissed_by` null. Open code-scanning alerts: 0. Open Dependabot alerts: #1 `glib`, #4 and #6 `extract-zip` — unchanged and tracked in `../development/security-remediation-plan.md`.
- `windows/webview2-readiness-gate` is dispositioned **superseded**; both of its `fix(windows):` commits are already present, so Linux did not merge it.
- Linux validation actually executed on the integrated tree after `rm -rf node_modules && npm ci`: desktop 154/154, extension 33/33, `npm run check` PASS, `npm run build` PASS, `cargo fmt --check` PASS, `cargo check --locked --all-targets` PASS, strict Clippy `-D warnings` PASS, `cargo test --workspace --locked` 262/262, `pytest sidecar/tests` 46/46, `git diff --check` PASS. No Rust, Python, dependency or lock-file change was involved.
- **Release state unchanged: no `v0.2.0` tag and no release exist.** The freeze candidate is `fe3feee`, but G4 is `WINDOWS_BLOCKED` (`COMPUTER_USE_UNAVAILABLE`), G5 is `WINDOWS_BLOCKED` pending a stable application and dedicated accounts, and G6 and G7 are `NOT RUN`. Release scope stays Windows portable Core/Full plus Extension; installer, signing, updater and a Linux GA artifact remain deferred. Whether Telegram send is advertised is still an open decision.
- Next Owner: **Windows Platform Owner**, for the M-CAND-01..04 manual acceptance queue in [`../validation/windows-queue.md`](../validation/windows-queue.md) and the G3 formal seven-asset release gate against the frozen candidate. State: `READY_FOR_WINDOWS`. No `WINDOWS_VERIFICATION_BLOCKING` for continued Linux cross-platform work; release approval remains blocked by the unmet acceptance gates, and the absence of a blocking item is not a release approval.

## Previous batch: Windows results and canonical integration (2026-09-30)

- Input/handoff `5f952d0091644775ec633a9e62a1696fe71fd471`, integration tree `60110a6`; branch `codex/windows-validation-5f952d0`. Workflow/build `951453cd55a06b913836e9fa6dd30314ffbf631c`; fixture implementation `4d1d9148dd13f187a734aa994e8ba4c796a33bd0`. Validation/handoff record revision is the commit containing this section. Git-only exchange; existing local artifacts preserved.
- Phase 3 G1 review PASS: both old Windows fixes superseded by current code; no wholesale merge. Worker permission fix/run `36700506149` PASS; alert #2 still open on default branch pending integration/rescan. Targeted 52/52 and Extension 33/33/check PASS. Initial fixture FAIL corrected without weakening production validation.
- CROSS_PLATFORM_REVIEW_REQUIRED: shared test-fixture change in `desktop/test/wdio-config.test.mjs`. No shared contract change. Next Owner: Cross-platform Owner for review, integration, default-branch CodeQL and release scope/SHA. Windows retains subsequent native acceptance. No WINDOWS_BLOCKING for continued development.
- Fresh Desktop/Native Host/CI worker and local Full assembly PASS; four real 7z shape probes PASS. This is not the final seven-asset release gate; Native Host path requires supported registration after relocation. Evidence: `../validation/windows-validation-history.md`.
- G4 GUI BLOCKED / COMPUTER_USE_UNAVAILABLE; React mount logs alone are not GUI PASS. G5 pairing/completed archive, G6 Telegram, G7 filesystem remain BLOCKED/NOT RUN as recorded in `../validation/windows-queue.md`. Manual queue M-CAND-01..04 covers concrete steps. Exact launched process cleaned; no driver/worker residuals. No v0.2.0 tag/release created; gates remain unmet.

## Previous batch: branch convergence and v0.2.0 release gating (2026-09-30)

- Plan: [`../development/branch-integration-release-plan.md`](../development/branch-integration-release-plan.md). **`dev` and `main` are both at `60110a6f000887d134bf5fc46a7b2d6da91635a8`; `dev` is an ancestor-free fast-forward of `main`, so the two serve identical content.** This is the Windows start revision. Handoff is through Git only; no direct filesystem synchronization was used. Current owner: Windows Platform Owner. State: `READY_FOR_WINDOWS`.
- What was integrated, per the Phase 2 disposition: `dev` was fast-forwarded from `1786c6a2` to `main`; PR #11 (squash `401a760`) added the integration/release plan; PR #12 (merge `60110a6`) ported the two non-duplicate assertions from the dev backport. Branches recorded as already included were **not** re-merged: `feat/extraction-aria2-pipeline`, `feature/u7-desktop-production-integration`, `release/v0.2.0`, `security/tweet-url-host-validation` (#5) and `security/dependency-advisories-2026-09-30` (squash-landed via #9/#7). `release/v0.1.1` and `release/v0.2.0-pre.1` are kept as historical release records and were not merged.
- Change from this batch: `extension/tests/content.test.js` only, test-only, +8 lines. Two assertions were added that the backport PR #10 exercised but `main` did not pin: credentials on an allowlisted host (`https://user:pw@x.com/alice/status/123` -> `null`) and a status link with trailing path segments (`https://x.com/alice/status/123/extra` -> accepted). PR #10 was closed as superseded; a URL-corpus comparison after normalizing numeric IDs showed 6 shapes unique to it, all of which already return the correct value under the `main` implementation, so no behavior was lost. **No runtime code, dependency, lock file or workflow changed.**
- PR #5 shows as `MERGED` (`61ff1ae`) only because advancing `dev` made its head an ancestor of the base branch; GitHub closed it automatically. `edab0d698e6e80275459bcefb59f28e93b212472` was already contained in `main` before this batch, `61ff1ae` is a pre-existing historical commit, and the tree at `60110a6` is unchanged by it. No regression was introduced.
- Linux evidence actually executed on this batch: `npm run test --workspace extension` 33/33 (was 32); `npm run check --workspace extension` PASS; `git diff --check` PASS; `gh merge-base --is-ancestor` checks for the fast-forward and disposition decisions. PR #11 and PR #12 CodeQL (actions, javascript-typescript, python, rust) all SUCCESS, both `CLEAN` before merge. The broader `main` gate from the previous batch (`npm ci`, desktop 154/154, `cargo test --workspace --locked` 262/262, `pytest sidecar/tests` 46/46, strict Clippy) was executed on `f3c0876`; this batch is test-only, so no product-code re-validation was required.
- **Release state: `v0.2.0` is NOT released and no `v0.2.0` tag exists.** Windows portable Core/Full plus Extension is the proposed scope; installer, code signing, the updater and a Linux GA artifact are deferred, not silently omitted. The release gates in the plan must be satisfied before any tag is pushed, because `windows-release.yml` triggers on `v*` and immediately builds and uploads. The absence of `WINDOWS_VERIFICATION_BLOCKING` is not a release approval.
- Still open on the default branch: Dependabot #1 `glib` (Linux-only, no compatible C1 upgrade path), #4 and #6 `extract-zip` (no upstream fixed version, RISK-023 accepted), Code scanning #2 (`actions/missing-workflow-permissions` in the Windows worker workflow). Secret scanning: 0 open.
- Windows action required (see [`../validation/windows-queue.md`](../validation/windows-queue.md)): WQ-SEC-PERMS-01; the Phase 3 disposition of `windows/webview2-readiness-gate` (integrate, extract selectively, or superseded — the two `fix(windows):` commits need Windows review and regression evidence); and the Windows release-scope acceptance for the candidate. Do not record any Windows PASS before executing them.

## Previous batch: security dependency remediation, Linux → Windows handoff (2026-09-30)

- Branch: `security/dependency-advisories-2026-09-30`; source base `origin/main` `c98d61061e3c09aa03ccd3609740fc5acb80cbe3`; implementation through `1d739085d19682df41604c3f82d9fd9d50f907e4`. **Merged to `main` via PR #9 (squash `ed8c1fcad0403169ac230ebbe924dff01f907056`) and PR #7 (squash `f3c087697881f18bf1f6dda2381216647e052c59`); merged `main` = `f3c087697881f18bf1f6dda2381216647e052c59` is the Windows start revision.** This record is a follow-up docs commit on `main`. Handoff is through Git only; no direct filesystem synchronization was used. Current owner: Windows Platform Owner. State: `READY_FOR_WINDOWS`.
- Shared change: `package-lock.json` only (`ip-address 10.7.0 → 10.7.2`, `brace-expansion 2.1.4 → 2.1.7`, `1.1.18 → 1.1.21`), plus new `SECURITY.md`, `.github/dependabot.yml` and documentation. No shared API, protocol, schema, data model, production code or Windows workflow changed. No `CROSS_PLATFORM_CHANGE_REQUIRED` and no `WINDOWS_BLOCKING`.
- Linux evidence on merged `main` `f3c0876` (all actually executed): `npm ci` PASS; desktop 154/154; extension 32/32; `npm run check` and `npm run build` PASS; `npm ls` shows 10.7.2 / 2.1.7 / 1.1.21; `npm audit` retains only the accepted `extract-zip` chain; `cargo fmt --check` PASS; `cargo check --locked --all-targets` PASS; strict Clippy `-D warnings` PASS; `cargo test --workspace --locked` 262/262; `pytest sidecar/tests` 46/46; all workflow and Dependabot YAML parse; `git diff --check` PASS. PR #9 CodeQL: actions, javascript-typescript, python, rust all SUCCESS.
- Default-branch rescan: Dependabot #11, #12, #14 (and incidentally #7–#10, #13) are now `fixed`. **Still open: #1 `glib`, #4/#6 `extract-zip`, Code scanning #2.** C1 concluded there is no compatible upgrade path (latest `wry 0.57.0` still requires `gtk ^0.18` + `webkit2gtk =2.0.2`); no Cargo dependency was changed. No Linux release-profile or GUI validation was performed.
- Windows action (P2, not blocking): WQ-SEC-PERMS-01 — add explicit least-privilege `permissions` to `.github/workflows/windows-worker-artifact.yml`, assess checkout credential persistence, run the worker artifact workflow and record the run ID, then recheck Code scanning #2. Consolidated Windows items: WQ-SEC-PERMS-01 (`WINDOWS_VERIFICATION_PENDING`), WQ-SEC-GLIB-01 and WQ-SEC-EXTRACTZIP-01 (`NOT_APPLICABLE`) in `../validation/windows-queue.md`. Do not record any Windows PASS before executing them; merged `main` changed no Windows workflow, so Windows runtime/GUI/packaging behavior is unchanged by this batch.

## Retained historical handoff: completed pre-release series migration (2026-09-30)

- Branch: `release/v0.2.0`; original Linux batch `ba0f8aa1fe4632917a3dde5990103508a44225b4`, formal handoff `7a3374b7af1a647addb0b9ab4e50e315468b6d26`. Windows fixes were executed at `e6d71ad44f8ef8cd8539259896787426e389ad67` and `2ec1aba78579932e7788fb3b3bb9e5fca597f906`. Final evidence/documentation commit is the commit containing this current-batch record; it will be exchanged through origin, without direct Linux filesystem synchronization.
- Current Owner: Windows Platform Owner; state: `WINDOWS_PASS` for automated migration. GUI/browser/installation acceptance remains `NOT_RUN` in the manual queue. Source changes are limited to Windows workflow handling; no shared contract change. Tracked tree is clean after the final evidence commit; existing untracked machine-local artifacts/caches are preserved and excluded.
- Ten `upload=false` runs PASS; ten `upload=true` draft runs PASS; all ten draft gates passed before any public release. Ten new preN pre-releases / 41 assets published and public download links verified. Ten old Release objects annotated and retired afterward; all old Git tags retained unchanged. Old IDs/download counts/backup paths are recorded in Windows history. Loss of original old Release URLs/IDs/counts was the accepted irreversible boundary.
- Workflow registered via formal PR #6 (merge `dd57861d93aaa94beb90c7774777c730b79919e5`). Repository publishing secret configured by user; ordinary `windows-release.yml` temporarily disabled during PAT tag creation, conflicting automatic run `36677316346` cancelled before build/upload, and original active state restored after the draft phase.
- Frozen 36-file old backup verified; pre5 source is isolated run `36655693790`, rechecked unexpired immediately before dispatch. Polluted old pre.6 files remain historical backup only. Historical FAIL/NOT_RUN and ZIP-era warnings are preserved.
- Evidence: `docs/validation/windows-validation-history.md`; migration index: `docs/releases/pre-release-renumbering.md`; independent manual acceptance: `docs/validation/windows-queue.md`; machine-local detailed artifacts: `validation-artifacts/migration-20260930-7a3374b`.
- Next Windows work: execute the queued per-version native GUI and applicable isolated browser/Native Host installation checks, recording observed results without reclassifying historical failures. Shared contract issues must be marked `CROSS_PLATFORM_CHANGE_REQUIRED`; Linux may consume the committed Windows evidence via Git.

## Retained historical handoff records

The following prior-batch records are retained for traceability. Their owner and pending-state descriptions are historical; current migration status is defined above and in the latest Windows validation history.

## Historical batch: v0.2.0 integration baseline, Linux → Windows handoff (2026-09-29)

### Release/WDIO policy implementation awaiting Windows validation (2026-09-29)

Cross-platform Owner implementation is on `release/v0.2.0`; source baseline `9048680e8f71202dacda4d2b7e0d05ddbddc66ea`. The handoff revision is the commit containing this section; confirm its full SHA from Git before Windows validation. The Windows release workflow separates strict asset integrity/upload from independent WDIO on the same run's executable (run ID/source SHA/exe SHA-256); pre-release explicitly dispatches the Windows workflow after tag creation. WDIO FAIL/BLOCKED alone may not prevent publication but must retain its true result and diagnostics; it is not a Windows GUI PASS. Windows Owner must run a new release rehearsal, record build/WDIO run IDs, hashes and release link, and complete separate GUI acceptance. The earlier `36568849798` `DevToolsActivePort` failure remains BLOCKED with unknown cause. This change does not retroactively alter the Windows validation status of the prior baseline. At handoff there must be no uncommitted changes; Windows Owner fetches the pushed revision and owns platform execution.

Follow-up: GitHub Actions run `36583487110` on `c63733d31d9b2c575b5eb501b46ec592f16418ea` failed at workflow definition validation (zero jobs); `actionlint` identified `runner.temp` in job-level `env`. It is corrected in the subsequent handoff commit by setting the directory in a step. This run is **NOT_RUN** for asset build and WDIO, not a product or GUI FAIL. A new tagged run on the corrected revision remains required.

2026-09-29 `v0.2.0-pre.15` tagged rehearsal: source `7482d0ea2c2be944d680e59709a182e2ed88a0c5`, Linux run `36585421973` PASS (pre-Release created), Windows run `36586074563` FAIL at `cargo test --workspace --locked`, `batch::tests::persists_each_discovery_candidate_before_completion` (`candidate was not streamed`, 119 passed/1 failed). No exe was built; Release assets and Actions artifacts are empty. Independent WDIO job `109470402553` skipped/NOT_RUN; GUI NOT_RUN. Do not apply the WDIO non-blocking exception to this build failure. Investigate the shared test synchronization and then rehearse a **new tag**; see `docs/releases/v0.2.0-pre.15.md`.

`pre.16` follow-up (source baseline `20b798a64cc0c5746a42d56b8e8eb96178ebc911`): Cross-platform Owner changed only the shared test's synchronization in `desktop/src-tauri/src/batch.rs`; production discovery behavior/contract is unchanged. Linux targeted test passed 25 successive runs; Windows Owner must verify the new tag's Windows Rust tests, build/asset integrity, independent WDIO using the same exe hash and GUI/manual queue. No Windows PASS can be inferred from Linux results. The handoff revision is the commit including `docs/releases/v0.2.0-pre.16.md`; keep tag SHA and any post-tag results distinct.

2026-09-29 `pre.16` execution: source/tag `1c72c2de73690b6c63fd31d0333deeccf1edee4c`, Linux run `36588175383` PASS, Windows run `36588840128` build job `109476286993` PASS; seven nonempty Release files published, downloaded exe SHA-256 `615b60c16b38bf851c1efc65e28e8bb4bf757fc77af07d5f92b207d56f01121d` matches manifest/SHA256SUMS/WDIO identity. Independent WDIO job `109484828255` **FAIL** at WebDriver session creation (`DevToolsActivePort file doesn't exist`, three attempts); identity, driver setup and preflight succeeded. Diagnostics artifact: `XArchive-v0.2.0-pre.16-wdio-diagnostics-36588840128`. Workflow conclusion FAIL reflects WDIO, **not** asset build failure; GUI/manual acceptance NOT_RUN. Windows Platform Owner owns session diagnosis and GUI/manual queue follow-up; see `docs/releases/v0.2.0-pre.16.md`. Post-tag documentation commits must not be confused with the tag source SHA.

2026-09-29 Release cleanup: six empty GitHub pre-Release objects were deleted (tags and historical notes retained); see `docs/releases/pre-release-renumbering.md` for IDs and the proposed old→new mapping. Continuous renumbering is **BLOCKED**, not completed: existing `pre.6` has two assets built from `main`, and its exact-tag historical Windows run `35567742785` failed Native Host configuration before packaging. Windows Platform Owner must establish an isolated, non-clobber, same-source rebuild and validate the assets before any migration. Existing nonempty Release tags/asset names have not changed; prior WDIO/GUI outcomes remain unchanged.

2026-09-29 isolated follow-up: Windows Actions run `36596395444` PASS for exact old `pre.6` tag source `ac586e609337947aeb51de8f5cce3185efc8995e`, Rust tests, Tauri and Native Host. Downloaded executable SHA-256 `4fcc100e63262fe9bdf50bf75e016aa6580ba146c020e23838b01724376eef4a`; evidence artifact `legacy-pre6-source-build-36596395444`. This is **partial build verification only**: no worker/7z/manifest/SHA256SUMS or Release upload. Continuous renumbering and Release Notes migration remain BLOCKED by incomplete packaging validation and historical tag collisions; Windows GUI NOT_RUN. See `docs/releases/pre-release-renumbering.md`.

2026-09-30 isolated packaging follow-up: run `36650561763` generated five downloadable candidates with matching SHA256SUMS/manifest, but extracted archives contain development files and lack external Full dependencies: **not publishable**. Corrected packaging commit `f2915313f47e1fe53c52c0b6f1d9af935ba50fcf` run `36651747470` FAIL at two Rust supervisor handshake timeout tests, before packaging; corrected asset validation NOT_RUN. No Release/tag renumbering or Notes migration occurred. Windows Owner to investigate/retry old-source tests and validate corrected archives; GUI remains NOT_RUN. See `docs/releases/pre-release-renumbering.md`.

2026-09-30 finite retry: isolated run `36652514096` (dispatch revision `9e3833229a29d9b8f6794b61a9e514cf5196e2c6`, checked-out tag source `ac586e609337947aeb51de8f5cce3185efc8995e`) PASS; downloaded five asset hashes, SHA256SUMS and four 7z integrity checks PASS, exe SHA-256 `635528a0b1d1a1bf7eae5a527771c70f3f3c2482a42d7eec83c995573b379fa3`. **Publication BLOCKED**: Windows archives omit internal `package-manifest.json`; Native Host manifest embeds runner-local absolute path. Prior run `36651747470` remains FAIL. No Release/tag/Notes edits; Windows packaging owner must repair and validate portable package metadata and paths. GUI NOT_RUN; historical tag collisions remain unresolved. See `docs/releases/pre-release-renumbering.md`.

2026-09-30 isolated correction: workflow revision `d5bc5e1dd65dc6d6d0ecf4b817e50d0cedbde6e9`, Windows run `36655693790` PASS on immutable old `pre.6` source `ac586e609337947aeb51de8f5cce3185efc8995e`. Downloaded artifact five SHA256SUMS/manifest identities and four 7z integrity checks PASS; dependency and Full archives contain source-bound internal package manifests and a Native Host **installation-required placeholder**, not a CI absolute path. Exe SHA-256 `4f8181459c30c682235573649c2c8713ea5bf191f4a65582837ab21607f25fa4`. This is isolated static packaging validation, NOT installed Native Host/GUI/Release acceptance. Existing Release and tags unchanged; `pre.7`/`pre.16` migration checks and occupied target tag/URL decision remain BLOCKED. See `docs/releases/pre-release-renumbering.md`.

### Batch

- Task: consolidate U7 (popup/options, batch + authenticated WebSocket channel, seven-file release tooling) and the security branch (R7 audit remediation, Windows packaging, WDIO recipes) into one baseline, align the product version to `0.2.0`, ship every release archive as a real 7z container, and hand the baseline to Windows for one concentrated validation batch.
- Branch: `release/v0.2.0`
- Current owner: Windows Platform Owner for the concentrated validation batch.
- Current state: `READY_FOR_WINDOWS` — Linux scope for this batch is complete; no `WINDOWS_VERIFICATION_BLOCKING` items are open.
- Release scope: personal-use project, provided as-is. It does **not** promise successful archiving of real X posts and does **not** guarantee usability on other devices. Asset positioning and the "no promises" wording are fixed in the root `README.md`.

### Revisions

- Integration merge: `61ff1ae` (base `783a021` = `origin/feature/u7-desktop-production-integration`; merged `edab0d6` = `security/tweet-url-host-validation`).
- Linux close-out: `06bfa39` (version alignment, Extension 7z contract, Popup version, README release scope).
- Previous Windows input revision: `7eacb82`; its results stay valid only for the pre-integration tree and must not be carried over as v0.2.0 results.

### Cross-platform work completed

- Resolved 33 merge conflicts across the five agreed boundaries: protocol security (U7 sidecar v2 kept; v1 files that U8 already deleted were not restored), Extension (both sides carried the same parsed-host Tweet-link validation, so the U7 superset was kept, 32/32), desktop and Sidecar (ENG-06 real clock ported into the single-file executor; ENG-05 bounded stderr capture and bounded supervisor line reading; ENG-12 redaction on Sidecar events and dependency errors; ENG-13 aria2 secret via short-lived `--conf-path` while proxy credentials stay in the child environment; ENG-03 `resolve_within` also applied to staging), build and release (U7 seven-asset workflow as the base plus the ENG-07 commit/digest step and ENG-01/08/09 packaging guards), documentation (both lineages' queue, handoff, roadmap and repository-map records merged and de-duplicated).
- Unified every product-facing version to `0.2.0` from a single source of truth (`Cargo.toml` workspace version) and extended the alignment regression to the Extension manifest/package and the dashboard placeholder literal.
- Changed the standalone Extension release package from ZIP to a real 7z container across the whole contract: asset naming and parsing (the historical `-extension.zip` name is now rejected), workflow creation (`7z a -t7z`), verification extraction (`7z x`; `Expand-Archive` cannot read 7z), upload step names, and tests.
- Popup now renders the installed Extension version from `chrome.runtime.getManifest().version` instead of a hardcoded string.
- Linux gates on this revision: `npm run check`; `npm test` 141/141 (desktop) and 32/32 (extension); `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --no-fail-fast` 262/262; `python -m pytest sidecar/tests -q` 46/46; both workflow YAML files parse; `git diff --check`. These are Linux facts only and imply no Windows PASS.

### Windows work required

One concentrated batch against this exact revision. IDs and exact steps are in `../validation/windows-queue.md` (`2026-09-29 v0.2.0 整合基线集中验证队列（R2 轮）`):

1. `WQ-V020-ARFMT-01` (P0) — all four archives are real 7z containers, pass `7z t` and extract cleanly.
2. `WQ-V020-PKG-FULL-01` (P0) — fresh Full `7z` build; manifest version `0.2.0`, installation manifest `release_tag: v0.2.0`, no local files, `.pyd`/`.dll` retained.
3. `WQ-V020-START-01` (P0) — separate startup evidence for the standalone `.exe`, the application-only `7z` and the Full `7z`; one success must not be generalised to the others.
4. `WQ-V020-EXT-01` (P1) — Extension `7z` contents after extraction, then load the extracted directory; Popup display and pairing state are recorded separately.
5. `WQ-V020-E2E-ORD-01` (P1) — direct `msedgedriver` recipe with `WDIO_DIRECT_DRIVER=1`.
6. `WQ-V020-TEST-01` (P1) — actual test counts on this revision, with every failure located to a named test.
7. `WQ-V020-MANUAL-01` (P1) — the outstanding M11–M13 manual checks.
8. `WQ-P1-16` / `WQ-P1-17` stay `WINDOWS_BLOCKED`; add the manual entry point but never record them as PASS.

### Expected behaviour, risks and no-go triggers

- Expected: the three "startable" shapes start as the README describes, the four archives extract as 7z, and the packaged manifest reports `0.2.0`.
- Deferred by design: no real X archive success is promised, so authentication, rate-limit and timeout observations during validation are recorded as environment observations rather than release blockers. The executor split (ENG-15) was superseded by the U7 single-file implementation, so RISK-024 stays open. Pairing remains manual in this version.
- No-go triggers: a missing or mismatched asset, an archive that is not a real 7z container, a described-as-startable shape that cannot start, sensitive files inside a package, or documentation that still promises unverified cross-device or real-X capability.

### Next owner

Windows Platform Owner. On completion, record PASS/FAIL/BLOCKED/NOT RUN with evidence locations, update this block, the queue and the validation history, and return the release decision to the Cross-platform Owner.

## Batch

- Task: add the authenticated local WebSocket transport and operational Extension popup/options UI, then hand off to Windows for browser, GUI, lifecycle, and package validation.
- Branch: `feature/u7-desktop-production-integration`
- Current owner: Linux Cross-platform Owner for the executor queue/backpressure fix and review of the Windows storage-path adjustment.
- Current state: `CROSS_PLATFORM_CHANGE_REQUIRED` (the Windows path mapping now passes limited GUI validation; rapid task submissions expose shared runner queue saturation and misleading `DOWNLOADING` states)

## Revisions

- Cross-platform input revision: `fcde5943af2f6ad15c833fa5ab88d6b1758345c6` (Windows validation batch that produced the `close_before_auth` evidence).
- Cross-platform implementation revision: `6d60429f818322b0f9a07a4e4f429d7b5bac7ee3` (includes `ff94af7` accepted-stream blocking-mode fix and stage counters, plus parsed-host validation for Extension Tweet links).
- Previous Windows input/handoff revision: `084354a5ca433b52372aca4bc70ac5fc544104fc` (prior validation batch; current input is below).
- Windows implementation revision: `7eacb82` (`fix(windows): map archive job IDs to safe staging names`).
- Windows validation revision: `7eacb82` (Windows-target `xarchive-storage` 37/37; formatting check; isolated Desktop release build and Full package assembly).
- Windows validation input revision: `6d60429f818322b0f9a07a4e4f429d7b5bac7ee3`.
- Latest archive-runtime failure report input: `57333ad3fa70702bbe9f889d128e5f230cf4a10e` (user's follow-up on the isolated package at implementation `7eacb82`).
- Current Windows repository validation input: `f03a9963db9b92bdfef6fd4d2570634c7fe43392`; code changes since `7eacb82` are absent, and the exact source revision of the user's currently loaded Extension is not exposed by the browser surface.
- Windows validation record: `../validation/windows-validation-history.md` under the 2026-09-27 entries; scheduler follow-up is WQ-WS-09 and extraction timeout investigation is WQ-WS-10 in `../validation/windows-queue.md`.

## Cross-platform Work Completed

- Added an authenticated loopback WebSocket listener using the mature `tungstenite 0.28` crate. RFC 6455 framing/handshake is delegated to the library; XArchive owns only the transport envelope and BrowserRequest/BrowserResponse routing.
- Listener binds `127.0.0.1` on default port `17321`, supports controlled `XARCHIVE_WEBSOCKET_PORT` and `XARCHIVE_WEBSOCKET_TOKEN` overrides, generates a process-local token when not supplied, requires authentication before `BrowserTransportAdapter`, and preserves `request_id` semantics.
- Integrated the listener into `RuntimeState` startup/stop and `replace_executor()` generation handling so old connections do not retain a stale executor service.
- Added Extension WebSocket settings/bridge with storage-backed configuration, one-time authentication, request timeout/concurrency limits, pending cleanup, bounded reconnect, explicit auth-failure state, and diagnostic Native Messaging fallback without replaying already-submitted business requests.
- Added Extension popup and options pages for channel/status display, page availability, reconnect, settings, manual port/token pairing, and recovery states. Popup/options assets and WebSocket files are included in Extension package inventory.
- Updated protocol/runtime/architecture/risk/repository-map documentation and Windows validation queue. Current pairing is intentionally manual; automatic port discovery and credential rotation are not implemented in this batch.
- Fixed strict-Clippy issues in shared redaction, protocol validation, batch filtering, batch commands, and network diagnostics without changing runtime behavior.
- Diagnosed and fixed the Windows pre-authentication disconnect. The shared listener polls its stop flag through a non-blocking `TcpListener`, but it never restored blocking mode on the accepted stream. POSIX `accept` does not inherit `O_NONBLOCK` while Winsock does, so on Windows the handshake and the authentication read returned `WouldBlock` immediately and were counted as a close before authentication. `handle_websocket_connection` now sets the accepted stream to blocking mode before applying the read timeout, and increments the read timeout only after authentication.
- Added `handshake_failed` and `auth_read_failed` diagnostics so a target environment can distinguish an HTTP upgrade failure, a missing authentication frame, and a received non-text frame without exposing the token.
- Added the regression test `authenticates_when_the_accepted_stream_starts_non_blocking`, which explicitly sets the accepted stream non-blocking to reproduce the Winsock inheritance and delays the authentication frame by 50 ms. Removing the fix makes this test fail immediately with `HandshakeIncomplete`, which confirms the test detects the defect rather than passing trivially.

## Windows Work Completed

- At `6d60429`, Windows-targeted `authenticates_when_the_accepted_stream_starts_non_blocking` passed (1/1); Extension Node tests passed (32/32); Desktop Vite build and `cargo fmt --all -- --check` passed. A fresh Windows Tauri release executable was built, and current Full package/Extension ZIP inventory and archive integrity checks passed. Detailed outputs and hashes are in the 2026-09-27 Windows validation history entry.
- The user reports successful current-token authentication and Extension task submission. Those submitted archive jobs failed in the pre-fix package with Windows `os error 123` because the generated job ID contains `:` from its ISO timestamp.
- Implemented `7eacb82`: `FileStore` now maps opaque job IDs to Windows-safe staging components consistently across creation, recovery lookup, and commit, while the persisted job ID remains unchanged. Small shared implementation adjustment; `CROSS_PLATFORM_REVIEW_REQUIRED`.
- PASS at `7eacb82`: Windows-target `xarchive-storage` suite (37/37), `cargo fmt --all -- --check`, Tauri Windows release build, and isolated Full package assembly at `validation-artifacts/windows-batch-os123/full-package`. Package components and manifest were present; package SHA-256 values are recorded in the validation history.
- Limited PASS by user report at `7eacb82`: Sidecar and Extension connected, and new tasks entered “Downloading” without the prior Windows `os error 123`; successful archive completion/output is not yet established.
- New user report: after four rapid submissions, two tasks stayed “Downloading” for more than 30 seconds and later tasks failed with `job executor command queue is full`. A read-only package database snapshot showed one `DOWNLOADING` and three `FAILED` jobs; current package process inventory showed Desktop, the Dashboard Sidecar, and one archive worker with gallery-dl child processes. This is consistent with one active runner and a full one-entry runner queue.
- Source inspection isolates the shared behavior: the runner channel has capacity 1 and receives work through `try_send`; execution state is persisted as `DOWNLOADING` before this send. Queue saturation therefore fails submitted jobs instead of retaining them as queued work. The 300-second extraction timeout means 30 seconds alone does not establish an extraction timeout.
- Latest user screenshot: all four jobs ultimately failed: two with `DOWNLOAD_TIMEOUT: gallery-dl timed out` and two with the already-diagnosed full runner queue. The corrected package config sets extraction/discovery/transfer budgets to 300/600/1800 seconds; the extraction code emits this timeout when gallery-dl exceeds its 300-second deadline. The reason gallery-dl did not finish is not localized, so no Windows-owned implementation failure is inferred. WQ-WS-10 queues a controlled single-job retest after the Linux queue fix.
- Latest user confirmation: X Home loads and Extension controls/buttons render over the page (`PASS`, limited to page access and control display). The screenshot does not identify the exact Edge Profile or package revision and does not establish authenticated Extension connectivity or archive success.
- Current read-only Edge check at repository input `d55ad58`: the X Home accessibility tree loaded and exposed `XArchive：保存` buttons on timeline items (`PASS` for X page access and content-script control rendering). The browser inventory exposes no profile name and lists multiple tabs, so the requested Codex Profile identity is unverified. Native app inventory returned `apps: []` on two observations; Dashboard/other Windows-native GUI checks are `BLOCKED` with `COMPUTER_USE_UNAVAILABLE` and remain in the manual queue where applicable.
- No state-changing UI actions were taken during this follow-up. The Windows Profile identity and archive completion remain unverified; WQ-WS-09/10 track Windows revalidation after the shared fix.

### Previous revision evidence (historical, not current input)

- Revalidated the new shared auth-timeout/diagnostic-counter handoff on Windows: WebSocket-focused Rust tests 4/4; complete Windows Desktop crate tests 111/111 with the repository Python 3.12 interpreter; Extension 26/26; Desktop Node 93/93; `cargo fmt --all -- --check`; workspace `npm run check` (Vite 52 modules and Extension syntax); Windows Tauri release executable build (`--no-bundle --ci`) passed.
- Assembled an exact-HEAD Full portable package directory and built/expanded/verified the Extension ZIP at `validation-artifacts/windows-ws-fcde594/`; Extension ZIP inventory is 12/12 and the package Native Host allowed origin matches the repository-derived Extension ID. The package tag `v0.0.0-pre.1` is local validation metadata, not a release.
- Full package directory and Bandizip-generated `.7z` archive are available under `validation-artifacts/windows-ws-fcde594`; archive integrity and extracted-file hashes passed. The Full package was locally augmented with aria2 from the user's `E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2` directory at `sidecar/aria2/aria2c.exe`; this is a validation artifact, not a release-builder change.
- Isolated outputs under `validation-artifacts/windows-ws-de46a8e/`; no Windows production code was changed.
- Computer Use was retried finitely; no native app targets were available, and the only Edge surface exposed the existing user profile. It was left untouched. Current live GUI WQ-WS-01/02/03/04 checks are `BLOCKED` with `COMPUTER_USE_UNAVAILABLE`.
- Previous `084354a` batch baseline (historical, not the current validation input): Windows 11 x64 validation without Windows production-code changes.
- Windows-target core/protocol/desktop Rust tests: 146 passed; Desktop Node: 93/93; Extension: 25/25; affected Sidecar tests: 15/15; Vite production build: 52 modules.
- In that previous `084354a` batch, built a Full package in an isolated validation directory and passed static package inventory; Extension ZIP extraction/inventory verified 12 files. This does not establish current-revision packaging.
- In that previous `084354a` batch, Full package Dashboard WebView2 readiness smoke passed 3/3 with pinned WebView2 Runtime `153.0.4234.48`, EdgeDriver `153.0.4234.46`, and tauri-driver `2.0.6`; this historical smoke is not a current GUI result.
- Existing local dependencies, logs, manual-validation files, and validation artifacts were preserved. Detailed hashes, commands, constraints, and first-attempt setup corrections are recorded in `../validation/windows-validation-history.md` under the `084354a` entry.

## Windows Work Required

- On an explicitly controlled/disposable Edge profile and the current Full package/Extension ZIP listed in the 2026-09-27 history entry, complete WQ-WS-01/02/03/04: Extension load and permissions; one correlated reconnect with `handshake_failed`/`auth_read_failed`/`auth_received` deltas; wrong/correct-token pairing and `query_status`; pending cleanup/reconnect; popup/options scaling and keyboard checks. Do not use the signed-in profile or record the token.
- At current input `6d60429`, Full package directory assembly, Extension ZIP build/inventory, Bandizip `.7z` creation, archive integrity and extracted-file SHA-256 comparison `PASS`; automated browser load was `BLOCKED` with `COMPUTER_USE_UNAVAILABLE`, and signing/release gate is `NOT RUN`. A later user screenshot confirms X Home access and visible Extension controls only; package-specific load, permissions, and service-worker checks remain unverified. The other GUI observations are historical and bound to their stated artifacts/revisions.
- User-reported manual follow-up: Full package startup/close/content display and Extension options/popup display passed. Later, `127.0.0.1:17321` had a listener and TCP connect passed; one WebSocket request returned `101`, and DevTools showed an outbound authentication frame. The Extension remained disconnected/unauthenticated, no inbound authentication response or request response was observed, while Native Host requests arrived and X-page task submission created a task. Earlier attempts showed `ERR_CONNECTION_REFUSED`; preserve this chronology rather than treating the listener as continuously available. See the dated follow-up in `../validation/windows-validation-history.md` and `../validation/windows-queue.md`.
- Current corrected Full package Sidecar-running state and bundled Extension Options page display are `PASS` by user screenshots; browser permissions, service-worker health, and full popup/accessibility assertions remain `BLOCKED`/`NOT RUN`.
- At the previous `fcde594` revision, WQ-WS-02 live pairing was `FAIL` by user screenshots. At current input `6d60429`, the shared fix's Windows regression test is `PASS`, but current live pairing has not been retested and is `BLOCKED` with `COMPUTER_USE_UNAVAILABLE`; preserve the prior failure as historical evidence. WQ-WS-01 has limited user-reported `PASS` for X Home access and Extension control display; Profile/package identity, permissions, service-worker state, WQ-WS-03 lifecycle, and WQ-WS-04 full accessibility remain unverified or `BLOCKED`/`NOT RUN`.
- Current `WQ-WS-05` static package directory, Extension ZIP inventory, and Bandizip archive/extract/hash checks `PASS` at `6d60429`; browser load and signing/release gate remain `NOT RUN`.
- Root cause for the supplied Full package startup failure is confirmed: its frozen worker predates Desktop's `--timeout-seconds` and `--discovery-timeout-seconds` launch arguments and exits with code 2 before the v2 handshake. A fresh worker built from current Sidecar source accepts those exact arguments and reaches `ready`; the corrected Full package archive passed integrity and 77-file extraction/hash comparison. The latest user screenshot now shows Sidecar connected in the corrected package; no archive task/download success is established.
- Follow-up on that corrected package: user screenshots show Sidecar connected and Extension WebSocket still disconnected. The latest selected DevTools request contains an outbound `authenticate` frame but no visible inbound response. The earlier aggregate listener counters are not correlated to that request; see the dated queue/history entries.
- Per the user's instruction, pairing-token privacy/security review is `NOT APPLICABLE`; the user states this is a local-validation token reset on Desktop restart. No compromise finding or rotation action is tracked. Pairing remains functionally unverified/failed; collect one correlated reconnect, Desktop counter deltas, and the selected Messages frame before assigning the receive/response failure boundary.
- Continue the existing Windows account-batch, gallery-dl/aria2, filesystem recovery, Native Host/Registry, signing, and release queue items. Previous account-batch and reconnect observations remain bound to their original artifact/revision until a new manual retest.

## Expected Behavior

- Only an authenticated WebSocket session can call `BrowserTransportAdapter`; unauthenticated messages receive a structured authentication failure and never create or query an archive Job.
- Business messages remain valid `BrowserRequest` values and responses retain the matching `request_id`.
- Desktop restart, listener replacement, Service Worker restart, and disconnect clear pending work and do not route a new request to a stale executor generation.
- The Extension UI reports the actual channel and distinguishes unconfigured, connecting, connected, disconnected, authentication-failed, request-failed, and Native fallback states.
- Manual Desktop-to-Extension port/token pairing is the supported first-version flow; loopback binding alone is not treated as identity.

## Validation Required

- Linux PASS: `cargo fmt --all -- --check`; `cargo check --workspace --all-targets --offline`; `cargo clippy --workspace --all-targets --offline -- -D warnings`; `cargo test --workspace --offline --no-fail-fast -q` (test groups 18/18, 110/110, 22/22, 7/7, 8/8, 19/19, 6/6, 36/36, 12/12); Sidecar `compileall` + pytest 35/35; Desktop Node 93/93; Extension 26/26; package/Native Host contract 10/10; Extension package plan/verify; `git diff --check`.
- Windows current revision: automated Rust/Node/frontend/format/Tauri executable checks pass as recorded below. Live GUI checks remain blocked until a controlled Edge profile and matching Full package are available; once available, verify auth timeout/counters, wrong/correct token, request delivery, reconnect and Native fallback. The exact manual steps and blocker evidence are in the dated queue/history records.

## Risks and Deferred Items

- The Windows observation that connections closed before authentication is now explained and fixed in shared code: the accepted stream inherited the listener's non-blocking mode on Winsock, so the authentication read returned `WouldBlock` instead of waiting. The fix is Linux-verified with a defect-sensitive regression test, but only a Windows revalidation against the exact handoff can confirm live pairing.
- The prior Windows WQ-WS-02 failure is preserved as historical evidence; the current queue status is pending exact-handoff revalidation, not a new Windows PASS.
- Automatic WebSocket port discovery and credential rotation are not implemented; manual pairing is documented and must be exercised in Windows WQ-WS-02.
- Real Edge/Chrome permissions and MV3 lifecycle, WebSocket authentication/request flow, Native Host fallback operation, and actual account/archive transfer remain unverified or failed as detailed in the manual follow-ups.
- P1-B real gallery-dl samples and P3-E real-account multi-page/authentication/SHA-256 acceptance remain `NOT RUN` on Linux because they require controlled external samples, credentials, or target artifacts.
- Do not treat synthetic fixtures, Linux loopback tests, Node tests, Vite builds, or package inventory as Windows acceptance.

## Relevant Tests

- Linux evidence is recorded in `docs/development/status.md` and covers the shared listener, bridge contract, GUI assets, package inventory, workspace tests, strict Clippy, and build checks.
- New shared regression coverage verifies a correctly configured token receives `authentication_response`, an authenticated loopback connection routes a real `query_status` BrowserRequest, a silent peer reaches `WEBSOCKET_AUTH_TIMEOUT`, and Desktop exposes token-free accepted/auth-received/auth-success/auth-failure/auth-response-failure/close-stage counters.
- Windows evidence must be recorded in `docs/validation/windows-queue.md` and the Windows validation history with the exact pushed handoff revision.

## Manual Windows Validation Queue

After Linux's executor scheduling change is handed back, run WQ-WS-09 and WQ-WS-10 in `../validation/windows-queue.md` on the exact returned revision. Confirm multiple rapid submissions remain durable and queued, execute without `job executor command queue is full`, then investigate one controlled archive that reaches terminal success or a sanitized, attributable failure. The configured extraction timeout is 300 seconds and transfer timeout is 1800 seconds.

## Cross-platform Follow-up

- `CROSS_PLATFORM_CHANGE_REQUIRED`: fix shared executor scheduling/backpressure. `runner_sender` is `sync_channel(1)` and uses `try_send`; `execute_persisted_from_factory` marks jobs `DOWNLOADING` before queue admission, so saturation produces `EXECUTOR_WORKER_FAILED` and loses accepted work rather than keeping it queued. Add defect-sensitive regression coverage for at least three overlapping submissions and define the durable queued/active state behavior.
- `CROSS_PLATFORM_REVIEW_REQUIRED`: review the small `xarchive-storage::FileStore` Windows staging-component mapping in `7eacb82`; persisted IDs and public contracts are unchanged.
- `WINDOWS_BLOCKING`: none.

## Next Owner

- Linux Cross-platform Owner: implement and verify the shared runner queue/backpressure correction, and review the small shared FileStore mapping at `7eacb82`. Windows Platform Owner resumes after that handoff for WQ-WS-09 exact-revision verification.

## 2026-09-27 Sidecar Follow-up Addendum (historical; before executor saturation reports)

- At the time of this earlier follow-up, the Windows Platform Owner was validating source implementation `6d60429f818322b0f9a07a4e4f429d7b5bac7ee3`; no tracked production source was changed in that follow-up.
- The Full package initially assembled for this batch contains stale worker SHA-256 `96C19695AC46E30AA23AF184ED41D0C8339FE4C98A2D771CF0781906402A60C2`; its CLI rejects Desktop timeout arguments, so that package's Sidecar runtime check is `FAIL` despite the earlier static inventory/archive PASS.
- An isolated corrected package is at `validation-artifacts/windows-batch-revalidation-6d60429-sidecar-current`; it uses the previously rebuilt compatible worker SHA-256 `B51566894CAA6C20AF5B81F2D93D1B8DFAC1B3EC3F24F6EBC8FF8B2222F1318A`. User screenshots and process paths confirm corrected Dashboard startup and Sidecar `hello -> ready` (`PASS`). Extension WebSocket reached Desktop, but the server returned `AUTHENTICATION_FAILED` (`FAIL`); WQ-WS-07 remains open for a current-token re-pair and authenticated request/response check.
- Computer Use exposes no native app target. Its Edge adapter does not expose the selected profile name and reports multiple tabs; no tab was opened. The user reports the intended Codex Profile is ready, but identity/one-tab state has not been independently confirmed.
- At that point, no `CROSS_PLATFORM_CHANGE_REQUIRED` or `CROSS_PLATFORM_REVIEW_REQUIRED` finding had been identified. This historical status was superseded by the later executor saturation reports above; current ownership and follow-up are recorded in the current batch sections.
# Platform Handoff（Windows → Cross-platform）

Windows Validation Queue 的唯一事实源仍是 [`../validation/windows-queue.md`](../validation/windows-queue.md)；平台验证流程见 [`../development/cross-platform-validation.md`](../development/cross-platform-validation.md)。

## 当前状态

- Branch：`security/tweet-url-host-validation`
- Implementation / tested revision：`589142f169b3cec1a72948074f9466f89c5680ff`（Windows batch 5）与 `7e3d646`（Windows batch 6 Full package baseline）+ 本节 Cross-platform batch 6 的 Linux follow-up（revision 待本次状态更新提交后生成）
- Last Windows validation record：`169f425`（batch 5 报告与 queue 结果）、`584eee8`（batch 6 Full package 报告与 handoff）；本轮最终 handoff revision 待本次状态更新提交后生成。
- 结果：batch 5 — WQ-DRV-01/02 `PASS`、WQ-ENG-13 Desktop suite 84/84 `PASS`、WQ-TEARDOWN-01 `PASS`、M9/M10 `PASS`；WQ-ENG-09b-ORD `BLOCKED_AUTOMATION`（153 钉版后 session 仍落 Edge 155 `msedge.exe`）。batch 6 — WQ-WORKER-BUILD-01 `WINDOWS_PASS`；WQ-PACKAGE-FULL-01 `WINDOWS_FAIL`（规范 fresh-build `spawn npm.cmd` `EINVAL` + `*.pyd`/版本来源评审）；M11 `BLOCKED_AUTOMATION`、M12/M13 `NOT RUN`；WQ-P1-16/17 `BLOCKED`。
- 状态：`READY_FOR_WINDOWS`。Windows 交回的四项（A `npm.cmd` spawn 修复、B `*.pyd` 过滤、C 版本来源、D WQ-ENG-09b-ORD recipe 复审）已在 Cross-platform batch 6 完成并通过 Linux 全量门禁；复验项全部 `WINDOWS_VERIFICATION_PENDING`。无 `WINDOWS_VERIFICATION_BLOCKING`。
- 工作副本：正式 E: checkout；只更新验证/交接文档，机器本地产物保留。

## Cross-platform follow-up

已全部结清（详见文末 “Cross-platform batch 6” 与 [`../validation/windows-queue.md`](../validation/windows-queue.md) 同名节）：A `npm.cmd` spawn 修复收口（`main()` + 直接执行守卫）；B `*.pyd` 移出全局排除清单（PyInstaller `_internal` 运行时 C 扩展必须随包）；C 版本来源统一到 Cargo `0.1.1`（`tauri.conf.json`、根/desktop `package.json` 对齐，manifest 默认派生 `tauri.conf.json`）；D WQ-ENG-09b-ORD reviewed recipe = `WDIO_DIRECT_DRIVER=1` 直连独立 msedgedriver（batch-2 步骤 4/5 已实证形状），写入 `windows-wdio-handoff.md`。

复验项（均 `WINDOWS_VERIFICATION_PENDING`）：WQ-PACKAGE-FULL-01-R2（P0 fresh Full build）、WQ-ENG-09b-ORD-R2（P1 新配方单次）、WQ-ENG-13-R2（P1 计数 91/91）、M11–M13（人工）。WQ-DRV-01 补丁、WDIO banner、84/84 Desktop suite、动态端口 teardown 与 WQ-ENG-03/08/09a 复用历史结论；未运行 full regression。

## Manual Windows Validation Queue

- **M8：**在可观察的 Windows 桌面会话独立启动当前构建，确认 Dashboard 与实际 WebView2 Runtime；Computer Use 有界重试后仍没有原生应用窗口。
- **WQ-P1-16/17：**等 ordinary native session 建立后再执行 advanced E2E。
- **既有项目：**GUI 的 DPI/焦点/辅助技术，WQ-P1-12 filesystem/reparse/Unicode/Sidecar fixture，真实账号、Named Pipe、workflow、archive extraction 保持原队列。
- **M9/M10：**本轮分别验证失败路径 driver/端口清理、clean install 与补丁幂等，均 PASS。
- **M11–M13：**Full 包 Dashboard/WebView2、Sidecar UI lifecycle、Codex Edge Profile + Extension + 真实下载（步骤见 `windows-queue.md` Windows batch 6 Manual 队列；M11 `BLOCKED_AUTOMATION`、M12/M13 `NOT RUN`）。

## 下一 Owner

**Windows Owner**：同步后按 [`../validation/windows-queue.md`](../validation/windows-queue.md) “Cross-platform batch 6” 第 4 节顺序执行——P0 WQ-PACKAGE-FULL-01-R2 fresh Full build（不设 `PORTABLE_ALLOW_BINARY_REUSE=1`）→ P1 WQ-ENG-09b-ORD-R2 direct msedgedriver 配方单次 → P1 WQ-ENG-13-R2 计数 91/91 → M11–M13 人工；逐项记录结果，失败保留日志交回 Cross-platform Owner。不得把 `BLOCKED_AUTOMATION`/`NOT RUN` 记为 PASS，不得改用旧配方重试 ordinary/advanced E2E。

## 同步方式

见文末 “跨会话交接总结” 的同名小节（该处为当前有效版本）。

## 跨会话交接总结（2026-09-28，Windows batch 2 收尾）


### 仓库实际状态（以 git 为准）

- Branch：`security/tweet-url-host-validation`；HEAD = `b918d605fae464933babb00391d33e5cc6cda694`，与 `origin` 完全同步（`git ls-remote` 一致）。
- `git status`：tracked 文件**无任何改动**（`--untracked-files=no` 为空）；未跟踪项仅为 ignored/本地产物：`.codex/`、`.venv-windows-validation/`、`aria2/`、`desktop/logs/`、`dist-portable/`、`gallery-dl/`、`manual-validation/`、`sidecar/build/`、`sidecar/gallery-dl/`、`sidecar/xarchive-downloader/`、`validation-artifacts/`。
- 最近 commits：`b918d60`（本轮 docs）→ `8ddb09a` → `27dcebc` → `e856343` → `5170161` → `a6311b3` → `01067b6`。
- `b918d60` 只改 3 个 `docs/` 文件（`windows-validation.md`、`platform-handoff.md`、`windows-queue.md`），无代码/依赖/测试改动。

### ⚠ 与聊天记忆的差异（以仓库为准）

本轮最初在 `5170161` 上验证，当时 diff 为 docs-only。**push 时被拒（non-fast-forward），fetch 后发现 Linux 侧已推进到 `8ddb09a`，其中 `e856343` 新增了 WDIO 测试层代码**（`desktop/e2e/support/startup-diagnostics.mjs`、`desktop/test/startup-diagnostics.test.mjs`，并修改两个 E2E spec 与 `desktop/wdio.conf.mjs`）。已 rebase 并保留双方文档内容。

因此：**`e856343` 的新测试层此前从未在 Windows 执行过**，本轮收尾补做了最小验证并发现新缺陷（见下）。

### 本轮新发现：`CROSS_PLATFORM_CHANGE_REQUIRED`

`desktop/test/startup-diagnostics.test.mjs:54` `resolves the artifact directory from WDIO_LOG_DIR and the default` 在 **Windows 失败**（Linux 上通过）：

```
AssertionError [ERR_ASSERTION]: Expected values to be strictly equal:
+ actual   'E:\project\desktop\custom-logs\startup'
- expected '\project\desktop\custom-logs\startup'
    at test\startup-diagnostics.test.mjs:56:12
```

根因：测试用 `path.join(path.sep, "project", "desktop")` 构造期望值。在 POSIX 上 `path.sep="/"` 得 `/project/desktop`；在 Windows 上 `path.sep="\"` 得 `\project\desktop`，而被测实现 `resolveDiagnosticsDir` 内部用 `path.resolve(root, …)`，会把无盘符的 root 解析为**当前盘符**（`E:`），于是实际值带 `E:\` 而期望值不带。**这是测试断言的平台假设缺陷，不是实现缺陷**——`path.resolve` 的盘符解析行为在 Windows 上是正确语义。建议修复方向：用平台无关的临时目录（如 `fs.mkdtempSync`）或 `path.resolve` 构造期望值，而不是硬编码 `path.sep` 拼接。此项属 shared test contract，**应由 Cross-platform Owner 修改**，Windows 侧不自行改测试。

命令：`npx --yes --package npm@11.19.0 -c "npm test --workspace desktop"` → **47 pass / 1 fail / 48 total**（`killTree` 通过，无沙箱权限问题）。日志：`validation-artifacts\windows-batch-20260928-5170161\desktop-test-b918d60.log`。

### 未完成工作分类

**可由下一 Task 立即继续**
- 无需等待外部条件：WQ-ENG-09b 已结清，无待办实现工作。
- 收尾复核上述 `startup-diagnostics` 测试修复后，在 Windows 重跑 `npm test --workspace desktop`，确认 48/48。

**需要额外信息或外部依赖**
- WQ-ENG-09b ordinary/advanced E2E：需上游 `tauri-driver` 修复 `map_capabilities()`，或降级到 `webdriver` 8.x。**不要在本仓库反复重试同一路径**。
- WQ-WORKER-BUILD-01 / WQ-PACKAGE-FULL-01 / WQ-PACKAGE-CORE-02：需重新生成含 `_internal\python312.dll` 的 worker artifact。
- WQ-ENG-07 / WQ-ENG-10（Actions）：需授权 CI run 与发布证书。
- WQ-RELEASE-06：需 `bundle.active=true`、代码签名证书、Updater 签名密钥。

**需要人工验证（Manual Windows Validation Queue）**
- WQ-ENG-06 真实账号错误脱敏（需专用非个人 X/gallery-dl + Telegram 账号）。
- WQ-ENG-01 破坏性打包删除保护（需一次性隔离副本，不得指向真实工作副本）。
- WQ-ENG-04 Named Pipe（**实现尚不存在**，`transport.rs` 目前 Unix-gated）。
- GUI / WebView2 / DPI / 键盘焦点 / 辅助技术（Computer Use 仍 `BLOCKED`，须人工桌面会话）。
- WQ-P1-12 剩余范围：permission/reparse/junction、长 JSON/Unicode、受控真实 Sidecar 下载 fixture。
- WQ-P1-02～05、P1-13：Native Host 安装/Registry/浏览器加载/Extension 实机。

**暂时不应继续**
- 在上游 `tauri-driver` 缺陷解除前，不要重复运行 WQ-ENG-09b 的 ordinary/advanced E2E——已用三种方式（WDIO 钉版、manual tauri-driver probe、直连 msedgedriver）确定性复现，重试无意义。
- 不要把 `data:,` 空白窗口再解读为产品启动缺陷：已证明那是 Edge 浏览器首屏。

### 下一 Task 必须保留的上下文

关键路径：
- `desktop/wdio.conf.mjs`（`edgeDriverVersion` 透传，32-33、48 行）
- `desktop/e2e/support/startup-diagnostics.mjs`（`resolveDiagnosticsDir` 用 `path.resolve`，第 23-28 行）
- `desktop/test/startup-diagnostics.test.mjs:54-64`（**待修的平台假设断言**）
- `desktop/e2e/specs/dashboard.e2e.mjs`、`wdio-plugin.e2e.mjs`（readiness 钩子已接入诊断采集）
- `desktop/scripts/wdio-tauri-service.mjs`（launcher/teardown，Windows 专属；`killTree` win32 分支）
- `docs/validation/windows-queue.md`（队列唯一事实源）、`docs/development/windows-validation.md`（逐轮证据）

环境事实（勿重复探测）：
- WebView2 Runtime = **153.0.4234.48**；Edge browser = **155.0.4283.18**（两者不同，勿用 Edge 版本推断 WebView2）。
- `tauri-driver` v2.1.0-alpha.0（`~/.cargo/bin/tauri-driver.exe`），需 `msedgedriver.exe` 在 PATH，否则启动即报 `CannotFindBinaryPath`。
- 可用 driver：`validation-artifacts\msedgedriver-153.0.4234.46\`（匹配 WebView2）、`msedgedriver-154.0.4258.24\`。以 `EDGEDRIVER_VERSION` 或 `TAURI_DRIVER_EDGE_VERSION` 钉版。
- `msedgedriver` 必须用 `--port=NNNN` 等号形式，`--port NNNN` 会报 `Invalid port. Exiting...`。
- 驱动程序需 `--native-port` 独立端口；建议用 45460+ 隔离端口，避免与遗留 4444/4445 冲突。
- system npm 11.17.0 低于项目 `engines.npm >=11.18.0`；跑 npm 命令用 `npx --yes --package npm@11.19.0 -c "…"`。
- 验证产物统一写入被 ignore 的 `validation-artifacts\windows-batch-20260928-5170161\`。

已知 workaround（直连 msedgedriver，可在无 tauri-driver 时取证）：
以 W3C `capabilities.alwaysMatch` 内联 `browserName=webview2` + `ms:edgeOptions.binary=<exe绝对路径>` + `ms:edgeOptions.webviewOptions={}`，直连 `msedgedriver --port=NNNN`，即可正常启动应用并取得 `h1=工作台`。

### 同步方式

沿 Git 将验证文档写回 source branch；不通过直接文件同步覆盖正式 Windows repo。Windows phase 使用 `git fetch` 后验证 exact implementation revision（见下方 round-3 section），并以 Git 提交号记录验证文档 revision。

## 2026-09-28 交接：Cross-platform reconcile 第 1 轮（状态确认 + 根因定位）

本节为收尾会话写入；只做仓库状态核对、定向验证和根因定位，未修改实现源码、依赖或测试。上方 Reconcile 清单与状态 `CROSS_PLATFORM_RECONCILE_REQUIRED` 保持有效。

### 源状态（本会话核对）

- Branch：`security/tweet-url-host-validation`；HEAD：`84af1ef18d964a1f05e096424bd67b9de78a89e1`（`docs: hand off Windows validation findings`）。
- Working tree clean；与 `origin/security/tweet-url-host-validation` 同步（0 ahead / 0 behind）；PR #5（→ `dev`）保持 OPEN。
- Implementation revision 仍为 `7b218f8`（Windows phase 验证对象）；reconcile 修复尚未开始。

### 本轮验证结果

| 检查 | 状态 | 结果 / 原因 |
|---|---|---|
| `npm run test --workspace desktop` | `PASS` | 42/42；含 `killTree → terminates a spawned child process`（Linux ~4 ms；Windows FAIL 未在 Linux 复现） |
| `cargo test -p xarchive-storage` | `PASS` | 29/29 |
| `npm ls --all`（当前已安装树） | `PASS` | exit 0，0 invalid；Windows cleanroom `npm ci` 后的 `ELSPROBLEMS` 在本机已安装树不复现 |
| cleanroom `npm ci` + `npm ls` 隔离复现 | `NOT RUN` | 本轮未搭建隔离 cleanroom；WQ-ENG-09 依赖契约复现属下一轮首要动作 |
| 全 workspace 回归（Rust 全量 / `npm run check`+`test` / Sidecar pytest） | `NOT RUN` | 本轮无实现改动，留给修复后的收口轮次 |
| Windows 侧行为 | `BLOCKED` | 本会话在 Linux 环境；WQ-ENG-03/08/09 的 Windows 重验须待共享修复完成后按 focused revalidation 执行 |

### 根因定位（第 1 轮调查结论）

- **WQ-ENG-03 metadata junction（`CROSS_PLATFORM_CHANGE_REQUIRED`）**：`crates/xarchive-storage/src/metadata.rs` 的 `build_archive_metadata` 只做 `relative` 词法检查（拒绝 `ParentDir`/`RootDir`/`Prefix`）并对最终路径做 `fs::symlink_metadata` reparse 检查；中间组件为指向根外的 junction/symlink 时会被路径解析跟随（Linux 可用中间 symlink 复现同一缺口）。共享的逐组件 containment 已存在于 `FileStore::resolve_within`（`crates/xarchive-storage/src/file_store.rs:147`），但该关联函数为模块私有、且 metadata 路径未复用。修复方向：提升可见性或提供 `pub(crate)` 包装，让 metadata 构建复用逐组件解析并补测试；完成后在 Windows 重跑真实 junction probe。
- **WQ-ENG-08 killTree（`CROSS_PLATFORM_REVIEW_REQUIRED`）**：`desktop/scripts/wdio-tauri-service.mjs` 的 `killTree()` win32 分支为 `taskkill /pid … /T /F`；`desktop/test/wdio-tauri-service.test.mjs:72` 已按 2026-09-16 结论在 killTree 前挂 `exit`/`close` 监听并以 10 s 窗口断言。Linux 本轮 42/42 通过；Windows 2026-09-27 失败（10 s 内未观察到终止、测试进程需人工中断）未在 Linux 复现，属 Windows 侧时序/进程生命周期问题，根因未定位。
- **WQ-ENG-09 依赖契约（`CROSS_PLATFORM_CHANGE_REQUIRED`）**：根 `package.json` 声明 `overrides`（`serialize-javascript: ^7.1.2`、`deepmerge-ts: ^8.0.2`），但 `package-lock.json` 根条目 `packages[""]` 只有 `name/version/workspaces`，**不含 `overrides`**（lockfileVersion 3；最近一次 lock 改动 `f9c088b`）。Windows cleanroom `npm ci` 后 `npm ls` 报 `serialize-javascript@7.1.2` 违反 Mocha `^6.0.2`、根 `deepmerge-ts@8.0.2` 违反嵌套 `@wdio/tauri-service 9.30.1` 的 `^7.0.3`。本机已安装树 `npm ls` 干净、隔离复现未做；修约方案（补记 overrides 进 lock / 调整 override 范围 / 其他）未定。
- **WQ-ENG-09 native E2E `data:,`**：`desktop/e2e/test-artifacts/wdio/startup/evidence.json` 是被 Git 跟踪的文件，由 `6555d35`（2026-09-27 01:11 +0800）加入且提交内容已是 `{"url": "data:,", "rootExists": false}`。当前分支没有任何生产者：全仓仅该文件本身含 `rootExists`；`desktop/e2e/support/native-startup.mjs` 只存在于 `feature/u7-desktop-production-integration`（`382258c`）与 `windows/webview2-readiness-gate`，不是 HEAD 祖先。推论：[`../development/windows-validation.md`](../development/windows-validation.md) 中 “current-run evidence.json” 可能引用已提交的陈旧文件而非 2026-09-27 run 的真实产出；triage 必须先核实证据来源，再查会话停在 `data:,` 的启动问题。文件处置决策（untrack + gitignore，或恢复生产者）未做。

### 未完成工作分类

- **可由下一 Task 立即继续**：WQ-ENG-03 metadata 修复与测试（Linux 可用中间 symlink 复现）；WQ-ENG-09 依赖契约隔离复现与 lock/override 修约；WQ-ENG-09 E2E 证据来源核实 + 启动 triage；WQ-ENG-08 依据 Windows 失败证据的代码级复核。
- **需要外部信息/环境**：Windows 侧 focused revalidation（须在共享修复完成、记录新 implementation revision 后）；Windows E: 工作副本上的原始失败日志。
- **需要人工验证**：真实账号、GUI/WebView2/辅助技术、Named Pipe、CI workflow run 等原 Manual Windows Validation Queue 项（状态不变）。
- **暂不应继续**：在未确认生产者前改写 `windows-validation.md` 的历史结论；不先隔离复现就改 overrides。

### 关键上下文（供下一 Task）

- 关键文件：`crates/xarchive-storage/src/metadata.rs`、`crates/xarchive-storage/src/file_store.rs`（`resolve_within`、`is_reparse_point`）、`desktop/scripts/wdio-tauri-service.mjs`（`killTree`/`waitForProcessGone`/`isPidAlive`）、`desktop/test/wdio-tauri-service.test.mjs`、根 `package.json` `overrides`、`package-lock.json`、`desktop/wdio.conf.mjs`、`docs/development/windows-validation.md:4378-4385`、`docs/validation/windows-queue.md:595-614`。
- 设计约束：AGENTS.md 跨平台工作流（Linux 先完成开发与验证，再集中进入 Windows phase）；Windows Validation Queue 唯一事实源为 `windows-queue.md`；`BLOCKED_AUTOMATION` 不得记 PASS；不得为过测试改业务行为。
- 不应重复的失败路径：不要用 Linux `npm ls` 干净结果替代 cleanroom 复现；不要在未确认生产者前把 `evidence.json` 当作 2026-09-27 run 证据；不要假设 `native-startup.mjs` 在当前分支存在。
- 未解决错误：Windows cleanroom `npm ls` `ELSPROBLEMS`（未复现）；Windows `killTree` 10 s 断言失败（未复现）；E2E 会话停在 `data:,`（未复现，无本地 producer）。

## 2026-09-28 交接：Cross-platform reconcile 第 2 轮（实现轮）

本节为收尾会话写入；完成第 1 轮列出的全部四项 reconcile 工作，含实现改动。上方状态与 Reconcile 清单已同步更新为 `FOCUSED_WINDOWS_REVALIDATION_REQUIRED`。

### 源状态（本会话核对与产出）

- Branch：`security/tweet-url-host-validation`；起点 HEAD：`8d81edd`（round-1 handoff，working tree clean）。
- 实现 commit：`01067b66a3aa214fb90f7d893c57bb971a7c7882`（`fix: close metadata intermediate-link escape; reconcile WQ-ENG-08/09`）；本 handoff 与队列/验证/设置文档记录在随后的 docs commit。
- 变更文件（仅 8 个实现 + 4 个文档）：`crates/xarchive-storage/{metadata.rs,file_store.rs,lib.rs}`、`desktop/scripts/wdio-tauri-service.mjs`、`.gitignore`、`package.json`、`package-lock.json`、删除 `desktop/e2e/test-artifacts/wdio/startup/evidence.json`；`docs/{development/setup.md,development/windows-validation.md,validation/windows-queue.md,status/platform-handoff.md}`。

### 各项结论

| 项 | 结论 | 证据 / 关键事实 |
|---|---|---|
| WQ-ENG-03 metadata 中间 junction/symlink 逃逸 | 已修复 | 先写 Linux 复现测试 `rejects_sidecar_intermediate_symlink_escape`（中间 symlink 指向根外）确认修复前 FAIL（根外文件被算入 metadata sha256）；`FileStore::resolve_within` 提为 `pub(crate)`，`build_archive_metadata` 放弃纯词法检查改为复用逐组件 containment；修复后 storage 30/30 |
| WQ-ENG-09a 依赖契约 | 已复现并完成最小修约（未动 overrides/lock 内容） | 隔离 cleanroom：npm 11.17.0（Windows 同版本）`npm ci` exit 0、`npm ls --all` exit 1，invalid 明细与 Windows 完全一致；同树版本二分 11.17 FAIL / 11.18 PASS / 11.19 PASS；`npm install --package-lock-only` 零 diff、手工补写 lock 根条目 `overrides` 后 11.17 仍 FAIL——“补记 overrides 进 lock” 两个候选均被证伪。修约：根 `engines.npm >=11.18.0` + lock 根条目由 npm 同步（仅 +3 行）+ setup.md 版本底线；11.17 对该 manifest 报 `npm warn EBADENGINE`（信号已验证）。engines 方案另在 cleanroom 全链验证：11.18 `npm ci` exit 0、`npm ls --all` exit 0 |
| WQ-ENG-09b evidence 来源与 `data:,` triage | 来源核实完成 + 文件处置完成；启动 triage 已入队（需 Windows 采集） | `git log --follow`：evidence.json 唯一提交为 `6555d35`（2026-09-27 01:11 +0800，提交内容已是 `{"url":"data:,","rootExists":false}`）；`git log --all -S`：写入方 `desktop/e2e/support/native-startup.mjs` 只在 `382258c`/`windows/webview2-readiness-gate`，`merge-base --is-ancestor` 确认非 HEAD 祖先；本分支 `wdio.conf.mjs` 日志目录为 `desktop/test-artifacts/wdio`（已 ignore），无任何代码写 startup/evidence.json。处置：untrack 该文件 + `desktop/e2e/test-artifacts/` 加入 `.gitignore`（不恢复非祖先分支的生产者）；`windows-validation.md` 4383 行 “current-run evidence.json” 引用已发勘误。`data:,` triage 结论：会话 URL 停在初始空文档、20 s 内无 `h1`；当前分支 spec 无 readiness gate（窗口 target 切换/空白文档处理只存在于非祖先分支）；三类假设（应用未导航 / 会话附着空白窗口 / 应用启动失败）的区分所需证据（window handles+URL/title 枚举、截图、`WDIO_LOG_DIR` service 日志、`desktop/logs` 应用日志、单独启动 binary）已写入队列 focused 步骤 |
| WQ-ENG-08 killTree/测试契约复核 | 复核完成；补诊断；不记 PASS | 契约检查：测试已按 2026-09-16 结论在 `killTree` 前挂 `exit`/`close` 监听（`test.mjs:84-90`），win32 分支为 `taskkill /pid … /T /F`；失败证据语义：10 s 窗口跑完 ⇒ `await killTree` 已返回（taskkill 未挂起）而 victim 未退出，随后存留的 child 阻断测试进程退出——布尔返回值无法区分 spawn 失败 / 非零退出 / 杀后存活。改动：win32 分支捕获 taskkill stderr、对 spawn 失败与非零退出 `console.warn`（含 exit code/stderr），resolve 契约与断言不变。Linux 42/42 通过仅作回归，不替代 Windows 结论 |

### Linux 验证（全部 PASS）

| 检查 | 结果 |
|---|---|
| `cargo fmt --all -- --check` | PASS |
| `cargo test -p xarchive-storage` | PASS 30/30（含新增复现测试） |
| `cargo test --workspace --locked` | PASS 190/190 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | PASS |
| `npm run check` / `npm run test`（root，desktop 42/42 + extension 13/13）/ `npm run build` | PASS |
| `npm ls --all`（本机 npm 11.19.0，含 engines manifest） | PASS exit 0 |
| cleanroom 复现（npm 11.17.0 `npm ci`+`npm ls`；11.18 engines 方案全链） | FAIL(11.17 ls，按预期复现) / PASS(11.18) |
| `node --check desktop/scripts/wdio-tauri-service.mjs`、`git diff --check` | PASS |
| Windows 侧行为 | `BLOCKED`（本会话在 Linux；见队列 focused 项） |

### Windows 队列累计

- 新增 focused revalidation 4 项（WQ-ENG-03 / WQ-ENG-08 / WQ-ENG-09a / WQ-ENG-09b），全部 `WINDOWS_VERIFICATION_PENDING`，记录于 [`../validation/windows-queue.md`](../validation/windows-queue.md) 的 “Cross-platform reconcile round 2”；本轮无 `WINDOWS_VERIFICATION_BLOCKING`。
- 原 Manual Windows Validation Queue（真实账号、GUI/WebView2、Named Pipe、workflow、archive extraction 等）与独立 PASS 复用结论不变。
- 按 AGENTS.md 批量开发、集中验证策略，本轮不在 Linux 阶段切换 Windows；进入 Windows phase 前按最终 diff 统一合并重复场景（队列 focused 表即本轮 handoff 清单）。

### 剩余工作分类

- **下一 Windows phase 立即执行**：队列 round-2 focused 四项（含 npm ≥ 11.18 升级前置、killTree 诊断采集、E2E window-handle 证据采集）。
- **需要人工验证**：原 Manual Windows Validation Queue 项（状态不变）。
- **暂不应继续**：不把本轮 Linux 结果记为 Windows PASS；不再引用被跟踪 evidence.json 作为任何 run 的证据；不在未采集 fresh 证据前改写 4383 行的 session 观察记录（勘误仅针对 “current-run evidence.json” 引用）。

## 2026-09-28 交接：Cross-platform reconcile 第 3 轮（WQ-ENG-09b triage）

本节为本会话写入。起点为 Windows 写回的 round-2 focused 结果（`5170161`）；本轮完成 reconcile、review 与全部非 Windows-dependent 工作，状态更新为 `READY_FOR_WINDOWS`。

### 源状态

- Branch `security/tweet-url-host-validation`；起点 HEAD `517016146e29e08cae38c8e33cfdb85c4b7e08`（先 `git fetch` 并快进；本地此前落后 origin 1 个 commit）。
- 本轮 implementation commit：`e856343ee03e44134f2189cba7e66dc83e00f3a0`（测试层 5 个文件）；handoff 与队列/状态/地图文档在本 commit 之后的 docs commit 记录。
- 治理文档核对：任务所列 `docs/development/platform-ownership.md`、`docs/development/git-platform-handoff.md`、`docs/validation/validation-policy.md` 在本地与 `origin` 树中均不存在；本轮按实际存在的 `AGENTS.md`、`docs/development/cross-platform-validation.md`、`docs/validation/windows.md`、`docs/validation/windows-queue.md`、`docs/development/windows-validation.md` 执行，未臆造这些文档的内容。

### Windows 上一轮结果复核（reconcile）

| 项 | Windows 结论 | Cross-platform 复核 |
|---|---|---|
| WQ-ENG-03 | `WINDOWS_PASS`（storage 27/27；真实 junction harness 1/1，根外文件未被改写） | 与 `01067b6` 的修复一致（`build_archive_metadata` 复用 `resolve_within`；Windows 不编译 `#[cfg(unix)]` 复现测试，故为 27 而非 Linux 30）。接受，无需再动 |
| WQ-ENG-08 | `WINDOWS_PASS`（42/42） | 根因落在验证沙箱的 `taskkill` 权限（提升权限后通过），非产品代码缺陷；本轮加入的 taskkill stderr/exit-code 诊断正是该结论的证据来源。接受 |
| WQ-ENG-09a | `WINDOWS_PASS`（npm 11.19 cleanroom `npm ci`+`npm ls`；npm 11.17 出现预期 `EBADENGINE` 并复现 unsupported 树） | 与 Linux cleanroom 二分结论一致。接受 |
| WQ-ENG-09b | `WINDOWS_FAIL`（EdgeDriver 155 下单一 handle、URL `data:,`、空标题、白屏；GUI 观察 `BLOCKED`） | 本轮 triage 目标，见下 |

### WQ-ENG-09b triage 结论

- **失败特征**：会话成功建立但 WebView 从未提交任何文档（`data:,`、空标题、白屏、仅一个 window handle）。这排除了“前端 JS 运行时报错”——那种情况文档 URL 会是 `tauri://` 协议且通常有错误内容。
- **已排除的仓库侧原因**（对照 2026-09-16 原生 WDIO `WINDOWS_PASS` 的 `3f70894`）：`desktop/src-tauri/tauri.conf.json`（含 `devUrl`/`frontendDist`/`beforeBuildCommand`/CSP/窗口配置）、`desktop/index.html`、`desktop/vite.config.js` 与 tauri 依赖版本**完全未变**；差异只有新增 command、`tauri-plugin-dialog` 注册与 capability 权限，均不参与导航。构建链 `npm run build` → `desktop/dist/index.html` 存在且 `frontendDist: ../dist` 指向它。
- **最可能的剩余类别：测试工具链的 driver ↔ WebView2 runtime 绑定问题**。官方 `@wdio/tauri-service` 按 Windows 注册表中的 **Edge 浏览器版本** 选择并缓存 msedgedriver（`%TEMP%\msedgedriver\{major}\`），而 Tauri 应用由 **WebView2 runtime** 驱动；本轮 Windows 观测到探测值 153 与实际 WebView2 155 不一致，且 4444 端口存在遗留的 153 driver。版本不匹配可解释“会话建立但目标文档为空白”。
- **未被证据排除的备选**：应用自身在 WebView2 下未导航（需独立启动观察）。该项保持人工队列，不由本轮 Linux 证据推断。
- **本轮未修改产品代码**；改动限于测试层可诊断性与 driver 版本可控性。

### 本轮改动（`e856343`）

- 新增 `desktop/e2e/support/startup-diagnostics.mjs`：枚举 window handle，逐个记录 URL/title 与截图，并写入失败原因，输出到 `WDIO_LOG_DIR/startup`（默认 `desktop/test-artifacts/wdio/startup`，已被 `.gitignore` 覆盖）。契约：best-effort，任何采集异常都不改变用例判定、不掩盖原始错误。
- `dashboard.e2e.mjs` / `wdio-plugin.e2e.mjs`：`before` 的 dashboard readiness 超时时先取证，再原样抛出 readiness 错误（附加证据路径）。通过路径行为不变。
- `wdio.conf.mjs`：`TAURI_DRIVER_EDGE_VERSION` 或 `EDGEDRIVER_VERSION` 透传为 service `edgeDriverVersion`；未设置时保持上游按 Edge 注册表探测，行为与改动前一致。
- 新增 `desktop/test/startup-diagnostics.test.mjs`（6 项单测）。

### Linux 验证（按当前 diff 的最小必要范围）

| 检查 | 结果 |
|---|---|
| `node --check`（新增模块、两个 spec、`wdio.conf.mjs`、新测试） | PASS 5/5 |
| WDIO 配置加载三态（未设置 → `undefined`；`TAURI_DRIVER_EDGE_VERSION=155` → `"155"`；`EDGEDRIVER_VERSION=154` → `"154"`） | PASS |
| `npm run test --workspace desktop` | PASS 48/48（42 既有 + 6 新增） |
| 真实 native E2E（Linux 本地） | `NOT RUN`：本机无 Xvfb/显示服务，Linux 侧只能做 config load 与单测；native 会话行为属 Windows 项 |
| Rust 全量 / Sidecar pytest / `npm run build` | `NOT RUN`：本轮 diff 仅涉及 Node 测试层，无 Rust/Python/构建产物变更，按最小必要范围不执行 |
| Windows 侧行为 | `BLOCKED`（本会话在 Linux） |

### Windows 队列累计（交回本轮 focused 清单）

- `WQ-ENG-09b revalidation`（`WINDOWS_VERIFICATION_PENDING`）：记录应用实际加载的 WebView2 runtime 版本 → 清理 `%TEMP%\msedgedriver\*` 与端口残留 → 以 `TAURI_DRIVER_EDGE_VERSION=<runtime major>` 重跑 ordinary/advanced E2E → 提交套件自动生成的 `startup-diagnostics-*.json` 与 `window-*.png`。
- `WQ-ENG-09b standalone comparison`（`WINDOWS_VERIFICATION_PENDING`，人工）：独立启动 binary 与 WebDriver 会话对照；同样空白则升级为产品启动路径问题。
- `WQ-ENG-03` / `WQ-ENG-08` / `WQ-ENG-09a` 的 `WINDOWS_PASS` 继续有效，本轮 diff 未命中其影响面（仅 `desktop/e2e/**`、`wdio.conf.mjs` 与新增测试）。
- 本轮**无 `WINDOWS_VERIFICATION_BLOCKING`**；`WQ-ENG-09b` 在 revalidation 前保持 `WINDOWS_FAIL`，不得记 PASS；`BLOCKED_AUTOMATION` 项（GUI/Computer Use）状态不变。

## 2026-09-28 交接：Cross-platform batch 收口（Plan 中非 Windows 依赖项）

本节记录本 batch 最后一段工作：完成 Plan 中所有不依赖 Windows 的开发与测试，统一汇总 Windows 专属验证项目，并为 `BLOCKED`/`NOT RUN` 项生成手工验证程序。状态仍为 `READY_FOR_WINDOWS`。

### 完成项

1. **关闭历史 Linux 验证缺口（Sidecar）**：`non-windows-completion.md` 此前记载“Linux 未安装 pytest/clippy”，该结论已过时。实测项目 `.venv` 含 `pytest 9.1.1`（Python 3.14.4），本轮实际执行 `python -m compileall -q sidecar`（PASS）与 `python -m pytest sidecar/tests -q`（**19 passed**），并已更正该文档中的过时限制。Sidecar 测试从本轮起是 Linux 常驻门禁的一部分。
2. **`archive_tweet` 全局锁项复核（Plan 中唯一保留的非 Windows 依赖开发项）**：按 ADR-009 处理，**不实施重构**，改为记录代码级证据与前置条件（见 [`../architecture/decisions.md`](../architecture/decisions.md) 的 “2026-09-28 复核”）。核实事实：`desktop/src-tauri/src/archive.rs:544-663` 在整个 `download_sidecar` 与 `complete_sidecar_archive` 期间持有 `RuntimeState` guard，其余 `Mutex<RuntimeState>` 消费者（`commands.rs` 状态/指标、`transport.rs` Browser 入口）在归档期间阻塞；且无 command 级测试覆盖该 fallback。保持 `NEEDS_DEVELOPMENT_REVIEW` 的三条理由：释放锁会改变并发语义（并发归档将从串行变为第二个请求返回 “sidecar is not running”）、正确解法需要 supervisor 租约设计、roadmap 已将入口切换门控在 Windows runtime 证据之后。
3. **Windows 专属项统一汇总 + 手工程序**：在 [`../validation/windows-queue.md`](../validation/windows-queue.md) 追加 “Windows 专属验证项目总览” 一节，按 A（已关闭）/ B（`BLOCKED`）/ C（`NOT RUN`）/ D（待 focused revalidation）/ E（其余 pending，步骤已在原表）分组，并为 B、C 组生成 **M1–M7 手工验证程序**（每项含前置、步骤、预期与 PASS/FAIL 判定）。

### Linux 验证

| 检查 | 结果 |
|---|---|
| `.venv/bin/python -m pytest sidecar/tests -q` | PASS 19/19 |
| `.venv/bin/python -m compileall -q sidecar` | PASS |
| 代码改动 | `NOT RUN`（本轮仅文档；上一 commit `e856343` 的 Node 48/48 仍为最新代码证据） |
| Rust / Node / Windows | `NOT RUN` / 复用 `e856343` 结论 / `BLOCKED`（无 Windows 环境） |

### Windows 队列与阻塞

- 本轮**无 `WINDOWS_VERIFICATION_BLOCKING`**，也未新增任何 Windows 专属实现。
- 交回 Windows 的执行清单：`WQ-ENG-09b` focused revalidation（round-3 表）+ 手工程序 M1–M7 + E 组中命中 diff 的 pending 项。
- 明确跳过（不可由 Linux 解除）：`WQ-ENG-04`（Named Pipe 实现不存在）、`WQ-ENG-06`（无测试账号）、`WQ-WORKER-BUILD-01`/PACKAGE（缺 artifact）、GUI/Computer Use（`BLOCKED_AUTOMATION`）、`WQ-P1-02/03/04/05/13`（Windows 专属实现未完成）。

### 下一 Owner

Windows Owner：先执行 `WQ-ENG-09b` focused revalidation，再按 M1–M7 推进被阻塞项；E 组按 `cross-platform-validation.md` 的影响面分析择命中项执行。若 M5 的独立启动对照显示应用在 WebView2 下同样空白，或 `WQ-ENG-09b` 仍无法归因于 driver/runtime 绑定，则把启动路径问题交回 Cross-platform Owner。

## 2026-09-28 Windows focused phase result

- Implementation revision tested: `01067b66a3aa214fb90f7d893c57bb971a7c7882`; checkout handoff document revision: `a6311b31a6791b9eae0ca531919048d02063d9bd`.
- `WQ-ENG-03` junction containment: `WINDOWS_PASS` (storage 27/27; actual Windows junction probe 1/1, outside file unchanged).
- `WQ-ENG-08` killTree: `WINDOWS_PASS` (Desktop Node 42/42 with elevated retry after sandbox `taskkill` access-denied).
- `WQ-ENG-09a` npm contract: `WINDOWS_PASS` on npm 11.19 cleanroom (`npm ci` and `npm ls`); npm 11.17 emits the expected `EBADENGINE` and still reproduces `ELSPROBLEMS` when unsupported.
- `WQ-ENG-09b` ordinary/advanced Tauri E2E: `WINDOWS_FAIL` on matching EdgeDriver 155; session remained on blank `data:,`. Fresh evidence includes one window handle, empty title, blank screenshot, and zero-byte new app log files. Computer Use native app inventory was unavailable, so visual GUI validation is `BLOCKED` and remains in the manual queue.
- Next Owner: Cross-platform Owner to triage whether the blank WebView is an app navigation/startup issue or session attachment/environment issue. No implementation change is authorized by current evidence alone; return only concrete code/contract changes to Windows for focused revalidation.

## 2026-09-28 交接：Cross-platform batch 3（WQ-ENG-13 修复 + workaround 评审结清）

本节由 Cross-platform Owner 写入，处理 Windows batch 2（`b918d60` / `fa52356`）交回的两项 reconcile 工作。状态 `READY_FOR_WINDOWS`，本轮 `WINDOWS_VERIFICATION_BLOCKING` 为空。

### 源状态核对

- 起点 `8ddb09a`（本地上一轮 handoff），`git fetch` 后发现落后 origin 2 个提交，已 `--ff-only` 快进到 `fa52356`；快进后 working tree clean、与 origin 同步，随后本轮产生 1 个实现 commit + 1 个文档 commit。
- 交接指令引用的 `docs/development/platform-ownership.md`、`docs/development/git-platform-handoff.md`、`docs/validation/validation-policy.md` 在本地与 `origin` 树中**均不存在**（`git ls-tree` 复核）。未臆造这些文件；本轮实际遵循的是 `AGENTS.md`、`docs/development/cross-platform-validation.md`、本文件与 `docs/validation/windows-queue.md`。若这三份治理文档确有需求，应作为独立文档任务补齐并登记到 `docs/architecture/repository-map.md`。

### 本轮完成

1. **WQ-ENG-13（`CROSS_PLATFORM_CHANGE_REQUIRED`）已修复**：只改测试、不改实现。`desktop/test/startup-diagnostics.test.mjs` 原用 `path.join(path.sep, "project", "desktop")` 构造期望值（Windows 上得到无盘符的 `\project\desktop`），而被测 `resolveDiagnosticsDir` 用 `path.resolve`（会把无盘符 root 解析为当前盘符 `E:`）。`path.resolve` 的盘符语义正确，故平台假设在测试侧：改用文件内既有 `tempRoot()` 生成绝对 root + `try/finally` 清理，两个分支断言语义不变。Implementation revision：`88050320e2d76fcd5eb6ceccfca5188ddd8b8e75`。
2. **workaround 评审结清（`CROSS_PLATFORM_REVIEW_REQUIRED`）**：结论为**暂不实施**。否决 `webdriver` 8.x 降级（与 WDIO 9.31.9 / `@wdio/tauri-service` 1.4.0 的 peer 契约冲突，会重现 WQ-ENG-09a 刚闭环的 `npm ls` invalid-tree 问题，legacy 协议已废弃，且 Linux 无法验证）；否决在无法验证的前提下把硬编码 build 输出路径注入 WDIO capabilities。保留 `TAURI_DRIVER_EDGE_VERSION`/`EDGEDRIVER_VERSION` 透传（无害且对上游修复后有用）。改为排入一次**有界实验**（失败即回滚），见队列 batch-3 第 3 节。
3. **文档更正**：`docs/validation/windows-wdio-handoff.md` 的 driver 前置章节此前写死 `152.0.4191.66` 且未说明钉版无效，会误导下一批 Windows。已按 Windows batch 2 实测证据更正为当前事实（WebView2 `153.0.4234.48`、Edge `155.0.4283.18`、可用 driver 153.0.4234.46 / 154.0.4258.24、`--port=NNNN` 等号形式），并新增明确的**停止条件**：上游修复前不重复 ordinary/advanced E2E、不把 `data:,` 空白窗口解读为产品缺陷。同时更正本文件顶部一处被截断的 validation record 哈希。
4. **round-3 结论更正**：round-3 的主导假设 “msedgedriver ↔ WebView2 runtime 版本不匹配” 只是表层症状；即使钉到一致的 153，session 仍无法创建，根因是上游 `tauri-driver` 能力协商缺陷。该更正已写入队列 batch-3 section。

### Linux 验证（按当前 diff 的最小必要范围）

| 检查 | 结果 |
|---|---|
| `node --check desktop/test/startup-diagnostics.test.mjs` | PASS |
| `node --test desktop/test/startup-diagnostics.test.mjs` | **PASS 6/6** |
| full suite / Rust workspace / Sidecar pytest | `NOT RUN`（本轮 diff 仅 1 个测试文件 + 文档；按最小必要范围不跑 full suite） |
| Windows 行为 | `BLOCKED`（本会话在 Linux，无 Windows 环境） |
| `git diff --check` | PASS |

### Windows 队列与阻塞

- 本轮**无 `WINDOWS_VERIFICATION_BLOCKING`**，未接管任何 Windows-specific implementation。
- 交回 Windows：① WQ-ENG-13 确认（`npm test --workspace desktop` 预期 48/48）；② WQ-ENG-09b 有界解除实验（失败即回滚）；③ WQ-P1-16/17 待上游解除后复跑。
- 保持不变：`WQ-ENG-04`（Named Pipe 未实现）、`WQ-ENG-06`（无测试账号）、`WQ-WORKER-BUILD-01`/PACKAGE（缺 artifact）、GUI/WebView2（`BLOCKED_AUTOMATION`）、`WQ-P1-02/03/04/05/13`；Manual 队列与手工程序 M1–M7 状态不变。

### 下一 Owner

Windows Owner：先确认 WQ-ENG-13（唯一需要复跑的测试项），再按队列 batch-3 第 3 节执行有界实验；实验若证伪 tauri-driver 不转发 `alwaysMatch`，立即回滚并把 WQ-ENG-09b ordinary/advanced 维持为 `BLOCKED_AUTOMATION`（上游缺陷，不重复重试），其余队列项按原状态推进。

## 2026-09-28 交接：Cross-platform batch 4（WQ-ENG-09b 契约兼容探针 v2）

本节由 Cross-platform Owner 写入，处理 Windows batch 3（`264f6ed` / `8805032`）交回的探针契约复审。状态 `READY_FOR_WINDOWS`，本轮 `WINDOWS_VERIFICATION_BLOCKING` 为空。

### 源状态核对

- 起点 `2ff52f7`（Windows batch 3 结果写回，与 origin 同步，工作树 clean）。
- 本轮变更：`desktop/wdio.conf.mjs` + 新测试 `desktop/test/wdio-config.test.mjs` + 3 个文档；**无产品代码、Rust、前端、依赖或打包改动**。

### 本轮完成

1. **判定 batch-3 探针失败归属**：`@wdio/tauri-service@1.4.0` 的 `onPrepare` 校验 `browserName` 只接受 `tauri`/`wry`，通过后立即 `delete cap.browserName`。因此 `browserName: "webview2"` 在构造 session 请求之前就被拒，探针**从未触达 tauri-driver**，其结果既不能证实也不能证伪 W3C 转发假设。Windows 的“不兼容即回滚”判定正确，无需重新打开。
2. **新增 Linux 侧契约事实**：服务自带嵌套 `webdriver@9.30.1`（root 为 `9.31.9`），两者 `build/node.js` 中 `desiredCapabilities` 均 0 命中，session 请求只把调用方 caps 原样包成 `alwaysMatch`（`node_modules/webdriver/build/node.js:1326`）。这封死了 legacy 字段路线（8.x 降级继续否决）；服务侧只删除 `hostname`/`port`/`browserName`，无 `beforeSession` 替换钩子，因此未知键随 `alwaysMatch` 透传，W3C 侧注入是唯一可用的探针面。
3. **实现探针 v2（`acda2b6`）**：`WDIO_EDGE_BINARY_PROBE=1` 时把**同一个**已解析 app binary 额外写入 `"ms:edgeOptions": { binary, webviewOptions: {} }`，`browserName` 保持 `tauri`。默认（未设置）时不新增任何键，capability 与探针前逐字节一致，因此现有 Linux/Windows 默认运行零影响。
4. **测试**：`desktop/test/wdio-config.test.mjs` 在隔离子进程中加载真实 config，覆盖默认形状不含 `ms:edgeOptions`、探针形状与 `tauri:options` 同源、`browserName ∈ {tauri, wry}`（v1 失败模式的回归护栏）、driver 钉版仅在显式配置时透传。

### Linux 验证（按当前 diff 的最小必要范围）

| 检查 | 结果 |
|---|---|
| `node --check wdio.conf.mjs` / `node --check test/wdio-config.test.mjs` | PASS |
| `node --test test/wdio-config.test.mjs` | **PASS 4/4** |
| `node --test`（desktop 全量） | **PASS 52/52**（原 48 + 新增 4） |
| `npm run check`（vite build） | `NOT RUN`（`wdio.conf.mjs` 不在 vite 依赖图内，diff 未触及前端/构建输入） |
| Rust workspace / Clippy / Sidecar pytest | `NOT RUN`（diff 无 Rust、Python 或协议改动） |
| tauri-driver 是否真的转发 `alwaysMatch` | `BLOCKED`（Linux 无 WebView2/msedgedriver，只能在 Windows 观察） |
| `git diff --check` | PASS |

### Windows 队列与阻塞

- 本轮**无 `WINDOWS_VERIFICATION_BLOCKING`**；探针默认关闭，不改变任何既有默认路径。
- 交回 Windows（详见队列 batch-4 第 3 节）：① desktop suite 计数确认预期 **52/52**；② **WQ-ENG-09b probe v2** 单次运行（`WDIO_EDGE_BINARY_PROBE=1`，命令见 `windows-wdio-handoff.md` §9），按三种结局之一记录并停止；③ WQ-P1-16/17 仅在结局 ① 后复跑。
- 保持不变：WQ-ENG-09b ordinary/advanced `BLOCKED_AUTOMATION`（上游缺陷）、`WQ-ENG-04`（Named Pipe 未实现）、`WQ-ENG-06`（无测试账号）、`WQ-WORKER-BUILD-01`/PACKAGE（缺 artifact）、GUI/WebView2（`BLOCKED_AUTOMATION`）、`WQ-P1-02/03/04/05/13`；Manual 队列与手工程序 M1–M7 状态不变。

### 下一 Owner

Windows Owner：先跑 ①（唯一计数性确认），再执行 ②（一次，勿重试）。若结局为 ②“仍 `msedge.exe`”，则上游结论定案，标记探针停用并停止一切 ordinary/advanced 重试；若结局为 ①，另立 follow-up 处理 binary 与 build 输出路径一致性，再复跑 WQ-P1-16/17。**不得**为通过测试而修改业务代码、降级 `webdriver` 或改动 `browserName`。

## 2026-09-29 交接：Cross-platform batch 5（WDIO 解阻移植）

本节由 Cross-platform Owner 写入，处理 `origin/windows/webview2-readiness-gate` 交回的两项 follow-up，并从该分支移植 Linux 可验证的 Windows E2E 解阻能力。状态 `READY_FOR_WINDOWS`，本轮 `WINDOWS_VERIFICATION_BLOCKING` 为空。

### 源状态核对

- 起点 `2843e61`（与 origin 同步，工作树 clean）。
- 本轮变更：新增 `desktop/scripts/patch-wdio-tauri-service.mjs` + `desktop/test/patch-wdio-tauri-service.test.mjs` + `desktop/e2e/support/native-startup.mjs` + `desktop/test/native-startup.test.mjs`；删除 `startup-diagnostics.mjs` 及其测试；修改 `desktop/wdio.conf.mjs`、两 spec、`desktop/scripts/wdio-tauri-service.mjs`、两 `test/*.test.mjs`、`package.json`（`postinstall`）、`desktop/package.json`（`pretest:e2e*`）；**无产品代码、Rust、前端、依赖或打包改动**。远程交回的 `CROSS_PLATFORM_REVIEW_REQUIRED`（spawn 补丁）已评审采纳；`CROSS_PLATFORM_CHANGE_REQUIRED`（`sidecar.rs`/schema/fixture）经字节比对确认为有效 current source，不改动；`desktop/src/main.js` 占位不采纳（死代码）；前端 readiness 标记另立 batch。

### 本轮完成

1. **依赖补丁（WQ-P1-16/WQ-P1-17 解阻面）**：根 `postinstall` + `pretest:e2e*` 触发幂等补丁；已安装树（`@wdio/tauri-service@1.4.0`、`@wdio/native-core@1.2.0`）实测命中两个缺陷模式，patch 4 文件 exit 0，二次运行全 `already patched`；单测 10/10（含幂等/容忍未来版本/精确替换/安装树接线）。
2. **可选 driver 通道**：`TAURI_DRIVER_PATH`、`EDGEDRIVER_PATH`（Windows 校验存在并前置 PATH）、`WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`、`WDIO_AUTO_DOWNLOAD_EDGE_DRIVER=0`；`outputDir=logDir`；Windows `tauri:options` 附带 `webviewOptions: {}`；`autoDownloadEdgeDriver` 保留 Windows 默认 true，仅新增显式关闭。单测 4→10（含平台感知断言）。
3. **teardown 端口追踪**：launcher 改为快照实际分配的 driver 端口（driver pool → 配置端口对回退），`collectPortOwnerPids` 接受已快照端口；4 项新增测试。
4. **readiness 合并**：单一 `native-startup.mjs`（session 快照 + `waitForApplicationDocument` + 失败取证 + `discovery.json` 时间线）；额外发现并修复读取失败字段（`<unavailable: …>` 字符串）被误判为已加载文档的问题。两 spec 改用新 gate；`waitForStartupContract` 未移植（前端无标记，另立 batch）。

### Linux 验证（按当前 diff 的最小必要范围）

| 检查 | 结果 |
|---|---|
| `node` 补丁脚本（`postinstall` 等价）+ 二次运行 | PASS（exit 0；4 patched → 全 already patched） |
| patch 单测 / wdio-config 单测 / service 单测 / readiness 单测 | **10/10** / **10/10** / **12/12** / **18/18** |
| `node --check`（新模块、两 spec、`wdio.conf.mjs`、新测试） | PASS 6/6 |
| `npm test --workspace desktop` | **84/84** |
| `npm run check`（vite build） | `NOT RUN`（diff 未触及前端/构建输入；但 desktop `check` 脚本本身为 `vite build`，收口时整体跑一次见下） |
| Rust workspace / Clippy / Sidecar pytest | `NOT RUN`（diff 无 Rust、Python 或协议改动） |
| `git diff --check` | PASS |
| 真实 native session / banner 修复效果 | `BLOCKED`（Linux 无 WebView2/msedgedriver） |

### Windows 队列与阻塞

- 交回 Windows（详见队列 batch-5 第 4 节）：WQ-DRV-01 补丁生效（P0）、WQ-DRV-02 banner 效果（P0）、WQ-ENG-09b-ORD ordinary 新配方（P1）、WQ-TEARDOWN-01 动态端口回归（P1）、WQ-ENG-13 计数确认 **84/84**（P1）、WQ-P1-16/17 保持 `BLOCKED`（P2）。
- BLOCKED 手工步骤：新增 **M8**（readiness 人工对照）、**M9**（driver/端口残留检查）、**M10**（依赖补丁确认）；M1–M7 不变。
- 保持不变：WQ-ENG-03/08/09a 与产品侧 WebView2 渲染 `WINDOWS_PASS`（diff 无交集，复用）；`WQ-ENG-04`（Named Pipe 未实现）、`WQ-ENG-06`（无测试账号）、`WQ-WORKER-BUILD-01`/PACKAGE、hosted gate `NOT RUN`。

### 下一 Owner

Windows Owner：按队列 batch-5 第 4 节顺序执行并逐项记录（P0 → P1 → P2）；`BLOCKED` 项只能走 M8–M10 并单独记录，不得把人工结论记为自动化 PASS。**不得**为通过测试而修改业务代码、降级 `webdriver` 或改动 `browserName`；`postinstall` 只改 `node_modules`，出现 `no … pattern` 警告时记录版本并交回 Linux（可能是上游已修复）。

### 2026-09-29 Windows batch 6 handoff（Full portable package）

- Windows 验证基线：`7e3d646a69e3a75d19b96f0407fde65229edffad`；Full 包：`validation-artifacts\full-package-20260929-7e3d646\XArchive-7e3d646-windows-x64-full\`（manifest/EXE metadata `0.1.0`）。
- 新版 Desktop Release、Full assembly（reuse 本轮 freshly-built binary）、manifest 和 Full 包内 worker/gallery-dl 本地协议 smoke 已通过；规范 fresh-build 命令因 Windows `spawn npm.cmd` 的 `EINVAL` 失败。相同 exe hash 的预组装目录有 runtime initialized 日志，但没有稳定可观察的 GUI；最终 commit-scoped package-root GUI、Codex Edge Profile / Extension 按钮 / 真实下载尚未完成。
- `CROSS_PLATFORM_CHANGE_REQUIRED`：修复 `desktop/scripts/build-portable-windows.mjs` 在 Windows 上启动 `npm.cmd` 的方式，再交 Windows fresh-build 验证。
- `CROSS_PLATFORM_REVIEW_REQUIRED`：评估 `*.pyd` 全局排除对 PyInstaller worker `_internal` 的影响；另确认 Cargo `0.1.1` 与 Tauri/PE/package `0.1.0` 的版本来源。真实下载未跑，不能假定没有影响。
- 下一 Owner：Cross-platform Owner 处理上述两项并更新 Plan / handoff；修复后返回 Windows Owner 完成 fresh Full build 和 queue M11–M13。队列细节见 [`../validation/windows-queue.md`](../validation/windows-queue.md) batch 6。无 `WINDOWS_VERIFICATION_BLOCKING`。

## 2026-09-29 交接：Cross-platform batch 6（Windows batch 5/6 reconcile 与 Linux 收口）

本节由 Cross-platform Owner 写入，按 [`../development/cross-platform-validation.md`](../development/cross-platform-validation.md) 合并 Windows batch 5（handoff `589142f`）与 batch 6（implementation `7e3d646`）的结果，并完成其中全部 Windows-independent 交回项。状态 `READY_FOR_WINDOWS`，本轮 `WINDOWS_VERIFICATION_BLOCKING` 为空。

### 源状态核对

- 起点 `589142f`（本地 HEAD），落后 origin 4 个 docs 提交（`169f425`、`7e3d646`、`62486ec`、`584eee8`）；已 stash → `--ff-only` 快进到 `584eee8` → stash pop，无冲突（合并时以 origin 为准回退 `7e3d646` 中随后被 `62486ec` 判定必须回滚的 `.pyd` 删减等，并叠加本轮新增）。
- 本轮变更（在合并后的树上）：`desktop/scripts/build-portable-windows.mjs`（`main()` + 直接执行守卫）、`desktop/scripts/portable-package.mjs`（移除 `*.pyd`、新增 `portablePackageVersion`）、`desktop/wdio.conf.mjs`（`WDIO_DIRECT_DRIVER` 直连模式）、`package.json` / `desktop/package.json` / `desktop/src-tauri/tauri.conf.json`（`0.1.0` → `0.1.1`）、`desktop/test/portable-package.test.mjs` / `desktop/test/wdio-config.test.mjs`（新增 6 项测试）；**无 Rust 产品代码、Extension、依赖锁或协议改动**。

### 本轮完成（A–D，详见队列 “Cross-platform batch 6” 节）

1. **A（`CROSS_PLATFORM_CHANGE_REQUIRED` 关闭）**：`npm.cmd` spawn 修复收口为可测的 `main()` + 直接执行守卫，并修复上轮遗留的未闭合 `main()` 与缺失入口调用（曾致 `node --check` 失败）。
2. **B（`CROSS_PLATFORM_REVIEW_REQUIRED` 关闭）**：`*.pyd` 移出全局排除清单——`6555d35` 意图是本地产物 dll 卫生，但全局排除误伤 PyInstaller one-dir worker `_internal` 的 7 个运行时 C 扩展（协议 smoke 不经真实 HTTPS，不能证明无影响）；`.pyc`/`.pyo`/`__pycache__`/`.env` 等卫生项保留，并加回归测试。
3. **C（`CROSS_PLATFORM_REVIEW_REQUIRED` 关闭）**：版本来源统一——canonical = Cargo workspace `0.1.1`；`tauri.conf.json`、根/desktop `package.json` 对齐；manifest 默认派生 `tauri.conf.json`（`PORTABLE_APP_VERSION` 仍可覆盖）；extension manifest 维持 `0.1.0`（独立生命周期，本轮回滚未点名）；新增跨文件一致性测试。
4. **D（`CROSS_PLATFORM_REVIEW_REQUIRED` 关闭）**：接受 tauri-driver legacy-caps 归因；实现并文档化 `WDIO_DIRECT_DRIVER=1` 直连独立 msedgedriver 配方（batch-2 步骤 4/5 形状已有成功证据：session `200`、`browserVersion=153.0.4234.48`、`h1=工作台` 截图）；未设置时默认路径逐字节不变。

### Linux 验证（全量门禁，当前 diff）

| 检查 | 结果 |
|---|---|
| `node --check`（5 个目标文件） | PASS 5/5 |
| `node --test test/portable-package.test.mjs test/wdio-config.test.mjs` | PASS **31/31** |
| `npm test`（根，含 desktop 与 extension） | PASS **91/91** + extension **13/13** |
| `npm run check`（vite build + extension check） | PASS |
| `cargo fmt --all -- --check` / `cargo test --workspace --no-fail-fast` / `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS / PASS / PASS |
| `.venv/bin/python -m pytest sidecar/tests -q` | PASS 19/19 |
| `git diff --check` | PASS |
| 真机 fresh Full build / WebView2 direct session | `NOT RUN`（本会话在 Linux，交下一 Windows 批次） |

### Windows 队列与阻塞

- 交回 Windows（队列 “Cross-platform batch 6” 第 4 节）：WQ-PACKAGE-FULL-01-R2（P0 fresh Full build：无 `EINVAL`、`.pyd` 随包、版本 0.1.1）、WQ-ENG-09b-ORD-R2（P1 direct recipe 单次）、WQ-ENG-13-R2（P1 计数 **91/91**）、M11–M13（人工，batch-6 原文）。
- 保持：WQ-PACKAGE-FULL-01 `WINDOWS_FAIL` 直至 R2 通过；WQ-ENG-09b-ORD 旧配方停止条件不变；WQ-P1-16/17 `BLOCKED`；M8 `BLOCKED_AUTOMATION`；M1–M10 与无交集 `WINDOWS_PASS` 项复用。
- 本轮**无 `WINDOWS_VERIFICATION_BLOCKING`**；未接管任何 Windows-specific implementation。

### 下一 Owner

Windows Owner：按队列 “Cross-platform batch 6” 第 4 节 P0 → P1 → 人工顺序执行并逐项记录；R2 失败时保留日志与错误原文交回 Cross-platform Owner，不得改用旧配方重试，不得为通过测试修改业务代码。
