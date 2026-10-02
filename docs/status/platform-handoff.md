# Current Platform Handoff

Status: `CURRENT`. Prior active records, including the Windows b015fbe result from `937af922dc262bc2b2abda44d31aae8fba9a9a70`, are preserved in [handoff history](platform-handoff-history.md).

## Batch and revisions

### Windows continuation (2026-10-02)

Current owner: Windows Platform Owner. Branch `codex/browser-automatic-pairing-windows`, source/handoff `0c74087cf26d7120dfe6bbabb7c66b37b879fe3e`, fetched from Git and fast-forwarded on E:; no direct sync. The Windows implementation revision is the commit containing this continuation; until committed it is uncommitted work. State: `WINDOWS_IMPLEMENTATION_IN_PROGRESS`, not browser/package acceptance or a completed return handoff.

Implemented Windows bootstrap through existing Named Pipe and current runtime coordinator; local-only listener, existing owner ACL, 32 tracked workers, three-second absolute control I/O, bounded nonblocking writes and stop-aware shutdown. Response retention is bounded by the exchange deadline, avoiding unbounded pipe flush/limbo threads. Registry manifest checks now require exactly the fixed Extension origin. Existing Native business adapter and forwarding remain in place.

`CROSS_PLATFORM_REVIEW_REQUIRED`: shared WebSocket wrapper has a small stop-check addition and Windows-only timeout polling, preserving the protocol and connection lifetime. Native Windows testing exposed shutdown waiting for an active recv: initial Desktop module execution hung and was explicitly terminated; first bounded fix still failed the existing <1s assertion at ~3s. Polling both pre-auth and authenticated reads resolved the targeted 12/12 Windows WebSocket tests. No assertion was weakened. Linux owner review/revalidation of this wrapper remains required after Git return.

Native Host tests PASS: library 11, entry 1, real Windows subprocess/pipe framing 1; Unix process test NOT_APPLICABLE. Pipe/registration targeted tests PASS 4/4, including repeated business requests, bootstrap ticket replay rejection, duplicate listener rejection and stalled-client shutdown. Strict Desktop/Host Clippy PASS after repairing an existing Windows-only collapsible-if lint. Initial zero-byte nonblocking pipe response failure is preserved in Windows history and fixed by waiting within the absolute deadline.

Desktop module rerun: 186/187 PASS; the sole config persistence AccessDenied FAIL was independently PASSed with the exact unchanged test under permitted escalated execution. It is an environment restriction, not a clean module PASS claim. fmt/docs audit/diff whitespace PASS. No production config workaround introduced.

Read-only HKCU inspection found both Edge and Chrome XArchive registration keys ABSENT. Current debug Host SHA-256 `dff2ecd767bf3d9b8d44c7724e732547466fffafaa365546cd61f40677de84ee` identifies the subprocess test artifact, not a Full package. Current-user pipe success does not prove cross-user ACL denial. Real registration/install, native GUI, Full packaging and browser/MV3/E2E remain NOT_RUN: no current automatic-pairing Full artifact has been assembled/installed, and native GUI automation is disabled in this session. Browser automation alone cannot establish Desktop GUI acceptance. Old acceptance failures are retained.

- Task: browser automatic pairing shared Phase 2/3, then Windows Native Host/Named Pipe integration.
- Branch: `dev`; input/source baseline: `82a0df75d3d1a4a223a2caaf5df0bbbd86de0164`.
- Shared implementation/source revision and handoff revision: the Git commit containing this record and the tested implementation. Before commit+push this remains uncommitted work; Windows must fetch the explicit resulting SHA before starting.
- Current state: `READY_FOR_WINDOWS` only upon that commit+push. Shared Phase 2/3 is implemented and Linux verified; this is not Windows acceptance.
- Current owner: Cross-platform Owner until Git delivery; next owner: Windows Platform Owner. The current Codex session has native Windows execution and will continue after delivery, as explicitly requested by the Owner.
- Windows `937af92` documentation was reconciled through Git cherry-pick without committing separately; original history/evidence is retained. No direct filesystem sync. E: caches, artifacts and untracked user data are preserved.

## Shared work completed

- Desktop defaults to `127.0.0.1:0`, publishes the actual endpoint and random runtime generation; only the diagnostic port override remains. Legacy token authentication is retired.
- Listener-scoped CSPRNG tickets: 256 bits, 30-second TTL, atomic single consumption, exact fixed Extension origin, shutdown/generation invalidation, 64-ticket capacity and 100ms issuance interval.
- Exact single Origin and `/` resource; no query credentials. First frame consumes the ticket before business. Absolute three-second handshake/auth read deadline, 16KiB handshake/4KiB auth budgets, 1MiB frame/message limit, 32 workers and 64 admissions/second. Shutdown interrupts idle and handshake sockets and reaps workers.
- Native Host shared framing/control forwarding validates response/request identity and redacts malformed control errors. Unix Host control entry checks browser origin and forwards to current-user Desktop Unix IPC; real subprocess framing regression and IPC→WebSocket authentication pass.
- Extension defaults to automatic bootstrap, single-flight connect, generation cancellation, strict control/auth responses, bounded retries, pending cleanup, authenticated live-socket status and worker reconstruction. Credentials are memory-only; automatic success clears legacy stored token. Native business mode is explicit; failures never replay across channels.
- Sender ID/URL/payload allowlist separates trusted popup/options management from X/Twitter content requests.

