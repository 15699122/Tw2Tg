# Windows F3 native import return — 2026-10-18

## Revision and environment

- Input: GitHub `dev` = `9e8fc3055d052b5281546eff6a813a6da37c79d4`.
  Ordinary fetch encountered a deleted configured feature ref; explicit
  `git fetch origin +refs/heads/dev:refs/remotes/origin/dev` succeeded and remote
  SHA matched the requested handoff. No formal workspace was overwritten by sync.
- Branch: `codex/windows-f3-native-import-20261018`. Initial branch/HEAD:
  `codex/windows-validation-40858835` / `eaca6f810f8c3a43be4ec52ca7c0c8bde4ff8d88`.
  Tracked tree was clean; after fetching the five old remote heads, no local-only
  commits remained. Untracked caches, prior packages and user data were preserved.
- Implementation: `775af5fa59c926db4740d5cfdd0d508e753f75df` native adapter;
  `ec58a5d7ca2e9a5953b889f9239f10599e5c34b5` responsive drain;
  `a81df969f7c3269c9a372e5235a351f5b08310d9` portable archive export;
  `36e51d0ed3234f2e67d3314ef2a9cbe352a5fc44` native E2E regression;
  final product source `dc731cce8a1b696af1dea00c3ad491b3e2ecc045`
  pins the root before revision verification. Handoff revision is the documentation
  commit containing the final cursor/receipts; all product changes are committed.
- Native Windows 11 Pro for Workstations Insider 10.0.29683 x64;
  PowerShell 7.6.6, Rust/Cargo 1.98.0, Node 24.20.0, Python 3.12.14,
  PyInstaller 6.22.3, WebView2 and signed Microsoft EdgeDriver 155.0.4283.45;
  tauri-driver cargo installation 2.1.0-alpha.0. `--version` is unsupported there.
  Native Computer Use and WebDriver are available. Captured window 1082×790;
  display scaling was not measured. Sandbox shell initialization failed; approved
  noninteractive native shell execution supplied the same Windows evidence.
- **Filesystem correction:** current E: checkout is ReFS. C: disposable fixture
  and application profiles are NTFS, verified with Get-Volume. NTFS claims below
  refer only to those C: runs. Base fixture:
  `%TEMP%\tw2tg-windows-f3-20261018`, including `归档 空间`.
  No real accounts, cookies, Telegram credentials or network download tasks used.

Common evidence root (local, retained):
`validation-artifacts/windows-f3-20261018/`. Committed sanitized receipts are in
[`evidence/windows-f3-20261018`](evidence/windows-f3-20261018/receipt.json).
All rows below are Windows-owned P1 unless marked P2, implementation IMPLEMENTED
unless explicitly PLANNED. Expected outcomes are rejection without durable writes,
successful exact round-trip or exit-0 build as appropriate. These scoped checks do
not authorize release publication or complete the whole Windows acceptance queue.

## Historical branch reconciliation

Counts are `dev-only / old-branch-only` against the exact input above. Local and
remote tips matched. All five branches remain; no whole merge/cherry-pick was used.
Full SHAs, subjects and per-commit changed paths: committed `branch-audit.json`.

| Branch | Tip | Counts | Unique commits and disposition |
|---|---|---:|---|
| `codex/browser-automatic-pairing-windows` | `30fc575` | 215 / 2 | `d65a01b`, `30fc575`: native pipe and bootstrap process test byte-identical on dev; runtime wiring has evolved. Receipts already in history; no duplicate restore. |
| `codex/windows-validation-1f14cea` | `937af92` | 219 / 1 | `937af92`: documentation only, b015fbe receipt already in history. |
| `codex/windows-validation-a3a02228` | `daf6805` | 136 / 3 | `f22ab3b`, `145f6ad`, `daf6805`: Sidebar callback prop/event regression already implemented; current navigation changed. Restore missing exact [receipt](windows-a3a02228-results.md), not old source/queue/cursor. |
| `codex/windows-validation-v020` | `e844ffe` | 280 / 1 | `e844ffe`: restore missing published-v0.2.0 validation receipt to history; keep original release/source/artifact identity. |
| `windows/webview2-readiness-gate` | `30b9ef8` | 424 / 7 | `5a1ecf1`, `b61a2e4`, `8b5cde4`, `35748f4`, `8a1714f`, `75d8c2c`, `30b9ef8`: current service already tracks allocated driver ports and pins native paths; banner/shell fixes present. Restore missing Full component receipt only. Run fresh native readiness/cleanup below. |

