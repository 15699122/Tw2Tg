# Desktop Logs, Release Log Policy and Application Icon Fix Plan

Owner: Linux Cross-platform Owner (shared logging contract, shared configuration, shared frontend styling, shared icon assets), Windows Platform Owner (Windows GUI acceptance and Windows artifact inspection).
Status: `BATCH_A_DELIVERED` — the Linux-side implementation landed on `dev` at `279d726`. This is an implementation record, not validation evidence: no Windows item in `docs/validation/windows-queue.md` has been executed for this batch, and the screenshots remain defect evidence for the shipped build only.

## 1. Objective

Close four defects reported against the current release version.

1. The runtime log panel sits too close to the page description above it.
2. The "自动跟随" checkbox renders its checked state in blue, which does not match this project's theme.
3. The default log level is tied to the Rust build profile rather than the release channel, so an optimized pre-release still defaults to `info` instead of `debug`.
4. `xarchive-desktop.exe` shows a plain green block because the bundled icon assets are placeholders.

The outcome is a page-scoped spacing fix, a themed checkbox, a channel-driven log default that reaches the application modules, and a real multi-size application icon.

## 2. Evidence and confirmed causes

Everything below was checked in this repository, not assumed.

### 2.1 Log panel spacing

- `desktop/src/pages/logs-page.jsx` renders `PageHeader`, then the optional `Alert`, then `<Card className="logs-panel">`.
- `desktop/src/style.css` defines `.logs-toolbar`, `.log-viewport`, and `.logs-bottom-button`, but no rule that places space between the page description and the log panel. `.log-viewport` has a `margin-top`, which is the space between the toolbar and the log body, not between the description and the panel.
- Consequence: the panel butts against the description. Any fix must be page-scoped, because the same description-to-card spacing is already correct on Settings and Dashboard.

### 2.2 Checkbox theme

- `.logs-toolbar select, .logs-toolbar input` applies `min-height: 36px`, `border`, `padding: 6px 10px`, and `background: #fff` to every input in the toolbar, including the checkbox.
- `.logs-follow input { width: 15px; min-height: 15px; }` narrows the box but does not reset the border, padding, or appearance.
- No rule anywhere in `desktop/src/style.css` sets a checked-state color, so the checkbox keeps the WebView2 default blue accent. The project theme accent is `--success: #087f45`, and the brand mark uses `#2e6f59`.

### 2.3 Release log level

`desktop/src-tauri/src/config.rs`:

```rust
#[cfg(debug_assertions)]
fn default_log_level() -> LogLevel { LogLevel::Debug }

#[cfg(not(debug_assertions))]
fn default_log_level() -> LogLevel { LogLevel::Info }
```

This keys the default on the Rust build profile, not on the release channel.

- `windows-release.yml` builds with `npm run build:tauri --workspace desktop`, which produces an optimized binary, so `debug_assertions` is off for every release, pre-release or not.
- Consequence: a pre-release built today still defaults to `info`. The requested behavior cannot be expressed by the current mechanism.
- The tag is already validated in `windows-release.yml` with `^v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$`, and `pre-release.yml` only creates tags matching `vMAJOR.MINOR.PATCH-preN`, so the channel is derivable from the validated tag without a new input.

The existing unit test `defaults_to_info_and_five_log_files` asserts the level through the same `cfg`, so it passes in both profiles and therefore proves nothing about the channel.

### 2.4 A distinction that must not be blurred

`desktop/src/pages/logs-page.jsx` initializes `const [level, setLevel] = useState("info")`. That is a client-side display filter over lines already written to the log file. It is not the backend level. Changing only the dropdown would satisfy nothing in the request and would misrepresent the state.

### 2.5 Application icon

`file desktop/src-tauri/icons/*` reports:

