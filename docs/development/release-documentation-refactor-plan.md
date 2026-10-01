# Release Documentation Refactor Plan

Owner: Linux Cross-platform Owner.
Status: `APPROVED` — approved by the user; the refactor consolidates release documentation into a single directory and removes the overlapping one.

## 1. Objective

Give the Release Agent one fixed entry point for how a release is decided, executed and recorded, without re-deriving the process from individual release notes.

The approved shape is a single release directory. There is deliberately no second, overlapping directory.

## 2. Target layout

```text
docs/
└── release/
    ├── release-policy.md
    ├── release-checklist.md
    ├── release-history.md
    ├── notes/
    │   └── v*.md
    └── migration/
        ├── pre-release-renumbering.md
        └── pre-release-asset-ledger.json
```

| Document | Answers | Does not own |
|---|---|---|
| `release-policy.md` | When a revision may become a candidate, what may be deferred, who decides | Per-round test logs |
| `release-checklist.md` | What to check and which evidence to record for one release | Generated PASS results |
| `release-history.md` | Where each version's facts and evidence live | Full release notes |
| `notes/` | What a specific version published and its limits | General release rules |
| `migration/` | Retired numbering rules and the machine-readable frozen ledger | Current release gating |

## 3. Scope

1. Add the three process documents.
2. Move `docs/releases/v*.md` to `docs/release/notes/`, the renumbering record to `docs/release/migration/`, and the frozen ledger to `docs/release/migration/`.
3. Delete `docs/releases/` entirely; no stub or redirect file is kept.
4. Repoint the pre-release and migration workflows, and any test fixture that resolves the ledger path.
5. Register the new documents in `docs/README.md`, `docs/architecture/repository-map.md` and `AGENTS.md`.

Out of scope: creating any tag or GitHub Release, changing publish semantics in the workflows, rewriting historical Windows validation results, and any unrelated refactoring.

## 4. Content of the new documents

## 6. Path updates required

- `.github/workflows/pre-release.yml`: notes check and `--notes-file`.
- `.github/workflows/pre-release-series-migration.yml`: ledger path, notes path, input description.
- `desktop/test/pre-release-migration-cli.test.mjs`: ledger path built from the project root.

Only in-repository paths change. GitHub `/releases/` URLs, external asset URLs and publish semantics are untouched.

## 7. Validation

Documentation and workflow-path change, so validation stays proportional:

1. Check every relative link in the moved and new documents.
2. Confirm the notes files and the ledger resolve at their new paths.
3. Run the affected pre-release migration tests.
4. Search for any surviving `docs/releases/` reference.
5. `git diff --check`, then review the final diff including the moves.

Not run: the full Rust, Node and Python suites, and any real release or Windows build, because this change touches no runtime code, dependency or lock file.

## 8. Completion criteria

1. `docs/release/` is the only release documentation directory.
2. The three process documents have distinct responsibilities and a fixed reading order.
3. No historical evidence is lost and every deletion has a recorded reason.
4. The workflows and the migration test resolve the new paths and pass.
5. No broken link, no fabricated result, and no rewritten tag or asset.

## 9. Progress

- [x] Step 1 — plan approved and recorded in this file.
- [x] Step 2 — created `docs/release/` with `release-policy.md`, `release-checklist.md` and `release-history.md`.
- [x] Step 3 — moved 33 notes to `docs/release/notes/`, the renumbering record and the frozen ledger to `docs/release/migration/`, and removed `docs/releases/` with no stub.
- [x] Step 4 — repointed `pre-release.yml`, `pre-release-series-migration.yml` and `pre-release-migration-cli.test.mjs`.
- [x] Step 5 — updated `docs/README.md`, `docs/architecture/repository-map.md`, `AGENTS.md`, `docs/status/platform-handoff.md`, `docs/validation/windows-validation-history.md` and `docs/development/branch-integration-release-plan.md`.
- [x] Step 6 — validation done.

## 10. Execution notes

Two decisions are worth recording because they differ from the assumptions made while planning.

