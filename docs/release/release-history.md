# Release History

Owner: Cross-platform Owner.
Status: `ACTIVE` — index of what exists for each version. Detailed per-version facts live in [`notes/`](notes/); the rules that produced them live in [`release-policy.md`](release-policy.md).

This file is an index, not a second copy of the release notes. It answers "does this version exist, what is its source, and where is its evidence".

## 1. How to read this index

- A row here means a **tag exists**. It does not by itself mean a GitHub Release was published with assets.
- `Published` reflects the release objects confirmed on GitHub at the time of writing. Anything not confirmed is written as `unverified` rather than inferred.
- A renumbered pre-release maps to its source tag. The retired `pre.N` row and the `preN` row are one publication event, not two.
- Build success, artifact completeness, automated UI result and product acceptance are separate columns and are never merged.
- Dates are the GitHub publication timestamps in UTC; the Windows validation record carries the run-level detail.

## 2. Published releases

Confirmed on GitHub for `15699122/Tw2Tg`.

| Version | Published (UTC) | Type | Source | Notes | Acceptance summary |
|---|---|---|---|---|---|
| `v0.2.0` | 2026-09-30 | release | `7910033` | [`notes/v0.2.0.md`](notes/v0.2.0.md) | Assets verified; GUI acceptance `WINDOWS_BLOCKED`; independent WDIO FAIL (environment class, non-blocking). Published under an explicit Owner authorization with a reduced scope. |
| `v0.2.0-pre7` | 2026-09-30 | pre-release | `1c72c2d` | [`notes/v0.2.0-pre7.md`](notes/v0.2.0-pre7.md) | Renumbered republish of `v0.2.0-pre.16`; 7 assets. Migration republish, not a new acceptance. |
| `v0.2.0-pre6` | 2026-09-30 | pre-release | `7abf69a` | [`notes/v0.2.0-pre6.md`](notes/v0.2.0-pre6.md) | Renumbered republish of `v0.2.0-pre.7`; 7 assets. |
| `v0.2.0-pre5` | 2026-09-30 | pre-release | `ac586e6` | [`notes/v0.2.0-pre5.md`](notes/v0.2.0-pre5.md) | Renumbered republish of `v0.2.0-pre.6`; 7 assets. |
| `v0.2.0-pre4` | 2026-09-30 | pre-release | `38e9a78` | [`notes/v0.2.0-pre4.md`](notes/v0.2.0-pre4.md) | Renumbered republish of `v0.2.0-pre.4`; 4 assets. |
| `v0.2.0-pre3` | 2026-09-30 | pre-release | `baf0b24` | [`notes/v0.2.0-pre3.md`](notes/v0.2.0-pre3.md) | Renumbered republish of `v0.2.0-pre.3`; 4 assets. |
| `v0.2.0-pre2` | 2026-09-30 | pre-release | `f2ae58d` | [`notes/v0.2.0-pre2.md`](notes/v0.2.0-pre2.md) | Renumbered republish of `v0.2.0-pre.2`; 4 assets. |
| `v0.2.0-pre1` | 2026-09-30 | pre-release | `0105ce9` | [`notes/v0.2.0-pre1.md`](notes/v0.2.0-pre1.md) | Renumbered republish of `v0.2.0-pre.1`; 2 assets. |
| `v0.1.1` | 2026-09-19 | release | unverified | [`notes/v0.1.1.md`](notes/v0.1.1.md) | Acceptance state per its own notes; not re-derived here. |
| `v0.1.1-pre3` | 2026-09-30 | pre-release | `de61eaa` | [`notes/v0.1.1-pre3.md`](notes/v0.1.1-pre3.md) | Renumbered republish of `v0.1.1-pre.3`; 2 assets. |
| `v0.1.1-pre2` | 2026-09-30 | pre-release | `a5f42cc` | [`notes/v0.1.1-pre2.md`](notes/v0.1.1-pre2.md) | Renumbered republish of `v0.1.1-pre.2`; 2 assets. |
| `v0.1.1-pre1` | 2026-09-30 | pre-release | `5afc1b8` | [`notes/v0.1.1-pre1.md`](notes/v0.1.1-pre1.md) | Renumbered republish of `v0.1.1-pre.1`; 2 assets. |
| `v0.1.0` | 2026-09-16 | release | unverified | [`notes/v0.1.0.md`](notes/v0.1.0.md) | First published release; acceptance state per its own notes. |

## 3. Retired `pre.N` series

These tags are retained unchanged as historical evidence and are never republished. Their release objects were retired after the replacement `preN` release was verified, so old download URLs intentionally stopped working. Full mapping, run IDs and release IDs are in [`migration/pre-release-renumbering.md`](migration/pre-release-renumbering.md) and [`../validation/windows-validation-history.md`](../validation/windows-validation-history.md).