```text
icon.ico: MS Windows icon resource - 1 icon, 1x1 with PNG image data, 1 x 1, 8-bit/color RGBA
icon.png: PNG image data, 1 x 1, 8-bit/color RGBA

## 3. Scope

### 3.1 Log page spacing (frontend, shared)

- Add a logs-page scope class and a page-scoped rule placing 24px between the page header block and the log panel.
- Keep the spacing correct when the read-error `Alert` is present.
- Do not change global header or card spacing, so Dashboard and Settings must not move.

### 3.2 Themed checkbox (frontend, shared)

- Give the checkbox its own rule that resets the inherited toolbar input styling: fixed size, no text padding, transparent background, theme border.
- Set the checked state to the project green through `accent-color`, and verify the visible checked color against the rendered result.
- Preserve native checkbox semantics, keyboard operation, and a visible focus ring.
- Do not introduce a new UI dependency.

### 3.3 Channel-driven default log level (shared configuration, shared build)

- Replace the `cfg`-based default with a build-time channel: `dev` defaults to `debug`, `prerelease` to `debug`, `release` to `info`.
- `windows-release.yml` derives the channel from the already validated `RELEASE_TAG` and passes it to the build **before** the executable is produced.
- An unknown or empty channel fails the build in CI instead of silently defaulting to a release.
- The build records the channel into the binary, so runtime behavior does not depend on a CI-only environment variable.
- Startup logs the channel, the channel default, and the effective level.
- Precedence is: explicit user selection, then the channel default. A user who selected `error` must keep `error`.
- Because an existing configuration file already stores a level, the "follow the channel default" state has to be representable; a stored `info` from an old install must not silently become `debug` on upgrade. The migration rule is defined and tested in the implementation.

### 3.4 Module debug coverage

Raising the file level is necessary but not sufficient. Coverage is required in the application diagnostic path for:

- Desktop startup, configuration application, runtime state transitions, and stage timings.
- Task and database lifecycle, results, and useful counts.
- Sidecar and gallery-dl startup, extraction stages, and error context.
- aria2 startup, RPC outcomes, download transitions, and retries.
- Telegram upload stages, retries, response status, and timings.
- Network proxy mode, resolution status, and redacted routing diagnostics.

Constraints:

- Reuse the existing logging mechanism in `desktop/src-tauri/src/logging.rs`. Do not introduce a new logging framework that has not been audited.
- In a `release` build, `info` keeps normal progress, warnings, and errors, and filters debug detail.
- Debug must not write unbounded child-process output into the log.
- Existing redaction, rotation, and size caps stay in force. Never log tokens, cookies, proxy passwords, authorization headers, or full sensitive request bodies.
- Child-process diagnostics must not be mixed into the Sidecar protocol stdout stream.
- The log page initializes its filter from the effective level on first load and afterwards respects the user's choice.
- "Modules log debug detail" means the application diagnostic path has real coverage. It does not mean enabling every third-party library's raw network output.

### 3.5 Application icon

- Maintainable SVG source, a PNG, and a multi-size ICO containing at least 16, 32, 48, and 256 pixels, adding 24, 64, and 128 for scaling quality.
- Visual direction: a green rounded square with a white simplified archive-box mark and an `X`, transparent outer edge, no fine text, legible at 16 pixels.
- Declare the icon resources in the Tauri configuration.
- Verify the directly built executable, not only the inactive bundle configuration.

### 3.6 Explicitly out of scope

- Publishing a release, changing an existing GitHub Release, or closing any `v0.2.0` acceptance gate.
- Enabling the Tauri bundler.
- Adding a user-facing runtime log-level switcher beyond what already exists.

```

Both assets are 1x1 placeholders. A 1x1 resource scales to a flat block, which is exactly the reported green rectangle. The green is what Windows shows when a resource cannot supply a real image, not a design choice.

- `desktop/src-tauri/tauri.conf.json` sets `"bundle": { "active": false }`, so there is no installer. The Windows resource comes from the directly built `xarchive-desktop.exe`, and the fix must be verified on that binary rather than on a bundle.

## 4. Ownership routing

Items 1, 2, and the shared parts of 3 and 4 are cross-platform and belong to the Linux owner. The release workflow change is also a shared repository concern.

The following need the Windows Platform Owner and cannot be validated on Linux:

- GUI appearance at 100%, 125%, and 150% scaling.
- The real optimized pre-release and release binaries and their actual log output.
- Real child-process diagnostics for gallery-dl and aria2.
- The icon in Explorer, the title bar, and the task bar.

The icon artwork itself is shared, but its appearance on Windows is a Windows acceptance item. No item in this batch changes a shared contract in a way that requires a separate cross-platform decision; if the channel mechanism turns out to need a new shared contract, mark `CROSS_PLATFORM_CHANGE_REQUIRED` rather than deciding it silently.

## 5. Validation plan

Linux, escalating `Targeted -> Module -> Subsystem`:

- Channel default resolution, including an unknown channel failing explicitly.
- User override precedence and the configuration migration rule.
- Debug writes, `info` filtering debug lines, warnings and errors preserved.
- Module level propagation, and the Sidecar protocol stream staying clean.
- Redaction, rotation, and size caps still holding under debug volume.
- Frontend build and tests, Rust formatting, and the affected Rust test targets.

Windows only, recorded in `docs/validation/windows-queue.md` as `NOT RUN`:

- `WQ-LOGS-020-01` panel spacing and `WQ-LOGS-020-02` checkbox appearance at three scaling factors.
- `WQ-LOGS-020-03` pre-release binary defaults to debug, and every module produces detailed output.
- `WQ-LOGS-020-04` release binary defaults to info and hides debug detail while normal logs remain.
- `WQ-LOGS-020-05` icon in the executable, Explorer, title bar, and task bar, using a clean directory so old shortcuts and the icon cache do not mislead.

