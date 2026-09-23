# Current Platform Handoff

This file records the current WebView2/readiness batch. Historical Windows
results remain in `docs/development/windows-validation.md`,
`docs/validation/windows-validation-history.md`, and
`docs/validation/windows-queue.md`.

## Batch

- Task: WebView2 blank-document readiness gate recovery, followed by current-source Full package build and component validation.
- Branch: `windows/webview2-readiness-gate`
- Current owner: Windows Platform Owner for the Full package follow-up; the
  carried-forward shared-source and WDIO patch review items remain assigned to
  Linux Cross-platform Owner.
- Current state: `WINDOWS_PASS` for local WebView2 ordinary readiness and Full
  package assembly/component probes; `BLOCKED` for real Extension-to-Desktop
  Named Pipe integration and GUI validation. The Windows Desktop Named Pipe
  server is absent from current source; Computer Use also failed to initialize
  after limited retry. `CROSS_PLATFORM_REVIEW_REQUIRED` and the earlier
  `CROSS_PLATFORM_CHANGE_REQUIRED` items remain open.
- Scope source: the current batch adds Full package assembly and Windows
  Sidecar/Extension/Native Host validation to the WebView2/readiness work. No
  standalone Plan file was found in the repository; this handoff is the current
  scoped plan/state record.

## Revisions

- Cross-platform input / handoff revision: `59c8221` (prior handoff)
- Windows input revision: `35748f41fe3c01954cadb150c7aa33edc3aa4152`
- Windows implementation revision: `8a1714fe63231859e138e7a09d2e84e413e8cde0`
- Windows validation revision: `8a1714fe63231859e138e7a09d2e84e413e8cde0`
  (this commit contains the exact two implementation files exercised by the
  user's 3/3 local E2E run and the corresponding validation records).
- Handoff documentation is finalized in the next commit on this branch.
- Full package source revision: `75d8c2ca1d59f14fcf83a9aef8e36c1990ee7144`
  (current HEAD; no implementation files changed for package validation).
- Full package validation revision: `75d8c2ca1d59f14fcf83a9aef8e36c1990ee7144`
  (the binaries and package were built from this source; results and local
  package path are recorded in `docs/validation/windows-validation-history.md`
  and `docs/validation/windows-queue.md`).

## Windows Work Completed

- Fixed WDIO native driver launching on Windows when executable paths contain
  spaces: the idempotent pretest patch changes `@wdio/native-core` direct
  executable spawn from `shell: true` to `shell: false`.
- Extended `desktop/wdio.conf.mjs` to use explicit tauri-driver and EdgeDriver
  paths, add the EdgeDriver directory to PATH, select a fixed WebView2 runtime,
  and provide Windows `webviewOptions: {}`.
- Retained the existing EdgeDriver version-banner compatibility patch.
- User ran `npm run test:e2e:windows --workspace desktop`: session URL was
  `http://tauri.localhost/`; all three Dashboard E2E assertions passed; user
  confirms the XArchive window appeared.

## Validation Results

- `PASS`: local WebView2 ordinary readiness, frontend startup contract, native
  Dashboard shell, and stable Dashboard regions (3/3).
- `PASS`: `node --check` for the two changed JavaScript modules and
  `git diff --check`.
- `FAIL` (historical, superseded for this harness/config): earlier matching
  WebView2/EdgeDriver runs stayed at `data:,`; retain those entries as history.
- `NOT_RUN`: advanced native E2E (`WDIO_ADVANCED=1`), hosted/release workflow,
  full regression, and packaged Sidecar/Extension/Native Host integration.
- The supplied screenshot shows runtime console diagnostics; the user's
  successful native XArchive report and 3/3 app-dashboard assertions establish
  the readiness result. No claim is made about separate native log collection
  or a timed 30-second standalone wait.
- Validation details: `docs/validation/windows-validation-history.md` and the
  latest reconciliation at the end of `docs/validation/windows-queue.md`.

## Cross-platform Follow-up

`CROSS_PLATFORM_REVIEW_REQUIRED`:

- Review the `@wdio/native-core` patch in
  `desktop/scripts/patch-wdio-tauri-service.mjs`: confirm upstream dependency
  compatibility, whether the pretest patch remains necessary, and Linux/macOS
  behavior. On non-Windows, `shell: false` matches the prior conditional's
  behavior; Windows direct executable spawning is the validated case.

`CROSS_PLATFORM_CHANGE_REQUIRED` (carried forward from the prior handoff):

- Decide whether `crates/xarchive-protocol/src/sidecar.rs`, both
  `shared/protocol-schema/*.schema.json` files, both JSONL fixtures, and
  `desktop/src/main.js` are valid current sources or should be integrated,
  updated, or removed. They were carried forward at the user's request and
  were not part of this WebView2 validation.

## Still Open

- Full package current-source assembly: `PASS`; local output is
  `validation-artifacts/portable-full-75d8c2c-r3/`. Worker v2 protocol,
  Extension identity, and Native Host stdio framing probes passed.
- Full package GUI / Sidecar E2E: `FAIL` at WDIO session creation because Edge
  disconnected from DevTools before the spec ran. The same failure reproduced
  with the source exe baseline; this is not a package-specific result.
- Browser Extension load and real Native Messaging: `BLOCKED` —
  `COMPUTER_USE_UNAVAILABLE` after limited retry. Windows Desktop Named Pipe
  server is also not implemented; Native Host returns `NATIVE_PIPE_UNAVAILABLE`.
  Keep `WQ-MAN-PORTABLE-RUNTIME-01` open with its Manual Windows Validation Queue
  steps; component-level PASS does not establish end-to-end integration.
- `NOT_RUN`: hosted readiness/release gate; the local E2E result does not
  establish hosted runner behavior.
- No full regression was run because the diff is limited to the Windows WDIO
  launch/configuration path.
- Local dependencies, logs, validation artifacts, and other untracked local
  data remain in the Windows working tree and must not be included in commits.

## Next Owner

Owner: Windows Platform Owner for the remaining Windows Named Pipe endpoint,
GUI retry, and Full package Extension-to-Desktop integration. Linux
Cross-platform Owner retains the independent shared-source and WDIO
`CROSS_PLATFORM_REVIEW_REQUIRED` follow-ups listed below.

Linux Cross-platform Owner follow-ups:

1. Review the shared WDIO native-core spawn patch and run the relevant Linux
   test-infrastructure check if needed.
2. Decide the carried-forward shared-source follow-ups listed above.
3. Keep the already-passing Windows local readiness result tied to its recorded
   validation revision; do not reopen the old blank-target failure without a
   changed relevant dependency, source, or environment.
4. Treat hosted readiness and Full Sidecar/Extension integration as separate
   `NOT_RUN` items until their own validation evidence exists.