The older PASS results remain bound to their original revisions. Old package,
protocol, screenshot and GUI claims are not inherited by current dev.

## Implementation and review boundary

Windows implements existing Coordinator/WorkspaceGuard ports using independently
opened OS file locks. Shared readers cover legacy execution and startup recovery;
drain closes admission and waits cancellably. Owner death releases locks. Root and
all ancestors remain open without delete sharing; selected media deny write/delete
sharing, reject every reparse attribute, ADS and escaping path. Windows volume/file
identity participates in the existing opaque preview revision. Apply revalidates
whole selected evidence under pins and commits existing facts/ledger transaction.

`CROSS_PLATFORM_REVIEW_REQUIRED`: small shared cfg/type integration, RuntimeState
mutex release during wait with post-drain workspace-path recheck, Windows reparse
test, portable path serialization, and test fixture corrections preserve existing
abstractions. No shared schema/protocol/architecture change. Existing same-file
1.0.6 is added only to Windows Desktop dependencies. Linux Owner must review and
rerun Linux import/maintenance/executor regressions before dev integration.

## Executed checks and retained failures

Commands run from repository root with `--locked` for Rust. Log names below are
relative to the evidence root. Every failed attempt is retained; a repaired check
is a new observation, not a rewrite of the earlier result.

| ID / target | Method and source | Result / evidence | Follow-up |
|---|---|---|---|
| F3-BASE compile | automated, input 9e8fc30 | FAIL: 20 errors, Linux-only maintenance/cfg/type references; `baseline-check.log`. | Repaired Windows wiring at 775af5f; affected build now passes. |
| F3-LOCKS / NTFS | integration, 775af5f / a81df96 / final dc731cc | PASS 6 test functions (one child helper), `native-locks-ntfs.log`, `final-native-ntfs.log`, `dc-native-ntfs.log`. `cargo test -p xarchive-workflow --test windows_native --locked`, fixture-root env on C:. Covers shared admission, drain/cancel/reopen, scope contention/partial rollback, write/delete/rename pinning, real junctions and killed-child lock release. | Earlier drive-root create failure (3/6) and successful ReFS retry retained separately; no ReFS→NTFS inference. Repeat after adapter/environment changes. |
| F3-STORAGE | automated, final dc731cc | PASS 9, `cargo test -p xarchive-storage --lib record_import --locked`; `dc-storage-import.log`. Ledger migration, idempotency, conflict rollback, archive facts. | Whole module failures below remain open. |
| F3-DESKTOP | automated, dc731cc source delta committed afterward | PASS 10, `cargo test -p xarchive-desktop --lib record_import --locked`; `root-pin-tests.log`. Root replacement, Unicode/ADS, stale evidence, complete batch export and archive JSON/YAML final-path round-trip. | Revalidate with changed command/path/serialization code. |
| F3-FRONTEND / driver adapter | automated, 775af5f | PASS 16, `frontend-targeted.log`: record-import and WDIO service regressions. | Source unchanged afterward; source tests alone are not GUI acceptance. |
| F3-DESKTOP-MODULE | automated, intermediate working tree after 775af5f | FAIL: 317 PASS, 3 FAIL, 1 ignored; `desktop-second.log`. Export `/source/private` check repaired using has_root; 10 targeted tests pass. Remaining POSIX-shebang fixture Win32 error193 and Linux-v2 startup expectation remain. | Cross-platform Owner review fixture/target applicability; Windows C1 adapter acceptance pending. Ignored case NOT_RUN. |
| F3-STORAGE-MODULE | automated, intermediate working tree after 775af5f | FAIL: 144 PASS / 21 FAIL; `storage.log`. One separator-sensitive import fixture corrected using Path equality; 20 no-replace failures remain because Windows atomic publication adapter is not implemented. | Windows C1 implementation PLANNED; do not enable v2 or mark its restart/publication acceptance PASS. |
| F3-DRIVER upstream | gui-automated, a81df96 debug | BLOCKED for app acceptance: startup hook FAIL, all sampled windows `data:,`, Edge156 launched instead of app; `wdio-native.log`, `wdio/startup/discovery.json`. | Existing tauri-driver capability translation/tool failure; keep direct documented recipe. No product change from this failure. |
| F3-DRIVER cleanup | integration, current service at a81df96 | PASS scoped failure teardown: configured45884, actual intermediary51102/native4445; service-owned drivers/listeners absent afterward; no manual driver kill for this upstream run. `driver-cleanup.json`, `wdio/wdio.log`. | Static allocated-pool port tests pass; late actual survivor and occupied-port checks are recorded below; collision cleanup remains FAIL. |
| F3-NATIVE debug | gui-automated, a81df96 | PASS 6, `wdio-direct-final.log`; executable SHA-256 `72ba0b7bbb38c13ab169d96ab37013cd3ddaefde6a5515322cff6c167eac5c24`. Mixed original index1 acceptance; zero synthetic Jobs; YAML idempotency; stale whole-set rejection with no durable prefix; P1 archive export/reimport. | This debug PASS is not transferred to a different artifact. Exact final Full run recorded in receipt. |

