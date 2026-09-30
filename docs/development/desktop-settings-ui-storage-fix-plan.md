# Desktop Settings Layout and Archive Directory Plan

Owner: Linux Cross-platform Owner (shared CSS, shared settings page, shared command contract, cross-platform checks), Windows Platform Owner (Windows GUI acceptance, Windows directory behavior).
Status: `IMPLEMENTED_ON_LINUX` — plan recorded 2026-09-30 against `dev` `f1456d5b0af224820f9837f0c20aefccde997aac`; implemented in handoff revision `3dd92d8` on `dev`. Linux implementation and Targeted/Module checks are complete; Windows validation is queued as `WQ-SET-020-01`..`WQ-SET-020-06`. This document is a plan, not validation evidence.

## 1. Objective

Fix seven defects reported against the released `v0.2.0` Windows Settings page, confirmed from Windows screenshots of the shipped build:

1. No separator between the page description and the `Core Bootstrap` heading.
2. `gallery-dl.exe` and its full path are too close inside the copyable-path block.
3. The `aria2` section icon is blue and does not match the neutral icons elsewhere.
4. The aria2 help note is too close to the buttons above and too far from the divider below.
5. `Desktop 观察` and its value `not_loaded` are too far apart.
6. The three Extension action buttons are too close, and their heights are inconsistent and not aligned.
7. `存储位置` offers no way to change the archive directory.

The screenshots are evidence of the released defects only. They are **not** acceptance evidence for the fix; a post-fix Windows capture is required.

## 2. Confirmed causes

| # | Location | Cause |
|---|---|---|
| 1 | `desktop/src/style.css` (`.settings-section:first-child`) | The rule removed the top border (`border-top: 0`) and reduced `padding-top` to 8px, so the first section had neither a separator nor breathing room under the page description. |
| 2 | `desktop/src/style.css` (`.copyable-path-text`) | The two-line grid had no `gap`, so the filename and the path were set on adjacent lines. |
| 3 | `desktop/src/style.css` (`.aria2-icon`) | The rule used the accent foreground and a tinted surface, unlike `.component-icon` for the Sidecar and Extension sections. |
| 4 | `desktop/src/style.css` (`.aria2-help`, `.settings-section`) | The note had no top margin, and the section bottom padding was the full 24px, which pushed the divider further away than the buttons. |
| 5 | `desktop/src/style.css` (`.extension-websocket-status > div`) | The grid items stretched to the tallest cell in the row, so the short `Desktop 观察` field spread its label and value apart when the neighboring `连接阶段` cell wrapped to multiple lines. |
| 6 | `desktop/src/style.css` (`.extension-actions`) | The container had no gap rule and no height rule, so buttons were laid out inline with default button metrics and per-button padding differences produced uneven heights. |
| 7 | `desktop/src/pages/settings-page.jsx`, `desktop/src/main.jsx`, `desktop/src-tauri/src/commands.rs` | The storage section only exposed `open_archive_folder`; no command existed to persist a new archive root. |

## 3. Scope

### 3.1 Core Bootstrap separator and spacing

- Keep the top border on the first settings section and give it the same 24px top padding as the other sections.
- Do not add a new divider component or a hardcoded height; reuse the existing `.settings-section` border token.
- Confirm the separator is not duplicated at the top of the page.

### 3.2 Copyable-path line spacing

- Add a 4px grid gap between the filename and the path line.
- The rule is shared, so verify every `CopyablePath` instance (gallery-dl, aria2, Extension directory, archive, logs) stays balanced.
- Keep the ellipsis/overflow behavior of the mono path line unchanged.

### 3.3 aria2 icon color

- Render the icon with the neutral foreground on the neutral subtle surface, matching the Sidecar and Extension section icons.
- Do not remove the `aria-hidden` attribute or the `component-icon` frame.

### 3.4 aria2 help note spacing

- Add top margin to the help note so it separates from the action row.
- Reduce the bottom padding of the aria2 section so the divider below is closer than the top gap.
- Keep the note text unchanged; this is a layout fix only.

### 3.5 Desktop 观察 label/value spacing

- Align the WebSocket status cells to the top of the row so each label sits directly above its own value regardless of neighbor height.
- Do not fix the row height or truncate the `连接阶段` text; wrapping is expected.

### 3.6 Extension action buttons

- Lay the three actions out in a flex row with a 10px gap, aligned center, wrapping when the window is narrow.
- Give the buttons a single shared height so all three are equal and vertically aligned.
- Keep each button's label, order, `disabled` state, and existing click handlers unchanged.

### 3.7 Archive directory selection

Target behavior:

