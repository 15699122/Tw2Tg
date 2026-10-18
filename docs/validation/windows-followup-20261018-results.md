# Windows follow-up return — 2026-10-18

## Identity and capability

- Received branch `codex/windows-f3-native-import-20261018`, remote/handoff exact
  `2f0f140558f1be57293886216298484c1902e9b7`. Initial local HEAD e7a75aa,
  tracked clean, no local-only commits; fetched remote then fast-forwarded.
- Current branch `codex/windows-f3-c1-followup-20261018`. Driver implementation
  `25c6ef6021b447d57224a6f720e7291559294d53`; final implementation/tests
  `9dc3762f46d4ced187a4ec2236dfdf7aa8734e8d`. No runtime Rust/frontend business
  change; Rust additions are Windows tests and an opt-in synthetic seed only.
  Handoff SHA is the documentation/evidence commit containing this return.
  Native ACL fixture isolation follow-up:
  `88c36af96e66ffaf6f74844548244f98a7c66fc6`; it clears inherited PSModulePath
  only in the test child so default Windows PowerShell can load its own ACL module.
  This changes no machine configuration or product behavior.
- Exact app from received handoff: Full source
  `dc731cce8a1b696af1dea00c3ad491b3e2ecc045`, local portable release build,
  Desktop SHA-256 `de957d15f1252496738062507447ab693fc7453b99c47584c79f5291ac821b1b`.
  ZIP SHA-256 `29a10fea730ae07d3cd987d9cbf4c7ca92a15c48a1b4ed16482d1469a38e35e8`.
  This round freshly runs this exact artifact; it does not inherit its old PASS.
- Native Windows11 Workstations Insider 10.0.29683 x64; repo E: ReFS, disposable
  profile/media/database C: NTFS. PowerShell7.6.7, Node24.20.0, Rust/Cargo1.98.0,
  Python3.12.14. WebView2/EdgeDriver155.0.4283.45, signed Microsoft driver SHA-256
  `dc99c208c10a5364e61c4e5b1341be492f3224de753adb23e973be97875b1892`.
  tauri-driver cargo installation2.1.0-alpha.0. Test-only ACL recipe passed using
  installed PowerShell7 and, after fixture isolation, default Windows PowerShell5.1.
  Initial inherited-module loading failure is retained as tooling evidence.
  No real accounts/credentials/downloads used.
- Runtime capabilities: native shell, NTFS ACLs, native Tauri/WebView2 WebDriver,
  filesystem/process inspection and Git remote work. Sandbox shell bootstrap remains
  unavailable; approved noninteractive native shell supplies evidence. This does not
  weaken assertions or transfer ownership.

Local evidence: `validation-artifacts/windows-followup-20261018/`. Committed
[receipt and hashes](evidence/windows-followup-20261018/receipt.json) bind individual
commands, actual executable hashes, source/dirty scope, logs and PNG screenshots.
The canonical evidence index hashes Git blobs; original local raw logs are retained.
All checks below: Windows Platform Owner, P1, blocks_development=false. Whole release
acceptance is not claimed; revalidate if relevant code/deps/platform/environment change.

## WQ-ENG-DRIVER-FOREIGN-OWNER — repaired and independently retested

Implementation: IMPLEMENTED. Final verification: PASS for ownership cleanup.
Earlier FAIL at e7a75aa remains in its [original receipt](windows-f3-20261018-results.md).

The old launcher added every tracked-port PID to its kill set. The repaired
launcher starts from its pool's live ChildProcess handles, verifies executable
identity, snapshots creation times and descendants, then rechecks identity before
cleanup. Port occupants are diagnostics and can never authorize termination.
Unrelated listeners also cannot cause cleanup verification to fail simply by staying
alive. Cleanup executes in finally after upstream teardown; ownership snapshots are
bounded and fail closed. POSIX keeps its root-PID cleanup path.

1. `node --test desktop/test/wdio-tauri-service.test.mjs desktop/test/wdio-driver-native.test.mjs`
   with `WDIO_NATIVE_CLEANUP=1` and exact driver paths: 14 PASS, no skips. Includes
   real tauri-driver/native child tree and an unrelated native listener; the latter
   remains alive/listening after owned-tree cleanup. Creation/PID-reuse and foreign
   ancestry classification have targeted regressions. `final-driver-tests.log`.
2. `run-occupied.ps1`: owned fixture holds native4445, actual upstream WDIO launches
   current pool/driver and runs teardown. Fixture process and listener survive;
   no driver processes remain. `occupied-final.json` and launcher/runner logs.
   The WDIO startup hook itself still fails on blank Edge `data:,` (existing
   upstream capability translation). That distinct target remains BLOCKED and
   is not relabeled PASS. Matching direct WebView2 executes GUI below.

`CROSS_PLATFORM_REVIEW_REQUIRED` for the small shared tooling script adjustment;
no product protocol/schema changed. Windows future driver/node/dependency upgrades
must rerun native collision/tree tests, not only mocks.

## WQ-F3-IMPORT-FAULT — native I/O and existing admission/recovery paths

Implementation: IMPLEMENTED. Scoped current F3 verification: PASS.

