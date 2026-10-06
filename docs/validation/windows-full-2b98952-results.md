# Windows Full 2b98952 build and validation — 2026-10-06

> 2026-10-06 follow-up: [user manual feedback and current FAIL/queue](../validation/windows-full-2b98952-feedback.md). First browser connection/task creation PASS scoped; status consistency and both download-mode completion FAIL user-observed. Earlier NOT_RUN below is the prior checkpoint, not the latest manual result.

## Identity and delivery

Source/build/validation revision: `2b98952aeaa717899ebcd13c09f058a28e998e73`, branch `codex/windows-validation-5c3efb79`. Effective Windows implementation: `44e60e369b5c57d3ed66b46fb610dc906a45780f`. Tracked input clean, prior local dependencies/data/artifacts preserved. Fetched origin; independently checked remote Cross-platform branch still points to `5c3efb79d2c7ffe3c84fe1f33821cd94b6c77a49`, the ancestor of this Windows return. No unintegrated newer Linux source was silently omitted.

Delivery: `dist-portable/XArchive-0.2.1-dev-2b98952-windows-x64-full.zip` (42,318,570 bytes, approximately 40.4 MiB), with adjacent `.zip.sha256.txt`. ZIP SHA-256: `06ab9d7973a413f2bbd28d7c81dd83d24daa2e77384f751714c7a4f7affa1e11`.

This is the repository's **Full portable installation package**, version **0.2.1 dev**; `tauri.conf.json` disables MSI/NSIS bundling. No new release/tag/public upload/signing is claimed. The Native Host installation manifest's `v0.2.1` is version-shaped metadata, not a published release identity. `build-identity.json` explicitly records source/implementation/channel and validation-candidate status.

Full directory contains Desktop, Extension (ID `iaajefkoanbkleojofoadeakelihbjne`), Native Host, fresh one-dir worker with Python runtime, pinned gallery-dl and aria2. No Telegram Local Bot API Server, tokens, cookies or accounts are bundled. Packaging retains the existing native-host absolute path manifest; after moving/unzipping, register/repair Native Host from the running package as documented, rather than treating the build-time absolute path as an installed browser registration.

## Build and provenance

Native Windows NTFS E: / PowerShell 7.6.6, Rust 1.98.0, Node 24.19.0; worker built with local Python 3.12.14 / PyInstaller 6.22.3. GUI/WinHTTP/loopback network available. Commands used the permitted escalated noninteractive shell because default sandbox initialization was unavailable.

- `cargo build -p xarchive-native-host --release --locked`: fresh current-source Native Host.
- `.venv-windows-validation/Scripts/python.exe -m PyInstaller --noconfirm --clean --distpath validation-artifacts/windows-full-2b98952/worker-dist --workpath validation-artifacts/windows-full-2b98952/worker-build sidecar/pyinstaller/xarchive-downloader.spec`: fresh isolated worker.
- Canonical `node desktop/scripts/build-portable-windows.mjs`, Full, dev channel, new isolated output, explicit fresh worker directory, Extension identity verified. `PORTABLE_ALLOW_BINARY_REUSE` unset; this invoked a new Vite + Tauri optimized release-profile build. Profile is release, channel is dev.
- Initial local gallery-dl trust gate **FAIL**: local SHA `0b36ae6734ed41e12be6be1b33d3165a450b3e0a811fc1b8c664c032f7f13b2c` differs from the reviewed pin. It was not executed or shipped. Downloaded the repository-pinned official build and verified SHA `80a92ecd47eb73268c7b8e58b6466e2d746f59ad06c0dbfa719c2c5801c7375a`. Temporarily staged only for packaging; original local file was backed up and restored with its original hash.
- Pinned aria2 archive trust gate PASS (`67d015301eef0b612191212d564c5bb0a14b5b9c4796b76454276a4d28d9b288`); packaged executable equals the extracted verified archive (`be2099c214f63a3cb4954b09a0becd6e2e34660b886d4c898d260febfe9d70c2`). Official external binaries are pinned inputs, not rebuilt from source.

## Executed verification