```text
Settings -> 更改归档目录 -> native directory picker
    -> validate -> persist config -> rebuild executor -> refresh status
```

- Add a Tauri command that validates the picked directory, creates it if missing, persists it in the existing download configuration, and rebuilds the executor around the new archive root.
- Reuse the existing configuration persistence and `replace_executor` path so a later launch restores the same directory.
- Reject relative paths, the portable root itself, and an existing file at the target path.
- Cancelling the picker must not change any state or write any file.
- Scope is **only the archive commit target**: the staging root, database, cache, and log locations stay unchanged, and existing files are not migrated.
- Show a busy label while applying and a clear error message on failure; never report success for an unapplied change.
- The new command must be registered in the Tauri invoke handler; an unregistered command is a build-level omission, not a runtime detail.
- Keep the existing `打开归档文件夹` action.

## 4. Ownership routing

- Linux Cross-platform Owner: `desktop/src/style.css`, `desktop/src/pages/settings-page.jsx`, `desktop/src/main.jsx`, `desktop/src-tauri/src/commands.rs`, the Tauri command registration, shared tests, and Linux checks.
- Windows Platform Owner: Windows GUI acceptance, the native directory picker, and Windows path behavior.
- The new command reuses the existing configuration and executor abstractions. If Windows finds that a correct implementation requires a shared contract, protocol, or schema change, mark `CROSS_PLATFORM_CHANGE_REQUIRED` and return to Linux. A small shared implementation adjustment that preserves the abstraction is `CROSS_PLATFORM_REVIEW_REQUIRED`.
- This batch is cross-platform-owned because the command, its validation, and the configuration reuse are shared. It does not require a Windows-side code change.

## 5. Validation plan

Escalation starts at Targeted and escalates only if the impact area requires it.

| Area | Content |
|---|---|
| Frontend targeted | Extend `desktop/test/ui-wiring.test.mjs` so the first-section border, the copyable-path gap, the neutral aria2 icon, the help-note margin, the top-aligned status cells, the button gap/height, and the archive-directory wiring cannot silently regress. Static assertions are not visual acceptance. |
| Frontend build | `npm run check` in `desktop`. |
| Rust targeted | `validate_archive_directory` cases: absolute directory outside the portable root, relative path, portable root, existing file. The test must be valid on Linux and Windows. |
| Rust module | Full `cargo test -p xarchive-desktop` and `cargo fmt --check`, because a new command is registered in the invoke handler. |
| Windows native | Native directory picker behavior, Chinese/space-containing paths, a non-writable directory, and a target that is an existing file. |
| Windows GUI | The six layout items at 100%/125%/150% scaling and a narrow window, plus the archive-directory flow. |

Key acceptance scenario: in an isolated test user, changing the archive directory to an empty folder updates the displayed path, persists across a restart, sends subsequent archived files to the new folder, and leaves existing files, the database, and the logs where they were.

Redirection and permission testing must use an isolated user or a virtual machine. When GUI automation is unavailable, record `BLOCKED` with `COMPUTER_USE_UNAVAILABLE` and keep the item open; an unexecuted GUI check is never recorded as PASS. Full workspace regression was not run for this change set; the reason is recorded in the round result.

## 6. Handoff

- Commit and push this batch through Git before Windows validation, recording branch, source commit, handoff commit, uncommitted-state status, and owner.
- Done for this batch: implementation commit `3dd92d8` on `dev`, pushed to `origin/dev`, working tree clean at handoff. Windows validates that exact revision, not `f1456d5` plus local edits.
- If Windows returns a shared contract change, mark `CROSS_PLATFORM_CHANGE_REQUIRED` and route it back through this document rather than patching on the Windows branch.
- Do not overwrite the Windows Owner's canonical working tree; direct sync is diagnostic only.
- Review the final `git diff` before finishing.
- This batch closes no `v0.2.0` acceptance gate; the previously recorded `WINDOWS_BLOCKED` / `NOT RUN` items stay open.

## 7. Completion criteria

- A visible separator and balanced spacing separate the page description from `Core Bootstrap`.
- The filename and path lines inside every copyable-path block are visually separated.
- The aria2 icon uses the same neutral treatment as the other section icons.
- The aria2 help note sits closer to the divider below than to the buttons above.
- `Desktop 观察` sits directly above `not_loaded` when `连接阶段` wraps.
- The three Extension buttons are equally tall, aligned, separated by 10px, and wrap instead of overlapping in a narrow window.
- The archive directory can be changed from the Settings page, the change persists, and it affects only later archives.
- Targeted and Module checks pass on Linux; every Windows item is recorded with an explicit revision and result state.

