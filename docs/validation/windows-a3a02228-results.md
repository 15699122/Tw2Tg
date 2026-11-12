# Windows a3a02228 validation return — 2026-10-06

## Revisions and scope

Input branch `cross-platform/automatic-pairing-reconcile-20261002`, requested handoff `a3a022286d79de7d8eac290618cdf4b7fbd6afb1`. Fetch first: ordinary fetch only fetched the configured Windows branch; explicit SHA fetch then obtained the target and `ls-remote` confirmed the Linux branch tip. Return branch `codex/windows-validation-a3a02228` created at that exact SHA. Input tracked clean; existing untracked dependencies, packages, caches and user data preserved. No direct sync.

Input includes reconciliation `6afb351`, shared fixes `6d77b6b`, history implementation `2e3d738`, documentation checkpoint `72762cd`, final handoff documentation `a3a0222`. The received handoff still mixed source `72762cd` and CROSS_PLATFORM_IN_PROGRESS/current Windows Owner; this return binds input validation/build to **a3a0222**, repaired validation/build to **f22ab3b1efc8625a88e6e60764ae26441c2c505d**. Documentation return revision is the commit containing this entry; push on the above return branch. Prior `fd7d834` manual failures remain historical evidence, not current-artifact results.

Requested `cross-platform-validation.md` §16 does not exist: it is navigation-only. Applied linked ownership/validation rules and `git-platform-handoff.md` §16 revision template. Read current handoff/queue/manual P1–P8 and the pairing/settings/proxy plans. Executor metrics instrumentation remains IMPLEMENTATION_NOT_READY, not an implied UI/backend completion.

Native Windows 11 Pro for Workstations Insider Preview 10.0.29680 / x64 NTFS E:, PowerShell 7.6.6, Rust 1.98.0, Node 24.19.0, Python 3.12.14. Native Computer Use available; captured viewport 1082×790, OS DPI/browser profile versions not recorded. No external account/cookies/Telegram credentials used. Sandbox shell initialization unavailable; approved noninteractive native shell used. Equivalent execution path produced native evidence without changing acceptance criteria.

## Implementation finding and repair

At input a3a0222, both Sidebar download navigation and SQLite status invoke `refreshDownloadPage`, but Sidebar neither receives this prop nor closes over App's scope. Independent invocation of the actual JSX-transformed Sidebar handlers reproduces **ReferenceError: refreshDownloadPage is not defined** for both targets; the old 216-test source-contract suite passed despite this gap. Scope WQ-DL-06 navigation: original FAIL retained.

Small shared repair **f22ab3b** passes the existing callback from App to Sidebar, preserving the API/architecture and download behavior. Added an event-invocation regression for both handlers plus parent prop binding; repaired Node suite 217 PASS. `CROSS_PLATFORM_REVIEW_REQUIRED`: Cross-platform Owner reviews/integrates this two-file repair. No changes to shared metrics contracts, cancellation policy, transport architecture or business logic.

## Exact artifacts

Canonical `node desktop/scripts/build-portable-windows.mjs`, Full/dev, `PORTABLE_ALLOW_BINARY_REUSE` unset: fresh Vite/Tauri optimized Desktop build at input and again after repair. Initial executable SHA `14d5d5962c18581ae713b20afe3c37a7d967522a7a88b2904f842c568d4296a6`; final Desktop SHA **c62d14ef01026f246f91880ac7063ea63abde462ba9477efbc5446083afb96a0**.

Final ZIP `dist-portable/XArchive-0.2.1-dev-f22ab3b-windows-x64-full.zip`, 42,331,639 bytes; SHA **936f40af27f91c3d93d5606e5f3d8482e2e0cf53156e23b12e104b7a97a2d5e2**, adjacent SHA file. Dev portable validation candidate, not release/signing/MSI/NSIS acceptance. Native Host and worker reused from the verified 2b98952 build **because their source, protocol and dependencies are unchanged**; no claim of fresh rebuilding those components. Native Host SHA `34d8ad97ff6afce042ca6950ad89364eb18d7f77b819ad6eb18b76b35a9a4665`; worker SHA `11129a095b680a2befe6c3a4ffe9fdffcf41a57e7225efd58a734ed2d0e393fc`. All acceptance observations below were newly executed against final components, not inherited.

