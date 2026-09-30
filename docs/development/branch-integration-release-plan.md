# Branch Integration and v0.2.0 Release Plan

Owner: Linux Cross-platform Owner (integration), Windows Platform Owner (Windows acceptance and publishing artifacts).
Status: `IN_PROGRESS` — plan approved 2026-09-30; the `v0.2.0` tag must not be created until every release gate in section 5 is satisfied.

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

- A squashed branch reports unique commits while its content is already present. Content comparison, not SHA counting, decides disposition.
- No branch is deleted, and no existing history or tag is rewritten by this plan.
- Force-push to `dev`, `main` or any release branch is not part of this plan.
