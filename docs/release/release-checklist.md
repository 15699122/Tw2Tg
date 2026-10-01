# Release Checklist

Owner: Release Agent, executed under [`release-policy.md`](release-policy.md).
Status: `TEMPLATE` — this file is a reusable template. It does not accumulate per-release results.

Copy this checklist into the release record for one version and fill it in. Record each item as:

```text
state | evidence location | owner | blocker or skip reason | follow-up
```

States are `PASS`, `FAIL`, `BLOCKED`, `NOT_RUN`, `NOT_APPLICABLE`. Every non-PASS item needs a reason, an evidence location and a follow-up. A checkbox alone is not a record.

**Release under preparation:** `<tag>`
**Candidate source SHA:** `<full 40-character SHA>`
**Executed by / date:** `<owner>` / `<YYYY-MM-DD>`

## 1. Release identity

| # | Item | State | Evidence | Owner | Note |
|---|---|---|---|---|---|
| 1.1 | Target tag matches the `vMAJOR.MINOR.PATCH-preN` or `vMAJOR.MINOR.PATCH` pattern | | | | |
| 1.2 | Branch, source commit, handoff commit and current owner recorded | | | | |
| 1.3 | Working tree has no uncommitted source change | | | | |
| 1.4 | Maturity and advertised scope stated in the notes | | | | |
| 1.5 | Deferred items listed explicitly | | | | |

## 2. Scope freeze

| # | Item | State | Evidence | Owner | Note |
|---|---|---|---|---|---|
| 2.1 | Feature scope frozen; no in-flight change alters the advertised scope | | | | |
| 2.2 | Version and channel reflected in source and packaging | | | | |
| 2.3 | Channel derived from the validated tag, not from the build profile | | | | |
| 2.4 | Known issues and their user-visible effect recorded | | | | |

## 3. Source verification

| # | Item | State | Evidence | Owner | Note |
|---|---|---|---|---|---|
| 3.1 | Selected test scope justified by risk; skipped scope justified | | | | |
| 3.2 | Formatter and linter clean (Rust, TypeScript) | | | | |
| 3.3 | Unit and module tests pass on the candidate SHA | | | | |
| 3.4 | Build passes | | | | |
| 3.5 | Full regression run, or its omission recorded with the reason | | | | |
| 3.6 | Log level follows the channel; a pre-release is verbose and a release is not | | | | |

## 4. Platform acceptance

| # | Item | State | Evidence | Owner | Note |
|---|---|---|---|---|---|
| 4.1 | Windows acceptance performed on this exact revision | | | | |
| 4.2 | Automated UI job result recorded as the real outcome | | | | |
| 4.3 | Manual queue items executed or registered with a blocker | | | | |
| 4.4 | Deferred items recorded as `WINDOWS_WORK_PENDING` / `WINDOWS_VERIFICATION_PENDING` | | | | |
| 4.5 | No previous version's PASS reused without `REVALIDATION_REQUIRED` | | | | |

## 5. Artifact verification

| # | Item | State | Evidence | Owner | Note |
|---|---|---|---|---|---|
| 5.1 | Every expected asset exists and is non-empty | | | | |
| 5.2 | Asset names and kinds match the asset contract | | | | |
| 5.3 | Tag, checked-out source and workflow SHA agree | | | | |
| 5.4 | Manifest `source_sha` agrees with the tag target | | | | |
| 5.5 | `SHA256SUMS` agrees with the manifest | | | | |
| 5.6 | Manifest hashes agree with independently recomputed hashes | | | | |
| 5.7 | Downloaded artifact spot-checked from the published Release | | | | |

## 6. Release decision

Decision: `GO` / `NO-GO` / `RESTRICTED` (requires an explicit authorization record under policy section 6).

| # | Item | State | Evidence | Owner | Note |
|---|---|---|---|---|---|
| 6.1 | All non-waivable integrity checks passed | | | | |
| 6.2 | Advertised capabilities have acceptance evidence | | | | |
| 6.3 | Every non-PASS item is described and assigned | | | | |
| 6.4 | Security advisories and scanning alerts have a disposition | | | | |
| 6.5 | Restricted decision records authorizer, SHA, risk and reduced scope | | | | |

## 7. Post-release review

| # | Item | State | Evidence | Owner | Note |
|---|---|---|---|---|---|
| 7.1 | Actual Release assets match what the notes describe | | | | |
| 7.2 | Published notes do not advertise an unverified capability | | | | |
| 7.3 | Independent UI validation is tied to the same build run, source SHA and executable hash | | | | |
| 7.4 | Diagnostics artifact retained for any failed or blocked run | | | | |

## 8. Documentation closeout

| # | Item | State | Evidence | Owner | Note |
|---|---|---|---|---|---|
| 8.1 | `notes/<tag>.md` written and reflects the real outcome | | | | |
| 8.2 | `release-history.md` updated with this version | | | | |
| 8.3 | Platform handoff and validation queue updated with the current state | | | | |
| 8.4 | Repository map and documentation index updated if files were added or moved | | | | |

## Reminders

- A green build is not GUI acceptance; a successful publication is not product acceptance.
- Never write PASS for a check that was not executed.
- Do not generalize one artifact's success to another shape of the same release.
- This template carries no results; the filled copy belongs to one version's record.