## Validation and evidence

Environment: native Windows Codex shell orchestrating Ubuntu WSL2 `Linux 6.18.33.2-microsoft-standard-WSL2`; Rust/Cargo 1.98.0; Node 26.7.0. Tests bind to baseline above plus this commit's reviewed changes. Test builds are local debug artifacts, not packaged GUI evidence.

| Scope | Result | Evidence |
|---|---|---|
| Rust workspace | PASS, 399 tests | `cargo test --workspace --quiet`, exit 0; Desktop 187, protocol 23, Host library 10 + entry 1 + process integration 1 |
| Desktop + Host strict lint/fmt | PASS after one callback-signature lint repair | `cargo clippy -p xarchive-desktop -p xarchive-native-host --all-targets -- -D warnings`; `cargo fmt --all -- --check`, exit 0 |
| Extension | PASS 52/52 | `npm test` Extension workspace; lifecycle/contract/sender tests |
| Desktop Node | PASS 189/189 after inventory correction | Initial 188/189 retained: new helper absent from expected inventory; updated strict expected list, affected 4/4 and final workspace rerun PASS |
| Frontend syntax/build | PASS | `npm run check`, `npm run build`; final Extension syntax check, exit 0; Vite chunk warning non-fatal |
| Source baseline | PASS | non-interactive `git fetch origin dev`; HEAD and origin/dev equal input SHA before delivery |
| Windows native/browser/package/E2E | NOT_RUN | Windows implementation not yet ready at this handoff; next Owner now has native execution capability |

Initial baseline Desktop compilation failed because status called missing `runtime_instance_id()`; implemented the listener-generation API. Initial strict Clippy reported the large HTTP error type mandated by tungstenite's Callback; one documented function-local allowance preserves its required signature. Automatic approval rejected several combined edits; bounded edits and explicit Owner approvals for IPC, Host and Extension integration were then used. No product assertion was weakened.

Common evidence fields: owner Cross-platform; priority P0; method automated/integration; platform Linux x86_64; prerequisites cached Rust/Node dependencies, no real account or browser required; expected fail-closed authentication with bounded resources and preserved business framing. Build origin is the Cargo/Node commands above; Native Host debug executable SHA-256 `da124cd731fd67167d63e5684efb1617668e746c7d0ef8c8974fd96b8cdf01d2` (local integration artifact, not Windows release). Revalidation is required when related source/dependencies/platform contracts change. No release is approved; Windows follow-up is the next action.

## Windows work and validation required

P0: consume the existing shared bootstrap request/response on Windows Named Pipe, pass the current coordinator from Runtime, enforce bounded client/server I/O and current-user ACL/pipe resource policy. Native Host must use trusted browser origin and the existing fixed Extension identity; do not generate a second secret or accept caller-supplied executable paths.

P0: native ticket/Origin/resource tests, actual pipe bootstrap and Host subprocess framing on Windows, Edge/Chrome registration/profile identity and real browser bootstrap/query. Bind source SHA and exact EXE/Extension hashes. Registration, GUI, packaging, cold start and E2E are distinct results; no mock/build closes those gates.

P1: controlled cold-start policy, UI automatic-connection wording, upgrade/repair/uninstall/portable movement, idle/worker/sleep observations. Current shared state restores on startup/status/next business request; no heartbeat is introduced and permanent MV3 liveness is not promised. Heartbeat is a separate evidence-driven decision, not a substitute for worker recovery.

## Risks and expected behavior

Old Extension token frames are rejected by the new Desktop. Explicit legacy mode only targets old diagnostic Desktop deployments; there is no automatic downgrade. New Extension against old Host yields a visible bounded control failure. GUI remains the existing manual-token presentation until the Windows UI batch.

Fixed Extension identity is `iaajefkoanbkleojofoadeakelihbjne`, derived from the committed manifest key. WebSocket origin excludes the trailing slash; Native Messaging invocation origin includes it. Windows cross-user/ACL and browser-origin behavior are still unverified.

WQ-WS-02/03 post-auth persistence/controlled 5s/45s results remain historical and unresolved; automatic-pairing Linux tests do not close them. Real archive/duplicates remain paused pending reviewed Windows pairing behavior. Telegram work and release publication are outside this batch.