| Retired tag | Replacement | Published today | Notes |
|---|---|---|---|
| `v0.1.1-pre.1` | `v0.1.1-pre1` | No — object retired | [`notes/v0.1.1-pre.1.md`](notes/v0.1.1-pre.1.md) |
| `v0.1.1-pre.2` | `v0.1.1-pre2` | No — object retired | [`notes/v0.1.1-pre.2.md`](notes/v0.1.1-pre.2.md) |
| `v0.1.1-pre.3` | `v0.1.1-pre3` | No — object retired | [`notes/v0.1.1-pre.3.md`](notes/v0.1.1-pre.3.md) |
| `v0.2.0-pre.1` | `v0.2.0-pre1` | No — object retired | [`notes/v0.2.0-pre.1.md`](notes/v0.2.0-pre.1.md) |
| `v0.2.0-pre.2` | `v0.2.0-pre2` | No — object retired | [`notes/v0.2.0-pre.2.md`](notes/v0.2.0-pre.2.md) |
| `v0.2.0-pre.3` | `v0.2.0-pre3` | No — object retired | [`notes/v0.2.0-pre.3.md`](notes/v0.2.0-pre.3.md) |
| `v0.2.0-pre.4` | `v0.2.0-pre4` | No — object retired | [`notes/v0.2.0-pre.4.md`](notes/v0.2.0-pre.4.md) |
| `v0.2.0-pre.6` | `v0.2.0-pre5` | No — object retired | [`notes/v0.2.0-pre.6.md`](notes/v0.2.0-pre.6.md) |
| `v0.2.0-pre.7` | `v0.2.0-pre6` | No — object retired | [`notes/v0.2.0-pre.7.md`](notes/v0.2.0-pre.7.md) |
| `v0.2.0-pre.16` | `v0.2.0-pre7` | No — object retired | [`notes/v0.2.0-pre.16.md`](notes/v0.2.0-pre.16.md) |
## 4. Tags with notes but no confirmed release object

These tags exist in Git and have release notes recording pipeline rehearsals or failures. A notes file is not evidence that a release was published, and several of these explicitly produced **no assets**. They are kept because they record unique failure and diagnosis evidence.

| Tag | Recorded outcome | Notes |
|---|---|---|
| `v0.1.1-pre.4` | Feature pre-release; no release object confirmed at the time of writing | [`notes/v0.1.1-pre.4.md`](notes/v0.1.1-pre.4.md) |
| `v0.2.0-pre.8` | Startup diagnostics and readiness-gate pre-release; no release object confirmed | [`notes/v0.2.0-pre.8.md`](notes/v0.2.0-pre.8.md) |
| `v0.2.0-pre.9` | Pipeline defect, **no assets produced**; superseded by `pre.10` | [`notes/v0.2.0-pre.9.md`](notes/v0.2.0-pre.9.md) |
| `v0.2.0-pre.10` | Driver probe fix and gate diagnostics pre-release; no release object confirmed | [`notes/v0.2.0-pre.10.md`](notes/v0.2.0-pre.10.md) |
| `v0.2.0-pre.11` | Rehearsal, **no assets produced**; superseded by `pre.12` | [`notes/v0.2.0-pre.11.md`](notes/v0.2.0-pre.11.md) |
| `v0.2.0-pre.12` | Rehearsal, **no assets produced**; superseded by `pre.13` | [`notes/v0.2.0-pre.12.md`](notes/v0.2.0-pre.12.md) |
| `v0.2.0-pre.13` | Rehearsal; Windows readiness gate not executed at the time of writing | [`notes/v0.2.0-pre.13.md`](notes/v0.2.0-pre.13.md) |
| `v0.2.0-pre.14` | Rehearsal, **no artifact produced** | [`notes/v0.2.0-pre.14.md`](notes/v0.2.0-pre.14.md) |
| `v0.2.0-pre.15` | Rehearsal of the Windows publish chain; no release object confirmed | [`notes/v0.2.0-pre.15.md`](notes/v0.2.0-pre.15.md) |

## 5. Known evidence gaps

- `v0.1.0` and `v0.1.1` source SHAs were not re-derived here and are marked `unverified` rather than inferred.
- The confirmation above lists release objects only. Download URLs and current asset availability were not re-checked in this pass.
- Detailed run-level evidence, including the failures recorded in the `pre.N` rehearsals, stays in [`../validation/windows-validation-history.md`](../validation/windows-validation-history.md) and the current [`../validation/windows-queue.md`](../validation/windows-queue.md).

## 6. Adding an entry

1. Add or update `notes/<tag>.md` first, with the real outcome and the unverified scope.
2. Add the row here with the confirmed publication state, the source SHA and the evidence link.
3. Record renumbering under section 3 and in the migration document; never count it as a second release.
4. Record a restricted publication with its authorization; the entry must show the reduced advertised scope.