1. **No note file was deleted.** The plan allowed deleting duplicates. Inspection showed the `preN` files are not duplicates of their `pre.N` sources: each maps to a different source tag and carries its own asset hashes and release IDs. The `pre.9`, `pre.11`, `pre.12`, `pre.14` files additionally record rehearsals that produced no assets. All 33 files were kept; only the retired-series ones are labelled as historical in the index.
2. **`docs/releases/` was removed without a redirect**, so every in-repository reference had to be repointed. A first `sed` pass mis-ordered the two path rules and produced `docs/release/notes/migration/...` for the ledger in the migration workflow. It was detected by grep and corrected to `docs/release/migration/...` before any commit.

## 11. Validation performed

- Link check over the moved and new documents: no broken relative link (`EXIT=0`). One link in the renumbering record was found broken by this check and fixed.
- `npm run test --workspace desktop`: **179/179 PASS**, including `frozen ledger covers both retired series with unique target tags`, which resolves the ledger through its new path.
- `git diff --check`: clean.
- Grep for `docs/releases`: no remaining reference in `docs/`, `.github/` or `desktop/`.
- Git records the 35 moved files as renames, so history stays traceable.

Not run: the full Rust and Python suites, and any real release or Windows build. This change touches documentation, two workflow paths and one test fixture; no runtime code, dependency or lock file changed.
### 4.1 `release-policy.md`

- Authority: general policy, actual CI behavior, batch scope and historical evidence each stay in their own file. A policy that contradicts a workflow is recorded as a difference, not assumed to be implemented.
- Stages: development build, pre-release, release candidate, release. `v0.x.y` without a pre-release suffix is not automatically production ready.
- Candidate admission: scope frozen, candidate pinned to a full commit SHA, no uncommitted source changes, handoff traceable, risk-matched regression executed, advertised scope backed by acceptance evidence, every FAIL/BLOCKED/NOT_RUN item carrying reason, impact, owner and follow-up, and a security disposition.
- Separate a frozen source candidate from a verified candidate artifact; asset completeness cannot be claimed before the artifacts exist.
- Gates in three layers: non-waivable artifact integrity (non-empty assets, asset contract, tag/source parity, manifest and SHA-256 agreement); acceptance gates tied to the advertised scope; deferrable platform validation that follows `../validation/validation-policy.md`.
- Restricted development-version publication: record the authorizer, the pinned SHA, the deferred items, the risk, the reduced scope and the notes requirement. An authorization never rewrites a real result and never waives artifact integrity.
- Ownership: Linux owns shared source and integration, Windows owns native acceptance, packaging and platform evidence, the Release Agent executes an already authorized process.
- Numbering: the live series is `vMAJOR.MINOR.PATCH-preN`; `pre.N` is read-only historical evidence. Existing tags are never rewritten.
- Candidate naming: this refactor defines "release candidate" as a state. It does not introduce an `-rcN` tag, because no workflow accepts that spelling yet.

### 4.2 `release-checklist.md`

Phases: release identity, scope freeze, source verification, platform acceptance, artifact verification, release decision, post-release review, documentation closeout.

Each item records `state | evidence | owner | blocker or skip reason | follow-up`, using PASS, FAIL, BLOCKED, NOT_RUN or NOT_APPLICABLE.

Channel check: a pre-release defaults to Debug and a release to Info, a user override always wins, and the channel is never inferred from the optimized build profile.

The template itself never accumulates per-release history; an execution record is bound to its version.

### 4.3 `release-history.md`

Columns: version/tag, date, source SHA, channel and maturity, artifact state, acceptance summary, evidence link.

Rules: a local notes file does not mean a GitHub Release exists; unverifiable fields are marked unverified; renumbered entries map to their source tag instead of counting as a second release; build success, artifact completeness, WDIO result and product acceptance are recorded separately.

## 5. Migration and cleanup rules

1. Files move first; content is reviewed afterwards.
2. A file is deleted only when its unique facts were merged into `release-history.md`, the renumbering record or the notes that replace it.
3. `pre.N` and `preN` files are not assumed to be duplicates of each other. If a retired-series file holds unique failure, build, acceptance or authorization evidence, it is kept and labelled as historical.
4. The frozen ledger is consumed by a live workflow and by tests, so its data is preserved; only its path changes.
5. `docs/releases/` is removed with no compatibility stub.