Pinned gallery-dl copied temporarily from the verified prior package for assembly; package SHA checked against current official pin. Original local gallery file restored to SHA `0b36ae6734ed41e12be6be1b33d3165a450b3e0a811fc1b8c664c032f7f13b2c`; not executed as a trusted package input. Packaged aria2 equals the independently verified pin extraction. Package identity metadata records component reuse and exact source.

## Executed results

| Target / method | Result | Evidence and limit |
|---|---|---|
| Input Desktop Node / automated | PASS | 216 tests, exit 0; local `node-a3a02228.log`. Does not prove event callback scope. |
| Input native Desktop + Storage command / automated | FAIL initial | Desktop 256 PASS, 1 FAIL, 1 ignored; PAC fixture's loopback response read got WinSock 10053 ConnectionAborted. Initial invocation stopped before Storage. Preserve `rust-tests.log`; not evidence of product PAC policy failure. |
| PAC targeted rerun / automated | PASS | Same source, 1/1, no production change, `pac-retry.log`. |
| Storage module / automated | PASS | 73/73 on native Windows, `storage.log`; includes migration/pagination/metric fencing. Linux count 76 is not Windows evidence. |
| Final Desktop module / automated | PASS | 257/257, 1 ignored native Credential Manager mutation test; `rust-retry.log`. Initial transient failure retained. Includes close-frame unit assertion; does not override packaged integration FAIL. |
| Final Node / automated | PASS | 217/217, `node-repaired.log`; actual Sidebar event invocation included. |
| Input navigation / targeted component execution | FAIL -> repaired PASS scoped | JSX-transformed real handler invocation reproduces both ReferenceErrors; new regression and final native GUI navigation/SQLite click succeed. |
| Canonical build + integrity / automated | PASS scoped | Both fresh Desktop builds; final 80 files, all installation inventory files, pairing helper, x64 PE, 8 worker .pyd and Python runtime, external pins, component startup, ZIP CRC/every file hash, clean extraction. `package-verification.json`, inventories/build logs. |
| Native Host/bootstrap/auth / integration | PASS scoped | Real packaged Host origin rejection, named-pipe bootstrap, WebSocket authentication; one running Desktop only, listener owned by validation PID 7056. Tickets only in memory. Not actual browser registration, persistence or automatic reconnect. |
| WQ-FULL-WS-CLOSE / integration | **FAIL** | Two final-package authenticated connections still fail close completion within 3 seconds despite shared repair. `packaged-bootstrap.json`, helper reports `graceful_close_within_3s=false`; its overall PASS means bootstrap/auth only. Close unit PASS and packaged close FAIL must stay distinct. No production cause inferred. |
| Packaged worker / integration | PASS scoped | JSONL hello/five capabilities, unknown-field and relative-staging INVALID_COMMAND, absolute controlled `.invalid` URL download_started -> expected DOWNLOAD_TIMEOUT, shutdown exit 0. `packaged-worker-probe.json`. No download_completed/archive acceptance. |
| History/Settings native GUI / gui-automated | PASS scoped | Empty history; independent Sidebar and SQLite callbacks; 21 synthetic terminal FAILED jobs inserted only into closed isolated fixture DB, no jobs submitted externally. Dashboard total 21/recent list 20; Downloads first page exactly 20 newest, second page exactly 1 oldest, end button disabled. Missing metrics display 未记录/不适用, never inferred. Settings renders and no storage section remains. Captures/tool observations at 1082×790. |
| GUI layout / gui-automated | FAIL scoped follow-up | Dashboard 查看详情 button text wraps into a narrow vertical stack at 1082×790; newly added component links show default-style buttons. Functionality/pagination evidence is separate from layout acceptance. Cross-platform presentation review; full DPI matrix NOT_RUN. |
| fmt/docs/whitespace / automated | PASS | `cargo fmt --all --check`, docs audit, final diff review; generated normalized-equal Cargo.toml restored. |

GUI wrote only ZIP-extracted private runtime copy. It was closed normally; no validation Desktop/worker process remained. ZIP/assembly directory kept pristine. Native Host Registry installation, system proxy settings, user profiles/data and real jobs were not modified.

Full repository regression, WDIO and native strict Clippy NOT_RUN: risk-based affected Desktop/Storage modules, UI suite, build and actual targeted integrations selected; unrelated unchanged modules/services not rerun. Linux strict Clippy `useless_conversion` failure at CloseFrame conversion remains separate follow-up. No weakening to make checks green.