| Target | Result | Evidence and boundary |
|---|---|---|
| Rust workspace | PASS | `cargo test --workspace --locked`, explicit local `PYTHON`, `CARGO_INCREMENTAL=0`: **512 passed, 0 failed, 1 ignored**. Credential Manager mutation test remains NOT_RUN. `rust-workspace.log`. |
| Node Desktop / Extension | PASS | `npm.cmd test`: **214 + 52 passed**, no failures. `node-suite.log`. |
| Sidecar Python | PASS | `pytest sidecar/tests -q` with unique basetemp: **61 passed**. `python-suite.log`. |
| Rust formatting / docs | PASS | `cargo fmt --all --check`; staged docs-audit and whitespace check. Build generated only normalized-equal Cargo.toml line endings; restored without source changes. Strict Clippy/WDIO-specific suites NOT_RUN, no implementation change this build; direct native GUI covers only the executed subset. |
| Canonical fresh Full build | PASS | New optimized Desktop, current Native Host and worker; no application-binary reuse. `full-build.log`, `native-host-build.log`, `worker-build.log`. Existing unused-function/linker warnings retained. |
| Package integrity | PASS | 80 files, all 15 installation inventory entries exist, pairing helper declared/present, required components nonempty, x64 PE headers, Desktop PE/manifest version 0.2.1 match. All **8 worker .pyd** files retained; `_internal/python312.dll` exists. No pre-created download directory, user DB/log/.env payload. `package-verification.json`, `package-files-sha256.json`. |
| ZIP integrity | PASS | CRC test, every archived file's independently recomputed SHA matches the directory inventory, clean extraction and executable hash equality. ZIP remained unchanged during runtime validation; runtime writes occurred only in `zip-extracted` copy. |
| Component startup | PASS scoped | Bundled worker `--help`, gallery-dl `--version` = `1.32.13:2026.09.20`, aria2 `--version` = `1.37.0`, exit 0. |
| Packaged worker protocol | PASS scoped | Exactly five capabilities, unknown-field and relative-staging INVALID_COMMAND, accepted download/download_started for controlled `.invalid` URL, expected timeout failure and shutdown exit 0. `packaged-worker-probe.json`. **No download_completed/real archive PASS**. |
| Real gallery-dl transfer | PASS scoped | Bundled executable downloads a controlled loopback PNG-path payload, 44,032 bytes, exact bytes/SHA `0eca7bd2e27eed780d510417b5fba47e9ce65bfafc27300e5f99728a2a530bb9`. `gallery-loopback.json`. Not Tweet metadata/worker download_completed/archive acceptance. Initial guessed `directlink:` prefix was rejected as unsupported fixture syntax; corrected ordinary HTTP image URL succeeds, no product change. |
| Native Host + WebSocket | PASS scoped | Real packaged stdio framing, wrong-origin rejection, named-pipe bootstrap from new running Desktop, ephemeral-ticket WebSocket authentication. Ticket never persisted/printed. `packaged-bootstrap.json`. Initial client Origin with trailing slash rejected HTTP 403; canonical browser Origin without slash accepted. This is controlled client integration, **not installed browser or automatic reconnect**. |
| WebSocket close probe | FAIL scoped / follow-up | With successful authentication, close handshake did not complete in the probe's 3-second window (`graceful_close_within_3s=false`), repeated on a fresh controlled connection. Client was terminated; no archived task or message sent. Review close/drain behavior and acceptance timing before claiming lifecycle PASS; no production root cause/fix inferred. |
| Native GUI | PASS scoped | Exact ZIP-extracted executable at 1082×790 capture size: startup/SQLite, first-use portable download directory, Sidecar GUI hello→ready, Settings/Sidecar render and bundled gallery path, normal Sidecar stop, download-page navigation/default false. Visible keyboard focus confirmed before Enter false / Space true transitions; saved config matches. Normal app exit, restart displays true; validation copy restored false and closed normally. `gui-observations.json`; Computer Use observations. No orphan worker observed after stop/exit. |

Executable SHA-256:

- Desktop: `ede12b9cbaae1149291ad7ddddbf66dcc4337e82b1a367da2a972ed7d80b0889`.
- Native Host: `34d8ad97ff6afce042ca6950ad89364eb18d7f77b819ad6eb18b76b35a9a4665`.
- Worker: `11129a095b680a2befe6c3a4ffe9fdffcf41a57e7225efd58a734ed2d0e393fc`.

Local evidence: `validation-artifacts/windows-full-2b98952/`; raw logs, package and private runtime copy are not Git payloads. No user account/credential system settings or Native Host Registry registration were changed.

## Remaining manual acceptance and ownership

| Queue / state | Prerequisite and next action |
|---|---|
| WQ-DL-01 remainder, WQ-DL-02..05/07/08 — NOT_RUN | Use this new Full ZIP and a dedicated authorized X account/cookies, controlled Tweet media. Execute worker download_completed and Desktop COMPLETE archive/SHA, actual aria2 on/off process tree, cancellation/recovery, junction/reparse protection. Old DOWNLOAD_TIMEOUT real-archive failure is preserved; the synthetic invalid-domain timeout is a separate negative probe. |
| WQ-DL-06, M10/P6, WQ-PROXY-11/17 remainder — NOT_RUN | Full DPI/narrow/maximized/hidden focus/busy/error matrix. Current-size download/keyboard/restart subset passed only. |
| M13, WQ-PROXY-02..18 remaining — NOT_RUN | Isolated Windows user/VM with controlled Registry/static/PAC/WPAD, proxy and destination capture. Preserve prior implicit loopback DIRECT risk from throwing PAC; fixtures do not close real actual-egress acceptance. |
| Browser automatic pairing / lifecycle — NOT_RUN | Dedicated browser profile and register/repair this moved Full Native Host; validate actual Extension identity, bootstrap, persistence, restart/reconnect and lifecycle timing. Controlled client authentication does not close automatic-browser FAIL. Include current 3-second WebSocket close observation in Cross-platform review. |
| Real Telegram / local Bot API — NOT_RUN | Dedicated authorized test Bot/Channel and credentials supplied inside application, explicit send authorization. No message transmitted this run. Local server is external to Full. |
| Credential Manager ignored test — NOT_RUN | Explicit isolated native credential fixture; not normal user credentials. |
| MSI/NSIS/signing/public release — NOT_APPLICABLE | Current repository delivery is portable Full; request builds development candidate, not new installer format or publication. |

State remains `WINDOWS_VERIFICATION_PENDING`. Next Owner **Cross-platform Owner** for Git integration of Windows `44e60e3` and review of shared WebSocket close/drain behavior (`CROSS_PLATFORM_REVIEW_REQUIRED` investigation; no shared code changed). Windows Platform Owner retains isolated browser/real-download/proxy/DPI/Telegram acceptance. Artifact build succeeded; complete Windows/external-service acceptance and release GO are not claimed.
