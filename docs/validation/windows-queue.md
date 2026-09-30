# Windows Validation Queue

本文是当前 Windows 验证队列的唯一入口。历史执行结果、环境日志和逐轮 reconciliation 保存在 [`../development/windows-validation.md`](../development/windows-validation.md)；Windows 执行规范和报告模板见 [`windows.md`](windows.md)。

当前 WQ-P1-16/WQ-P1-17 的具体执行顺序和 PowerShell 步骤见 [`windows-wdio-handoff.md`](windows-wdio-handoff.md)。

### 2026-09-30 preN migration execution and manual acceptance

Source: `release/v0.2.0`, handoff `7a3374b7af1a647addb0b9ab4e50e315468b6d26`, batch `ba0f8aa`. Windows Owner is executing the Plan in `../status/platform-handoff.md`; preflight evidence is in `windows-validation-history.md`.

| ID | Category / purpose | Related files/change | Prerequisite and exact steps | Expected result | Priority / blocks Linux development | Current status / manual interaction |
|---|---|---|---|---|---|---|
| WQ-MIG-DISPATCH-01 | Integration: execute frozen migration | Migration workflow, ledger, per-target notes | Formal handoff and workflow registration; user-configured publishing secret | Ten verified drafts before any publication; public links before old Release retirement | P0 / no | `WINDOWS_PASS`: 10 dry runs, 10 drafts, 10 public pre-releases / 41 asset URLs verified; 10 old Release objects retired, old tags retained; ordinary workflow restored |
| WQ-MIG-GUI-01 | Runtime: per-version Windows GUI observations | Migrated application exe/archive/Full assets, where the historical era supplies them | After asset gates, extract into isolated version directories, start each available application shape separately on an observable native desktop; collect version/package SHA, window screenshot and startup logs; close and check process cleanup | Actual startup/window result for each shape, without carrying another version's result forward | P1 / no | `NOT_RUN`: package migration preserves historical acceptance outcomes; GUI remains separate. Native Computer Use inventory is available this round; no XArchive window was opened. Requires manual/native desktop interaction |
| WQ-MIG-INSTALL-01 | Integration: Extension/Native Host installation | Applicable migrated dependency/Full/Extension packages | After package gates, use isolated browser profile and documented installer, replace an installation-required Host path only through supported installation, inspect Registry/origin/host path, open Popup/Options, and record connection result separately | Browser load and installed path/origin/host connectivity observed; no placeholder treated as installed acceptance | P1 / no | `NOT_RUN`: installation was not executed by this historical metadata migration; requires isolated profile/Registry/browser acceptance after applicable assets are available |

These entries add no historical PASS. In particular old `v0.2.0-pre.16` WDIO FAIL and Native Host/GUI NOT_RUN remain unchanged. No ordinary `windows-release.yml` build/upload is dispatched for migration; its static same-source gate review is recorded separately.

### 2026-09-30 dependency advisory remediation (security/dependency-advisories-2026-09-30)

Plan: `../development/security-remediation-plan.md` (batches A–D). Baseline `origin/main` `c98d6106`. Batch A (npm `ip-address`/`brace-expansion`) and batch C investigation are Linux-owned; the items below are the Windows-owned parts and must not be recorded as PASS until actually executed.

| ID | Category / purpose | Related files/change | Prerequisite and exact steps | Expected result | Priority / blocks Linux development | Current status / manual interaction |
|---|---|---|---|---|---|---|
| WQ-SEC-PERMS-01 | CI hardening: explicit workflow token permissions | `.github/workflows/windows-worker-artifact.yml` (Code scanning #2, `actions/missing-workflow-permissions`) | After the batch D change is pushed to `main`, dispatch the worker workflow and confirm `xarchive-downloader.exe`, `_internal\python312.dll` and the uploaded artifact are produced; confirm the job succeeds with `contents: read` | Worker artifact integrity preserved under least-privilege token; default-branch CodeQL re-scan closes #2 | P2 / no | `WINDOWS_VERIFICATION_PENDING`: not executed on Linux. Note the repository default token permission is already `read`, so this is an explicit-declaration gap, not a confirmed write-token exposure |
| WQ-SEC-GLIB-01 | Dependency evidence: Windows `glib` dependency tree | `Cargo.lock`, `desktop/src-tauri/Cargo.toml` (Dependabot #1, GHSA-wrw7-89jp-8q8g) | With a usable registry cache, run `cargo tree --locked --target x86_64-pc-windows-msvc -i glib` and record whether the GTK/WebKitGTK chain pulls `glib 0.18.5` on Windows; execute an optimized/release profile desktop build | Recorded Windows dependency evidence and a release-profile build result, replacing the `NOT RUN` left by the Linux offline-cache failure | P2 / no | `NOT_RUN`: Linux could not complete this because `--offline` lacks `clipboard-win`. This item must not be closed as `NOT_APPLICABLE` |
| WQ-SEC-EXTRACTZIP-01 | Security regression: `extract-zip` symlink handling | `@wdio/cli` → `@wdio/utils` → `@puppeteer/browsers` (Dependabot #4/#6) | Only if batch B produces a code change: exercise WDIO browser/driver acquisition in an isolated temp directory and confirm the two advisories' distinct attack shapes (symlink escaping the extraction dir; symlink-then-file same name) are both covered | Regression evidence for both GHSA-jmr9-qjv8-65gv and GHSA-7pqw-9j4j-h8q3, recorded separately | P2 / no | `NOT_RUN`: batch B currently keeps the existing RISK-023 accepted risk because upstream ships no fixed version. If no code change is made, this stays `NOT_APPLICABLE` and the accepted risk is unchanged |

## 状态规则

- `WINDOWS_VERIFICATION_PENDING`：功能或代码已有，但需要在 Windows 目标环境确认；不阻塞 Linux 开发。
- `WINDOWS_VERIFICATION_BLOCKING`：只有缺少 Windows 结果会使后续 Linux 设计或实现无法可靠继续时使用。
- `WINDOWS_PASS`：当前关联 revision 已完成 Windows 验证。
- `WINDOWS_FAIL`：Windows 验证发现需要处理的项目代码或平台问题。
- `WINDOWS_BLOCKED`：前置环境、账号、权限或外部服务不可用。
- `NOT RUN`：本轮没有执行，必须同时说明原因。
- `NOT APPLICABLE`：当前项目配置或验证范围不适用。

当前没有 `WINDOWS_VERIFICATION_BLOCKING` 项目。

### 2026-09-29 v0.2.0 整合基线集中验证队列（R2 轮）

**2026-09-29 发布策略变更（仅适用于实现该变更的后续 revision）：** `WQ-P0-WHITE-04` 中“WDIO readiness 失败禁止上传”不再作为发布门禁；这是策略替换，不是旧检查通过。每次 Release/pre-Release 都由 Windows 构建 workflow 的独立 job 下载同一次 run 的 `.exe`、验证 source SHA 与 exe SHA-256、执行 WDIO 并保存诊断；FAIL/BLOCKED 不单独阻断发布。资产缺失、空文件、manifest/SHA-256 不符仍阻断上传。Windows Owner 逐次记录 tag、build run ID、source SHA、exe SHA-256、Release 链接、WDIO job/run ID、PASS/FAIL/BLOCKED/NOT_RUN、原因、环境、日志 artifact、人工 GUI 结果与后续动作；run `36568849798` 仍为 `BLOCKED_AUTOMATION`，不能继承为新版本 PASS。

分支 `release/v0.2.0`（整合 merge `61ff1ae`，Linux 收口 `06bfa39`）。本轮只验证**整合后基线**在 Windows 上的包、格式与启动形态；Linux 门禁结果不得改写为 Windows PASS。发布口径见根 `README.md`《使用范围与限制》与《发布资产（v0.2.0）》：本版本不承诺真实 X 归档成功，也不保证其它设备可用，但每个被称为"可启动"的资产必须取得实际启动证据。

| ID | 类别 | 验证项目 | 前置条件 | 精确行为 | 预期结果 | 优先级 | 状态 |
|---|---|---|---|---|---|---|---|
| WQ-V020-ARFMT-01 | Packaging | 四个压缩包均为真实 7z 且可解压 | 整合后 revision、7-Zip | 对 `windows-x64.7z`、`windows-x64-repository-dependencies.7z`、`windows-x64-full.7z`、`extension.7z` 分别执行 `7z l`/`7z t` 与实际解压；确认 magic/格式不是 ZIP 改名 | 四个包均以 7z 格式创建并通过完整性测试；解压目录结构与 package manifest 一致；`Expand-Archive` 不应被用于这些包 | P0 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-V020-PKG-FULL-01 | Packaging | 整合后 fresh Full `7z` 构建（R2） | `XARCHIVE_EXTENSION_ID`、Windows 全量工具链 | 在新目录执行 `npm run build:portable:windows --workspace desktop`（`PORTABLE_PACKAGE_TYPE=full`） | `package-manifest.json` 版本为 `0.2.0` 且与 `tauri.conf.json` 一致；Extension/Native Host/worker 组件齐全；`native-host` installation manifest 的 `release_tag` 为 `v0.2.0`；无 `.env`/SQLite/日志/缓存/测试产物，且保留 `_internal/*.pyd`/`.dll` | P0 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-V020-START-01 | Runtime | 三种"可启动"形态分别取证 | WQ-V020-PKG-FULL-01 产物 | 依次尝试启动：单独 `.exe`、应用 `7z` 解压后的 `.exe`、Full `7z` 解压后的 `.exe`；各自记录进程、窗口、页面与可诊断错误 | 三者均可按 README 描述启动；**不得**用其中一次成功推断其余形态；任一形态无法启动则 no-go 并修正文档或资产 | P0 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-V020-EXT-01 | Integration | Extension `7z` 解包后内容与加载 | WQ-V020-ARFMT-01 | 解压 Extension 包，核对 `manifest.json`（`version: 0.2.0`、固定 public key、Extension ID）、Popup/Options 与 `src/` 必需文件、无私钥/凭据/测试目录；按受控方式加载解压目录 | 清单与 `extension-package.mjs` 元数据一致；Popup 显示的版本来自 `chrome.runtime.getManifest().version`；Popup 可打开。**连接状态单独记录**，不得把 Popup 展示记为配对成功 | P1 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-V020-E2E-ORD-01 | Integration | 直连 msedgedriver 配方 E2E（WQ-ENG-09b-ORD-R2） | 固定 WebView2 runtime 与匹配 `msedgedriver`、普通 release `.exe` | 按 [`windows-wdio-handoff.md`](windows-wdio-handoff.md) 的 reviewed 配方以 `WDIO_DIRECT_DRIVER=1` 运行 dashboard spec | session 建立、契约门与 Dashboard 断言通过；失败保留 readiness/contract/discovery 取证与驱动日志 | P1 | `BLOCKED_AUTOMATION`（Actions run `36568849798`：`DevToolsActivePort file doesn't exist`；driver/runtime 一致且应用进程可存活，失败发生在 WebView2 session 创建阶段） |
| WQ-V020-TEST-01 | Regression | 整合后测试计数与失败判定复核 | 整合后 revision | 记录 `npm test`（desktop 与 extension 计数）、`cargo test --workspace`、`pytest sidecar/tests` 的实际数字与失败明细 | 计数与该 revision 的实际一致；任何 FAIL 必须定位到具体用例，不得以"Windows 环境差异"笼统带过 | P1 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-V020-MANUAL-01 | Manual | M11–M13 人工验证项 | Windows 目标机 | 按队列中既有手工步骤执行并记录证据位置 | 逐项 PASS/FAIL，附截图或日志路径 | P1 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-16 / WQ-P1-17 | Integration | 现有 `BLOCKED` 项 | 受控 Windows 环境 | 沿用既有手工步骤补齐执行入口 | **本轮不记 PASS**；保持 `BLOCKED` 并写明缺失前置 | — | `WINDOWS_BLOCKED` |

### 2026-09-21 P0 pre.7 Desktop white-screen follow-up

本轮 Linux 已完成启动诊断、fallback、前端日志桥、production dist contract、native smoke 分层证据和 release 上传前 readiness gate；真实 Windows WebView2 仍未执行。用户截图与 `v0.2.0-pre.7` 日志中的 `application runtime initialized` 只证明 native runtime 启动，不证明 document、asset、React mount 或 Dashboard readiness。

| ID | 类别 | 验证项目 | 关联修改/目标 | Windows 原因 | 前置条件 | 精确行为 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-P0-WHITE-01 | Runtime/GUI | Final ordinary `.exe` startup readiness | `desktop/src/bootstrap.js`、`desktop/index.html`、`desktop/src/main.jsx`、`dashboard.e2e.mjs`、`windows-release.yml` | WebView2 document/asset/React mount 和真实窗口生命周期不能由 Linux build 替代 | Windows 10/11、WebView2、Node/npm、Tauri driver、最终 release `.exe` | 启动同一待发布 `.exe`；采集 session、URL、readyState、`#root`、`data-xarchive-startup`、fallback、Dashboard；失败保存截图/page source/日志 | `react_mount_completed`、fallback 消失、Dashboard `h1/main/nav/summary` 可见；无持续白屏；退出无残留进程 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P0-WHITE-02 | Packaging/Regression | Final Full bundle startup parity | `windows-release.yml` Full bundle assembly、同一 embedded frontend assets | Full bundle 目录布局、资源定位和外部文件环境只能由 Windows artifact 验证 | WQ-P0-WHITE-01 通过、最终 Full `.7z`、可解压目录 | 解压 Full bundle，启动其中 `.exe`，执行同一 readiness evidence 和 30 秒观察；核对 artifact SHA-256/manifest | Full bundle 与 application-only `.exe` 均渲染 Dashboard；资源路径不依赖 Linux/CI 工作目录；无白屏 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P0-WHITE-03 | Diagnostics | Frontend/WebView2 failure evidence | `log_frontend_event`、bootstrap global handlers、WDIO evidence collection | frontend console、WebView2 resource error 和 profile/session 条件属于目标环境 | WQ-P0-WHITE-01 任一失败、独立日志目录、可访问 Windows event/driver logs | 保留 frontend startup events、Rust log、WDIO/tauri-driver/msedgedriver stderr、URL、readyState、截图、page source、artifact hash | 可区分 asset/document、entry module、React mount、IPC 或 automation/environment failure；不得只有 `application runtime initialized` | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P0-WHITE-04 | Packaging/Release | Upload gate on exact final artifact | `.github/workflows/windows-release.yml` `Collect executable` 后 readiness gate | 只有 CI Windows runner 能确认即将上传的同一 `.exe` 可用 | 修复后的 tag/source parity workflow、Windows runner、最终 `.exe` | 在归档和 GitHub Release upload 前运行 `npm run test:e2e:windows --workspace desktop`；失败收集 diagnostics 并停止后续资产步骤 | readiness 失败时不创建/上传发布资产；通过后才允许 archive、manifest 和 upload | P0 | no | `WINDOWS_VERIFICATION_PENDING` |

### 2026-09-21 Extension identity / release reliability queue（U18）

当前 canonical 开发/自托管 identity 为 `iaajefkoanbkleojofoadeakelihbjne`（由仓库外私钥对应的 public key 派生）。本队列只覆盖 Linux 无法替代的 Windows/浏览器/发布证据；Linux 侧的 identity 派生与 package parity 测试不算 Windows PASS。

| ID | 类别 | 验证项目 | 关联修改/目标 | Windows 原因 | 前置条件 | 精确行为 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-EXT-ID-01 | Browser/Runtime | Canonical Extension ID in Chrome/Edge developer mode | `extension/manifest.json` `key`、`desktop/scripts/extension-identity.mjs` | 浏览器加载 ID 由目标浏览器决定，Linux 无法替代 | 含 `key` 的 Extension 目录、Edge 和/或 Chrome | 加载 unpacked Extension；reload；重启浏览器；换解压目录 | 浏览器显示 `iaajefkoanbkleojofoadeakelihbjne`；reload/重启/换目录后不变；Chrome 与 Edge 一致或记录差异 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-REL-01 | Packaging/Release | Windows workflow tag/source parity | `.github/workflows/windows-release.yml` | 只能由真实 GitHub runner 触发，判断 checkout ref 与 asset 来源 | 修复后的 workflow、已存在 tag 与 Release | tag push run 与 `workflow_dispatch --ref <tag>` run 各执行一次 | run `headBranch`/`headSha` 等于 tag commit；不一致时打包/上传前失败，无资产 | P0 | no | `WINDOWS_PASS (pre.7 tag-push run 35570021396)` |
| WQ-REL-02 | Packaging/Release | Windows workflow tag/source parity and five-asset metadata gate | `.github/workflows/windows-release.yml`、`release-manifest-cli.mjs`、`release-assets.mjs` | 只能由真实 GitHub runner 触发，判断 checkout ref、资产来源、hash/size 和 metadata | 修复后的 workflow、已存在 tag 与 Release | tag push run 与 `workflow_dispatch --ref <tag>` run 各执行一次；检查 JSON manifest、`SHA256SUMS`、五类资产和上传顺序 | run `headBranch`/`headSha` 等于 tag commit；五类资产齐全；hash/size/source SHA/license metadata 可复核；不一致时打包/上传前失败，无资产 | P0 | no | `WINDOWS_PASS (pre.7 tag-push run 35570021396)` |
| WQ-REL-03 | Release hygiene | Contaminated `v0.2.0-pre.6` assets | GitHub Release `v0.2.0-pre.6` | 资产来源为 `main` 手动 run，无法在 Linux 判断分发影响 | 维护者权限 | 删除或明确标注 source 不匹配的 2 个资产；不移动 tag | Release 不再展示会被误当作 pre.6 tag/source 证据的资产 | P1 | no | `DONE (annotated 2026-09-21)` |

`WQ-REL-01/02` 的 pre.6 尝试记录（2026-09-21）：run `35567742785` 使用 tag source `ac586e609337947aeb51de8f5cce3185efc8995e`，但执行的是 tag 内旧 workflow；Rust check/tests 与 Tauri build 通过，随后在 Native Host 步骤因旧 workflow 只读取 `secrets.XARCHIVE_EXTENSION_ID` 而当前值位于 Repository Variable，导致 `XARCHIVE_EXTENSION_ID` 为空并失败。worker、五类资产、release manifest、`SHA256SUMS` 和上传均未执行，Release 资产保持原状。该结果应记录为旧 workflow 的 `FAIL`，不能作为当前 workflow parity 或完整发布 PASS。

`WQ-REL-03` 处理记录（2026-09-21）：已在 `v0.2.0-pre.6` Release notes 追加资产来源警告，说明两个资产来自 `main` 手动 run `35518832674`、不含 Extension/Native Host/Full 资产，且 tag-level Windows 构建为失败；**未删除**资产（其中 `.7z` 已有 1 次下载记录），如需彻底删除由维护者执行：

```bash
gh release delete-asset v0.2.0-pre.6 XArchive-v0.2.0-pre.6-windows-x64.7z --repo 15699122/Tw2Tg --yes
gh release delete-asset v0.2.0-pre.6 XArchive-v0.2.0-pre.6-windows-x64.exe --repo 15699122/Tw2Tg --yes
```

未执行或前置缺失时必须记录 `WINDOWS_BLOCKED`/`NOT RUN`；不得用 Linux 派生 ID 或 synthetic-ID package 测试替代真实浏览器 ID 证据。

Linux 侧前置状态（2026-09-21）：manifest public `key` 与 `desktop/scripts/extension-identity.mjs` 已落地并通过 Node 契约测试（desktop 55/55），`windows-release.yml` 已加入 checkout tag 绑定、tag/source parity gate、identity verify，并由 `native-host-manifest-cli.mjs` 统一生成包内 host manifest；因此 `WQ-EXT-ID-01`、`WQ-EXT-ID-02`、`WQ-REL-01`、`WQ-REL-02` 的 Linux 前置已满足，仍保持 `WINDOWS_VERIFICATION_PENDING`，等待真实 Windows/浏览器/GitHub runner 证据。

### 2026-09-20 v0.2.0-pre.5 Windows Actions build result

本次使用 `v0.2.0-pre.5` tag（commit `ea2b8d3afb289239edec29e2e00620870bed2fe6`）执行 Windows Release Build：

| 项目 | 状态 | 证据 | 结果 |
|---|---|---|---|
| Windows Release Build | `WINDOWS_FAIL` | run `35507188780` | 失败于 `Build Native Messaging Host` 步骤 |
| Windows worker/依赖下载/打包/上传 | `NOT RUN` | run `35507188780` | 失败后全部跳过 |
| Windows Release assets for `v0.2.0-pre.5` | `NOT RUN` | GitHub Release asset API | 没有生成或上传任何资产 |

分类：Windows CI 前置配置问题（缺失 `XARCHIVE_EXTENSION_ID` secret），不是代码回归。修复前置后应重新触发 Windows workflow 并核对四类资产；不得用 `v0.2.0-pre.4` 历史资产替代。

### 2026-09-20 v0.2.0-pre.6 Windows Actions build result

本次使用最终 `v0.2.0-pre.6` tag/source 执行 Windows Release Build：

| 项目 | 状态 | 证据 | 结果 |
|---|---|---|---|
| Windows Release Build（tag push） | `WINDOWS_FAIL` | run `35518801950`，tag `v0.2.0-pre.6`，source `435a9085a7d66bb12b9b515012999730080573d6` | 失败于 `Run Rust tests`；`xarchive-sidecar-supervisor` 的两个 v2 handshake tests 超时 |
| `spawn_ready_v2_completes_the_capability_handshake` | `WINDOWS_FAIL` | run `35518801950` | `sidecar v2 hello handshake timed out` |
| `spawn_ready_v2_rejects_worker_without_required_capabilities` | `WINDOWS_FAIL` | run `35518801950` | `sidecar v2 hello handshake timed out` |
| Windows Tauri/Native Host/worker/package build | `NOT RUN` | run `35518801950` | Rust test failure 后全部跳过 |
| Windows Release assets for `v0.2.0-pre.6` | `NOT RUN` | GitHub Release asset API | 没有生成或上传 `.exe`、7z、repository-dependencies 或 Full bundle |
| Manual workflow dispatch run `35518832674` | `NOT APPLICABLE` | run source 为 `main` | 虽然成功，但未使用 `v0.2.0-pre.6` tag/source，不能作为本 Release 构建证据 |

分类：Windows CI / platform-specific Sidecar supervisor test failure，当前不能记录为 `WINDOWS_BLOCKED`，也不能跳过失败测试继续上传资产。建议后续处理 `crates/xarchive-sidecar-supervisor` Windows fixture、子进程启动、stdout framing 和 v2 hello handshake timeout；修复后使用新的最终 commit 重新构建并核对四类资产。

### 2026-09-20 Browser Extension production-hardening queue

以下队列对应 `docs/development/roadmap.md` U17 的 E1–E9。E0 文档/事实对账属于 Linux development，不单独进入 Windows 队列。当前所有项目均不阻塞后续 Linux 开发；只有完成对应 Linux implementation 和 applicable verification 后，才进入集中 Windows validation phase。

| ID | 类别 | 验证项目 | 关联修改/目标 | Windows 原因 | 前置条件 | 精确行为 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-EXT-E1-01 | Integration/Regression | Browser protocol/schema parity and batch status | `xarchive-protocol::browser`、Browser schemas、Extension background、Desktop transport | 浏览器 Native Messaging payload、packaged consumer 和真实 error boundary 需目标环境确认 | E1 Linux contract tests、Extension package、Desktop artifact | 发送单条/批量 `query_status`、混合已归档/未归档、unknown field、错误 request_id | Schema/Rust/JS 一致；每个 Tweet 有明确状态；未知字段和错配 ID 被拒绝 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-EXT-E2-01 | Browser/Runtime | X DOM identity and dynamic timeline extraction | `extension/src/content-core.js`、DOM fixtures | X 页面 DOM、virtualized article 和真实 quote/reply 布局只能由浏览器确认 | Edge、受控 X 账号或可重复页面 fixture | 普通 Tweet、reply、quote、详情页、媒体 Tweet、滚动加载、节点复用 | 主 Tweet ID/URL 正确；reply_to 不指向自身；quote 与主 Tweet 分离；失败安全降级 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-EXT-E3-01 | Browser/Runtime | NativeBridge timeout, duplicate ID and reconnect | `extension/src/background.js` | MV3 Service Worker 生命周期、runtime.lastError、Native Host crash/restart 是浏览器行为 | Edge/Chrome、Native Host、Desktop transport、E3 tests | 并发/乱序、重复 request_id、超时、Host crash、Service Worker reload、重连 | pending request 有限失败；旧 port 不影响新 port；错误码可诊断；request_id 不串线 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-EXT-E4-01 | Integration | Page query_status and button state machine | `extension/src/content.js`、`content-core.js`、background、Browser response | 真实页面注入、动态 DOM 和 Desktop 状态更新需实机确认 | E1/E2/E3、Edge/Chrome、可运行 Desktop | 初次扫描、增量 Tweet、queued/running/complete/failed/disconnected、重复点击 | 状态与 Desktop 一致；不重复提交；断线可重试；不显示伪造成功 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-EXT-E5-01 | Runtime/Integration | Windows Named Pipe Desktop transport | `desktop/src-tauri/src/windows_transport.rs`、Native Host Windows client | pipe ACL 的跨用户拒绝、GUI 生命周期和 packaged Native Host/ Desktop 连接仍需实机；本机 loopback 不能证明这些场景 | Windows Full package、固定 pipe name、当前用户 | Desktop 启停、Native Host 早/晚启动、多连接、malformed request、query/archive forwarding | 合法请求到达 Desktop；非法请求安全失败；无跨用户连接；退出无残留 pipe | P0 | no | `WINDOWS_VERIFICATION_PENDING`（89 个 Windows-target library tests PASS，Named Pipe loopback 覆盖两次请求；GUI/跨用户/打包 Host 实连未执行） |
| WQ-EXT-E6-01 | Registry/Filesystem | Current-user Native Host install/repair/unregister | `windows_transport.rs`、`commands.rs`、host manifest | HKCU、浏览器注册路径和 portable move 的实时写入尚未执行 | Full package、真实 Extension ID、Edge/Chrome | inspect/install/repair/unregister；移动 portable root 后 repair；重复执行 | Chrome/Edge 状态可诊断；只写 HKCU；manifest path/origin/executable 一致；无残留失效值 | P0 | no | `WINDOWS_VERIFICATION_PENDING`（静态 Full package manifest/identity PASS；真实注册、repair、unregister 未执行） |
| WQ-EXT-E7-01 | Runtime/UI | Explicit Extension/Host/transport connection status | `ExtensionStatus`、Windows commands/UI | 浏览器加载、Host 注册、transport session 和 Service Worker 状态需实测 | E5/E6、Desktop UI、Edge/Chrome | files missing/ready、Host unregistered、browser not loaded、disconnected、connected、error | 文件、Registry、浏览器、transport 四类状态不混淆；checking 只在请求期间显示 | P0 | no | `WINDOWS_VERIFICATION_PENDING`（status implementation + Vite build PASS；native window/browser automation BLOCKED，见队列） |
| WQ-EXT-E8-01 | Packaging/Release | Extension ZIP and release parity | Extension packaging script、workflow、Native Host package、installation manifest | 真实 Windows artifact、manifest encoding、Extension ID 和发布资产需确认 | CI secret、release tag、Full/Core/Extension assets | 检查 required files、version、hash/size/license、allowed_origins、Core exclusion | ZIP/host/installation/release metadata 一致；无 tests/cache/secrets；真实 ID 不被 synthetic ID 替代 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-EXT-E9-01 | Regression | End-to-end browser archive/query/reconnect | E1–E8 所有关联模块 | 真实 Edge/Chrome、Named Pipe、Registry、Desktop executor 和 X 账号需联合验证 | 所有 E1–E8 前置 PASS、受控账号/fixture | 页面点击 archive、查询状态、Desktop restart、browser restart、Host crash、重连和重复请求 | Job 创建/复用、状态更新、错误诊断、恢复和 request_id 全链路正确 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |

如果缺少真实 Extension ID、Edge/Chrome、Registry 权限、Named Pipe server、受控 X 账号或完整 release artifact，必须将对应项目标记为 `WINDOWS_BLOCKED`、`BLOCKED_AUTOMATION` 或 `NOT RUN`，并记录精确原因；不得用 synthetic-ID package test 或 Linux Unix transport PASS 替代。

#### WQ-EXT-E4-01 BLOCKED / NOT RUN 手工验证步骤

当 Edge/Chrome、真实 Desktop/Native Host、受控 X 页面或 GUI automation 不可用时，跳过自动化并保留 `WINDOWS_BLOCKED`/`BLOCKED_AUTOMATION`/`NOT RUN`：

1. 加载当前 Extension，打开 Service Worker DevTools 和 X 页面 DevTools；记录 Extension ID、浏览器版本、Desktop/Native Host 版本和 source hash。
2. 打开包含至少 101 个可见/可扫描 Tweet article 的 timeline 或可重复 fixture；确认 `query_status` 按最多 100 个 Tweet ID 分批，重复 Tweet ID 不重复请求，且不发送 Cookie、signed URL、本地路径或媒体数据。
3. 让 Desktop 返回混合 `COMPLETE`、`QUEUED`、`DOWNLOADING`、`AUTH_REQUIRED`、`FAILED` 和 `NOT_ARCHIVED` 的 `archive_status_batch`；确认每个 article 的按钮分别显示“已归档”“排队中”“归档中”“需要登录”“重试”和“保存”。
4. 点击“保存”后确认按钮进入“提交中”，收到单条 `archive_status` 后正确映射到 queued/running/complete/failed；queued/running/complete 状态不得再次提交。
5. 新增 timeline article、触发滚动和 SPA 路由切换；确认只对新增/受影响 article 做增量查询，Extension 自己插入按钮不会触发额外状态查询。
6. 使 Native Host/Desktop 断开或返回 structured error；确认相关按钮显示“重试”，重新连接后可发起新查询/归档，失败不显示“已归档”。
7. 若浏览器、真实 Desktop/Host、受控页面或 automation target 缺失，记录请求 payload、Service Worker console、页面日志、截图和未执行原因，状态保持 `WINDOWS_BLOCKED` 或 `NOT RUN`。

#### WQ-EXT-E2-01 BLOCKED / NOT RUN 手工验证步骤

当 Windows 浏览器、受控 X 账号、真实 Extension package 或 GUI automation 不可用时，跳过自动化并保留 `WINDOWS_BLOCKED`/`BLOCKED_AUTOMATION`/`NOT RUN`，不得将 Linux fixture 测试改写为 Windows PASS：

1. 将当前 Linux source 单向同步到 Windows 工作副本，记录 branch、commit、working tree changes、Windows 版本、Edge/Chrome 版本和 Extension package SHA-256。
2. 在 Edge 中加载 Extension developer-mode package；记录实际 Extension ID、manifest 加载错误、Service Worker 状态和 content script 注入结果。若 Chrome 可用，重复执行并分别记录结果。
3. 使用受控 X 页面或脱敏可重复 fixture，分别打开 timeline、Tweet detail、reply、quote、媒体 Tweet 和包含多个 status link 的页面。
4. 对每个页面检查：主 Tweet ID 来自主 Tweet permalink；quote link 不会覆盖主 Tweet；reply_to 不会等于当前 Tweet ID；无法可靠识别 parent 时显示/传输 null，而不是错误 ID。
5. 滚动加载新 Tweet，触发 SPA 路由切换和 virtualized article 节点复用；确认新增/变化 article 能注入一次按钮，既有 article 不重复注入，旧 Tweet ID 不残留在复用节点上。
6. 使用浏览器 DevTools/Extension logs 记录解析失败、console error、按钮注入次数和 request payload；确认 payload 不包含 Cookie、浏览器 profile path 或本地文件路径。
7. 若真实 X 账号、浏览器、Extension package 或 automation target 缺失，记录缺失前置、实际命令/手工步骤、日志/截图路径和未执行场景，状态保持 `WINDOWS_BLOCKED` 或 `NOT RUN`。

#### WQ-EXT-E3-01 BLOCKED / NOT RUN 手工验证步骤

当 Edge/Chrome、真实 Native Host、Desktop transport、Service Worker 调试能力或 automation target 不可用时，跳过自动化并保留 `WINDOWS_BLOCKED`/`BLOCKED_AUTOMATION`/`NOT RUN`：

1. 在 Windows 工作副本记录 source branch、commit、working tree changes、浏览器版本、Native Host executable hash 和 Desktop artifact hash。
2. 加载 Extension，打开 Service Worker DevTools，确认 Native Messaging host manifest、Extension ID 和 `allowed_origins` 一致。
3. 发送一个不会自动返回的 `archive_request` 或 `query_status`，确认约 10 秒后收到 `NATIVE_REQUEST_TIMEOUT`，pending 数量归零；随后同一 `request_id` 可以重新使用。
4. 在第一个请求 pending 时再次发送相同 `request_id`，确认立即返回 `DUPLICATE_REQUEST_ID`，且原请求仍保持自己的 timeout/disconnect 语义，没有被替换。
5. 让 Native Host 返回带 `error_code`、`error_message`、`retryable` 的错误，确认 Service Worker 和页面侧保留结构化错误信息，而不是只显示通用字符串。
6. 创建旧 Native Messaging port，随后断开并建立新 port；从旧 port 发送 late response/disconnect，确认不会拒绝新 port 的 pending request，也不会清空新连接。
7. 制造 `postMessage` 抛错、Native Host crash、Desktop shutdown、`runtime.lastError` 和重启恢复；确认每条路径都清理 pending timer，并允许下一次请求建立新连接。
8. 若浏览器、Host、Desktop、DevTools 或 automation 前置不可用，保存 PowerShell 命令、Service Worker console、Desktop 日志、进程/PID 和截图路径，状态保持 `WINDOWS_BLOCKED` 或 `NOT RUN`。

### 2026-09-19 U8 legacy-path removal handoff

U7 Linux production wiring 已完成，U8 已删除同步 `archive_tweet`、Sidecar v1 runtime、旧 file/progress/complete events、`DownloadRouter` fallback、v1 Schema/fixtures 和 v1 PyInstaller entrypoint。当前 Desktop 业务入口只有 executor commands，当前媒体链路是 extraction-only → aria2-only transfer。以下 Windows 项目全部需要在目标环境重新确认，不得把 Linux PASS 外推为 Windows PASS。

| ID | 类别 | 验证项目 | 关联修改/目标 | Windows 原因 | 前置条件 | 精确行为 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-U7-01 | Build/Runtime | Packaged Sidecar v2 handshake and extraction | `xarchive-sidecar-supervisor` v2、`desktop/src-tauri/src/production.rs` | packaged worker、Windows stdout framing、进程启动和 artifact layout 需目标环境确认 | Windows worker artifact、Desktop artifact、项目 Python/worker 配置 | 发送 v2 `hello`；验证 capability；发送 `extract`；记录 `ready/extraction_started/extracted/failed` | v2 handshake 成功；v1 worker 被拒绝；job/request identity 正确；无旧 download event | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-U7-02 | Runtime/Integration | Extraction → plan → aria2 multi-GID transfer | `production.rs`、`xarchive-download`、`Aria2Supervisor` | `aria2c.exe`、Windows process tree、真实路径和 RPC lifecycle 不能由 Linux 外推 | Windows `aria2c.exe`、受控 media server/fixture、writable staging | 覆盖 waiting→active→complete、多媒体、progress、cancel、shutdown、timeout、error/removed | plan 顺序和 identity 保持；progress 单调；GID/aria2 进程清理；partial/`.aria2` 清理 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-U7-03 | Integration | Expired URL refresh and new GID retry | U6 refresh contract + U7 production wiring | 真实 signed URL expiry、Windows file lock 和新旧 GID lifecycle 需实机确认 | 受控 expired URL fixture、aria2c.exe、可重复 extraction fixture | 首次 transfer 403/expired；确认一次 extraction refresh、旧 GID remove、新 plan/new GID；集合变化 fixture | 仅 refresh 一次；identity/filename 集合不变才重试；集合变化返回 `EXTRACTION_RESULT_CHANGED`；普通错误不 refresh | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-U7-04 | Filesystem/Integration | Staging verification and ArchiveService commit | `ArchiveService`、`FileStore`、`production.rs` | Windows rename/file lock/reparse/path semantics 需目标文件系统确认 | Desktop artifact、writable archive/cache/staging、可制造 lock/reparse 的 fixture | 检查 path escape、symlink/junction、缺失/多余文件、size/hash、staging→final commit | 只提交经过验证的文件；路径逃逸/reparse/缺失文件失败；SQLite、metadata、media、Job state/event 一致 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-U7-05 | Runtime/Regression | Executor cancellation, shutdown, recovery and late-result fencing | `executor.rs`、`archive.rs`、`production.rs` | Windows Sidecar/aria2 子进程、句柄、restart 和 lock 行为需实机确认 | Desktop artifact、Sidecar/aria2 fixture、可控 SQLite/archive root | submit/query/cancel/shutdown；在 extraction/transfer/staging 阶段关闭并重启；制造 late result | `CANCELLED`/`INTERRUPTED` 不被 late result 覆盖；恢复读取 execution spec；不重复归档；无残留进程 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |

如果 WDIO/WebView2、artifact、aria2c.exe、signed URL fixture 或账号前置不可用，按 `BLOCKED_AUTOMATION` / `WINDOWS_BLOCKED` / `NOT RUN` 跳过，并执行 `docs/development/windows-validation.md` 中的 U7 手工步骤。

### 2026-09-18 Overall extraction/aria2/bootstrap architecture handoff

下列项目来自已冻结的总体 Plan。U3–U13 尚未实现的部分使用 `NOT RUN` 并标记为 `PLANNED`；只有对应 Linux 功能完成、artifact 可获得后，才转入 `WINDOWS_VERIFICATION_PENDING`。Sidecar v1、gallery-dl media download、`DownloadRouter` fallback 和 `archive_tweet` 的描述属于历史验证事实；它们已在 U8 当前 revision 删除，不得将历史记录解释为当前可运行路径或新的 Windows PASS。

U2 Linux follow-up：Rust Sidecar Supervisor 已在 Unix 上使用独立 process group，并在 shutdown/force cleanup 时发送组级终止信号；Windows Job Object 尚未实现或验证。以下 Windows 项目保持 pending，若自动化或目标 artifact 被阻塞，必须按手工步骤记录 `BLOCKED`/`NOT RUN`。

U3 Linux follow-up：Sidecar protocol v2 contract 已在 Linux 落地（Rust `sidecar_v2` 模型、Python `protocol_v2`/`worker_v2`/`extraction`、Schema 与 fixtures，Rust protocol 15/15 通过）。但 Supervisor 尚未 spawn v2 worker，Desktop 尚未消费 v2 事件，extraction-only 和 aria2 transfer 尚未实现，因此 WQ-ARCH-01 的 Windows 端到端验证仍无法运行，状态保持 `NOT RUN — PLANNED`；待 U7 完成运行时接线后转入 `WINDOWS_VERIFICATION_PENDING`。

U4 Linux follow-up：v2 extraction-only 适配层已在 Linux 落地（强制 `--skip-download`、防御性拒绝媒体写入 flag、`sanitize_filename` 净化、`stable_media_id` 三级 identity、result 序列化剥离 `raw` 且不携带下载事实，Sidecar pytest 28/28 通过）。但 v2 worker 仍未被 Supervisor spawn，v1 媒体下载链路仍是 MIGRATION 残留，无法在 Windows 端到端确认 extraction-only 行为，WQ-ARCH-02 状态保持 `NOT RUN — PLANNED`；待 U7 运行时接线完成且 v2 worker artifact 可获得后转入 `WINDOWS_VERIFICATION_PENDING`。手工验证步骤（若届时 Windows 自动化被阻塞）：在 Windows 工作副本用 packaged worker 发送 v2 `extract`（单图/视频/多媒体 fixture），确认 stdout JSONL 只有 typed extraction result、工作目录与 staging 无媒体主体文件、media identity 与顺序稳定、signed URL/header 不落库不进普通日志，并将结果如实记为 PASS/FAIL/BLOCKED。

U5 Linux follow-up：aria2-only `MediaTransferPlan` 与 transfer driver 已完成 Linux contract/integration 验证（`xarchive-download` 20 unit + 7 integration tests，workspace test/clippy/fmt 通过），但尚未接入 Supervisor/Desktop production chain，U6 refresh 也尚未实现。因此 WQ-ARCH-03 仍为 `NOT RUN — PLANNED`，不能把 driver contract 测试当作 Windows aria2/runtime PASS。待 U6/U7 接线并生成 Windows artifact 后，转为 `WINDOWS_VERIFICATION_PENDING`。

U6 Linux follow-up：refresh contract 已完成 Linux 验证（401/403/expired/signature/access-denied 分类、一次性完整 extraction refresh、stable media identity + filename 集合匹配、`EXTRACTION_RESULT_CHANGED`、非 URL/本地错误不 refresh）。但它尚未接入 Desktop/Supervisor production chain，无法在 Windows 端确认旧 GID 移除、新 GID 创建、真实 signed URL expiry、Windows file lock 和 restart recovery；WQ-ARCH-03 继续保持 `NOT RUN — PLANNED`，待 U7 artifact 可获得后转入 `WINDOWS_VERIFICATION_PENDING`。

| ID | 类别 | 验证项目 | 关联修改/目标 | Windows 原因 | 前置条件 | 精确行为 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-ARCH-01 | Planned handoff | Sidecar v2 handshake、capability 与 v1 rejection | U3/U7、`xarchive-protocol`、Sidecar、Supervisor、Desktop | packaged worker、stdout framing 和实际 artifact 只能在 Windows runtime 确认 | v2 worker artifact、Desktop artifact、valid/invalid fixtures | 发送 `hello`；验证 capability；发送 v1/unknown field | v2 ready 正确；v1、unknown command/field、缺失 capability 明确失败；无旧 download event | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-ARCH-02 | Planned handoff | extraction-only 无媒体主体文件 | U4/U7、gallery-dl extractor、typed ExtractionResult | Windows worker、真实 Edge Cookie、临时目录和 packaged gallery-dl 行为需实机确认 | v2 worker、固定 gallery-dl、受控 X fixture/账号、空 staging | 执行单图、视频、多媒体 extraction；检查输出、临时 workspace 和 JSONL | 只产生 typed ExtractionResult；不产生媒体主体文件；stable identity/order 正确；signed URL/header 不落库或进普通日志 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-ARCH-03 | Planned handoff | aria2-only transfer、refresh 与 `.aria2` cleanup | U5–U7、`xarchive-download`、executor、ArchiveService | aria2c.exe、Windows process/file lock、真实 signed URL expiry 不能由 Linux 外推 | aria2 artifact、fake/media server 或受控账号、Desktop artifact | 覆盖 waiting→active→complete、multi-GID、403 refresh、cancel/shutdown、失败和重启 | 旧 GID 被移除；新 extraction 只创建新 GID；progress 单调；`.aria2`/不完整文件清理；Job 状态与最终 commit 一致 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |

#### U6 BLOCKED / 手工验证步骤

如果 Windows 自动化、Desktop artifact、aria2c.exe、受控 signed URL fixture 或 WebView2 session 不可用，跳过自动化并记录 `WINDOWS_BLOCKED`/`BLOCKED_AUTOMATION`，不得记为 PASS：

1. 使用同一 Linux working tree 对应的 Windows 工作副本，记录 branch、commit、working tree changes、Windows 版本、aria2c 版本和 artifact hash。
2. 启动 loopback aria2 RPC 与 fake media server，准备同一 stable media identity 的旧 URL 与刷新后的新 URL。
3. 首次 transfer 返回 HTTP 403/expired signature；确认旧 GID 被 remove，随后只发生一次 extraction refresh 和一次新 GID 提交。
4. 验证新 URL 下载成功、最终文件名/identity 不变、旧 `.aria2`/partial 文件被清理。
5. 让 refresh 返回新增、删除或重排的 media identity；确认结果为 `EXTRACTION_RESULT_CHANGED`，不得静默覆盖或重复归档。
6. 分别制造 404、磁盘满、无权限、用户 cancel、应用 shutdown；确认这些路径不会触发 refresh，并分别记录 `TRANSFER_FAILED`、`CANCELLED` 或 `INTERRUPTED`。
7. 如果任一前置条件缺失，保存 PowerShell 命令、日志、截图、进程/PID 和 artifact 路径，并将项目标记为 BLOCKED/NOT RUN。
| WQ-ARCH-04 | Planned handoff | Core Bootstrap 缺组件启动、安装、校验和 rollback | U9–U10、ComponentManager、Setup Wizard | Windows executable、safe extract、atomic activation、权限和 WebView2 需目标环境确认 | 单文件 Core EXE、embedded catalog、损坏/篡改/断网 fixture | 无组件启动设置页；安装 Worker/gallery-dl/aria2；测试 hash mismatch、corrupt ZIP、interrupted install、external path、rollback | 应用可启动但归档入口禁用；错误可诊断；只激活已验证版本；失败不破坏旧版本 | P0 | no | `NOT RUN — PLANNED` |
| WQ-ARCH-05 | Planned handoff | Release asset、embedded catalog 和 Offline Bundle parity | U11–U13、release workflow、component catalog | Windows one-dir、Native Host、Extension ZIP 和最终资产布局需发布环境确认 | draft release assets、SHA256SUMS、licenses、Offline Bundle | 构建 Worker、Native Host、Extension、gallery-dl/aria2 specs、Core 和 Offline Bundle；比较 embedded/external catalog | 资产名称/版本/hash/布局一致；无 tests、cache、credentials；license/notices 完整；不得覆盖已发布资产 | P1 | no | `NOT RUN — PLANNED` |
| WQ-ARCH-06 | Planned handoff | Native Host、Extension developer-mode load 和状态枚举 | U12、Native Host、Extension、Desktop status | Named Pipe/Registry/ACL、浏览器扩展加载和 reconnect 是 Windows/browser 行为 | Native Host ZIP、Extension ZIP、固定 Extension ID、Edge/Chrome | 安装/导入 Host；加载解压扩展；删除/恢复文件；重启浏览器和 Desktop | `MISSING`/`FILES_READY`/`BROWSER_NOT_LOADED`/`NATIVE_HOST_NOT_REGISTERED`/`DISCONNECTED`/`CONNECTED` 不混淆；request_id 路由正确 | P1 | no | `NOT RUN — PLANNED` |

### 2026-09-20 U12 Linux handoff

U12 的 Linux 范围已完成：`native-host-package.mjs` 只负责可审计的 Native Host/Extension 发布契约，不修改 Windows Registry、不连接浏览器、不伪造固定 Extension ID，也不实现 Windows Named Pipe。Desktop status 已明确区分 Extension 文件存在、浏览器加载、Native Host 注册和连接状态。

| ID | 类别 | 验证项目 | 关联修改 | Windows 原因 | 前置条件 | 精确步骤 | 预期结果 | 优先级 | 状态 |
|---|---|---|---|---|---|---|---|---|---|
| WQ-U12-01 | Packaging | Native Host/Extension installation manifest | `desktop/scripts/native-host-package.mjs` | 实际 host manifest 路径、Registry registration、ACL 和浏览器允许来源是 Windows 行为 | `v0.2.0-pre.2` 或更新 Release、Native Host `.exe`、Extension ID/发布密钥 | 生成并检查 `package-manifest.json`、`com.tw2tg.xarchive.json`；核对 `allowed_origins`、host executable 绝对路径和包内相对路径 | manifest schema、Native Host name、Extension ID、allowed origin、文件列表一致；无路径逃逸 | P0 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-U12-02 | Runtime/Integration | Native Host Registry/ACL registration | `crates/xarchive-native-host`、U12 installation manifest | Registry hive、用户/管理员安装权限、Named Pipe/stdio 进程启动和 ACL 只能由 Windows 确认 | Windows 用户账户、Native Host `.exe`、固定 Extension ID、host manifest | 按用户级方式注册 host；启动 Edge/Chrome；从 Extension 发起 `query_status` 和 `archive_request`；卸载后重复请求 | 合法请求经 Native Host 转发；无权限时明确错误；卸载后不残留 host；无错误连接到其他用户实例 | P0 | `WINDOWS_BLOCKED` — 本轮无 Windows Registry/ACL/浏览器环境；执行下方手工步骤 |
| WQ-U12-03 | Browser | Edge/Chrome developer-mode load and reload | `extension/manifest.json`、`extension/src/background.js` | Service Worker 生命周期、浏览器 ID 和 developer-mode UI 是 Windows/browser 行为 | Edge/Chrome、Extension 目录、固定 Extension ID 或开发者模式实际 ID | 解压/加载 Extension；确认 manifest、background、content scripts；刷新 Service Worker；删除/恢复 `manifest.json` 或 `background.js` | 缺文件明确失败；恢复后可重新加载；权限仅为 nativeMessaging/storage 和 X/Twitter host permissions | P1 | `WINDOWS_BLOCKED` — 本轮无浏览器实机；执行下方手工步骤 |
| WQ-U12-04 | Runtime/Regression | Extension/Native Host reconnect and status enum | Desktop `ExtensionStatus`、Extension `NativeBridge` | 浏览器重启、Service Worker 重启、Native Host 断开/重连和 Windows endpoint 行为需实机 | WQ-U12-02/03 PASS、Desktop Release、可观察日志 | 依次测试 files missing、files ready、browser not loaded、host not registered、disconnect、reconnect、connected；重复 request_id 和并发请求 | UI 不把 files-ready 显示为 connected；pending requests 在断开时拒绝；重连后新请求可用；request_id 不串线 | P0 | `WINDOWS_BLOCKED` — 依赖 WQ-U12-02/03；执行下方手工步骤 |

#### WQ-U12 BLOCKED 手工验证步骤

1. 解压 `XArchive-v0.2.0-pre.2-windows-x64-repository-dependencies.7z` 到全新目录；记录 Release tag、Windows 版本、Edge/Chrome 版本和目录 SHA-256。
2. 准备固定 Extension ID；如果使用未打包开发者模式，记录浏览器实际生成的 ID，不得把临时 ID 写回仓库或 embedded catalog。
3. 检查 `extension/manifest.json` 为 MV3，包含 `nativeMessaging`、`storage`、`https://x.com/*`、`https://twitter.com/*`，service worker 为 `src/background.js`。
4. 生成 `native-host/com.tw2tg.xarchive.json`，确认 `name`、`type=stdio`、`path` 和 `allowed_origins` 与 manifest ID 一致；只注册到当前 Windows 用户，不使用未知管理员权限覆盖。
5. 在 Edge 和 Chrome 分别打开扩展开发者模式并加载 Extension 目录；记录加载错误、Service Worker 状态和扩展 ID。
6. 启动 XArchive Desktop，确认设置页状态依次能区分：文件缺失、文件已就绪但浏览器未加载、Native Host 未注册、断开、已连接；文件存在本身不得显示“已连接”。
7. 从 Extension 发起 `query_status` 和受控 `archive_request`；确认 response 的 `request_id` 与请求一致。关闭 Native Host/桌面后确认 pending request 明确失败，重新启动后新请求恢复。
8. 删除并恢复 `manifest.json`、`background.js`、`content.js`，分别确认浏览器和 Desktop 的错误提示；卸载 host 后确认 Registry/host manifest 不再使连接成功。
9. 若 WebView2、Registry、浏览器、账号或 Native Host 前置不可用，记录 PowerShell 命令、日志、截图、进程/PID 和原因，保持 `WINDOWS_BLOCKED`/`NOT RUN`，不得改写为 PASS。

### 2026-09-20 pre-release UI / Extension / Native Host 修复批次

本批次基于 Linux source branch `feature/u7-desktop-production-integration`、working tree changes（Plan 文档和业务实现均未提交）的实施结果。当前 Linux 已验证：`get_extension_status` 的旧 `not_loaded → 检测中…` UI 契约已修正；Full portable/release packaging 已加入 Native Host executable、manifest、Extension ID secret 校验和 `allowed_origins` 生成；Extension Bridge 10/10、Desktop Node 46/46、Native Host Rust 8/8 和 Vite build 通过。以下 Windows 项目仍未执行，不能由 Linux 结果外推为 PASS。

| ID | 类别 | 验证项目 | 关联修改 | Windows 原因 | 前置条件 | 精确行为 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-UI-20260920-01 | GUI/DPI | Homepage spacing and setup layout | `dashboard-page.jsx`、`style.css` | WebView2 font metrics、window size 和 DPI 不能由静态 CSS 完全判断 | Windows Desktop build、100/125/150% DPI | 打开 Dashboard，检查首次使用说明、下载目录区块、运行环境卡、侧栏设置间距 | 无异常大块留白；说明层级一致；按钮可见且可点击；响应式布局不溢出 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-UI-20260920-02 | GUI/DPI | Settings divider layout and globe icon | `settings-page.jsx`、`icon.jsx`、`style.css` | SVG/WebView2 rasterization 和 DPI 裁切需实机确认 | Windows Desktop build、100/125/150% DPI | 打开 Settings，检查 section 分隔线、图标边界、焦点环和滚动 | 设置 section 无重复外框；globe 完整不裁切；键盘焦点可见 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-UI-20260920-03 | GUI/Interaction | Copyable path controls | `copyable-path.jsx`、`settings-page.jsx`、`main.jsx` | Clipboard/WebView2、中文/空格/长路径和键盘事件需实机确认 | 可写和只读目录、中文/空格路径 | 点击、Enter、Space 复制归档/日志/aria2/gallery-dl/Extension 路径 | 剪贴板内容完整；路径截断但 title 完整；成功反馈只出现在对应控件；错误可诊断 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-EXT-20260920-01 | Runtime/UI | Extension status refresh and explicit states | `commands.rs`、`main.jsx`、`ui-state.js`、`connection-status.jsx` | 浏览器加载、Registry、Native Host 和 transport session 只能在 Windows 实际观察 | Desktop build、Edge/Chrome、Extension directory | 依次验证 files missing、files ready、refresh/checking、host not registered、browser disconnected、connected、error | checking 只存在于请求期间；文件存在不误报 connected；刷新后显示明确最终状态 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-NATIVE-20260920-01 | Packaging | Native Host artifact and manifest parity | `build-portable-windows.mjs`、`portable-package.mjs`、`native-host-package.mjs`、release workflow | Windows `.exe`、absolute path、manifest encoding 和真实 bundle 内容不能由 Linux 产物替代 | Windows release run with `XARCHIVE_EXTENSION_ID` secret、7z/manifest tools | 解压 Full/Repository dependencies，检查 host executable、manifest、installation manifest、license/source files；确认 Core 不含 Native Host | `com.tw2tg.xarchive`、host path、Extension ID、`allowed_origins`、file list 和 package manifest 完全一致；无路径逃逸 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-NATIVE-20260920-02 | Registry/Runtime | User-level Native Host registration and repair | 新增 Windows platform command/adapter（待实现）、`commands.rs` | Registry hive、权限、manifest path、Edge/Chrome key 名称是 Windows 行为 | 当前 Windows 用户、host manifest/exe、Edge/Chrome | 安装、刷新、修复、移动 portable root 后再次修复、取消注册；读取 Registry 和 manifest | 当前用户可用；两浏览器注册状态可诊断；移动后不保留失效绝对路径；取消注册无残留；不要求未知管理员权限 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-NATIVE-20260920-03 | Browser/Integration | NativeBridge error and reconnect | `extension/src/background.js`、Native Host、transport | `runtime.lastError`、Service Worker restart、Named Pipe/stdio lifecycle 需浏览器实机确认 | WQ-NATIVE-20260920-02 PASS、Edge/Chrome、Desktop running | 关闭 Host/Desktop、发起 pending request、重启 Service Worker/Desktop、重复 request_id 和并发请求 | pending request 明确失败；下一次请求创建新 port；重连后 request_id 不串线；X 页面能显示可操作错误 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |

本批次没有 `WINDOWS_VERIFICATION_BLOCKING` 项。Windows 阶段必须基于最终 Linux diff 单向同步到 E: 工作副本；如缺少浏览器、Registry 权限、固定 Extension ID、真实 artifact 或 GUI automation，按 `WINDOWS_BLOCKED`/`BLOCKED_AUTOMATION`/`NOT RUN` 记录，不得把静态测试结果改写为 PASS。

### 2026-09-20 U13 Offline Bundle Linux handoff

U13 的 Linux 范围已完成：`offline-bundle-package.mjs` 只定义 Offline Bundle 的组件完整性、路径安全、hash/size/license 元数据和 Release/catalog parity；不下载、签名、解压真实 Windows artifact，不写入 embedded catalog，也不创建 `config/`、`cache/`、`download/`、`logs/` 等运行时目录。

| ID | 类别 | 验证项目 | 关联修改 | Windows 原因 | 前置条件 | 精确步骤 | 预期结果 | 优先级 | 状态 |
|---|---|---|---|---|---|---|---|---|---|
| WQ-U13-01 | Packaging/Parity | Offline Bundle component completeness | `desktop/scripts/offline-bundle-package.mjs` | Windows `.exe`、one-dir worker、Native Host、Extension、gallery-dl、aria2 和实际目录只能由 Windows artifact 确认 | U11 Release assets、U12 Native Host/Extension package、固定 component catalog | 组装 Offline Bundle；检查 6 个组件、required_files、license_files、manifest、catalog 和实际目录 | 组件无缺失/重复；Core/Offline 边界正确；无未知文件、cache、credential 或运行时目录 | P0 | `WINDOWS_BLOCKED` — 本轮没有可安全在 Linux 生成的完整 Windows component set；执行手工步骤 |
| WQ-U13-02 | Packaging/Security | Offline Bundle extraction/path/license | U13 manifest contract、ComponentManager、release workflow | Windows 7z/ZIP 解压、ACL、reparse、可执行 probe、许可证扫描和签名只能在目标环境确认 | 完整 Offline Bundle、7z/ZIP 工具、license/source notices、测试目录 | 解压到全新目录；检查路径逃逸、绝对路径、junction/reparse、runtime 目录、license 和 source notices；执行 hash/probe | 只创建预期包内目录；拒绝路径逃逸/reparse；hash/size/license/catalog 一致；运行时目录由首次启动创建 | P0 | `WINDOWS_BLOCKED` — 当前无真实 Offline Bundle/Windows filesystem；执行手工步骤 |
| WQ-U13-03 | Runtime | Offline Bundle startup/bootstrap parity | Core Bootstrap、ComponentManager、portable runtime | WebView2、Windows executable probe、Bootstrap UI、activation marker 和权限行为需实机 | 解压后的 Offline Bundle、全新 portable root、WebView2 | 启动 Desktop；查看 Bootstrap/catalog 状态；确认组件版本/hash 与 bundle manifest；执行首次 setup；重启并验证 active markers | Bundle 可启动；catalog/manifest/实际组件一致；setup 不覆盖旧版本；失败可诊断并可 rollback；不执行动态 latest | P0 | `WINDOWS_BLOCKED` — 依赖 Windows WebView2、真实 catalog 和 artifact；执行手工步骤 |
| WQ-U13-04 | Packaging/Release | Offline Bundle signature and release upload | U11 workflow、U13 manifest、Release assets | Windows 签名工具、证书、SHA256SUMS、GitHub assets 和最终发布权限不在 Linux | 签名证书、最终 Release、完整资产、SBOM/license 扫描结果 | 对 bundle/exe 签名；校验签名、SHA256SUMS、manifest/catalog；上传并下载回归；比较下载文件 hash | 签名有效；上传/下载不改变 hash；Release、manifest、catalog、bundle parity 一致；失败不替换已发布资产 | P1 | `WINDOWS_BLOCKED` — 本轮无签名证书和最终 Offline Bundle；执行手工步骤 |

#### WQ-U13 BLOCKED 手工验证步骤

1. 在 Windows 工作副本准备同一 release tag 的 Desktop `.exe`、worker、Native Host、Extension、gallery-dl、aria2 和 U13 manifest；记录每个文件 SHA-256、size、version 和 license/source notice。
2. 使用当前 `v0.2.0-pre.2` 或更新 release 资产组装 Offline Bundle；组件必须分别落在 `components/<id>/` 或 manifest 声明的固定相对路径，禁止使用绝对路径和 `..`。
3. 解压到全新目录，检查不存在预创建的 `config/`、`cache/`、`download/`、`logs/`；检查不存在 `.git`、node_modules、Python virtualenv、缓存、凭据和未知文件。
4. 比较 `offline-bundle-manifest.json`、`release-manifest.json`、embedded catalog 和实际目录：组件 ID、版本、artifact、SHA-256、size、required_files、license_files、catalog_version 必须完全一致。
5. 检查每个 component 的 required files 和 license files；执行 gallery-dl/aria2/worker/Native Host probe；probe 失败或版本不匹配时不得激活。
6. 启动 Offline Bundle，验证 Bootstrap UI、首次 setup、active marker、组件缺失诊断、失败恢复和 rollback；确认不发生动态 `latest` 下载。
7. 若证书、Windows WebView2、真实组件、浏览器或签名工具缺失，记录命令、日志、hash、截图和原因，保持 `WINDOWS_BLOCKED`/`NOT RUN`，不得标记为 PASS。

### 2026-09-20 U14 Linux full verification handoff

U14 Linux applicable verification 已完成。验证对象为 `feature/u7-desktop-production-integration` / HEAD `f2ae58d`，working tree dirty，包含未提交的 U12/U13 修改；以下结果不能外推为 Windows PASS。

| 类别 | Linux 结果 | Windows 仍需验证 |
|---|---|---|
| Build/Toolchain | Rust fmt/check/clippy、Node check/build、Python compileall PASS | MSVC/Windows SDK/Tauri Windows build、WebView2 runtime |
| Runtime | Rust workspace tests PASS；Desktop Rust 86/86；Linux Tauri/WDIO smoke 2/2 | Windows process tree、Job Object、Named Pipe、WebView2、Native Host |
| Integration | Sidecar pytest 21/21；Node Desktop 44/44；Extension 7/7 | aria2c.exe、真实 extraction/transfer/commit、Edge/Chrome、真实 X account |
| Packaging | U11/U12/U13 manifest/portable contracts PASS | Windows bundle assembly、7z/ZIP extraction、catalog/assets parity、signature、SHA256SUMS、license scan |
| Regression/Hygiene | `git diff --check` PASS；working tree 状态已记录 | Windows revalidation must use the final committed revision, not this dirty working tree |

U14 阶段没有 `WINDOWS_VERIFICATION_BLOCKING` 项目。所有 Windows-only 项目保持 `WINDOWS_VERIFICATION_PENDING` 或 `WINDOWS_BLOCKED`；执行顺序统一为 Build/Runtime/Filesystem/Integration/Packaging/Regression。

#### U14 BLOCKED Windows 手工验证步骤

1. 将最终提交后的 Linux source 单向同步到 Windows 工作副本，记录 branch、commit、working tree 状态、Windows 版本、架构、WebView2、Edge/Chrome、Rust、Node、Python 和 artifact SHA-256。
2. 执行 Windows release build、worker build、U12 Native Host/Extension manifest 检查和 U13 Offline Bundle assembly；不要把当前 dirty working tree 直接当作最终发布验证源。
3. 在全新目录解压 Core/Full/Offline Bundle，确认不存在 `config/`、`cache/`、`download/`、`logs/`、`.git`、node_modules、virtualenv、凭据和未知文件。
4. 执行组件 required_files/license_files/probe/hash/size 校验，比较 Release manifest、Offline Bundle manifest、embedded catalog 和实际目录。
5. 手工验证 Core Bootstrap、首次 Setup Wizard、WebView2 设置页、Extension developer-mode load、Native Host registration、Named Pipe/ACL、aria2、Sidecar v2、真实 transfer、staging/commit、restart/recovery 和 signature/upload。
6. 若缺少 Windows、WebView2、证书、浏览器、账号、真实组件或自动化 driver，跳过对应自动化并记录 `WINDOWS_BLOCKED`、`BLOCKED_AUTOMATION` 或 `NOT RUN`；保存 PowerShell 命令、日志、截图、PID、artifact 路径和原因，不得改写为 PASS。

### 2026-09-26 工程审查整改 Windows 队列

来源为 [`../review/engineering-audit-2026-09-26.md`](../review/engineering-audit-2026-09-26.md)（只读审查，未修改产品代码）。下列项目在对应 Linux 修复完成并同步到 Windows 工作副本前保持 `WINDOWS_VERIFICATION_PENDING`；本轮没有 Windows 环境，全部为 `NOT RUN`。

| ID | 类别 | 验证项目 | 关联修改 | Windows 原因 | 精确手工步骤 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|
| WQ-ENG-01 | Build | P0 打包输出目录删除保护 | `desktop/scripts/build-portable-windows.mjs` | 路径越界、junction 与删除语义只能在 Windows 确认 | 在临时目录设置 `PORTABLE_OUTPUT_DIR` 指向项目根、其父目录、用户目录与含 junction 的目录，分别执行 `npm run build:portable:windows --workspace desktop` | 全部越界输入在删除前失败并给出明确原因；合法输出目录正常生成 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-ENG-02 | Build/Toolchain | 当前分支 Windows 编译与发布构建 | `desktop/src-tauri/src/transport.rs` 条件编译导入 | 条件编译差异只能在 Windows toolchain 复现 | 同步修复后源码，执行 `cargo check --workspace --locked`、`cargo test --workspace --no-fail-fast` 与 `npm run build:tauri --workspace desktop` | 无未使用/未定义编译错误；测试全通过；release `.exe` 生成 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-ENG-03 | Filesystem | 归档目录中间 junction/reparse 越界 | `crates/xarchive-storage` FileStore/metadata | reparse point 与目录句柄语义是平台行为 | 在归档根下创建指向根外的 junction 作为中间目录，分别执行归档提交、metadata 校验与恢复扫描 | 越界路径被拒绝且不写入根外；现有 symlink/reparse 拒绝测试仍通过 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-ENG-04 | Runtime | IPC 连接上限与读期限 | `desktop/src-tauri/src/transport.rs` | 连接与超时行为需真实运行环境 | 启动应用后并发建立大量不完整连接、保持半帧并超时，观察线程、内存与恢复 | 连接被限制或按期限关闭；应用保持可用；无持续资源增长 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-ENG-05 | Process | Sidecar/aria2 输出上限与 PowerShell 解压 | `xarchive-sidecar-supervisor/readers.rs`、`desktop/src-tauri/src/aria2.rs` | 子进程输出、超长行与 PowerShell 展开需实机 | 用产生超长单行与大量 stderr 的假 worker 运行归档；用含空格与中文的路径执行 aria2 下载/解压 | 输出被限长并产生稳定错误码；aria2 在含空格/中文路径下正确展开与执行 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-ENG-06 | Integration | 真实账号错误脱敏与凭据边界 | `sidecar/errors.py`、`storage/database/jobs.rs`、日志与 UI | 真实 gallery-dl 错误内容只能在 Windows 复现 | 用受控账号触发 AUTH_REQUIRED、限流与网络失败，检查日志页、任务错误、SQLite `last_error_message` 与剪贴板导出 | 错误中无 Cookie、Token、Authorization 头与凭据 URL；保留稳定错误码与可诊断信息 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-ENG-07 | Packaging | 发布 provenance 与包内容清单 | `windows-release.yml`、`build-portable-windows.mjs` | 产物与 tag 绑定、真实包内容需在 Windows 生成后检查 | 手动 dispatch 传入与 checkout ref 不一致的 tag；生成 Full/Core 包并在组件目录放置无敏感哨兵 `.env`/测试文件 | 不一致发布被拒绝；包内文件清单符合允许列表；哨兵文件不进入产物 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-ENG-08 | Regression | 审查整改后全量 Windows 回归 | R7 P0/P1 全部修复 | 跨模块改动的平台回归只能在 Windows 完成 | 在最终 diff 上执行 Node workspace check/test/build、Sidecar compileall/pytest、Rust fmt/check/test/strict Clippy、Tauri release build | 全部 PASS；任何 FAIL/BLOCKED/NOT RUN 记录原因，不得记为 PASS | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-ENG-09 | Integration | Node 测试工具链 override 后的 Tauri 原生 E2E | 根 `package.json` overrides、桌面测试工具链 | `extract-zip` 公告无修复版本 | 重新安装依赖后检查 `@wdio/tauri-service` 与 WDIO 版本、锁定 `msedgedriver.exe` 路径与 SHA-256，运行 `npm ci`、`npm run check`/`test`、`npm run build:tauri:wdio --workspace desktop` 和 `npm run test:e2e:windows:advanced --workspace desktop` | 普通与 advanced E2E 均通过，报告与失败输出无序列化异常；EdgeDriver 取得方式符合预期；退出后无遗留 `tauri-driver`/`msedgedriver` 进程与 4444/4445 端口占用 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-ENG-10 | Build/Toolchain | 固定 Rust toolchain 与 Action SHA 后的 Windows 构建 | 新增 `rust-toolchain.toml`、`.github/workflows/*.yml` Action 固定 | toolchain channel 解析与 Action 解析只在 Windows/CI 复现 | 在 Windows 工作副本确认 `rustc --version` 与 `rust-toolchain.toml` 一致；执行 `cargo check --workspace --locked`、`cargo test --workspace --locked`、`npm run build:tauri --workspace desktop`；触发两个 workflow 确认 Action 按 SHA 解析且 worker artifact 生成 | `rustc` 版本与 pin 一致；构建与测试通过；workflow 成功；无 Action 版本漂移告警 | P2 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-ENG-11 | Build | Executor 模块拆分后的 Windows 编译与回归 | `desktop/src-tauri/src/executor/*` 拆分 | 拆分是跨境平台构建的编译面变化，需在 MSVC 下确认 | 同步拆分后源码，执行 `cargo check --workspace --locked --all-targets`、`cargo test --workspace --locked`、`cargo clippy --workspace --all-targets --locked -- -D warnings` 与 `npm run build:tauri --workspace desktop` | 无编译/Clippy 错误；测试通过；release `.exe` 生成；`cargo --locked` 不需要改动 | P2 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-ENG-12 | Process/Security | aria2 RPC secret 传递方式与 Windows 进程可见性 | `crates/xarchive-download/src/supervisor.rs`（ENG-13 已改走短期 `--conf-path` 文件，不再进入 argv） | Windows temp 文件 ACL 与真实 aria2c.exe `--conf-path` 行为只能实机确认 | 同步最新源码；在 Windows 用受控随机 secret 启动真实 `aria2c.exe`，通过任务管理器/`wmic process get CommandLine` 确认命令行无 secret；检查 temp 目录 secret 文件权限与进程退出后删除；执行受控下载验证功能 | 命令行无 secret；secret 文件仅调用方 temp ACL 可见且随进程退出删除；诊断/日志不含完整 secret；真实下载通过 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |

本轮没有 `WINDOWS_VERIFICATION_BLOCKING`。Linux 侧修复与回归完成后仅重验命中对应 diff 的条目，不重复无交集的历史项目。


### 2026-09-17 GUI/metrics/logging batch handoff

本轮 Linux 已完成 Dashboard 全量 JobMetrics、日志五档统一、固定主内容滚动边界、服务状态跳转、Tauri 原生 executable picker 和 GitHub Extension 外链。当前没有 Windows 环境，因此下列项目只进入集中式手工验证队列；自动化无法建立 native WebView2 session 时必须标记 `BLOCKED_AUTOMATION`，不得记为 PASS。

| ID | 类别 | 验证项目 | 关联修改 | Windows 原因 | 精确手工步骤 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|
| GUI-W-METRICS-07 | Runtime/GUI | Dashboard 全量任务统计 | `xarchive-storage/src/database/jobs.rs`、`commands.rs`、`desktop/src/main.jsx`、`dashboard-page.jsx` | 需要真实 Tauri/SQLite 启动和 WebView2 渲染 | 启动 portable artifact；准备至少 4 个任务，覆盖 active/complete/failed；刷新工作台并与 SQLite 记录核对 | 五项指标来自全库，不能因最近列表限制为 20 条；状态数量准确 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| GUI-W-LOG-08 | Runtime/GUI | 日志五档过滤与内部滚动 | `desktop/src/lib/log-lines.js`、`logs-page.jsx`、`style.css`、logging commands | 文件轮转、WebView2 select/input/scroll 行为需 Windows 确认 | 在设置页依次保存 Error/Warning/Info/Debug/Silent；打开运行日志，筛选、搜索、滚动、自动跟随、复制和打开目录 | 不显示 Trace；Warning 使用统一名称；Silent 不显示日志；滚动条位于日志框内部 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| GUI-W-DIALOG-09 | Filesystem/GUI | gallery-dl/aria2 原生文件选择器 | `@tauri-apps/plugin-dialog`、`main.jsx`、`settings-page.jsx`、Tauri capability | Windows 原生 dialog、路径编码和 exe 过滤器不能由 Linux 替代 | 在设置页点击两个“选择文件”；选择含空格/中文路径的 `gallery-dl.exe` 与 `aria2c.exe`；取消选择；分别校验并保存，再重启 | 对话框可打开/取消；路径完整保留；校验成功后才保存；取消不覆盖旧值 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| GUI-W-EXT-10 | Integration/GUI | Extension GitHub 外链与加载指南 | `settings-page.jsx`、Extension status | 浏览器协议、WebView2 外链行为和 Edge/Chrome 加载只能实机确认 | 点击 GitHub Extension 目录按钮；Edge/Chrome 分别按指南加载；删除必需文件后重新检测 | 外链打开正确目录；缺文件显示明确错误；不再出现本地导入控件；文件就绪不误报浏览器已连接 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| A11Y-W-NAV-11 | GUI/Accessibility | 服务状态键盘跳转与焦点 | `connection-status.jsx`、`main.jsx`、`settings-page.jsx` | WebView2 键盘焦点、滚动和读屏行为需目标环境确认 | 仅用 Tab/Enter/Space 遍历侧栏服务状态；激活 SQLite/Sidecar；检查设置页目标卡片滚动和焦点环；用 Narrator/NVDA 复核 | 状态行可操作、焦点可见、对应 section 到达且无横向滚动 | P1 | no | `BLOCKED_AUTOMATION` |

#### BLOCKED_AUTOMATION 手工步骤

若 WDIO/Tauri native session 因 `DevToolsActivePort`、WebView2 或 driver 生命周期失败，跳过自动化并保留失败日志。使用最新 Windows working copy 手工执行：

1. 构建并启动 portable artifact，记录 Windows 版本、DPI、WebView2、Node 和 Rust/Tauri 版本。
2. 进入工作台，准备跨越 20 条最近列表限制的任务数据，核对 Dashboard 全量指标。
3. 进入设置，使用原生 dialog 选择带中文、空格和长路径的 `gallery-dl.exe`/`aria2c.exe`，测试取消、非法文件、校验、保存和重启持久化。
4. 打开运行日志，逐项测试五档等级、搜索、内部滚动、自动跟随、复制和打开日志目录；确认页面主滚动与日志框滚动互不抢占。
5. 仅用键盘激活 SQLite/Sidecar 状态行，确认跳转至设置对应卡片；用 Narrator/NVDA 检查名称、状态、错误和保存反馈。
6. 点击 GitHub Extension 目录按钮，按 Edge 和 Chrome 指南加载；删除/恢复必需文件，确认状态提示不会误报浏览器连接。

记录截图、录屏、前端 console、Rust 日志和失败步骤；`BLOCKED_AUTOMATION`、`BLOCKED`、`NOT RUN` 均不得记为 PASS。

### 2026-09-17 Linux reconciliation after latest Windows validation

上一轮 Windows working-tree 验证确认 worker one-dir artifact 和 Core portable smoke 已通过，但发现两个测试契约问题：Rust runtime fixture 的 root 仍为 POSIX 绝对路径，Desktop portable contract test 对目录后缀使用了 POSIX 分隔符。两项均已在 Linux 修复并完成适用回归测试。由于当前 working tree 仍包含这些测试修改，所有受影响的 Windows 项目必须等待修复后 Windows 重验；历史 PASS/FAIL 只作为对应旧 revision 的证据保留。

| ID | 最新状态 | Linux reconciliation evidence | 下一步 Windows 验证 |
|---|---|---|---|
| WQ-P0-01 | `WINDOWS_VERIFICATION_PENDING` | Rust fixture 已改为相对 root；Linux workspace Rust test 日志中所有套件为 `ok` | 在修复后的 Windows working tree 执行完整 `cargo test --workspace --no-fail-fast`；确认上一轮 root fixture FAIL 不再复现 |
| WQ-WORKER-BUILD-01 | `WINDOWS_PASS` | revision `7e3d646` 上用 Python 3.12.14 / PyInstaller 6.22.3 生成 one-dir Windows worker；`python312.dll` 存在；Full 包内 worker `hello → ready`、未知字段 `INVALID_COMMAND`、调用 bundled gallery-dl 并返回预期不支持 URL 错误均通过；SHA-256 见 batch 6 记录 | 受控真实下载仍在 Full package 手工验收项内；worker 构建/基础协议不需重跑，除非输入变更 |
| WQ-PACKAGE-FULL-01 | `WINDOWS_FAIL` | Full 包内容由 `PORTABLE_ALLOW_BINARY_REUSE=1` 使用本轮刚构建的 binary 成功组装，但规范 fresh-build 命令在 Windows Node `spawn npm.cmd` 处报 `EINVAL`；Cargo workspace `0.1.1` 与 Tauri/PE/manifest `0.1.0` 也需确认；Computer Use 未保留可观察窗口，真实 X 下载未执行。问题分类 `CROSS_PLATFORM_CHANGE_REQUIRED` + `CROSS_PLATFORM_REVIEW_REQUIRED` | 修复 Windows spawn、确认产品版本来源后重跑 fresh Full build；按 batch 6 Manual Windows Validation Queue 完成窗口、Extension 和真实账号下载检查 |
| WQ-PACKAGE-CORE-02 | `WINDOWS_VERIFICATION_PENDING` | Core gallery-dl 显式 `excluded`；Node contract test 已改为 `path.join()`；上一轮 Core package/start PASS 仅属于旧 working tree | 重新组装 Core，确认即使 source 目录存在也不含 gallery-dl，再执行设置页外部路径和本地 Extension 导入 |

本轮没有 `WINDOWS_VERIFICATION_BLOCKING`。自动化能力不足的项目继续跳过自动化并生成手工验证步骤；`BLOCKED` / `BLOCKED_AUTOMATION` / `NOT RUN` 均不得记为 PASS。

### Linux reconciliation after Windows revalidation R2（2026-09-17）

Linux follow-up 针对 Windows R2 的两个测试失败完成修复：

- `runtime.rs` 使用相对路径 fixture，避免 Windows 将 `/tmp/...` 解释为反斜杠路径；
- `desktop/test/portable-package.test.mjs` 使用 `node:path.join()` 构造 `sidecar/gallery-dl` 后缀，避免 POSIX 字符串断言；
- 没有修改业务运行时行为，也没有为通过 Windows 验证而放宽契约。

Linux verification：`cargo fmt --all -- --check`、Desktop runtime tests 2/2、workspace Rust tests（日志中各 crate 均 `test result: ok`）、Desktop Node 31/31、Desktop Vite build、Extension 7/7、Sidecar pytest 12/12、PyInstaller syntax/compileall、`git diff --check` 均通过。

Windows revalidation status：`WQ-P0-01`、`WQ-WORKER-BUILD-01`、`WQ-PACKAGE-CORE-02` 均保持 `WINDOWS_VERIFICATION_PENDING`，等待包含本轮测试修复的 Windows working tree 重验；`WQ-PACKAGE-FULL-01` 继续 `WINDOWS_BLOCKED`，原因仍为缺少受控 gallery-dl artifact。上一轮 worker/Core PASS 不因 Linux 回归自动继承到当前 dirty revision。

### 2026-09-17 非 Windows 阶段最终收口

本轮已完成 Linux 可实现的 portable 包纯逻辑测试、Full/Core manifest 契约测试、worke
gallery-dl 参数测试、Sidecar `--gallery-dl` 参数回归测试，以及 PyInstaller spec 和
Windows worker artifact workflow。当前 Linux 没有 Windows PyInstaller bootloader、MSVC、
WebView2 或真实 Windows executable，因此以下项目不在 Linux 执行：

| ID | 项目 | 状态 | 阻塞原因 | 后续手工验证 |
|---|---|---|---|---|
| WQ-WORKER-BUILD-01 | PyInstaller Windows worker 构建与 `--help` smoke | `BLOCKED` | 需要 Windows runner 或 Windows PyInstaller bootloader | 在 Windows 执行 `Windows Sidecar Worker Artifact` workflow；下载 zip，解压到 `sidecar/xarchive-downloader/`，运行 `xarchive-downloader.exe --help`，记录版本、文件清单和 SHA-256 |
| WQ-PACKAGE-FULL-01 | Full portable 组装与 worker/gallery-dl/Extension 完整性 | `BLOCKED` | 缺少真实 worker artifact、Windows Desktop `.exe` 和 gallery-dl.exe | 准备三个 artifact 后执行 `PORTABLE_PACKAGE_TYPE=full npm run build:portable:windows --workspace desktop`；检查 manifest、目录清单，启动应用并完成 Sidecar hello/ready 与实际下载 smoke |
| WQ-PACKAGE-CORE-02 | Core portable 边界与设置入口 | `BLOCKED` | 缺少 Windows Desktop `.exe`，且需 WebView2 实机 | 执行 `PORTABLE_PACKAGE_TYPE=core npm run build:portable:windows --workspace desktop`；确认无 `extension/` 和 `sidecar/gallery-dl/`，在设置页配置 gallery-dl，点击 GitHub Extension 目录并按指南加载 |
| WQ-GALLERY-CORE-03 | 外部 gallery-dl.exe 校验、持久化和 worker 调用 | `BLOCKED_AUTOMATION` | 需要 Windows executable、中文/空格路径和真实进程启动 | 使用 `C:Toolsgallery dlgallery-dl.exe` 等路径，校验/保存后重启；确认 `config/config.yaml` 持久化，Sidecar 参数包含完整路径且无控制台窗口 |
| WQ-EXT-CORE-04 | Extension GitHub 外链、文件检测和浏览器加载 | `WINDOWS_VERIFICATION_PENDING` | 需要 Windows WebView2、Edge/Chrome 外链和真实扩展目录 | 点击 GitHub Extension 目录；按 Edge/Chrome 指南加载；删除/恢复 manifest、background.js 或 content.js 后重新检测 | 外链正确打开；缺文件显示明确错误；文件就绪不误报浏览器实时连接 |
| WQ-WEBVIEW2-05 | 浏览器加载、Native Host、WebView2 GUI | `BLOCKED` | 需要 Windows WebView2、Edge/Chrome、Named Pipe/Registry 和 GUI automation | 按 `windows-validation.md` 的 BLOCKED-02/BLOCKED-03 执行；自动化不可用时记录 `BLOCKED_AUTOMATION` 并保留截图、日志和版本信息 |
| WQ-RELEASE-06 | 安装器、签名、Updater、Tray 和真实账号 | `NOT RUN` | 当前 bundle 关闭，且缺少证书、账号和外部服务 | 仅在发布 artifact、签名证书、测试账号和服务凭据齐备后，按 BLOCKED-01/BLOCKED-04 执行；否则保持 NOT RUN，不得记为 PASS |

### 2026-09-17 Full/Core portable 与 Core Extension 管理（Linux handoff）

Linux 已完成共享配置、外部 gallery-dl 校验/保存、Extension 本地目录导入校验/原子替换，以及 `PORTABLE_PACKAGE_TYPE=full|core` 构建脚本和 `package-manifest.json`。由于当前仓库尚未定义可信 Extension 远程发布 artifact、固定 URL 和 SHA-256，本轮没有实现实际网络下载，GUI 下载按钮保持禁用并明确提示发布源未配置。

| ID | 验证项 | 关联修改 | 精确步骤 | 状态 |
|---|---|---|---|---|
| WQ-PACKAGE-FULL-01 | Full 包完整性 | `build-portable-windows.mjs`、Sidecar/Extension | 构建 Full 包；检查 worker、gallery-dl、Extension、manifest；启动并完成 Sidecar handshake | `WINDOWS_VERIFICATION_PENDING` |
| WQ-PACKAGE-CORE-02 | Core 包边界 | `build-portable-windows.mjs`、设置页 | 构建 Core 包；确认不含 gallery-dl/extension；进入设置页，确认可配置外部 gallery-dl，并可打开 GitHub Extension 目录 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-GALLERY-CORE-03 | Core 外部 gallery-dl | `validate_gallery_dl_path`、`save_gallery_dl_path`、worker args | 粘贴官方 `gallery-dl.exe` 路径；校验/保存；重启后确认路径保留并能被 worker 调用 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-EXT-CORE-04 | Core Extension 文件检测与浏览器加载 | `get_extension_status`、设置页 | 打开 GitHub Extension 目录；按 Edge/Chrome 指南加载；验证有效/缺文件状态和中文/空格路径 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-EXT-DOWNLOAD-05 | Core Extension 下载 | 尚缺受信任发布 artifact | 配置固定 URL/SHA-256 后执行下载、校验、临时解压、原子替换和失败恢复；当前源未定义，跳过 | `WINDOWS_VERIFICATION_PENDING` |

#### BLOCKED / 手工步骤

若自动化无法建立 Windows WebView2 session，跳过自动化并记录 `BLOCKED_AUTOMATION`，手工执行：分别组装 Full/Core；检查 `package-manifest.json` 与目录清单；在 Core 设置页粘贴官方 `gallery-dl.exe` 路径并保存；准备临时 Extension 目录并导入；删除 manifest 或必需脚本验证错误；使用中文、空格和只读目录验证路径与回滚；确认下载按钮在发布源未配置时保持禁用，不应发起任意网络请求。

### 2026-09-17 发布问题修复批次（Linux development handoff）

本轮根据预发布版本问题完成 Linux 可实现修复：数据库初始化与下载目录 setup 解耦，任务列表使用数据库 fallback；设置页增加页面级 Error Boundary；新增运行日志页面和 `read_application_logs`/`open_log_folder` commands，当前实时显示采用 1 秒轮询；Release 主程序和已覆盖的内部子进程增加 Windows 无控制台启动边界。

以下项目尚未在 Windows 实机完成，必须保持 `WINDOWS_VERIFICATION_PENDING`：

| ID | 验证项 | 关联模块 | 前置条件 | 精确行为 | 优先级 | 状态 |
|---|---|---|---|---|---|---|
| WQ-REL-DB-01 | 首次启动 SQLite 与任务列表 | `runtime.rs`、`commands.rs`、`archive.rs` | 全新 Windows portable 目录 | 不选择下载目录启动，SQLite 为 ready，任务列表正常返回空数组或历史任务 | P0 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-REL-SETTINGS-02 | 设置页白屏回归 | `main.jsx`、`settings-page.jsx`、`error-boundary.jsx` | Windows WebView2 应用 | 进入设置页并操作 aria2/Extension/日志设置，无白屏、无未捕获渲染异常 | P0 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-REL-LOG-03 | 运行日志实时界面 | `logs-page.jsx`、`logging.rs` | 应用可启动并生成日志 | 日志页初始读取成功，1 秒内显示新日志，筛选/搜索/自动跟随/复制/打开目录有效 | P1 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-REL-CONSOLE-04 | 主程序与子进程不弹终端 | `main.rs`、`platform.rs`、supervisor crates、`aria2.rs` | Release portable 包，Sidecar/aria2 可用 | 启动应用、Sidecar、aria2、执行任务全过程无 cmd/PowerShell/Python 控制台闪现 | P0 | `WINDOWS_VERIFICATION_PENDING` |

#### BLOCKED / 手工验证步骤

若 Windows 自动化无法建立 WebView2 session，将以下项目标记为 `WINDOWS_BLOCKED` 或 `BLOCKED_AUTOMATION`，不得改为 PASS：

1. 解压最新 portable 包到全新目录，启动应用；不选择下载目录，确认左下角 SQLite 为“已连接”，工作台任务列表不显示数据库未初始化错误。
2. 进入“设置”，确认页面不是白屏；依次刷新 aria2、重新检测 Extension、保存日志设置，并观察是否出现页面错误面板。
3. 进入“运行日志”，启动/停止 Sidecar 或刷新状态；确认新增日志约 1 秒内出现，切换等级、搜索文本、滚动离底后自动跟随暂停，再点击“回到底部”恢复。
4. 点击“复制可见日志”和“打开日志目录”，确认剪贴板内容及目录位置正确。
5. 开启屏幕录制，启动应用、Sidecar、aria2 检测/下载和一次任务，慢速回放确认没有 cmd、PowerShell、Python 或 Windows Terminal 窗口闪现。
6. 若失败，记录 Windows 版本、架构、应用 commit、日志文件路径、进程名/PID、截图或录屏；不要把自动化阻塞当作产品 FAIL。

## 重验元数据与增量重验

队列状态按 [`../development/cross-platform-validation.md`](../development/cross-platform-validation.md) §3.3 的重验规则维护。每个验证项可记录重验元数据：

- `last validated revision`：最近一次给出当前状态时的 Linux revision；
- `impact area`：相关的文件、模块或行为；
- `dependencies`：影响结论有效性的依赖或前置；
- `revalidation decision`：`KEEP_VALID`（当前 diff 无交集，保持原结论）或 `REVALIDATION_REQUIRED`（有交集或依赖变化）。

判定规则：

- 若当前 diff 与某项影响区无交集且相关依赖未变化，则该项保持上一轮结论（含 `WINDOWS_PASS`），本轮不必重复执行，也不得仅凭「队列仍为 pending」就把整份队列当成下一轮默认执行清单；
- 若存在交集、依赖变化或行为可能使原结论失效，则标记 `REVALIDATION_REQUIRED` 并回到 `WINDOWS_VERIFICATION_PENDING`；
- 历史条目不强求补填无法可靠追溯的 revision；元数据在后续轮次实际重验时随结果一并维护。

### 2026-09-16 Linux security-contract follow-up

- Linux 已将 `SidecarCommand` 标记为 `serde(deny_unknown_fields)`，并新增协议层回归测试，确认已移除的 per-request `executable` 字段不会被 JSONL consumer 接受。
- 这只闭合了跨层 schema/model contract 的 Linux 可验证部分；WQ-P1-12 仍为 `WINDOWS_VERIFICATION_PENDING`。Windows 仍需在真实 Sidecar/便携运行时中验证命令拒绝、合法命令执行、路径权限、symlink/junction/reparse 和错误诊断。
- 若真实 Windows endpoint、受控 Sidecar fixture 或 reparse harness 不可用，跳过对应验证并记录为 `BLOCKED`/`NOT RUN`；手工步骤继续使用本文件“BLOCKED / NOT RUN 项目与手工验证入口”中的步骤，不得将 Linux 协议测试外推为 Windows PASS。

### 2026-09-16 Linux follow-up after Python consumer failure

- Windows 增量复验发现 Python worker 对带 `executable` 的未知字段返回 `ready`；Linux 已在 `sidecar/src/xarchive_downloader/__init__.py` 增加与 `download-command.schema.json` 对齐的允许字段检查，并新增 worker regression。
- WQ-P1-12 现回到 `WINDOWS_VERIFICATION_PENDING`，不是 `WINDOWS_PASS`。Windows 重验必须确认未知字段被拒绝、合法 hello/download/cancel/shutdown 仍可用，并继续执行可用的路径权限、symlink/junction/reparse 和长 JSON fixture。

### 2026-09-16 Windows incremental security-contract validation

- Linux `dev` clean HEAD `fe185a262258cedbde78e481de479a69848caf11` 已经通过受控单向同步到 `E:ShiraishiVSCode WorkspaceTw2Tg`；关键文件哈希匹配，Windows 本地依赖和验证资料保留。
- WQ-P1-12 的 Rust protocol/supervisor/Desktop tests、fmt、targeted strict Clippy 和 Sidecar pytest 均通过（11/11、4/4、70/70、10/10）。
- Windows Python worker unknown-field probe 失败：带 schema 禁止的 `executable` 字段的 `hello` 被返回为 `ready`，说明 Python consumer 没有执行 `additionalProperties: false` 边界。WQ-P1-12 更新为 `WINDOWS_FAIL`；分类为跨平台项目安全契约缺口，不是 Windows 环境误报。
- Windows path permission、symlink/junction/reparse、长 JSON 和真实 Sidecar download 仍为 `NOT RUN`，原因是缺少受控 fixture；不得用 Rust 协议测试或 Python pytest 外推通过。
- Linux 后续已补齐 Python consumer 拒绝未知字段及 worker regression；WQ-P1-12 已回到 `WINDOWS_VERIFICATION_PENDING`，等待 Windows 重验。
### 2026-09-17 设置页组件批次（Linux 开发收口，pre-2）

本批次改动（Linux 已验证，Windows 结果待集中验证）：

- 前端：`Icon`/`CopyablePath`/`ConnectionStatus`/`ExtensionConnectionStatus` 组件化、`ui-state.js` 纯逻辑、Sidecar 真实路径显示与复制、aria2 最新版语义 + 自定义路径校验/保存。
- Rust：新增 `validate_aria2_path`、`save_aria2_path`、`get_sidecar_path`、`copy_text_to_clipboard`（`arboard`）commands 并注册；`latest_aria2_release` 单版本语义。

Linux 验证（全部 PASS）：`cargo fmt --all -- --check`；`cargo clippy -p xarchive-desktop --all-targets -- -D warnings`；`cargo test -p xarchive-desktop --all-targets --no-fail-fast`（72 passed）；`cargo check -p xarchive-desktop --all-targets`；`npm run check --workspace desktop`（vite build）；`npm run test --workspace desktop`（23/23，含新增 `desktop/test/ui-state.test.mjs` 与 `desktop/test/ui-wiring.test.mjs`）；`git diff --check`。

新增 Windows 队列项（均不阻塞 Linux 开发）：

| 项目 | 状态 | 原因 | 手工验证步骤 |
|---|---|---|---|
| WQ-GUI-20 设置页复制路径（Sidecar/Extension 目录） | `WINDOWS_VERIFICATION_PENDING` | WebView2 剪贴板写入与 `arboard` 在 Windows 的实际行为只能在目标环境确认 | 启动应用 → 设置页 → 点击 "Sidecar gallery-dl 路径" 与 "Extension 目录" 复制按钮 → 用记事本粘贴验证内容一致；验证长路径、含空格与 Unicode 路径；确认复制失败时有错误反馈且不崩溃。 |
| WQ-GUI-21 Sidebar Extension 状态映射 | `WINDOWS_VERIFICATION_PENDING` | 状态显示依赖 WebView2 渲染与真实 Extension 文件存在性 | 分别在 Extension 文件齐全/缺失两种状态下观察侧栏底部：缺失显示"文件缺失"，不出现永久"检测中…"；与设置页状态一致。 |
| WQ-GUI-22 aria2 自定义路径校验与保存 | `WINDOWS_VERIFICATION_PENDING` | `validate_aria2_path` 的可执行探测、`save_aria2_path` 写 `config.yaml` 需 Windows 文件/进程行为确认 | 输入有效 `aria2c.exe` 绝对路径 → 校验显示成功 → 保存 → 重启应用确认 `config.yaml` 持久化且优先使用自定义路径；输入文件夹路径、不存在路径、非 aria2 可执行文件时校验失败且不覆盖旧配置。 |
| WQ-GUI-23 aria2 "下载并安装最新版" | `WINDOWS_VERIFICATION_PENDING` | Windows 下载/解压/SHA-256 流程仅能在 Windows 执行 | 点击"下载并安装"→ 等待完成 → 确认安装路径、版本检测显示 v1.37.0、SHA-256 校验通过；人为断网时验证错误反馈。 |
| WQ-GUI-24 设置页图标/布局/DPI 回归 | `WINDOWS_VERIFICATION_PENDING`（GUI/DPI 项在自动化不可用时为 `BLOCKED_AUTOMATION`） | 图标几何与 DPI 渲染需真实 WebView2 环境 | 100%/125%/150% DPI 下截图对比侧栏与设置页图标对齐、`CopyablePath` 布局、aria2 路径输入行换行行为；键盘 Tab 顺序与焦点环可见。 |

### 2026-09-18 工作台、日志和设置页 UI 收口

本批次改动（Linux 已验证，Windows 结果待集中验证）：

- Dashboard 移除重复的“数据库”概览卡，保留左下角 SQLite 服务状态。
- `StatusRow` 图标容器与文本选择器拆分，修复 Sidecar/Extension 图标垂直对齐；运行环境卡片改为统一纵向间距。
- gallery-dl 启动检测增加 `validate_gallery_dl_path`；原生选择文件后直接调用 `save_gallery_dl_path`，成功显示已保存路径，失败提示重新选择。
- aria2 移除可编辑路径输入框，保留原生选择、校验、保存，并把“下载并安装”并入同一操作区。
- 日志搜索占位符字号、Extension 按钮、存储路径和日志字段间距调整；日志等级与最大日志文件数改为两列布局，窄屏回退单列。

Linux 验证（全部 PASS）：`npm run test --workspace desktop`（33/33）；`npm run check --workspace desktop`（Vite build）；`cargo check -p xarchive-desktop --all-targets`；`git diff --check`。

追加/更新 Windows 队列项（均不阻塞 Linux 开发）：

| 项目 | 状态 | 关联修改与原因 | 精确验证步骤 |
|---|---|---|---|
| WQ-GUI-25 gallery-dl 自动选择、校验与保存 | `WINDOWS_VERIFICATION_PENDING` | `desktop/src/main.jsx`、`desktop/src/pages/settings-page.jsx`；原生文件对话框、Windows 可执行文件探测、配置保存和 WebView2 状态刷新需目标环境确认 | 在未安装/路径失效时进入设置页，确认仅显示“未检测到 gallery-dl 可执行文件”和“选择文件”；选择有效 `gallery-dl.exe`，确认自动校验、保存、版本反馈和路径显示；选择无效文件，确认显示“请重新选择”且不覆盖旧配置；取消对话框确认状态不变；重启确认配置保持 |
| WQ-GUI-26 aria2 操作区与自定义路径回归 | `WINDOWS_VERIFICATION_PENDING` | `desktop/src/pages/settings-page.jsx`、`desktop/src/style.css`；WebView2 原生选择、按钮换行和 `aria2c.exe` 探测需目标环境确认 | 确认页面没有路径文本输入框；选择有效 `aria2c.exe` 后执行校验、保存；确认“下载并安装”位于自定义 aria2 操作区；在窄窗口下确认无横向滚动；验证无效路径不覆盖旧配置 |
| WQ-GUI-27 Dashboard/Logs/Settings 渲染与 DPI 回归 | `WINDOWS_VERIFICATION_PENDING`（GUI/DPI 项自动化不可用时为 `BLOCKED_AUTOMATION`） | `desktop/src/pages/dashboard-page.jsx`、`shared.jsx`、`settings-page.jsx`、`style.css`；真实 WebView2、字体和 DPI 才能确认几何居中与间距 | 在 100%/125%/150% DPI 观察 Sidecar/Extension 图标中心对齐、四项概览卡、运行环境卡片底部留白、日志 placeholder 字号、日志两列字段、Extension/存储间距；使用 Tab 检查顺序、焦点环和按钮可用性；在最小支持窗口宽度验证无溢出 |



## 本轮收口的 BLOCKED / NOT RUN 项目与手工验证入口

以下项目不因 Linux 收口而标记为 PASS。它们要么缺少 Windows/外部前置，要么关联功能尚未实现；进入 Windows validation phase 时按下列手工步骤处理。

| 项目 | 状态 | 原因 | 手工验证步骤 |
|---|---|---|---|
| 真实 Edge/X Cookie、Telegram 账号、Credential Manager | `BLOCKED` | 缺少受控测试账号、Edge profile、凭据和外部服务授权 | 准备专用非个人测试账号和空白 Edge profile；设置项目 Python/Sidecar；执行单媒体、多媒体、Quote/Reply、重复提交、认证失败和重启恢复；确认 Cookie/token/secret 不进入 stdout、SQLite payload、WebView 或日志；Telegram 使用测试 chat 验证保存/读取/删除、重启和失败重试。 |
| GUI WebView2/DPI/屏幕阅读器/原生桌面自动化 | `BLOCKED` | 依赖 Windows WebView2、DPI 环境和可用 GUI automation target；Linux 静态检查不能替代 | 在 Windows 启动 Tauri Debug；设置 100%、125%、150% DPI；测试最小窗口、Tab/Shift+Tab、Enter/Escape、焦点和错误状态；使用 Narrator/NVDA 检查角色、名称、状态、焦点和对比度；保存截图/录屏及工具错误。 |
| Native Host Named Pipe、Registry、浏览器安装 | `NOT RUN` / `BLOCKED` | Named Pipe server、manifest/Registry/installer 前置尚未形成最终可验证 artifact | 若 artifact 已提供：注册 host manifest，使用 `.pipexarchive-v1`；管理员/普通用户分别测试启动、request/response、request_id、多连接、断线重连、非法消息和退出。若 server/manifest 未提供，保留 `NOT RUN`，不得用 framing 单测替代。 |
| externalBin、Installer、signing、updater、Tray/Autostart | `NOT RUN` / `BLOCKED` | 当前 bundle/installer 或签名前置未完成/未提供 | 若生成 artifact：执行全新安装、覆盖升级、自定义非 ASCII 路径、卸载、签名/SmartScreen、失败回滚、数据保留、Tray、Single Instance 和 Autostart；若 bundle inactive 或 artifact 不存在，记录 `NOT APPLICABLE`/`NOT RUN` 及缺失前置。 |
| 真实 executor worker 接管 `ArchiveExecutionContext` | `NOT RUN` | Linux 已完成 runner-owned `ExecutorConfig`/`ProductionExecutionFactory`、immutable execution spec、single active runner、startup recovery scan、filesystem recovery action 和运行中 cancellation；最终用户入口切换尚未完成，Windows 运行时行为仍需目标环境确认 | 在最终接入 revision 上启动 Desktop；设置项目 Python/Sidecar；执行 submit/query/cancel/shutdown、成功/失败/terminal skip、duplicate/concurrent jobs；确认 runner 从 `archive_job_requests` 加载 request，独立创建 Database/FileStore/Sidecar，检查 RuntimeState 锁、SQLite state/event/error 顺序、attempt fencing、cancel 后 Sidecar 进程退出、异常退出和重启恢复。若缺少最终 artifact 或 restart fixture，保留 `NOT RUN`，不得用 Linux contract 替代。 |

## 2026-09-27 R7 非 Windows 开发收口与 Windows 集中验证汇总

本轮 Linux development phase 已结束。Plan 中所有不依赖 Windows 的开发与测试均已完成；以下项目只能在 Windows 目标环境执行，本轮记为 `NOT RUN`（无 Windows 环境），并对其中 `BLOCKED` 项给出手工验证步骤。进入 Windows validation phase 前不得将下列任何项目记为 PASS。

### Linux 收口证据（2026-09-27）

- ENG-13（P1，RISK-005 子向量）：`--rpc-secret` 移出子进程 argv，改走 owner-only 短期 `--conf-path` 文件；`xarchive-download` 18/18、workspace 189/189（8 crates）、`cargo fmt --check`、严格 Clippy `-D warnings`、`cargo check --workspace --locked --all-targets`、`npm run check`/`test`、Sidecar compileall + pytest 19/19 全部通过。
- R7 其余 P0/P1/P2（ENG-01～ENG-12、ENG-14～ENG-16）的 Linux 实现与验证结论见 roadmap 执行进度表；本轮未改动其代码，仅新增 WQ-ENG-12 覆盖 ENG-13 的 Windows 侧。
- 当前分支 `security/tweet-url-host-validation`（工作树干净，已 push，与远端同步）；同步到 Windows 前以 push 后的 HEAD revision 为准，见本轮最终报告记录。

### Windows 专属验证汇总（按类别）

| 类别 | 队列项 | 状态 | 手工验证入口 |
|---|---|---|---|
| Build/Toolchain | WQ-ENG-01、WQ-ENG-02、WQ-ENG-10、WQ-ENG-11 | `WINDOWS_VERIFICATION_PENDING`（本轮 `NOT RUN`） | 按各条目精确手工步骤执行：越界 `PORTABLE_OUTPUT_DIR` 删除前失败；`cargo check --workspace --locked` / `cargo test` / `npm run build:tauri`；toolchain 版本与 pin 一致；executor 拆分后 MSVC 无编译/Clippy 错误且 release `.exe` 生成 |
| Filesystem | WQ-ENG-03 | `WINDOWS_VERIFICATION_PENDING`（本轮 `NOT RUN`） | 归档根下创建指向根外的 junction 作为中间目录，执行归档提交/metadata 校验/恢复扫描；越界被拒绝且不写入根外 |
| Runtime/Process | WQ-ENG-04、WQ-ENG-05、WQ-ENG-12 | `WINDOWS_VERIFICATION_PENDING`（本轮 `NOT RUN`） | 半帧连接突发与超时恢复；超长单行/大量 stderr 假 worker 输出限长与稳定错误码；含空格/中文路径的 aria2 下载解压；WQ-ENG-12 用受控随机 secret 启动真实 `aria2c.exe`，经任务管理器/`wmic process get CommandLine` 确认命令行无 secret，检查 temp secret 文件权限与退出后删除，并执行受控下载 |
| Integration | WQ-ENG-06、WQ-ENG-09 | `WINDOWS_VERIFICATION_PENDING`（本轮 `NOT RUN`） | 受控账号触发 AUTH_REQUIRED/限流/网络失败，检查日志页、任务错误、SQLite 与剪贴板无凭据残留；Node override 后重装依赖，锁定 driver 路径与 SHA-256，运行普通与 advanced E2E，退出后无遗留进程/端口 |
| Packaging/Regression | WQ-ENG-07、WQ-ENG-08 | `WINDOWS_VERIFICATION_PENDING`（本轮 `NOT RUN`） | tag/checkout 不一致发布被拒绝；哨兵 `.env`/测试文件不进入产物；在最终 diff 上执行 Node/Rust/Sidecar/Tauri 全量回归 |
| GUI/WebView2/DPI/辅助技术 | GUI-W-*、EXT-W-*、WQ-GUI-20～27、A11Y-W-NAV-11 | `WINDOWS_VERIFICATION_PENDING` / `BLOCKED_AUTOMATION`（本轮 `NOT RUN`） | 自动化无法建立 native WebView2 session 时跳过自动化并保留失败日志，按各条目手工步骤执行并保存截图/录屏；`BLOCKED_AUTOMATION` 不得记为 PASS |
| 真实账号/凭据/外部服务 | 真实 Edge/X Cookie、Telegram 账号、Credential Manager | `BLOCKED`（本轮 `NOT RUN`） | 见下表手工步骤 |
| Native Host/Named Pipe/Registry/浏览器实机 | 相关条目 | `NOT RUN` / `BLOCKED`（本轮 `NOT RUN`） | 见下表手工步骤；若 server/manifest 未提供则保留 `NOT RUN` |
| Installer/signing/updater/Tray/Autostart | 相关条目 | `NOT RUN` / `BLOCKED`（本轮 `NOT RUN`） | 见下表手工步骤；当前 `bundle.active=false` 时记 `NOT APPLICABLE`/`NOT RUN` 及缺失前置 |

### BLOCKED 项目手工验证步骤（本轮跳过执行，仅生成步骤）

| 项目 | 状态 | 阻塞原因 | 手工验证步骤（进入 Windows phase 时执行） |
|---|---|---|---|
| 真实 Edge/X Cookie、Telegram 账号、Credential Manager | `BLOCKED` | 缺少受控测试账号、Edge profile、凭据和外部服务授权 | 1. 准备专用非个人测试账号和空白 Edge profile。2. 设置项目 Python/Sidecar。3. 执行单媒体、多媒体、Quote/Reply、重复提交、认证失败和重启恢复。4. 确认 Cookie/token/secret 不进入 stdout、SQLite payload、WebView 或日志。5. Telegram 使用测试 chat 验证保存/读取/删除、重启和失败重试。6. 记录账号类型、profile 路径、网络条件与失败分类 |
| GUI WebView2/DPI/屏幕阅读器/原生桌面自动化 | `BLOCKED`（自动化侧为 `BLOCKED_AUTOMATION`） | 依赖 Windows WebView2、DPI 环境和可用 GUI automation target；Linux 静态检查不能替代 | 1. 在 Windows 启动 Tauri Debug/Release portable artifact，记录 Windows 版本、DPI、WebView2、Node、Rust/Tauri 版本。2. 设置 100%、125%、150% DPI，测试最小窗口、Tab/Shift+Tab、Enter/Escape、焦点和错误状态。3. 使用 Narrator/NVDA 检查角色、名称、状态、焦点和对比度。4. 保存截图/录屏、前端 console、Rust 日志及工具错误。5. WDIO 因 `DevToolsActivePort`/driver 生命周期失败时保留失败日志并手工覆盖 |
| Native Host Named Pipe、Registry、浏览器安装 | `BLOCKED`（前置缺失时为 `NOT RUN`） | Named Pipe server、manifest/Registry/installer 前置尚未形成最终可验证 artifact | 1. 若 artifact 已提供：注册 host manifest，使用 `\\.\pipe\xarchive-v1`。2. 管理员/普通用户分别测试启动、request/response、request_id、多连接、断线重连、非法消息和退出。3. 用 Edge/Chrome 实机加载验证 Service Worker 重连。4. 若 server/manifest 未提供，保留 `NOT RUN` 并记录缺失前置，不得用 framing 单测替代 |
| externalBin、Installer、signing、updater、Tray/Autostart | `BLOCKED`（无前置时为 `NOT APPLICABLE`/`NOT RUN`） | 当前 bundle/installer 或签名前置未完成/未提供 | 1. 若生成 artifact：执行全新安装、覆盖升级、自定义非 ASCII 路径、卸载、签名/SmartScreen、失败回滚、数据保留、Tray、Single Instance 和 Autostart。2. 若 bundle inactive 或 artifact 不存在，记录 `NOT APPLICABLE`/`NOT RUN` 及缺失前置。3. 不得把未执行项目标为 PASS |
| 真实 executor worker 接管 `ArchiveExecutionContext` | `NOT RUN` | 最终用户入口切换尚未完成，Windows 运行时行为仍需目标环境确认 | 1. 在最终接入 revision 上启动 Desktop。2. 设置项目 Python/Sidecar。3. 执行 submit/query/cancel/shutdown、成功/失败/terminal skip、duplicate/concurrent jobs。4. 确认 runner 从 `archive_job_requests` 加载 request，独立创建 Database/FileStore/Sidecar，检查 RuntimeState 锁、SQLite state/event/error 顺序、attempt fencing、cancel 后 Sidecar 进程退出、异常退出和重启恢复。5. 若缺少最终 artifact 或 restart fixture，保留 `NOT RUN`，不得用 Linux contract 替代 |

本轮没有 `WINDOWS_VERIFICATION_BLOCKING`。Windows validation phase 开始时须先同步本次 Linux 最终 diff（含 ENG-13），按上表一次性执行；`FAIL`、`BLOCKED`、`NOT RUN` 均须记录原因，不得记为 PASS。

### 2026-09-16 GUI settings / Extension batch

本轮 Linux implementation 已完成以下与截图直接相关的改动：

- 工作台和设置页分离；设置入口位于侧栏底部并与主导航分隔；归档位置、日志设置、Sidecar 控制、aria2 管理和 Extension 指南不再堆叠在工作台。
- 统一本地 Windows 字体 fallback、图标 SVG 容器、按钮焦点和响应式布局；aria2 不再使用会造成 `a`/`2` 上下错位的隐式 Grid 文本 Logo。
- 配置相对路径进行词法归一化，`./logs` 应显示为 `<portable-root>/logs`。
- Desktop 新增 Extension 文件完整性检查和 Extension 文件夹打开命令；浏览器实时连接仍未由当前跨平台代码实现，不能把文件存在报告为“浏览器已加载”。
- 未实现 Sidecar 远程下载按钮：仓库当前没有可核验的 XArchive Sidecar 发布 artifact、版本 allowlist、SHA-256 清单、签名和许可证分发定义；本轮只保留现有 Sidecar 启停和 aria2 官方包下载能力。

对应集中式 Windows handoff：

| ID | 类别 | 验证项目 | 关联修改 | 前置条件 | 精确步骤/命令 | 预期结果 | 优先级 | 状态 |
|---|---|---|---|---|---|---|---|---|
| GUI-W-SETTINGS-01 | Runtime/GUI | 工作台与设置页信息架构 | `desktop/src/main.jsx`、`desktop/src/style.css` | Windows Tauri artifact、WebView2 | 启动应用；分别打开“工作台”“设置”；检查侧栏底部设置入口、分隔线、页面切换、工作台滚动长度和设置区块 | 工作台只显示概览/任务/运行环境；设置页显示 Sidecar、aria2、Extension、存储、日志；无横向滚动和关键内容遮挡 | P1 | `WINDOWS_VERIFICATION_PENDING` |
| GUI-W-VISUAL-02 | GUI/DPI | 图标、字体和路径显示 | `desktop/src/main.jsx`、`desktop/src/style.css`、`portable.rs` | Windows 100%/125%/150% DPI | 启动 release portable artifact；检查 Sidecar、aria2、folder、browser 图标；检查中文、英文、数字、代码路径；检查日志路径 | 图标在容器内居中；字体层级一致；日志显示为 `...logs`，不含 `./logs`；路径不溢出卡片 | P1 | `WINDOWS_VERIFICATION_PENDING` |
| EXT-W-FILES-03 | Runtime/Integration | Extension 文件检测与加载指南 | `commands.rs`、`lib.rs`、`desktop/src/main.jsx` | portable artifact 含 `extension/` | 删除或重命名一个必需 Extension 文件后重新检测；恢复文件后再检测；展开 Edge/Chrome 指南并执行加载步骤 | 缺文件时显示明确错误；文件完整时显示“文件已就绪”；指南步骤可读且没有把文件就绪误报成实时连接 | P1 | `WINDOWS_VERIFICATION_PENDING` |
| EXT-W-NATIVE-04 | Integration | Native Host/Named Pipe/浏览器实时连接 | `crates/xarchive-native-host`、Windows transport backend、Extension background bridge | 最终 Windows Named Pipe server、Host manifest、Registry、Edge/Chrome 实机 | 注册 Host；加载扩展；打开/刷新 `https://x.com/`；观察 hello、重连、断开和 GUI 状态；重复 Edge/Chrome | 合法连接可建立；断开可诊断；Service Worker 重启可重连；当前缺少 backend/manifest 时标记 BLOCKED，不使用 Linux 文件检查替代 | P0 | `WINDOWS_VERIFICATION_PENDING` |
| SIDECAR-W-DOWNLOAD-05 | Packaging/Runtime | Sidecar 下载、校验、安装和握手 | 尚缺 artifact contract；当前无生产下载实现 | XArchive Sidecar Windows x64 release、SHA-256、签名/许可证清单、网络 | 在 artifact contract 完成后，从设置页选择版本并下载；校验失败、断网、解压失败、升级、回滚和 `hello → ready` | 仅可信 artifact 可安装；失败不破坏旧版本；安装后配置指向 XArchive JSONL worker；当前因发布物未定义标记 BLOCKED | P0 | `WINDOWS_VERIFICATION_PENDING` |
| A11Y-W-SETTINGS-06 | GUI/Accessibility | 设置页键盘、焦点、读屏和对比度 | `desktop/src/main.jsx`、`desktop/src/style.css` | WebView2、Narrator 或 NVDA | 仅使用键盘遍历导航、按钮、表单、`details`；使用 Narrator/NVDA 检查标题、按钮名称、状态和错误；检查 100/125/150% DPI | 所有可交互元素可到达；焦点可见；图标按钮有名称；错误和设置保存状态可读；无关键内容被缩放裁切 | P1 | `WINDOWS_VERIFICATION_PENDING` |

## 当前队列

### 2026-09-18 U2 Sidecar cooperative cancellation handoff

Linux 已完成并验证 Sidecar 的 cooperative cancellation：worker 在 gallery-dl 运行期间消费控制队列；`cancel` 返回 `CANCELLED`，应用 shutdown 返回 `INTERRUPTED`，超时返回 `DOWNLOAD_TIMEOUT`；POSIX 使用独立 process session，Windows 使用 `taskkill /T /F` 回收子进程树。以下项目不阻塞 Linux 开发；若 Windows 自动化前置不可用，跳过自动化并按手工步骤记录 `BLOCKED` 或 `NOT RUN`，不得记为 PASS。

| ID | 类别 | 验证项目 | 关联修改 | Windows 原因 | 前置条件 | 精确行为 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-SIDECAR-CANCEL-01 | Runtime/Process | 下载期间 cancel 回收 gallery-dl 及其子进程树 | `sidecar/src/xarchive_downloader/__init__.py`、`gallery.py`、`process.py` | Windows process tree、taskkill 时序和句柄回收不能由 Linux 外推 | Windows worker、可控长时间 gallery-dl fixture、Process Explorer 或 PowerShell | 启动下载；下载未完成时发送 `cancel`；记录 worker、gallery-dl 及孙进程 PID；等待终止并检查 staging 文件 | 收到稳定 `CANCELLED`；整个进程树退出；不再写 staging；无残留锁定文件或孤儿进程 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-SIDECAR-CANCEL-02 | Runtime/Process | shutdown interruption 与正常 sidecar shutdown 区分 | `__init__.py`、`gallery.py`、Desktop executor/archive cancellation path | Windows shutdown/CTRL 生命周期和子进程回收需实机确认 | Desktop debug/release artifact、可控长下载 fixture | 下载期间关闭应用或发送 executor shutdown；随后检查 sidecar、gallery-dl 和子进程；重新启动应用 | Job 持久化为 `INTERRUPTED`；sidecar process tree 退出；重启恢复扫描可诊断且不重复提交 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-SIDECAR-CANCEL-03 | Runtime/Process | timeout、cancel 与自然失败错误码边界 | `errors.py`、`gallery.py`、worker protocol | Windows timer、exit code 和 stderr/console 行为需确认 | 可控 fixture：超时、用户取消、非零 exit | 分别触发三种场景并保存 JSONL/stdout、stderr、Job event | 分别得到 `DOWNLOAD_TIMEOUT`、`CANCELLED`、`EXTRACT_OR_DOWNLOAD_FAILED`；stdout 仍为合法 JSONL，stderr 不泄漏秘密 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |

#### U2 Windows 自动化被 BLOCKED 时的手工步骤

1. 在最新 Linux working tree 对应的 Windows 副本中构建 worker/Desktop artifact；记录 Windows 版本、架构、Python worker 版本、Tauri/Node 版本。
2. 使用一个不会访问真实 X 账号的本地长运行 gallery-dl fixture，令其再启动一个子进程；确认 fixture 仅写入临时 staging。
3. 启动 Sidecar，发送 `hello`、`download`；下载进行中发送 `cancel`，保存 JSONL、stderr、PID 树和 staging 目录快照。
4. 重复执行下载期间 shutdown，确认 Job 状态是 `INTERRUPTED` 而不是 `CANCELLED`，并检查重启后 recovery 行为。
5. 使用短 timeout、自然非零退出和缺失 executable 分别验证 `DOWNLOAD_TIMEOUT`、`EXTRACT_OR_DOWNLOAD_FAILED`、`SIDECAR_DEPENDENCY_MISSING`；确认 JSONL stdout 无非协议文本。
6. 若缺少长运行 fixture、可观察 PID 工具、最终 artifact 或受控 staging，分别记录 `BLOCKED`/`NOT RUN` 原因；不得使用手工 `Stop-Process` 后把项目写成 PASS。

| ID | 类别 | 验证项目 | 关联模块/修改 | Windows 原因 | 前置条件 | 精确行为 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-P0-01 | Build/Toolchain | Windows workspace 与 Tauri baseline | `Cargo.toml`、`package.json`、`desktop/`、`sidecar/` | MSVC、Windows SDK、WebView2、Python executable 和 Tauri 构建不能由 Linux 完全替代 | Windows toolchain、项目 `.venv`、Node dependencies | 修复后的 Windows working tree 中执行 Rust fmt/check/test/clippy、Node check/test/build、Sidecar tests、Tauri build/start/cleanup | 所有适用检查通过，无项目代码失败；历史 runtime 路径断言失败不得复现 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P0-02 | Integration | 真实 X/Edge Cookie archive | Edge Profile、Sidecar、Storage、Desktop archive flow | Cookie 加密存储、Edge Profile 和真实 X 响应只能在目标环境确认 | 测试账号、Edge Profile、gallery-dl、可用网络 | 覆盖无媒体、单图、多图、视频、Quote/Reply、重复任务和异常退出后的真实归档 | Cookie 不泄露；Tweet、媒体、SQLite、staging 正确且幂等 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P0-03 | Filesystem | 文件 SQLite 应用级恢复 | `crates/xarchive-storage/migrations/`、Storage、Tauri Desktop | 文件锁、应用重启、Windows 路径和异常退出无法由 in-memory 测试充分判断 | Desktop artifact、受控目录、可重复数据、旧库副本 | 验证真实文件 DB、关闭/重启、遗留 staging、`0001 → 0002 → 0003` 和异常退出恢复 | 状态恢复、迁移、staging 清理和唯一约束正确 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P0-04 | Integration | Native Host/Named Pipe end-to-end | Native Host、Protocol、Desktop `transport.rs`、Desktop IPC | Named Pipe server、ACL、连接和 Windows IPC 生命周期是平台行为；Linux transport contract 不能替代端到端验证 | Named Pipe server、`.pipexarchive-v1`、ACL 方案、最新 Desktop artifact | 验证请求/响应、request_id 路由、多连接、重连、关闭、非法消息和权限拒绝；确认 transport adapter 的 submit/query 响应与 Native Host framing 一致 | 合法请求正确转发，非法或越权请求明确失败，无串线或挂起；request_id 不丢失 | P0 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-01 | Integration | DownloadRouter 与真实 aria2 业务集成 | `xarchive-download`、Desktop Job、Sidecar/Job orchestration | aria2c.exe、Windows 路径、真实 media URL 和进程恢复需目标环境确认 | 受控 aria2c.exe（当前半永久化验证目录：E:ShiraishiVSCode WorkspaceTw2Tgaria2）、media server、可重复归档场景 | 验证 gallery-dl 默认、错误回退、403 后重新提取、transfer lifecycle 和 Job 状态同步 | fallback 只在适用错误触发，状态、事件和文件结果一致 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-02 | Packaging/Integration | Native Host browser installation | Native Host manifest、Registry、Installer | Registry、浏览器扩展 ID 和安装权限是 Windows 专属行为 | Host manifest、固定 Extension ID、Edge/Chrome 实机 | 验证安装、升级、卸载、管理员/非管理员、扩展加载和 Service Worker 重连 | 浏览器能加载 Host，连接和错误反馈符合协议 | P1 | no | `WINDOWS_FAIL` |
## 2026-09-16 Linux completion 与 Windows 验证汇总

- 当前没有 `WINDOWS_VERIFICATION_BLOCKING`。
- Linux 开发阶段已结束：当前工作区完成了所有不依赖 Windows 的开发和验证。本阶段完成项包括 Rust workspace fmt、检查和测试（全部通过）、Desktop Tauri crate 编译检查通过，以及 `desktop/src-tauri/src/transport.rs` 上的 Linux/Unix transport endpoint 注册与回归覆盖。相关结论已同步到 `docs/development/status.md`、`docs/development/roadmap.md`、`docs/development/non-windows-completion.md` 和本文。
- 当前 Linux 状态事实（与本阶段 Windows 待办直接相关）：
  - Desktop 已在 Linux/Unix 上注册 Unix domain socket transport endpoint，Native Host 在 Linux 上改用 `UnixStream::connect` 连接。`desktop/src-tauri/src/transport.rs` 已成为 Desktop 生产 endpoint 注册的一部分，不再是仅作为 contract adapter。Native Host 在 Linux/Unix 上通过配置的 `XARCHIVE_PIPE_ENDPOINT` 使用 Unix stream 连接 Desktop，Windows 下仍保留 `OpenOptions` 文件打开路径，Desktop 不注册 Named Pipe server。
  - R1 Browser transport 已通过 Unix endpoint 接入 `BrowserTransportAdapter`/`ArchiveApplicationService`，同步 `archive_tweet` fallback 保持保留；R2 真实传输接入仍未完成。
  - R2 当前不是 Windows 阻塞项：Sidecar failure event 尚未提供 fresh media URL，Rust/Python/Schema extraction-result contract 尚未定义；在该 Linux 设计前置完成前，不执行或伪造 aria2 application fallback 结果。
- Windows 验证目标不是重复 Linux 已验证项，而是验证以下 Windows 专属/平台依赖项目：
  - Windows 专属验证项（按当前队列状态）：
  - `WINDOWS_VERIFICATION_PENDING`（需在 Windows 目标环境重验）：WQ-P0-01、WQ-P0-02、WQ-P0-03、WQ-P0-04、WQ-P1-01、WQ-P1-12、WQ-P1-14、WQ-P1-15、WQ-P1-16、WQ-P1-17、WQ-P1-18、WQ-P1-19、WQ-P2-01、WQ-P2-02，以及本轮 `WQ-GUI-20`～`WQ-GUI-27`。
    - `WINDOWS_FAIL`（Windows 验证中发现的问题，尚未修订/重验通过）：WQ-P1-02、WQ-P1-03、WQ-P1-04、WQ-P1-05、WQ-P1-13。其中 WQ-P1-04（Native Host / Named Pipe end-to-end）与当前 Linux transport endpoint 的事实直接相关：Linux 已实现 Unix endpoint 和 Native Host `UnixStream::connect`，但 Windows Named Pipe server、ACL、多连接、重连、关闭和浏览器加载仍需独立 Windows 验证，不能将 Linux Unix socket 测试外推为 Windows PASS。
  - `BLOCKED` / `NOT RUN` 项目（当前无法执行，必须记录原因和手工步骤）：
    - 真实 Edge/X Cookie、Telegram 账号、Credential Manager：`BLOCKED` — 缺少受控测试账号、Edge profile、凭据和外部服务授权。手工步骤：准备专用非个人测试账号和空白 Edge profile；设置项目 Python/Sidecar；执行单媒体、多媒体、Quote/Reply、重复提交、认证失败和重启恢复；确认 Cookie/token/secret 不进入 stdout、SQLite payload、WebView 或日志；Telegram 使用测试 chat 验证保存/读取/删除、重启和失败重试。
    - GUI WebView2/DPI/屏幕阅读器/原生桌面自动化：`BLOCKED` — 依赖 Windows WebView2、DPI 环境和可用 GUI automation native target。手工步骤：在目标 Windows 机器上确认 WebView2 runtime、DPI 设置和屏幕阅读器环境可用；手工运行应用、检查窗口创建/DPI 缩放/键盘聚焦/对比度/可访问性信息；仅在环境准备好后再考虑 WDIO/Windows-native session 自动化。
    - Native Host manifest/Registry、Edge/Chrome 实机加载（WQ-P1-02）：`WINDOWS_FAIL`/手工验证方式。手工步骤：部署 Native Host manifest 和注册表项，确认固定 Extension ID、浏览器加载、Service Worker 重连和 Named Pipe/ACL 端到端行为；在 Windows 上使用真实或受控浏览器配置逐项验证，而不是依赖 Linux Unix endpoint 结果。
    - 便携 `.exe` 目录布局与首次下载 setup（WQ-P1-18）、日志等级与轮转（WQ-P1-19）：`WINDOWS_VERIFICATION_PENDING`/手工验证方式。手工步骤：在 Windows 上运行便携目录、验证首次启动布局、`config.yaml` 持久化、下载目录选择和日志轮转行为。
    - 安装器、签名、updater、Tray/Single Instance/Autostart（WQ-P2-01 等）：`NOT RUN` — 当前阶段不生成安装器，相关 artifact/证书/发布环境尚未具备。手工步骤：仅在后续具备打包/签名环境时执行。
- 进入 Windows validation phase 前的准备建议：
  - 先确认目标 Windows 环境的 WebView2、Edge WebDriver/tauri-driver、项目 Python/Sidecar、前置目录和（如需要）受控媒体 fixture 是否可用；
  - 对因缺少前置而无法运行的项目，明确标记 `BLOCKED` 或 `NOT RUN`，并保留上述手工步骤，不以 Linux contract、fake transport 或静态检查替代 Windows 结论；
  - WQ-P1-04 在 Windows 上必须单独验证 Named Pipe server/客户端、ACL、多连接、重连、关闭和浏览器加载，不可因为 Linux 已有 Unix endpoint 而跳过或改判通过。
- 本汇总不新增业务代码修改，仅反映当前 Linux 状态与累计 Windows 待办。后续 Windows 验证结果应写回本文及 `windows-queue.md` 对应行。

### 2026-09-18 Plan 收口后的 BLOCKED Windows 手工验证补充

以下项目不在 Linux 阶段执行，进入 Windows validation phase 时若自动化前置不可用，直接按手工步骤记录，不得把跳过记为 PASS：

| ID | 项目 | BLOCKED / NOT RUN 原因 | 手工验证步骤 | PASS 标准 |
|---|---|---|---|---|
| MANUAL-WIN-GUI-01 | WebView2/DPI/键盘/屏幕阅读器 | 缺少可用 Windows native GUI automation target 或真实 WebView2 环境 | 在 Windows 10/11 启动最新同步 artifact；依次设置 100%、125%、150% DPI；进入工作台、运行日志、设置；用鼠标和 Tab/Shift+Tab 操作；检查 Sidecar/Extension 图标、四项指标、日志 placeholder、gallery-dl/aria2 控件、日志两列字段、Extension/存储间距；使用 Narrator/NVDA 检查名称、角色、状态、焦点；保存截图/录屏和工具日志 | 无横向溢出、图标几何居中、焦点可见、按钮可操作、屏幕阅读器名称/状态正确；失败需记录截图、窗口尺寸、DPI、WebView2 版本 |
| MANUAL-WIN-FILE-01 | gallery-dl/aria2 原生文件选择 | 原生文件对话框和 Windows executable probe 不能由 Linux fake dialog 替代 | 设置页选择有效/无效/中文/空格/长路径的 `gallery-dl.exe` 和 `aria2c.exe`；确认 gallery-dl 选择后自动校验/保存；aria2 选择后校验/保存；取消对话框；重启应用；检查 `config/config.yaml` 和页面状态 | 有效路径保存且重启保持；无效路径显示错误且不覆盖旧配置；取消不改变状态；页面没有 aria2 文本路径输入框 |
| MANUAL-WIN-IPC-01 | Named Pipe / Native Host / Registry | Windows Named Pipe、ACL、Registry、浏览器扩展安装尚未由 Linux Unix socket 结果覆盖 | 安装/注册 Native Host manifest；管理员和普通用户分别启动；使用 `.pipexarchive-v1` 发送 archive/query request；验证 request_id、重复提交、多个连接、断线重连、非法消息、关闭；在 Edge/Chrome 加载扩展并观察 Service Worker 重连 | 合法请求正确返回；request_id 不串线；非法/越权连接失败；重连和关闭无挂起；浏览器能加载 Native Host |
| MANUAL-WIN-FS-01 | portable runtime/ACL/reparse/长路径/restart | Windows 文件系统语义、ACL、锁、junction/reparse 和跨卷提交没有 Linux 等价结论 | 使用含中文/空格和跨盘的便携目录；首次启动选择 Downloads/XArchive；制造 staging、WAL/SHM、文件锁、junction/reparse、长路径和异常退出；重启应用并执行 recovery；检查日志、SQLite、最终归档和权限 | 不发生路径逃逸或数据损坏；reparse/link 按策略拒绝；恢复不伪造 COMPLETE；跨卷提交和权限错误可诊断 |
| MANUAL-WIN-EXT-01 | 真实 Edge/X Cookie、Telegram、Credential Manager | 缺少受控测试账号、Edge profile、Credential Manager 和 Telegram test chat | 准备专用非个人账号和空白 profile；执行单媒体、多媒体、Quote/Reply、重复提交、认证失败、限流和重启；检查 Cookie/token/secret 不进入 stdout、SQLite、WebView 或日志；验证 Telegram 保存/读取/删除/重试 | 凭据不泄露；认证/限流/重试状态可诊断；发送幂等且跨重启恢复 |
| MANUAL-WIN-PKG-01 | installer/signing/updater/Tray/Autostart | 当前 Linux 阶段无最终 installer、签名证书或发布 artifact | 只有 artifact/证书准备完成后执行全新安装、覆盖升级、自定义非 ASCII 路径、卸载、SmartScreen、失败回滚、数据保留、Tray、Single Instance、Autostart | 安装/升级/卸载/回滚可重复；签名和 SmartScreen 结果符合发布要求；数据按约定保留 |
| WQ-P1-03 | Regression | Windows GUI rendering and accessibility | `desktop/src/main.jsx`、`desktop/src/style.css`、Tauri | WebView2、DPI、系统字体、屏幕阅读器和命中区域不能由 Linux 静态检查替代 | WebView2、DPI 环境、键盘、Narrator/NVDA | 验证 100/125/150% DPI、最小窗口、Tab、键盘、Focus-visible、辅助技术和对比度 | 真实渲染、交互和辅助技术反馈符合预期 | P1 | no | `WINDOWS_FAIL` |
| WQ-P1-04 | Integration | Credential Manager 与 Telegram account flow | `xarchive-telegram`、Windows secret backend | Credential Manager 用户边界和真实账号/网络行为依赖 Windows/外部环境 | Credential Manager backend、Bot token、Telegram test chat | 验证保存/读取/更新/删除、应用重启、日志隔离、真实发送和限流 | Secret 不泄露，真实发送和重试状态正确 | P1 | no | `WINDOWS_FAIL` |
| WQ-P1-05 | Runtime/Packaging | Sidecar、externalBin 和进程生命周期 | Tauri packaging、Sidecar、Tray/Autostart | Windows 子进程、资源路径、关闭和自启动行为需要目标环境 | bundled Sidecar、Tauri bundle | 验证启动、关闭、崩溃恢复、资源定位、Tray、Single Instance 和 Autostart | 资源可定位，子进程安全退出，生命周期正确 | P1 | no | `WINDOWS_FAIL` |
| WQ-P1-12 | Security/Privacy | Security boundary regression | Protocol、Storage、Desktop、Sidecar protocol/schema | Windows path semantics、reparse points、Desktop IPC packaging 和真实 Sidecar 边界不能由 Linux 完全替代 | 最新 Linux working tree、Windows workspace、受控 staging | 验证 URL/Tweet ID mismatch、metadata mismatch、executable override、敏感 settings、长 JSON、普通文件和 symlink/junction/reparse | 非法 identity、override、敏感 settings 和 link/reparse 被拒绝；合法 Unicode 文件仍可归档 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-13 | Privacy/Filesystem | Windows user-data privacy boundary | Desktop archive root、SQLite、staging、settings | Windows ACL、user profile、working directory 和共享目录权限不能由 Linux mode 替代 | 普通用户、非管理员账户、ACL 工具、受控临时目录 | 验证不同 working directory/盘符下 archive root、SQLite、WAL/SHM、staging ACL 和跨用户读取 | 数据目录定位稳定，仅当前用户可读写，权限错误可诊断 | P1 | no | `WINDOWS_FAIL` |
| WQ-P2-01 | Packaging | Installer、signing 和 updater | Tauri bundle、installer、updater | 安装器、签名、SmartScreen、Defender、升级/回滚为 Windows 发布行为 | installer artifact、证书/签名环境、发布测试机 | 验证全新安装、覆盖升级、自定义路径、卸载、签名、失败回滚和数据保留 | 安装、升级、卸载和回滚符合发布要求 | P2 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P2-02 | Filesystem/Regression | Windows filesystem stress and stability | Core、Storage、Desktop 用户目录/staging | 保留字符、长路径、文件锁和并行时序需要 Windows 文件系统确认 | 多盘、空格/中文/Unicode、保留名、长路径、文件锁、磁盘不足 | 执行路径、锁、磁盘和并行恢复压力场景 | 无路径逃逸、数据损坏或未处理崩溃 | P2 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-14 | Runtime/Integration | R1 executor runner-owned production integration, Browser transport contract and synchronous fallback regression | `desktop/src-tauri/src/executor.rs`、`transport.rs`、`archive.rs`、`runtime.rs`、`commands.rs`、`crates/xarchive-storage/migrations/0004_archive_job_requests.sql`、Storage、SidecarSupervisor、FileStore | Linux 已完成 immutable execution spec、`ExecutorConfig`/`ProductionExecutionFactory`、job_id spec loading、runner-owned Database/FileStore/Sidecar、attempt fencing、single active runner、startup recovery scan、真实 recovery action、运行中 cancellation 和 transport protocol contract；最终用户入口切换以及 Windows 运行时行为仍需目标环境确认。目前 Desktop 已在 Linux/Unix 上注册 Unix domain socket transport endpoint，Native Host 在 Linux 上改用 `UnixStream::connect` 连接；Windows Named Pipe/ACL 仍属平台适配与验证项，不把 Unix socket 测试外推为 Windows PASS。 | 在当前最终接入 revision 上执行 executor lifecycle status、submit/query/cancel/shutdown、duplicate/concurrent jobs、Browser request_id 路由、synchronous fallback、execution spec reload、attempt fencing、Sidecar crash、graceful shutdown、startup/restart recovery、resource ownership 和无长锁阻塞检查 | Windows Tauri artifact、项目 Python/Sidecar、受控 SQLite/archive root、可重复 fixtures、可制造异常退出和文件锁 | lifecycle status 与实际 worker 存活一致；execution spec 可重启加载；transport request_id 和状态枚举与 schema 一致；cancel 后 Sidecar 退出；late result 不覆盖 INTERRUPTED；Sidecar/Database/FileStore 由 runner ownership；无 RuntimeState 长锁阻塞；startup recovery 与 WQ-P1-15 facts/action 一致；无残留进程或重复归档 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-20 | Runtime/Integration | Async submit-and-schedule failure compensation | `desktop/src-tauri/src/executor.rs`、`transport.rs`、`commands.rs` | Windows SQLite locking、process startup、Sidecar/factory failure 和线程退出行为不能由 Linux contract 完全替代 | Windows Tauri artifact、可写/不可写 SQLite 路径、可控失败的 Sidecar/worker fixture | 分别执行 Browser transport 与 Tauri submit；验证立即返回 `QUEUED`；制造无效数据库路径、Sidecar 启动失败和 executor execution failure；查询 Job、错误字段和事件 | 正常 submit 不等待长 I/O；重复请求不重复 worker；调度/执行失败持久化为 `FAILED`，错误码和 `DOWNLOAD_FAILED` 事件可查询；无静默 queued Job | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-21 | Integration/Filesystem | Browser transport vs Tauri submit parity | `desktop/src-tauri/src/transport.rs`、`commands.rs`、Native Host | Windows Named Pipe、真实 Tauri command IPC 和 SQLite context 生命周期需实机确认 | Windows Desktop artifact、Native Host、Named Pipe backend、受控 SQLite | 通过 Browser archive request 和 Tauri invoke 分别提交同一 Tweet；比较 response state、request_id/job_id、重复提交、query、cancel 和 failure event 顺序 | 两入口均立即返回同一初始状态；复用同一 active Job；状态/错误/事件顺序一致；取消后不被 startup recovery 重新执行 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-15 | Filesystem/Runtime | R1 commit recovery facts and action validation | `desktop/src-tauri/src/executor.rs`、Storage `ArchiveService`、FileStore staging/archives | Linux 已实现 filesystem facts/action；真实 Windows rename/lock/restart/reparse 行为必须在目标文件系统确认 | 覆盖 DOWNLOADED+staging、DOWNLOADED+final、DOWNLOADED+neither、COMPLETE+final、COMPLETE+missing、mixed batch recovery；检查 SQLite state/event/error order | Windows workspace、受控 archive root、可制造异常退出和文件锁、旧/新 SQLite fixtures | Resume 不伪造 COMPLETE；final 存在时补写 COMPLETE；staging 可从 `tweet.json` 重做本地 commit；缺失 commit 可诊断；terminal 不重复处理；批量错误隔离 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-16 | Regression/Automation | WebdriverIO + @wdio/tauri-service native Windows smoke | desktop/wdio.conf.mjs、desktop/e2e/specs/dashboard.e2e.mjs、Tauri release artifact、WebView2 | Windows WebView2、Edge WebDriver 版本匹配、真实 native window 生命周期和 DOM 可见性不能由 Linux Node/Vite 检查替代 | Windows 11、WebView2、Node/npm、已安装 WDIO dependencies、npm run build:tauri 生成的 xarchive-desktop.exe | 在 Windows 执行 npm run test:e2e:windows --workspace desktop；验证 service external provider 启动/连接/关闭、Dashboard heading、main、导航和概览区域；保留 WDIO/service 日志 | service 自动准备匹配 Edge WebDriver；Tauri 窗口可连接并在测试结束退出；smoke 全部通过；失败时输出 binary/driver/port 原因 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-17 | Regression/Automation | tauri-plugin-wdio advanced API and log bridge | desktop/src-tauri/Cargo.toml, src-tauri/src/lib.rs, src-tauri/capabilities/wdio.json, tauri.conf.json, desktop/src/main.jsx, desktop/wdio.conf.mjs, desktop/e2e/specs/wdio-plugin.e2e.mjs | Linux 已完成 plugin 配置，但 browser.tauri.execute、mocking、窗口级 IPC 和前后端日志转发仍依赖 Windows WebView2/native window | Windows WebView2、Node/npm、WDIO dependencies、npm run build:tauri:wdio --workspace desktop 生成的专用 artifact | 在 Windows 运行 npm run test:e2e:windows:advanced --workspace desktop；通过 browser.tauri.execute 检查 window.wdioTauri，验证 execute/invoke interception、mock 生命周期、日志捕获和 session teardown；随后用普通 release artifact 运行 WQ-P1-16，确认不携带 debug-only plugin | 高级 API 可用且 teardown 无 mock-store warning；普通 release smoke 仍通过；日志不泄露凭据；插件不进入普通 release artifact | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-18 | Runtime/Filesystem/Packaging | 便携 `.exe` 目录布局与首次下载 setup | `portable.rs`、`config.rs`、`runtime.rs`、`commands.rs`、`aria2.rs`、`crates/xarchive-storage/src/file_store.rs`、`desktop/scripts/build-portable-windows.mjs`、`desktop/src/main.jsx` | exe 同目录解析、系统 Downloads 定位、跨卷 rename 提交、sidecar/Extension 实际分发和 Windows 文件权限只能在目标环境确认 | `build:portable:windows` 便携目录、无 `config.yaml` 首启状态、受控 `download` 目录、可选第二盘符 | 首启生成 `config/`（`config.yaml` + `archive.sqlite3`）、`cache/staging/`、`download/`、同级 `logs/`、`sidecar/`、`extension/`，不创建 `telegram/`；GUI 选择 portable 或系统 `Downloads/XArchive`；拒绝创建 `download/` 时 fallback；staging→最终目录提交；sidecar env 优先、fallback `config.yaml` | 布局与文档一致、`config.yaml` 持久化、fallback 生效、便携目录移动后无绝对路径残留、无路径逃逸 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-19 | Runtime/Regression | 日志等级与轮转 | `logging.rs`、`config.rs`、`desktop/src/main.jsx` 设置 UI、便携 `logs/` | 日志文件创建/删除、只读/权限行为和轮转时序依赖 Windows 文件系统 | 便携工作副本、`logs/` 可写与只读两种场景、可编辑 `config.yaml` | 验证 Release 默认 `info`/Debug 默认 `debug`、YAML/GUI 显式配置优先、等级过滤、`silent` 不写文件、`xarchive-*.log` 超过 `max_files`（1–100，默认 5）删除最旧、越界值回退默认并给出诊断 | 等级与轮转符合预期，权限错误可诊断且应用不崩溃 | P1 | no | `WINDOWS_VERIFICATION_PENDING` |


## 历史 WDIO 队列核验（2026-09-15）

- WQ-P1-16 当前为 WINDOWS_FAIL：本轮 ordinary smoke 已执行，driver 下载成功但 Dashboard session 创建三次均报 `DevToolsActivePort file doesn't exist`，0/1 spec 通过；失败路径留下 driver/端口，手工清理后才恢复环境。
- WQ-P1-17 当前为 WINDOWS_FAIL：本轮 advanced 已执行，2/2 spec 均未创建 session，三次重试均报 `DevToolsActivePort file doesn't exist`；advanced plugin API、日志桥接、mock cleanup 和自动 teardown 未能完成验收。
- 具体命令、输出摘要、进程清理证据和 Linux 后续动作见 `../development/windows-validation.md` 的 2026-09-15 最新章节。当前 driver 下载不是阻塞根因；下一轮先调查 Windows WebView2/Edge native session 启动和自动 cleanup，不使用 WINDOWS_VERIFICATION_BLOCKING。

## 便携 runtime 队列说明（2026-09-16）

- WQ-P1-18/WQ-P1-19 对应本轮便携 `.exe` 布局、`config/config.yaml`、首次下载目录 setup、日志等级与轮转实现；Linux Rust/Node 门禁和便携目录组装 smoke 已通过，Windows 验证前保持 `WINDOWS_VERIFICATION_PENDING`，不阻塞后续 Linux 开发。
- 对应 Windows 执行步骤见 [`windows-wdio-handoff.md`](windows-wdio-handoff.md) 的 PORTABLE-W-01/PORTABLE-W-02 与第 10 节。

## Linux 详细测试结果与 Windows 后续步骤（2026-09-15）

本轮 Linux source 为 branch `dev`、HEAD `0537d32c9b4d2ef71ec508467d75378a767a34e7`，working tree dirty（包含本任务前已有的 WDIO 实现与文档修改）。环境为 Node v26.7.0/npm 11.19.0、Rust/Cargo 1.98.0、Python 3.14.4；未发现 `FAIL_PRODUCT` 或 `FAIL_TEST`。

| 层级/项目 | 状态 | 证据摘要 |
|---|---|---|
| static：WDIO scripts/config/spec syntax | `PASS` | `node --check` 覆盖 service adapter、wdio config、advanced wrapper、build wrapper、plugin spec；WDIO config load 和 service adapter/provider/app binary/spec 探针通过 |
| unit/integration：Node workspace | `PASS` | `npm run check`、`npm run test`、`npm run build`；Extension 7/7，Desktop Node test 0/0 |
| unit/integration：Rust workspace | `PASS` | fmt、workspace/all-targets check、`wdio-e2e` feature check、workspace tests 150/150、strict Clippy |
| unit/integration：Sidecar | `PASS` | compileall 通过，pytest 10/10 |
| packaging/build：普通 Tauri | `PASS` | `npm run build:tauri --workspace desktop`，生成 `target/release/xarchive-desktop` |
| packaging/build：`wdio-e2e` Tauri | `PASS` | `npm run build:tauri:wdio --workspace desktop`，生成 Linux release binary |
| browser_e2e：独立 Browser Mode | `NOT APPLICABLE` | 当前仓库没有独立 Browser Mode 配置或脚本；未临时创建测试架构 |
| native_e2e：Linux WDIO | `PASS` | Ubuntu 26.04 使用 `webkitgtk-webdriver`（`/usr/bin/WebKitWebDriver`）后运行 `npm run test:e2e --workspace desktop`；Dashboard smoke 2/2，tauri-driver/session/teardown 完成 |

Windows 后续必须按以下顺序执行，Linux 结果不能替代其中任何一项：

1. 在 `E:ShiraishiVSCode WorkspaceTw2Tg` 检查同步 revision、工作树是否包含 dirty changes，并确认 Windows 依赖、`.venv`、WebView2 和 Edge driver 前置。
2. 设置并验证匹配的 `msedgedriver.exe`：`where.exe msedgedriver.exe`、`msedgedriver.exe --version`；优先使用已保存的 152.0.4191.66，并记录 PATH、版本和 SHA-256。
3. 运行 `npm ci --no-audit --no-fund`、`npm run check`、`npm run test`、`npm run build`、`npm run build:tauri:wdio --workspace desktop`，确认专用 artifact、`wdio` capability 和 guest JS 边界。
4. 设置 `WDIO_APP_BINARY`、`WDIO_ADVANCED=1`、`WDIO_CAPTURE_LOGS=1` 和 `WDIO_LOG_DIR`，运行 `npm run test:e2e:windows:advanced --workspace desktop`；确认 `window.wdioTauri`、`browser.tauri.execute`、invoke interception、mock/restore、前后端日志和非零失败退出码。
5. 无论 advanced 成功或失败，都检查 `xarchive-desktop`、`tauri-driver`、`msedgedriver` 进程以及 1420/4444/4445/9223 端口；不使用手工 `Stop-Process` 作为 PASS 证据。
6. 清理 advanced 专用环境变量，执行 `npm run build:tauri --workspace desktop` 和 `npm run test:e2e:windows --workspace desktop`；确认 Dashboard DOM、普通 artifact 不含 WDIO capability/guest JS，且无 `plugin:wdio|execute not allowed by ACL`。
7. 仅在上述 WDIO 不能稳定覆盖时，使用 Computer Use 验证原生文件选择器、托盘、通知、安装器、DPI/多显示器、拖放或视觉布局；工具不可用时记录 `BLOCKED_AUTOMATION` 并给出完整人工步骤。

## 已有 Windows 结果但不关闭当前队列的项目

以下结果已在历史报告中记录，但不能扩大为当前队列项目的完整 PASS：

- Windows workspace fmt/check/clippy/test、Node check/test/build、Sidecar pytest 和 Tauri Debug/Release build 已在 `5f18ae0` clean Linux commit 对应的 canonical E: 工作副本完成；项目 `.venv` 前置下 WQ-P0-01 为 `WINDOWS_PASS`。未设置 `PYTHON` 的额外 Rust 测试诊断仍复现 `NotRunning`，属于环境前置失败，不改写为项目代码失败。
- 2026-09-14 Windows 轮：项目 Python 前置下 Windows `144/144` workspace tests、Desktop `58` tests、Tauri Release build、Debug startup/cleanup 和 Tauri MCP backend/window smoke（`127.0.0.1:9223`）为 `PASS`；Tauri MCP WebView eval 层（DOM/截图/console/IPC invoke）为 `BLOCKED`（2 秒 timeout）。应用级 executor real-worker integration、应用级旧库迁移/重启、reparse/ACL/长路径、bundle/packaging 和真实外部账号仍然 `NOT RUN` / `BLOCKED` / pending。该轮验证对象包含 MCP Bridge、executor recovery 和 cancellation 的 dirty working tree；其后的 Linux 仅做 release `unused_mut` warning 的 warning-only 修复，不影响行为。
- aria2 artifact、版本/hash、loopback RPC、Unicode/空格路径、暂停/恢复、进程中断恢复和 .aria2 清理已有独立 Windows 证据；当前半永久化验证目录为 E:ShiraishiVSCode WorkspaceTw2Tgaria2。WQ-P1-01 的项目级 DownloadRouter、Desktop Job、403 refresh 和 transfer lifecycle 仍 pending。
- WQ-P1-12 的自动化安全边界子集已有通过记录；reparse/junction/长 JSON/Unicode 专项缺少可重复 Windows harness，仍 pending。
- Storage 库级 migration/reopen/profile 测试已有通过记录；WQ-P0-03 的 Desktop 文件数据库、应用重启和旧库升级仍 pending。

## 集中式 Windows handoff

进入 Windows validation phase 前，基于最终 diff、当前 Plan、变更模块和本队列合并重复场景，按以下类别执行：

1. **Build/Toolchain**：WQ-P0-01。
2. **Runtime**：Sidecar、Tauri、Job executor、production fallback 和进程清理，关联 WQ-P0-01、WQ-P1-05、WQ-P1-14。
3. **Filesystem**：WQ-P0-03、WQ-P1-12、WQ-P1-13、WQ-P1-14、WQ-P1-15、WQ-P2-02。
4. **Integration**：WQ-P0-02、WQ-P0-04、WQ-P1-01、WQ-P1-02、WQ-P1-04。
5. **Packaging**：WQ-P1-05、WQ-P2-01。
6. **Regression**：WQ-P1-03、WQ-P1-16 和所有本轮受影响的协议/存储/Sidecar 场景。

### 当前 BLOCKED / NOT RUN 手工验证步骤（2026-09-16）

以下项目在缺少对应 Windows 前置时跳过，不得记为 PASS：

1. **Native Host / Named Pipe / Registry**：若 Desktop endpoint、Named Pipe server、manifest 或 Registry artifact 未提供，记录 `NOT RUN`；artifact 可用后，注册 `.pipexarchive-v1`，分别以普通用户和管理员执行合法 archive/query、request_id 错配、非法 JSON、断线重连、多连接和退出清理，记录 ACL 与进程结果。
2. **真实账号 / Credential Manager / Telegram**：若无专用测试账号、空白 Edge profile、Credential Manager backend 或 Telegram test chat，记录 `BLOCKED`；前置具备后执行无媒体/单媒体/多媒体、认证失败、token 脱敏、保存/读取/删除、重启恢复和重试验证。
3. **GUI / WebView2 / accessibility**：若 Computer Use/native accessibility target 不可用，记录 `BLOCKED`；前置具备后在 100/125/150% DPI 执行最小窗口、Tab/Shift+Tab、Enter/Escape、Focus-visible、Narrator/NVDA、对比度和原生对话框步骤，保存截图/工具日志。
4. **Filesystem / reparse / ACL**：若无法创建受控旧库、文件锁、symlink/junction/reparse 或第二用户 fixture，记录 `NOT RUN`；前置具备后执行旧库迁移、异常退出恢复、跨盘提交、普通文件与 reparse 对比、跨用户读取和权限错误诊断。
5. **aria2 / R2 integration**：若 fresh media URL fixture、受控 media server 或 aria2 artifact 不完整，记录 `NOT RUN`；前置具备后依次验证 gallery-dl 成功、适用错误 fallback、403 refresh 不复用旧 URL、transfer polling、cancel/shutdown、`.aria2` 清理和最终 Job/file commit。

具体命令、人工交互要求、状态记录格式和错误分类以 [`windows.md`](windows.md) 为准。验证结束后将结果写入历史验证报告，并回到本文件更新当前队列状态。

### 最新重验结论（2026-09-15 17:20）

本轮从 Linux 最新 dirty working tree 经 /mnt/e 完成受控单向同步；Node/Rust 静态门禁和两套 Tauri build 通过。普通与 advanced WDIO 均实际启动 tauri-driver 并尝试创建 WebView2 session，但均因 `DevToolsActivePort file doesn't exist` 未进入 spec；失败路径均需手工清理 driver。两项保持 WINDOWS_FAIL，当前没有 WINDOWS_VERIFICATION_BLOCKING。
### 手动 driver 后的最新重验（2026-09-15 17:42）

Microsoft 官方 msedgedriver 152.0.4191.66 已下载到 E: 验证副本并经 PATH 发现；本轮 service 自动下载同版本 driver 成功，tauri-driver 也正常监听，但 advanced/ordinary session 均因 `DevToolsActivePort file doesn't exist` 失败。两项继续 WINDOWS_FAIL；手工清理残留 driver 不计为自动 teardown PASS。

### Blocker recovery follow-up (2026-09-15)

- `BLOCKED_ENV` 的实际阻塞已进一步收敛：Edge WebDriver 自身、tauri-driver `/status` 代理、直接启动 release app、driver PATH/显式路径和临时独立 identifier/profile 均已受控检查；最小 HTTP WebDriver probe 仍在 45 秒内超时，不能确认 session 创建。
- `BLOCKED_AUTOMATION` 的失败清理仍未修复：driver 残留可以按精确 PID 人工清理，但没有自动 teardown 证据；Computer Use 的 `sky` trusted RPC/native app inventory 仍不可用。
- 本轮没有修改业务代码、生产 Tauri 配置或依赖；E: 临时 probe/config/driver 副本已清理，普通 release artifact 已恢复。WQ-P1-16/WQ-P1-17 继续保持 `WINDOWS_FAIL`，待 Linux 后续处理测试生命周期/Windows native session 条件后重验。
- 人工操作指南与停止条件见 [`../development/windows-validation.md`](../development/windows-validation.md) 的 “BLOCKED_ENV / BLOCKED_AUTOMATION blocker recovery analysis” 章节。

### 当前 dirty source 的 Windows WDIO 重验（2026-09-15 20:18）

- Build/Toolchain、Sidecar、Rust workspace 150/150、专用/普通 Tauri build、WDIO syntax/config load 和普通 artifact 的 capability/guest-JS 隔离均为 `PASS`。
- WQ-P1-17 advanced 为 `FAIL`：2 workers/2 specs，0 passed；最终 `EXIT_CODE=1`，错误为 `session not created: DevToolsActivePort file doesn't exist`。
- WQ-P1-16 ordinary 为 `FAIL`：1 spec，0 passed；最终 `EXIT_CODE=1`，同一 native session 错误。
- 两个失败路径都留下 `tauri-driver`/`msedgedriver` 和 4444/4445 或动态端口监听；按精确 PID 手工清理后恢复为无相关进程/端口，不能视为自动 teardown 通过。
- Computer Use native inventory 不可用（`sky` 未配置、`apps=[]`），GUI/DPI/键盘/屏幕阅读器项目保持 `BLOCKED_AUTOMATION`；真实账号、Named Pipe、应用级 SQLite/restart/recovery、ACL/reparse/长路径、externalBin 和发布流程保持各自 `BLOCKED`/`NOT RUN`。

### 最新 Windows 重验结论（2026-09-16，Linux `dev` HEAD `cb1e5816bcef7480c46b255782c586682ceab16c`）

- 本轮 Linux source working tree clean；受控同步到 `E:ShiraishiVSCode WorkspaceTw2Tg` 后关键文件 SHA-256 `24/24` 匹配。Windows Node/Rust/Sidecar 门禁、普通/专用 Tauri build、Debug startup、便携 artifact 组装与首启 SQLite/log 初始化均 `PASS`。
- WQ-P1-17 advanced 的 native session、Dashboard `2/2`、plugin API/execute、mock/restore 为 `PASS`；WQ-P1-16 ordinary Dashboard `2/2` 为 `PASS`。但两次成功退出后均留下 `tauri-driver`/`msedgedriver` 和 4444/4445，手工清理才恢复，因此 WQ-P1-16/WQ-P1-17 整体继续 `WINDOWS_FAIL`，不能把 spec PASS 外推为完整生命周期 PASS。
- WQ-P1-18 仅完成便携目录组装和进程/SQLite/log 初始化 smoke；首次下载目录交互、`config.yaml` 持久化、fallback、跨卷提交保持 `WINDOWS_VERIFICATION_PENDING`。WQ-P1-19 日志等级/轮转保持 `WINDOWS_VERIFICATION_PENDING`。
- 当前没有 `WINDOWS_VERIFICATION_BLOCKING`。Linux 后续优先处理 WDIO service/tauri-driver 自动 teardown；详细命令、PID、状态边界和未执行项目见 [`../development/windows-validation.md`](../development/windows-validation.md) 的 2026-09-16 章节。

### Linux 修复与队列状态更新（2026-09-16）

- 根因定位：`@wdio/native-core` 的 `DriverProcess.stop()` 只对 tauri-driver 直接子进程执行 `SIGTERM`/`SIGKILL`，没有进程树清理（同库对 dev-server 使用 `taskkill /T /F`）；Windows 上 tauri-driver 及其 msedgedriver 子进程因此残留，4444/4445 持续监听。
- Linux 修复：`desktop/scripts/wdio-tauri-service.mjs` 的 launcher 在上游 teardown 前快照 driver PID 与驱动端口占用者，teardown 后对幸存进程执行进程树 kill（Windows `taskkill /T /F`、POSIX `SIGKILL`），无法清理时使运行失败；新增 `desktop/test/wdio-tauri-service.test.mjs`（`node --test` 8/8 通过），Linux Node 门禁 check/test/build 通过。业务 Rust、前端和生产 Tauri capability 无改动。
- WQ-P1-16/WQ-P1-17 修复后按 [`../development/cross-platform-validation.md`](../development/cross-platform-validation.md) §10 回到 `WINDOWS_VERIFICATION_PENDING`：下一轮 Windows 必须重跑 advanced 与 ordinary，且成功与失败退出路径均无 `tauri-driver`/`msedgedriver`/4444/4445 残留、无需手工 `Stop-Process`，才可改判 `WINDOWS_PASS`。
- WQ-P1-18/WQ-P1-19 保持 `WINDOWS_VERIFICATION_PENDING`，等待受控 Windows 交互/日志 fixture；本轮未将其提前改判。

### Windows 修复后重验结论（2026-09-16 11:20）

- Linux `dev` HEAD `a20027455651ef5f4f9faed527948bc1830375a6` 已按受控规则同步到 `E:ShiraishiVSCode WorkspaceTw2Tg`；源工作树在验证开始时 clean，未将 E: 的依赖、target、driver、日志或用户数据反向同步。
- Rust/Node/Sidecar/build 基线均通过；portable `.exe` 可启动并创建 `configarchive.sqlite3` 与同级日志，验证结束后进程已清理。
- WQ-P1-17 advanced：native session、Dashboard 2/2、plugin API、execute、mock/restore 均通过；但 `onComplete` 仍报告 PID `23148` 未在 5 秒内确认退出（tracked survivors `23148, 40748`），因此整体为 `WINDOWS_FAIL`。
- WQ-P1-16 ordinary：native Dashboard 2/2 通过；但 `onComplete` 仍报告 PID `45032` 未在 5 秒内确认退出（tracked survivors `49032, 45032`），因此整体为 `WINDOWS_FAIL`。
- 两次命令退出后的立即复查均未发现 `tauri-driver`、`msedgedriver`、`xarchive-desktop` 或 4444/4445/1420/9223 LISTEN；这只能说明最终环境恢复，不能抵销 teardown hook 的失败证据。未使用手工 Stop-Process 作为通过条件。
- 新增 `desktop/test/wdio-tauri-service.test.mjs` 在 Windows 为 `7 passed, 1 failed`：`killTree` 子进程终止测试约 5.3 秒后断言失败；`npm run test` 因同一桌面测试失败而为 `FAIL`。该失败需要 Linux 后续处理，验证阶段不修改代码。

### Linux 修复与队列状态更新（2026-09-16 第二轮）

针对上一节 Windows 复验暴露的 teardown hook 误报与测试失败，Linux 端完成第二轮修复（仅测试基础设施，业务代码零改动）：

- **根因一（hook 误报）**：Windows 上 `taskkill /T /F` 报告成功后，OS 尚未完成回收，`kill(0)` 在确认窗口内仍把已终止的 driver PID 判为存活；两次运行的事后复查（无 `tauri-driver`/`msedgedriver` 进程、无 4444/4445 监听）证实进程实际已清理。固定 alive-check 窗口在 Windows 双向不可靠：既可把已死进程误判为活（本轮），PID 复用时也可把活进程误判为死。
- **根因二（测试失败）**：`killTree` 在 Windows 等待 taskkill 自身的 `close` 事件才 resolve，此时受害进程可能已发出 `exit` 事件；测试在 `killTree` 之后才挂 `once(child, "exit")` 监听器，事件已被错过，等待直至超时后断言失败（约 5.3 秒），与进程是否被杀无关。
- **修复内容**：`waitForProcessGone` 改为「child `exit` 事件（仍持有句柄时）→ 轮询（确认窗口 5s→10s，poll 250ms）→ 超时后以 tracked driver 端口是否仍 LISTEN 做最终仲裁」；被复用的 stale PID 无端口监听时不再使运行失败。`portListenerCheck` 在 win32 用 netstat 检查；POSIX 依赖 `kill(0)` 轮询（SIGKILL 后幸存者只能是僵尸进程，不占用端口）。`killTree` 测试改为在 `killTree` 之前挂 `exit`/`close` 监听。
- **队列状态**：WQ-P1-16/WQ-P1-17 依据上述修复回到 `WINDOWS_VERIFICATION_PENDING`；历史 `WINDOWS_FAIL` 证据全部保留在 windows-validation.md。

Windows 重验要求（在原要求之上补充）：

1. 成功与失败退出路径均无 `tauri-driver`/`msedgedriver`/4444/4445 残留、无需手工 `Stop-Process`（不变）；
2. teardown hook 必须无错误完成：允许出现 safety-net 警告（需作为证据记录），但不得再出现「PID 未在确认窗口内退出」导致的运行失败——若警告后 tracked 端口仍 LISTEN 则仍为 FAIL；
3. `desktop/test/wdio-tauri-service.test.mjs` 在 Windows 以 `8 passed, 0 failed` 通过。


### Windows 最新 working-tree 验证结论（2026-09-16 22:14–22:48 +08:00）

- Linux `dev` source HEAD 为 `ec58a200eef3f19e9ae89419b5b605ecc4ff5fad`，working tree 含未提交修改；已从 `W:homeshiraishiVSCode WorkspaceTw2Tg` 单向同步到实际 E: 工作副本 `E:ShiraishiVSCode WorkspaceTw2Tg`。8 个关键源/目标 SHA-256 对匹配，目标本地依赖、driver、target、日志、用户数据和其它额外文件未删除。
- Windows Node check/test/build、Rust fmt/check/test、Sidecar compileall/pytest、普通与专用 Tauri build、portable 组装、WDIO syntax/config load 均通过；Rust workspace test 在显式设置可用 `PYTHON` 后为 158 passed。strict Clippy 仍失败于既有 `desktop/src-tauri/src/runtime.rs:77` `unused_mut`，故 WQ-P0-01 为 `WINDOWS_FAIL`。
- WQ-P1-17 advanced native E2E：Dashboard 2/2、plugin API/execute 1/1、mock/restore 1/1，退出码 0；WQ-P1-16 ordinary Dashboard 2/2，退出码 0。两次运行的上游 teardown 后各有 2 个 driver survivor，由项目 safety-net 进程树清理；最终 `tauri-driver`/`msedgedriver` 进程和 4444/4445 监听均为空。按 handoff 规则，WQ-P1-16/WQ-P1-17 改为 `WINDOWS_PASS`，并保留 safety-net 警告作为生命周期证据。
- portable build 生成 `dist-portable/XArchive/xarchive-desktop.exe`（17,460,736 bytes）；fresh 便携目录启动进程保持存活并创建 `config/archive.sqlite3`，`download/` 和 `telegram/` 未创建。当前脚本会在构建阶段预创建 `config/`、`cache/`、`logs/`、`sidecar/`，与旧 handoff 中“首启前这些目录不存在”的表述冲突；首次下载目录选择、`config.yaml` 持久化、fallback、跨卷提交和日志轮转仍未完整验证，WQ-P1-18/WQ-P1-19 保持 pending。
- 真实账号、Credential Manager、Telegram、Named Pipe/Registry/浏览器安装、reparse/ACL/长路径、executor/restart/recovery、installer/signing/updater，以及 GUI settings/DPI/keyboard/accessibility/Extension 缺失文件交互均未完成；GUI 自动化入口在本机不可用，按 `BLOCKED_AUTOMATION` 或 `NOT RUN` 记录，不用 WDIO Dashboard smoke 外推。

#### Linux reconciliation after 2026-09-16 22:14 working-tree validation（2026-09-17）

Previous Windows validation:

- `WQ-P0-01`: `WINDOWS_FAIL` — strict workspace Clippy failed only at pre-existing `desktop/src-tauri/src/runtime.rs:77` `unused_mut`; all other baselines passed.
- `WQ-P1-16` / `WQ-P1-17`: `WINDOWS_PASS` for the dirty working-tree run, with safety-net warning retained.
- `WQ-P1-18` / `WQ-P1-19` and new GUI/Extension items: `WINDOWS_VERIFICATION_PENDING` / `BLOCKED_AUTOMATION`.

Linux fix:

- Restricted the post-construction mutation of `RuntimeState` to Unix builds only: `let state` remains immutable on Windows, while Unix uses a `#[cfg(unix)]`-gated `let mut state` before assigning `transport_server`. This is a warning-only, behavior-invariant change; Unix transport startup, recovery scan, feature-gated fields and public APIs are unchanged.

Linux verification:

- `PASS`: `cargo fmt --all -- --check`; `cargo clippy -p xarchive-desktop --all-targets -- -D warnings`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p xarchive-desktop --all-targets --no-fail-fast` (71 passed); `cargo test -p xarchive-native-host --all-targets --no-fail-fast` (8 passed); `cargo check -p xarchive-desktop --all-targets`; `npm run check --workspace desktop`; `npm run test --workspace desktop` (8/8); `npm run check --workspace extension`; `npm run test --workspace extension` (7/7); `git diff --check`.

Current Windows status:

- `WQ-P0-01`: `WINDOWS_VERIFICATION_PENDING` — history retains the prior `WINDOWS_FAIL`; Linux fixed the Clippy cause locally, but the result must not be promoted to `WINDOWS_PASS` until strict workspace Clippy actually reruns on the fixed revision in Windows. Revalidation must use the project Python setup that resolved the Sidecar handshake `NotRunning` environment issue in the prior run.
- `WQ-P1-16` / `WQ-P1-17`: unchanged `WINDOWS_PASS`; current Linux diff does not touch the WDIO service adapter, specs, build scripts, capabilities, GUI entry or plugin registration.
- Other pending/blocked items: unchanged; GUI settings/visual/accessibility, Extension native loading, real accounts/credentials, IPC/filesystem fixtures, installer/packaging and complete portable setup still require Windows evidence or controlled fixtures.

### Windows 最新 working-tree 验证结论（2026-09-17 18:04–18:23 +08:00）

- Linux source 为 dev / HEAD a5f42ccc4b6d661e3cf80338b44859e5178e8480，working tree dirty；已从 W:homeshiraishiVSCode WorkspaceTw2Tg 单向同步到 E:ShiraishiVSCode WorkspaceTw2Tg。Robocopy exit 3，FAILED=0、MISMATCH=0；依赖、缓存、日志、target、用户数据和目标额外目录均保留。
- Windows Node check/test/build、Rust fmt/check/strict Clippy、Sidecar compileall/pytest、Tauri release/WDIO build、普通/advanced WDIO native smoke 均通过。完整 workspace Rust tests 为 163 passed, 1 failed，失败是 runtime 测试硬编码 /tmp 期望在 Windows 的路径分隔符差异；WQ-P0-01 更新为 WINDOWS_FAIL。
- PyInstaller spec 可生成 one-file worker，--help 与 JSONL probe 通过，但生成位置与 workflow 要求不一致；worker artifact 记录 WINDOWS_FAIL。Full portable 因缺少 sidecargallery-dl 记录 BLOCKED；Core portable 组装、manifest、首启和 SQLite/log 初始化 PASS。
- WQ-P1-16 ordinary Dashboard 2/2 与 WQ-P1-17 advanced 2 specs/4 tests 均 PASS；两次 teardown 的 2 个 driver survivor 被 safety-net tree-kill，最终进程和端口为空。设置/日志/aria2/Extension 真实 UI、DPI/键盘/读屏、真实账号、Named Pipe/Registry、executor/recovery、reparse/ACL/长路径、Full 分发、日志轮转和 installer 仍为 BLOCKED/NOT RUN/WINDOWS_VERIFICATION_PENDING。

#### Linux follow-up required

- 修正 desktop/src-tauri/src/runtime.rs Windows 单测中的 POSIX 硬编码期望，先做 Linux regression，再重跑 Windows 完整 workspace tests。
- 统一 sidecar/pyinstaller/xarchive-downloader.spec、.github/workflows/windows-worker-artifact.yml 与 build-portable-windows.mjs 的 worker 目录契约；提供受控 gallery-dl Full artifact 来源后重跑相关验证。
- 本轮未修改业务代码；其余队列按各自前置保持 pending/blocked/not run。


### 2026-09-17 Windows revalidation after Linux follow-up

本轮已按当前 roadmap 只执行受 Linux follow-up 影响的 Windows 项目。Linux source 为 dev / a5f42ccc4b6d661e3cf80338b44859e5178e8480，working tree dirty；同步到 E: 的 Robocopy exit 3，FAILED=0、MISMATCH=0，未删除 Windows 本地额外内容。

| ID | 最新状态 | 当前证据 | 后续处理 |
|---|---|---|---|
| WQ-P0-01 | WINDOWS_FAIL | 完整 cargo test 为 163/164；runtime Windows path fixture 仍失败。npm test 另有 portable path suffix 断言失败。 | Linux 修正两个测试的路径比较后先回归，再重跑完整 workspace 与 npm test |
| WQ-WORKER-BUILD-01 | WINDOWS_PASS | PyInstaller 6.22.3 生成 sidecardistxarchive-downloaderxarchive-downloader.exe；--help exit 0；SHA-256 449880D9D765901F099E1F40F12A0854E4CE3594970CDC9438002BEB3CCBE48E | 将本轮 artifact 清单/SHA-256 保留到发布或 CI 证据；业务代码无需因本项修改 |
| WQ-PACKAGE-FULL-01 | WINDOWS_BLOCKED | worker 和 Desktop release 已具备，但 sidecargallery-dl 缺失；portable script 明确拒绝组装 Full | 提供受控 gallery-dl.exe artifact/source 后重跑 Full manifest、目录清单、启动和 Sidecar smoke |
| WQ-PACKAGE-CORE-02 | WINDOWS_PASS | Core 包成功生成；manifest/release exe/nested worker 存在，gallery-dl 未包含；启动 8 秒创建 SQLite/logs 后关闭 | 仍需手工设置页、外部 gallery-dl 配置和 Extension 导入验收 |
| WQ-P1-16 / WQ-P1-17 | WINDOWS_PASS（KEEP_VALID） | 当前 diff 未命中 WDIO service/spec/capability，沿用上一轮有效结果 | 后续只在相关 service/spec/capability 改动时重验 |

本轮其余队列项目按原状态保留：未执行项继续标注 NOT RUN 或相应 BLOCKED/BLOCKED_AUTOMATION，不能因为 Core/worker 通过而提升为 PASS。Linux 后续重点是两个测试路径契约和 Full gallery-dl artifact 前置条件。

### 2026-09-17 Windows revalidation after R2 Linux fixture fixes

本轮基于 Linux 修复后的最新 dirty working tree 完成 Windows 重验。Linux source 为 dev / a5f42ccc4b6d661e3cf80338b44859e5178e8480，E: 同步 Robocopy exit 3，FAILED=0、MISMATCH=0；未删除 Windows 本地额外内容。

| ID | 最新状态 | 当前证据 | 后续处理 |
|---|---|---|---|
| WQ-P0-01 | WINDOWS_FAIL | Rust workspace tests、fmt/check/clippy 全部通过；Node 全套仍因 killTree Windows subprocess test 超时失败，30 个其它 Desktop tests 与 Extension 7/7 通过 | 调查 killTree 的 taskkill/exit-close 等待与测试生命周期；修复或明确归类后重跑 npm 全测 |
| WQ-WORKER-BUILD-01 | WINDOWS_PASS | 当前 spec 生成 nested one-dir exe；--help、JSONL hello/unknown-field probe、SHA-256 均通过 | 保留 artifact manifest/hash 作为 CI/release 证据 |
| WQ-PACKAGE-CORE-02 | WINDOWS_PASS | Core 包边界、manifest、nested worker 和 8 秒启动 SQLite/logs smoke 通过 | 仍需设置页外部 gallery-dl 配置与 Extension 导入手工验收 |
| WQ-PACKAGE-FULL-01 | WINDOWS_BLOCKED | worker/Desktop 已具备，但 sidecargallery-dl 缺失；portable script 明确拒绝 Full | 提供受控 gallery-dl.exe artifact/source 后重跑 Full manifest、启动和 Sidecar smoke |
| WQ-P1-16 / WQ-P1-17 | WINDOWS_PASS（KEEP_VALID） | 本轮 diff 未命中 WDIO service/spec/capability，沿用上一轮结果 | 仅在相关影响面变化或手工要求时重验 |

本轮其它项目继续按原状态保留：NOT RUN、BLOCKED、BLOCKED_AUTOMATION 和 NOT APPLICABLE 均不得提升为 PASS。Linux 后续重点为 killTree 测试生命周期、Full gallery-dl artifact 前置；未扩大为业务代码开发。
### 2026-09-17 最新 dirty working-tree Windows 验证更新

本轮源状态为 Linux `dev` / HEAD `a5f42ccc4b6d661e3cf80338b44859e5178e8480`，working tree dirty；同步到实际 E: 目录 `E:ShiraishiVSCode WorkspaceTw2Tg`，Robocopy `FAILED=0`、`MISMATCH=0`，未删除本地依赖、缓存、target、portable/validation artifacts、driver 或用户数据。完整证据见 [`../development/windows-validation.md`](../development/windows-validation.md) 的“Windows 最新 working-tree 验证结论（2026-09-17，本次实际验证）”。

| 队列项目 | 本轮最新状态 | 本轮证据与边界 | Linux 后续 |
|---|---|---|---|
| WQ-P0-01 Windows toolchain/baseline | `WINDOWS_PASS` | 实际 E: 目标目录的 Node check 通过、Desktop 31/31 + Extension 7/7、Rust fmt/check/test/clippy、Sidecar compileall/pytest 12/12、Tauri release build 均通过 | 保留历史失败记录；无需因本轮 baseline 修改业务代码 |
| WQ-P1-16 ordinary native WDIO | `WINDOWS_PASS` | Dashboard 2/2、exit 0；最终无 app/driver/4444/4445 残留 | 后续仅在 service/spec/capability 影响区变化时重验 |
| WQ-P1-17 advanced native WDIO | `WINDOWS_PASS` | 2 specs / 4 tests、plugin execute/mock/restore 通过；最终无 app/driver/4444/4445 残留 | 后续仅在 service/spec/capability 影响区变化时重验 |
| WQ-WORKER-BUILD-01 Windows worker artifact | `WINDOWS_BLOCKED` | 现有 E: one-dir exe 的 `--help` 因缺少 `_internalpython312.dll` 失败；本轮未执行 GitHub Actions artifact workflow | 重新生成完整 Windows artifact，记录版本、文件清单、SHA-256、`--help` 和 JSONL probe |
| WQ-PACKAGE-CORE-02 Core package | `WINDOWS_VERIFICATION_PENDING` | Core manifest/目录边界 PASS；但 worker runtime、设置页外部 gallery-dl 配置和 Extension 导入未验收 | 有效 worker 和手工 GUI 前置后重验 |
| WQ-PACKAGE-FULL-01 Full package | `WINDOWS_BLOCKED` | 脚本因缺少必需 `sidecargallery-dl` 明确失败 | 提供受控 gallery-dl artifact/source 后重跑 Full manifest、启动和 Sidecar smoke |
| WQ-REL-DB-01 / WQ-REL-SETTINGS-02 / WQ-REL-LOG-03 / WQ-REL-CONSOLE-04 | `WINDOWS_VERIFICATION_PENDING` / `NOT RUN` | release 启动 smoke 不能替代 SQLite 任务列表、设置页、日志页和全过程无控制台的人工验收 | 使用 Windows WebView2/人工或稳定 native spec 验收 |
| WQ-P1-12 security boundary | `WINDOWS_VERIFICATION_PENDING` | Rust/unit 与 native smoke 通过；reparse/ACL/长路径/真实进程边界尚未执行 | 准备可控 Windows filesystem/IPC fixture 后重验 |
| Installer/signing/updater | `NOT APPLICABLE` | 当前 `bundle.active=false`，没有 installer/certificate 前置 | 发布 bundle 启用后再建立单独 handoff |

本轮额外记录：通过 `Tw2Tg-CodexAlias` reparse alias 运行 Vite check/build 会产生路径解析 FAIL；切换实际 E: 目标目录后通过。该问题属于验证工作区路径，不修改项目代码。Computer Use 初始化仍被 `helper_unknown_error: setup refresh had errors` 阻塞，因此设置/日志/DPI/辅助技术等保持 `BLOCKED_AUTOMATION` 或 `NOT RUN`，不能由 Dashboard WDIO PASS 外推。
### Full portable retry after user-provided gallery-dl artifact (2026-09-17)

用户提供的 `E:ShiraishiVSCode WorkspaceTw2Tggallery-dlgallery-dl.exe` 已在 E: 验证副本中验证并 staging 到 `sidecargallery-dlgallery-dl.exe`。artifact 版本 `1.32.12`，SHA-256 `0B36AE6734ED41E12BE6BE1B33D3165A450B3E0A811FC1B8C664C032F7F13B2C`，`--version`/`--help` 均通过。

| 队列项目 | 重试结果 | 说明 |
|---|---|---|
| WQ-PACKAGE-FULL-01 Full package | `WINDOWS_VERIFICATION_PENDING` | Full portable assembly、manifest、目录边界和 8 秒启动/SQLite/log 清理 smoke 通过；`download/`、`telegram/` 未创建 |
| WQ-WORKER-BUILD-01 Windows worker artifact | `WINDOWS_FAIL` | bundled worker `--help` 仍因缺少 `_internalpython312.dll` 失败；当前 artifact 无法完成 worker runtime 验收 |
| Full Sidecar handshake/受控下载 | `BLOCKED` | 依赖有效 worker artifact；本次未将 gallery-dl 的 PASS 外推为完整 Full runtime PASS |

Linux 后续：重新生成完整 Windows worker one-dir artifact，确认 `_internalpython312.dll` 等依赖齐全后，重跑 worker help、JSONL handshake、Full portable startup 和受控下载。用户提供的 gallery-dl artifact 仅存在于 E: 验证副本，不回写 Linux source。

### 2026-09-17 latest Windows result reconciliation

最新 Windows 结果确认：Full portable 使用用户提供的 `gallery-dl.exe` 后，目录组装、manifest 和启动 smoke 通过，但 bundled worker 执行 `--help` 仍因缺少 `_internalpython312.dll` 失败。该结果属于当前 Windows working copy 的真实 runtime FAIL，不能由 Linux compile 或 PyInstaller spec 静态检查替代。

Linux follow-up 已完成：删除 `sidecar/pyinstaller/entrypoint.py` 的重复 `main()` 调用；Windows worker workflow 在 smoke 前检查 one-dir artifact 必须包含 `_internalpython312.dll`；Core manifest 的 `extension.user_importable` 改为 `false`，与当前设置页移除本地导入入口的实现一致。

| 项目 | 当前状态 | 重验条件 |
|---|---|---|
| WQ-WORKER-BUILD-01 | `WINDOWS_VERIFICATION_PENDING` | 重新运行 Windows worker workflow；artifact 目录必须包含 `_internalpython312.dll`，然后通过 `--help`、JSONL hello/unknown-field probe 和 SHA-256 |
| WQ-PACKAGE-CORE-02 | `WINDOWS_VERIFICATION_PENDING` | 使用新 worker artifact 重组 Core；确认 manifest `user_importable=false`、无 bundled gallery-dl，并完成设置页/Extension 外链手工检查 |
| WQ-PACKAGE-FULL-01 | `WINDOWS_VERIFICATION_PENDING` | 使用新 worker 与受控 gallery-dl artifact 重组 Full；完成 worker startup、Sidecar hello/ready、受控下载和启动清理 |

本轮没有 `WINDOWS_VERIFICATION_BLOCKING`。WDIO ordinary/advanced 历史通过结果仅在相关 service/spec/capability 无交集时保持有效；本轮未修改其影响区。若 Windows 自动化或 artifact workflow 不可用，标记 `WINDOWS_BLOCKED`/`BLOCKED_AUTOMATION` 并执行现有手工步骤，不得标记 PASS。
### 2026-09-19 U7 latest Windows validation reconciliation

本轮 Linux source 为 `feature/u7-desktop-production-integration` / HEAD `79232f24641f88e11b077611723f0af5cd92760e`，working tree dirty；已从 Linux source 单向同步到实际 `E:ShiraishiVSCode WorkspaceTw2Tg`。Robocopy `FAILED=0`、`MISMATCH=0`，没有删除 E: 本地依赖、缓存、target、driver、worker/gallery-dl 或 validation artifacts。完整实际结果见 [`../development/windows-validation.md`](../development/windows-validation.md) 的“2026-09-19 U7 latest dirty working-tree Windows validation”。

| 队列项目 | 本轮最新状态 | 本轮证据与边界 | Linux 后续 |
|---|---|---|---|
| WQ-P0-01 Windows toolchain/baseline | `WINDOWS_VERIFICATION_PENDING` | Node check 33/33 + 7/7、fmt、download 23+7、supervisor 5/5 通过；上一轮 workspace check/test/clippy/Tauri build 因 `transport.rs` Windows `PathBuf` 条件导入失败；Linux 已修复并完成 82/82 Desktop、workspace、clippy 回归 | 用当前 revision 重跑 Windows workspace/Tauri；确认 `PathBuf` FAIL 不再复现 |
| WQ-U7-01 / WQ-ARCH-01 Sidecar v2 packaged handshake | `FAIL` / `BLOCKED` | E: worker `--help`/DLL 通过，但 v2 hello 返回 `protocol_version:1`/`UNSUPPORTED_PROTOCOL_VERSION`；当前 artifact 是旧 v1 | 生成当前 v2 source 的 Windows worker，再重跑 hello/capability/v1 rejection/extract |
| WQ-U7-02 aria2 transfer | `BLOCKED` | aria2c、Desktop current artifact、受控 media server/fixture 未形成；download crate 23+7 只证明 Rust contract | 准备 aria2c 与受控 fixture 后重跑 multi-GID/progress/cancel/cleanup |
| WQ-U7-03 expired URL refresh | `BLOCKED` | 依赖当前 Desktop runtime、aria2c、signed URL expiry fixture | 重跑一次 refresh/new GID/collection-change 场景 |
| WQ-U7-04 staging/ArchiveService commit | `BLOCKED` | Windows Desktop compile FAIL；file lock/reparse fixture 未执行 | 修复 compile 后执行 path/file/lock/reparse/commit 验收 |
| WQ-U7-05 executor cancel/shutdown/recovery | `BLOCKED` | Windows Desktop artifact、worker、aria2 和 restart/SQLite fixture 不可用 | 修复 compile 并准备 controlled crash/restart/late-result fixture 后重验 |
| WQ-WORKER-BUILD-01 Windows worker artifact | `WINDOWS_FAIL` | one-dir `--help` exit 0、`_internalpython312.dll` 存在，但 v2 probe 显示 artifact/source 版本不一致 | 重新生成当前 v2 artifact，记录文件清单/hash/hello/unknown-field |
| WQ-PACKAGE-CORE-02 Core package | `WINDOWS_VERIFICATION_PENDING` | Core manifest/目录边界 PASS；完整 current-source runtime 被 Desktop compile 与 worker v2 artifact 阻塞 | 修复 compile、更新 worker 后重跑 startup、设置页和 Extension 外链 |
| WQ-PACKAGE-FULL-01 Full package | `WINDOWS_VERIFICATION_PENDING` | Full manifest/目录边界使用 E: 受控 gallery-dl PASS；完整 runtime 未验收 | 更新 Desktop/worker 后重跑 Full startup、Sidecar v2 和受控下载 |
| Sidecar full pytest | `WINDOWS_VERIFICATION_PENDING` | 上一轮 29/33；4 个 POSIX `#!/bin/sh` fake executable 在 Windows 触发 `WinError 193`；Linux 已改为 `sys.executable` 驱动的 Python fixture，Sidecar Linux full pytest 33/33 | 用当前 revision 重跑 Windows full pytest；确认 fixture 不再触发 `WinError 193` |
| WQ-P1-16 / WQ-P1-17 native WDIO | `WINDOWS_PASS（KEEP_VALID）` | 本轮未修改 service/spec/capability，沿用历史 session 证据；不外推为 U7 production runtime PASS | 仅在影响面变化或手工要求时重验 |

本轮新增的明确 Windows 平台问题是 `desktop/src-tauri/src/transport.rs` 的 `PathBuf` 条件导入；它属于项目跨平台代码问题，需 Linux 开发阶段处理。其余当前 FAIL/阻塞分别是 Windows 测试 fixture、旧 worker artifact 和缺失的 Desktop/aria2/真实 filesystem 前置，不应通过扩大业务开发范围解决。

### 2026-09-19 Linux reconciliation after U7 Windows results

本轮根据 Windows dirty-working-tree 结果重新评估 Plan，没有继续旧的 GUI/WDIO 计划。已处理两个明确属于 Linux 可修复范围的问题：

- `desktop/src-tauri/src/transport.rs` 将 `PathBuf` 从 Unix-only import 中移出，Windows `BrowserTransportAdapter` 可无条件使用该类型；
- `sidecar/tests/test_protocol_v2.py` 中两个 POSIX `#!/bin/sh` fake executable 改为由 `sys.executable` 启动的 Python fixture，并为 extraction command 增加测试专用 `executable_args`，生产默认仍为空。

Linux verification：`cargo fmt --all -- --check`、`cargo check --workspace --all-targets`、Desktop Rust 82/82、workspace Rust tests/doc-tests、strict Clippy、Sidecar compileall/pytest 33/33、Node check、Desktop 33/33、Extension 7/7、Desktop/Extension build 和 `git diff --check` 均通过。

最新 Windows 结果的状态调整如下：

- `WQ-P0-01`：从上一轮 `WINDOWS_FAIL` 回到 `WINDOWS_VERIFICATION_PENDING`，等待当前 revision 的 Windows workspace/Tauri 重验；
- Sidecar full pytest fixture：从测试 fixture `FAIL` 回到 `WINDOWS_VERIFICATION_PENDING`，等待当前 revision 的 full pytest；
- `WQ-U7-01` / `WQ-ARCH-01`、`WQ-WORKER-BUILD-01`：仍为 Windows artifact/protocol FAIL 或 BLOCKED，原因是已有 packaged worker 返回 v1/`UNSUPPORTED_PROTOCOL_VERSION`，必须重新生成 v2 artifact；
- `WQ-U7-02` 至 `WQ-U7-05`：继续 `WINDOWS_VERIFICATION_PENDING`/`WINDOWS_BLOCKED`，因为 aria2c、真实 signed URL、Windows file lock/reparse、Desktop runtime 和 restart/recovery 尚未执行；
- Core/Full manifest 边界可以保留该 revision 的 PASS 证据，但 current-source runtime 不能标记 PASS；
- 历史 WDIO `WINDOWS_PASS（KEEP_VALID）` 仅适用于未受影响的 service/spec/capability 范围，不能替代 U7 production runtime 验收。

本轮没有 `WINDOWS_VERIFICATION_BLOCKING` 项目。下一次 Windows handoff 的最小范围是：当前 revision 的 workspace/Tauri build、Sidecar full pytest、重新生成并探测 v2 worker，然后才执行 U7-01 至 U7-05 runtime 项目。
### 2026-09-19 current-revision Windows revalidation

本轮已重新同步并验证 Linux 最新 dirty revision。transport.rs Windows PathBuf 问题已在 Linux 修复后通过 Windows workspace/Tauri 重验；Sidecar fixture 与 worker packaging 仍有独立问题。

| 队列项目 | 本轮最新状态 | 证据 | Linux 后续 |
|---|---|---|---|
| WQ-P0-01 | WINDOWS_FAIL | Node 33/33 + 7/7、Rust 190 tests、fmt/check/clippy、Tauri build 均通过；full Sidecar pytest 仍 31/33 | 修复剩余 2 个 Windows POSIX fake fixtures 后重跑 |
| WQ-U7-01 / WQ-ARCH-01 | WINDOWS_VERIFICATION_PENDING | 当前新生成 worker 仍由 xarchive_downloader.main 启动 v1；v2 hello/shutdown 返回 UNSUPPORTED_PROTOCOL_VERSION；Linux 已切换 spec 到专用 v2 entrypoint | 在 Windows 重建当前 source artifact，再重跑 hello/capability/v1 rejection/extract |
| WQ-WORKER-BUILD-01 | WINDOWS_VERIFICATION_PENDING | PyInstaller one-dir、DLL、--help PASS，但上一轮 v2 protocol FAIL；Linux 已新增 v2 entrypoint/v1 fallback 分离 | 生成当前 v2 worker artifact，再重跑 hello/unknown-field/shutdown 并记录 hash |
| WQ-U7-02 至 WQ-U7-05 | BLOCKED | aria2c、signed URL、Windows filesystem/restart fixtures 或可用 v2 worker 不足 | 准备对应受控 fixtures 后重验 |
| WQ-PACKAGE-CORE-02 | WINDOWS_VERIFICATION_PENDING | Core boundary/startup smoke PASS；worker v2 和 GUI 流程未验收 | 更新 worker 后重跑 runtime、设置页和 Extension 外链 |
| WQ-PACKAGE-FULL-01 | WINDOWS_VERIFICATION_PENDING | Full boundary/startup smoke PASS；v2 Sidecar/受控下载未验收 | 更新 worker 后重跑 Full runtime |
| Sidecar full pytest | WINDOWS_VERIFICATION_PENDING | 上一轮 31/33；剩余失败是 Windows WinError 193；Linux 已将两个 fixture 改为 `sys.executable` 驱动，Linux full pytest 33/33 | 使用当前 revision 重跑 Windows full pytest，确认不再触发 WinError 193 |
| WQ-P1-16 / WQ-P1-17 | WINDOWS_PASS（KEEP_VALID） | service/spec/capability 未变化 | 影响面变化时再重验 |

本轮没有业务代码修改；历史 FAIL 保留，不能因 Rust/Tauri baseline PASS 自动提升 U7 runtime 项。

### 2026-09-19 Linux reconciliation after current-revision Windows revalidation

本轮基于最新 Windows 事实继续执行了两个明确的 Linux follow-up，没有重复已通过的 Rust/Tauri baseline，也没有扩展到 GUI/WDIO 或 U7 runtime：

- `sidecar/tests/test_extraction_only.py` 中剩余两个无扩展名 POSIX fake executable 改为 `sys.executable` 驱动的 Python fixture；
- `sidecar/pyinstaller/xarchive-downloader.spec` 改为使用专用 `entrypoint_v2.py`；新增 `entrypoint_v1.py` 保留 migration fallback；v2 entrypoint 支持 `--gallery-dl` 并启动 `worker_v2`，避免当前 artifact 继续误启动 v1。

Linux verification：Sidecar compileall/pytest 33/33、PyInstaller entrypoint/spec syntax、Rust fmt/check/strict Clippy、workspace Rust tests/doc-tests、Desktop 82/82、Node check、Desktop 33/33、Extension 7/7、Desktop/Extension build 和 `git diff --check` 均通过。

状态重新评估：

- `WQ-P0-01`：上一轮 Windows `PathBuf` compile FAIL 已通过当前 revision 的 Windows workspace/Tauri 重验，保持历史 PASS 证据；本轮没有新增影响区，不重复执行；
- Sidecar full pytest：`WINDOWS_VERIFICATION_PENDING`，等待当前 revision full pytest；
- `WQ-U7-01` / `WQ-ARCH-01` / `WQ-WORKER-BUILD-01`：`WINDOWS_VERIFICATION_PENDING`，Linux v2 entrypoint 修复不能替代 Windows artifact 重建和 v2 probe；
- `WQ-U7-02` 至 `WQ-U7-05`：继续 `WINDOWS_VERIFICATION_PENDING` 或 `WINDOWS_BLOCKED`，因为 aria2c、signed URL、Windows file lock/reparse、U7 runtime 和 restart/recovery 仍未完成；
- Core/Full manifest、startup smoke 和历史 WDIO KEEP_VALID 证据不升级为 U7 production runtime PASS。

本轮没有 `WINDOWS_VERIFICATION_BLOCKING` 项目。下一次 Windows 最小 handoff 是：重建并探测 v2 worker、运行 full Sidecar pytest，然后执行 U7-01 至 U7-05 的实际 runtime 验证。
### 2026-09-19 current-revision Windows revalidation after v2 worker and fixture follow-up

本轮最新状态已从 Linux source 单向同步到实际 E:ShiraishiVSCode WorkspaceTw2Tg。Linux source 为 feature/u7-desktop-production-integration / HEAD 79232f24641f88e11b077611723f0af5cd92760e，working tree dirty；Robocopy exit 3，Files copied=150、FAILED=0、MISMATCH=0。当前 worker、spec、v2 entrypoint 和 extraction fixture 已用代表性 SHA-256 复核匹配。详细证据见 ../development/windows-validation.md 的本节。

| 队列项目 | 本轮最新状态 | 本轮证据与边界 | Linux 后续 |
|---|---|---|---|
| WQ-P0-01 Windows toolchain/baseline | WINDOWS_PASS | Node check 33/33 + 7/7、Rust fmt/check/clippy、Rust 190 tests、Sidecar pytest 33/33、Tauri release build 均通过 | 无 baseline follow-up |
| WQ-U7-01 / WQ-ARCH-01 Sidecar v2 packaged handshake | WINDOWS_PASS（范围受限） | 当前 v2 worker hello 返回 ready/capabilities；unknown field 返回 INVALID_COMMAND；shutdown probe exit 0 | 准备真实 extraction fixture 后扩展验证 |
| WQ-WORKER-BUILD-01 Windows worker artifact | WINDOWS_PASS | PyInstaller one-dir 使用 entrypoint_v2.py；--help 通过；_internalpython312.dll 存在；v2 JSONL probe 通过 | 无 packaging follow-up；保留 artifact/hash 记录 |
| WQ-U7-02 aria2 transfer | BLOCKED | 未形成 aria2c、media server、multi-GID/progress/cancel 受控前置 | Linux/验证环境准备 aria2c 和受控 fixture |
| WQ-U7-03 expired URL refresh | BLOCKED | 缺少 signed URL expiry 与 collection-change fixture | 准备过期 URL/刷新和新 GID fixture |
| WQ-U7-04 staging/ArchiveService commit | BLOCKED | 未执行 Windows file lock/reparse/path/commit fixture | 准备 Windows filesystem fixture 后重验 |
| WQ-U7-05 executor cancel/shutdown/recovery | BLOCKED | 未执行 restart/SQLite/late-result fencing fixture | 准备 controlled crash/restart/recovery fixture |
| WQ-PACKAGE-CORE-02 Core package | WINDOWS_VERIFICATION_PENDING | Core manifest/目录边界和 worker assembly 通过；完整 runtime、设置页和 Extension 外链未验收 | 补做 Core runtime/manual boundary |
| WQ-PACKAGE-FULL-01 Full package | WINDOWS_VERIFICATION_PENDING | Full manifest/目录边界、E: gallery-dl、worker assembly 和 8 秒 startup smoke 通过；真实下载/完整 runtime 未验收 | 补做 Full Sidecar/extraction/download/runtime |
| Sidecar full pytest | WINDOWS_PASS | 当前 E: full pytest 33/33；POSIX fixture WinError 193 不再复现 | 无 fixture follow-up |
| WQ-P1-16 / WQ-P1-17 native WDIO | WINDOWS_PASS（KEEP_VALID） | service/spec/capability 未变化，沿用历史 session 证据 | 影响面变化时再重验 |

本轮没有 WINDOWS_VERIFICATION_BLOCKING。需要 Linux 后续处理的是 U7 runtime 前置与未执行的 GUI/Native Host/installer 等验收，不是为了制造 PASS 而扩大业务开发。

### 2026-09-19 current-revision Windows result reconciliation

最新 Windows 验证已关闭上一轮 worker/fixture/baseline 阻塞，但只在明确验证范围内关闭：

| 队列项目 | 当前状态 | 证据边界 | 后续 |
|---|---|---|---|
| WQ-P0-01 | `WINDOWS_PASS` | Node 33/33 + 7/7、Rust 190 tests、fmt/check/strict Clippy、Tauri release build、Sidecar pytest 33/33 | 无 baseline follow-up |
| WQ-U7-01 / WQ-ARCH-01 | `WINDOWS_PASS`（范围受限） | packaged v2 `hello` 返回 ready/capabilities；unknown field 返回 `INVALID_COMMAND`；shutdown exit 0 | 需要真实 extraction fixture，不能外推 production runtime |
| WQ-WORKER-BUILD-01 | `WINDOWS_PASS` | entrypoint_v2 one-dir artifact、`_internalpython312.dll`、`--help`、v2 JSONL probe 均通过 | 保留 artifact/hash 记录 |
| Sidecar full pytest | `WINDOWS_PASS` | 当前 revision 33/33；WinError 193 不再复现 | 无 fixture follow-up |
| WQ-U7-02 | `WINDOWS_BLOCKED` | aria2c、media server、multi-GID/progress/cancel fixture 未形成 | 准备 aria2c 和受控 media fixture |
| WQ-U7-03 | `WINDOWS_BLOCKED` | signed URL expiry、collection-change fixture 未执行 | 准备 refresh/new-GID fixture |
| WQ-U7-04 | `WINDOWS_BLOCKED` | file lock/reparse/path/staging→commit fixture 未执行 | 准备 Windows filesystem fixture |
| WQ-U7-05 | `WINDOWS_BLOCKED` | restart/SQLite/late-result fencing fixture 未执行 | 准备 controlled recovery fixture |
| WQ-PACKAGE-CORE-02 | `WINDOWS_VERIFICATION_PENDING` | assembly/boundary/worker smoke 通过；完整 runtime、设置页和 Extension 外链未验收 | 补做 Core runtime/manual boundary |
| WQ-PACKAGE-FULL-01 | `WINDOWS_VERIFICATION_PENDING` | assembly/boundary、受控 gallery-dl、worker 和 startup smoke 通过；真实下载/完整 runtime 未验收 | 补做 Full Sidecar/extraction/download/runtime |
| WQ-P1-16 / WQ-P1-17 | `WINDOWS_PASS（KEEP_VALID）` | service/spec/capability 未变化，沿用历史 session 证据 | 影响面变化时重验 |

本轮没有 `WINDOWS_VERIFICATION_BLOCKING`。`WINDOWS_PASS` 仅表示对应项目和验证范围已实际通过；不得把 packaged handshake、assembly 或 startup smoke 扩展解释为 U7 production runtime PASS。

### 2026-09-19 U8 current Linux revision handoff

U8 已在 Linux 完成旧入口和旧下载路径删除，并通过适用的 workspace Rust、Desktop、Sidecar、Node、Extension、fmt/check/clippy/build 验证。当前 Windows queue 只保留平台专属或目标环境依赖项目；不得因为 U7 历史 packaged worker PASS 而把 U8 current revision 的完整 runtime 标记为 PASS。

| ID | 当前状态 | 原因 | 手工验证步骤 | 预期结果 |
|---|---|---|---|---|
| WQ-U8-01 | `WINDOWS_VERIFICATION_PENDING` | U8 删除了 v1 command/event、`DownloadRouter`、同步 `archive_tweet` 和重复 PyInstaller entrypoint；需要确认 current v2-only artifact 与 Desktop command surface | 从 Linux source 同步 current working tree；重建 one-dir worker；执行 `--help`；发送 v2 `hello`、未知字段、legacy v1 `download`、`shutdown`；检查 Tauri command registry 和 worker artifact 内容 | v2 handshake/capabilities 成功；未知字段和 legacy v1 command 被拒绝；shutdown 正常退出；不存在 v1 entrypoint、旧 download event 或 `archive_tweet` command |
| WQ-U7-02 | `WINDOWS_BLOCKED` | 缺少受控 aria2c/media-server/multi-GID fixture；Linux driver contract 不能替代 Windows process/RPC 证据 | 准备 `aria2c.exe` 和 loopback media server；提交单/多媒体 plan；记录 GID、progress、complete、timeout、cancel、shutdown、removed；检查 aria2 与 staging 子进程 | plan 顺序和 stable identity 保持；progress 单调；GID/进程退出；partial 与 `.aria2` 清理；错误映射稳定 |
| WQ-U7-03 | `WINDOWS_BLOCKED` | 缺少 signed URL expiry、refresh 和 collection-change fixture | 首次 URL 返回 401/403/expired；确认一次 extraction refresh、旧 GID remove、新 plan/new GID；再执行集合变化与普通文件错误场景 | 仅 URL expiry refresh 一次；集合变化返回 `EXTRACTION_RESULT_CHANGED`；普通错误、cancel、timeout、权限/磁盘错误不 refresh |
| WQ-U7-04 | `WINDOWS_BLOCKED` | 缺少 Windows lock/reparse/path/staging-to-commit fixture | 在 Unicode/空格/长路径目录执行归档；制造打开句柄、junction/symlink/reparse、缺失/多余文件和 size/hash mismatch；执行 staging→final commit | unsafe path/reparse、缺失/多余/不匹配文件被拒绝；合法文件 commit 成功；SQLite metadata、media、Job state/event 一致 |
| WQ-U7-05 | `WINDOWS_BLOCKED` | 缺少可控 cancel/shutdown/crash/restart/SQLite recovery fixture | 在 extraction、aria2 transfer、staging commit 三阶段分别 cancel、shutdown、强制退出并重启；查询 late result、execution spec、attempt 和 recovery scan | `CANCELLED`/`INTERRUPTED` 语义稳定；late result 不覆盖终态；execution spec 可重载；不重复归档；无残留 Sidecar/aria2 进程或锁 |

以上 `WINDOWS_BLOCKED` 项本轮跳过自动执行，手工步骤已提供；待 fixture、账号或人工 Windows 环境可用时按步骤执行并回写本文件/`windows-validation.md`。本轮没有 `WINDOWS_VERIFICATION_BLOCKING`。

### 2026-09-20 U9 ComponentManager handoff

U9 Linux scope 已完成：`desktop/src-tauri/src/components.rs` 提供固定 embedded catalog schema、组件字段校验、目录 artifact deterministic SHA-256/size、license/layout/probe 检查、safe relative path、symlink/special-file 拒绝、`.part` staging、atomic activation、current/previous marker 和 rollback。当前 catalog 为空是安全边界，因为 U11 尚未生成真实版本化 release assets/hash；本轮不执行动态网络下载或 Windows 专属验证。

| ID | 类别 | 当前状态 | Windows 原因/阻塞 | 手工验证步骤 | 预期结果 |
|---|---|---|---|---|---|
| WQ-U9-01 | Catalog/Integrity | `WINDOWS_VERIFICATION_PENDING` | Windows artifact、权限、大小写/路径语义和真实 catalog asset 需目标环境确认 | 同步 current Linux source；构建/启动 Desktop；读取 embedded catalog；使用合法目录、hash mismatch、size limit、缺失 license/probe、重复 id、非法 traversal/path fixture 分别调用 ComponentManager | 只接受 schema/version/hash/size/layout/license/probe 全部通过的 artifact；错误分类稳定；不存在动态 `latest` 或 unsigned remote manifest |
| WQ-U9-02 | Filesystem/Activation | `WINDOWS_BLOCKED` | Windows rename、句柄占用、ACL、junction/reparse 和 atomic marker 行为无法由 Linux 外推；本轮无 Windows fixture | 在中文/空格路径安装两个版本；制造 `.part`、打开旧版本句柄、current marker 中断、junction/reparse 和只读目录；执行 activate、current 查询、rollback | 旧版本不被破坏；激活只切换已校验目录；`.part` 不被视为 active；中断后可诊断并 rollback；reparse/special file 被拒绝 |
| WQ-U9-03 | Runtime/Probe | `WINDOWS_BLOCKED` | Worker/gallery-dl/aria2 的真实 Windows executable probe 和权限/退出码尚未具备受控 asset | 使用 U11 生成的固定 worker/gallery-dl/aria2 assets；执行 catalog probe、版本输出、非零退出、缺失 DLL 和权限拒绝场景 | 版本/protocol compatibility 正确；probe 失败不激活；错误不泄漏路径外敏感信息 |
| WQ-U9-04 | Packaging/Parity | `WINDOWS_VERIFICATION_PENDING` | Full/Core portable assembly、license 文件和最终 asset/hash 尚未由 U11 生成 | 分别构建 Core/Full；检查 `package-manifest.json`、components 目录、license/notices、catalog 与实际文件 hash/size/layout；比较 Offline Bundle parity | manifest/catalog/实际目录一致；Core/Full 边界正确；无 cache/credential/未知文件；license/notices 完整 |

本轮 `WQ-U9-02`、`WQ-U9-03` 因 Windows filesystem/executable fixture 不可用而跳过自动执行，按上述手工步骤保留；本轮没有 `WINDOWS_VERIFICATION_BLOCKING`。U9 Linux 测试结果写入 `status.md`，不将 Linux PASS 外推为 Windows PASS。

### 2026-09-20 U10 Core Bootstrap handoff

U10 Linux scope 已完成：Desktop 新增 `get_component_bootstrap_status` command 和 Settings Core Bootstrap 状态卡；启动后可读取固定 embedded catalog、本地 active marker、缺失组件和诊断 message。现有 `complete_download_setup` 继续处理首次 portable/system Downloads 选择，并在成功后重建 executor runtime。Bootstrap 不执行动态网络下载、不绕过 ComponentManager 校验；当前空 catalog 明确表示等待 U11 release assets。

| ID | 类别 | 当前状态 | Windows 原因/阻塞 | 手工验证步骤 | 预期结果 |
|---|---|---|---|---|---|
| WQ-U10-01 | Runtime/Bootstrap | `WINDOWS_VERIFICATION_PENDING` | Core `.exe` 启动、WebView2 UI、portable root 和本地 marker 行为需目标环境确认 | 使用 current Windows Core artifact 启动；打开 Settings → Core Bootstrap；读取 catalog status；在无 components、旧 current marker、合法 active version 三种目录状态下重启 | 空 catalog 显示等待 release assets；合法 active version 显示 ready；缺失/无效 marker 显示诊断，不阻塞 Desktop 设置页启动 |
| WQ-U10-02 | Setup/Filesystem | `WINDOWS_BLOCKED` | Windows 目录 ACL、只读目录、Known Downloads、跨卷路径和 WebView2 原生交互尚未具备受控环境 | 首次启动分别选择 portable directory 和 system Downloads/XArchive；测试中文/空格/只读/不可写/第二盘符；重启并检查 config、archive.sqlite3、cache/staging、download 和 executor 状态 | 失败选择不清除旧配置；成功选择持久化；数据库和 Job 查询可用；executor 使用新 archive/staging root；错误可诊断 |
| WQ-U10-03 | Component Activation | `WINDOWS_BLOCKED` | 真实 Windows component assets、EXE probe、ACL、file lock/reparse 和 rollback fixture 尚未由 U11 提供 | 使用固定 catalog + U11 assets；从 Settings/Bootstrap 手工触发验证/激活；制造 hash mismatch、缺失 license/probe、占用旧版本句柄、junction/reparse、activation 中断；执行 rollback | 只激活 hash/size/layout/license/probe 全部通过的版本；失败保留旧 active；`.part` 不可见为 active；rollback 可恢复且无残留不可信文件 |
| WQ-U10-04 | GUI/Regression | `WINDOWS_VERIFICATION_PENDING` | Core Setup Wizard 的 WebView2、键盘焦点、DPI、错误提示和首次启动 smoke 属于 Windows GUI 行为 | 执行首次启动、设置页导航、Bootstrap 状态刷新、portable/system Downloads 按钮、失败重试和重启恢复；记录截图/日志 | 设置页可启动；状态和错误文案可见；按钮禁用/重试正确；无白屏、死锁或静默 queued Job |

本轮 `WQ-U10-02`、`WQ-U10-03` 因 Windows filesystem/真实 asset 前置不可用而跳过自动执行，手工步骤已保留；本轮没有 `WINDOWS_VERIFICATION_BLOCKING`。不得把 Linux Bootstrap/UI wiring PASS 外推为 Windows Core runtime PASS。

### 2026-09-20 U11 Release assets/pipeline handoff

U11 Linux scope 已完成：新增 `desktop/scripts/release-assets.mjs` 定义版本化 Windows x64 资产命名（`XArchive-<tag>-windows-x64.exe/.7z`）与 manifest 契约（tag、platform、catalog_version、assets、licenses、SHA-256、size_bytes、license 相对路径），禁止动态 `latest`、无 hash、无 size、无 license；新增 `desktop/test/release-assets.test.mjs` 覆盖接受与拒绝用例。Linux 不生成、签名、上传真实资产；真实构建、哈希、签名、许可证扫描、发布上传和 Core/Offline Bundle parity 等待 Windows/CI。

| ID | 类别 | 当前状态 | Windows 原因/阻塞 | 手工验证步骤 | 预期结果 |
|---|---|---|---|---|---|
| WQ-U11-01 | Build/Release | `WINDOWS_VERIFICATION_PENDING` | 真实 Windows `.exe`/`.7z` 构建、CI runner、版本号与 tag 一致性需发布环境确认 | 在 Windows runner 上执行现有 Tauri/windows-release workflow；用 `release-assets.mjs` 校验资产名与 tag；记录 tag、workflow run、输出资产路径 | 资产名符合 `XArchive-<tag>-windows-x64.exe/.7z`；tag 与源码、notes、workflow 输入一致；无动态 `latest` 或未版本化资产 |
| WQ-U11-02 | Packaging/Hash | `WINDOWS_BLOCKED` | 真实资产 SHA-256/size、`SHA256SUMS`、解压 smoke、7z 内容边界需真实产物，当前无 Windows artifact | 用 `Get-FileHash -Algorithm SHA256` 记录 `.exe`/`.7z`；校验 size_bytes；解压 `.7z` 并确认只含 `xarchive-desktop.exe`；生成并校验 `SHA256SUMS`；用 manifest 校验函数比对 | hash/size 与 manifest 一致；7z 只含预期可执行文件；hash mismatch 的资产被拒绝；记录可复现 |
| WQ-U11-03 | Packaging/License | `WINDOWS_BLOCKED` | 真实捆绑内容、license 文本、源码获取方式、许可证扫描需最终产物，当前无 Windows artifact | 按 `THIRD_PARTY_NOTICES.md` 检查实际捆绑文件、版本、license 文本和源码获取方式；运行许可证扫描；更新 release checklist | 捆绑内容与 notices 一致；license 完整；扫描与法律审查结论可追溯；未完成项不得发布 |
| WQ-U11-04 | Packaging/Parity | `WINDOWS_BLOCKED` | Core/Offline Bundle 组装、manifest/catalog/实际目录一致性、embedded catalog 对齐需真实产物 | 分别组装 Core/Offline Bundle；比较 `package-manifest.json`、components 目录、实际文件 hash/size/layout、embedded catalog；确认 Core/Full 边界 | manifest/catalog/实际目录一致；Core/Full 边界正确；无 cache/credential/未知文件；embedded catalog 与发布资产版本一致 |

本轮 `WQ-U11-02`、`WQ-U11-03`、`WQ-U11-04` 因缺少真实 Windows 产物而跳过自动执行，手工步骤已保留；本轮没有 `WINDOWS_VERIFICATION_BLOCKING`。不得把 Linux 命名/manifest 契约 PASS 外推为 Windows 构建/发布 PASS。

### 2026-09-20 v0.2.0-pre.3 GitHub Actions release result

本轮对 `v0.2.0-pre.3` 的 GitHub Release 和 Windows workflow 进行了集中检查。Release tag 指向 `baf0b241237afbd9fb7435f96403af2de5598d91`；当前分支后续文档提交 `6e97ea2f4e645c61314aa782e4c871091a75b894` 不属于该 tag。GitHub Release 正文虽已更新，但不能改变构建 source revision。

| ID | 类别 | 当前状态 | 证据与失败原因 | 后续动作 |
|---|---|---|---|---|
| WQ-U11-01 | Build/Release | `WINDOWS_FAIL` | Windows Release Build run `35492155317` 在 `Run Rust tests` 失败；`xarchive-sidecar-supervisor` 的两个 `spawn_ready_v2_*` 测试均出现 `sidecar v2 hello handshake timed out`；Tauri/worker/build/package/upload 全部未执行 | 修复或确认 Windows Sidecar v2 测试 fixture、进程启动和 stdout handshake 行为；用包含修复的最终 tag 重跑完整 workflow |
| WQ-U11-02 | Packaging/Hash | `NOT RUN` | `v0.2.0-pre.3` Release 资产列表为空，workflow 在 Rust 测试阶段停止，没有 `.exe` 或 `.7z` 可供 hash/size/解压验证 | 仅在 Windows build 成功后执行 SHA-256、size、7z 解压和 manifest 对照 |
| WQ-U11-03 | Packaging/License | `NOT RUN` | 没有生成 Full/Core 或 repository-dependencies 产物，无法检查真实捆绑内容、许可证文本和来源 | 构建成功后执行 notices/license/source scan，并记录真实资产路径 |
| WQ-U11-04 | Packaging/Parity | `NOT RUN` | 没有实际 Windows bundle、manifest/catalog 和 Release assets，无法比较目录、hash、size、catalog parity | 构建成功后组装并解压 Core/Full/Offline Bundle，比较 manifest、embedded catalog 与实际目录 |

本轮重复的手动 workflow run `35492159783` 已取消，不能作为验证证据。此前成功的 `v0.2.0-pre.2` run `35485163451` 不适用于 `v0.2.0-pre.3`，不得将其四类资产或 Windows PASS 结果外推到当前 tag。

当前 `v0.2.0-pre.3` 的准确发布结论是：**pre-release 对象已创建，但构建失败且没有上传任何发布资产；不可视为完整可下载的 Windows 发布版本。**

### 2026-09-20 v0.2.0-pre.4 GitHub Actions release result

为避免复用 `v0.2.0-pre.3` 的旧 tag/source mismatch，本轮以当前分支提交 `38e9a78a56260f7064b9ebf6a5230b0a9260002e` 创建 `v0.2.0-pre.4`，并执行 Windows runner 构建。

| ID | 类别 | 当前状态 | 证据与结果 | 后续边界 |
|---|---|---|---|---|
| WQ-U11-01 | Build/Release | `WINDOWS_PASS`（pre.4 scope） | run `35497313604` 成功；Rust tests、Tauri、worker、gallery-dl/aria2 下载、Core/Full assembly 和四次 Release upload 全部通过 | 保留 pre.3 的历史 `WINDOWS_FAIL`；后续 release 必须继续检查 tag/source parity |
| WQ-U11-02 | Packaging/Hash | `WINDOWS_VERIFICATION_PENDING` | pre.4 已上传 `.exe` 和三类 `.7z`，Release asset list 完整；本轮尚未把真实 SHA-256/size manifest、SHA256SUMS 和解压证据写入验证报告 | 下载 pre.4 资产，执行 `Get-FileHash`、size、7z listing 和 manifest 对照 |
| WQ-U11-03 | Packaging/License | `WINDOWS_VERIFICATION_PENDING` | Full/repository-dependencies 资产已上传，但真实 bundle license/source scan 尚未形成完整证据 | 检查 `LICENSE`、`THIRD_PARTY_NOTICES.md`、外部来源、license files 和扫描结果 |
| WQ-U11-04 | Packaging/Parity | `WINDOWS_VERIFICATION_PENDING` | pre.4 source/tag parity 和四类资产上传通过；Core/Full/Offline Bundle 与 embedded catalog 的完整 parity 尚未单独验收 | 解压并比较 manifest、catalog、组件 hash/size/layout、运行时目录边界和签名 |

pre.4 Release asset 清单：

| 资产 | 大小（bytes） | 状态 |
|---|---:|---|
| `XArchive-v0.2.0-pre.4-windows-x64.exe` | 18,250,240 | `uploaded` |
| `XArchive-v0.2.0-pre.4-windows-x64.7z` | 4,266,292 | `uploaded` |
| `XArchive-v0.2.0-pre.4-windows-x64-repository-dependencies.7z` | 5,918,138 | `uploaded` |
| `XArchive-v0.2.0-pre.4-windows-x64-full.7z` | 34,257,996 | `uploaded` |

本轮没有 `WINDOWS_VERIFICATION_BLOCKING`。`v0.2.0-pre.4` 是当前 source 的正确 Windows runner 构建对象，但 U7 真实 runtime、U9/U10 filesystem/activation、U12 browser/Registry/Native Host reconnect、U13 final Offline Bundle parity/signature/license scan 仍保持 `WINDOWS_VERIFICATION_PENDING` 或 `WINDOWS_BLOCKED`，不得因 workflow 成功而提前改为 `WINDOWS_PASS`。
### 2026-09-20 current HEAD Windows validation result

本轮基于 Linux source HEAD 4812f29847a6c2ae77eed608e91ff4c2d4bc4769，branch feature/u7-desktop-production-integration，working tree 在文档写回前干净。Linux → E:ShiraishiVSCode WorkspaceTw2Tg 单向同步完成：Robocopy exit 3、Files copied=164、MISMATCH=0、FAILED=0；E: 本地依赖、缓存、target、gallery-dl、aria2、logs 和 validation artifacts 保留。完整记录见 docs/development/windows-validation.md 的本节。

| 队列项目 | 本轮最新状态 | 本轮证据与边界 | Linux 后续 |
|---|---|---|---|
| WQ-P0-01 | WINDOWS_PASS | Node 44/44 + 7/7、Rust 188 tests、fmt/check/strict Clippy、Sidecar 21/21、Tauri release build 通过 | 无 baseline follow-up |
| WQ-U8-01 | WINDOWS_PASS（current-source scope） | v2-only worker hello/capability、unknown-field、legacy v1 rejection、shutdown、E2E/WDIO 通过；隔离 E: stale U8 文件后 legacy-symbol check 为空 | 保持 Linux source/E: stale-file audit |
| WQ-U7-02 至 WQ-U7-05 | WINDOWS_BLOCKED | aria2、signed URL、Windows filesystem/restart fixtures 不可用 | 准备受控 runtime fixtures |
| WQ-U9-01 | WINDOWS_VERIFICATION_PENDING | Windows build/startup 和 contract tests 通过；embedded catalog 当前为空，无真实 asset activation | 提供 U11 catalog/assets 后重验 |
| WQ-U9-02 / WQ-U9-03 | WINDOWS_BLOCKED | ACL、reparse、lock、atomic activation、真实 component probe 未执行 | 准备 Windows filesystem/executable fixtures |
| WQ-U9-04 | WINDOWS_VERIFICATION_PENDING | Core/Full manifest boundary 通过；真实 license/hash/catalog parity 未完成 | 组装真实 U11/Core/Offline assets |
| WQ-U10-01 / WQ-U10-04 | WINDOWS_VERIFICATION_PENDING | Core/Full startup、ordinary WDIO 2/2、advanced WDIO 4/4 通过；Bootstrap Settings/marker/manual setup 未验收 | 运行 Settings Bootstrap 和 marker/manual setup |
| WQ-U10-02 / WQ-U10-03 | WINDOWS_BLOCKED | Known Downloads、ACL、跨卷、组件激活和 rollback fixture 未执行 | 准备人工 Windows filesystem/assets |
| WQ-U11-01 | WINDOWS_FAIL | 已记录的 v0.2.0-pre.3 GitHub Actions run 35492155317 在 Rust v2 handshake tests 失败；本轮未重触发外部 workflow | 使用最终 tag 重跑 workflow 后再发布 |
| WQ-U11-02 / WQ-U11-03 / WQ-U11-04 | NOT RUN | pre.3 workflow 未产出 exe/7z/release manifest，无法做 hash/license/parity | 构建成功后执行 Get-FileHash、7z、license 和 parity |
| WQ-U12-01 | WINDOWS_VERIFICATION_PENDING | Native Host/Extension contract tests 通过；真实 release host、Registry manifest 和 browser ID 未验证 | 准备固定 host asset/Extension ID |
| WQ-U12-02 至 WQ-U12-04 | WINDOWS_BLOCKED | Registry/ACL、Edge/Chrome developer mode、Native Host reconnect 未执行 | 准备浏览器和当前用户环境 |
| WQ-U13-01 至 WQ-U13-04 | WINDOWS_BLOCKED | 无六组件 Offline Bundle、真实 catalog/hash/license/signature | 生成最终 Offline Bundle 后执行 parity/signature |
| WQ-P1-16 / WQ-P1-17 | WINDOWS_PASS（KEEP_VALID） | 当前 WDIO service/spec 运行通过；不外推为真实 Native Host/Registry PASS | 影响区变化时重验 |

本轮没有 WINDOWS_VERIFICATION_BLOCKING。Windows GUI helper 两次初始化失败，按 BLOCKED_AUTOMATION 记录，不判定产品失败。当前 local build/worker/WDIO PASS 不能清除 pre.3 外部 Release workflow 的历史 WINDOWS_FAIL。

### 2026-09-20 current HEAD Windows validation status summary

- WINDOWS_PASS：local baseline、U8 current-source worker/WDIO、Node/Rust/Sidecar、Core/Full build/assembly/startup。
- WINDOWS_FAIL：v0.2.0-pre.3 recorded GitHub Actions Release workflow；本轮未重跑外部 workflow。
- WINDOWS_BLOCKED：U7 runtime、U9 filesystem/probe、U10 filesystem/activation、U12 browser/Registry、U13 Offline Bundle/signature。
- NOT RUN：pre.3 release asset hash/license/parity because no assets were produced.
- WINDOWS_VERIFICATION_PENDING：U9 catalog/asset activation, U9 packaging parity, U10 Bootstrap/manual setup, U11 release rerun, U12 packaging contract and U13 final parity.

本轮没有业务代码修改；E: stale source 文件仅移动到 validation-artifactsstale-sync-20260920 以避免污染 current-source 验证，未反向同步到 Linux。
### 2026-09-20 current dirty UI/Extension/Native Host/release validation update

本轮 Linux source 为 bd3e58ddf064ab015a3c04036086a01a871062e6，branch 为 feature/u7-desktop-production-integration，working tree 含 UI、Extension、Native Host packaging、release workflow 和文档改动。Linux 到 E: 单向同步完成，Robocopy exit 3，171 copied、0 mismatch、0 failed；E: 本地依赖、缓存、target、logs 和 validation-artifacts 保留。

| Queue item | Current status | Evidence / boundary | Linux follow-up |
|---|---|---|---|
| WQ-P0-01 | WINDOWS_PASS (local build/contract scope) | npm check/test/build、Tauri release build、Native Host release build、worker v2 probe 通过；Desktop 46/46、Extension 10/10 | no baseline code change from this evidence |
| WQ-P1-16 / WQ-P1-17 | WINDOWS_FAIL | ordinary WDIO 0/1 and advanced WDIO 0/2; WebView2/tauri-driver session starts but dashboard h1 never renders | diagnose current E2E/native render path; do not mark KEEP_VALID |
| WQ-U7-01 / WQ-ARCH-01 | WINDOWS_PASS (handshake only) | current one-dir worker --help, v2 hello/capabilities, unknown-field INVALID_COMMAND and clean exit passed | real extraction/transfer remains separate |
| WQ-U7-02 to WQ-U7-05 | WINDOWS_BLOCKED | aria2, signed URL/media, Windows lock/reparse and restart/recovery fixtures unavailable | provide controlled Windows fixtures |
| WQ-U9-01 / WQ-U9-04 | WINDOWS_VERIFICATION_PENDING | Core/Full local manifests and boundaries assembled; real catalog/assets/hash/license parity absent | use final assets for activation/parity |
| WQ-U9-02 / WQ-U9-03 | WINDOWS_BLOCKED | ACL, reparse, lock, atomic activation and real component probe not executed | provide filesystem/executable fixtures |
| WQ-U10-01 / WQ-U10-04 | WINDOWS_VERIFICATION_PENDING | Full package starts and is cleaned up, but WDIO dashboard render fails and Bootstrap/manual setup was not accepted | diagnose render, then perform Settings/marker/setup GUI checks |
| WQ-U10-02 / WQ-U10-03 | WINDOWS_BLOCKED | Known Downloads, ACL, cross-volume and activation/rollback fixtures unavailable | prepare manual Windows filesystem/assets |
| WQ-U11-01 | NOT RUN for current dirty workflow | historical pre4 run 35497313604 passed its recorded source/tag; current workflow changes were not sent to an external runner | rerun workflow with current final source/tag |
| WQ-U11-02 to WQ-U11-04 | NOT RUN | no final release assets were downloaded for hash, license and parity evidence | obtain real assets and run release acceptance |
| WQ-U12-01 | WINDOWS_PASS (local package boundary only) | Full assembly includes Native Host exe, com.tw2tg.xarchive.json, installation manifest and allowed_origins using a synthetic local ID | repeat with real release Extension ID |
| WQ-U12-02 to WQ-U12-04 | WINDOWS_BLOCKED | no Registry/ACL, Edge/Chrome developer-mode or Native Host reconnect environment | perform real browser/system integration |
| WQ-U13-01 to WQ-U13-04 | NOT RUN / WINDOWS_BLOCKED | final Offline Bundle, signature, catalog/hash/license and parity evidence unavailable | build final bundle and execute parity/signature scan |

本轮没有 WINDOWS_VERIFICATION_BLOCKING。WDIO failure logs: E:ShiraishiVSCode WorkspaceTw2Tgvalidation-artifactscurrent-dirty-20260920wdio-smoke.log and wdio-advanced.log。Local package logs and worker probe logs are in the same directory. 本轮没有修改 Linux 业务代码；只追加验证记录，Windows 工作副本产生的 build/test artifacts 未反向同步。

### 2026-09-20 Linux reconciliation after current-dirty Windows result

Linux 已重新读取上一节 Windows 结果，并按 [`../development/cross-platform-validation.md`](../development/cross-platform-validation.md) 重新评估当前 Plan：

- `WQ-P1-16/WQ-P1-17` 保持 `WINDOWS_FAIL`。当前 UI diff 命中其影响区，旧的 KEEP_VALID 不再适用；Dashboard `h1` 未渲染是 Windows native session/render failure，但当前没有足够证据归因到业务代码。
- `WQ-P0-01` 仅保持 local Node/Tauri/Native Host build/contract scope 的 `WINDOWS_PASS`；不覆盖 native WDIO GUI、Registry、Named Pipe 或真实浏览器连接。
- `WQ-U12-01` 仅为 synthetic Extension ID 的 local package boundary `WINDOWS_PASS`；真实 release Extension ID、Registry manifest、Edge/Chrome 和 reconnect 仍为 `WINDOWS_BLOCKED` 或 `WINDOWS_VERIFICATION_PENDING`。
- `WQ-U11-01` 当前 dirty release workflow 未执行；历史 `v0.2.0-pre.4` PASS 只适用于其记录的 tag/source，不覆盖当前 dirty workflow 修改。
- Linux 未发现由本轮 Windows 结果确定的业务代码 failure；不修改 UI、Tauri capability 或 Native Host 逻辑来猜测修复 Windows native render failure。

本次 Linux follow-up：运行受影响的 Desktop/Extension/Native Host/package tests、Vite build、Rust fmt/check 和脚本 syntax；将结果写回本节。未完成 Windows 项目继续使用 `WINDOWS_VERIFICATION_PENDING`、`WINDOWS_FAIL`、`WINDOWS_BLOCKED` 或 `NOT RUN`，不得提前标记 PASS。
### 2026-09-20 incremental Windows revalidation after Linux reconciliation

当前 Linux source HEAD 仍为 bd3e58ddf064ab015a3c04036086a01a871062e6，当前业务 working-tree 影响区与上一轮相同。本轮 Linux → E: 单向同步 Robocopy exit 3，171 copied、0 mismatch、0 failed；重新执行 Node check/test/build、Rust fmt/check 和 Native Host 8/8 测试，全部通过。

| Queue item | Current status | Update |
|---|---|---|
| WQ-P0-01 | WINDOWS_PASS | current local Node/Rust/Native Host build and contract scope revalidated |
| WQ-P1-16 / WQ-P1-17 | WINDOWS_FAIL | prior current-dirty ordinary/advanced native WDIO failure remains valid; not rerun because no affected code or prerequisite change |
| WQ-U7-01 / WQ-ARCH-01 | WINDOWS_PASS (handshake only) | carried forward; no worker source or artifact-impact change |
| WQ-U7-02 to WQ-U7-05 | WINDOWS_BLOCKED | required Windows runtime fixtures remain unavailable |
| WQ-U9/U10 filesystem, activation and manual GUI items | WINDOWS_BLOCKED or WINDOWS_VERIFICATION_PENDING | no assets, ACL/reparse/marker or usable manual GUI prerequisite added |
| WQ-U11-01 | NOT RUN for current dirty workflow | no external runner invocation; historical pre.4 result remains tag-scoped |
| WQ-U11-02 to WQ-U11-04 | NOT RUN | no final release asset set |
| WQ-U12-01 | WINDOWS_PASS (local package boundary only) | synthetic Extension ID boundary only; no real release identity |
| WQ-U12-02 to WQ-U12-04 | WINDOWS_BLOCKED | Registry/browser/Named Pipe/reconnect environment absent |
| WQ-U13-01 to WQ-U13-04 | NOT RUN / WINDOWS_BLOCKED | final Offline Bundle and parity/signature/license evidence absent |

本轮没有业务代码修改；仅同步验证范围并追加 Linux 验证记录。WDIO 现有失败日志仍位于 E:ShiraishiVSCode WorkspaceTw2Tgvalidation-artifactscurrent-dirty-20260920。
### 2026-09-20 Windows validation queue update after native-render and Native Host audit

本轮使用 Linux branch feature/u7-desktop-production-integration、HEAD bd3e58ddf064ab015a3c04036086a01a871062e6 及既有 working-tree changes 单向同步到 E:ShiraishiVSCode WorkspaceTw2Tg；关键源文件哈希一致。详细证据、命令、日志路径和错误分析见 docs/development/windows-validation.md 的同日期章节。

| ID | 本轮状态 | 证据/原因 | 后续 |
| --- | --- | --- | --- |
| WQ-P1-16 | FAIL | 普通 release binary 可建立 WebView2/tauri-driver session，但 20 seconds 内 dashboard h1 不存在；ordinary 日志位于 validation-artifacts/wdio-diagnostic-20260920/ordinary-binary/wdio-console.log。 | 保持失败；先补齐前端 console、Tauri/Rust、WebView2/asset-load 诊断，不修改 assertion/capability。 |
| WQ-P1-17 | FAIL | wdio-e2e binary 的 ordinary spec 与 advanced 2-spec run 均在 session 成功后因 dashboard/h1 缺失失败；ordinary/e2e binary 对比未显示仅由 E2E 注入导致。 | 继续 Windows root-cause 诊断；未确认项目代码问题前不回 Linux 修改业务代码。 |
| WQ-U12-01 | PASS（仅 local boundary） | synthetic Full package、Native Host manifest、installation manifest 文件边界生成成功；cargo build、8 Native Host tests、4 package tests 通过。 | 不升级为真实浏览器/Registry/transport acceptance。 |
| WQ-U12-02 | BLOCKED | XARCHIVE_EXTENSION_ID 缺失；HKCU/HKLM Chrome/Edge NativeMessagingHosts key 均不存在；当前没有可执行 Registry lifecycle 流程。 | 获取真实发布 ID 后实现/验证 install、repair、unregister。 |
| WQ-U12-03 | BLOCKED | 真实 Extension ID 与已安装 package 不存在；Chrome executable 未发现；未进行 synthetic ID 浏览器验收。 | 验证真实 Edge/Chrome developer-mode loading 和 manifest path。 |
| WQ-U12-04 | BLOCKED | Windows Native Host 使用 OpenOptions endpoint；Desktop transport server 仅 cfg(unix)，没有 Windows Named Pipe server。 | 实现/验证 Named Pipe、transport、reconnect、pending request、真实 query_status/archive_request。 |
| WDIO cleanup | PASS with caveat | 三条 WDIO 路径最终无残留 xarchive-desktop/driver，4444/4445 已释放；但每次 upstream teardown 后有 2 个 driver 需 tree-kill。 | 保留 cleanup 证据并继续确认正常/异常路径 ownership。 |
| Frontend/WebView2 logs | BLOCKED | Rust/Tauri log 仅 application runtime initialized；前端 console 未落盘，不能确认 asset load 或错误类别。 | 先修复诊断捕获路径；不要把日志缺失当作无错误。 |
### 2026-09-20 Follow-up diagnostic and Native Host integration gate

- WQ-P1-16/WQ-P1-17 仍为 FAIL。新的 process evidence 确认 WDIO 启动了目标 release binary，并使用临时 WebView2 profile、automation flags 和 remote-debugging-port=0；未取得可连接 CDP endpoint，asset/document/console 根因仍未确认。
- WQ-U12-02~U12-04 仍为 BLOCKED。真实集成的 gate 为真实 Extension ID、Windows Named Pipe server/client、HKCU Registry lifecycle、portable path repair、真实 Edge/Chrome extension load 及 query_status/archive_request/reconnect 验证。详细方法已写入 windows-validation.md 同日期章节。

### 2026-09-21 validation after Linux startup-observability changes

Linux source HEAD `dd777219f85dbf9cce1076fd5deaa81be584c60e`, branch `feature/u7-desktop-production-integration`, dirty working tree. Linux to E: Robocopy synchronization completed with 195 copied, 0 mismatch, 0 failed; Windows-local dependencies, target/dist, logs and validation artifacts were preserved.

| ID | Current status | Evidence / follow-up |
| --- | --- | --- |
| WQ-P0-01 | WINDOWS_PASS (local build/contract scope) | Vite build, Rust fmt/check/test, ordinary and WDIO-E2E Tauri release builds passed; current binaries were saved with SHA-256 evidence in windows-validation.md. |
| WQ-P1-16 | WINDOWS_BLOCKED for current revalidation | WDIO worker failed before application session/page evidence with Node `uv_os_get_passwd returned ENOMEM`; historical pre-observability WINDOWS_FAIL remains baseline only. Resolve machine resource/Node and cleanup blocker, then rerun ordinary binary with startup evidence. |
| WQ-P1-17 | WINDOWS_BLOCKED for current revalidation | Advanced WDIO was not started because the ordinary page-level worker prerequisite is blocked. Do not change assertions or capabilities. |
| WQ-U12-01 | WINDOWS_PASS (local package boundary only) | Native Host framing, package and Extension identity contracts passed; this is not real browser/Registry/Named Pipe integration. |
| WQ-U12-02 to WQ-U12-04 | WINDOWS_BLOCKED | Real Extension ID/Registry lifecycle, Chrome/Edge developer-mode loading, Windows Named Pipe server, reconnect/pending request, real query_status and archive_request remain unavailable. |
| WDIO cleanup | FAIL / environment follow-up | Current WDIO retries required targeted cleanup after service confirmation failure on driver ports 4460/4445. The full desktop Node test also fails in the Windows `killTree` case. |
| Full final UI readiness | NOT RUN | Current release workflow gate was not run on an external runner and local WDIO could not create a page session. |

Linux follow-up remains diagnostic-only: collect frontend console, Rust/Tauri and WebView2 logs, compare ordinary and wdio-e2e binaries, confirm asset loading and startup environment, and only then consider a Linux business-code fix. No Linux business code was modified by this validation run.


### 2026-09-21 screenshot-based native render root cause and Linux reconciliation

A user-provided Windows screenshot shows the React ErrorBoundary error `extensionBusy is not defined`. Linux source inspection confirms the exact mismatch at `desktop/src/main.jsx:135-156`: `Sidebar` uses `extensionBusy` but does not destructure that prop. The parent passes the value, so this is a frontend component prop-scope bug and explains the missing Dashboard `h1` after the native session starts.

| ID | Updated interpretation | Linux follow-up |
| --- | --- | --- |
| WQ-P1-16 | `WINDOWS_VERIFICATION_PENDING` for the fixed revision | Observed binary failed because `Sidebar` did not destructure `extensionBusy`; Linux fixed the prop contract and added UI wiring coverage. Rebuild/resync and rerun ordinary WDIO against the fixed artifact. |
| WQ-P1-17 | `WINDOWS_VERIFICATION_PENDING` for the fixed revision | The same shared Sidebar path affects advanced WDIO; screenshot was not an independent advanced run. Rerun advanced after the Linux fix. |
| Native render diagnosis | PROJECT_CODE_FAILURE_CONFIRMED | Do not change h1 assertions, wait semantics, or capabilities. Keep WebView2/asset logging and cleanup diagnostics for the post-fix rerun. |

Linux reconciliation complete: `desktop/src/main.jsx` now passes `extensionBusy` through the `Sidebar` parameter contract, and `desktop/test/ui-wiring.test.mjs` asserts both parent passing and child destructuring. Linux tests/build/checks pass. The fix has not been validated on the repaired Windows artifact; WQ-P1-16/WQ-P1-17 and P0 final artifact items remain `WINDOWS_VERIFICATION_PENDING`.
### 2026-09-21 queue update after the repaired Windows artifacts

| ID | Current status | Evidence / Linux follow-up |
| --- | --- | --- |
| WQ-P0-01 | WINDOWS_PASS (local build/contract scope) | Current UI contracts 15/15, Desktop Node 68/68, Vite, Rust fmt/check/test 87/87, Native Host release build/tests and ordinary/WDIO-E2E builds passed. This does not include final native WDIO acceptance. |
| WQ-P1-16 | WINDOWS_FAIL | Ordinary native WDIO now renders Dashboard `h1` and stable regions, but fails the startup marker assertion (`initial_ipc_settled` vs `react_mount_completed`). Fix deterministic startup observability on Linux, then rerun. |
| WQ-P1-17 | WINDOWS_FAIL | Advanced WDIO has the same dashboard startup-marker failure; WDIO plugin checks pass. Do not relax assertions/capabilities. |
| Full-package UI readiness | WINDOWS_FAIL | The Full package executable starts a WebView2 session and renders the shell, but its ordinary WDIO run fails the same startup contract (`initial_ipc_started` vs `react_mount_completed`). |
| WQ-U12-01 | WINDOWS_PASS (local package boundary only) | Full package, Native Host manifest and installation manifest assembled with the repository-derived local ID; 13 package/identity tests and 8 Rust Native Host tests passed. |
| WQ-U12-02 to WQ-U12-04 | WINDOWS_BLOCKED | Real release ID, Registry lifecycle, browser loading, Windows Named Pipe server/transport, reconnect and real request/job flow remain unavailable. |
| WDIO cleanup | PASS with caveat | Target processes absent and validation ports have no listeners; service tree-kill remains required after upstream teardown in the logs. |
| WQ-U7-02 to WQ-U7-05 | WINDOWS_BLOCKED | Required Windows runtime, signed media, filesystem lock/reparse and restart fixtures are unavailable. |
| WQ-U9-01/WQ-U9-04 | NOT RUN | Real catalog/assets/hash/license parity inputs are unavailable; local package boundary was checked separately. |
| WQ-U9-02/WQ-U9-03 | WINDOWS_BLOCKED | ACL, reparse, lock, atomic activation and component probe fixtures are unavailable. |
| WQ-U10-01/WQ-U10-04 | NOT RUN | Manual Bootstrap Settings, marker and setup GUI acceptance was not executed; Full-package UI readiness is separately WINDOWS_FAIL. |
| WQ-U10-02/WQ-U10-03 | WINDOWS_BLOCKED | Known Downloads, ACL, cross-volume and activation/rollback fixtures are unavailable. |
| WQ-U11-01 to WQ-U11-04 | NOT RUN | No external release runner or final release asset set was available. |
| WQ-U13-01 to WQ-U13-04 | NOT RUN | Final Offline Bundle, signature, catalog, hash and license inputs were unavailable. |

The earlier `extensionBusy is not defined` native-render defect is confirmed fixed by this rerun: the page renders in all three tested executable paths. The remaining failure is the startup-observability state contract, not Dashboard rendering. Linux fixed the state ownership by keeping `react_mount_completed` as the final DOM readiness marker and recording initial IPC stages as diagnostic events only. No UI assertion or capability relaxation is authorized by this evidence. No `WINDOWS_VERIFICATION_BLOCKING` item was created.

### 2026-09-21 Linux reconciliation after startup-marker failure

| ID | Updated status | Linux reconciliation / Windows next step |
|---|---|---|
| WQ-P1-16 | `WINDOWS_VERIFICATION_PENDING` | Windows ordinary WDIO rendered Dashboard but failed because IPC overwrote the final marker. Linux removed those state mutations and added contract coverage; rebuild/resync and rerun ordinary WDIO. |
| WQ-P1-17 | `WINDOWS_VERIFICATION_PENDING` | Advanced WDIO rendered Dashboard and plugin checks passed, but the same marker contract failed. Rerun advanced against the fixed artifact without relaxing assertions/capabilities. |
| Full-package UI readiness | `WINDOWS_VERIFICATION_PENDING` | Full executable rendered the shell but exposed `initial_ipc_started`; Linux fixed deterministic marker ownership. Rerun Full-package readiness. |
| WQ-P0-WHITE-01/02/04 | `WINDOWS_VERIFICATION_PENDING` | Final ordinary `.exe`, Full bundle and upload gate require evidence from the rebuilt artifact; no PASS is inferred from the local Linux build. |

Linux verification for this reconciliation: Desktop Node `69/69`, Vite build, WDIO syntax, Rust fmt/check/test `87/87`, and `git diff --check` passed.

### 2026-09-21 v0.2.0-pre.8 GitHub Actions release result

| ID | Status | Evidence / next step |
|---|---|---|
| WQ-REL-PRE8-BUILD | `PASS` | Actions run `35593193897` passed checkout/source parity, Rust workspace check/tests, Windows Tauri build, Native Host build, worker build and official external dependency smoke. |
| WQ-P0-WHITE-01 / WQ-P1-16 | `WINDOWS_BLOCKED` | Final executable UI readiness gate failed before page/session creation with `WebDriverError: session not created: DevToolsActivePort file doesn't exist` at `http://127.0.0.1:4444/session`. This is a Windows WebView2/Edge driver/tauri-driver session blocker; no Dashboard assertion result was produced by this run. |
| WQ-P1-17 | `WINDOWS_BLOCKED` | Advanced run was not reached because the release workflow gate uses the ordinary final executable gate and that prerequisite failed. |
| WQ-P0-WHITE-02 | `WINDOWS_BLOCKED` | Full bundle assembly did not run because the final executable readiness gate failed first. |
| WQ-P0-WHITE-04 | `WINDOWS_FAIL` | Release upload gate correctly stopped all archive, manifest and Release upload steps after readiness failure. |
| Release assets | `NOT RUN` | `v0.2.0-pre.8` Release exists as prerelease but has zero assets; no failed artifact was uploaded. |
| WDIO cleanup | `PASS with caveat` | Service tree-killed surviving driver PIDs after upstream teardown; the run log did not provide a successful application session. |

The Linux startup-marker fix is therefore not yet Windows-validated. Do not weaken the readiness assertion or capabilities; resolve the Windows native session prerequisite and rerun the same tag/source. No `WINDOWS_VERIFICATION_BLOCKING` item was created.

### 2026-09-21 Linux diagnostics follow-up

Linux added `desktop/scripts/windows-ui-readiness-preflight.ps1` and workflow diagnostics collection. The next Windows run must record WebView2/EdgeDriver/tauri-driver versions, direct executable startup, `msedgewebview2`/driver process state, ports `1420/4444/4445/9223`, application logs and WDIO logs. A successful preflight is not a UI PASS; it only separates application-start, driver-start, session-creation and DOM-readiness failures.

If `windows-latest` continues to fail at `DevToolsActivePort` while a controlled interactive Windows runner can create sessions, move UI validation to a self-hosted interactive runner and publish only the exact hash-verified artifacts produced by the build job. Keep all related items `WINDOWS_VERIFICATION_PENDING` until that decision is supported by new evidence.

Linux implementation status: `windows-ui-readiness-preflight.ps1` now records direct executable startup, tool versions, WebView2 registry facts, process snapshots and port snapshots. The workflow copies application/WDIO logs and always uploads the diagnostics directory while still stopping archive and Release upload on readiness failure. Linux contract tests/build/Rust checks pass; Windows execution remains `WINDOWS_VERIFICATION_PENDING`.

R3/R7 Linux closeout: WDIO now defaults to `autoInstallTauriDriver=false` and `autoDownloadEdgeDriver=false`; the release workflow pins `tauri-driver 2.1.0-alpha.0` and `msedgedriver 152.0.4191.66`, validates the EdgeDriver version, adds the pinned tauri-driver directory to PATH, and uploads toolchain metadata/SHA-256 with readiness diagnostics. Linux Node `71/71`, Vite check/build, JS syntax, Rust workspace check/test, strict Clippy and diff check passed. Windows verification remains pending because PATH discovery, WebView2 compatibility and session creation cannot be established on Linux.

### 2026-09-21 ccaa649 reconciliation update

Windows now reports new facts for `ccaa649`: sync/build/contract/preflight scope PASS; WQ-P1-16/WQ-P1-17 remain `WINDOWS_BLOCKED` because installed `@wdio/tauri-service 1.4.0` rejects the observed `Microsoft Edge WebDriver ...` banner; remaining items remain `WINDOWS_BLOCKED` or `NOT RUN` for missing fixtures. The latest Windows round did not contradict the earlier `extensionBusy` or startup-marker Linux fixes, but neither fix has native session/DOM evidence.

Linux follow-up completed in this turn: preflight now emits `msedgedriver_banner_accepted`/`msedgedriver_banner_reason` for both `MSEdgeDriver` and `Microsoft Edge WebDriver` banners, with a contract test pinning that behavior. Linux verification: Desktop Node `72/72`, Vite check/build, JS syntax, Rust fmt/check/workspace tests, strict Clippy and `git diff --check` passed. No business code, UI assertion, production capability or dependency was changed to manufacture a session PASS. WQ-P1-16/WQ-P1-17 stay `WINDOWS_BLOCKED` until real session evidence exists; no `WINDOWS_VERIFICATION_BLOCKING` was created.

### 2026-09-21 synchronized revalidation closeout

Windows confirmed the synchronized source passes Node `72/72`, Extension `21/21`, Rust fmt/check/workspace tests/strict Clippy, Sidecar pytest `21/21`, ordinary/WDIO-E2E builds, Native Host release build, Full package assembly and direct executable preflight. WQ-P1-16/WQ-P1-17 remain `WINDOWS_BLOCKED` because installed `@wdio/tauri-service 1.4.0` still cannot parse the observed `Microsoft Edge WebDriver ...` banner.

Linux added `desktop/scripts/edge-driver-banner.mjs` and `desktop/test/edge-driver-banner.test.mjs` to make that service limitation a Linux-regression contract: the legacy banner is service-accepted, while the current Microsoft banner is preflight-accepted but service-rejected. This is diagnostic-only test infrastructure; it changes no business code, Dashboard assertion, production capability, dependency version or download policy. Linux verification: Desktop Node `76/76`, Vite check/build, JS syntax, Rust fmt/check/workspace tests, strict Clippy and `git diff --check` passed. WQ-P1-16/WQ-P1-17 stay `WINDOWS_BLOCKED` until a real Windows session reaches DOM assertions.

#### BLOCKED Windows manual verification steps

These steps are the handoff for the currently blocked Windows-only work. Execute them on the Windows validation workspace before any release tag is created:

1. **Toolchain / Build:** sync the final Linux working tree; record branch, commit and dirty state; run `npm ci`, `cargo --version`, `node --version`, `npm run build:tauri --workspace desktop`; preserve the exact executable SHA-256.
2. **Pinned drivers:** install `tauri-driver 2.1.0-alpha.0` with `cargo install tauri-driver --version 2.1.0-alpha.0 --locked`; place the matching `msedgedriver 152.0.4191.66` on PATH; verify `where.exe` and `--version`; record both SHA-256 values. If unavailable or mismatched, mark `BLOCKED_ENV`.
3. **Preflight:** run `desktop/scripts/windows-ui-readiness-preflight.ps1` against the final executable; collect `environment.json`, process/port snapshots, direct-startup result, WebView2 registry facts and crash/profile evidence. Classify failure as `APP_START_FAILED`, `WEBVIEW2_RUNTIME_MISSING`, `EDGE_DRIVER_START_FAILED`, `TAURI_DRIVER_START_FAILED`, `SESSION_CREATION_FAILED` or `DOM_READINESS_FAILED`.
4. **Minimal session probe:** start the pinned `tauri-driver` and verify `/status`; start the pinned `msedgedriver`/external provider as configured; issue one `POST /session`; preserve stdout/stderr and the exact request/response. A `DevToolsActivePort` failure is `SESSION_CREATION_FAILED`, not a React/Dashboard failure.
5. **Ordinary UI gate:** set `WDIO_APP_BINARY`, `WDIO_LOG_DIR`, `WDIO_AUTO_INSTALL_TAURI_DRIVER=0`, `WDIO_AUTO_DOWNLOAD_EDGE_DRIVER=0`, and `EDGEDRIVER_VERSION`; run `npm run test:e2e:windows --workspace desktop`; require Dashboard DOM, `react_mount_completed`, fallback absence and automatic cleanup.
6. **Advanced validation:** only after ordinary session creation succeeds, run `npm run build:tauri:wdio --workspace desktop` and `npm run test:e2e:windows:advanced --workspace desktop`; verify plugin execute, mock/restore, logs and cleanup. If ordinary gate fails, mark advanced `BLOCKED` with the prerequisite error.
7. **Hosted-runner suitability:** repeat the minimal session and ordinary gate at least three times on `windows-latest`; repeat on a controlled interactive Windows machine if available. Compare session success, app startup, Dashboard readiness, stderr and cleanup. Do not label hosted/self-hosted suitability from a single run.
8. **Publish decision:** only if the same hash-verified artifact passes build, ordinary UI gate, advanced gate and Full bundle validation may a new `v0.2.0-pre.9` tag/release be created. Never reuse `v0.2.0-pre.8`.
### 2026-09-21 queue update for current `ccaa649`

| ID | Current status | Evidence / Linux follow-up |
| --- | --- | --- |
| WQ-P0-01 | WINDOWS_PASS (local build/contract scope) | Node 92/92, Rust workspace with explicit Python, Sidecar pytest 21/21, Tauri builds, worker probe and local package boundary passed. Native UI readiness is separate and blocked. |
| WQ-P1-16 | WINDOWS_BLOCKED | Ordinary WDIO cannot reach a session because installed `@wdio/tauri-service` parses neither the pinned 152 nor matching 153 driver banner; actual output is `Microsoft Edge WebDriver ...`, parser expects `MSEdgeDriver ...`. |
| WQ-P1-17 | WINDOWS_BLOCKED | Advanced WDIO shares the same session/driver prerequisite and was not run after the ordinary precondition failed. |
| WQ-P1-18 | NOT RUN | Portable first-run setup, move, permission fallback and manual filesystem acceptance were not executed. |
| WQ-P1-19 | NOT RUN | Portable log-level and rotation acceptance was not executed. |
| WQ-U7-01 / WQ-ARCH-01 | WINDOWS_PASS (packaged handshake scope only) | Current worker `--help`, v2 hello/capabilities, v1 rejection, unknown-field rejection and clean shutdown passed. Real extraction/transfer remains separate. |
| WQ-U7-02 to WQ-U7-05 | WINDOWS_BLOCKED | aria2, signed media, filesystem lock/reparse and restart/recovery fixtures are unavailable. |
| WQ-U9-01 / WQ-U9-04 | NOT RUN | Real catalog/assets/hash/license activation and parity inputs are unavailable; only local package boundary was checked. |
| WQ-U9-02 / WQ-U9-03 | WINDOWS_BLOCKED | ACL, reparse, lock, atomic activation and component probe fixtures are unavailable. |
| WQ-U10-01 / WQ-U10-04 | NOT RUN | Manual Bootstrap/marker/setup GUI acceptance was not executed; direct executable preflight is not a substitute. |
| WQ-U10-02 / WQ-U10-03 | WINDOWS_BLOCKED | Known Downloads, ACL, cross-volume and activation/rollback fixtures are unavailable. |
| WQ-U11-01 to WQ-U11-04 | NOT RUN | No external release runner or final asset set was executed for current `ccaa649`. Historical pre8 tag run remains separately recorded as session-gate failure with no assets. |
| WQ-U12-01 | WINDOWS_PASS (local package boundary only) | Native Host release build, framing/package/identity contracts and repository-derived-ID Full package passed; no real browser/system integration. |
| WQ-U12-02 to WQ-U12-04 | WINDOWS_BLOCKED | Real Extension ID/Registry lifecycle, Edge/Chrome load, Windows Named Pipe transport, reconnect and real `query_status`/`archive_request` Job flow remain unavailable. |
| WQ-U13-01 to WQ-U13-04 | NOT RUN | Final Offline Bundle, signature, catalog, hash and license inputs were unavailable. |

No new `WINDOWS_VERIFICATION_BLOCKING` item was created. The required next Linux task is WDIO service/driver compatibility diagnosis; it must not change UI assertions or capabilities to manufacture a pass.

### 2026-09-21 specified root driver retry

| ID | Current status | Evidence / Linux follow-up |
| --- | --- | --- |
| WQ-P1-16 | WINDOWS_BLOCKED | Retried ordinary WDIO with the requested root `msedgedriver.exe` (`152.0.4191.66`, SHA-256 `9E9B1F048D2CC781DEEE084E6CB6E9F2F3417A33ED45D96CF7C34BE4EB23077B`). The installed WDIO service still reports `Driver: unknown` because of its banner parser mismatch; no session or DOM assertion was reached. |
| WQ-P1-17 | WINDOWS_BLOCKED | Advanced WDIO was not run because the ordinary shared driver/session prerequisite remains blocked. |

### 2026-09-21 synchronized current-source revalidation

| ID | Current status | Evidence / Linux follow-up |
| --- | --- | --- |
| WQ-P0-01 | WINDOWS_PASS (local build/contract scope) | Current synchronized source passed Node 72/72, Extension 21/21, Rust fmt/check/tests/strict Clippy, Sidecar pytest 21/21, ordinary/WDIO Tauri builds, Native Host release build, Full package assembly and direct executable preflight. Native UI session acceptance remains separate. |
| WQ-P1-16 | WINDOWS_BLOCKED | New ordinary artifact and the requested fixed driver were used with auto install/download disabled. The driver banner is recognized by the preflight, but installed `@wdio/tauri-service 1.4.0` reports `Driver: unknown`; no WebDriver session or DOM assertion was reached. |
| WQ-P1-17 | WINDOWS_BLOCKED | Advanced WDIO was not run because ordinary WDIO's shared driver/session prerequisite failed. |
| WQ-P1-18 / WQ-P1-19 | NOT RUN | Manual portable setup, cross-volume/permission fallback and log rotation were not executed. |
| WQ-U7-02 to WQ-U7-05 | WINDOWS_BLOCKED | Required aria2, signed media, lock/reparse and restart/recovery fixtures remain unavailable. |
| WQ-U9-01 / WQ-U9-04 | NOT RUN | Real catalog/assets/hash/license activation inputs remain unavailable. |
| WQ-U9-02 / WQ-U9-03 | WINDOWS_BLOCKED | ACL, reparse, lock, atomic activation and real component fixtures remain unavailable. |
| WQ-U10-01 / WQ-U10-04 | NOT RUN | Manual Bootstrap/marker/setup GUI acceptance was not executed. |
| WQ-U10-02 / WQ-U10-03 | WINDOWS_BLOCKED | Known Downloads, ACL, cross-volume and activation/rollback fixtures remain unavailable. |
| WQ-U11-01 to WQ-U11-04 | NOT RUN | No external current-source release runner or final asset set was executed. |
| WQ-U12-02 to WQ-U12-04 | WINDOWS_BLOCKED | Real Extension ID/Registry/browser/Named Pipe/Job integration remains unavailable. |
| WQ-U13-01 to WQ-U13-04 | NOT RUN | Final Offline Bundle, signature, catalog, hash and license inputs remain unavailable. |

No Linux business code was modified. The next Linux task is WDIO service/driver compatibility diagnosis; do not weaken UI assertions or capabilities to manufacture a pass.

### 2026-09-21 incremental revalidation after the driver-banner helpe

| ID | Current status | Evidence / Linux follow-up |
| --- | --- | --- |
| WQ-P0-01 | WINDOWS_PASS (local build/contract scope retained) | New helper/test infrastructure passed Windows Desktop 76/76, check/build and full Node workspace regression; prior current-source Rust/Tauri/packaging/preflight PASS remains valid because production paths were unchanged. |
| WQ-P1-16 | Ordinary Windows WDIO | WINDOWS_BLOCKED | Native render PASS; clean dependency blocked by `@wdio/tauri-service@1.4.0` driver-output parser; E:-only diagnostic patch passed 1/1. Requires reproducible Linux dependency fix and clean-install revalidation. |
| WQ-P1-17 | Advanced Windows WDIO | WINDOWS_BLOCKED | WDIO-feature render and guest JS PASS; clean dependency blocked by the same parser; E:-only diagnostic patch passed 2/2. Requires reproducible Linux dependency fix and clean-install revalidation. |
| WQ-P1-18 / WQ-P1-19 | NOT RUN | Manual portable setup, permission/cross-volume fallback and log rotation remain unexecuted. |
| WQ-U7-02 to WQ-U7-05 | WINDOWS_BLOCKED | aria2, signed media, lock/reparse and restart/recovery fixtures remain unavailable. |
| WQ-U9-01 / WQ-U9-04 | NOT RUN | Real catalog/assets/hash/license activation inputs remain unavailable. |
| WQ-U9-02 / WQ-U9-03 | WINDOWS_BLOCKED | ACL, reparse, lock, atomic activation and component fixtures remain unavailable. |
| WQ-U10-01 / WQ-U10-04 | NOT RUN | Manual Bootstrap/marker/setup GUI acceptance remains unexecuted. |
| WQ-U10-02 / WQ-U10-03 | WINDOWS_BLOCKED | Known Downloads, ACL, cross-volume and activation/rollback fixtures remain unavailable. |
| WQ-U11-01 to WQ-U11-04 | NOT RUN | No external current-source release runner or final asset set was executed. |
| WQ-U12-02 to WQ-U12-04 | WINDOWS_BLOCKED | Real Extension ID, Registry lifecycle, browser loading, Named Pipe transport, reconnect and Job flow remain unavailable. |
| WQ-U13-01 to WQ-U13-04 | NOT RUN | Final Offline Bundle, signature, catalog, hash and license inputs remain unavailable. |

No Linux business code was modified. Do not weaken UI assertions or capabilities to turn the native WDIO blocker into a pass.

### 2026-09-21 post-banner-fix Windows revalidation queue

| ID | Current status | Evidence / Linux follow-up |
| --- | --- | --- |
| WQ-P1-16 | WINDOWS_VERIFICATION_PENDING | Windows rendered Dashboard for both ordinary and advanced binaries after a diagnostic banner correction in `node_modules`. Linux delivered a reproducible fix: `desktop/scripts/patch-wdio-tauri-service.mjs` (idempotent, widens `findMsEdgeDriver` to accept `Microsoft Edge WebDriver` in addition to `MSEdgeDriver`), wired as root `postinstall`. Re-validate with a clean `npm ci` (no manual `node_modules` edit) and confirm ordinary WDIO `1/1` plus native rendering evidence. |
| WQ-P1-17 | WINDOWS_VERIFICATION_PENDING | Same parser root cause as WQ-P1-16. After the clean-install revalidation produces ordinary session success, run `npm run test:e2e:windows:advanced --workspace desktop` and confirm advanced `2/2` plus plugin guest JS evidence. |
| WQ-P1-18 / WQ-P1-19 | NOT RUN | Manual portable setup, permission/cross-volume fallback and log rotation remain unexecuted. |
| WQ-U7-02 to WQ-U7-05 | WINDOWS_BLOCKED | aria2, signed media, lock/reparse and restart/recovery fixtures remain unavailable. |
| WQ-U9-01 / WQ-U9-04 | NOT RUN | Real catalog/assets/hash/license activation inputs remain unavailable. |
| WQ-U9-02 / WQ-U9-03 | WINDOWS_BLOCKED | ACL, reparse, lock, atomic activation and component fixtures remain unavailable. |
| WQ-U10-01 / WQ-U10-04 | NOT RUN | Manual Bootstrap/marker/setup GUI acceptance remains unexecuted. |
| WQ-U10-02 / WQ-U10-03 | WINDOWS_BLOCKED | Known Downloads, ACL, cross-volume and activation/rollback fixtures remain unavailable. |
| WQ-U11-01 to WQ-U11-04 | NOT RUN | No external current-source release runner or final asset set was executed. |
| WQ-U12-02 to WQ-U12-04 | WINDOWS_BLOCKED | Real Extension ID, Registry lifecycle, browser loading, Named Pipe transport, reconnect and Job flow remain unavailable. |
| WQ-U13-01 to WQ-U13-04 | NOT RUN | Final Offline Bundle, signature, catalog, hash and license inputs remain unavailable. |

### 2026-09-22 clean-install native WDIO revalidation

| ID | Current status | Evidence / Linux follow-up |
| --- | --- | --- |
| WQ-P0-01 | WINDOWS_PASS (local build/contract scope) | Current synchronized source passed clean npm ci, Node check/test/build, Rust fmt/check/tests/strict Clippy, Sidecar pytest 21 passed, Native Host release build, ordinary and WDIO Tauri builds, Core/Full assembly, direct startup preflight and packaged worker v2 contract probe. |
| WQ-P1-16 | WINDOWS_PASS (current local native WDIO scope) | Clean npm ci applied the reproducible service patch; fixed 152 driver was discovered and a real WebDriver session was created. Ordinary Dashboard WDIO passed 3/3, with no final process/port residue. Exact workflow-pinned tauri-driver 2.1.0-alpha.0 and release-runner parity remain unvalidated. |
| WQ-P1-17 | WINDOWS_PASS (current local advanced native WDIO scope) | Advanced WDIO passed Dashboard 3/3 and plugin 2/2 after the same clean install. The service reported surviving driver PIDs during teardown; the safety net removed them and final inspection was clean. Exact pinned toolchain and hosted/release-runner repeat remain pending. |
| WQ-P1-18 / WQ-P1-19 | NOT RUN | Manual portable first-run setup, cross-volume/permission fallback and log rotation were not executed. |
| WQ-U7-01 / WQ-ARCH-01 | WINDOWS_PASS (packaged handshake scope only) | Full worker --help, v2 hello/capabilities, v1 rejection, unknown-field rejection and shutdown passed. Real extraction/transfer remains separate. |
| WQ-U7-02 to WQ-U7-05 | WINDOWS_BLOCKED | aria2, signed media, filesystem lock/reparse and restart/recovery fixtures remain unavailable. |
| WQ-U9-01 / WQ-U9-04 | NOT RUN | Real catalog/assets/hash/license activation and parity inputs remain unavailable; only local package boundaries were checked. |
| WQ-U9-02 / WQ-U9-03 | WINDOWS_BLOCKED | ACL, reparse, lock, atomic activation and component probe fixtures remain unavailable. |
| WQ-U10-01 / WQ-U10-04 | NOT RUN | Manual Bootstrap/marker/setup GUI acceptance was not executed; direct executable preflight is not a substitute. |
| WQ-U10-02 / WQ-U10-03 | WINDOWS_BLOCKED | Known Downloads, ACL, cross-volume and activation/rollback fixtures remain unavailable. |
| WQ-U11-01 to WQ-U11-04 | NOT RUN | No external release runner or final signed asset set was executed for current ccaa649. |
| WQ-U12-01 | WINDOWS_PASS (local package boundary only) | Native Host release build, framing/package/identity contracts and repository-derived-ID Full package passed; no real browser/system integration. |
| WQ-U12-02 to WQ-U12-04 | WINDOWS_BLOCKED | Real Extension ID/Registry lifecycle, Edge/Chrome load, Named Pipe transport, reconnect and real query_status/archive_request flow remain unavailable. |
| WQ-U13-01 to WQ-U13-04 | NOT RUN | Final Offline Bundle, signature, catalog, hash and license inputs remain unavailable. |

No Linux business code was modified. Native WDIO is now PASS for this
controlled local scope, but the exact workflow-pinned tauri-driver and
release-runner parity remain follow-up validation. No new
WINDOWS_VERIFICATION_BLOCKING item was created.
### 2026-09-22 exact workflow-pinned tauri-driver retry

| ID | Current status | Evidence / Linux follow-up |
| --- | --- | --- |
| WQ-P1-16 | WINDOWS_FAIL (exact pinned-toolchain attempt) | tauri-driver 2.1.0-alpha.0 installed and started successfully, but pinned EdgeDriver 152.0.4191.66 rejected the installed Edge 154.0.4258.24 before session/DOM creation. The previous v2.0.6 native WDIO PASS remains local-scope evidence only. Align the browser/driver versions or provide a controlled Edge 152 runtime. |
| WQ-P1-17 | WINDOWS_BLOCKED | Advanced WDIO was not run because ordinary WDIO failed at the shared WebDriver session prerequisite. |
| WQ-P1-18 / WQ-P1-19 | NOT RUN | Unchanged: manual portable setup, permission/cross-volume fallback and log rotation remain unexecuted. |

No Linux business code was modified. The exact pinned tauri-driver installation
is now verified, but the workflow's pinned EdgeDriver/browser combination is
not compatible on this machine.

### 2026-09-22 Linux commit record for the banner fix

The Linux-side banner fix delivered for this round is committed as `03332a1`
(`fix: accept Microsoft Edge WebDriver banner in Tauri E2E harness`, 18 files
changed) on `feature/u7-desktop-production-integration` and pushed to origin;
the working tree is clean and the `v0.2.0-pre.8` tag was not moved. The commit
changed no content files relative to the workspace the two 2026-09-22 Windows
rounds above were executed against, so those results still describe revision
`03332a1`, and the next Windows sync must record `03332a1` as the Linux revision
baseline.

Post-commit Linux re-verification at that revision: Desktop
`npm run test --workspace desktop` `82/82`, Extension check/test `21/21`, Desktop
Vite production build and `git diff --check` all passed; no `crates/`,
`desktop/src-tauri/` or business frontend source was touched by this commit.

Queue states stay as recorded above and are not promoted:

| ID | Current status | Evidence / follow-up |
| --- | --- | --- |
| WQ-P1-16 | `WINDOWS_PASS` (clean-install local v2.0.6 scope) / `WINDOWS_FAIL` (exact pinned-toolchain attempt) | Local clean install created a real session and ordinary Dashboard passed 3/3; the pinned EdgeDriver 152.0.4191.66 then rejected the installed Edge 154.0.4258.24 before session/DOM creation. Requires browser/driver version alignment or a controlled Edge 152 runtime. |
| WQ-P1-17 | `WINDOWS_PASS` (clean-install local scope) / `WINDOWS_BLOCKED` (pinned toolchain) | Advanced Dashboard 3/3 and plugin 2/2 passed locally after the same clean install; the pinned-toolchain path is blocked by the shared ordinary session prerequisite. |
| Release readiness gate | `WINDOWS_VERIFICATION_PENDING` | Must run ordinary WDIO against the exact published artifact under the aligned pinned toolchain before any asset upload; hosted/release-runner parity is still unproven. |

Do not weaken assertions or fall back silently to the local toolchain when
reporting the pinned workflow result.

### 2026-09-22 v0.2.0-pre.10 hosted release runs

| ID | Current status | Evidence / follow-up |
| --- | --- | --- |
| Release readiness gate (hosted, pinned toolchain) | `WINDOWS_FAIL` | Run `35699308051` (attempt 2 and 3 identical): pinned tauri-driver 2.1.0-alpha.0 + msedgedriver 152.0.4191.66 matched WebView2 152.0.4191.66, session created, but the gate-attached WebView2 document stayed at the blank `data:,` initial document (`rootExists:false`) for the full 20 s wait; ordinary Dashboard spec never saw the app. Deterministic on the hosted runner. |
| WQ-P1-16 (hosted / release-runner scope) | `WINDOWS_FAIL` | Supersedes the earlier hosted `BLOCKED_ENV` (driver mismatch): the pinned-toolchain barrier is resolved; the failure is now the blank WebView2 document in the gate-launched production exe. Local v2.0.6 clean-install scope PASS evidence remains unchanged. |
| WQ-P1-17 (advanced) | `WINDOWS_BLOCKED` | Not run: advanced WDIO depends on the ordinary gate prerequisite. |
| WQ-P1-18 / WQ-P1-19 | NOT RUN | Unchanged: manual portable setup, permission/cross-volume fallback and log rotation remain unexecuted. |
| Hosted Rust test flake | transient `WINDOWS_FAIL`, passed on rerun | `xarchive-sidecar-supervisor` `spawn_ready_v2_*` handshake timeouts on attempt 1; same pattern as the `v0.2.0-pre.6` run `35518801950`. Passed on both later reruns of the same run. |
| WQ-U11 assets (exe/7z/manifest/hash) | NOT RUN | Steps 19–35 skipped after the gate failure; `v0.2.0-pre.10` release intentionally has zero assets per contract; tag must not be reused. |

Harness follow-up (Linux, no product code): capture screenshot/app log/window
targets in the diagnostics directory, wait on startup signals (URL leaving
`data:,`, `data-xarchive-startup`, root content) with a larger budget, and
separate hosted-environment from production-build behavior via a controlled
local run of the same `v0.2.0-pre.10` exe before the next release tag.

### 2026-09-22 Readiness Gate 目标发现修复 - Windows 验证队列（Phase 3-7 完成后）

本节登记 Phase 3-7 Linux 实现完成后，需要在 Windows 环境验证的项目。
Linux 验证已完成：node --check、desktop 单元测试 89/89、Vite check、workflow YAML 解析、git diff --check。

| ID | 类别 | 验证项目 | 关联修改/目标 | Windows 原因 | 前置条件 | 精确行为 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-P0-WHITE-01A | Runtime/Automation | target 发现诊断（hosted） | `desktop/e2e/support/native-startup.mjs`、`dashboard.e2e.mjs`、`windows-release.yml` | handle timeline 只有 hosted runner 能证明 session 附着的是哪个 WebView target | hosted readiness diagnostic run | session 建立后枚举 handle，记录每 handle URL/title/`#root`/marker/fallback；区分 `ONLY_BLANK_DOCUMENTS` 与 `APPLICATION_DOCUMENT_NOT_FOUND` | 诊断产物能明确回答是否存在另一个应用 handle；失败分类可判读 | P0 | no | `WINDOWS_BLOCKED` — d3fd814: session 创建失败，无 handle timeline |
| WQ-P0-WHITE-01B | Runtime/Regression | 受控本地同构工具链 readiness | 同上 + 本地 pinned `tauri-driver 2.1.0-alpha.0`/匹配 msedgedriver | 本地 interactive Windows 与 hosted 行为差异需对照 | 本地工具链与 WebView2 major 对齐 | ordinary release exe；WDIO 先发现目标再断言；首启+重复启动 | Dashboard 3/3、`react_mount_completed`、fallback 缺失、无残留 | P0 | no | `WINDOWS_FAIL` — d3fd814: EdgeDriver 152 拒绝 Edge 154；Edge 154 诊断确认 blank document 是 Tauri startup/target-attachment 问题 |
| WQ-P0-WHITE-01C | Runtime/Regression | hosted readiness gate 稳定性 | `windows-release.yml`、`windows-readiness-diagnostic.yml` | hosted runner 是发布 gate 的实际执行环境 | WQ-P0-WHITE-01A 诊断可判读 | 最多 3 次 diagnostic run（不建 Release、不上传资产） | 连续 PASS 才进入新 prerelease tag；失败按 handle timeline 分类，不做盲目重跑 | P0 | no | `WINDOWS_NOT_RUN` — d3fd814: 本地前置失败后未执行 hosted 验证 |
| WQ-P0-WHITE-01D | Runtime/Diagnostics | preflight→gate 隔离 | `windows-release.yml` 隔离步骤 | preflight 直启可能留下进程/profile 污染影响 gate | 任一 hosted/local run | gate 前检查应用/driver 进程与 1420/4444/4445/9223 端口 | 无残留才启动 gate；有残留 FAIL 并写入诊断 | P0 | no | `WINDOWS_FAIL` — d3fd814: preflight 后 msedgewebview2 状态变化，隔离检查失败；应用/driver/端口残留本身清理正常 |
| WQ-P0-WHITE-03R | Diagnostics | 失败证据完整性 | `captureReadinessFailure`、workflow finally 收集 | 现有诊断缺 screenshot/page source/app log，无法裁决根因 | 任一 gate 失败 | 失败时必须存在 `failure.json`、handle timeline、page source、screenshot（或其失败记录）、WDIO/app 日志 | 证据完整且能区分 `FAIL_PRODUCT`/`FAIL_TEST`/`BLOCKED_ENV` | P0 | no | `WINDOWS_BLOCKED` — d3fd814: session 失败导致无应用文档发现，证据未产生 |
| WQ-P0-WHITE-04A | Packaging/Release | 超时注入验证 | `windows-release.yml` env vars | 只有 Windows runner 能验证超时是否正确注入到 WDIO 进程 | WQ-P0-WHITE-01B 工具链对齐 | 检查 gate 运行时环境变量 `WDIO_STARTUP_DISCOVERY_TIMEOUT`/`WDIO_STARTUP_CONTRACT_TIMEOUT` 已设置 | 超时值在 WDIO 进程环境中可见，未设置时使用模块默认值 | P1 | no | `WINDOWS_PASS` — d3fd814: 超时注入验证通过（invocation scope） |
| WQ-P0-WHITE-04B | Diagnostics | WDIO 日志目录契约 | `windows-release.yml`、wdio-tauri-service | `WDIO_LOG_DIR` 与服务日志写入目录不一致可能导致日志缺失 | WQ-P0-WHITE-01B 工具链对齐 | 检查 gate 完成后 `READINESS_DIAGNOSTICS` 下是否有 WDIO 写入的日志文件 | 日志文件存在且非空；不再依赖 `Copy-Item -ErrorAction SilentlyContinue` 掩盖缺失 | P1 | no | `WINDOWS_FAIL` — d3fd814: WDIO 日志写到 desktop/logs，READINESS_DIAGNOSTICS 无非空 WDIO log |
| WQ-P0-WHITE-04C | Runtime/Diagnostics | session start 快照 | `native-startup.mjs` `snapshotSessionStart` | session 创建后快照只有 Windows WebView2 环境能证明时机与内容 | WQ-P0-WHITE-01B 工具链对齐 | 检查首次成功 gate 运行的 `startup/session-start.json` | 文件存在，包含窗口句柄数、当前 URL、标题和时间戳 | P1 | no | `WINDOWS_BLOCKED` — d3fd814: 无成功 WebDriver session，无 session-start snapshot |

#### 手工验证步骤（针对 BLOCKED 的 Windows 验证项目）

以下手工验证步骤适用于无法在当前 Linux 环境自动执行的 Windows 验证项目。请按顺序执行，并在完成后更新队列状态。

##### 前置要求

1. Windows 10/11 机器，已安装 WebView2 Runtime
2. Node.js 22+ 和 npm
3. Rust toolchain (stable-x86_64-pc-windows-msvc)
4. 已构建的 Tauri 可执行文件 (`xarchive-desktop.exe`)
5. msedgedriver 152.0.4191.66（或与本地 WebView2 major 版本匹配的版本）

##### WQ-P0-WHITE-01A/B/C/D 手工验证

1. **环境准备**：
   ```powershell
   # 设置 PIN 工具链路径
   $driverDir = Join-Path $env:RUNNER_TEMP "xarchive-webdriver-tools\msedgedriver\152.0.4191.66"
   $env:Path = "$driverDir;$env:Path"
   where.exe msedgedriver.exe
   msedgedriver.exe --version  # 应显示 152.0.4191.66
   ```

2. **执行 diagnostic run**：
   ```powershell
   # 设置环境变量
   $env:WDIO_APP_BINARY = "path\to\xarchive-desktop.exe"
   $env:READINESS_DIAGNOSTICS = "path\to\diagnostics"
   $env:WDIO_STARTUP_DISCOVERY_TIMEOUT = "30000"
   $env:WDIO_STARTUP_CONTRACT_TIMEOUT = "25000"
   $env:WDIO_AUTO_INSTALL_TAURI_DRIVER = "0"
   $env:WDIO_AUTO_DOWNLOAD_EDGE_DRIVER = "0"
   $env:TAURI_DRIVER_PROVIDER = "external"

   # 运行测试
   npm run test:e2e:windows --workspace desktop
   ```

3. **验证结果**：
   - 检查 diagnostics 目录下是否有 `startup/window-discovery-timeline.json`
   - 确认 handle timeline 区分了 `ONLY_BLANK_DOCUMENTS` 与 `APPLICATION_DOCUMENT_NOT_FOUND`
   - Dashboard 3/3 测试通过
   - 退出后检查无残留进程和端口

##### WQ-P0-WHITE-03R 手工验证

1. **确诊断产物完整性**：
   - gate 失败时检查 `READINESS_DIAGNOSTICS/startup/` 下是否有：
     - `failure.json`（包含错误信息、URL、readyState、startupState、rootExists）
     - `window-discovery-timeline.json`（handle 枚举历史）
     - `current-page.html` 或其错误记录
     - `dashboard-startup-failure.png` 或其错误记录
   - 检查 WDIO 日志和应用日志是否被收集

2. **失败分类**：
   - 根据 `window-discovery-timeline.json` 的最终状态进行分类：
     - `NO_WINDOW_HANDLES` → `BLOCKED_AUTOMATION` 或应用早期崩溃
     - `ONLY_BLANK_DOCUMENTS` → `FAIL_TEST`/自动化或 WebView 生命周期
     - `APPLICATION_DOCUMENT_NOT_FOUND` → `FAIL_TEST`（非 blank 文档但无产品标记）
     - `APPLICATION_DOCUMENT_FOUND` 但契约超时 → `FAIL_PRODUCT` 候选

##### WQ-P0-WHITE-04A 手工验证

1. **验证超时注入**：
   - 在 gate 运行时检查环境变量：
     ```powershell
     $env:WDIO_STARTUP_DISCOVERY_TIMEOUT  # 应为 "30000"
     $env:WDIO_STARTUP_CONTRACT_TIMEOUT   # 应为 "25000"
     ```
   - 验证 native-startup.mjs 使用这些值作为超时（而非仅依赖默认值）

##### WQ-P0-WHITE-04B 手工验证

1. **验证日志目录契约**：
   - gate 完成后检查 `READINESS_DIAGNOSTICS` 目录下是否有 WDIO 写入的日志文件
   - 确认不再依赖 `Copy-Item -ErrorAction SilentlyContinue` 掩盖日志缺失
   - 检查 `WDIO_LOG_DIR` 与服务实际日志写入目录是否一致

##### WQ-P0-WHITE-04C 手工验证

1. **验证 session start 快照**：
   - 首次成功 gate 运行后检查 `startup/session-start.json` 是否存在
   - 确认文件包含：窗口句柄数、当前 URL、标题和时间戳
   - 验证 snapshot 是在 session 创建后立即记录的（而非测试执行中）

### 2026-09-22 d3fd814 readiness-gate validation results

| ID | Current status | Evidence / follow-up |
| --- | --- | --- |
| WQ-P0-WHITE-01A | BLOCKED | Pinned ordinary WDIO could not create a session, so no application target handle/URL/title timeline was available. |
| WQ-P0-WHITE-01B | FAIL | Exact tauri-driver 2.1.0-alpha.0 started, but EdgeDriver 152 rejected installed Edge 154 before DOM readiness. |
| WQ-P0-WHITE-01C | NOT RUN | Hosted diagnostic workflow was not invoked after the local pinned prerequisite failed. |
| WQ-P0-WHITE-01D | FAIL | Preflight process snapshots contained changing msedgewebview2 state; later state did not equal the preflight-after snapshot. App/driver processes and target ports were clean after failure. |
| WQ-P0-WHITE-03R | BLOCKED | Session failure preceded application-document discovery, so handle/page-source/screenshot/startup evidence was not exercised. |
| WQ-P0-WHITE-04A | PASS (invocation scope) | Both startup timeout variables were present in the ordinary gate invocation; full gate behavior remains unverified because session creation failed. |
| WQ-P0-WHITE-04B | FAIL | The failed run wrote service capture under desktop/logs and left the readiness diagnostics directory without a non-empty WDIO log. |
| WQ-P0-WHITE-04C | BLOCKED | No successful WebDriver session, therefore no session-start snapshot. |

Supporting checks: Linux Desktop 89/89 and Extension 21/21; Windows npm ci,
Node check/test/build, syntax, Tauri release build and direct preflight passed.
No Linux business code was modified.
### 2026-09-22 d3fd814 Edge 154 diagnostic result

The locally downloaded driver is retained at
E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\msedgedriver-154.0.4258.24\msedgedriver.exe.
It reports 154.0.4258.24 and SHA-256
85BABA4548CCE09AB1416816CF17047872E7FBF05B5899C0DFDE454AF63E42D4.

| ID | Status | Evidence / follow-up |
| --- | --- | --- |
| Edge154 driver installation | PASS | Project-local E: driver was available, version and SHA-256 verified. |
| Edge154 ordinary WDIO prerequisite | PASS | WDIO accepted the existing EdgeDriver and reported an exact Edge 154 match; tauri-driver 2.1.0-alpha.0 became ready. |
| Edge154 application startup and target discovery | FAIL | With an absolute WDIO_APP_BINARY, the session remained at data:, for 30000 ms; no XArchive document or root was found. |
| Edge154 Dashboard E2E | FAIL | 0 passed, 1 failed in the startup hook. See the E2E log and startup discovery timeline. |
| Edge154 cleanup | PASS | No target processes or listeners on 4444/4445/1420/9223 remained. |

This diagnostic result does not promote the workflow-pinned EdgeDrive
152.0.4191.66 gate to PASS. Linux follow-up remains required for the Windows
Tauri application launch and WebView target attachment path.

### 2026-09-22 d3fd814 current-source Windows validation rerun

The current dirty Linux source was synchronized one-way to the E: validation
copy with 0 mismatches and 0 failed files. Linux Desktop 89/89, Extension
21/21, check/build, changed-script syntax and git diff checks passed.

| ID | Current status | Evidence / follow-up |
| --- | --- | --- |
| WQ-P0-WHITE-01A | FAIL | Edge154 diagnostic session created one handle, but all 30000 ms samples were data:, / ONLY_BLANK_DOCUMENTS; no XArchive document or root appeared. |
| WQ-P0-WHITE-01B | PASS (diagnostic only) | EdgeDriver 154.0.4258.24 matched Edge 154.0.4258.24. Historical pinned 152 mismatch remains a separate FAIL. |
| WQ-P0-WHITE-01C | NOT RUN | No hosted diagnostic workflow was invoked locally. |
| WQ-P0-WHITE-01D | FAIL | Preflight process snapshots changed because of existing WebView2 process activity, although target processes and ports were clean after the run. |
| WQ-P0-WHITE-03R | PASS | failure.json, handle discovery timeline, current page source and screenshot were all generated. |
| WQ-P0-WHITE-04A | PASS | Both startup timeout variables were injected and reflected in the 30000 ms discovery artifact. |
| WQ-P0-WHITE-04B | FAIL | WDIO log files existed but were zero bytes; requested diagnostics log capture remains incomplete. |
| WQ-P0-WHITE-04C | FAIL | WebDriver session creation succeeded, but startup/session-start.json was not generated. |
| Advanced native gate | BLOCKED | Shared ordinary startup/document discovery failed; advanced execution was not repeated. |
| Hosted/release-runner gate | NOT RUN | Requires hosted workflow invocation and credentials. |
| Installer/asset acceptance | NOT APPLICABLE | Not affected by the current target-discovery diff; release executable build covered the applicable packaging check. |

Required Linux follow-up is to align/control WebView2 and EdgeDriver runtime
versions, diagnose the blank WebView target after tauri-driver session creation,
and correct the session-start and WDIO log capture contracts before re-running
ordinary and advanced native gates. No business code was modified.
### 2026-09-22 Linux 修复：session-start 快照与 WDIO 日志目录契约（测试基础设施轮）

本节登记针对 current-source rerun 暴露的两个 harness 缺陷的 Linux 修复。
依赖源码核实（`node_modules/@wdio/tauri-service/dist/esm/index.js`）确认：

- **04C 根因**：`snapshotSessionStart` 已在 `desktop/e2e/support/native-startup.mjs`
  定义并导出，但 `dashboard.e2e.mjs` 从未调用，因此 session 创建成功也不会
  生成 `startup/session-start.json`；且 snapshot 的 `capabilities` 字段从未填充。
- **04B 根因**：service 日志捕获读取 WDIO config 的 `outputDir`
  （`_config.outputDir || join(process.cwd(), 'logs')`）；service 选项 `logDir`
  只在 standalone `init()` 路径生效，本项目 runner 模式不走该路径。
  `wdio.conf.mjs` 未设置 `outputDir`，因此日志落到 `desktop/logs`，
  诊断目录中的文件为空（零字节）。

本轮修复（仅测试基础设施，业务代码零改动）：

| 文件 | 修改 |
| --- | --- |
| `desktop/e2e/support/native-startup.mjs` | `snapshotSessionStart` 填充 `capabilities`（`browser.capabilities`） |
| `desktop/e2e/specs/dashboard.e2e.mjs` | `before` hook 在 `waitForDashboard` 之前调用 `snapshotSessionStart("dashboard-before-hook")` |
| `desktop/wdio.conf.mjs` | config 显式增加 `outputDir: logDir`，与 `WDIO_LOG_DIR` 一致 |
| `.github/workflows/windows-release.yml` | 01D：隔离检查只对 `xarchive-desktop`/`tauri-driver`/`msedgedriver` 残留和 readiness 端口监听 FAIL（写 `isolation-failure.txt`），msedgewebview2 后台活动降级为诊断信息；04B：gate 结束后检查 `READINESS_DIAGNOSTICS` 下存在非空 `*.log`（写 `log-capture-status.txt`），无日志时 FAIL，不再依赖 `Copy-Item -ErrorAction SilentlyContinue` 掩盖缺失 |
| `desktop/test/native-startup.test.mjs` | 新增 2 个 `snapshotSessionStart` 单元测试（成功快照 + 命令失败仍写文件） |
| `desktop/test/ui-wiring.test.mjs` | 新增 04B/04C/01D 接线断言 |

Linux 验证：`node --check`（5 个改动脚本）、desktop 单元测试 91/91、
Vite check、workflow YAML 解析、`git diff --check` 全部通过。

修复后的 Windows 验证队列状态（其余项目维持 current-source rerun 结果）：

| ID | 状态 | 说明 |
| --- | --- | --- |
| WQ-P0-WHITE-01A | FAIL（维持） | d3fd814 current-source rerun：全部 30000 ms 样本为 `data:,` / ONLY_BLANK_DOCUMENTS。属 Windows Tauri startup/target-attachment 问题，Linux 无法修复。 |
| WQ-P0-WHITE-01B | PASS (diagnostic only)（维持） | EdgeDriver 154.0.4258.24 与 Edge 154 匹配；历史 pinned 152 失败仍为独立 FAIL。 |
| WQ-P0-WHITE-01C | NOT RUN（维持） | 仍需 hosted diagnostic run。 |
| WQ-P0-WHITE-01D | `WINDOWS_VERIFICATION_PENDING` | 隔离检查逻辑已修复（残留 FAIL + msedgewebview2 噪声降级），需 Windows 重新验证。 |
| WQ-P0-WHITE-03R | PASS（维持） | 失败证据完整性已在 rerun 中验证。 |
| WQ-P0-WHITE-04A | PASS（维持） | 超时注入已在 rerun 中验证。 |
| WQ-P0-WHITE-04B | `WINDOWS_VERIFICATION_PENDING` | `outputDir` 契约修复 + gate 日志完整性检查，需 Windows 重新验证：日志应直接落在 `READINESS_DIAGNOSTICS` 且非空。 |
| WQ-P0-WHITE-04C | `WINDOWS_VERIFICATION_PENDING` | `snapshotSessionStart` 已接入 spec，需 Windows 重新验证：session 创建后应生成含窗口句柄数、URL、标题、capabilities 和时间戳的 `startup/session-start.json`。 |

> 更新：上表 01D/04B/04C 已由下方「2026-09-22 d3fd814 Windows revalidation
> reconciliation (22:30)」在本地 workflow 等价 gate 中验证为 PASS，
> `WINDOWS_VERIFICATION_PENDING` 状态就此关闭。

Windows 验证前置条件不变：受控 WebView2/EdgeDriver runtime 对齐
（WebView2 Runtime 153.0.4234.48 vs Edge/driver 154 的配对问题仍待解决），
然后先重跑本地 pinned readiness，再评估 hosted 诊断。

### 2026-09-22 d3fd814 Windows revalidation reconciliation (22:30)

基于 E: 工作副本 `validation-artifacts\current-20260922-gate-2230` 的本地
workflow 等价 gate，更新本轮队列状态：

| ID | 状态 | 本轮证据与后续 |
| --- | --- | --- |
| WQ-P0-WHITE-01A | `WINDOWS_FAIL` | Edge/driver session 创建成功，但 30000 ms 内始终为 `data:,`、`ONLY_BLANK_DOCUMENTS`，`rootExists:false`。需 Linux 后续处理 Windows target attachment/runtime 配对问题。 |
| WQ-P0-WHITE-01B | `PASS (diagnostic only)` | `msedgedriver 154.0.4258.24` 与 Edge 154 匹配；workflow pinned 152 的历史 FAIL 不变。 |
| WQ-P0-WHITE-01C | `NOT RUN` | hosted/release-runner diagnostic 尚未执行。 |
| WQ-P0-WHITE-01D | `PASS` | preflight→gate 隔离检查未发现应用/driver/1420/4444/4445/9223 残留；WebView2 后台活动仅作诊断。 |
| WQ-P0-WHITE-03R | `PASS` | 失败证据完整：session-start、discovery、failure、截图和非空 WDIO 日志均产生。 |
| WQ-P0-WHITE-04A | `PASS` | 30000 ms discovery / 25000 ms contract 超时配置实际生效。 |
| WQ-P0-WHITE-04B | `PASS` | `READINESS_DIAGNOSTICS` 下存在 2 个非空 WDIO 日志。 |
| WQ-P0-WHITE-04C | `PASS` | `startup/session-start.json` 记录 1 个 window handle、`data:,`、capabilities 和 driver 版本。 |

Dashboard assertion、advanced native E2E 及 release asset steps 因 01A 前置失败
分别记为 `BLOCKED`、`BLOCKED`、`NOT RUN`；不能把诊断/隔离 PASS 提升为 UI
readiness PASS。

### 2026-09-22 01A runtime-pairing 调查（Linux 代码级）与新验证项 WQ-P0-WHITE-05A

针对 22:30 reconciliation 中 01A 的两个候选原因（runtime 配对不一致 /
release artifact 未暴露应用文档），本轮在 Linux 端对启动/附加路径做了代码级
核实（零代码修改，仅调查与文档）：

1. **应用窗口配置正常**：`desktop/src-tauri/tauri.conf.json` 只有单个
   `main` 窗口（title `XArchive`），无 `visible:false`、无延迟创建；
   仓库代码中没有任何 `WEBVIEW2_*` / `remote-debugging-port` 引用。
2. **直接启动证据**：此前 Windows 验证已证明普通二进制直接启动可原生渲染
   Dashboard（v2.0.6 clean-install scope），应用本身能完成首次导航。
3. **依赖源码关键发现**（`node_modules/@wdio/tauri-service/dist/esm/index.js`
   `resolveTargetEdgeVersion`，约 L1638-1660）：service 自己的版本判定优先级
   明确写着——msedgedriver 应匹配的是**实际渲染应用的 WebView2 Runtime**
   （优先级：显式 driver pin > 固定 runtime 文件夹
   `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` > 注册表 Evergreen），而不是
   已安装 Edge 浏览器版本。
4. **配对缺口**：E: 机器 Evergreen WebView2 Runtime 为 `153.0.4234.48`，
   实际渲染应用的就是它；但 22:30 诊断 run 使用的 msedgedriver
   `154.0.4258.24` 是按 Edge **浏览器** 154.0.4258.32 匹配的。
   driver(154) vs 实际渲染引擎(153) 的 CDP 错配与 observed 症状
   （session 创建成功、初始 target `data:,`、文档永不加载）一致。
   `startup/session-start.json` 证明从 session 建立瞬间 target 就是 `data:,`，
   与「CDP 握手成功但导航协议不工作」相符。

| ID | 类别 | 验证项目 | Windows 原因 | 精确行为 | 预期结果 | 优先级 | 状态 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| WQ-P0-WHITE-05A | Runtime/Diagnostics | driver 与实际渲染引擎（WebView2 Runtime）版本配对实验 | msedgedriver 需匹配实际渲染应用的 WebView2 Runtime（依赖源码 `resolveTargetEdgeVersion` 判定优先级），此前按 Edge 浏览器版本匹配造成 154 driver 驱动 153 runtime | 路径 A：使用与 Evergreen WebView2 Runtime `153.0.4234.48` 配对的 msedgedriver（优先精确版本 `153.0.4234.48`，不可得时用最接近的 153.0.4234.x），重跑 ordinary gate（同一 exe、同一诊断目录布局）；路径 B：安装固定版本 WebView2 Runtime 154 并设 `WEBVIEW2_BROWSER_EXECUTABLE_FOLDER` 指向该文件夹后重跑 | `session-start.json` 初始 URL 离开 `data:,` 或 discovery timeline 出现 `APPLICATION_DOCUMENT_FOUND`；若仍为 `ONLY_BLANK_DOCUMENTS`，则 runtime 配对假设被证伪，升级为 tauri-driver/WebView2 附加机制调查 | P0 | `WINDOWS_VERIFICATION_PENDING` |

约束：两条路径都不得修改 workflow pinned `msedgedriver 152.0.4191.66`、
产品代码、依赖版本或 readiness 断言；实验通过后仍需按原计划回到
pinned 工具链与 hosted 验证（WQ-P0-WHITE-01B/01C）。

## Manual Windows Validation Queue

### 2026-09-23 E5–E7 / current-source Full package

The 2026-09-23 Computer Use limitation was superseded for checks the user
manually completed on 2026-09-24. The manual report confirms app startup/exit,
Sidecar start/stop, account-batch page display, Chinese-path handling, and
initial Extension connection. It also records task/download failures and a
reconnect failure; these are not attributed to Computer Use. Unperformed
registry, pipe ACL, filesystem-fault, signing, and release checks remain open.

| ID | Status | Manual steps / completion evidence |
|---|---|---|
| MANUAL-WIN-E5-01 | `NOT RUN` | App startup/exit and initial transport status were manually observed; named-pipe endpoint inspection, two sequential Native Host requests, pipe cleanup, and cross-user ACL denial were not tested. |
| MANUAL-WIN-E6-01 | `FAIL` (reconnect outcome); `NOT RUN` (registry assertions) | After reopening the app, the user refreshed, repaired, unregistered, then repaired Native Host again; Extension remained disconnected. HKCU values, manifest executable path, package-move repair, and unregister cleanup were not independently inspected. |
| MANUAL-WIN-E7-01 | `PASS` (initial connection); `FAIL` (after app restart) | Extension loaded in Edge and initially connected/was recognized. After Desktop restart it remained disconnected despite refresh/repair/unregister/re-repair. Screenshots show both states; service-worker diagnostics were not supplied. |
| MANUAL-WIN-FULL-01 | `PASS` (user report; screenshots) | Full package app started and closed with no residual `xarchive-desktop.exe` process. Sidecar start/stop worked; Dashboard showed connected and `hello → ready`. No successful download is claimed. |
| MANUAL-WIN-WDIO-TEARDOWN-01 | `PASS` (closed 2026-09-23; validation revision `26f8c37`) | Ran `node --test desktop/test/wdio-tauri-service.test.mjs` with normal Windows child-process control: 8 passed, 0 failed, including the `killTree` child-termination assertion. The full Desktop suite was not rerun because the current handoff changed documentation only. |
| MANUAL-WIN-FULL-SIDECAR-02 | `PASS` (limited, user screenshot) | The corrected package Dashboard now shows Sidecar connected and the screenshot path is `full-package-sidecar-fix`. This supersedes the prior automation block for the visible running state. No controlled task/download success is established. |

### 2026-09-24 P1/P2/P3 account batch Windows validation

Source/validation input revision: `dac0a153d03fa174c600435c87afabf476aded34` on `feature/u7-desktop-production-integration`. Automated Windows-target/module checks and isolated Full/Core package assembly passed; exact counts and commands are in `windows-validation-history.md`. Full regression was not run.

User evidence confirms manual GUI access was available for this validation. The app launches/closes cleanly; Sidecar start/stop and service status work; the account-batch page renders; Edge initially loads and connects the Extension. Failures: an Extension archive action appears in Dashboard only after refresh and then fails; manually starting aria2 did not make downloads succeed; after app restart, Extension remained disconnected despite refresh/Native Host repair/unregister/re-repair; account discovery stayed at zero candidates/pending, and after pause discovery remained `RUNNING` for about 30 seconds until the user cancelled the batch. Account and Tweet identifiers are fully redacted. aria2 shown as “not detected” is evidence only, not a confirmed root cause.

| ID | 类别 | 验证项目 | 关联修改/目标 | Windows 原因 | 前置条件 | 精确手工步骤 | 预期结果 | 优先级 | 状态 |
|---|---|---|---|---|---|---|---|---|---|
| MANUAL-WIN-BATCH-01 | Runtime/GUI | 账号归档页面与 Tauri command surface | `desktop/src/pages/batches-page.jsx`、`main.jsx`、`commands.rs`、`lib.rs` | WebView2 渲染、真实 Tauri IPC、窗口生命周期和 DPI 只能由 Windows 验证 | Windows 10/11、WebView2、Full package、Sidecar | 打开账号归档页面，检查表单、筛选、Badge 和批次列表；观察批次发现/候选/提交/完成/失败计数；保存脱敏截图及日志 | 页面与内容显示正常（PASS）；Extension 提交任务后需刷新 Dashboard 才出现，出现后失败（FAIL）；批次创建后候选/待提交持续为 0 | P0 | Page display `PASS`; task visibility/completion `FAIL` (visible only after refresh, then failed); candidate discovery `FAIL` (zero candidates/pending during observed run) |
| MANUAL-WIN-BATCH-02 | Runtime/Recovery | 暂停、继续、取消、失败重试和重启恢复 | `batch_cancellation`、`dispatch_batch_pass`、`archive_batches`/`batch_candidates` | 需要真实长任务、进程取消、Windows 文件锁和重启行为 | Full package、当前账户批次 | 创建批次；在发现阶段点击暂停；记录状态和等待时间；之后主动取消。继续、失败重试及重启恢复需另行测试 | 暂停后发现仍显示 `RUNNING` 约 30 秒，用户因无进展主动取消；取消后 UI 显示 `CANCELLED`，发现阶段仍显示 `RUNNING`。继续、重试、重启恢复 `NOT RUN` | P0 | Pause/progress observation `FAIL` (behavioral expectation/root cause pending); resume/retry/restart recovery `NOT RUN` |
| MANUAL-WIN-BATCH-03 | Extraction/Media | 真实 gallery-dl 多页发现与媒体完整性 | Sidecar `discover`、`extraction.py`、`archive_completeness.rs`、aria2/ArchiveService | 需要成功的真实抓取/下载和文件 | Full package、gallery-dl、aria2、测试下载目录 | 执行发现并下载；覆盖媒体类型；检查候选去重、文件数量/名称/大小/SHA-256及重复归档行为 | 本次未能正常抓取或下载；批次没有进入可验证的媒体完成状态。下载/抓取 `FAIL`；文件产物、内容、大小、SHA-256及完整性规则 `NOT RUN`。手动启动 aria2 后仍失败；设置页显示“未检测到 aria2”，根因未确定 | P0 | Extraction/download `FAIL`; file-integrity checks `NOT RUN` (no successful downloads) |
| MANUAL-WIN-BATCH-04 | Browser/Credential | 账号归档与浏览器凭据/Extension/Native Host 集成 | `desktop/src-tauri/src/archive.rs`、Sidecar browser/profile、Transport、Extension | Cookie/Profile、Named Pipe、Registry、ACL 和浏览器连接属于 Windows 平台边界 | Edge、Full package、Native Host | 加载 Extension，观察初次连接；点击 Tweet 归档按钮；重启 Desktop 并测试刷新/修复/取消注册/再次修复；检查日志/SQLite 是否含敏感凭据 | Edge 加载及初次识别/连接 `PASS`; 点击后任务需刷新才出现且随后失败 `FAIL`; 应用重启后 Extension 未连接且修复流程未恢复 `FAIL`; 凭据脱敏检查 `NOT RUN` | P0 | Initial load/connection `PASS`; task propagation and restart reconnect `FAIL`; credential-redaction verification `NOT RUN` |
| MANUAL-WIN-BATCH-05 | Filesystem/Packaging | Windows 路径、ACL、锁、打包和发布门禁 | `FileStore`、staging、aria2 supervisor、portable package/release workflow | 文件系统边界和发布签名需 Windows 现场验证 | Full/Core package、独立测试目录 | 验证中文/长路径、跨卷、锁、异常退出和 staging；检查 package boundary；运行签名/release gate | 中文路径 `PASS`（用户报告）；先前 Full/Core 组装及静态 manifest 检查仍 `PASS`；长路径、跨卷、文件锁、异常退出、staging、签名和 release gate 均 `NOT RUN` | P0 | Chinese path `PASS`; Full/Core static package checks `PASS`; remaining filesystem/signing/release checks `NOT RUN` |

已观察到失败的项目保持 `FAIL` 并进入对应 owner 的诊断；未执行或缺少前置条件的项目保持 `NOT RUN`/`BLOCKED`，不得改为 PASS。手工执行时必须记录：branch、source revision、Windows version/architecture、WebView2/Edge/Chrome/gallery-dl/aria2 版本、账号类型（完全脱敏）、命令、截图/日志/SQLite 摘要、artifact SHA-256、实际结果和未执行原因。

### 下一 Windows batch：共享修复 revalidation

输入基线为 `89258c52100113a6a1cdfccf25f16ae915a45668` 后的 Linux handoff。
上一节的历史 `FAIL`/`NOT RUN` 不删除，但所有受本轮共享代码影响的项目必须针对新
handoff revision 标记 `REVALIDATION_REQUIRED`；历史 PASS 只在其依赖未变时继续有效。

| ID | 状态 | 手工步骤 / 预期结果 |
|---|---|---|
| MANUAL-WIN-BATCH-REVAL-01 | `REVALIDATION_REQUIRED` | 从 Browser/Native Host 提交一个受控 Tweet；不点击 Dashboard 刷新，等待不超过 3 秒，确认 Job 自动出现并展示持久化状态；保存截图、Extension/service-worker/Rust 日志和时间戳。预期：无需手工刷新，失败码/消息可见且不含 URL userinfo/query token。 |
| MANUAL-WIN-BATCH-REVAL-02 | `REVALIDATION_REQUIRED` | 对新账号批次执行多页发现；首条 candidate 出现时从独立 SQLite 只读连接查询 `batch_candidates`，确认早于 `discovery_completed` 落库；核对临时目录不含外部 `job_id` 路径组件，结束后已清理。 |
| MANUAL-WIN-BATCH-REVAL-03 | `REVALIDATION_REQUIRED` | discovery RUNNING 时点击暂停；确认 batch 与 discovery 同时显示 `PAUSED`，候选停止增长；继续前等待旧 worker registry 释放，再确认仅一个 worker；取消后确认 PENDING 变 CANCELLED、SUBMITTED Job 不被取消。重复快速点击 pause/resume，确认无 RUNNING→CANCELLED 覆盖或双 worker。 |
| MANUAL-WIN-BATCH-REVAL-04 | `REVALIDATION_REQUIRED` | 在未设置 `XARCHIVE_ARIA2_RPC_SECRET` 的干净进程中执行一个媒体 Tweet；确认 Desktop 启动受控 loopback aria2、RPC 握手和下载进入既有状态机；检查普通日志/SQLite/前端诊断不包含 RPC secret。预期：不再以 `ARIA2_NOT_CONFIGURED` 失败。 |
| MANUAL-WIN-BATCH-REVAL-05 | `REVALIDATION_REQUIRED` | 使用 v5 数据库副本启动新 Full/Core artifact，确认 migration 到 v6 后 batch/candidate/job 行数与 ID 不变，`PRAGMA foreign_key_check` 无输出；执行 pause/cancel/retry。记录数据库 SHA-256 前后值和迁移日志。 |
| MANUAL-WIN-BATCH-REVAL-06 | `REVALIDATION_REQUIRED` | 保持 Extension/Native Host 原失败步骤：Desktop 重启后检查连接，执行 refresh/repair/unregister/re-register；保存 HKCU 两项、Native Host manifest、Named Pipe、Service Worker 和进程日志。该项仍由 Windows Platform Owner 实现/诊断，本轮只累积验证，不提升为 PASS。 |

2026-09-24 补充验证：`MANUAL-WIN-BATCH-REVAL-01` 的 Full-package native dashboard startup 子项已由固定 Runtime WDIO smoke 验证 PASS（3/3）；真实 Extension archive submission/自动可见性仍未重测。`MANUAL-WIN-BATCH-REVAL-04` 的本地 aria2 组件子项 PASS（版本、RPC、pause/unpause、localhost Range 下载与 SHA-256），但通过 Desktop/Sidecar 对真实提取结果启动 aria2 并完成归档仍 NOT RUN。详见 `windows-validation-history.md` 的 2026-09-24 targeted revalidation 补充记录。

### 2026-09-24 current-source automated revalidation (`553e378`)

Source/validation input revision: `553e3788467041ef43ac5657c969064ce45d016c`. The prior user-observed failures above belong to `full-package-r1` at `dac0a153` and remain historical until retested against this revision. The current Linux batch adds shared executor replacement and gallery-dl JSONL adapter changes; Windows product source was not changed in this Linux batch.

| ID | Current result | Evidence / remaining action |
|---|---|---|
| MANUAL-WIN-BATCH-REVAL-01 | `REVALIDATION_REQUIRED` | Prior current-source WDIO dashboard startup passed 3/3 in a normal Windows process after forwarding the pinned WebView2/driver paths. Real Extension archive submission, automatic Dashboard visibility, duplicate suppression and execution remain unverified on the new handoff; the earlier `EXECUTOR_UNAVAILABLE` result is historical and must not be promoted. |
| MANUAL-WIN-BATCH-REVAL-02 | `REVALIDATION_REQUIRED` | Windows-target affected Rust tests and Sidecar discovery tests passed. Controlled multi-page real gallery-dl account discovery, live SQLite timing, canonical JSONL candidate identity, and temporary-directory lifecycle remain unverified. The synthetic real-shape JSONL fixture is Linux evidence only. |
| MANUAL-WIN-BATCH-REVAL-03 | `REVALIDATION_REQUIRED` | Pause/cancel controls were observed in the prior package, but the ~30-second `RUNNING` observation belongs to `dac0a153`. Re-run pause/resume/cancel/retry/restart recovery against this handoff; old-worker generation and single-worker semantics are Linux-tested, not Windows-accepted. |
| MANUAL-WIN-BATCH-REVAL-04 | `REVALIDATION_REQUIRED` | Current-source frozen worker handshake/invalid-field/shutdown and independent local aria2 component checks passed. Application-managed extraction from real gallery-dl output, aria2 transfer, SHA-256 archive completeness and secret redaction remain `NOT RUN`; the earlier executor failure is not a current-source result. |
| MANUAL-WIN-BATCH-REVAL-05 | `REVALIDATION_REQUIRED` | Windows-target migration tests passed and a prior user report says the package migration completed. Open a copied v5 database in the new Full/Core artifact, compare batch/candidate/job row counts and IDs, and capture `PRAGMA foreign_key_check`; the user report alone lacks this evidence. |
| MANUAL-WIN-BATCH-REVAL-06 | `REVALIDATION_REQUIRED` | Current-source Native Host Windows tests and release/package static checks passed. Live Edge, HKCU registration, Named Pipe and Desktop restart reconnect remain unverified. The prior refresh/reconnect `FAIL` is bound to the older artifact and must not be silently reused. |
| Full Desktop Node workspace suite | `BLOCKED` (runner did not terminate) | The full command did not produce a completion summary after about two minutes and was interrupted. The narrower Desktop UI wiring suite passed 20/20. Re-run the workspace suite in a healthy runner. |
| Current-source Full package dashboard launch | `BLOCKED` (automation/environment) | WDIO worker failed before launching the app with Node `uv_os_get_passwd returned ENOMEM`; fixed versions were WebView2 `153.0.4234.48`, EdgeDriver `153.0.4234.46`, and tauri-driver `2.0.6`. This is not a product assertion failure. |

Computer Use retry exhausted for this batch: native app inventory was empty and the available API had no native `launch_app` operation. Keep GUI-bound rows blocked and continue non-GUI checks. No cross-platform change/review was identified from current-revision automated evidence. Next owner remains Windows Platform Owner.

本批次不存在 `WINDOWS_BLOCKING`。上述项目可在新正式 handoff push 后立即进入下一
Windows batch；若真实账号/签名 artifact 不可用，保持 `NOT RUN`，不得把共享 module
tests 写成 Windows acceptance。

### 2026-09-24 user manual revalidation report (current Extension/account-batch build)

The user supplied six screenshots and observations from the current Windows build. The exact package source revision and artifact hash were not included in this report; preserve this as user-reported evidence and bind it to an exact artifact before using it as a release acceptance result. All account names, Tweet IDs, batch IDs, and account URLs are intentionally omitted.

| ID | Result | User-observed evidence / disposition |
|---|---|---|
| MANUAL-WIN-BATCH-REVAL-01 | `FAIL` (executor); task visibility and deduplication `PASS` | Edge Extension loads, the page action appears and can be clicked. A task appears in Dashboard after about 0.5 seconds without refresh. Repeating the same item does not create another task; another item creates a separate task. Each task then fails with `EXECUTOR_UNAVAILABLE: job executor is closed`. Preserve the exact error code/message; investigate the executor lifecycle before claiming end-to-end archive success. |
| MANUAL-WIN-BATCH-REVAL-02 | `FAIL` (discovery); pause/cancel controls `PASS` (UI observation only) | Account discovery produced no candidates/tasks for two tested accounts. The batch UI showed one batch, one in progress, zero candidates, zero submitted, zero completed, and zero failures. Pause and cancel could be used normally. Worker termination, SQLite candidate persistence, and multi-page discovery internals were not independently inspected. User recommends temporarily disabling account discovery until its implementation is revisited; this is a product recommendation, not a code change in this validation report. |
| MANUAL-WIN-BATCH-REVAL-03 | `PASS` (pause/cancel controls only); remaining semantics `NOT RUN` | The current report says discovery can be paused and cancelled. It does not establish that candidates stop growing, the old worker releases its registry, only one worker resumes, submitted jobs are preserved, or late results cannot change terminal state. Keep those assertions `NOT RUN`. |
| MANUAL-WIN-BATCH-REVAL-04 | `BLOCKED` by executor failure | The Extension request reaches Dashboard but fails before successful job execution with `EXECUTOR_UNAVAILABLE`. No successful application-managed aria2 transfer, final file, or integrity result was produced. The independent aria2 binary/RPC fixture remains a separate PASS and does not change this result. |
| MANUAL-WIN-BATCH-REVAL-05 | `PASS` (user-reported package migration) | User reports the v5-to-v6 database migration test completed normally. Before/after database hashes, row counts, and `foreign_key_check` output were not included; retain those as evidence gaps if this is needed for release sign-off. |
| MANUAL-WIN-BATCH-REVAL-06 / WQ-EXT-E7-01 | `FAIL` (refresh did not reconnect); further diagnosis deferred | Clicking Refresh in Desktop Settings did not reconnect the Edge Extension; the UI showed Extension disconnected while Sidecar was connected. At the user's request, defer additional reconnect testing until the planned Extension refactor. This observed failure remains recorded and is not converted to PASS or NOT RUN. |
| WQ-EXT-E5-01 / WQ-EXT-E6-01 | `NOT RUN` (deep transport/registry checks) | This report does not include Named Pipe endpoint/ACL evidence, HKCU registry values, manifest path/origin inspection, or unregister cleanup checks. |
| MANUAL-WIN-BATCH-05 / WQ-U7-04 / WQ-U7-05 | `NOT RUN` (user expectation: PASS) | Staging, long/cross-volume paths, file locks, abnormal exit, recovery, and cleanup were skipped. “Expected PASS” is the user's expectation only; no test result is claimed. |
| WQ-U7-01 / WQ-U7-02 / WQ-U7-03 | `NOT RUN` or upstream `BLOCKED` | The current screenshots do not establish real packaged extraction, completed app-managed aria2 transfer, or expired-URL refresh/new-GID behavior. The closed executor currently prevents the normal task from reaching those validations. |

#### Triage and ownership

- `CROSS_PLATFORM_CHANGE_REQUIRED`: resolved by the Linux handoff; real Windows revalidation is now the next step.
- Account discovery disablement is a user recommendation pending implementation/owner decision. Do not treat this report as evidence that a feature flag or disablement has already been implemented.
- Extension refresh/reconnect remains a Windows-observed failure, with further acceptance deferred until the Extension refactor. Reopen WQ-EXT-E5/E6/E7 after that work is available.
- Filesystem/staging/fault recovery remains `NOT RUN`; the user expects PASS, but this expectation is not a test result.
- Report date: 2026-09-24. Identifiers are fully redacted; do not add screenshot-derived account or Tweet identifiers to Git documentation.
### 2026-09-24 WebSocket / Extension GUI Windows queue

本批次 Linux shared implementation 已完成；以下项目必须绑定包含本批改动的精确 Git revision。Linux 的 listener、Extension fake WebSocket、发布清单和 GUI 静态检查不能替代 Windows Edge/Chrome 实机证据。

| ID | 类别 | 验证项目 | 关联修改 | Windows 原因 | 前置条件 | 精确行为 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-WS-01 | Browser/Runtime | Edge/Chrome Extension load and WebSocket permission | `extension/manifest.json`、popup/options、WebSocket bridge | 浏览器 MV3 service worker、host permission、真实 WebSocket API 和版本行为需实机 | Windows Edge/Chrome 116+、current Full package/Extension ZIP at `6d60429`、controlled profile | 分别加载 Extension；打开 popup/options；检查 manifest 无错误、Service Worker 正常、popup 可打开设置、options 可保存设置 | Extension 可加载；GUI 可操作；无 CSP/permission/import 错误；不能因静态 Node 测试替代 | P0 | no | `BLOCKED` for full acceptance; current read-only Edge check `PASS` for X Home and injected `XArchive：保存` button rendering. Profile/package identity, manifest permissions, and service-worker health remain unverified; exact remaining GUI steps stay in the manual queue. |
| WQ-WS-02 | Security/Pairing | Authenticated loopback WebSocket | `websocket_transport.rs`、`websocket-bridge.js`、Desktop `get_extension_status` | token 配对、错误 token、未认证请求和本地 token 展示需目标环境 | current Desktop Full package, Edge/Chrome, controlled profile | 复制当前 Desktop 设置中的端口/token 到 options；先测试错误 token，再测试正确 token；发送 `query_status`/`archive_request`；重启 Desktop 重复配对 | 错误 token 明确认证失败且不调用 business adapter；正确 token 后请求成功；token 不进入 Browser payload、Job spec 或日志；端口变化可诊断 | P0 | no | `FAIL` for the current user attempt: one WebSocket was accepted and its auth frame received, then Desktop returned `AUTHENTICATION_FAILED`. Re-pair with the token from the currently running Desktop process; successful authentication and request/response remain `NOT RUN`. |
| WQ-WS-03 | Lifecycle | Worker/Desktop reconnect and pending cleanup | WebSocket retry、Native fallback、RuntimeState executor replacement | MV3 worker 休眠/重启、Desktop 重启和旧 executor 代际需实机 | WQ-WS-01/02、controlled test fixture | 请求 pending 时停止/重启 Desktop、重载 Service Worker、断开/恢复网络；观察 popup/options 状态；再次提交新 request_id | pending 明确失败且不串线；worker 重启从 storage 恢复并重新认证；重连有界；旧 executor 不接收新请求；Native fallback 状态明确 | P0 | no | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE`; current handoff lifecycle sequence not run. |
| WQ-WS-04 | GUI/Accessibility | Popup/options visual and keyboard behavior | `extension/popup.*`、`extension/options.*`、截图参考样式 | WebView/browser chrome、DPI、键盘和焦点需 Windows 浏览器验证 | WQ-WS-01、controlled Edge/Chrome profile | 100%/125%/150% 缩放检查 popup/options；测试 Tab、Enter、Space、Escape/关闭、长状态文案和设置保存失败 | 状态层级清晰；无溢出/遮挡；焦点可见；保存失败保留输入；显示未配置/连接中/已认证/认证失败/回退等真实状态 | P1 | no | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE`; current-revision scaling/keyboard/focus/error states not run. |
| WQ-WS-05 | Packaging | Extension ZIP/Full package includes GUI and WebSocket bridge | `release-assets.mjs`、`native-host-package.mjs`、manifest/package scripts | 真实 Windows artifact、文件编码、Extension ID 和发布 zip 只能由 Windows/CI 验证 | 固定 Extension ID、local validation tag, Windows runner | 生成 Extension ZIP/Full package；检查 popup/options/src bridge 文件、版本、hash、包内测试文件与构建产物；Token 隐私/安全检查按用户要求跳过 | Static package assembly/inventory and archive integrity; browser load remains separate | P0 | no | `PASS` at `6d60429`: fresh Windows Tauri executable, Full package manifest/required files/Extension identity parity, Extension ZIP inventory 12/12, Bandizip archive test, and 76/76 extracted-file SHA-256 match. No browser load, signing, or release publishing is claimed. |

### WebSocket / Extension GUI BLOCKED 手工验证步骤

以下步骤适用于 WQ-WS-01 至 WQ-WS-05。缺少 Windows、Edge/Chrome、WebView2、Desktop release 或受控 X 页面时，跳过对应步骤并记录 `WINDOWS_BLOCKED`，不得记为 PASS：

Current source input is `6d60429f818322b0f9a07a4e4f429d7b5bac7ee3`. The corrected package is `validation-artifacts/windows-batch-revalidation-6d60429-sidecar-current`. WQ-WS-02 now has a user-observed `AUTHENTICATION_FAILED` after a received auth frame; use the token from the currently running Desktop process for the next attempt. WQ-WS-03/04 remain `BLOCKED` — `COMPUTER_USE_UNAVAILABLE`; profile identity is not exposed by the browser adapter. The Desktop executable SHA-256, archive hash, and detailed dated results are in `windows-validation-history.md`.

#### 2026-09-27 Manual Windows Validation Queue

| Field | Record |
|---|---|
| IDs / names | WQ-WS-01 Extension load and permissions; WQ-WS-02 WebSocket authentication and request/response; WQ-WS-03 reconnect and pending cleanup; WQ-WS-04 popup/options scaling and keyboard behavior |
| Related change | Shared accepted-stream blocking-mode fix and diagnostics in `6d60429`; Extension parsed-host Tweet URL validation; exact package/Extension artifacts listed above |
| Status / blocker | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE`; two Computer Use inventories exposed no native app target and only the existing Edge user profile |
| Purpose | Verify the new Windows socket fix in a real browser/runtime, then verify authentication, request routing, lifecycle cleanup, and current UI behavior |
| Prerequisites | Controlled/disposable Edge or Chrome profile; fresh Desktop package and Extension ZIP above; local synthetic/test-owned fixture; no signed-in account |
| Steps | 1. Load the package Extension and ZIP in separate disposable profiles; inspect manifest, permissions, Service Worker, Popup and Options. 2. Refresh Desktop diagnostics, reconnect once, and correlate the same DevTools socket with `handshake_failed`, `auth_read_failed`, and `auth_received` deltas. 3. Test an intentionally wrong token and then the current token; send `query_status` and verify matching `request_id`. 4. With a synthetic request pending, restart Desktop/Extension worker and interrupt/recover the connection; check cleanup, bounded reconnect and no replay. 5. Check Popup/Options at 100%, 125%, 150%, plus Tab/Enter/Space/Escape, focus, long status and save-error behavior. |
| Expected result | Extension loads without manifest/permission errors; wrong token is rejected before business routing; correct token authenticates and returns the matching request; pending work is cleaned and reconnect is bounded; UI remains usable at the listed scales and keys |
| Evidence / result | No current-revision GUI evidence collected. Automated Windows socket regression passes 1/1 at `6d60429`; do not promote it to live browser acceptance. Record browser/runtime versions, redacted screenshots/logs, exact socket counter deltas and artifact hash when completed. |
| Notes | Never record the pairing token. The earlier `fcde594` user-reported pairing failure remains historical; exact current-revision behavior is unknown until these steps run. |

1. 记录 Windows 版本、架构、Edge/Chrome/WebView2 版本、Desktop artifact 路径和 SHA-256；确认 Extension 从当前工作树或正式 Git revision 加载。
2. 启动 Desktop，在设置页确认 WebSocket 端口，并按需用当前会话 token 进行功能性配对。用户说明该 token 仅用于本地验证且每次重启后重置；其隐私/安全性检测为 `NOT APPLICABLE`，不执行 token 暴露风险评估或强制轮换。确认 Options 能保存并恢复设置。
3. 先输入错误 token，发送 `query_status`/`archive_request`，确认 UI 显示认证失败、Desktop 不会进入业务 adapter；再输入正确 token，确认认证成功后请求可以完成。
4. 在请求 pending 时停止/重启 Desktop、重载 Service Worker、断开/恢复 WebSocket；确认 pending 请求明确失败、旧响应不串线、新 request_id 可重新提交，认证失败不会无限重试。
5. 依次测试 100%、125%、150% 缩放；使用 Tab、Enter、Space、Escape 检查焦点、可见焦点环、关闭行为、长状态文案、保存失败输入保留和无横向溢出。
6. 生成 Extension ZIP/Full package，核对 popup/options、WebSocket settings/bridge、manifest、Native Host fallback 文件、版本和 hash，并检查无 tests、node_modules、缓存、日志或私钥。按用户要求跳过配对 token 的隐私/安全性检查。

### 2026-09-24 WebSocket Windows batch result (`084354a`)

- Windows source/validation input revision: `084354a5ca433b52372aca4bc70ac5fc544104fc` on `feature/u7-desktop-production-integration`.
- Windows production implementation: none. The batch validated the Linux handoff on Windows; no source-code changes were needed. Windows validation artifacts are isolated under `validation-artifacts/windows-ws-084354a/`; pre-existing local dependencies, logs, manual-validation files, and artifacts were preserved.
- Environment: Windows 11 x64; Node `24.19.0`; Python `3.12.14` / PyInstaller `6.22.3`; WebView2 Fixed Runtime `153.0.4234.48`; EdgeDriver `153.0.4234.46`; tauri-driver `2.0.6`.

| Item | Status | Evidence | Limits / follow-up |
|---|---|---|---|
| Windows-target Rust tests | `PASS` | `cargo test -p xarchive-core -p xarchive-protocol -p xarchive-desktop --target x86_64-pc-windows-msvc --offline --target-dir validation-artifacts/windows-ws-084354a/target --quiet`; 18 + 19 + 109 = 146 tests passed | Linker emitted non-fatal incremental-cache access warnings; command exit was 0. |
| Desktop Node suite | `PASS` | `npm test --workspace desktop` under normal Windows child-process control; 93 passed, 0 failed | A restricted attempt first failed the `killTree` child-process assertion and was interrupted because process inspection/termination was denied. Re-running the suite with normal process-control permission passed; this was an execution boundary, not a source fix. |
| Extension tests | `PASS` | `npm test --workspace extension`; 25 passed | Does not exercise real MV3/browser storage or GUI. |
| Sidecar affected tests | `PASS` | `.venv-windows-validation\Scripts\python.exe -m pytest sidecar\\tests\\test_discovery.py sidecar\\tests\\test_extraction_only.py -p no:cacheprovider --basetemp validation-artifacts/windows-ws-084354a/pytest-temp`; 15 passed | Does not exercise real account extraction or media download. |
| Desktop production frontend | `PASS` | `npm run check --workspace desktop`; Vite transformed 52 modules | Existing Tauri API dynamic/static import warning remains non-fatal. |
| Extension ZIP | `PASS` (inventory/extraction only) | `XArchive-v0.0.0-pre.1-extension.zip`, 16,642 bytes; 12-file inventory verified after extraction; SHA-256 `779F54F69528EFEE34EDD6F7FF8D6D412FEC7E5FEC9EC4BB008E587049944820` | Synthetic local pre-release tag is for package validation only; no release artifact was published. Browser loading remains `NOT RUN`. |
| Current-source Full package | `PASS` (assembly/static contract) | Built with isolated Tauri CLI release output, fresh PyInstaller worker, current Extension, Native Host and local Full-package gallery-dl; package manifest type `full`; required 11 paths present; Extension inventory has 15 files; app SHA-256 `6B5284DBDD1BB843BCF49DEC296024F8391134B261D65D3CB2FE5C64303133EF`; worker SHA-256 `8F4FBBDB5AC084FCD73B5DD899695F67AEFC8540695FD8D87A0515C121EAEAD4` | `aria2` is optional and not included in this assembled package. No real account/transfer or Extension registration is implied. Summary: `validation-artifacts/windows-ws-084354a/full-package-summary.json`. |
| Full package Dashboard startup/readiness | `PASS` | `npm run test:e2e:windows --workspace desktop` against the isolated Full package with WebView2 `153.0.4234.48`, EdgeDriver `153.0.4234.46` and tauri-driver `2.0.6`; URL `http://tauri.localhost/`; 3/3 passed | Driver must be discoverable on `PATH`; setting `EDGEDRIVER_PATH` alone is not consumed by the service. An initial direct `cargo build` artifact used `localhost:1420` without the Tauri CLI frontend build and failed; rebuilding through `tauri build --no-bundle` embedded the frontend and the final smoke passed. Neither failed setup attempt is treated as final product status. |
| Computer Use / Extension GUI | `BLOCKED` — `COMPUTER_USE_UNAVAILABLE` | Bounded retry returned `apps: []`; browser surface showed only the already-running user Edge profile, and no native app launch target was available | Did not inspect or manipulate the user's signed-in tabs. Edge/Chrome Extension load, pairing/auth, reconnect, and popup/options keyboard/scaling remain in the Manual Windows Validation Queue. |
| WQ-WS-01 through WQ-WS-04 | `BLOCKED` | No controlled browser profile/native GUI session was available | Continue only in a disposable Edge/Chrome profile with a local test Desktop token and synthetic request fixture; do not use a signed-in account as a substitute. |
| Release signing/publishing | `NOT RUN` | No signing or release workflow was invoked | Requires the release signing environment and a release target. |

#### Triage and ownership at the time of automated validation

- At that point, `CROSS_PLATFORM_CHANGE_REQUIRED` had not been identified from automated/package evidence, and no shared implementation had been changed.
- The subsequent user-reported manual result below supersedes that pending classification for WebSocket connectivity: the UI rendered but the Desktop/Extension connection failed. Component-level root cause remains unisolated.
- The earlier account-discovery/executor/reconnect results remain tied to their separately recorded artifact/revision and are not upgraded or cleared by the WebSocket Dashboard smoke.

### 2026-09-24 Full package manual WebSocket follow-up (`windows-ws-084354a`)

User-reported manual result from the Full package assembled for the `084354a` input. The screenshots identify the loaded Extension directory as `validation-artifacts/windows-ws-084354a/full-package/extension`; the app was the corresponding Full package. Account names, Tweet IDs, Extension identity, and the pairing token are intentionally omitted. The token was masked in the supplied settings screenshot and was not transcribed.

| Item | Status | User-reported evidence | Remaining limit / follow-up |
|---|---|---|---|
| Full package app startup, close, and main content | `PASS` (manual report) | User confirmed the app starts, closes, and displays its content normally. | Does not cover abnormal exit, restart recovery, or release acceptance. |
| Extension folder load and basic UI | `PASS` (manual report) | Extension loaded from the Full package directory; options/settings and popup opened and displayed. | No evidence for keyboard navigation, zoom/DPI scaling, or ZIP installation. |
| Desktop ↔ Extension WebSocket connection | `FAIL` (manual report) | Options showed port `17321`, masked token, and a connecting message; the status area showed WebSocket disconnected. Popup showed Desktop disconnected and WebSocket connection closed. Desktop's Extension section showed no Native Host request / no active request and the service was not connected. | This confirms a user-visible integration failure, but does not isolate whether the listener, token pairing, browser runtime, or Extension bridge is the cause. Requires shared transport diagnosis; do not infer successful authentication or request delivery. |
| WQ-WS-03 reconnect and pending cleanup | `NOT RUN` | No controlled pending request, Service Worker restart, network interruption, or Desktop restart/reconnect sequence was reported. | Test lifecycle and pending cleanup after connection is diagnosed. |
| WQ-WS-04 basic visual display | `PASS` (limited) | Supplied screenshots show options/settings and popup content rendering. | 100/125/150% scaling, keyboard/focus behavior, overflow and save-failure behavior remain `NOT RUN`. |
| WQ-WS-05 browser load from package | `PASS` (Full package Extension directory only) | User reports the bundled Extension loaded and the UI opened from the package's `extension` directory. | ZIP browser loading remains `NOT RUN`; package load does not imply successful WebSocket pairing. |

Evidence source: user report and three supplied screenshots on 2026-09-24. No independent GUI capture or token/auth diagnostic was performed in this documentation update.

### 2026-09-24 Extension ZIP manual load and pairing retry

User reports that the Extension ZIP loads successfully and its UI appears. Using the pairing token generated in Desktop, both the automatic copy action and manual token entry failed to connect. The supplied Popup screenshot shows Desktop `未配置`, Native Messaging fallback, WebSocket `未配置 · 端口 17321`, and a closed WebSocket status. No account name, Tweet ID, Extension identity, or token value is recorded.

| Item | Status | Evidence / limit |
|---|---|---|
| Extension ZIP browser load and Popup display | `PASS` (user-reported) | User confirms the ZIP loads and Popup UI displays. This closes the prior ZIP-load `NOT RUN` only for this tested package/browser run; it does not establish service-worker diagnostics, permission correctness, or WebSocket operation. |
| App-generated token pairing | `FAIL` (user-reported) | Both copy-to-clipboard and manual entry failed; Popup remained WebSocket unconfigured/closed on port `17321` and indicated Native Messaging fallback. The screenshot does not reveal whether the token was persisted or whether Desktop received a WebSocket handshake. |
| Authentication result / request routing | `NOT RUN` | No verified connected state or controlled `query_status`/archive request was reported. This is not evidence that the server rejected a valid token. |

Evidence source: user report and supplied screenshot on 2026-09-24; no independent browser interaction or token inspection was performed.

### 2026-09-24 Later Full package WebSocket diagnostic evidence

This is a later point-in-time follow-up for the same Full package. Keep the earlier connection-refused evidence as historical; the later listener and one successful WebSocket upgrade show that reachability changed across attempts/times. Account names, Tweet IDs, pairing tokens, and Extension identity remain fully redacted.

| Item | Status | Evidence / limit |
|---|---|---|
| Loopback listener / TCP reachability | `PASS` (user-reported, point-in-time) | Listener bound to `127.0.0.1:17321` under the Full package Desktop process; `Test-NetConnection` returned `True`. Does not establish authentication or request handling. |
| WebSocket upgrade | `PASS` (single DevTools request) | One `ws://127.0.0.1:17321/` request returned `101 Switching Protocols`. This is only the HTTP upgrade result. |
| Authentication and response flow | `FAIL` (user-reported) | Extension remained disconnected/unauthenticated; Desktop showed Native Host requests arriving and WebSocket unauthenticated. DevTools showed an outbound authentication frame and no inbound authentication response; user reports no requests received responses. Token acceptance/rejection and server-side receipt are `NOT RUN`/unknown. |
| Native Host task submission | `PASS` (limited) | X-page archive button created a task and Native Host activity was visible. Task execution/download is not established. |
| Root cause | `NOT ISOLATED` | Earlier `ERR_CONNECTION_REFUSED` and later TCP/101 evidence differ by attempt/time. Client lifecycle race is only a hypothesis. Correlate client close and server accept/auth/close events in a controlled run, without logging the token. |
| Lifecycle recovery and request completion | `NOT RUN` | No controlled pending request, Service Worker restart, or end-to-end WebSocket request/response was completed. |

Classification remains `CROSS_PLATFORM_CHANGE_REQUIRED`; no code fix or responsible component has been established. See `windows-validation-history.md` for the detailed chronological record.

### 2026-09-25 Windows revalidation: current revision status

- Exact source/validation input: `98f16845b9e37f0dea419e7bffc890b1be3d2063`; shared implementation under test: `de46a8ee7ac937f9ed64db5f16b9e8c4cb1c178d`.
- Windows production implementation: none. The tracked tree was clean at handoff; unrelated untracked Windows dependencies, logs, manual files, and validation artifacts were preserved. Build/test outputs are isolated under `validation-artifacts/windows-ws-de46a8e/`.
- Automated `PASS`: WebSocket Rust tests 4/4; full Windows Desktop Rust crate 111/111 (with repository Python configured); Desktop Node 93/93; Extension Node 26/26; `cargo fmt --all -- --check`; workspace `npm run check` (Vite 52 modules plus Extension syntax); Windows Tauri release executable build.
- One initial Desktop Rust run without `PYTHON` had 3 Sidecar-dependent batch tests fail because the worker did not start. Setting `PYTHON` to the repository's Python 3.12 interpreter yielded 111/111. The first restricted Desktop Node attempt also encountered denied child-process teardown; the normal-permission rerun passed 93/93. These setup failures are documented and are not retained as product failures.
- Current live WQ-WS-01/02/03/04: `BLOCKED` — `COMPUTER_USE_UNAVAILABLE`. Limited Computer Use retry found no native app target; the only Edge surface used an existing profile and was left untouched. Follow the controlled-profile steps in `windows-validation-history.md`.
- Current WQ-WS-05 Full package directory assembly, Extension ZIP plan/build/extract/inventory (`12/12`), and Bandizip `.7z` create/test/extract/hash comparison (`77/77`) are `PASS` at package input `fcde594`. Browser load remains `NOT RUN`; signing/release gate remains `NOT RUN`.
- Full workspace regression, account discovery, aria2/media transfer, staging/path/fault recovery, and unrelated database migration were not rerun because the current diff does not touch those components and the relevant prior evidence remains revision-scoped.
- No new `CROSS_PLATFORM_CHANGE_REQUIRED` or `CROSS_PLATFORM_REVIEW_REQUIRED` finding was established by current automated Windows validation. The original live WebSocket no-response report remains historical and unresolved by GUI retest; ownership stays with Windows until the blocked queue can be exercised or a new shared issue is demonstrated.

### 2026-09-25 package preparation for manual WebSocket verification

- Package source revision: `fcde5943af2f6ad15c833fa5ab88d6b1758345c6`. The only changes after application-source validation input `98f16845b9e37f0dea419e7bffc890b1be3d2063` are documentation, so the previously built Windows executable contains identical application code.
- Full portable package directory: `validation-artifacts/windows-ws-fcde594/full-package`.
- Extension ZIP: `validation-artifacts/windows-ws-fcde594/XArchive-v0.0.0-pre.1-extension.zip`.
- The package uses local validation tag `v0.0.0-pre.1`; this is not a release artifact. The portable directory is `validation-artifacts/windows-ws-fcde594/full-package`; a Bandizip CLI 8.0 (Beta, x64) 7z archive is `validation-artifacts/windows-ws-fcde594/XArchive-v0.0.0-pre.1-windows-x64-full.7z`. Bandizip `t` reported `All OK`; extraction into an isolated directory matched all 77 source files by relative path and SHA-256 (missing 0, extra 0, hash differences 0). Browser load, GUI startup and live WebSocket pairing remain `NOT RUN`/`BLOCKED` as separately recorded.
- User-provided aria2 source directory: `E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2`. The executable is `E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2\1.37.0\aria2-1.37.0-win-64bit-build1\aria2c.exe` (version 1.37.0; SHA-256 `BE2099C214F63A3CB4954B09A0BECD6E2E34660B886D4C898D260FEBFE9D70C2`). A copy was placed in this validation Full package at `full-package\sidecar\aria2\aria2c.exe` before creating the `.7z`; the package manifest/build source was not changed. This only enables package detection/manual testing and does not prove app-managed download success.
- Full package checks: `package_type=full`, `platform=windows-x64`, desktop executable, Extension source folder, Native Host executable/manifest, worker, Full gallery-dl payload and the locally staged aria2 executable present; `package-manifest.json` Extension ID and Native Host origin agree with the Extension manifest. Signing/publishing was not attempted.
- Recreate the local Full `.7z` from PowerShell (after the Full directory and optional local aria2 copy are prepared):
  ```powershell
  $package = 'E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\windows-ws-fcde594\full-package'
  $archive = 'E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\windows-ws-fcde594\XArchive-v0.0.0-pre.1-windows-x64-full.7z'
  Set-Location -LiteralPath $package
  & 'C:\Program Files\Bandizip\bz.exe' c -fmt:7z -l:9 -r $archive '*'
  & 'C:\Program Files\Bandizip\bz.exe' t $archive
  ```
  The command above creates a local validation archive; it does not sign or publish a release.
- Extension ZIP: planned from repository `extension/`, staged only the declared 12 package files, compressed with Windows `Compress-Archive`, expanded to an isolated verification directory and checked using `desktop/scripts/extension-package.mjs verify`; `12/12` match. Repository-derived Extension ID: `iaajefkoanbkleojofoadeakelihbjne`.
- SHA-256: Desktop executable `AF07ED237538125E0ECA1689AE2D6295479CD50334318F76F8D421E4F888F074`; Native Host `76DC613744D83B65A3478FB6D72B534E7F72CDF983D4D83E5BE78C3279BCE153`; gallery-dl `0B36AE6734ED41E12BE6BE1B33D3165A450B3E0A811FC1B8C664C032F7F13B2C`; Extension ZIP `32FDCB29AA847CFF4DDE48A1C0BE4FB10F44C92549782A1B87436BADD30A4FAB`.

#### Detailed manual steps

1. For this Sidecar retest, use the corrected package at `E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\windows-ws-fcde594\full-package-sidecar-fix` or extract `E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\windows-ws-fcde594\XArchive-v0.0.0-pre.1-windows-x64-full-sidecar-fix.7z` into a new test folder with Bandizip. The original `full-package` / `XArchive-v0.0.0-pre.1-windows-x64-full.7z` are retained only to reproduce the stale-worker failure. Confirm `xarchive-desktop.exe`, `package-manifest.json`, `extension`, `native-host`, and `sidecar` exist at the selected root. Prefer the assembled corrected directory for the first run because the bundled Native Host manifest contains its executable path; if using the separately extracted archive and testing Native Messaging, repair/re-register Native Host from that location. Do not move a package after registering Native Host.
2. Start `xarchive-desktop.exe`. If the first-use “选择下载目录” panel appears, choose either “创建便携目录” or “使用系统下载目录” and wait for setup to finish. On the Dashboard, click “启动 Sidecar” and wait until the status reads “Sidecar 正在运行” / `ready` (the app performs the v2 `hello → ready` handshake). Do not submit an archive task while it still reads “Sidecar 未启动”. If start fails, save the complete red error text and check `<package root>\logs\xarchive-*.log` and whether `xarchive-downloader.exe` remains running; omit literal token values when transcribing evidence. Then open Settings → Extension and confirm WebSocket is enabled on port `17321`; use the app's copy action for the current token.
3. In Edge, visit `edge://extensions`, turn on Developer mode, and load the unpacked Extension directory at `...\full-package-sidecar-fix\extension`. If testing the ZIP instead, extract `XArchive-v0.0.0-pre.1-extension.zip` to a separate local directory first, then use “Load unpacked” on that extracted folder. Test one installation at a time because both copies share one Extension ID.
4. Open Extension details and confirm it is enabled with no manifest/load errors. Open its Options page and Popup. Keep DevTools available for the Extension Service Worker and inspect console/Network → WS only if needed; redact the token before saving any evidence.
5. In Options, enter port `17321` and first use an intentionally incorrect test token. Expected: explicit authentication failure; Desktop must not count auth success or route a business request. Replace it with the copied current token and reconnect. Expected: connected/authenticated state and Desktop diagnostic counters advance. If the state remains disconnected or reaches `auth_timeout` after about 5 seconds from socket open, record exact status/error and counters without recording either token.
6. Exercise a `query_status` request only if the current UI/test fixture provides a controlled way to send one. Confirm the response carries the same `request_id`. If no such control is available, record that substep `NOT RUN`; do not treat the visible Extension button or a task row as proof of WebSocket response.
7. With a test-owned request pending, stop/restart Desktop and restart/reload the Extension Service Worker. Confirm pending work ends clearly, reconnection is bounded, the new session authenticates, no request is duplicated/replayed, and Native Messaging fallback status is accurate. Do not use an account or post with identifying data.
8. Check Options and Popup at 100%, 125%, and 150% zoom. Use Tab, Enter, Space and Escape; verify visible focus, no clipped/overlapping content, long error text, connecting/connected/auth-failed/timeout states, and settings-save failure behavior.
9. If verifying aria2 detection, start the app from the extracted Full package, open Settings → aria2 and refresh detection. Expected path: `<extracted Full package>\sidecar\aria2\aria2c.exe`; confirm the UI detects that bundled executable and reports version 1.37.0. The original user-provided source directory is `E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2`; its executable is nested at `1.37.0\aria2-1.37.0-win-64bit-build1\aria2c.exe`. If the package copy is missing after extraction, select that original executable manually and record the changed setup. To test downloading, use only a controlled test item; task creation or aria2 detection alone is not a download PASS. Confirm the app-managed transfer, completed file and content/hash.
10. Record each item separately as `PASS`, `FAIL`, `BLOCKED`, or `NOT RUN`, together with source revision `fcde594`, package label `full-package-sidecar-fix`, browser version and redacted evidence. The corrected archive is a local validation artifact, not a signed or published release; signing/release gate remains out of scope.

### 2026-09-25 Full package Sidecar startup follow-up

- User-reported observation from the supplied Full package: Dashboard showed `Sidecar 未启动` and a red `Sidecar 操作失败：sidecar is not running` message while the first-run download-folder setup panel remained visible.
- **Confirmed cause in the supplied package:** Desktop starts the worker with `--timeout-seconds 300 --discovery-timeout-seconds 600`, but the packaged frozen `xarchive-downloader.exe --help` did not include these options. Replaying Desktop's exact command exited with code 2 and stderr `error: unrecognized arguments: --timeout-seconds 300 --discovery-timeout-seconds 600`. The worker therefore exited before `hello → ready`, which explains the supervisor/UI `NotRunning` result. This is a stale packaged worker artifact, not evidence of a shared protocol/contract defect.
- **Corrected package checks:** rebuilt the worker from current source with PyInstaller 6.22.3 / Python 3.12.14. Its help includes both options; a direct v2 JSONL probe using the exact Desktop arguments and packaged `gallery-dl.exe` returned `ready` with required capabilities and exited 0 after `shutdown`. The resulting worker SHA-256 is `B51566894CAA6C20AF5B81F2D93D1B8DFAC1B3EC3F24F6EBC8FF8B2222F1318A`.
- Assembled `validation-artifacts/windows-ws-fcde594/full-package-sidecar-fix` using that worker and copied the supplied local aria2 executable into `sidecar/aria2/aria2c.exe`. Bandizip produced `validation-artifacts/windows-ws-fcde594/XArchive-v0.0.0-pre.1-windows-x64-full-sidecar-fix.7z` (34,877,156 bytes; SHA-256 `1DB74B9DEBF5CB6E72A098D871EE20C70582D28E8E4B495E0B1AEF16BC9552C5`). Bandizip archive test passed; extraction comparison found 77/77 files identical by SHA-256.
- Status: supplied Full package Sidecar launch `FAIL` (root cause confirmed); fresh worker exact-argument handshake `PASS`; corrected archive test and extraction/hash comparison `PASS`; corrected package Dashboard Sidecar-running state `PASS` (new user screenshot). The screenshot does not establish a successful archive task or download.
- Remaining manual retest: submit one controlled task only after Sidecar reads `ready`. If it fails, capture the full alert and newest `<package root>\\logs\\xarchive-*.log`, plus `Get-Process xarchive-downloader -ErrorAction SilentlyContinue | Select-Object Id,ProcessName,Path`; redact credentials, tokens, and private URLs.

### 2026-09-25 Full package Extension WebSocket follow-up

- User screenshot confirms the Extension Options page loaded from `full-package-sidecar-fix\extension`; it shows WebSocket enabled on port `17321` but still `已断开`. Desktop shows Sidecar connected, Extension files ready and Native Host registered, but reports no browser connection (`not_loaded`) and shows every WebSocket diagnostic counter at zero.
- Edge DevTools Network lists six `127.0.0.1` WebSocket entries, each HTTP `101`, with displayed resource size `0 B` and short durations (2–52 ms). A `101` confirms HTTP Upgrade only. The screenshot does not show the selected connection's Messages frames, so it does not establish whether `authenticate` was sent, received, or answered. The Desktop counters may be a stale UI snapshot until its refresh control is used.
- Read-only local process inspection during this investigation found `xarchive-desktop.exe` running from `validation-artifacts/windows-ws-fcde594/full-package-sidecar-fix` and listening on `127.0.0.1:17321`; TCP `TIME_WAIT` entries were also present. This is point-in-time evidence and does not correlate each DevTools row with the same process/session.
- Status: Extension page load/display `PASS` (user screenshot); WebSocket upgrade attempts `PASS` (six user-supplied 101 entries); authenticated connection and request response `FAIL` (user-reported disconnected/no response); protocol auth result and exact failure component `NOT RUN` / unresolved. If Desktop counters are refreshed after a new 101 attempt and `accepted` remains 0, that conflicts with this process's accept path and should be investigated as runtime/session mismatch or stale diagnostics before attributing token failure.
- Next manual capture, with no token in the screenshot: (1) click the Desktop Extension panel refresh; (2) initiate one reconnect; (3) select that newest `127.0.0.1:17321` entry in DevTools and open Messages; (4) report only whether an outbound `authenticate` and inbound `authentication_response` appear, plus Desktop counters before/after. If a response exists, note only `authenticated: true/false` and `error_code`, never `token`.
- User clarification (2026-09-25): the pairing token is for local validation only and resets on each Desktop restart. Per the user's request, token privacy/security review is skipped (`NOT APPLICABLE`); no compromise finding or rotation action is tracked. The token value remains omitted from this validation record.

### 2026-09-25 WebSocket accepted but closed before authentication

- New user screenshot from the corrected Full package shows the Extension diagnostics after refresh: `accepted=100`, `auth_received=0`, `auth_succeeded=0`, `auth_failed=0`, `auth_response_failed=0`, `close_before_auth=99`, `close_after_auth=0`. The Extension-side popup still says WebSocket disconnected and Native Messaging fallback; Edge's extension error page reports `net::ERR_SOCKET_NOT_CONNECTED` at the `new WebSocket(...)` call in `websocket-bridge.js:55`.
- For this aggregate snapshot only, the Desktop listener accepted the WebSocket connections, but none of the 100 had incremented the server's authentication-message receive counter; 99 closed before authentication. A wrong token would be evaluated only after `auth_received` increments and would increase `auth_failed`. HTTP 101 still proves only the Upgrade handshake. This snapshot is not correlated to the later selected request that visibly sent an auth frame.
- Root cause remains unresolved between Extension WebSocket event/send lifecycle and connection teardown/timing. This snapshot alone does not attribute failure to token contents or prove the auth-response path failed. It is point-in-time aggregate evidence and is superseded for the selected later request by the DevTools frame observation below.
- New user screenshot (2026-09-25) shows a selected Edge DevTools WebSocket request with an outbound `authenticate` frame. The frame payload includes a token, but its value is intentionally not copied into this record. No inbound `authentication_response` is shown; the Extension Options page still reads disconnected and the console reports `ERR_SOCKET_NOT_CONNECTED` at `websocket-bridge.js:55`.
- This outbound frame is evidence that the Extension sent an authentication message for the selected connection. The earlier Desktop counter snapshot (`accepted=100`, `auth_received=0`, `close_before_auth=99`) was not captured in a correlated single attempt and must not be used to claim that Desktop failed to receive this particular frame. Server receipt and response for this selected request remain unknown.
- Token privacy/security review: `NOT APPLICABLE` per the user's instruction; the token is described by the user as local-validation-only and reset on Desktop restart. No rotation or compromise action is part of this queue. Keep the literal value out of committed records.
- Next minimal capture: make one reconnect and correlate the same request across DevTools Messages and the Desktop diagnostic counters immediately before/after it. Record only whether outbound `authenticate` and inbound `authentication_response` appear, the boolean `authenticated` result or `error_code`, and counter deltas. Do not infer server receipt from the 101 upgrade alone.

### 2026-09-27 current-package Sidecar and Codex Profile follow-up

| ID | 类别 | 验证项目 | 关联修改/目标 | Windows 原因 | 前置条件 | 精确行为 | 预期结果 | 优先级 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-WS-06 | Runtime/Packaging | Corrected Full package Dashboard Sidecar startup | Windows input `6d60429`; corrected package `validation-artifacts/windows-batch-revalidation-6d60429-sidecar-current`; worker `B51566894CAA6C20AF5B81F2D93D1B8DFAC1B3EC3F24F6EBC8FF8B2222F1318A` | Desktop app window is not exposed to Computer Use; first-run folder requires user choice | Close the old stale-package Desktop process; launch corrected package; choose a user-approved archive directory | Click Start Sidecar; confirm runtime state/log and process path refer to corrected package | Dashboard reads Sidecar ready without error; no archive task required | P1 | no | `PASS` (user screenshot shows Dashboard Sidecar `hello -> ready`; process paths point into corrected package). User's separate CLI probe failed because `Start-Process -ArgumentList` split the gallery path; a quoted-argument rerun passed locally. |
| WQ-WS-07 | Browser/Integration | Verify selected Codex Edge Profile and XArchive Extension channel | Corrected Full package Extension; Desktop port `17321` | Profile identity is not visible in supplied captures; pairing requires the Desktop's current process token | Corrected Desktop Sidecar is ready; Extension is loaded in the user-selected profile | Correlate one authentication attempt with Desktop counters and DevTools Messages; then pair again with current Desktop token | `authentication_response.authenticated=true`; Extension and Desktop show connected; record no token or page/account content | P1 | no | `PASS` (limited, user-reported: Options shows authenticated and archive tasks were submitted; Profile identity remains unverified. Archive execution failure is WQ-WS-08.) |

| WQ-WS-08 | Runtime/Filesystem | Revalidate archive execution with Windows-safe staging IDs | Windows implementation `7eacb82`; isolated package `validation-artifacts/windows-batch-os123/full-package` | ISO timestamp in the opaque job ID contains `:`, which Windows rejects in a directory name | Corrected package and compatible Sidecar | Submit one controlled archive task; inspect state and output | No `os error 123`; staging creation, recovery lookup, and commit use the same mapped component | P1 | no | `PASS (limited, user-reported: tasks entered “Downloading” and no longer report `os error 123`; complete archive commit/output remains unverified)` |
| WQ-WS-09 | Runtime/Executor | Revalidate burst submissions after shared runner queue/backpressure fix | Linux follow-up revision to be handed back; Windows package `validation-artifacts/windows-batch-os123/full-package` is pre-fix | Desktop schedules each job concurrently while the runner has one active slot and a one-entry `try_send` queue | Linux fix and regression tests committed; exact revision built into a Windows package | Submit at least three controlled archive requests in quick succession; observe durable states and eventual completion/failure; check task history after refresh and Desktop restart | Every accepted task remains queued or active until it reaches a terminal state; no accepted task is converted to `EXECUTOR_WORKER_FAILED` solely because the runner is temporarily full | P1 | yes | `CROSS_PLATFORM_CHANGE_REQUIRED` (pre-fix user report: two rows displayed “Downloading” >30s; later requests failed with `job executor command queue is full`. Thirty seconds is below the configured 300-second extraction timeout.) |
| WQ-WS-10 | Runtime/External integration | Investigate gallery-dl extraction timeouts and verify one controlled archive completes | Corrected Full package `validation-artifacts/windows-batch-revalidation-6d60429-sidecar-current`; current package config has extraction 300s / discovery 600s / transfer 1800s | User's four-job attempt ended with two `DOWNLOAD_TIMEOUT: gallery-dl timed out` and two shared runner-queue failures; timeout cause is not yet localized to Windows, network/service, or content/authentication | Linux queue fix returned and packaged; Sidecar running; use a controlled known-good public/synthetic fixture where supported, then one user-authorized test URL if needed | Run one job at a time; capture start/end elapsed time, terminal error code, and whether expected archive files exist (hash files without recording post/account content); inspect sanitized Sidecar logs for gallery-dl exit/timeout context | Controlled fixture and eligible test URL reach a terminal success within configured extraction/transfer budgets and output is committed; otherwise record the exact sanitized failure boundary | P1 | no | `BLOCKED` (latest user attempt `FAIL`: two extraction timeouts; no controlled fixture or sanitized worker diagnostics yet. Configuration/source confirms the 300s extraction deadline but not the underlying cause.) |

The 2026-09-27 user screenshots also confirm the packaged Extension Options and Popup render and the Desktop receives an authentication frame. This is partial `PASS` evidence for WQ-WS-01, not a completed permission/service-worker/Profile identity check; the auth result is tracked under WQ-WS-02/07.

Latest user confirmation and current read-only Edge inspection: X Home loads and the accessibility tree contains injected `XArchive：保存` buttons on timeline items (`PASS`, page access and content-script rendering). Browser inventory lists multiple tabs and exposes no Profile name. The screenshot/DOM does not identify the exact package revision and does not change WQ-WS-02 authentication, WQ-WS-03 lifecycle, archive completion, or WQ-WS-09/10 status. Native app inventory returned `apps: []` twice; do not infer current Desktop GUI state from the browser check.

The stale worker failure and corrected-package probe evidence are recorded in `windows-validation-history.md` under the 2026-09-27 follow-up. Do not treat the CLI handshake as Dashboard/Extension acceptance.
### Windows phase results for 2026-09-27 handoff (`7b218f8a5ff4a10590cea3cf762fd30a82d6c6c9`)

The Windows phase has run against the exact handoff revision. Detailed commands, toolchain, failure evidence, and the Manual Windows Validation Queue are in [`../development/windows-validation.md`](../development/windows-validation.md), section “Windows validation phase for 2026-09-27 handoff”.

| Queue item | Result | Reconcile / remaining work |
|---|---|---|
| WQ-ENG-01 | `PASS` safe guard unit + Full assembly; direct destructive root integration `BLOCKED` by automatic safety review | Retry only in an isolated disposable target. |
| WQ-ENG-02, WQ-ENG-10 (local toolchain), WQ-ENG-11 | `PASS` | `cargo` check/test/Clippy and release build passed; remote workflow dispatch remains NOT RUN. |
| WQ-ENG-03 | `FAIL` | `CROSS_PLATFORM_CHANGE_REQUIRED`: metadata validator accepted an outside file through a Windows junction. |
| WQ-ENG-04 | `BLOCKED` | Windows Named Pipe server is not implemented (transport currently Unix-gated). |
| WQ-ENG-05 | `PARTIAL PASS` | Worker checks and real aria2 Unicode/space-path download passed; PowerShell archive extraction remains in Manual Windows Validation Queue. |
| WQ-ENG-06 | `BLOCKED` | Dedicated non-personal external service accounts are unavailable; no personal credentials used. |
| WQ-ENG-07 | `PARTIAL PASS` | Local Full package/manifest passed; release tag mismatch and CI sentinel checks NOT RUN. |
| WQ-ENG-08 | `FAIL` | Shared `killTree` process/test lifecycle assertion failed; `CROSS_PLATFORM_REVIEW_REQUIRED`. |
| WQ-ENG-09 | `FAIL` | `npm ls` contract invalid after clean install and both native E2E runs remained on `data:,`; `CROSS_PLATFORM_CHANGE_REQUIRED`. |
| WQ-ENG-10 (Actions) | `NOT RUN` | CI workflow Action pin/artifact checks need an authorized CI run. |
| WQ-ENG-12 | `PASS` | Real aria2/secret-file/process-list/local-download probe passed; actual inherited temp ACL entries are recorded in the validation report. |
| GUI/WebView2/accessibility | `BLOCKED` | Computer Use native-app inventory empty; see manual queue. |

**Cross-platform reconcile required before ownership can return to Windows.** Apply the shared fixes, record their implementation revision, and hand off only the corresponding failed/blocked items for focused Windows revalidation; valid independent PASS results above remain reusable.

Cross-platform reconcile round 1（2026-09-28，状态确认与根因定位，无实现改动）已写入 [`../status/platform-handoff.md`](../status/platform-handoff.md)；本表各项状态维持不变。

### Cross-platform reconcile round 2（2026-09-28，实现轮）

Implementation revision：`01067b66a3aa214fb90f7d893c57bb971a7c7882`（branch `security/tweet-url-host-validation`）。上表 `FAIL` 项对应的共享修复已在 Linux 完成实现与验证（根因、证据与 Linux 验证明细见 [`../status/platform-handoff.md`](../status/platform-handoff.md) 的 2026-09-28 第 2 轮 section；证据勘误见 [`../development/windows-validation.md`](../development/windows-validation.md) 的 “Cross-platform reconcile round 2 corrections”）。下表记录本轮 2026-09-28 Windows focused revalidation 结果；未命中项继续复用有效历史 PASS 或保留其原状态。

| ID | 关联修改（`01067b6`） | Windows 原因 | 精确步骤 | 预期结果 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|
| WQ-ENG-03 focused | `crates/xarchive-storage/src/metadata.rs` 改用 `FileStore::resolve_within`（提升为 `pub(crate)`）；`lib.rs` 新增 `rejects_sidecar_intermediate_symlink_escape` 测试 | 真实 junction/reparse 行为只能在 Windows 确认 | `cargo test -p xarchive-storage`；在归档 staging 下创建指向根外的中间 junction，执行 `build_archive_metadata` | storage 27/27；真实 junction 返回 `InvalidPath`，根外文件不变 | no | `WINDOWS_PASS` |
| WQ-ENG-08 focused | `desktop/scripts/wdio-tauri-service.mjs` killTree win32 分支增加 taskkill stderr/exit-code 诊断（布尔契约与断言不变） | 2026-09-27 失败仅在 Windows 出现，根因需目标环境诊断 | npm 11.19 下执行 `npm test --workspace desktop`；观察诊断并确认测试退出/无残留 | 42/42；沙箱内首次 taskkill Access denied，精确清理后 elevated rerun 通过 | no | `WINDOWS_PASS` |
| WQ-ENG-09a focused | 根 `package.json` `engines.npm >=11.18.0`、`package-lock.json` 根条目同步、`docs/development/setup.md` 版本底线 | clean install 契约须在目标环境复验 | 隔离 cleanroom：npm 11.19 `npm ci` + `npm ls --all`；npm 11.17 `npm ci` + `npm ls --all` | npm 11.19 两项 exit 0；npm 11.17 显示 `EBADENGINE`，并按预期保留 `ELSPROBLEMS` | no | `WINDOWS_PASS` |
| WQ-ENG-09b focused | `desktop/e2e/test-artifacts/wdio/startup/evidence.json` untrack、`desktop/e2e/test-artifacts/` 加入 `.gitignore`（无生产者恢复；`01067b6`） | Tauri/WebView2 会话启动与窗口枚举只能在 Windows triage | WDIO 专用 build；ordinary + advanced E2E；EdgeDriver 155；临时诊断 spec 枚举 handles/URL/title 并截图 | session 建立；fresh observation 为一个 window、`data:,`、空 title、白屏；ordinary 1 fail，advanced 2 fail；根因未定 | no | `WINDOWS_FAIL` |

本轮没有 `WINDOWS_VERIFICATION_BLOCKING`。`WQ-ENG-09b` 的 app-level root cause 未定，Computer Use 未能列出任何 native app/window；保持 GUI observation `BLOCKED`，而 E2E acceptance 为 `WINDOWS_FAIL`。

#### 2026-09-28 focused results and Manual Windows Validation Queue

- WQ-ENG-03: `WINDOWS_PASS`；storage 27/27，真实 junction harness 1/1。
- WQ-ENG-08: `WINDOWS_PASS`；Desktop Node 42/42；测试开始时版本差/权限前置均已隔离处理。
- WQ-ENG-09a: `WINDOWS_PASS` on npm 11.19; npm 11.17 emitted the expected engine guard and reproduced the unsupported-version invalid tree.
- WQ-ENG-09b: `WINDOWS_FAIL`; matching EdgeDriver 155 established a session but page stayed `data:,`. Evidence: session `9dcfe7619c30e685d30c8c4d836e4a61`, one handle `D271E7BD7DD7761C8E0718CA6DE2BB2B`, empty title, screenshot `validation-artifacts\windows-batch-20260928-01067b6\startup-evidence\window-1.png`. New `desktop\logs\wdio-*.log` files were 0 bytes. The tool inventory had no native apps (`listApps` unsupported; `listWindows` unavailable), so GUI observation is `BLOCKED`.
- Manual follow-up: open `target\release\xarchive-desktop.exe` in an observable Windows session, compare standalone launch with WebDriver, capture app/WebView2 logs plus screenshot, and repeat ordinary/advanced E2E only after the blank startup cause is isolated. Keep WQ-P1-12 path/reparse/long JSON/Unicode/real-download fixtures in the manual queue.

Cross-platform Owner should triage WQ-ENG-09b from this evidence before Windows ownership resumes; no code change was inferred from the blank screen alone.

### Cross-platform reconcile round 3（2026-09-28，WQ-ENG-09b triage）

Triage 由 Cross-platform Owner 基于 `5170161` 的 fresh 证据完成（结论全文见 [`../status/platform-handoff.md`](../status/platform-handoff.md) 的 round-3 section）。已排除的假设：应用导航路径自 2026-09-16 原生 WDIO PASS（`3f70894`）以来无任何变更——`desktop/src-tauri/tauri.conf.json`、`desktop/index.html`、`desktop/vite.config.js`、CSP 与 tauri 依赖版本一致，diff 仅为新增 command、dialog 插件注册与 capability 权限；构建链 `beforeBuildCommand: npm run build` + `frontendDist: ../dist` 完整，`desktop/dist/index.html` 存在。

round-3 提出的最可能类别为**测试工具链：msedgedriver 与 WebView2 runtime 版本不匹配**。该假设已由 Windows batch 2 部分证实并进一步收敛：WebView2 runtime 实为 `153.0.4234.48`（不是探测到的 155），但**即使把 driver 钉到正确的 153，session 仍无法创建**，因此版本不匹配只是表层症状；真正根因是上游 `tauri-driver` 的能力协商缺陷（见下方 batch-2 section）。应用本身在独立启动与直连 msedgedriver 两条路径下均正常渲染。

| ID | 关联修改 | Windows 原因 | 精确步骤 | 预期结果 | 阻塞 Linux | 状态 |
|---|---|---|---|---|---|---|
| WQ-ENG-09b revalidation | `desktop/wdio.conf.mjs`（`TAURI_DRIVER_EDGE_VERSION`/`EDGEDRIVER_VERSION` → `edgeDriverVersion`）、`desktop/e2e/support/startup-diagnostics.mjs`、两个 E2E spec 的 readiness 钩子 | WebView2 runtime 版本、driver 缓存与窗口附着只能在 Windows 观察 | ① 记录**应用实际加载的 WebView2 runtime 版本**（不是 Edge 浏览器版本）与 tauri-driver 实际启动的 msedgedriver 版本，二者必须一致；② 清理 `%TEMP%\msedgedriver\*` 缓存并确认 4444/45446+ 无遗留 tauri-driver/msedgedriver 后，以 `TAURI_DRIVER_EDGE_VERSION=<WebView2 runtime major>` 重跑 ordinary 与 advanced E2E；③ 提交 `WDIO_LOG_DIR\startup\startup-diagnostics-*.json` 与 `window-*.png`（本轮起由套件在 readiness 失败时自动生成） | **已执行（Windows batch 2）。driver 钉版不足以解除阻塞**：WebView2 runtime 实为 `153.0.4234.48`（非 155），钉版后仍 `session not created`，应用从未启动。根因在上游 `tauri-driver`，见下方 batch-2 section | no | `BLOCKED_AUTOMATION`（上游缺陷） |
| WQ-ENG-09b standalone comparison（人工） | 无代码变更 | GUI 观察不可由 WebDriver 代替；Computer Use 仍 `BLOCKED` | 在可观察的 Windows 桌面会话中直接打开 `target\release\xarchive-desktop.exe`，记录窗口是否渲染 Dashboard、截图与 `logs\` 应用日志，并与 WebDriver 会话对照 | **已执行（自动化等价）**：进程存活，Win32 `EnumWindows` 得到可见顶层窗口 `TITLE='XArchive'`（`RECT=147,5,1240,801`），应用日志写入 `application runtime initialized`；独立启动**渲染正常**，故不升级为产品启动路径问题 | no | `WINDOWS_PASS` |

本轮无 `WINDOWS_VERIFICATION_BLOCKING`；`WQ-ENG-09b` 在 revalidation 之前保持 `WINDOWS_FAIL`，`WQ-ENG-03`/`WQ-ENG-08`/`WQ-ENG-09a` 的 `WINDOWS_PASS` 继续有效。

### Windows 专属验证项目总览（2026-09-28 收口汇总）

本节是本文件所有仍开放 Windows 专属项的**统一汇总入口**，不替代上文各 phase 的详细证据与逐项步骤表；状态取自各表“状态”列与最近 phase（2026-09-28 focused / round-3）结论。

#### A. 本轮已关闭（可复用，无需重复执行）

| ID | 结论 | 备注 |
|---|---|---|
| WQ-ENG-02 / WQ-ENG-11 | `WINDOWS_PASS` | Windows workspace `cargo check/test/clippy` 与 release build 通过 |
| WQ-ENG-03 | `WINDOWS_PASS` | 真实 junction harness 1/1，根外文件未被改写；metadata containment 修复生效 |
| WQ-ENG-08 | `WINDOWS_PASS` | 42/42；原失败归因验证沙箱 `taskkill` 权限，非产品缺陷 |
| WQ-ENG-09a | `WINDOWS_PASS` | npm 11.19 cleanroom `npm ci`+`npm ls` 干净；11.17 按预期 `EBADENGINE` |
| WQ-ENG-12 | `WINDOWS_PASS` | 真实 aria2 接受 `--conf-path`，secret 不进 argv，退出后删除 |
| WQ-ENG-05 | `PARTIAL PASS` | worker 协议与 aria2 Unicode/空格路径下载通过；PowerShell 解压仍属手工队列 |
| WQ-ENG-10（本地工具链部分） | `WINDOWS_PASS` | `rustc` 与 pin 一致、workspace 构建与测试通过；Actions dispatch 部分见 C 组 |

#### B. 仍为 `BLOCKED`（本轮跳过，按下节手工程序执行）

| ID | 项目 | 阻塞原因（不可由 Linux 解除） |
|---|---|---|
| WQ-ENG-01 | 打包输出目录删除保护的破坏性集成用例 | 自动安全审查拒绝以真实仓库根为目标；需一次性隔离副本 |
| WQ-ENG-04 | Windows Named Pipe server/ACL/生命周期 | 实现尚未存在（`transport.rs` 目前 Unix-gated），属 Windows 平台适配 |
| WQ-ENG-06 | 真实账号错误脱敏与凭据边界 | 缺少专用非个人 X/gallery-dl 与 Telegram 测试账号 |
| WQ-WORKER-BUILD-01 | Windows worker one-dir artifact 与 `--help` | 现有 artifact 缺 `_internal\python312.dll`，需重新生成 |
| WQ-PACKAGE-FULL-01 / WQ-PACKAGE-CORE-02 | Full/Core portable 组装与运行 | 依赖有效 worker artifact；Full 另需受控 gallery-dl |
| WQ-WEBVIEW2-05 / A11Y-W-NAV-11 | WebView2 GUI、键盘/读屏/DPI/对比度 | `BLOCKED_AUTOMATION`：Computer Use 无 native app inventory；须人工桌面会话 |
| WQ-P1-02 / WQ-P1-03 / WQ-P1-04 / WQ-P1-05 / WQ-P1-13 | Native Host 安装/Registry/浏览器加载/Extension 实机 | `WINDOWS_FAIL`：功能或注册未在目标机实现/验证，属 Windows 专属实现 |

#### C. 仍为 `NOT RUN`（需授权环境或发布前置）

| ID | 项目 | 未执行原因 |
|---|---|---|
| WQ-ENG-07 | 发布 provenance：tag 不匹配拒绝、CI 哨兵排除、Actions/artifact 固定 | 需要授权的 CI dispatch 与发布证书 |
| WQ-ENG-10（Actions 部分） | workflow 在 CI runner 上按 SHA 解析并产出 artifact | 需授权的 CI run |
| WQ-RELEASE-06 | 安装器、签名、Updater、Tray、真实账号 | 当前 `bundle.active=false`，无证书/账号/外部服务 |

#### D. 阻塞于上游依赖（非本仓库可修复）

| ID | 项目 | 触发 / 阻塞原因 |
|---|---|---|
| WQ-ENG-09b ordinary / advanced E2E | Tauri native E2E 停在 `data:,` | 产品侧已 `WINDOWS_PASS`（应用渲染正常）；E2E 套件被上游 `tauri-driver` `map_capabilities()` 缺陷阻塞，见 batch-2 section |

#### 2026-09-28 Windows batch 2：WQ-ENG-09b 根因定位（triage 结论）

本节在 round-3 之后记录 Windows 侧实际执行结果。验证对象为 `517016146e29e08cae38c8e33cfdb85c4b7e082a`（其实现内容与 `01067b6` 相同），随后 rebase 到 `8ddb09a` 之上。完整证据链见 [`../development/windows-validation.md`](../development/windows-validation.md) 的 “WQ-ENG-09b startup triage — root cause isolated”。

结论：被记录为 “blank `data:,` WebView” 的窗口**不是 Tauri 应用，而是 msedgedriver 回退启动的 Edge 浏览器 `msedge.exe`（155）首屏**——应用从未被启动。根因是上游 `tauri-driver` `map_capabilities()` 只把 `ms:edgeOptions.binary` 写入 legacy `desiredCapabilities`，而 `webdriver` 9.31.9 只发 W3C `capabilities`；`tauri-driver` `3.0.0-alpha.1`（crates.io 最新）该逻辑逐字相同，未修复。

| ID | 关联修改 | Windows 原因 | 精确步骤 | 结果 | 状态 |
|---|---|---|---|---|---|
| WQ-ENG-09b 产品行为 triage | 无产品代码改动 | Tauri 前端在 Windows WebView2 的真实渲染只能在 Windows 确认 | 独立启动 exe + Win32 `EnumWindows`；再绕过 tauri-driver 直连 `msedgedriver --port=4445`（W3C `alwaysMatch` 内联 `browserName=webview2` + `ms:edgeOptions.binary` + `webviewOptions`），查 `/url`、`/title`、`h1`、`/screenshot` | 两条路径均渲染完整 dashboard：`url=http://tauri.localhost/`、`title=XArchive`、`h1=工作台`、真实截图 211 949 bytes | `WINDOWS_PASS`（产品侧结清） |
| WQ-ENG-09b ordinary / advanced E2E | `e856343` 的 `edgeDriverVersion` 透传（保留无害，不解决问题） | 同上 | 以 `EDGEDRIVER_VERSION=153.0.4234.46` 钉版重跑 `npm run test:e2e:windows`（isolated port 45460）+ manual tauri-driver probe | `session not created: ... only supports Microsoft Edge version 153 / Current browser version is 155 ... binary path msedge.exe`；`xarchive-desktop` 从未出现 | `BLOCKED_AUTOMATION`（上游 tauri-driver 契约缺陷） |
| WQ-ENG-03 / WQ-ENG-08 / WQ-ENG-09a | 无 | — | 本轮未重跑 | 当前 diff 与三者影响区无交集，沿用既有结论 | `WINDOWS_PASS`（复用） |
| **WQ-ENG-13（新增）** `startup-diagnostics` 测试平台假设 | `e856343` 新增 `desktop/e2e/support/startup-diagnostics.mjs`、`desktop/test/startup-diagnostics.test.mjs`、两个 E2E spec、`desktop/wdio.conf.mjs` | 该测试层此前从未在 Windows 执行；路径拼接的平台差异只能在 Windows 复现 | `npx --yes --package npm@11.19.0 -c "npm test --workspace desktop"` | **47 pass / 1 fail**：`startup-diagnostics.test.mjs:54` `resolveDiagnosticsDir` 断言失败（actual `E:\project\desktop\custom-logs\startup` vs expected `\project\desktop\custom-logs\startup`）。测试用 `path.join(path.sep, …)` 构造期望值，而实现用 `path.resolve` 会补当前盘符——**测试断言的平台假设缺陷，非实现缺陷** | `WINDOWS_FAIL`（`FAIL_TEST`）→ `CROSS_PLATFORM_CHANGE_REQUIRED` |
| Computer Use 原生 GUI | 无 | 原生窗口清单能力 | `listApps` / `listWindows` 不可用 | 以 Win32 `EnumWindows` + WebDriver 截图取得等效证据，但不等同 GUI 交互/焦点/DPI/辅助技术验收 | `BLOCKED` |

本轮 `WINDOWS_OWNED` implementation 变更为空：本仓库无对应可改代码，未修改业务代码、依赖或测试。剩余阻塞属 shared/cross-platform 测试基础设施议题；仅当决定在 `wdio.conf.mjs` 或 WDIO 依赖版本层面加 workaround 时，才构成 `CROSS_PLATFORM_REVIEW_REQUIRED` 的 shared implementation change。

#### E. 其余 `WINDOWS_VERIFICATION_PENDING`（步骤已在对应表内，不在此重复）

`WQ-P0-01`～`WQ-P0-04`、`WQ-P1-01`、`WQ-P1-12`、`WQ-P1-14`～`WQ-P1-21`、`WQ-P2-01`/`WQ-P2-02`、`GUI-W-*`、`GUI-W-SETTINGS-01`～`GUI-W-EXT-10`、`EXT-W-*`、`SIDECAR-W-DOWNLOAD-05`、`WQ-GALLERY-CORE-03`、`WQ-EXT-CORE-04`、`WQ-EXT-DOWNLOAD-05`、`WQ-REL-DB-01`/`SETTINGS-02`/`LOG-03`/`CONSOLE-04`。各表已含精确步骤、预期结果与优先级；执行时按 `cross-platform-validation.md` 的影响面分析只跑命中 diff 的条目。

#### 手工验证程序（BLOCKED / NOT RUN 项统一执行清单）

通用前置：Windows 正式 E: checkout 经 `git fetch` 对齐到本节对应 implementation revision；记录 Windows 版本、DPI、WebView2/Edge、Node/npm、Rust/Tauri、Python 版本；所有产物、日志、截图写入被 ignore 的 `validation-artifacts\`，不得修改 Windows 工作副本代码。**任一项 FAIL 都要记录关键错误与判定依据；未执行必须写明原因，不得记 PASS。**

**M1 — WQ-ENG-01 打包输出删除保护（破坏性，需隔离副本）**

- 前置：一次性可丢弃的仓库副本（不得指向真实工作副本）。
- 步骤：在副本中分别以 `PORTABLE_OUTPUT_DIR` 指向项目根、其父目录、用户目录、含 junction 的目录执行 `npm run build:portable:windows --workspace desktop`；再做一组正常输出目录的对照构建。
- 预期：所有越界输入在删除动作之前失败并给出明确原因；合法输出目录正常生成；副本中源文件完好。
- 判定：越界输入全部拒绝且无误删 → PASS；任一越界输入进入删除路径 → FAIL（安全缺陷，立即停止后续用例）。

**M2 — WQ-WORKER-BUILD-01 / WQ-PACKAGE-FULL-01 / WQ-PACKAGE-CORE-02 artifact 链路**

- 前置：重新触发 `Windows Sidecar Worker Artifact` workflow，产出 one-dir zip；Full 另需受控 `gallery-dl.exe`（记录版本与 SHA-256）。
- 步骤：① 解压 worker，检查 `_internal\python312.dll` 等依赖齐全；② 运行 `xarchive-downloader.exe --help` 并记录 exit code、文件清单与 SHA-256；③ 依次组装 `PORTABLE_PACKAGE_TYPE=full|core`；④ 检查 `package-manifest.json` 与目录边界（Core 不含 `sidecar\gallery-dl`）；⑤ 启动应用 8 秒，确认 `config\archive.sqlite3`、`logs` 创建且未预建 `download/`，随后精确关闭。
- 预期：worker `--help` 成功；Full 包含 gallery-dl/Extension，Core 不含；启动清理正常。
- 判定：`--help` exit 0 且 manifest/目录符合 → PASS；仍缺 `_internal` → 保持 `WINDOWS_BLOCKED` 并记录缺失文件清单。

**M3 — WQ-ENG-04 Named Pipe 与 Native Host 端到端（实现前置于验证）**

- 前置：Windows Named Pipe server 已实现（当前不存在，队列保持 `WINDOWS_BLOCKED`，本程序在被明确告知已实现前不执行）。
- 步骤：实现后启动 Desktop，确认 `\\.\pipe\xarchive-v1` 监听；用受控客户端验证合法请求转发、request_id 路由、多连接并发、Desktop 退出后重连、非法/超长/半帧消息拒绝、普通用户与管理员 ACL 差异；再用 Native Host 经 Edge 实际加载确认 framing 一致。
- 预期：合法请求正确转发，非法或越权请求明确失败，无串线或挂起；ACL 拒绝符合预期。
- 判定：全部子项通过 → PASS；任一子项失败 → FAIL 并记录 request_id、错误码与日志。

**M4 — WQ-ENG-06 真实账号错误脱敏（需专用测试账号）**

- 前置：专用非个人 X/gallery-dl 与 Telegram 测试账号；不得使用个人浏览器 profile 或真实凭据。
- 步骤：分别触发认证失效、限流、网络失败、真实 Tweet 归档与真实发送；检查日志页、任务错误、SQLite `last_error_message` 与剪贴板导出内容。
- 预期：错误中无 Cookie、Token、Authorization 头、凭据 URL；保留稳定错误码与可诊断信息。
- 判定：全部通过 → PASS；任一泄露 → FAIL（安全缺陷）。

**M5 — GUI / WebView2 / 可访问性（`BLOCKED_AUTOMATION` 的人工替代程序）**

- 前置：可观察的 Windows 桌面会话（自动化 inventory 不可用时必须走本程序）；`npm run build:portable:windows` 产出的包。
- 步骤：① 100%/125%/150% DPI 下检查图标居中、系统字体 fallback、路径溢出；② 仅用 Tab/Enter/Space 遍历侧栏服务状态与设置页目标区块，确认焦点环与无横向滚动；③ 打开运行日志，验证五档筛选、搜索、内部滚动、自动跟随、复制、打开目录；④ 触发 Sidecar/aria2 状态变化并全程录屏观察是否出现控制台窗口；⑤ 用 Narrator/NVDA 复核服务状态与错误提示；⑥ 制造错误（无网络、非法配置）确认错误展示可读。
- 预期：各项符合设计；无控制台闪现；焦点与对比度满足既定结论。
- 判定：全部满足 → 对应 GUI-W-*/A11Y-W-NAV-11/WQ-REL-* 记 PASS；自动化仍不可用时保持 `BLOCKED_AUTOMATION`，人工结论单独记录，不得据此把自动化项记 PASS。

**M6 — WQ-ENG-07 / WQ-ENG-10 CI 与发布 provenance（需授权 CI）**

- 前置：具备 workflow dispatch 权限的账号与测试发布证书。
- 步骤：① 用与 checkout ref 不一致的 tag 触发发布 workflow，确认被拒绝；② 在组件目录放置哨兵 `.env`/测试文件后生成 Full/Core 包，确认哨兵未进入产物并记录文件清单；③ 在 CI runner 上确认 5 个 Action 按 commit SHA 解析、无浮动 tag 告警，worker artifact 正常生成。
- 预期：不一致发布被拒绝；包内容符合允许列表；Action 无版本漂移。
- 判定：三项通过 → PASS；无授权环境 → 保持 `NOT RUN` 并写明原因。

**M7 — WQ-RELEASE-06 安装器/签名/Updater/Tray（发布前置未满足）**

- 前置：`bundle.active=true`、正式图标集、代码签名证书、Updater 签名密钥与测试账号。
- 步骤：生成安装器并验证安装/升级/回滚/卸载；校验签名链与 Publisher；配置并验证 Updater 检查与安装；验证 Tray 与 Single Instance；全程记录无控制台行为。
- 预期：全部功能可用且签名有效。
- 判定：前置齐备且全部通过 → PASS；否则保持 `NOT RUN`（当前 `bundle.active=false`，不得记 PASS）。

### Cross-platform batch 3（2026-09-28，WQ-ENG-13 修复 + workaround 评审）

本节由 Cross-platform Owner 写入，处理 Windows batch 2 交回的两项 reconcile 工作。源码起点 `fa52356`（与 origin 同步）；本轮变更仅 1 个测试文件 + 4 个文档，**无产品代码、依赖或配置改动**。

#### 1. WQ-ENG-13 已修复（`CROSS_PLATFORM_CHANGE_REQUIRED` 关闭，交回 Windows 确认）

| 项 | 内容 |
|---|---|
| 根因 | `desktop/test/startup-diagnostics.test.mjs` 用 `path.join(path.sep, "project", "desktop")` 构造期望值；POSIX 得 `/project/desktop`，Windows 得 `\project\desktop`（无盘符）。被测实现 `resolveDiagnosticsDir` 用 `path.resolve`，会把无盘符 root 解析为当前盘符（`E:`），故 Windows 断言失败。**这是测试的平台假设缺陷，`path.resolve` 的盘符语义是正确行为，因此只改测试、不改实现。** |
| 修复 | 期望值改用 `fs.mkdtempSync` 生成的**绝对** root（复用文件内既有 `tempRoot()` 辅助函数），并用 `try/finally` 清理；`WDIO_LOG_DIR` 与默认两个分支的断言语义不变 |
| Linux 验证 | `node --check test/startup-diagnostics.test.mjs` PASS；`node --test test/startup-diagnostics.test.mjs` **6/6 PASS**（diff 仅 1 个测试文件，按最小必要范围执行，未跑 full suite） |
| 交回动作 | Windows 执行 `npx --yes --package npm@11.19.0 -c "npm test --workspace desktop"`，预期 **48/48**（`FAIL_TEST` → `PASS_AFTER_TEST_FIX`） |
| 状态 | `WINDOWS_VERIFICATION_PENDING`（修复待 Windows 确认；Linux 侧 6/6 不替代 Windows 结论） |

#### 2. `CROSS_PLATFORM_REVIEW_REQUIRED` 评审结论：暂不实施 workaround

针对 “是否在 `wdio.conf.mjs` / WDIO 依赖版本层面加 workaround”，Cross-platform Owner 评审如下：

| 选项 | 结论 | 理由 |
|---|---|---|
| 降级到 `webdriver` 8.x（仍发送 `desiredCapabilities`） | **否决** | `@wdio/tauri-service` 1.4.0 与 WDIO 9.31.9 以 `webdriver` 9 为 peer 契约；降级会重现 WQ-ENG-09a 刚闭环的 `npm ls` invalid-tree 依赖契约问题，且 legacy 协议已废弃；Linux 无法验证该变更 |
| 现在就注入 `alwaysMatch` 能力 workaround | **暂不实施** | 该假设（tauri-driver 是否把 `alwaysMatch` 原样转发给 msedgedriver）只能在 Windows 验证；在无法验证的前提下把硬编码 build 输出路径写入测试配置，会让后续每次运行都掩盖真实的上游错误类别 |
| 维持现状，等待上游修复 | **采纳** | 保留 `e856343` 的 `TAURI_DRIVER_EDGE_VERSION`/`EDGEDRIVER_VERSION` 透传（无害且对将来有用），WQ-ENG-09b ordinary/advanced 保持 `BLOCKED_AUTOMATION`（上游缺陷），不记 PASS、不重复重试 |

同时更正 round-3 的结论：当时的主导假设是 “msedgedriver ↔ WebView2 runtime 版本不匹配”，Windows batch 2 已证明版本不匹配只是**表层症状**——即使把 driver 钉到与 WebView2 一致的 153，session 仍无法创建，根因是上游能力协商缺陷。`windows-wdio-handoff.md` 的 driver 前置章节已按实测证据更正（旧的 152 版本号作废，并新增停止条件）。

#### 3. 累积到下一 Windows batch 的非阻塞工作

| ID | 工作 | 前置 / 步骤 | 判定 | 优先级 |
|---|---|---|---|---|
| WQ-ENG-13 确认 | 重跑 `npm test --workspace desktop` | 无（仅需 `git fetch` 对齐本轮 implementation revision） | 48/48 → `PASS_AFTER_TEST_FIX`；仍失败 → 保留 `FAIL_TEST` 并回报 | P1 |
| WQ-ENG-09b-unblock-experiment（**有界实验，失败即回滚**） | 一次性探针：在 WDIO capabilities 中内联 `browserName: "webview2"` + `ms:edgeOptions.binary=<绝对路径>` + `webviewOptions: {}`，与既有 `tauri:options` 并存，钉 `EDGEDRIVER_VERSION=153.0.4234.46` 跑一次 ordinary E2E | 专用 WDIO build 已就绪；隔离端口 45460+ | session 建立且 `url=http://tauri.localhost/`、`title=XArchive`、`h1=工作台` → 采纳 workaround（需再补 binary 路径与 build 输出一致性的 follow-up）；仍报 `session not created … msedge.exe` → **立即回滚该改动**，证明 tauri-driver 不转发 `alwaysMatch`，WQ-ENG-09b 保持 `BLOCKED_AUTOMATION` 直到上游修复 | P2 |
| WQ-P1-16 / WQ-P1-17 | 解除上游阻塞后按 `windows-wdio-handoff.md` §9 停止条件解除路径复跑 | WQ-ENG-09b ordinary/advanced 解除 | 按既有步骤与预期判定 | P2 |

本轮 `WINDOWS_VERIFICATION_BLOCKING`：**无**。本轮不接管任何 Windows-specific implementation；唯一的仓库变更是 shared 测试契约修复与文档更正。

#### Windows batch 3 执行结果（2026-09-28，`264f6ed`）

| ID | Windows 结果 | 证据与处置 |
|---|---|---|
| WQ-ENG-13 确认 | `PASS_AFTER_TEST_FIX` | Desktop Node suite 48/48。首次受限运行 47/48，唯一失败为 `killTree` 下层 `taskkill` 被沙箱拒绝（PID 41536，`Access denied`）；清理精确子进程后提升权限重跑通过。日志见 `validation-artifacts\windows-batch-20260928-264f6ed\desktop-npm-test*.log`。 |
| WQ-ENG-09b-unblock-experiment | `BLOCKED_AUTOMATION`（探针与 runner 契约不兼容） | 按队列单次运行探针时，`@wdio/tauri-service@1.4.0` 在 `onPrepare` 拒绝 `browserName: "webview2"`（仅支持 `tauri` / `wry`），因此 0 specs、无 WebDriver session；worker 随后报 `uv_os_get_passwd returned ENOMEM`。临时改动已回滚。该结果不触达 tauri-driver，也不改变 WQ-ENG-09b upstream root cause。 |
| WQ-ENG-03 / WQ-ENG-08 / WQ-ENG-09a | `WINDOWS_PASS`（复用） | 当前验证范围未命中其影响区，复用有效历史 PASS。 |
| WQ-P1-16 / WQ-P1-17 | `BLOCKED_AUTOMATION` | 仍受上游 `tauri-driver` W3C capability 转发缺陷阻塞；不重复普通/高级 E2E。 |

本轮唯一代码验证对象为 `88050320e2d76fcd5eb6ceccfca5188ddd8b8e75`。**Manual Windows Validation Queue：** (1) Cross-platform Owner 重审与 `@wdio/tauri-service` 1.4.0 契约兼容的 WQ-ENG-09b 探针设计，或等待上游修复；(2) 上游修复前不运行 ordinary/advanced E2E；(3) 既有 GUI/Computer Use、WQ-P1-12 filesystem/reparse/Unicode/download、真实账号、Named Pipe、workflow、archive extraction 项保持原状态。

WQ-ENG-13 的 `CROSS_PLATFORM_CHANGE_REQUIRED` 已由 `8805032` 解决并经 Windows 确认。WQ-ENG-09b 探针定义需标记 `CROSS_PLATFORM_REVIEW_REQUIRED`：当前 recipe 在 service 校验阶段即不可执行。下一 Owner：Cross-platform Owner 复审该 recipe；WQ-P1-16/17 继续等待上游修复。

### Cross-platform batch 4（2026-09-28，WQ-ENG-09b 契约兼容探针 v2）

源码起点 `2ff52f7`（与 origin 同步）。本节关闭 Windows batch 3 交回的 `CROSS_PLATFORM_REVIEW_REQUIRED`，变更仅 `desktop/wdio.conf.mjs` + 1 个新测试 + 文档，**无产品代码、Rust、前端、依赖或打包改动**。

#### 1. Batch 3 reconcile

| 交回项 | 复核结论 |
|---|---|
| WQ-ENG-13 | `8805032` 已被 Windows 确认 **48/48** → 关闭为 `PASS_AFTER_TEST_FIX`。首次 47/48 的 `taskkill … Access denied` 属沙箱/提权环境限制，不是产品或测试缺陷，不重新打开。 |
| 探针 v1 失败 | **配方缺陷在我方，Windows 的回滚判定正确。** `@wdio/tauri-service@1.4.0` 在 `onPrepare` 校验 `browserName`（源码 `dist/esm/index.js`：只接受 `tauri`/`wry`，校验后立即 `delete cap.browserName`），`webview2` 在进入 tauri-driver 前即被拒绝，因此探针从未触达上游，**既不能证实也不能证伪** W3C 转发假设。 |
| `uv_os_get_passwd ENOMEM` | worker 启动失败后的次生错误，非独立缺陷，不立项。 |
| 新增 Linux 侧证据 | `@wdio/tauri-service` 自带嵌套 `webdriver@9.30.1`（root 为 `9.31.9`），`build/node.js` 中 `desiredCapabilities` **0 命中**，`startWebDriverSession` 只把调用方 caps 原样包成 `alwaysMatch`（`node_modules/webdriver/build/node.js:1326`），故依赖 legacy 字段的通路在当前依赖树中不可达，`webdriver` 8.x 降级选项继续否决。同时服务侧只删除 `hostname`/`port`/`browserName`（`dist/esm/index.js` 的 `onWorkerStart` 与 standalone `stripUnsupportedProps`），没有 `beforeSession` 替换钩子，因此未知键会随 `alwaysMatch` 透传——W3C 侧注入是可证的探针面，而 tauri-driver 是否再转发给 msedgedriver 仍未知。 |

#### 2. 探针 v2：契约兼容、默认关闭（本轮已实现）

| 项 | 内容 |
|---|---|
| 位置 | `desktop/wdio.conf.mjs` |
| 开关 | `WDIO_EDGE_BINARY_PROBE=1`（未设置时 capability 与探针前**逐字节一致**，不新增任何键） |
| 探针形状 | `browserName: "tauri"`（保持服务契约）+ 既有 `tauri:options.application` + 新增 `"ms:edgeOptions": { binary: <同一 appBinaryPath>, webviewOptions: {} }` |
| 检验假设 | tauri-driver 是否把调用方的 W3C `alwaysMatch` 转发给 msedgedriver。若转发，msedgedriver 将以我们的 build 作为 binary，而不是回退 `msedge.exe` |
| 单变量约束 | 只镜像 binary，不移动 `tauri:options.application`；不改 driver 取得方式、不改 build、不改 spec |
| Linux 验证 | `node --check` PASS；`node --test test/wdio-config.test.mjs` **4/4**；`node --test`（desktop）**52/52**。测试锁定默认形状不含 `ms:edgeOptions`、探针形状、`browserName ∈ {tauri, wry}`（v1 失败模式的回归护栏）、钉版仅在显式配置时透传 |
| 边界 | Linux 只能证明 **capability 形状正确**，不能证明 tauri-driver 是否转发；后者只能在 Windows 观察 |

#### 3. 累积到下一 Windows batch

| ID | 工作 | 前置 / 步骤 | 判定 | 优先级 |
|---|---|---|---|---|
| WQ-ENG-13 计数确认 | `npx --yes --package npm@11.19.0 -c "npm test --workspace desktop"` | `git fetch` 对齐本轮 implementation revision | 预期 **52/52**（原 48 + `wdio-config.test.mjs` 4）；Windows 已 52/52，维持通过 | P1 |
| **WQ-ENG-09b probe v2**（单次、默认关闭） | `WDIO_EDGE_BINARY_PROBE=1`（可叠加既有 `EDGEDRIVER_VERSION=153.0.4234.46`）+ 隔离端口跑一次 ordinary E2E；**不需要再手改 `wdio.conf.mjs`** | 专用 WDIO build 就绪；按 §9 记录 WebView2 runtime 与实际 msedgedriver 版本 | Windows 单次运行启动了 app，但三次 session 创建均报 `DevToolsActivePort file doesn't exist`；0 spec 通过。记 `BLOCKED_AUTOMATION`，未能判定 alwaysMatch 转发，交 Cross-platform Owner 复审探针行为；不重试 | P2 |
| WQ-P1-16 / WQ-P1-17 | `BLOCKED_AUTOMATION`；probe v2 未建立 native session（`DevToolsActivePort file doesn't exist`） | Cross-platform review 给出 reviewed recipe，或 tauri-driver 上游修复 | 不运行 ordinary/advanced E2E，直到解除条件满足 | P2 |

本轮 `WINDOWS_VERIFICATION_BLOCKING`：**无**。探针默认关闭，因此不影响任何现有 Linux/Windows 默认运行；未接管 Windows-specific implementation。

#### Windows batch 4 执行结果（2026-09-28，handoff `069bdbd`）

| ID | Windows 结果 | 证据与处置 |
|---|---|---|
| WQ-ENG-13 Desktop Node suite | `PASS`（52/52） | `npx --yes --package npm@11.19.0 -c "npm test --workspace desktop"`，Node `v24.19.0`、npm `11.19.0`；日志 `validation-artifacts\windows-batch-20260928-069bdbd\desktop-npm-test-elevated.log`。首次受限运行因 `taskkill ... Access denied` 在 `killTree` 超时；相同命令提升权限后 52/52，无代码变更。 |
| WQ-ENG-09b probe v2 | `BLOCKED_AUTOMATION` | 单次运行 `WDIO_EDGE_BINARY_PROBE=1`、`EDGEDRIVER_VERSION=153.0.4234.46`、`TAURI_DRIVER_PORT=45460`。tauri-driver/msedgedriver 启动了 `target\release\xarchive-desktop.exe`，运行日志写入 `application runtime initialized`；但三次 WebDriver session 创建均失败：`session not created: DevToolsActivePort file doesn't exist`，0/1 spec。未命中预设的 session+dashboard、`msedge.exe`、或 `onPrepare` 拒绝三种结局，不能判定 W3C `alwaysMatch` 转发假设。日志 `validation-artifacts\windows-batch-20260928-069bdbd\wdio-probe-v2.log`；WDIO backend log 为 0 bytes。 |
| Computer Use / native GUI | `BLOCKED_AUTOMATION` | 两次 `cua.getState()` 均返回 `apps: []`；此 runtime 不提供 `cua.listWindows()`。未取得可控原生窗口或截图，不能以应用启动日志代替 GUI 验收。 |
| WQ-ENG-03 / WQ-ENG-08 / WQ-ENG-09a | `WINDOWS_PASS`（复用） | 本轮仅新增 WDIO capability 配置/测试，影响面与这些已通过项目无交集。 |

实验结束自动清理通过：无 `xarchive-desktop`、`tauri-driver`、`msedgedriver` 进程，无本轮端口 `1420/4444/4445/45460–45463` 监听。`target\release\xarchive-desktop.exe` SHA-256 为 `780E9BEEC6313AAB512AECF8EE853C0D9DE62DD258D468C61EEEB897A8A0341A`。未运行全量 regression；本轮只验证 desktop suite 与该 handoff 指定的单次 E2E probe。

**Manual Windows Validation Queue：** GUI/WebView2/DPI/键盘焦点/辅助技术仍须可观察的桌面会话人工验收（本轮 Computer Use inventory 两次为空）；WQ-P1-12 filesystem/reparse/Unicode/controlled Sidecar fixture、真实账号、Named Pipe、workflow、archive extraction 保持原队列。WQ-P1-16/WQ-P1-17 仍 blocked，须待 Cross-platform Owner 复审 `DevToolsActivePort` 结果并给出可执行探针或上游修复后再运行。

---

### Cross-platform batch 5（2026-09-29，WDIO 解阻移植：依赖补丁 + 可选 driver 通道 + readiness gate 合并）

本节由 Cross-platform Owner 写入，处理 `origin/windows/webview2-readiness-gate` 交回的两项 follow-up，并从该分支移植 Linux 可验证的 Windows E2E 解阻能力。源码起点 `2843e61`（与 origin 同步，工作树 clean）。**不是整支合并**：远程分支还含 U8–U14、协议 v2、下载链路、发布链路的架构演进，另行立项；本轮只取 readiness/解阻面，不改变默认行为。

#### 1. 远程交回项处置（Linux 可判定，无需 Windows）

| 项 | 结论 | Linux 证据 |
|---|---|---|
| `CROSS_PLATFORM_REVIEW_REQUIRED`：`patch-wdio-tauri-service.mjs` 的 `@wdio/native-core` spawn 补丁 | **采纳并已实现**：Linux 已核实当前安装的 `@wdio/native-core@1.2.0`（esm/cjs）确含 `shell: process.platform === 'win32'`；改为 `shell: false` 后 Linux 行为不变（原条件在非 Windows 本就为 false），补丁幂等、版本不匹配时只告警 | `grep` 安装树命中模式；`node --test test/patch-wdio-tauri-service.test.mjs` 10/10 |
| `CROSS_PLATFORM_CHANGE_REQUIRED`：`sidecar.rs`、两 schema、两 JSONL fixture 是否为有效 current source | **有效，无需改动**：与远程分支**字节相同**，当前分支 `lib.rs` 已 re-export、`deny_unknown_fields` 契约有回归测试，repository-map 有职责登记；远程交回时“用户要求原样带回”，无实质分歧 | `diff` 相同 + `cargo test -p xarchive-protocol` 通过 |
| `desktop/src/main.js`（Sprint 0 占位 `desktopScaffold`） | **不采纳**：当前分支入口为 `desktop/src/main.jsx`（`index.html` 直接引用），全仓无任何引用 `src/main.js`；远程 `bootstrap.js` 同样无引用链。移植占位文件会引入死代码 | `index.html`、`vite` 引用链、`grep` 无引用 |
| 前端 readiness 标记（`data-xarchive-startup`/`startup-fallback`，`waitForStartupContract` 前提） | **暂缓，另立 batch**：当前前端无该标记；它的移植依赖远程 `90905f7` 的 `index.html`/`bootstrap.js`/`main.jsx`/`commands.rs` 改动，属桌面启动契约特性，不是本轮解阻的必需前置 | `grep` 当前前端无标记 |

#### 2. 本轮实现（Linux 可验证）

| # | 改动 | 说明 |
|---|---|---|
| 1 | 新增 `desktop/scripts/patch-wdio-tauri-service.mjs` + 根 `postinstall` + `desktop` `pretest:e2e*` | 幂等依赖补丁：banner 正则同时接受 `MSEdgeDriver` 与 `Microsoft Edge WebDriver`（WQ-P1-16/WQ-P1-17 的 `Driver: unknown` 根因）；`native-core` 改为直接 spawn（远程 2026-09-23 含空格路径被 cmd 截断的根因）。仅改 `node_modules`，写入后校验 |
| 2 | 新增 `desktop/test/patch-wdio-tauri-service.test.mjs`（10 项） | banner 补丁重写/幂等/容忍未来版本/精确替换/双横幅接受；spawn 补丁重写/幂等/只替换 win32 条件；安装树接线检查 |
| 3 | `desktop/wdio.conf.mjs` 增加可选通道（默认关闭） | `TAURI_DRIVER_PATH`、`EDGEDRIVER_PATH`（Windows 校验存在并前置 PATH）、`WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`、`WDIO_AUTO_DOWNLOAD_EDGE_DRIVER=0`；`outputDir` 与 logDir 同目录（日志不再散落 `desktop/logs`）；Windows `tauri:options` 附带 `webviewOptions: {}`。`autoDownloadEdgeDriver` 保留 Windows 默认 true，仅新增显式关闭 |
| 4 | `desktop/test/wdio-config.test.mjs`（4→10） | 可选通道透传、`outputDir` 一致性、平台感知的 `tauri:options` 期望（WQ-ENG-13 教训） |
| 5 | `desktop/scripts/wdio-tauri-service.mjs` + 4 项测试 | teardown 快照**实际分配**的 driver 端口（优先 driver pool，回退配置端口对）；`collectPortOwnerPids(ports)` 接受已快照端口 |
| 6 | 合并 readiness 实现：新增 `desktop/e2e/support/native-startup.mjs`（18 项测试），删除 `startup-diagnostics.mjs` 及其测试 | session-start 快照、`waitForApplicationDocument`（空白文档不再当作应用文档；`ONLY_BLANK_DOCUMENTS`/`NO_WINDOW_HANDLES`/`WINDOW_TARGET_SWITCH_FAILED` 区分）、`captureReadinessFailure`、`discovery.json` 时间线；`WDIO_LOG_DIR` 优先、`READINESS_DIAGNOSTICS` 兼容；失败读记 `<unavailable: …>` 且**不得被误判为已加载文档**；两 spec 改用新 gate+取证钩子 |
| 7 | 文档 | repository-map、setup、`windows-wdio-handoff.md` 新配方；历史结论不改写 |

#### 3. Linux 验证（按当前 diff 的最小必要范围）

| 检查 | 结果 |
|---|---|
| `node desktop/scripts/patch-wdio-tauri-service.mjs`（`postinstall` 等价） | exit 0；4 个文件 patched；二次运行 exit 0，全部 `already patched` |
| `node --test test/patch-wdio-tauri-service.test.mjs` | **10/10** |
| `node --check`（新模块、两 spec、`wdio.conf.mjs`、新测试） | PASS 6/6 |
| `node --test test/wdio-config.test.mjs` | **10/10**（原 4 + 新增 6） |
| `node --test test/wdio-tauri-service.test.mjs` | **12/12**（8 既有 + 4 新增） |
| `node --test test/native-startup.test.mjs` | **18/18**（含发现/回退/证据全路径） |
| `npm test --workspace desktop` | **84/84**（原 52 − 6 旧取证 + 38 新增/新增断言；killTree 仍通过） |
| `git diff --check` | PASS |
| Rust workspace / Sidecar pytest | `NOT RUN`（diff 无 Rust/Python/协议改动；`sidecar.rs` 字节相同） |
| 真实 native session / banner 修复效果 | `BLOCKED`（Linux 无 WebView2/msedgedriver，见下） |


#### 4. 交回 Windows 的验证项（全部 `WINDOWS_VERIFICATION_PENDING`）

| ID | 工作 | 前置 / 步骤 | 判定 | 优先级 |
|---|---|---|---|---|
| WQ-DRV-01 依赖补丁生效 | `npm ci` 输出含 `[patch-wdio-tauri-service] done`；`node desktop/scripts/patch-wdio-tauri-service.mjs` 二次运行全部 `already patched`；`@wdio/tauri-service` dist 含 patched 正则、`@wdio/native-core` dist 含 `shell: false,` | `git fetch` 对齐本轮 implementation revision 后执行 | 4 文件 patched + 幂等 → 通过；出现 `no … pattern` 警告 → 记录依赖版本并交回 Linux | P0 |
| WQ-DRV-02 banner 修复效果 | 运行 `npm run test:e2e:windows --workspace desktop`；观察 service 的 EdgeDriver 发现输出不再是 `Driver: unknown` | WQ-DRV-01 通过；已构建普通 artifact | 不再 `Driver: unknown` → 通过；仍 unknown → `FAIL` 并保留 service 日志 | P0 |
| WQ-ENG-09b-ORD ordinary readiness（新配方） | 固定 runtime（`WEBVIEW2_BROWSER_EXECUTABLE_FOLDER`）+ 预置匹配 driver（`EDGEDRIVER_PATH`）+ `WDIO_AUTO_DOWNLOAD_EDGE_DRIVER=0` + 隔离端口，跑一次 ordinary E2E；**不再手改 `wdio.conf.mjs`** | WQ-DRV-01/02 通过；专用/普通 build 就绪；记录 WebView2 runtime 与实际 driver 版本 | session 建立且 `url=http://tauri.localhost/`、`h1=工作台` → `WINDOWS_PASS`；仍空白则提交 `session-start.json`/`discovery.json`/`failure.json` 判定归属；`DevToolsActivePort`/`msedge.exe` 回退 → 保持 `BLOCKED`，不重试 | P1 |
| WQ-TEARDOWN-01 动态端口清理回归 | 占用 4444/4445 后跑一次 E2E（含失败路径），检查 allocator 回退端口与跑后无 driver/app 进程、无端口监听 | WQ-ENG-09b-ORD 已执行（无论成败） | 回退端口被清理且无残留 → 通过；残留精确 PID 清理记录 → `FAIL` 并保留 netstat/tasklist | P1 |
| WQ-ENG-13 计数确认 | `npx --yes --package npm@11.19.0 -c "npm test --workspace desktop"` | `git fetch` 对齐 implementation revision | 预期 **84/84**（52 既有 − 6 + 38），变化只来自本轮新增/替换的测试文件 | P1 |
| WQ-P1-16 / WQ-P1-17 advanced | `BLOCKED`：须待 WQ-ENG-09b-ORD 有效 native session 后才复跑；上游 `tauri-driver` 修复前不得运行 | reviewed recipe 或上游修复 | 按既有步骤与预期判定 | P2 |

本轮 `WINDOWS_VERIFICATION_BLOCKING`：**无**。新增通道默认关闭，`postinstall` 只改 `node_modules`；未接管 Windows-specific implementation。历史 `WINDOWS_PASS`（WQ-ENG-03/08/09a、产品侧 WebView2 渲染）继续有效，本轮 diff 与其影响区无交集。

#### 5. BLOCKED 项的手工验证程序（本轮新增；M1–M7 保持有效，不重复）

通用前置同 M 节总述。以下程序只在自动化不可用或 session 仍被阻塞时执行，结论**单独记录**，不得把人工结论记为自动化项 PASS。

**M8 — E2E readiness 人工对照（`BLOCKED_AUTOMATION` 的 session 归属程序）**

- 前置：可观察的 Windows 桌面会话；本轮 implementation revision 的构建产物。
- 步骤：① 独立启动 `target\\release\\xarchive-desktop.exe`，记录窗口是否渲染 Dashboard、记录应用实际加载的 WebView2 runtime 版本（不是 Edge 浏览器版本）与 `logs\\` 应用日志；② 若最近一次自动化 run 留下 `session-start.json`/`discovery.json`，对照其 handle 数、初始 URL、超时前最后一次采样；③ 若自动化从未建立 session（`onPrepare` 拒绝/无 handle），本程序只做产品侧判定，不反推 automation 结论。
- 预期：独立启动渲染正常 → 产品启动路径 PASS，automation 失败归因保留在 WDIO/driver/toolchain 侧；独立启动同样空白 → 升级为产品启动问题并交回 Cross-platform Owner。
- 判定：独立启动渲染完整 → 本项 PASS（产品侧），automation 项保持原 `BLOCKED`；否则 FAIL（产品侧）并记录截图与日志。

**M9 — driver/端口残留人工检查（teardown 证据程序）**

- 前置：一次 E2E run（含失败路径）刚结束；知道本轮分配的端口（WDIO 日志 `TAURI_DRIVER_PORT` 或 allocator 回退端口）。
- 步骤：`tasklist` 查 `xarchive-desktop`/`tauri-driver`/`msedgedriver`；`netstat -ano -p tcp` 查本轮端口与 1420/4444/4445 是否仍有 LISTEN；如有残留，按精确 PID 停止并记录 PID/端口/归属。
- 预期：无本轮进程与监听。
- 判定：干净 → PASS；有残留 → FAIL 并把精确 PID、端口、WDIO 日志段交回 Linux（launcher 快照逻辑回归依据）。

**M10 — 依赖补丁的人工确认（Windows 首次同步后一次）**

- 前置：Windows 工作副本完成 `npm ci`。
- 步骤：① 检查 `npm ci` 输出含 `[patch-wdio-tauri-service] done`；② 二次运行 `node desktop\\scripts\\patch-wdio-tauri-service.mjs`，确认全部 `already patched`；③ 若出现 `no … pattern` 警告，记录 `@wdio/tauri-service`/`@wdio/native-core` 实际版本与警告行，不修改任何仓库文件。
- 预期：①② 通过；出现警告时按步骤 ③ 交回 Linux（可能是上游已修复，补丁应退役）。
- 判定：①② 通过 → PASS；警告 → 保持 PENDING 并交回，不记 FAIL（补丁设计为容忍）。

#### Windows batch 5 执行结果（2026-09-29，handoff `589142f`）

| ID | Windows 结果 | 证据与处置 |
|---|---|---|
| WQ-DRV-01 | `PASS` | `npm ci --no-audit --no-fund`（npm 11.19.0）成功安装 546 packages，postinstall patch 4 个文件；手动二次执行补丁脚本均为 `already patched`。 |
| WQ-DRV-02 | `PASS` | WDIO 日志识别 `Microsoft Edge WebDriver 153.0.4234.46`，不再显示 `Driver: unknown`。 |
| WQ-ENG-13 | `PASS` | Desktop suite **84/84**。首次受限执行的 `taskkill ... Access denied` 归于权限限制；提权执行同一命令通过。 |
| WQ-ENG-09b-ORD | `BLOCKED_AUTOMATION` | 单次 ordinary recipe 已加载 WebView2 Runtime 153.0.4234.48 与 pinned EdgeDriver 153.0.4234.46，但 session 落到 Edge `155.0.4283.24` 的 `msedge.exe` binary，返回 driver/browser 版本不兼容，0 specs。按停止条件不重试。证据在 `validation-artifacts\windows-batch-20260929-589142f\wdio-ordinary-elevated*`。标记 `CROSS_PLATFORM_REVIEW_REQUIRED`，由 Cross-platform Owner 复审 binary selection / WebView2 channel。 |
| WQ-TEARDOWN-01 | `PASS` | 占用 4444/4445 后动态分配 60735/60736；失败路径 `onComplete` 停止 driver，检查无应用/driver 残留进程或端口 listener。 |
| WQ-P1-16 / WQ-P1-17 | `BLOCKED` | ordinary native session 未建立，不满足 advanced E2E 前置；未执行。 |
| M8 / Computer Use GUI | `BLOCKED_AUTOMATION` | 有界重试后 `cua.getState()` 仍为 `apps: []`，没有可控 XArchive 原生窗口；现有 Edge tabs 不能替代 native GUI 验收。 |
| M9 / M10 | `PASS` | M9 的动态端口失败路径清理通过；M10 的 clean install 与补丁幂等确认通过。 |

本轮 `WINDOWS_VERIFICATION_BLOCKING`：**无**。`WQ-ENG-03/08/09a` 与本轮 diff 无交集，复用历史 Windows PASS。未运行完整 regression、advanced E2E、其他 manual queue 项。日志目录：`validation-artifacts\windows-batch-20260929-589142f\`。Implementation revision：`589142f169b3cec1a72948074f9466f89c5680ff`；validation record revision：`169f425`。下一 Owner：Cross-platform Owner，处理 `CROSS_PLATFORM_REVIEW_REQUIRED` 并为 WQ-ENG-09b-ORD 提供 review 后的新执行配方。其余手工队列保持待办：M8 Dashboard/WebView2 GUI、DPI/焦点/辅助技术，WQ-P1-12 filesystem/reparse/Unicode/Sidecar fixture、真实账号、Named Pipe、workflow、archive extraction。不得在 binary selection recipe 获得复核前重复 ordinary/advanced E2E。

### Windows batch 6 Full package（2026-09-29，implementation `7e3d646`）

| ID | 结果 | 证据与处置 |
|---|---|---|
| WQ-WORKER-BUILD-01 | `WINDOWS_PASS` | Python 3.12.14 / PyInstaller 6.22.3 的 Windows one-dir worker 已进入 Full 包；`python312.dll` 存在；协议 `hello → ready`、unknown field → `INVALID_COMMAND`，以及 bundled gallery-dl 对本地 unsupported URL 的 `started → failed` 和 shutdown/exit 0 均通过。SHA-256 与日志见 `../development/windows-validation.md` batch 6。真实下载仍未验证。 |
| WQ-PACKAGE-FULL-01 | `WINDOWS_FAIL`（部分组装 PASS） | 新 Desktop Release 生成成功，manifest、worker、gallery-dl、Extension 均已组装；规范 fresh-build 命令在 Windows `spawn npm.cmd` 返回 `EINVAL`。使用刚生成 binary 并设置 `PORTABLE_ALLOW_BINARY_REUSE=1` 后 assembly 成功。根因标记 `CROSS_PLATFORM_CHANGE_REQUIRED`。 |
| M11 / Full Dashboard 与 WebView2 | `BLOCKED_AUTOMATION` | Computer Use 首次发现窗口，随后窗口状态采集遇 user-input signal；重新枚举无可控 XArchive window。包内只确认 `application runtime initialized`，不视为 Dashboard PASS。 |
| M12 / Full Sidecar UI lifecycle | `NOT RUN` | 没有稳定可控的主窗口；由人工在 Full 包内启动/停止 Sidecar 并核对 UI 状态与进程清理。 |
| M13 / Codex Edge Profile、Extension 按钮、真实下载 | `NOT RUN` | 本轮未操作 Edge Profile、未加载/刷新此包 Extension、未访问 Tweet 或真实下载。必须由人工按下列步骤确认端到端完成。 |

**Manual Windows Validation Queue：**

1. **M11：**从 `validation-artifacts\full-package-20260929-7e3d646\XArchive-7e3d646-windows-x64-full\xarchive-desktop.exe` 启动；确认主窗口和 Dashboard 可见，记录 SQLite/Sidecar 状态。若窗口不出现，保留包内 `logs\xarchive-*.log` 与进程信息。
2. **M12：**在运行环境卡片启动 Sidecar，确认 hello → ready 和绿色运行状态；再停止，确认 UI 更新且 worker 进程退出。
3. **M13：**在 Codex Edge Profile 的 `edge://extensions` 加载/刷新同一 Full 包的 `extension` 目录；访问 X，确认保存按钮出现、Extension 显示已连接；对一个公开 Tweet 创建一次任务，确认 Job 最终到完成或明确失败，并核对日志与下载文件。创建任务或停留在“下载中”都不算下载 PASS。

`CROSS_PLATFORM_CHANGE_REQUIRED`：修复 portable Windows build script 的 `npm.cmd` spawn。`CROSS_PLATFORM_REVIEW_REQUIRED`：评审打包过滤器排除 PyInstaller `_internal` 中 7 个 `.pyd` 的影响；本轮协议 smoke 通过，但未执行真实下载。两项完成后返回 Windows Owner 重跑 fresh Full build，并完成 M11–M13。

### Cross-platform batch 6（2026-09-29，Windows batch 5/6 reconcile 与 Linux 收口）

本节由 Cross-platform Owner 写入，按 [`../development/cross-platform-validation.md`](../development/cross-platform-validation.md) 合并 Windows batch 5（handoff `589142f`）与 batch 6（implementation `7e3d646`）的结果，并完成其中全部 Windows-independent 交回项。状态 `READY_FOR_WINDOWS`，本轮 `WINDOWS_VERIFICATION_BLOCKING` 为空。

#### 1. Batch 5/6 reconcile 结论

- **Batch 5 结果全部接受**：WQ-DRV-01/02、WQ-ENG-13（84/84）、WQ-TEARDOWN-01、M9/M10 `PASS` 复用，不再重复执行；WQ-ENG-09b-ORD 的 `BLOCKED_AUTOMATION` 归因（`tauri-driver` 只把 binary 写入 legacy `desiredCapabilities`，webdriver 9.x 只发 W3C `alwaysMatch`，因此 153/153 钉版无效）接受，转第 2 节 D 项处理；M8 `BLOCKED_AUTOMATION`、WQ-P1-16/17 `BLOCKED` 维持。
- **Batch 6 结果全部接受**：WQ-WORKER-BUILD-01 `WINDOWS_PASS` 复用；WQ-PACKAGE-FULL-01 拆为两类交回项——`npm.cmd` spawn `EINVAL`（`CROSS_PLATFORM_CHANGE_REQUIRED`）与 `.pyd` 过滤/版本来源（`CROSS_PLATFORM_REVIEW_REQUIRED`）；M11 `BLOCKED_AUTOMATION`、M12/M13 `NOT RUN` 维持手工队列。
- WQ-PACKAGE-FULL-01 保持 `WINDOWS_FAIL`，直到第 4 节 R2 项通过为止；不得因 A–D 在 Linux 完成就提前改为 PASS。

#### 2. 本轮 Linux 完成项（A–D）

**A. `npm.cmd` spawn 修复（`CROSS_PLATFORM_CHANGE_REQUIRED` 关闭，待 Windows 复验）**：`desktop/scripts/build-portable-windows.mjs` 的入口流程封装为 `main()` 并加直接执行守卫（`process.argv[1]` 与 `fileURLToPath(import.meta.url)` 比较），修复上轮完成后遗留的未闭合 `main()` 与缺失入口调用（曾致 `node --check` 失败）；`portable-package.test.mjs` 改从模块导出取 `spawnOptionsForCommand`，导入不再触发整个打包流程。fresh-build 的实际消除效果见第 4 节 WQ-PACKAGE-FULL-01-R2。

**B. `*.pyd` 过滤评审（`CROSS_PLATFORM_REVIEW_REQUIRED` 关闭，待 Windows 复验）**：模式来自 `6555d35`（Windows sidecar dll hygiene），但被全局应用后连同 PyInstaller one-dir worker `_internal` 的运行时 C 扩展一并排除（`_ssl`/`_hashlib` 等 7 个模块）；协议 smoke 不经过真实 HTTPS，不能证明无影响，Windows 评审结论正确。已从 `EXCLUDED_PACKAGE_PATTERNS` 移除 `*.pyd`（`*.pyc`/`*.pyo`/`__pycache__`/`.env` 等卫生项保留），并在排除清单处注明原因，新增回归测试 `package file filter keeps PyInstaller runtime extension modules`。

**C. 版本来源确认（`CROSS_PLATFORM_REVIEW_REQUIRED` 关闭，待 Windows 复验）**：canonical 来源是 Cargo workspace `version = "0.1.1"`（`de61eaa` 的有意提升）。`desktop/src-tauri/tauri.conf.json`（PE/EXE metadata 与 Tauri bundle 来源）、根 `package.json`、`desktop/package.json` 由 `0.1.0` 对齐到 `0.1.1`；`package-manifest.json` 版本改为默认派生自 `tauri.conf.json`（`PORTABLE_APP_VERSION` 仍可覆盖用于 release rehearsal），消除 `unknown` 与 PE 不一致两种偏差。`extension/manifest.json` 保持 `0.1.0`（浏览器扩展独立生命周期，本轮回滚未点名；如需同步另立任务）。新增测试 `portable manifest version derives from the Tauri config` 与 `product version sources stay aligned`。

**D. WQ-ENG-09b-ORD recipe 复审（`CROSS_PLATFORM_REVIEW_REQUIRED` 关闭）**：reviewed recipe 为绕过 `tauri-driver`、直连独立 msedgedriver，capability 使用 batch-2 步骤 4/5 已实证成功的形状（session `200`、`browserName=webview2`、`browserVersion=153.0.4234.48`、真实 `h1=工作台` 截图，见 `../development/windows-validation.md`）。`wdio.conf.mjs` 新增 opt-in `WDIO_DIRECT_DRIVER=1`（`WDIO_DIRECT_DRIVER_PORT` 默认 4445）实现该形状：`services: []`、`hostname: 127.0.0.1` + port、`browserName: webview2` + `ms:edgeOptions.binary=<app>` + 顶层 `webviewOptions: {}`，应用由 msedgedriver 自行启动；未设置时默认路径逐字节不变（`test/wdio-config.test.mjs` 覆盖）。配方步骤见 [`windows-wdio-handoff.md`](windows-wdio-handoff.md) 的 “batch-6 reviewed recipe”。

#### 3. Linux 验证（全量门禁，当前 diff）

| 检查 | 结果 |
|---|---|
| `node --check`（5 个目标文件） | PASS 5/5 |
| `node --test test/portable-package.test.mjs test/wdio-config.test.mjs` | PASS **31/31** |
| `npm test`（根） | PASS **91/91** + extension **13/13** |
| `npm run check`（vite build + extension check） | PASS |
| `cargo fmt --all -- --check` | PASS |
| `cargo test --workspace --no-fail-fast` | PASS（全部 ok） |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | PASS |
| `.venv/bin/python -m pytest sidecar/tests -q` | PASS 19/19 |
| `git diff --check` | PASS |
| 真机 fresh Full build / WebView2 direct session | `NOT RUN`（本会话在 Linux，交第 4 节） |

#### 4. 交回 Windows 的验证项（全部 `WINDOWS_VERIFICATION_PENDING`）

| ID | 项目 | 关联修改 | 精确步骤/命令 | 预期结果 | 优先级 | 人工交互 |
|---|---|---|---|---|---|---|
| WQ-PACKAGE-FULL-01-R2 | fresh Full build 复验 | A+B+C：`build-portable-windows.mjs`、`portable-package.mjs`、三个版本文件 | 同步后**不设** `PORTABLE_ALLOW_BINARY_REUSE=1`，执行规范 fresh-build 命令 `npm run build:portable:windows --workspace desktop`；随后检查产物与 worker `_internal` | 无 `EINVAL`/spawn 失败；fresh binary 直接组装成功；`package-manifest.json` 与 EXE properties 均 `0.1.1`；worker `_internal` 含 `_ssl.pyd` 等 7 个 `.pyd`；本地协议 smoke 保持 PASS（batch-6 步骤） | P0 | no |
| WQ-ENG-09b-ORD-R2 | direct msedgedriver 配方 ordinary smoke | D：`wdio.conf.mjs` opt-in direct mode | 按 [`windows-wdio-handoff.md`](windows-wdio-handoff.md) “batch-6 reviewed recipe”：独立 `msedgedriver.exe --port=<p>`（pinned 153.0.4234.46），设 `WDIO_DIRECT_DRIVER=1`、`WDIO_DIRECT_DRIVER_PORT=<p>`、`WDIO_APP_BINARY`，执行 `npm run test:e2e:windows --workspace desktop`，**单次** | session 直连成功且启动的是 `xarchive-desktop` 而非 `msedge.exe`；`dashboard.e2e.mjs` 通过（`h1=工作台`）；记录 `browserVersion`、截图与 session 结束后无 app/driver/端口残留。失败则保留日志交回，不改配置、不重试旧配方 | P1 | no |
| WQ-ENG-13-R2 | Desktop suite 计数确认 | 本轮 diff（测试层新增 6 项） | `npm test --workspace desktop`（`taskkill` 受限时按 batch-5 经验提权重跑同一命令） | **91/91** PASS | P1 | no |
| M11–M13 | Full 包人工队列 | A+B+C 修复后的包 | 步骤保持本文件 Windows batch 6 “Manual Windows Validation Queue” 原文 | 同原文判定（创建任务/停留下载中 ≠ 下载 PASS） | P1 | yes |

复用（diff 无交集，不重复执行）：WQ-DRV-01/02、WQ-ENG-13（84/84 结论）、WQ-TEARDOWN-01、M9/M10、WQ-ENG-03/08/09a、WQ-WORKER-BUILD-01；M1–M10 手工程序与 D 组上游阻塞项状态不变。旧配方 ordinary/advanced E2E 在 WQ-ENG-09b-ORD-R2 结果出来前仍不重试；WQ-P1-16/17 仍 `BLOCKED` 于 ordinary native session。

本轮 `WINDOWS_VERIFICATION_BLOCKING`：**无**。日志与产物继续记录于 `validation-artifacts\` 并回写 `../development/windows-validation.md`。