Direct-driver probes at 45886/45997 failed to bind IPv6; bounded retries stopped.
Port4447 with `--allowed-ips=127.0.0.1` started successfully. Direct-driver cleanup
is recipe-owned PID cleanup, not evidence of the upstream launcher safety net.
Tauri MCP JS helper twice failed to register resolve-ref; Computer Use and actual
WebDriver supplied equivalent native evidence. Python YAML package absent; installed
Node YAML parser compared native JSON/YAML exports successfully. These are tooling
observations and did not justify production changes.

## Native GUI / save observations

Earlier GUI artifacts are separately identified:

| Source / executable SHA-256 | Observation and evidence |
|---|---|
| 775af5f / `bcd5b422d171c96467afd61c0c1a44be0f5344c0f559b8de4d059dc6da2bb2e9` | Actual import page: mixed preserve/accept selects original index1; accepted1, Jobs0/Tweets1/ledger1; duplicate and YAML reimport identical1. Record/root edits invalidate preview. Mutating second media after a two-record preview rejects entire set; DB stays Jobs0/Tweets1/ledger1. `db-before.json`, `db-after-mixed.json`, `db-after-stale.json`, screenshots `import-mixed-success.jpg`, `stale-media-rejected.jpg`. |
| same 775af5f | FAIL: during blocked drain, runtime mutex blocked status/UI and cancellation remained pending; `cancel-first-timing.json`. Reproduced with real NTFS maintenance-lock child; product fix ec58a5d. |
| ec58a5d / `326f5933f8a485369aac641d45669c28183493064ae86b842eff308064869803` | Actual cancel requested 9499ms after start; observed failure response and re-enabled Accept in493ms, before30s timeout. Same valid selection then accepted2 after holder stopped. `cancel-final-measured.json`, `cancel-released-import-success.jpg`. |

Native export buttons saved synthetic JSON883 bytes/YAML613 bytes into actual
Known Folder `E:\Shiraishi\Downloads`; Node YAML decode equaled JSON, imported
Tweet81002 identity preserved. Default Blob-download save behavior was observed;
no Save As dialog was presented, and arbitrary chooser locations/cancel/overwrite
are NOT_RUN. Final-artifact GUI/save recheck is identified in the committed receipt.