## Individual WQ-DL results and manual queue

All rows bind to f22ab3b/above hashes, Windows x64; Windows Platform Owner executes. Prior full-archive FAIL remains open. No whole-row PASS from a capability/build/fixture subset.

| ID / priority | Current result / reason | Required next steps and expected evidence |
|---|---|---|
| WQ-DL-01 / P0 | PASS protocol subset; completed download **NOT_RUN**, authorized fixture/account not configured | Start exact Full Sidecar; confirm hello/capabilities; select mode before submission; run authorized media to download_completed and committed archive/SHA. |
| WQ-DL-02 / P0 | **NOT_RUN**, real direct-download acceptance missing | Dedicated account/cookies and authorized Tweet; keep settings fixed during run; capture new cancellation source, terminal state, archive inventory/bytes/SHA. Earlier user's real-download FAIL not closed. |
| WQ-DL-03 / P1 | **NOT_RUN**, browser-cookie native integration missing | Dedicated browser profile/cookie fixture; verify allowed direct gallery-dl acquisition and redacted failure, no credential export. |
| WQ-DL-04 / P0 | **NOT_RUN**, actual active-transfer lifecycle fixture missing | Isolated controlled slow transfer, cancel/timeout; identify executor token vs worker event and descendant exits; no partial archive/orphan. New error text alone is not a repair of interruption/recovery. |
| WQ-DL-05 / P1 | PASS relative-path rejection subset; reparse/real staging **NOT_RUN** | Controlled drive-letter staging + junction escape fixture; assert rejection/no writes outside tree. |
| WQ-DL-06 / P1 | Input navigation FAIL repaired; final navigation/history/missing metrics PASS scoped; DPI/toggle/restart/native picker **NOT_RUN** | Exact Full, 100/125/150/200% DPI, native directory picker/cancel, keyboard/focus/busy/long-path, aria2 persistence. Fix/recheck dashboard detail-button layout. |
| WQ-DL-07 / P1 | **NOT_RUN**, real aria2 backend/process evidence missing | Enable before creating task; same authorized media, equivalent committed content, actual process tree. |
| WQ-DL-08 / P1 | **NOT_RUN**, controlled interrupted active download absent | Close isolated fixture during real transfer, restart/reconcile/retry; diagnosable interrupted state and no partial COMPLETE. |

Additional queues:

- WQ-FULL-STATUS-01 / P1: **NOT_RUN** actual-browser >30s/status and no-active-job settings-change matrix. Current copy/refresh-source regressions pass, canonical connection-source semantics still deferred CROSS_PLATFORM_CHANGE_REQUIRED. Dedicated profile, exact Native Host registration, timestamp/runtime instance ID and actual listener port required.
- WQ-FULL-WS-CLOSE / P1: **FAIL**, Cross-platform + Windows investigate actual DeadlineStream/socket/close integration and timing; reproduce with same packaged stdio/bootstrap/client, require peer close within 3s before closure acceptance.
- WQ-HISTORY-METRICS / P1: **NOT_RUN / IMPLEMENTATION_NOT_READY**, Cross-platform Owner owns reviewed attempt-scoped instrumentation contract. Existing persisted/absent metric display does not establish runtime collection.
- WQ-HISTORY-NAV / P1: Cross-platform review/integrate f22ab3b; retain input FAIL then revalidate resulting canonical source. Pagination storage and native fixture PASS only.
- WQ-HISTORY-LAYOUT / P2: Cross-platform presentation fix + Windows current-size/DPI validation; dashboard detail buttons must remain readable and usable.
- PAC/WPAD actual egress, browser restart/reconnect, Credential Manager mutation, Telegram real send and other manual rows remain NOT_RUN with existing isolated-account/OS/service prerequisites. No new send authorization inferred.

Next Owner **Cross-platform Owner** for Git reconciliation/review of f22ab3b, native close FAIL investigation, layout and deferred status/metrics contract decisions. Windows Platform Owner retains real browser/download/process/DPI/picker/PAC acceptance on the reconciled fresh build. State `WINDOWS_VERIFICATION_PENDING`, with scoped FAIL; release GO not claimed. No WINDOWS_BLOCKING escalation. Local raw evidence `validation-artifacts/windows-a3a02228/` stays out of Git.
