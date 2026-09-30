# Desktop UI Polish and Known Folder Remediation Plan

Owner: Linux Cross-platform Owner (shared CSS, shared call sites, cross-platform checks), Windows Platform Owner (Windows known-folder behavior and Windows GUI acceptance).
Status: `IMPLEMENTED_ON_LINUX` — plan recorded 2026-09-30 against `dev` `bd40402d8d3a455911216e199233d66156d67657`; Linux implementation and Targeted/Module checks are complete, and Windows validation is queued as `WQ-UI-020-01`..`WQ-UI-020-04`. This document is a plan and is not validation evidence.

## 1. Objective

Fix four defects reported against the released `v0.2.0` Windows package:

1. The Dashboard `最近任务` and `运行环境` cards do not share a bottom edge.
2. The sidebar `服务状态` rows render with a default black border that does not match the design language.
3. The sidebar version/platform line (`v0.2.0 · windows`) is too close to the separator above and too far from the sidebar bottom edge.
4. `使用系统下载目录` resolves `%USERPROFILE%\Downloads` by string concatenation instead of querying the Windows known-folder location, so a redirected Downloads folder is ignored.

## 2. Confirmed causes

| # | Location | Cause |
|---|---|---|
| 1 | `desktop/src/style.css` (`.dashboard-grid`) | `align-items: start` keeps each card at its own content height. |
| 2 | `desktop/src/components/connection-status.jsx`, `desktop/src/style.css` | The rows are native `<button>` elements and `.connection-line-button` has no appearance reset, so the default border and background are painted. |
| 3 | `desktop/src/style.css` (`.version-label`, `.sidebar-footer-separator`, `.sidebar`) | `.version-label` uses `margin: -8px 8px 0`, pulling the text toward the separator, while the sidebar bottom padding stays at 20px. |
| 4 | `desktop/src-tauri/src/portable.rs` (`system_download_directory`) | The Windows branch reads `USERPROFILE` and appends `Downloads`. |


## 3. Scope

### 3.1 Dashboard card alignment

- Change the two-column Dashboard grid so the cards in a row stretch to equal height.
- Let the jobs card content area absorb the extra height so the empty and loading states stay balanced.
- Do not hardcode pixel heights, spacer elements, or JavaScript measurement.
- Keep the single-column narrow layout unstretched; equal height is only required for the two-column layout.
- Job growth must expand the card instead of clipping rows.

### 3.2 Sidebar service status styling

- Reset `.connection-line-button`: no border, transparent background, explicit padding, consistent radius, full-width row.
- Align the status dot, label, and trailing status text consistently across the three rows.
- Reuse existing neutral surfaces, text colors, and status colors; add no new visual system.
- Use a light hover background instead of a bordered look.
- Keep the existing `:focus-visible` outline so removing the border never removes keyboard focus indication.
- Preserve the click-to-settings behavior, `aria-label`, and Tab/Enter/Space activation.

### 3.3 Sidebar version line spacing

- Remove the negative top margin on `.version-label`.
- Rebalance the separator spacing, the version line height, and the sidebar bottom padding together instead of pushing only the text down.
- Keep the version and platform values sourced from runtime status; do not hardcode `v0.2.0 · windows`.
- Check short window heights so the footer does not compress or clip the navigation.

### 3.4 System Downloads resolution

Target behavior:

```text
current user Downloads known folder
    -> XArchive subdirectory
    -> persisted as the selected download directory
```

- Replace the Windows `USERPROFILE` + `Downloads` concatenation with the known-folder query through the Tauri path resolver.
- Keep the `XArchive` subdirectory product behavior.
- Do not use `KF_FLAG_DEFAULT_PATH`; the current registered location must be queried, not the default one.
- On query failure return a clear error; never silently fall back to a C: drive guess, a user-profile join, or the portable directory.
- Keep directory-creation failure diagnosable and do not mark setup complete.
- Do not migrate or overwrite an already persisted download directory; existing installs keep their configured path.
- Do not redesign the Linux/macOS directory strategy in this batch.

## 4. Ownership routing

- Linux Cross-platform Owner: shared CSS in `desktop/src/style.css`, shared test updates, shared call-site adjustments, Linux checks.
- Windows Platform Owner: Windows known-folder behavior, native build, and Windows GUI acceptance.
- If the resolver change turns out to require a shared API, protocol, or configuration contract change, mark `CROSS_PLATFORM_CHANGE_REQUIRED` and return to Linux.
- A small shared adjustment that preserves the existing abstraction is `CROSS_PLATFORM_REVIEW_REQUIRED`.

## 5. Validation plan

Escalation starts at Targeted and escalates only if the impact area requires it.

| Area | Content |
|---|---|
| Frontend targeted | Extend the existing UI wiring test so the service-button appearance reset, the removed negative footer margin, and the equal-height grid rule cannot silently regress. Static assertions are not visual acceptance. |
| Frontend build | `npm run check` in `desktop`. |
| Rust targeted | Download directory resolution, `XArchive` subdirectory join, failure handling, config persistence, and unchanged portable behavior. |
| Windows native | Default Downloads, redirected Downloads on another volume, non-ASCII and space-containing paths, and an unavailable or non-writable Downloads location. UNC paths are validated only when a suitable test share exists. |
| Windows GUI | Card bottom edges, service status appearance, and footer spacing at 100%/125%/150% scaling, plus the narrow-window and keyboard-focus cases. |

Key acceptance scenario: in an isolated test user with Downloads redirected to another volume, choosing `使用系统下载目录` creates and persists `<redirected Downloads>\XArchive` and does not create an archive directory under the original `%USERPROFILE%\Downloads`.

Redirection testing must use an isolated user or a virtual machine so the daily account is not modified. When GUI automation is unavailable, record `BLOCKED` with `COMPUTER_USE_UNAVAILABLE` plus a manual validation item; an unexecuted GUI check is never recorded as PASS. Full regression is not run for this change set because the diff is limited to sidebar/dashboard styling and one directory-resolution path; the reason is recorded in the round result.

## 6. Handoff

- Deliver this batch through Git, recording branch, source commit, implementation commit, validation commit, uncommitted-state status, and owner.
- Do not overwrite the Windows Owner's canonical working tree; direct sync is diagnostic only.
- Review the final `git diff` before finishing.
- This batch does not close any open `v0.2.0` acceptance gate, and the previously recorded `WINDOWS_BLOCKED` / `NOT RUN` items stay open unless new evidence closes them.

## 7. Completion criteria

- Dashboard cards in the two-column layout share a bottom edge within about 1 CSS pixel in all job-list states.
- No default black border on any sidebar service row; hover and keyboard focus remain visible.
- The version line sits visually centered between the separator and the sidebar bottom edge.
- The selected system download directory matches the Windows known-folder Downloads location plus `XArchive`, including when Downloads is redirected.
- Targeted checks and builds pass on Linux; Windows items are recorded with an explicit revision and result state.

