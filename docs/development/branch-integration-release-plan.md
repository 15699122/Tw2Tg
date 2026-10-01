# Branch Integration and v0.2.0 Release Plan

Owner: Linux Cross-platform Owner (integration), Windows Platform Owner (Windows acceptance and publishing artifacts).
Status: `PUBLISHED_WITH_OPEN_ACCEPTANCE` — Owner authorized publication on 2026-09-30 despite open acceptance gates. Tag `v0.2.0` is at `7910033c9bdb2c649383ee9ddc7af063b258c8c7`; run `36705896154`. Telegram is explicitly unavailable/unadvertised. The original gate plan below is historical; publication does not imply acceptance PASS. Current Windows results and remaining work are in `../validation/windows-queue.md` (2026-10-01).

## 1. Objective

Converge the remaining branches into one integration line, re-validate that line, and then publish `v0.2.0` from a fixed `main` revision.

The approved shape is **categorical convergence**, not an unconditional merge of every branch:

> fixed baseline on `main` → advance `dev` to that baseline → review every other branch and give each a disposition → validate the resulting candidate revision → integrate `dev` into `main` → tag `v0.2.0` on the final `main` SHA.

## 2. Terminology

- **Already included**: the branch tip is an ancestor of `main`, or the change was incorporated through a squash merge. Either way no further merge is required.
- **Superseded**: the branch content is contained in the baseline, or a newer implementation replaced it. Recorded with the reason; not merged again.
- **Historical release branch**: retains the record of a published release. Never merged wholesale into the integration line.
- **Windows-owned branch**: requires Windows Platform Owner review and validation before integration.

## 3. Branch inventory (as of `main` `b93bfcfee93c991fed37c70ea01800d9123c78f1`)