A blocked GUI item is recorded as `BLOCKED` with the blocker and manual steps. It is never recorded as PASS.

## 6. Handoff

The implementation lands on `dev` through a normal Git commit and push. Until that commit exists, the working tree is not a handoff and must not be described as one.

## 7. Completion criteria

1. The log panel has deliberate spacing from the description, with no Dashboard or Settings regression.
2. The "自动跟随" checked state uses the project theme and keeps native keyboard and focus behavior.
3. A pre-release binary defaults to `debug` and a release binary defaults to `info`, driven by the validated release channel and recorded in the binary.
4. An explicit user level overrides the channel default and survives a restart.
5. Debug output actually covers the modules listed in 3.4, with redaction, rotation, and size caps intact, and the Sidecar protocol stream unaffected.
6. `icon.ico` is a genuine multi-size resource and the built executable shows the new icon on Windows.
7. Every Windows item is recorded as executed with evidence, or as `BLOCKED` with manual steps. None is claimed as PASS without execution.

## 8. What the implementation actually changed

Recorded here so the plan and the code do not drift apart.

### 8.1 Log page spacing and checkbox

`desktop/src/pages/logs-page.jsx` adds `logs-page-panel` next to the existing `logs-panel`. `desktop/src/style.css` adds `.logs-page-panel { margin-top: 24px; }`, which is scoped to the log page so Dashboard and Settings spacing is untouched.

The checkbox gets its own `input[type="checkbox"]` rule that resets the toolbar input styling with `appearance: none`, zero padding, a theme border, and a fixed 15px box. The checked state uses `--success` with a white check drawn from an inline SVG, because an inset white shadow read as a filled square at small sizes rather than as a check. `accent-color` was rejected: WebView2 renders it inconsistently and the project already themes every control explicitly.

### 8.2 Channel-driven default level

`desktop/src-tauri/src/build_channel.rs` is new. `LoggingConfig.level` changed from `LogLevel` to `Option<LogLevel>`, and `effective_level()` is the single accessor the runtime, the status command, and the log page now use. `None` means "follow the channel"; `Some` is an explicit user choice that survives an upgrade, which is what keeps an existing installation from being silently rewritten to `debug`.

`desktop/scripts/build-tauri.mjs` is new and `build:tauri` now routes through it, because `option_env!` is read at compile time and the value has to be present while the crate compiles. `windows-release.yml` exports the already validated `RELEASE_TAG` before the build step.

Two design points that were corrected during implementation:

- `release_channel` originally fell back to `Dev` for an unrecognized value. That quietly ships a verbose binary under a stable release name, so it now panics with the parse error. The build script rejects the same value first, so this is defense in depth.
- `release_channel` cannot be a `const fn`: matching on the `str` from `option_env!` is not allowed in a constant context. It is an ordinary function.

### 8.3 Module debug coverage

`RuntimeState::record` plus `debug`/`warn`/`error` helpers in `runtime.rs` are the single write path, so the level filter, secret redaction, and the rotation and size caps apply uniformly. Coverage was added for runtime paths and database readiness, configuration errors, network mode and redacted summary with timeouts, transport startup, executor startup recovery, Sidecar launch arguments and environment, proxy setting application, archive directory changes, and logging setting changes.

Two constraints shaped this:

- The Sidecar line records argument names and the environment variable *count*, not values. A proxy value can carry credentials.
- `config_error` is consumed while building `database_error`, so a diagnostic copy is taken first. The same applies to `database_path` and `cache_root`, which move into the struct.

`detect_aria2` takes only an `AppHandle` and has no runtime state, so aria2 detection is not yet covered by the helper. That remains a gap rather than being papered over with a global logger.

### 8.4 Icon

`desktop/scripts/make-icon.py` describes the artwork as primitives and renders every size from that one source with supersampling. The first attempt added a darker band behind the box; at 16px the band edge crossed the box and read as two separate color blocks, so the plate is now one flat green. `icon.ico` holds 16/24/32/48/64/128/256, and `tauri.conf.json` declares the resources while `bundle.active` stays `false`.

- `desktop/src-tauri/build.rs` performs no icon handling; Tauri supplies the Windows resource through the icon configuration.

## 9. Windows validation follow-up (2026-10-01)

Source 6be3269, exact evidence in windows-validation-history.md. Optimized channel debug/info defaults work; Logs page still initializes Info despite backend Debug, so WQ-LOGS-020-02 FAIL / CROSS_PLATFORM_CHANGE_REQUIRED. Cross-platform Owner must bind initial display filter to effective logging level and add tests. Native current-size spacing/green checkbox/title icon and pre-release Error save/restart subchecks observed; multi-DPI, all icon surfaces and complete override/module diagnostics matrix remain open. No release approval.
