# Current Platform Handoff

This file records the current WebView2/readiness batch. Historical Windows
results remain in `docs/development/windows-validation.md`,
`docs/validation/windows-validation-history.md`, and
`docs/validation/windows-queue.md`.

## Batch

- Task: WebView2 blank-document readiness gate investigation and Windows launch fix
- Branch: `windows/webview2-readiness-gate`
- Current owner: Linux Cross-platform Owner (Windows implementation and local
  validation are committed; this handoff is ready to push).
- Current state: `WINDOWS_PASS` for local WebView2 ordinary readiness;
  `CROSS_PLATFORM_REVIEW_REQUIRED`; `CROSS_PLATFORM_CHANGE_REQUIRED` for the
  carried-forward shared sources; Sidecar/Extension package integration remains `NOT_RUN`.
- Scope source: user request limits this batch to WebView2/readiness. No
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

- `NOT_RUN`: Full package Sidecar/Extension integration with a valid worker,
  Extension, and Native Host; see `WQ-MAN-PORTABLE-RUNTIME-01`.
- `NOT_RUN`: hosted readiness/release gate; the local E2E result does not
  establish hosted runner behavior.
- No full regression was run because the diff is limited to the Windows WDIO
  launch/configuration path.
- Local dependencies, logs, validation artifacts, and other untracked local
  data remain in the Windows working tree and must not be included in commits.

## Next Owner

Owner: Linux Cross-platform Owner, after Windows commit and push.

Required actions:

1. Review the shared WDIO native-core spawn patch and run the relevant Linux
   test-infrastructure check if needed.
2. Decide the carried-forward shared-source follow-ups listed above.
3. Keep the already-passing Windows local readiness result tied to its recorded
   validation revision; do not reopen the old blank-target failure without a
   changed relevant dependency, source, or environment.
4. Treat hosted readiness and Full Sidecar/Extension integration as separate
   `NOT_RUN` items until their own validation evidence exists.