| Branch | Unique / behind | Disposition | Rationale |
|---|---:|---|---|
| `dev` | 0 / 172 | advance to `main` | ancestor of `main`; no unique work |
| `feat/extraction-aria2-pipeline` | 0 / 164 | already included | ancestor of `main` |
| `feature/u7-desktop-production-integration` | 0 / 92 | already included | ancestor of `main` |
| `release/v0.2.0` | 0 / 6 | already included | pre-release series work already in `main` |
| `security/tweet-url-host-validation` (PR #5) | 0 / 131 | already included | head is an ancestor of `main`; Code scanning #1 is `fixed` |
| `security/dependency-advisories-2026-09-30` | 6 / 4 | already included (squash) | content landed via PR #9 (`ed8c1fc`) and PR #7 (`f3c0876`); unique SHAs are squash artifacts, not missing work |
| `security/dev-tweet-url-host-validation` (PR #10) | 1 / 172 | review after `dev` advances | backport of host validation to the old `dev` line; decide whether it still adds test coverage |
| `release/v0.1.1` | 2 / 164 | historical | carries `chore: prepare v0.1.1 release` and a Windows `PathBuf` import fix; do not merge wholesale |
| `release/v0.2.0-pre.1` | 2 / 164 | historical | carries `chore: prepare v0.2.0-pre.1 release`; release preparation must not be replayed onto the `0.2.0` line |
| `windows/webview2-readiness-gate` | 7 / 134 | Windows batch | contains `fix(windows): track allocated WebDriver ports in cleanup` and `fix(windows): launch WebView2 E2E native driver reliably`; requires Windows review and regression evidence |

Rules for this inventory:

- Ahead/behind counts come from the GitHub compare API against the recorded `main` SHA; they are not a merge instruction on their own.

## 4. Execution phases

### Phase 1 — Freeze the inventory

Record the branch list, the base SHA, the open pull requests and the tag list. Confirm the Windows Owner has no unpushed Windows work before `dev` moves.

### Phase 2 — Establish the `dev` baseline

1. Advance `dev` to the current `main` using a fast-forward; `dev` is an ancestor, so no content conflict is possible.
2. Re-evaluate PR #10 against the new baseline. If the backport carries test cases the baseline lacks, port only those tests. If the baseline already covers it, close the PR as superseded.
3. PR #5 stays targeted at `dev` as requested. Once `dev` contains the baseline its content is already present; record it as included rather than replaying a 41-commit batch.
4. Extract from historical release branches only genuinely missing fixes. Do not replay `chore: prepare ... release` commits onto the `0.2.0` line.

### Phase 3 — Windows batch

1. Windows Platform Owner reviews `windows/webview2-readiness-gate` and decides: integrate, extract selectively, or superseded.
2. Windows Platform Owner completes WQ-SEC-PERMS-01 (explicit least-privilege `permissions` in `.github/workflows/windows-worker-artifact.yml`, dispatch, artifact verification, Code scanning #2 recheck).
3. Windows acceptance runs against the explicit candidate SHA, not against a moving branch.
4. Code and evidence return through Git. The branch is not integrated before its evidence exists.

### Phase 4 — Freeze the candidate and sync `main`

1. Update `docs/status/platform-handoff.md`, `docs/development/status.md` and `docs/development/risk-register.md` for the final revision.
2. Review the final `git diff`.
3. Run the validation escalation appropriate to the diff, plus the default-branch CodeQL scan.
4. Integrate `dev` into `main`.
5. Confirm the final relationship between `dev` and `main` and record both SHAs. A fast-forward relationship is preferred; a squash merge is acceptable if recorded, but the two branches must end up serving the same content.

### Phase 5 — Publish `v0.2.0`

1. Fix the release SHA: the `main` commit the tag will point at.
2. Prepare the draft release and decide the tag/asset ordering before the tag exists, so no empty public release is exposed.
3. `.github/workflows/windows-release.yml` triggers on `v*` tags and verifies that the checked-out commit equals the tag commit. Pushing `v0.2.0` is therefore an action with build and upload side effects, not a harmless marker.
4. Build from the release SHA, then verify the asset list, embedded versions, `package-manifest.json`, SHA-256 sums and tag/source parity.
5. Run the release-scope acceptance on the candidate assets.
6. Publish only when every gate in section 5 is met.
7. If a source fix is required after the tag exists, do not silently move the tag. Fix forward, re-validate, and produce a new candidate.

## 5. Release gates for `v0.2.0`

Proposed release scope: Windows portable Core/Full plus the browser Extension. Installer, code signing, the updater and a Linux GA artifact are explicitly out of scope for this release and are deferred, not silently omitted.

| Gate | Requirement | Current state |
|---|---|---|
| Linux shared validation on the final diff | tests, build, static checks pass | must be re-run for the final diff |
| CodeQL on the default branch | all languages complete successfully | required; `pending` is not success |
| Windows fresh build, worker runtime, asset integrity | pass on the candidate SHA | not run for the candidate |
| Windows portable startup and core GUI | pass | not run for the candidate |
| Extension install, real browser pairing, task submission | pass | not run for the candidate |
| Core archive run completes with correct output and reliable state after restart | pass | a task reaching `Downloading` is not a pass |
| Telegram send, if advertised in this release | pass with a controlled account | not run; decide whether it is advertised |
| Integrated Windows branch fixes | regression evidence for the port-cleanup and driver-launch changes | not run |
| WDIO native E2E | upstream `tauri-driver` capability defect blocks the suite | may be replaced by recorded manual acceptance; an unexecuted suite is never recorded as PASS |
| Open security advisories | scope, mitigation, acceptance owner and review date recorded | `glib`, `extract-zip` and the worker workflow permission item remain open |

`WINDOWS_VERIFICATION_BLOCKING` absence means no Linux cross-platform work is blocked. It is not a release approval.

### Security advisory scope for the release

- `glib`: the dependency chain is Linux-only in the current lock file. That bounds the Windows release scope but must not be reused as a Linux GA justification.
- `extract-zip`: no fixed version exists upstream. The release pipeline does not exercise the extraction path. RISK-023 remains, with mitigation and review date recorded.
- Windows worker workflow permissions: handled in Phase 3.

## 6. Documentation routing

- Ownership and handoff: `docs/development/platform-ownership.md`, `docs/development/git-platform-handoff.md`
- Current handoff state: `docs/status/platform-handoff.md`
- Windows-only work: `docs/validation/windows-queue.md`
- Security remediation: `docs/development/security-remediation-plan.md`
- Risk acceptance: `docs/development/risk-register.md`
- Release notes history: `docs/releases/`

## 7. Status rules

- `IN_PROGRESS`: the plan is approved and execution has started.
- `READY_FOR_WINDOWS`: the Linux side of a phase is complete and Windows action is required.
- `BLOCKED`: a phase cannot start; the reason is recorded.
- `RELEASED`: the tag exists, assets are published, and the release gates were satisfied beforehand.

## 8. Execution log (2026-09-30)

### Phase 1 — Freeze the inventory: complete

Recorded the branch list, the base SHA `b93bcfc`, open PRs and tags through the GitHub API. No branch, tag or release was changed.

### Phase 2 — Establish the `dev` baseline: complete

- `dev` was fast-forwarded from `1786c6a2` to `main`, then again to the final `60110a6f000887d134bf5fc46a7b2d6da91635a8`. `dev` and `main` are now identical and `dev` is a strict fast-forward of `main`; no merge commit or force-push was needed.
- PR #10 (the `dev` backport) was reviewed and **closed as superseded**. Diffed against current `main` it would delete 1137 lines across 15 extension files. Its URL test corpus was compared with numeric IDs normalized: 20 distinct shapes in PR #10, 32 in `main`, 6 unique to PR #10, and all 6 already return the correct value under the `main` implementation. The two assertions `main` did not pin were ported directly instead.
- PR #11 (squash `401a760`) added this plan document. PR #12 (merge `60110a6`) added the two ported assertions in `extension/tests/content.test.js`. Both were `CLEAN` with all four CodeQL languages `SUCCESS` before merge.
- Historical release branches `release/v0.1.1` and `release/v0.2.0-pre.1` were not merged. Their `chore: prepare ... release` commits must not be replayed onto the `0.2.0` line.
- **PR #5 caveat:** after `dev` advanced, GitHub automatically marked PR #5 as `MERGED` (`61ff1ae`) because its head became an ancestor of the base branch. This was not a deliberate merge. Its head `edab0d6` was already contained in `main` before this batch, `61ff1ae` is a pre-existing historical commit, and the tree at `60110a6` is unchanged by it. No regression was introduced, but the PR's base branch is no longer a live work item.

### Phase 3 — Windows batch: complete for the Linux-owned parts

`windows/webview2-readiness-gate` was dispositioned as **superseded**: both of its `fix(windows):` commits are already present in the current code, so Linux performed no merge of that branch.

The Windows validation batch for candidate `5f952d0` returned three commits on `codex/windows-validation-5f952d0`, **all now integrated to `main` and `dev` at `fe3feee7e1f6089b370d64c32f484dbe1fd438b5`** by fast-forward, with no merge commit:

- `951453c` — explicit least-privilege `permissions` plus `persist-credentials: false` in `.github/workflows/windows-worker-artifact.yml`. Executed on Windows as run `36700506149` (SUCCESS). Code scanning alert #2 is now `fixed` on the default branch by rescan (`fixed_at` 2026-09-30T10:40:33Z, `dismissed_by` null), not dismissed by a human. Open code-scanning alerts: 0.
- `4d1d914` — shared test-fixture change in `desktop/test/wdio-config.test.mjs`, flagged `CROSS_PLATFORM_REVIEW_REQUIRED`. **Reviewed and accepted; the flag is discharged.** The fixture writes a zero-byte placeholder so the Windows-only `existsSync` guard in `wdio.conf.mjs` is satisfied while the test only imports configuration. Linux reviewer evidence: the same test passes *without* the fixture because that guard sits behind `process.platform === "win32"`, and a direct probe of the guard shows it would `THROW EdgeDriver executable not found` with no placeholder and proceed with one. The fixture is required on Windows, does not weaken production validation, and the placeholder is never spawned.
- `fe3feee` — validation records, the manual acceptance queue M-CAND-01..04, and the handoff.

### Phase 4 — Freeze the candidate and sync `main`: complete for this iteration

`dev` and `main` are both at `fe3feee7e1f6089b370d64c32f484dbe1fd438b5` and `dev` is a strict fast-forward of `main`. Linux validation actually executed on the integrated tree after a clean `rm -rf node_modules && npm ci`: `npm test --workspaces` desktop **154/154** and extension **33/33**; `npm run check` PASS; `npm run build` PASS; `cargo fmt --check`, `cargo check --locked --all-targets` and strict Clippy `-D warnings` PASS; `cargo test --workspace --locked` **262/262**; `pytest sidecar/tests` **46/46**. No Rust, Python, dependency or lock-file change was involved, but a desktop test file changed, so the suite was rerun rather than assumed.

This was the freeze candidate before the explicit Owner publication authorization. See the current status above and `../releases/v0.2.0.md`; remaining acceptance gates are tracked separately from publication.

### Phase 5 — Publish `v0.2.0`: published under Owner authorization

`v0.2.0` was published from `7910033` in run `36705896154`; seven assets and source/hash identity passed G3. Owner authorization explicitly allowed open acceptance gates and excluded Telegram availability. Windows portable Core/Full plus Extension is the published scope; installer, signing, updater and Linux GA remain deferred. The 2026-10-01 Windows observations advance startup/Sidecar acceptance, while completed archive/task persistence and the remaining filesystem acceptance stay open. Historical WDIO FAIL is preserved and the same failed recipe is not rerun.


- A squashed branch reports unique commits while its content is already present. Content comparison, not SHA counting, decides disposition.
- No branch is deleted, and no existing history or tag is rewritten by this plan.
- Force-push to `dev`, `main` or any release branch is not part of this plan.