| Native check | Steps / expected | Result |
|---|---|---|
| Mid-selection SQLite capacity failure | On C: disk DB, DELETE journal, small page/cache quota; accept small index0 then large index1. Expect whole transaction rollback. | Native `database or disk is full` at index1; Jobs/Tweets/ledger all0. Drop maintenance, reacquire reader and maintenance, remove quota, retry accepts2; reopen confirms both. |
| OS journal I/O denial | New disposable NTFS directory; save exact ACL, deny current principal CreateFiles on that directory only; attempt transaction; restore exact descriptor before assertions/cleanup. | Native `unable to open database file` at index0; all facts/ledger/Jobs0; retry after ACL restore accepts2. No persistent ACL or DB lock remains. |
| Existing production/recovery competition | Hold actual ProductionCoordinator reader, start cancellable import drain, verify new production admission denied; cancel, release writer, take maintenance; run real startup recovery. | Recovery denied before candidate writes, persisted DOWNLOADED unchanged. Release maintenance, rerun legacy recovery; persisted FAILED/incomplete as existing contract requires. Reader admission works afterward. |

Storage `record_import` filter **11 PASS** at9dc3762 and final88c36af (includes both
native faults); Desktop `windows_native_production` **1 PASS** at9dc3762. Raw logs include
native error diagnostics and test executable paths; exact hashes in receipt.
Native crash-release/coordination tests are rerun separately when recorded there.

Limit: this proves the current admission port and actual legacy startup path, not
unimplemented C1 v2 process containment/publication or its live-writer-through-
finalization/restart matrix. Those remain gated under WQ-C1-WINDOWS, not inherited
from F3 tests. Hardware device removal/power loss is NOT_RUN and is not simulated
by a SQLite quota; the separate NTFS journal denial proves actual OS I/O rejection.

## WQ-F3-GUI-MATRIX — exact Full artifact, current supported UI

Method: gui-automated, native direct EdgeDriver/WebView2. Harness source9dc3762,
app source/hash above, isolated C: NTFS root `归档 文件`, 41 canonical archive/media
records, one intentionally seeded COMPLETE Job and one paused Batch41.

`run-matrix.ps1` / `record-matrix.e2e.mjs`: **3 tests PASS**, comprising:

- 12 exports: Job/Tweet/Batch × JSON/YAML × download/archive through actual DOM
  controls and native Blob-download buttons. Job/Tweet each1; Batch41 unique records
  (beyond20-row UI page); JSON/YAML documents equal; archive paths portable.
- Ambiguous scopes disable export. Record and root edits invalidate preview and
  disable accept until re-preview. Existing changed payload is preserved as conflict.
- Preview then replace root: stale rejection. Preview then mutate media: digest
  rejection. No synthetic Job or partial imported facts. Canonical database facts
  and all41 media hashes are identical before/after; Jobs1/Tweets41/ledger41/media41.
- Files really saved under actual Known Folder `E:\Shiraishi\Downloads` (ReFS),
  collision suffixes preserve names; exact filenames/sizes and document copies are
  in `exports.json`. NTFS claims apply to C: profile/archive and fault fixtures,
  not to E: save destination.

Screenshots: export matrix, conflict, stale root, stale media. Current page provides
text inputs plus Blob downloads, **no upload/folder chooser or Save As UI**. Chooser
cancel/overwrite/arbitrary-directory interaction is NOT_APPLICABLE to this version's
record page, not a claim of successful native dialog testing. Adding such controls
would be new UI scope. Current default-save path/Unicode archive-root behaviors pass.

Intermediate failures are retained: WebDriver clear did not update React state,
fixed with actual Ctrl+A/Backspace events; direct Storage seed used a short-path root
instead of command-canonical root, fixed by canonicalizing the test fixture. Neither
required product changes. Corrected fixtures/native actions then passed all checks.

## C1 — concrete prerequisite returned, not implemented or accepted

Implementation PLANNED; verification NOT_RUN; defer reason
`CROSS_PLATFORM_CHANGE_REQUIRED`. Windows capabilities are available; the missing
piece is an approved shared production containment/provisioning/recovery binding
and safe Windows SDK dependency boundary. See the [current plan](../development/windows-f3-c1-followup-20261018-plan.md)
for exact call sites, existing generic ports, proposed native requirements and
Linux/Windows actions. The workspace unsafe prohibition and frozen no-replace /
writer ownership guarantees were not relaxed. Windows v2 stays gated.

Cross-platform Owner must decide/commit the shared binding and return it; Windows
then implements native no-replace/Job Objects and the NTFS publication/restart/crash
matrix. Existing no-replace module failures remain unresolved historical evidence;
this round did not rerun the whole module and does not claim a new module PASS.

Full regression NOT_RUN: changed production scope is the Windows driver tooling;
Rust changes are bounded Windows tests/seed. Targeted native tests, formatter,
diff review and documentation audit are recorded. ZIP/7z exchange stays deferred.
Next Owner: Cross-platform Owner for the C1 decision and tooling review; Windows
retains C1 implementation/acceptance after that gate. This return is not completion
of the entire four-item request or whole Windows queue.