Final source dc731cc / executable
`de957d15f1252496738062507447ab693fc7453b99c47584c79f5291ac821b1b`:
exact release-executable WDIO repeated all six tests successfully, independent of
   the earlier debug/a81 results. Native Computer Use on this same Full executable
also previewed and accepted existing Tweet81002: accepted0/identical1. DB remained
Jobs0/Tweets3/ledger3; `final-full-duplicate.jpg` and final DB receipt. Profile root
was explicitly isolated on C: NTFS, so missing Extension files in that fixture
profile do not test package component discovery or real browser pairing.

Committed text log copies trim trailing whitespace only; original local logs are
retained. The interrupted closeout terminal action failed automatic approval review
because the review service hit its usage limit; no unsafe-action determination was
made and that action did not execute. User requested continuation, ordinary review
then succeeded; no product change or acceptance was inferred from the tool failure.

## Final Full candidate and acceptance boundaries

Final product source: `dc731cce8a1b696af1dea00c3ad491b3e2ecc045`.
Canonical `npm run build:portable:windows --workspace desktop` rebuilds Desktop;
`PORTABLE_ALLOW_BINARY_REUSE` unset. Native Host cargo release and worker PyInstaller
were freshly built in this batch at775af5f; their modules are unchanged through
final source. Gallery/aria2 inputs match reviewed pins. Extension identity remains
`iaajefkoanbkleojofoadeakelihbjne`. Final artifact filenames, exact hashes, sizes,
inventory and actual release-executable WDIO results: committed `receipt.json`,
`package-verification.json`, `package-files-sha256.json` and final build/E2E logs.

This is a Full **portable development candidate**, not an installed/signed release.
Installer installation/uninstallation, real browser pairing/downloads, external
services, whole GUI style/layout matrix, all export selector combinations and
database I/O fault injection are NOT_RUN this batch. Follow-up IDs and executable
steps are in the current [Windows queue](windows-queue.md), not old branch cursors.
Full workspace regression NOT_RUN: bounded adapter risk was covered incrementally;
broader affected module failures remain explicitly FAIL above.

Next Owner: Cross-platform Owner for `CROSS_PLATFORM_REVIEW_REQUIRED` review and
Linux revalidation/integration through Git. Windows retains C1 and remaining native
acceptance. No whole-queue WINDOWS_PASS or release readiness claim. ZIP/7z exchange
remains deferred. Reuse requires unchanged relevant source, dependencies, platform
contract and environment; otherwise REVALIDATION_REQUIRED.


Final Full native save: Tweet81002 JSON and YAML buttons each generated a file
in `E:\Shiraishi\Downloads`, using `download-records (1).json` / `.yaml` to
preserve the earlier same-name files. Final JSON equals the synthetic canonical
export; both final formats compare equal with Node YAML. Default-save behavior
only, no chooser acceptance inferred. Sanitized copies and hashes are in receipt.


## Late native driver cleanup checks

- Actual survivor helper PASS: parent tauri-driver64384 and child msedgedriver54084
  at4449/4450 were passed to the current launcher cleanup; both processes/listeners
  disappeared. Taskkill logged255 for the parent because the child was concurrently
  terminated; final PID/port verification passed. `driver-survivor.log.txt`.
- Initial temporary probe had an encoded-space fixture path and failed before
  driver startup; corrected with fileURLToPath. This harness failure is retained
  separately and did not change product/tooling source.
- Occupied-native-port check FAIL: owned fixture bound4445, upstream service used
  intermediary52738/native4445, and blank-Edge readiness failed again. Cleanup then
  logged tree-killing61524 and terminated the recipe runner (exit1, no final recipe
  output). Current collectPortOwnerPids/reapSurvivorDrivers accepts port-owner PIDs
  without verifying driver ancestry, so this collision result is unsafe to call
  cleanup PASS. All test-owned driver processes/listeners were absent on the final
  check. Evidence: occupied runner/launcher logs and receipt. Windows tooling Owner
  must constrain target ownership and repeat collision/survivor checks. This is a
  driver-tooling failure, not evidence that the F3 application failed.
