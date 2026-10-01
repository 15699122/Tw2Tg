# Release Policy

Owner: Cross-platform Owner, with the Windows Platform Owner for native acceptance and packaging.
Status: `ACTIVE` — general policy for every release of this repository.

This document defines when a revision may become a release candidate and what may be published. It does not record a specific batch; batch scope lives in [`../development/branch-integration-release-plan.md`](../development/branch-integration-release-plan.md) and per-version facts live in [`release-history.md`](release-history.md).

## 1. Authority and reading order

| Question | Source of truth |
|---|---|
| May this revision be published, and under what conditions | this document |
| What to check and what evidence to record | [`release-checklist.md`](release-checklist.md) |
| What actually exists for each version | [`release-history.md`](release-history.md) |
| What a version advertised | [`notes/`](notes/) |
| What CI actually enforces | [`.github/workflows/`](../../.github/workflows) |
| Validation states, deferral and fallback rules | [`../validation/validation-policy.md`](../validation/validation-policy.md) |
| Platform ownership and handoff | [`../development/platform-ownership.md`](../development/platform-ownership.md) |

A rule here that contradicts a workflow is a recorded difference to resolve, not a statement that the workflow already implements it. When they disagree, the workflow output and the recorded evidence describe what happened; this document describes what is intended.

## 2. Stages

- **Development build** — built from a working revision, not published.
- **Pre-release** — `vMAJOR.MINOR.PATCH-preN`, published for broad technical verification.
- **Release candidate** — a pre-release whose revision is frozen and whose artifacts are verified against the acceptance scope in section 4. This is a *state*, not a tag spelling.
- **Release** — `vMAJOR.MINOR.PATCH`.

`v0.x.y` without a pre-release suffix does not by itself mean production ready. Maturity must be stated explicitly in the notes: this project is pre-1.0, and a release may still declare an unverified scope.

## 3. Candidate admission

A revision may be declared a release candidate when all of the following hold.

1. **Scope frozen.** The advertised feature set and the deferred items are written down.
2. **Pinned source.** The candidate is a full commit SHA. A dirty working tree is not a candidate.
3. **No uncommitted source.** Only documentation may be uncommitted; any uncommitted source change blocks the candidate.
4. **Traceable handoff.** The branch, source commit, handoff commit, uncommitted state and current owner are recorded per [`../development/git-platform-handoff.md`](../development/git-platform-handoff.md).
5. **Regression executed.** Validation is selected by risk per [`../validation/validation-policy.md`](../validation/validation-policy.md). If the full regression was not run, the reason is recorded, not the gap.
6. **Advertised scope has acceptance evidence.** Every capability named in the notes has a corresponding result.
7. **Every non-PASS item is described.** Each FAIL, BLOCKED and NOT_RUN item carries its reason, impact, owner and follow-up.
8. **Security disposition recorded.** Open advisories and scanning alerts have an explicit decision. The count alone is not a decision, and "not blocked by them" is not a conclusion.

A frozen source candidate and a verified candidate artifact are different states. Asset completeness cannot be claimed before the artifacts exist and have been checked.

## 4. Gates

### 4.1 Non-waivable artifact integrity

These block publication. An authorization in section 6 does not waive them.

- Every expected asset exists, is non-empty, and matches the declared asset contract.
- The published source equals the tag target; manifest `source_sha` agrees with it.
- `SHA256SUMS` agrees with the manifest, and every manifest hash agrees with the actual file.
- The executable hash in the manifest agrees with the independently recomputed value.

### 4.2 Acceptance scope

Gates correspond to the scope the notes advertise. A capability that is advertised needs acceptance evidence; an undisclosed capability is not accepted by silence.

A known defect that affects the advertised scope is not dismissed because the automated UI job is non-blocking. Non-blocking describes the automated job's effect on publication, not the defect's effect on users.

### 4.3 Deferrable platform validation

Deferral follows [`../validation/validation-policy.md`](../validation/validation-policy.md):

- A Windows UI job that fails or is blocked does not by itself block publication, but the real result, environment, driver details, logs and diagnostics are preserved.
- A missing input or a failed identity check is NOT_RUN, never PASS.
- Missing input or a mismatched asset is a blocking packaging failure.
- A green build or a successful publication is not Windows GUI acceptance.
- Deferral is recorded as `WINDOWS_WORK_PENDING` or `WINDOWS_VERIFICATION_PENDING`, with a manual queue entry where needed. `WINDOWS_BLOCKING` is reserved for cases where continuing would be unreliable.
- A previous version's PASS is never reused. Reuse requires unchanged code, dependencies, platform contracts and environment; otherwise record `REVALIDATION_REQUIRED`.

## 5. Ownership

- **Cross-platform Owner (Linux)** — shared source, contracts, integration line, candidate freeze, Linux-side verification.
- **Windows Platform Owner** — native acceptance, packaging, platform evidence, artifact-level confirmation.
- **Release Agent** — assembles the candidate from an existing authorized decision, executes the checklist, and records the outcome. It does not invent a scope, and it does not turn a deferred check into a passed one.

A finding that requires changing a shared contract returns to Linux as `CROSS_PLATFORM_CHANGE_REQUIRED` before the candidate is refreshed.

## 6. Restricted development-version publication

A version may be published before every gate closes, when the Owner authorizes it explicitly. The record must contain:

- the authorizer and the date;
- the pinned source SHA;
- each deferred item and its reason;
- the known risk;
- the reduced advertised scope;
- the notes requirement stating which capabilities are *not* claimed.

An authorization does not change a real result, does not promote an unverified capability, and does not bypass section 4.1. It authorizes publication of a narrower claim, not a stronger one.

## 7. Numbering and tags

- The live pre-release series is `vMAJOR.MINOR.PATCH-preN`.
- The retired `vMAJOR.MINOR.PATCH-pre.N` series is read-only historical evidence and is never republished.
- Existing tags are never rewritten and existing history is never force-pushed. A renumbering migration is a separate, recorded event; see [`migration/pre-release-renumbering.md`](migration/pre-release-renumbering.md).
- Deleting or replacing a published Release is a separate authorized decision, performed only after the replacement is verified, and it is recorded in the history.

## 8. Candidate tag spelling

This policy defines the release candidate as a state and deliberately introduces no `-rcN` tag. No workflow currently accepts that spelling, so creating one would break the tag validation that guards the publish path. If a distinct RC series is ever required, it is a separate change covering the tag pattern, the GitHub pre-release flag and the workflow routing, executed before any RC tag exists.

## 9. Channel behaviour

The build channel decides the default log level: a pre-release defaults to Debug, a release defaults to Info, and an explicit user setting always wins. The channel must come from the validated release tag, never from the build profile, because release builds are always optimized and a profile check would classify every pre-release as a release.