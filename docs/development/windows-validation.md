# Windows 开发与验证清单

> 适用平台：Windows 10/11，优先验证 Edge，并回归 Chrome。文档日期：2026-09-10。

> **兼容入口与历史记录。** 当前 Windows Validation Queue 的唯一权威入口是 [`../validation/windows-queue.md`](../validation/windows-queue.md)。本文保留既有验证规范补充、历史执行结果和 reconciliation；开头的队列表格是历史快照，不应作为当前状态源。

## 2026-09-20 v0.2.0-pre.5 Windows Release Build verification

本次检查针对 GitHub pre-release `v0.2.0-pre.5`。tag `v0.2.0-pre.5` 指向 commit `ea2b8d3afb289239edec29e2e00620870bed2fe6`；对应的 Windows Release Build workflow_dispatch run 为 `35507188780`。

### Actions 结果

| 项目 | 状态 | 证据 | 结果 |
|---|---|---|---|
| Windows Release Build | `FAIL` | run `35507188780` | 失败于 `Build Native Messaging Host` 步骤 |
| Windows worker / 外部依赖 / 打包 / artifact 与 Release 上传 | `NOT RUN` | run `35507188780` steps API | 失败后全部跳过 |
| `v0.2.0-pre.5` GitHub Release assets | `NOT RUN` | Release asset API | 资产列表为空（0 个资产） |

### 分类与后续建议

- 分类：Windows CI 前置配置问题。`Build Native Messaging Host` 步骤要求 CI secret `XARCHIVE_EXTENSION_ID`（真实的 32 位小写 Chrome Extension ID），当前仓库未配置该 secret；synthetic ID 只允许用于本地 package-contract 测试。
- 后续处理：在仓库配置真实 `XARCHIVE_EXTENSION_ID` 后，重新触发 `windows-release.yml`（`release_tag=v0.2.0-pre.5`），并核对四类资产的文件大小、SHA-256、解压内容和 manifest。不要跳过该步骤或用 `v0.2.0-pre.4` 资产替代。

## 2026-09-20 v0.2.0-pre.6 Windows Release Build verification

本次验证针对最终 `v0.2.0-pre.6` tag，source commit 为 `435a9085a7d66bb12b9b515012999730080573d6`。正确的 tag-push Windows workflow run 为 `35518801950`。

### Actions 结果

| 项目 | 状态 | 结果 |
|---|---|---|
| Windows Release Build（tag push） | `FAIL` | 失败于 `Run Rust tests`，后续构建、打包和上传步骤跳过 |
| `spawn_ready_v2_completes_the_capability_handshake` | `FAIL` | `unexpected supervisor error: sidecar v2 hello handshake timed out` |
| `spawn_ready_v2_rejects_worker_without_required_capabilities` | `FAIL` | `unexpected supervisor error: sidecar v2 hello handshake timed out` |
| Tauri executable / Native Host / worker / external dependencies | `NOT RUN` | 前置 Rust tests 失败 |
| `.exe`、7z、repository-dependencies、Full bundle | `NOT RUN` | 未进入组装步骤 |
| GitHub Release assets | `NOT RUN` | `v0.2.0-pre.6` 资产列表为空 |

失败分类为 Windows CI / platform-specific Sidecar supervisor test failure，不是前置环境缺失导致的 `WINDOWS_BLOCKED`。当前不能跳过失败测试或将本次 run 记为 Windows PASS。后续修复应调查 Windows 子进程启动、stdout framing、worker v2 hello 输出和 handshake timeout；修复后使用新的 tag/source 重新执行完整 Windows workflow。

### 2026-09-21 v0.2.0-pre.6 manual rerun attempt

本次尝试使用当前已配置的 canonical Extension ID，直接以既有 tag `v0.2.0-pre.6` 重新触发 Windows workflow：

| 项目 | 状态 | 证据 | 结果 |
|---|---|---|---|
| workflow dispatch | `FAIL` | run `35567742785` | 使用 `--ref v0.2.0-pre.6` 和 `release_tag=v0.2.0-pre.6`，source 为 `ac586e609337947aeb51de8f5cce3185efc8995e` |
| Rust workspace check | `PASS` | run `35567742785` | Windows runner 完成 `cargo check --workspace` |
| Rust workspace tests | `PASS` | run `35567742785` | 旧 tag workflow 的 `cargo test --workspace` 完成成功 |
| Tauri executable | `PASS` | run `35567742785` | `Build Windows Tauri executable` 完成成功 |
| Native Host build/package | `FAIL` | run `35567742785` | 失败于旧 workflow 的 `Build Native Messaging Host` |
| `XARCHIVE_EXTENSION_ID` | `BLOCKED` | failed log | 旧 workflow 只读取 `secrets.XARCHIVE_EXTENSION_ID`；当前值配置在 Repository Variable，因此 runner 环境为空 |
| worker / external dependencies / package | `NOT RUN` | run steps API | Native Host 前置失败后全部跳过 |
| artifact / GitHub Release upload | `NOT RUN` | run steps API | 所有 upload steps 均 skipped |
| `v0.2.0-pre.6` Release assets | `UNCHANGED` | Release asset API | 仍只有原有的 application-only `.exe` 和 `.7z`，没有新增完整资产 |

关键错误摘要：

```text
XARCHIVE_EXTENSION_ID secret is required before publishing Windows Native Host assets
```

该结果不是当前工作区 workflow 的验证结果。`v0.2.0-pre.6` tag 内的 workflow 不包含当前 checkout ref、source/tag parity、Repository Variable fallback、Extension ZIP、release manifest 或 `SHA256SUMS` 修复。因此本次不能将 `v0.2.0-pre.6` 变成完整 Extension/Native Host 发布，也没有移动 tag 或覆盖已有资产。

另有一次手动 workflow run `35518832674` 使用默认 `main` source，虽然最终成功，但不属于 `v0.2.0-pre.6` 的构建证据，不用于 Release asset 或 source parity 结论。

## 2026-09-21 v0.2.0-pre.7 Windows Release verification

本次验证针对新建的 `v0.2.0-pre.7` tag。Release source commit 为 `7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692`，tag push Windows workflow run 为 `35570021396`。

### Actions 结果

| 项目 | 状态 | 证据 | 结果 |
|---|---|---|---|
| checkout 与 source/tag parity | `PASS` | run `35570021396` | `headBranch=v0.2.0-pre.7`、`headSha=7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692`；parity gate 通过 |
| Rust workspace check | `PASS` | run `35570021396` | Windows runner 完成 `cargo check --workspace` |
| Rust workspace tests | `PASS` | run `35570021396` | `cargo test --workspace` 成功 |
| Tauri executable | `PASS` | run `35570021396` | Windows executable 构建成功 |
| Native Host | `PASS` | run `35570021396` | canonical Extension ID 校验和 Native Host 构建成功 |
| Windows worker | `PASS` | run `35570021396` | PyInstaller worker 构建和 smoke check 成功 |
| External dependencies | `PASS` | run `35570021396` | gallery-dl 与 aria2 下载、版本检查和打包成功 |
| Five-asset release manifest gate | `PASS` | run `35570021396` | JSON manifest、五类资产 hash/size/license metadata 和 `SHA256SUMS` 生成成功 |
| Artifact / GitHub Release upload | `PASS` | run `35570021396` | 所有 artifact 与 Release upload steps 成功 |
| Duplicate manual dispatch | `CANCELLED` | run `35570054865` | 同 source 的重复手动 run 在完成前取消，不作为资产来源 |

### Release assets

`v0.2.0-pre.7` Release 当前包含 7 项已上传资产：

```text
SHA256SUMS-v0.2.0-pre.7.txt
XArchive-v0.2.0-pre.7-extension.zip
XArchive-v0.2.0-pre.7-release-manifest.json
XArchive-v0.2.0-pre.7-windows-x64-full.7z
XArchive-v0.2.0-pre.7-windows-x64-repository-dependencies.7z
XArchive-v0.2.0-pre.7-windows-x64.7z
XArchive-v0.2.0-pre.7-windows-x64.exe
```

从 GitHub Release 下载后，在 Linux 上独立执行 SHA-256 对照：

```text
SHA256SUMS comparison: PASS
release manifest content: PASS
```

manifest 确认：

```text
tag:          v0.2.0-pre.7
source_sha:   7abf69a075f64e5f7d7d66ad0cc0ecc3b35f4692
extension_id: iaajefkoanbkleojofoadeakelihbjne
asset_count:  5
```

该结果证明 Windows 构建、五类资产打包、hash/size metadata 和 Release 上传已完成；**不证明** Windows Named Pipe、Registry、Edge/Chrome developer-mode、Service Worker reconnect 或真实 Browser → Desktop 归档链路已经完成。

## 2026-09-20 v0.2.0-pre.3 GitHub Actions release verification

本次检查针对 GitHub pre-release `v0.2.0-pre.3`。Release tag `v0.2.0-pre.3` 指向提交 `baf0b241237afbd9fb7435f96403af2de5598d91`；当前分支后续的文档提交 `6e97ea2f4e645c61314aa782e4c871091a75b894` 不在该 tag 中。GitHub Release 正文已后续更新为中文版本，但不改变 tag 对应的构建源代码。

### Actions 结果

| 项目 | 状态 | 证据 | 结果 |
|---|---|---|---|
| Windows Release Build（push） | `FAIL` | run `35492155317` | 失败于 `Run Rust tests`；后续 Tauri、worker、外部依赖、打包和上传步骤全部跳过 |
| Windows Release Build（workflow_dispatch） | `CANCELLED` | run `35492159783` | 重复手动 run 被取消，不能作为成功证据 |
| v0.2.0-pre.3 Release asset list | `FAIL` | GitHub Release API | 资产列表为空；没有 `.exe`、应用压缩包、repository-dependencies 包或 full bundle |
| Windows Release Build artifacts | `NOT RUN` | run `35492155317` artifacts API | 没有生成可下载的 workflow artifact |

### 失败测试

Windows runner 已完成 checkout、Node/Python/Rust toolchain setup、依赖安装和 Rust workspace check。Rust 测试大部分通过，但 `xarchive-sidecar-supervisor` 的以下两个测试失败：

- `tests::spawn_ready_v2_completes_the_capability_handshake`
- `tests::spawn_ready_v2_rejects_worker_without_required_capabilities`

两项错误均为：

```text
unexpected supervisor error: sidecar v2 hello handshake timed out
```

测试摘要为 `4 passed; 2 failed`，进程以 exit code `1` 结束。因此本次 run 未执行以下项目：

- Windows Tauri executable build；
- Windows worker build；
- gallery-dl 和 aria2 下载及 smoke check；
- executable / 7z / repository-dependencies / full bundle 组装；
- workflow artifact 上传；
- GitHub Release asset 上传。

### 发布结论

`v0.2.0-pre.3` 当前不能视为包含正确完整构建内容的可用预发布版本。其 Release 仍为 pre-release，但资产为空；不得把 `v0.2.0-pre.2` 的四类历史资产或此前成功 workflow 的结果外推到 `v0.2.0-pre.3`。

前一成功 Windows run `35485163451` 对应的是 `v0.2.0-pre.2` / commit `f2ae58db1f5f8be901e1c45f7629147056edeea9`，不能替代 `v0.2.0-pre.3` 的构建验证。

### 分类与后续建议

- 分类：Windows CI / platform-specific test failure，当前应记录为 `WINDOWS_FAIL`，而不是 `WINDOWS_PASS` 或 `WINDOWS_BLOCKED`。
- 首要修复范围：分析 `crates/xarchive-sidecar-supervisor` Windows 测试 fixture、子进程启动方式、stdout framing 和 hello handshake timeout；不要跳过失败测试直接上传资产。
- 修复后必须使用包含修复的最终提交创建新的 tag/release，重新执行完整 Windows workflow，并确认四类资产、文件大小、hash、解压内容、manifest、license 和 Release asset list。
- 在构建成功前，不应将 `v0.2.0-pre.3` 标记为可供用户下载的完整 Windows 发行包。

## 2026-09-20 v0.2.0-pre.4 GitHub Actions release verification

为修正 `v0.2.0-pre.3` 的 tag/source parity 问题，本轮使用当前分支最终提交创建 `v0.2.0-pre.4`，并通过 GitHub Windows runner 重新构建和发布。

### Actions 结果

| 项目 | 状态 | 证据 | 结果 |
|---|---|---|---|
| Windows Release Build（tag push） | `PASS` | run `35497313604` | checkout、Rust tests、Tauri、worker、外部依赖、四类 archive/artifact 和四次 Release upload 全部通过 |
| v0.2.0-pre.4 source/tag parity | `PASS` | tag `v0.2.0-pre.4`、commit `38e9a78a56260f7064b9ebf6a5230b0a9260002e` | workflow `headSha`、tag object commit 和 Release target 一致 |
| Workflow artifacts | `PASS` | run artifacts API | executable、7z、repository-dependencies、full bundle 四项均存在且未过期 |
| GitHub Release assets | `PASS` | Release asset API | 四项资产均为 `uploaded` 状态 |

### v0.2.0-pre.4 资产

| 资产 | 大小（bytes） | 状态 |
|---|---:|---|
| `XArchive-v0.2.0-pre.4-windows-x64.exe` | 18,250,240 | `uploaded` |
| `XArchive-v0.2.0-pre.4-windows-x64.7z` | 4,266,292 | `uploaded` |
| `XArchive-v0.2.0-pre.4-windows-x64-repository-dependencies.7z` | 5,918,138 | `uploaded` |
| `XArchive-v0.2.0-pre.4-windows-x64-full.7z` | 34,257,996 | `uploaded` |

本轮 workflow 已生成并上传四类资产；资产内容、SHA-256、7z 解压边界、license/source scan、真实 bundle parity 和运行时集成仍需按照 Windows Validation Queue 的专项步骤继续检查，不能仅凭 workflow 成功关闭全部 Windows 项目。

### 与 v0.2.0-pre.3 的区别

- `v0.2.0-pre.3` 仍保留为历史失败/错配记录：其 tag 指向 `baf0b24`，早期 workflow 失败且没有资产；后续对分支新提交触发的构建虽然成功，但 source 不属于该 tag，因此不把它视为 pre.3 的正确最终构建。
- `v0.2.0-pre.4` tag、workflow source 和 Release target 均指向 `38e9a78`，是当前 source 的正确构建发布对象。

Linux 可验证协议、Rust 核心、Python 逻辑和前端静态检查，但不能替代 Windows 专属集成验证。本文集中记录必须在 Windows 实机或 Windows CI 完成的任务。

## 2026-09-17 Full/Core portable handoff

本轮 Linux 已完成 Full/Core 包类型 manifest、Sidecar worker/gallery-dl 配置分离、Core 外部 `gallery-dl.exe` 校验保存、Core 本地 Extension 导入和构建脚本分支。Windows 仍需验证真实 portable 包、Windows executable、WebView2 路径/文件交互、浏览器加载和无终端行为；所有相关项目保持 `WINDOWS_VERIFICATION_PENDING`。可信远程 Extension artifact 尚未定义，Core 下载流程本轮不执行。

## 2026-09-17 发布问题修复批次 handoff

本轮 Linux 已完成：首次启动时独立初始化 `config/archive.sqlite3`、任务列表数据库 fallback、设置页 Error Boundary、运行日志页面（历史读取 + 1 秒轮询）、日志筛选/搜索/自动跟随/复制/打开目录，以及 Release 主程序和已覆盖子进程的无控制台启动设置。Linux 证据和当前唯一队列见 [`../validation/windows-queue.md`](../validation/windows-queue.md) 的 `WQ-REL-*` 项目。

以下项目必须在 Windows 保持 `WINDOWS_VERIFICATION_PENDING`，不能由 Linux 结果外推：

| ID | 项目 | Windows 验证重点 | 状态 |
|---|---|---|---|
| WQ-REL-DB-01 | SQLite/任务列表首次启动 | 全新 portable 目录且未完成下载目录 setup 时 SQLite ready、任务列表无初始化错误 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-REL-SETTINGS-02 | 设置页回归 | WebView2 进入设置页、切换 aria2/Extension/日志设置无白屏 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-REL-LOG-03 | 运行日志页面 | Windows 文件读取、轮询刷新、筛选、搜索、滚动、复制、打开目录 | `WINDOWS_VERIFICATION_PENDING` |
| WQ-REL-CONSOLE-04 | 无终端窗口 | Release 主程序、Sidecar、aria2、PowerShell 解压和任务执行全过程无控制台闪现 | `WINDOWS_VERIFICATION_PENDING` |

若 WDIO/WebView2 自动化仍因 `DevToolsActivePort` 或 driver 生命周期失败，则跳过自动化并按 `BLOCKED_AUTOMATION` 记录，使用队列文档中的手工步骤：全新 portable 启动、进入设置页、打开运行日志并触发 Sidecar/aria2 状态变化、复制日志、录屏观察主程序和子进程窗口。失败时收集 Windows 版本、revision、日志文件、进程 PID、截图/录屏；不得把未执行项目标为 PASS。

## 2026-09-16 GUI settings / Extension batch handoff

本轮 Linux source 完成了工作台/设置页拆分、字体和图标统一、`./logs` 路径归一化、Extension 文件完整性检测和 Edge/Chrome 分步骤加载指南。Linux 已完成 Vite、Node、Rust 和 Extension 门禁；本节只记录 Windows 后续验证，不把 Linux 结果外推为 Windows GUI 或浏览器集成通过。

### Linux validation evidence

| 项目 | 状态 | 证据 |
|---|---|---|
| Desktop Vite check/build | `PASS` | `npm run check --workspace desktop`、`npm run build --workspace desktop` |
| Desktop Node tests | `PASS` | `npm run test --workspace desktop`，8/8 |
| Extension check/tests | `PASS` | `npm run check --workspace extension`、`npm run test --workspace extension`，7/7 |
| Desktop Rust check/tests | `PASS` | `cargo check -p xarchive-desktop --all-targets`；`cargo test -p xarchive-desktop --all-targets --no-fail-fast`，71/71 |
| Rust formatting/lint | `PASS` | `cargo fmt --all -- --check`；Desktop strict Clippy 通过 |
| Windows WebView2/DPI/Native Host/Named Pipe | `BLOCKED` / `WINDOWS_VERIFICATION_PENDING` | 当前 Linux 环境不能提供 Windows WebView2、真实 Edge/Chrome、Registry、Named Pipe 和辅助技术；步骤见 `../validation/windows-queue.md` 的 GUI-W、EXT-W 项目 |
| Sidecar 下载按钮 | `BLOCKED` / `WINDOWS_VERIFICATION_PENDING` | 当前仓库没有可信 XArchive Sidecar Windows artifact、allowlist、SHA-256、签名和许可证清单；本轮未实现伪下载入口 |

### Required Windows handoff

1. 同步当前 Linux source（branch `dev`、最终 commit 以验证开始时记录为准）到 Windows 工作副本，不反向同步 Windows 依赖、缓存或用户数据。
2. 构建并启动 portable artifact，检查工作台/设置页、Sidecar、aria2、Extension、路径和日志界面。
3. 在 100%、125%、150% DPI 下检查图标居中、系统字体 fallback、路径溢出、键盘焦点和屏幕阅读器名称。
4. 删除/恢复 `extension` 必需文件，验证文件状态提示；注册 Native Host 并加载扩展后，再验证真实 hello、重连和断开状态。
5. 只有在 Sidecar artifact contract 完成后，才执行 Sidecar 下载、SHA-256、签名、安装、升级、回滚和 `hello → ready` 验证。

详细项目、前置条件、命令、预期结果、优先级及人工交互要求以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准。

## 状态定义

| 状态 | 含义 |
|---|---|
| 已完成 | 已有明确测试结果或代码验证结果 |
| 待实现 | 对应功能尚未开发 |
| 待验证 | 功能已有，但尚未在 Windows 目标环境验证 |
| 阻塞 | 依赖工具、凭据、证书或外部环境 |
| 跳过（不进行验证） | 本轮明确不执行；不表示 PASS、FAIL 或已完成验证 |

> 本项目开发阶段遵循“批量开发、集中验证”规则：Linux 可继续完成的功能不因最终需要 Windows 验证而暂停。开发过程中发现的 Windows 项目先进入累计 Windows Validation Queue；只有缺少 Windows 结果会使后续 Linux 设计或实现无法可靠继续时，才使用 `WINDOWS_VERIFICATION_BLOCKING`。当前 Queue 没有 `WINDOWS_VERIFICATION_BLOCKING` 项目。

### Linux reconciliation after incremental security-contract validation（2026-09-16）

本轮依据当前 diff 和增量验证策略，仅处理共享 Sidecar command contract 的 Linux 可验证边界。`crates/xarchive-protocol/src/sidecar.rs` 为 `SidecarCommand` 增加 `serde(deny_unknown_fields)`，并新增回归测试，确认已移除的 per-request `executable` 字段不会被 Rust JSONL consumer 接受。

#### Validation scope

| 范围 | 项目 | 结果 |
|---|---|---|
| Validated | `cargo test -p xarchive-protocol` | PASS，11/11 |
| Validated | `cargo test -p xarchive-sidecar-supervisor` | PASS，4/4 |
| Validated | `cargo test -p xarchive-desktop` | PASS，70/70 |
| Validated | `cargo fmt --all -- --check` | PASS |
| Validated | `git diff --check`、docs Markdown 相对链接检查 | PASS，`BAD_LINKS: NONE` |
| Not required | Python、Node、Tauri build、WDIO、全 workspace full suite | 本轮仅修改 Rust shared protocol model、targeted regression 和验证文档；未命中这些模块/平台影响区 |
| Deferred | Windows WQ-P1-12、真实 Sidecar/Named Pipe、ACL、reparse/junction、GUI 和账号链路 | 依赖 Windows endpoint、受控 fixture 或外部环境 |

`Full test suite not run because current changes are limited to the shared Sidecar command contract and its direct Rust consumers.`

WQ-P1-12 已回到 `WINDOWS_VERIFICATION_PENDING`，不得依据本轮 Linux contract 测试提前标记为 `WINDOWS_PASS`。若 Windows 真实 endpoint、受控 Sidecar fixture 或 reparse harness 仍不可用，则跳过对应项目并记录为 `BLOCKED`/`NOT RUN`；手工步骤沿用 `../validation/windows-queue.md` 的 BLOCKED / NOT RUN 入口。

## 当前 Windows Validation Queue

以下队列根据当前 Plan、最终工作区变更、Windows 相关模块和历史验证结果累计维护。除明确标记外，状态均为默认的 `WINDOWS_VERIFICATION_PENDING`，不要求中断当前 Linux development phase。

| ID | Validation item | Related feature/change | Files/modules | Why Windows is required | Exact behavior | Prerequisite | Expected result | Priority | Blocks Linux development | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-P0-01 | Windows toolchain and full baseline | 当前 Rust/Node/Tauri/Sidecar 工作区及后续批量变更 | `Cargo.toml`、`package.json`、`desktop/`、`sidecar/` | MSVC、Windows SDK、WebView2、Python executable 和 Windows 构建行为不能由 Linux 完全替代 | 执行 workspace fmt/check/test/clippy、Node check/test/build、Sidecar tests、Tauri build/start/cleanup | Windows toolchain、项目 `.venv`、Node dependencies | 所有适用基础检查通过，无项目代码失败 | P0 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P0-02 | 真实 X/Edge Cookie archive | gallery-dl 默认链路、Sidecar 和 ArchiveService | Edge Profile、`sidecar/`、`xarchive-sidecar-supervisor`、`xarchive-storage` | Cookie 加密存储、Edge Profile 和真实 X 响应只能在目标环境确认 | 无媒体/单图/多图/视频/Quote/Reply/重复任务/异常退出后的真实归档 | 明确测试账号、Edge Profile、gallery-dl、可用网络 | Cookie 不泄露；Tweet/媒体/SQLite/staging 正确且幂等 | P0 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P0-03 | 文件 SQLite 应用级恢复 | Telegram send state migration、Desktop 启动恢复 | `crates/xarchive-storage/migrations/`、`xarchive-storage`、Tauri Desktop | 文件锁、应用重启、Windows 路径和异常退出无法由 in-memory 测试充分判断 | 写入真实文件 DB、关闭/重启、遗留 staging、`0001 → 0002`、`0002 → 0003`、异常退出恢复 | Desktop artifact、受控测试目录、可重复数据 | 状态恢复、迁移、staging 清理和唯一约束符合预期 | P0 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P0-04 | Native Host/Named Pipe end-to-end | Native Host endpoint forwarding、后续 Windows Named Pipe backend | `xarchive-native-host`、`xarchive-protocol`、Desktop IPC | Named Pipe server、ACL、连接和 Windows IPC 生命周期是平台行为 | 请求/响应、request_id 路由、多连接、重连、关闭、非法消息和权限拒绝 | Windows Named Pipe server、endpoint `\\.\\pipe\\xarchive-v1`、ACL 方案 | 合法请求正确转发，非法/越权请求明确失败，无串线或挂起 | P0 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P1-01 | DownloadRouter and real aria2 business integration | `DownloadRouter`、Desktop Job 结果处理、403 fallback、Sidecar/Job 接入 | `crates/xarchive-download`、`desktop/src-tauri/src/lib.rs`、Sidecar/Job orchestration | aria2c.exe 进程、Windows 路径、真实 media URL 和进程恢复需目标环境确认 | gallery-dl 默认；Router 错误不 panic；失败 Job/事件持久化；403 后重新提取；aria2 transfer 生命周期；失败回退和 Job 状态同步 | 受控 aria2c.exe、真实或本地 HTTP media server、可重复 Desktop archive 场景 | fallback 只在适用错误触发，状态、事件和文件结果一致 | P1 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P1-02 | Native Host browser installation | Host manifest、Registry、Edge/Chrome 加载 | `crates/xarchive-native-host`、manifest/installer（待实现） | Registry、浏览器扩展 ID 和安装权限是 Windows 专属行为 | 安装/升级/卸载、管理员/非管理员、扩展加载、Service Worker 重启和重连 | Host manifest、固定 Extension ID、浏览器实机 | Edge/Chrome 能加载 Host，连接和错误反馈符合协议 | P1 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P1-03 | Windows GUI rendering and accessibility | Dashboard GUI 修补与白色视觉重设计 | `desktop/src/main.jsx`、`desktop/src/style.css`、Tauri | WebView2/DPI/系统字体/屏幕阅读器/命中区域不能由 Linux 静态检查替代 | 100/125/150% DPI、最小窗口、Tab、键盘、Focus-visible、Narrator/NVDA、对比度 | GUI automation native-app target、WebView2、辅助技术 | 真实渲染、交互和辅助技术反馈通过 | P1 | no | 跳过（不进行验证） |
| WQ-P1-04 | Credential Manager and Telegram account flow | `SecretStore` abstraction、真实 Telegram transport | `xarchive-telegram`、Windows secret backend（待实现） | Credential Manager 用户边界和真实账号/网络行为是 Windows/外部环境事项 | 保存/读取/更新/删除、应用重启、日志隔离、真实 Bot API 发送与限流 | Windows Credential Manager backend、Bot token、Telegram test chat | Secret 不泄露，真实发送和重试状态正确 | P1 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P1-05 | Sidecar/externalBin/process lifecycle | Tauri externalBin、Sidecar packaging、Tray/Autostart | `desktop/src-tauri/`、Sidecar packaging（待实现） | Windows 子进程、资源路径、关闭和自启动行为需要目标环境 | 启动、关闭、崩溃恢复、资源定位、Tray、Single Instance、Autostart | bundled Sidecar、Tauri bundle、Windows shell environment | 资源可定位，子进程安全退出，生命周期符合预期 | P1 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P2-01 | Installer, signing and updater | M7/M8 发布能力 | Tauri bundle、installer、updater（待实现） | 安装器、签名、SmartScreen、Defender、升级/回滚为 Windows 发布行为 | 全新安装、覆盖升级、自定义路径、卸载、签名、失败回滚、数据保留 | installer artifact、证书/签名环境、发布测试机 | 安装、升级、卸载和回滚符合发布要求 | P2 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P2-02 | Windows filesystem stress and stability | 用户目录、staging、文件恢复和历史偶发路径问题 | `xarchive-core`、`xarchive-storage`、Desktop | 保留字符、磁盘、锁、长路径和并行时序需要 Windows 文件系统确认 | 非系统盘、空格/中文/Unicode、保留名、长路径、文件锁、磁盘不足、并行恢复 | Windows 多盘/受控权限/磁盘空间 | 无路径逃逸、数据损坏或未处理崩溃 | P2 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P1-12 | Security boundary regression for current Linux batch | 本轮 URL/metadata identity binding、settings allowlist、Sidecar executable removal、staging link rejection | `crates/xarchive-protocol`、`crates/xarchive-storage`、`desktop/src-tauri/src/lib.rs`、Sidecar protocol | Windows path semantics、reparse points、Desktop IPC packaging 和真实 Sidecar 进程边界不能由 Linux 完全替代 | 使用不匹配 Tweet ID/URL、metadata mismatch、任意 executable 字段、普通文件、symlink/junction/reparse point、敏感 settings key 和长 JSON 做拒绝测试 | 最新 Linux source working tree、Windows Rust workspace、受控 staging 目录、可创建 symlink/junction 的权限 | 非法 identity、executable override、敏感 settings 和 link/reparse 文件均被拒绝；合法 Unicode 文件仍可归档 | P1 | no | WINDOWS_VERIFICATION_PENDING |
| WQ-P1-13 | Windows user-data privacy boundary | 本轮仍使用应用归档目录、SQLite、staging 和 settings repository；Windows ACL 尚未由代码设置 | `desktop/src-tauri/src/lib.rs`、`crates/xarchive-storage`、`X-Archive/` | Windows ACL、user profile、当前工作目录和共享目录权限不能由 Linux 文件 mode 充分替代 | 在默认启动、非管理员用户、共享目录和不同盘符下检查 archive root/SQLite/staging ACL、路径定位和跨用户读取 | Windows 普通用户/非管理员账户、可检查 ACL 的测试工具、受控临时目录 | 数据目录定位稳定，仅当前用户可读写；不依赖 current working directory；权限错误可诊断 | P1 | no | 跳过（不进行验证） |

当前没有 `WINDOWS_VERIFICATION_BLOCKING`：上述项目虽有 P0/P1/P2 优先级，但当前 Linux 代码、测试和设计均可继续推进，不存在必须先取得 Windows 结果才能可靠完成的后续 Linux 实现。

## Windows Validation Preparation / 集中式 Handoff

Linux development phase 结束后，基于最终 `git diff`、当前 Plan、变更模块、Windows 代码路径、项目配置、历史验证和上方 Queue 合并重复场景，按以下顺序一次性执行。单次完整启动覆盖多个功能时，不拆成重复启动测试。

### 1. Build / Toolchain

- **ID:** W-H-01
- **Test name:** Windows workspace baseline and Tauri build
- **Purpose:** 确认 MSVC/SDK/Node/Python/Tauri 以及当前批量开发结果可构建。
- **Related changes:** 所有当前 Rust、Node、Tauri、Sidecar 变更。
- **Prerequisites:** Windows 10/11、MSVC、Windows SDK、WebView2、Node/npm、项目 `.venv`。
- **Steps / command:** `npm ci`; `npm run check`; `npm run test`; `npm run build`; `cargo fmt --all -- --check`; `cargo check --workspace`; `cargo test --workspace`; `cargo clippy --workspace --all-targets -- -D warnings`; `npm run build:tauri`。
- **Expected result:** 命令通过，构建 artifact 生成，无项目代码导致的失败。
- **Priority:** P0
- **Manual interaction required:** no

### 2. Runtime

- **ID:** W-H-02
- **Test name:** Combined Tauri, Sidecar and process lifecycle
- **Purpose:** 一次覆盖 Desktop 启动、SQLite 初始化、Sidecar hello、状态查询、停止和进程清理。
- **Related changes:** Tauri commands、Native Host endpoint、Sidecar orchestration、externalBin。
- **Prerequisites:** 可启动 Desktop artifact、Sidecar executable/configuration、Python fallback（如适用）。
- **Steps / command:** `npm run dev:tauri`；执行 start/stop Sidecar、状态刷新、退出；记录进程和日志。
- **Expected result:** 应用启动、Sidecar 握手、停止和退出清理成功，无残留进程；非阻塞 Chromium 清理 warning 单独记录。
- **Priority:** P0
- **Manual interaction required:** yes

- **ID:** W-H-08
- **Test name:** R1 executor production integration, recovery and synchronous fallback
- **Purpose:** 在阶段二 runner-owned context、startup recovery 和 cancellation 完成后，验证 Tauri command、worker ownership、SQLite recovery、FileStore commit 和旧同步 fallback 行为等价；阶段一 revision 仅作为 contract/queue 前置，不作为 production acceptance。
- **Related changes:** `desktop/src-tauri/src/executor.rs`、`archive.rs`、`runtime.rs`、`commands.rs`、`xarchive-storage`、SidecarSupervisor、FileStore。
- **Prerequisites:** 当前 executor integration revision、Windows Tauri artifact、项目 Python/Sidecar、受控 SQLite/archive root、可重复 fixtures、可制造异常退出和文件锁。
- **Steps / command:** 执行 submit/query/cancel/shutdown、duplicate/concurrent jobs、同步 fallback、Sidecar crash、graceful shutdown、restart recovery；按 WQ-P1-15 检查 DOWNLOADED/staging/final/COMPLETE/missing/mixed recovery cases，并在 Windows 文件锁/异常退出条件下验证 rename 与重启行为；记录每个 Job 的 state/event/error 顺序。
- **Expected result:** RuntimeState 不被长 I/O 锁住；单 Job 单 worker；fallback 与 executor contract 等价；recovery 不重复归档、不错误报告 COMPLETE；Sidecar/worker 无残留进程；SQLite 事件和错误字段顺序正确。
- **Priority:** P1
- **Manual interaction required:** yes

### 3. Filesystem

- **ID:** W-H-03
- **Test name:** File SQLite restart, migration and Windows path matrix
- **Purpose:** 合并文件 DB 恢复、staging、路径、锁和 Unicode 场景。
- **Related changes:** `xarchive-storage` migrations、FileStore、用户目录和 Desktop archive root。
- **Prerequisites:** 真实文件 DB、系统盘和非系统盘、受控权限、可制造文件锁/异常退出。
- **Steps / command:** 按 WQ-P0-03/WQ-P2-02 执行真实文件 DB 写入、重启、迁移、遗留 staging、空格/中文/Unicode/长路径、文件锁和异常退出。
- **Expected result:** 文件不逃逸 archive root，迁移和重启恢复正确，锁/磁盘错误可诊断，数据不损坏。
- **Priority:** P0
- **Manual interaction required:** yes

### 4. Integration

- **ID:** W-H-04
- **Test name:** Browser Extension → Native Host → Named Pipe → Desktop
- **Purpose:** 合并浏览器集成、Native Messaging、Named Pipe、Registry 和 request_id 路由验证。
- **Related changes:** Native Host forwarding、Windows endpoint、manifest/Registry、Extension bridge。
- **Prerequisites:** Named Pipe server、ACL、Host manifest、固定 Extension ID、Edge/Chrome。
- **Steps / command:** 按 WQ-P0-04/WQ-P1-02 执行 archive/query、并发、多标签、重连、Desktop 未启动、非法请求和权限场景。
- **Expected result:** 合法消息按 request_id 正确返回；无效、断线、权限和 Desktop 不可用时快速返回结构化错误。
- **Priority:** P0
- **Manual interaction required:** yes

- **ID:** W-H-05
- **Test name:** Gallery-dl / DownloadRouter / aria2 / Telegram integration
- **Purpose:** 合并真实 X、403 fallback、aria2 transfer、Credential Manager 和 Telegram 发送链路。
- **Related changes:** `DownloadRouter`、Sidecar/Job 接入、Telegram SecretStore/send state。
- **Prerequisites:** 测试账号、Edge Profile、aria2c.exe、Bot token/test chat、优先使用本地 HTTP fake server。
- **Steps / command:** 执行真实或受控 media 场景、403/认证/限流错误、aria2 pause/resume/recovery、Telegram send/retry/restart。
- **Expected result:** 默认 gallery-dl；仅适用错误 fallback；Secret/ Cookie 不泄露；Job、文件、Telegram 状态一致。
- **Priority:** P0
- **Manual interaction required:** yes

### 5. Packaging

- **ID:** W-H-06
- **Test name:** Bundled Sidecar, installer, signing and updater
- **Purpose:** 确认发布 artifact、安装/升级/卸载和资源分发。
- **Related changes:** externalBin、Tauri bundle、Native Host manifest、installer/updater。
- **Prerequisites:** bundle/installer artifact、签名证书、发布测试机。
- **Steps / command:** 执行全新安装、覆盖升级、自定义路径、非 ASCII 路径、卸载、失败回滚、Updater 和 Defender/SmartScreen 检查。
- **Expected result:** 资源可定位，数据按策略保留，升级/回滚安全，签名和安装行为符合预期。
- **Priority:** P1
- **Manual interaction required:** yes

### 6. Regression

- **ID:** W-H-07
- **Test name:** GUI and previously verified Windows regression suite
- **Purpose:** 复验本轮修改涉及的 GUI 语义/错误恢复/aria2 gating，并保留历史 Windows 基线。
- **Related changes:** `desktop/src/main.jsx`、`desktop/src/style.css`、`xarchive-download`、`xarchive-native-host`。
- **Prerequisites:** W-H-01/W-H-02 通过；GUI automation target、WebView2、Narrator/NVDA（如适用）。
- **Steps / command:** 执行现有 Node/Rust/Sidecar/Tauri 基线，并按 WQ-P1-03 进行真实 DPI、Tab、键盘、Focus-visible、辅助技术和对比度复验。
- **Expected result:** 历史通过行为保持；新增项逐项给出 PASS/FAIL/BLOCKED，不以 Linux 结果替代 Windows 结论。
- **Priority:** P1
- **Manual interaction required:** yes

### 7. Security / Privacy regression

- **ID:** W-H-09
- **Test name:** Current Linux security-boundary batch regression
- **Purpose:** 验证本轮 identity binding、Sidecar trust boundary、settings allowlist 和文件 link 防护在 Windows 上保持一致。
- **Related changes:** `xarchive-protocol` URL/Tweet ID 校验；`xarchive-storage` Sidecar metadata identity binding、settings key/JSON/size 校验、symlink/reparse rejection；Desktop removal of generic settings IPC and per-request Sidecar executable override。
- **Prerequisites:** 最新 Linux working tree 已单向同步；Windows Rust workspace；受控 staging 目录；可创建测试 symlink/junction 的权限。
- **Steps / command:** 运行 `cargo test --workspace`；执行不匹配 URL/Tweet ID、Sidecar metadata ID、敏感 settings key、非法 JSON、超长 JSON、普通文件、symlink/junction/reparse point 场景；检查 Tauri command manifest 中不存在 generic settings command，归档请求不能注入 executable。
- **Expected result:** 所有非法输入明确失败；合法归档通过；没有越出 staging/archive root、没有任意 executable 启动、没有敏感 settings 返回 WebView。
- **Priority:** P1
- **Manual interaction required:** no

### 8. Data directory ACL and privacy boundary

- **ID:** W-H-10
- **Test name:** Windows archive root, SQLite and staging ACL verification
- **Purpose:** 确认归档数据不会因当前工作目录或宽松目录权限暴露给其他 Windows 用户。
- **Related changes:** 当前 Desktop archive root 初始化和 FileStore/SQLite 数据布局；后续 user-data directory hardening。
- **Prerequisites:** Windows 普通用户和第二个本地用户、可检查 ACL 的工具、空白测试 profile。
- **Steps / command:** 分别从快捷方式、安装目录、其他 working directory 启动；检查 archive root、SQLite、WAL/SHM、staging 和 metadata 文件位置与 ACL；使用第二用户尝试读取。
- **Expected result:** 数据目录位置稳定且符合产品约定；第二用户无权读取；WAL/SHM 和临时文件不会落到未保护目录。
- **Priority:** P1
- **Manual interaction required:** yes

当前集中式 handoff 中没有 `WINDOWS_VERIFICATION_BLOCKING` 项。进入 Windows Validation Preparation 的前提是 Linux development phase 已完成，而不是某个普通 pending 项目单独完成。

## 本轮最终收口：BLOCKED / NOT RUN 手工验证

### 2026-09-19 U7 BLOCKED / NOT RUN 手工验证

U7 Linux production wiring 已完成，但当前 Linux 环境不能替代 Windows artifact、aria2c.exe、Windows process/file-lock、WebView2、Named Pipe 和 restart/recovery 证据。以下步骤用于 Windows 自动化或前置被阻塞时的手工替代；执行结果必须如实记录为 `PASS`、`FAIL`、`WINDOWS_BLOCKED` 或 `NOT RUN`。

#### U7-MANUAL-01：Sidecar v2 handshake 与 extraction-only

1. 同步 Linux source 对应的 `feature/u7-desktop-production-integration` revision 到 Windows 工作副本，记录 branch、commit、working tree changes、artifact hash、Windows 版本和架构。
2. 准备 packaged v2 worker、Desktop artifact、固定单图/视频/多媒体 fixture；设置项目 Python/worker 环境。
3. 启动 worker 或 Desktop production executor，发送 v2 `hello`，记录 stdout JSONL；发送 v1 hello 或 unknown field 作为负例。
4. 提交 `extract`，检查 `ready`、`extraction_started`、`extracted/failed` 的 `job_id`/`request_id`；检查 workspace/staging 不产生媒体主体文件。
5. 检查 `ExtractionResult` 的 media identity/order、filename、allowlisted headers；确认 signed URL/header 不写入 SQLite、`tweet.json` 或普通日志。

#### U7-MANUAL-02：aria2 transfer、refresh 与 cleanup

1. 准备 `aria2c.exe`、loopback RPC、fake media server 和可写 Unicode/空格 staging 目录；记录 aria2 版本和 RPC 端口。
2. 执行单媒体、多媒体 transfer，观察 waiting→active→complete、progress、最终文件和 `.aria2` 文件。
3. 让首次 URL 返回 403/expired signature；确认旧 GID 被移除，执行一次新的 extraction，使用新 plan/new GID 重试。
4. 让 refresh 返回新增/删除/重排 media identity；确认结果为 `EXTRACTION_RESULT_CHANGED`，不得静默覆盖或重复归档。
5. 分别制造 404、磁盘空间不足、权限错误、cancel、shutdown、timeout；确认普通错误、cancel、shutdown、timeout 不触发 refresh，并记录错误码和进程清理结果。

#### U7-MANUAL-03：staging、commit、restart/recovery

1. 提交 Job 后分别在 extraction、transfer、staging→final commit 阶段关闭 Desktop；保留 SQLite、WAL/SHM、staging、logs 和进程列表。
2. 重启应用，检查 execution spec、Job state、event/error 顺序、attempt fencing 和 late-result 行为。
3. 准备 `DOWNLOADED + staging`、`DOWNLOADED + final`、`DOWNLOADED + neither`、`COMPLETE + final`、`COMPLETE + missing` fixtures。
4. 制造文件锁、junction/reparse、路径逃逸、缺失文件和多余文件；确认 staging verification 拒绝不安全输入，hash/size/identity 校验失败不会提交。
5. 确认成功 commit 后 SQLite metadata/media、`tweet.json`/`tweet.txt`、最终目录和 Job state/event 一致；确认没有残留 Sidecar/aria2 子进程。

#### U7-MANUAL-04：BLOCKED_AUTOMATION 替代步骤

如果 WDIO/WebView2 因 `DevToolsActivePort`、driver 生命周期、GUI target 或缺少 artifact 被阻塞：

1. 记录阻塞原因、命令、Windows 版本、Node/WebView2/driver 版本、artifact 路径和 PID。
2. 不把 WDIO 或 Computer Use 未执行写成 PASS；改用 PowerShell/人工启动 Desktop 和 worker。
3. 使用相同 Tweet fixture 分别从 Browser transport 与 Tauri invoke 提交，记录返回耗时、`request_id`、`job_id`、state、events 和 errors。
4. 保存 stdout/stderr、SQLite、日志、截图/录屏、进程列表和 staging 目录；明确标记 `WINDOWS_BLOCKED` 或 `NOT RUN`。

## 2026-09-17 非 Windows 阶段最终 Windows handoff

本轮 Linux development phase 已结束。Linux 侧已完成可执行的代码、契约测试、构建脚本静态检查、PyInstaller spec/入口定义和 Windows artifact workflow；没有执行 Windows 专属验证，也没有将 Linux worker 伪装为 Windows `.exe`。以下项目统一留给 Windows 阶段；缺少前置条件的项目按 `BLOCKED` 或 `NOT RUN` 跳过，并使用对应手工步骤。

## 2026-09-18 异步 Job submit/schedule Windows handoff

本批次 Linux 侧已完成 Browser transport 与 Tauri command 的 submit-and-schedule 统一，并增加调度失败补偿：无效 persistence 路径会返回 `EXECUTOR_SCHEDULE_FAILED` 并将已创建 Job 标记为 `FAILED`；后台 production executor/factory 失败会持久化 `EXECUTOR_WORKER_FAILED` 和 `DOWNLOAD_FAILED` 事件。Linux 已通过 Desktop 80 tests、workspace check/test/strict Clippy、Node、Sidecar 12/12 和普通/WDIO Tauri build。

以下项目不能由 Linux 结果替代，且当前 Windows 环境未执行；自动化前置不可用时必须跳过自动化并按手工步骤记录，不得标记为 PASS：

### Runtime / Integration

- **ID:** W-HANDOFF-RUNTIME-ASYNC-01
- **Test name:** Browser transport and Tauri submit-and-schedule parity
- **Purpose:** 确认两条生产入口都立即返回初始状态，并共享 Job identity、query、cancel 和错误事件语义。
- **Related changes:** `desktop/src-tauri/src/executor.rs`、`transport.rs`、`commands.rs`、Native Host transport。
- **Prerequisites:** Windows Desktop artifact、Native Host/Named Pipe backend、可写 SQLite、受控 Tweet fixture、项目 Python/Sidecar。
- **Steps / command:** 分别通过 Browser archive request 和 Tauri invoke 提交同一 Tweet；记录 `request_id`、`job_id`、返回 state 和时间；重复提交；查询 Job；执行 cancel；比较两入口事件和错误字段。
- **Expected result:** 两入口均快速返回 `QUEUED` 或当前 active state，不等待完整 Sidecar/FileStore I/O；重复请求只产生一个 active Job；query/cancel/failure event 顺序一致；主动取消的 Job 不因 startup recovery 自动重新执行。
- **Priority:** P1
- **Manual interaction required:** yes
- **Status:** `WINDOWS_VERIFICATION_PENDING`

- **ID:** W-HANDOFF-RUNTIME-ASYNC-02
- **Test name:** Scheduling failure and executor failure compensation
- **Purpose:** 确认调度失败不会留下静默 queued Job，production execution failure 会写入可诊断错误。
- **Related changes:** `ArchiveApplicationService::submit_and_schedule_persisted`、`schedule_persisted`、SQLite Job/Event persistence。
- **Prerequisites:** Windows 可写和不可写数据库路径、可控 Sidecar 启动失败 fixture、可查询 SQLite 的 portable workspace。
- **Steps / command:** 使用不可创建的 database path 提交 Job；使用缺失/不可启动的 Sidecar 提交 Job；重新打开数据库并查询 Job state、`last_error_code`、`last_error_message` 与事件；记录应用日志和线程/进程退出情况。
- **Expected result:** 无效数据库路径返回 `EXECUTOR_SCHEDULE_FAILED` 且 Job 为 `FAILED`；后台 executor/factory failure 最终为 `FAILED`，错误码为 `EXECUTOR_WORKER_FAILED` 或更具体的 production error；存在 `DOWNLOAD_FAILED` 事件；没有永久 queued 且无错误的 Job。
- **Priority:** P1
- **Manual interaction required:** yes
- **Status:** `WINDOWS_VERIFICATION_PENDING`

### BLOCKED_AUTOMATION 手工替代步骤

如果 WDIO/WebView2 native session 因 `DevToolsActivePort`、WebView2、driver 生命周期或 GUI automation target 不可用而 BLOCKED：

1. 记录 Windows 版本、架构、WebView2、Node、Rust/Tauri、artifact revision 和 SQLite 路径。
2. 启动最新 portable artifact，使用 PowerShell Stopwatch 记录 Browser request 与 Tauri invoke 的返回耗时。
3. 使用相同 Tweet fixture 分别提交两条入口，记录 JSON response、Job ID、state、query 结果和事件列表。
4. 关闭/禁用 Sidecar 后重复提交，确认 failure code、`FAILED` 状态和 `DOWNLOAD_FAILED` 事件可见。
5. 使用不可写目录或缺失父目录测试 database path，确认 `EXECUTOR_SCHEDULE_FAILED` 和错误日志；恢复可写路径后确认后续新 Job 可正常提交。
6. 执行 cancel 后重启应用，确认主动取消的 Job 不被 recovery 自动重新执行；保留 SQLite、日志、PowerShell 输出、截图和进程列表。

若缺少 Windows artifact、Sidecar fixture、Named Pipe backend 或测试账号，则项目标记 `BLOCKED`/`NOT RUN`，说明具体缺失前置，不得用 Linux Unix socket、fake executor 或静态检查替代 Windows 结论。

### Build / Toolchain

- **ID:** W-HANDOFF-BUILD-01
- **Test name:** Windows PyInstaller worker artifact
- **Purpose:** 生成真正可分发的 `xarchive-downloader.exe`，确认 worker 可启动。
- **Related changes:** `sidecar/pyinstaller/entrypoint.py`、`sidecar/pyinstaller/xarchive-downloader.spec`、`.github/workflows/windows-worker-artifact.yml`。
- **Prerequisites:** Windows runner、Python 3.12、网络、PyInstaller。
- **Steps / command:** 运行 GitHub Actions `Windows Sidecar Worker Artifact`；下载 zip；解压后执行 `xarchive-downloader.exe --help`；记录 `Get-FileHash -Algorithm SHA256`、文件清单和 Python/PyInstaller 版本。
- **Expected result:** 生成目录中存在 worker executable；`--help` 返回 0；artifact 可复制到 portable 包的 `sidecar/xarchive-downloader/`。
- **Priority:** P0
- **Manual interaction required:** yes
- **Status:** `BLOCKED`（当前 Linux 无 Windows bootloader/MSVC；workflow 尚未执行）。

### Runtime

- **ID:** W-HANDOFF-RUNTIME-01
- **Test name:** Full/Core worker startup and gallery-dl argument propagation
- **Purpose:** 确认 worker handshake、`--gallery-dl` 参数和无控制台启动行为。
- **Related changes:** `runtime.rs`、`commands.rs`、`sidecar/src/xarchive_downloader/__init__.py`。
- **Prerequisites:** Windows worker artifact、Full/Core portable 目录、官方 gallery-dl.exe、Windows WebView2。
- **Steps / command:** 启动 Full/Core `.exe`；检查 Sidecar `hello → ready`；在 Core 设置页配置 gallery-dl；执行一次受控下载；检查日志和进程命令行。
- **Expected result:** worker 只使用 XArchive worker 协议；gallery-dl 作为独立 executable 被传入；无额外 Python/venv 依赖、无控制台窗口、错误不泄露原始 secret/stderr。
- **Priority:** P0
- **Manual interaction required:** yes
- **Status:** `BLOCKED`（依赖 W-HANDOFF-BUILD-01 和真实 gallery-dl.exe）。

### Filesystem

- **ID:** W-HANDOFF-FS-01
- **Test name:** Portable paths and Core Extension import
- **Purpose:** 验证中文、空格、移动目录、只读目录和导入回滚。
- **Related changes:** `portable.rs`、`config.rs`、`commands.rs`、`build-portable-windows.mjs`。
- **Prerequisites:** Core portable 包、有效/无效 Extension fixtures、可写和只读目录、第二盘符（如可用）。
- **Steps / command:** 将 portable 目录放到 `C:\测试 目录\XArchive` 和第二盘符；导入有效 Extension；再导入缺 manifest、缺 `src/background.js`、缺 `src/content.js` 的目录；将目标目录设为只读后重复导入；检查 `config.yaml`、backup 和 temporary 目录。
- **Expected result:** 相对路径基于 portable root；有效导入原子替换；无效导入保留旧版本；失败后 backup 可恢复；不发生路径逃逸或未知特殊文件复制。
- **Priority:** P0
- **Manual interaction required:** yes
- **Status:** `BLOCKED_AUTOMATION`（Linux 无 Windows 文件权限/reparse 语义）。

### Integration

- **ID:** W-HANDOFF-INTEGRATION-01
- **Test name:** Edge/Chrome Extension and Native Host integration
- **Purpose:** 验证 Extension 加载、Native Host、浏览器请求和断线重连。
- **Related changes:** `extension/`、Native Host crates、Desktop transport/commands。
- **Prerequisites:** Edge/Chrome、Extension 目录、Native Host manifest、Windows Named Pipe/Registry backend、测试页面或受控 X 账号。
- **Steps / command:** 注册 host manifest；分别在 Edge/Chrome 加载解压 Extension；打开受控页面；执行归档请求、重复 request_id、非法 URL、断线重连和退出；检查 Registry/Named Pipe ACL。
- **Expected result:** 合法请求与 response 的 `request_id` 匹配；非法请求被拒绝；无跨连接串线；无关用户无法访问业务 pipe；浏览器重载后可恢复。
- **Priority:** P0
- **Manual interaction required:** yes
- **Status:** `BLOCKED`（当前 Linux 没有 Windows Named Pipe/Registry backend 实机条件）。

### Packaging

- **ID:** W-HANDOFF-PACKAGING-01
- **Test name:** Full/Core portable package boundaries and release artifacts
- **Purpose:** 验证目录布局、manifest、artifact、许可证和发布可复制性。
- **Related changes:** `build-portable-windows.mjs`、`portable-package.mjs`、worker workflow、Extension。
- **Prerequisites:** Windows Desktop release `.exe`、worker zip、gallery-dl.exe、Full/Core 输出目录、许可证和版本信息。
- **Steps / command:** 分别执行 `$env:PORTABLE_PACKAGE_TYPE='full'; npm run build:portable:windows --workspace desktop` 和 `$env:PORTABLE_PACKAGE_TYPE='core'; npm run build:portable:windows --workspace desktop`；解析 `package-manifest.json`；用 `Get-ChildItem -Recurse` 检查目录；记录各 artifact SHA-256。
- **Expected result:** Full 包含 worker、gallery-dl、Extension；Core 不包含 gallery-dl/Extension 但包含 worker；两者 manifest 与目录一致；不预创建 `download/`；无未知凭据或缓存。
- **Priority:** P0
- **Manual interaction required:** no
- **Status:** `BLOCKED`（缺少真实 Windows Desktop/worker artifact）。

### Regression

- **ID:** W-HANDOFF-REGRESSION-01
- **Test name:** WebView2 GUI, logs, SQLite, no-console and release regression
- **Purpose:** 汇总验证启动、设置、日志、数据库、DPI、键盘和子进程行为。
- **Related changes:** `main.jsx`、settings/pages、`logging.rs`、`runtime.rs`、Windows process flags。
- **Prerequisites:** 可启动 Windows portable 包、WebView2、100/125/150% DPI、可选 Narrator/NVDA、真实或 fixture worker。
- **Steps / command:** 启动全新 portable 目录；不选择下载目录检查 SQLite；进入设置/日志页；测试 Tab/Shift+Tab、Enter/Escape、Focus-visible、复制/打开目录、日志筛选和轮转；观察 Sidecar/aria2/worker 全流程是否弹控制台；最后重启应用。
- **Expected result:** 无白屏、无数据库初始化错误、日志可读且按配置轮转、路径和剪贴板正确、无控制台闪现、退出无残留进程、DPI 和辅助技术无阻塞缺陷。
- **Priority:** P1
- **Manual interaction required:** yes
- **Status:** `BLOCKED_AUTOMATION`（依赖 Windows WebView2/native GUI target；自动化不可用时按 BLOCKED-02 手工执行）。

本 handoff 中没有 `WINDOWS_VERIFICATION_BLOCKING` 项；所有阻塞项均不阻塞后续 Linux 开发。当前 Linux Python 环境缺少 `PyYAML`，因此 GitHub Actions YAML 仅完成结构化静态核对，未声称经过 YAML parser 验证。

### BLOCKED-01：真实 Edge/X、Credential Manager、Telegram

- **Status:** `BLOCKED`
- **Reason:** 缺少受控测试账号、Edge profile、Windows secret backend 和外部服务授权。
- **Manual steps:** 准备专用测试账号和空白 Edge profile；设置项目 `.venv\\Scripts\\python.exe`；启动 Desktop/Sidecar；执行无媒体、单媒体、多媒体、Quote/Reply、重复 Job、认证失败、限流和应用重启；检查 SQLite、stdout/stderr、WebView、日志和 staging 中不出现 Cookie/token/secret；Telegram 使用测试 chat 验证 save/read/delete、重启和 retry。
- **Expected:** 认证数据不泄露，Job/文件/SQLite/Telegram 状态一致，失败可重试且不重复发送。

### BLOCKED-02：GUI/WebView2/DPI/辅助技术

- **Status:** `BLOCKED`
- **Reason:** 依赖 Windows WebView2、DPI、屏幕阅读器和可用 GUI automation target。
- **Manual steps:** 启动 Tauri Debug；设置 100%、125%、150% DPI；测试最小窗口、Tab/Shift+Tab、Enter/Escape、Focus-visible、错误状态和 Job 列表；使用 Narrator/NVDA 检查角色、名称、状态、焦点和对比度；保存截图/录屏及工具错误。
- **Expected:** 无布局截断、焦点丢失、不可操作控件或辅助技术缺失。

### BLOCKED-03：Native Pipe/Registry/浏览器安装

- **Status:** `BLOCKED` until final Windows backend/artifact exists; otherwise `NOT RUN`.
- **Reason:** Named Pipe server、manifest、Registry 和安装权限是 Windows-specific；当前 Linux 不能提供实机结论。
- **Manual steps:** 若 artifact 可用，注册 host manifest，使用 `\\.\\pipe\\xarchive-v1`；管理员/普通用户分别测试启动、request/response、request_id、多连接、断线重连、非法消息、权限拒绝和退出。若 backend/manifest 未提供，记录缺失前置并跳过，不把 framing 单测记为 Windows PASS。
- **Expected:** 合法请求正确路由，非法/越权请求失败，无串线、死锁或残留进程。

### BLOCKED-04：Installer/externalBin/signing/updater/Tray

- **Status:** `BLOCKED` until artifact/certificate exists; otherwise `NOT RUN` 或 `NOT APPLICABLE`。
- **Reason:** 当前发布 artifact、签名证书或 bundle 前置不足。
- **Manual steps:** 若 artifact 可用，执行全新安装、覆盖升级、自定义非 ASCII 路径、卸载、签名/SmartScreen、失败回滚、数据保留、Tray、Single Instance 和 Autostart；否则记录 `bundle.active=false`/artifact/证书缺失并跳过。
- **Expected:** 安装、升级、卸载、回滚、资源定位和数据保留符合发布要求。

## 当前基线（2026-09-09）

| 项目 | 最新结果 | 状态 |
|---|---|---|
| Node workspace | `npm ci` 成功安装 70 个依赖并审计为 0 个漏洞；`npm run check`、`npm run test`、`npm run build` 通过；Desktop 无 Node 测试用例，Extension 6 个测试全部通过。npm 提示 `esbuild` postinstall script 尚未批准 | 已完成基础验证；安装脚本警告已记录 |
| Rust workspace | 当前 Windows `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`、严格 `cargo clippy --workspace --all-targets -- -D warnings`、Debug/Release 编译和 `npm run build:tauri` 均通过，共 69 个 crate 单元测试通过；Linux let-chain 修复已完成 Windows re-validation | 基础验证和 clippy 已完成 |
| Rust 测试稳定性 | 一次并行验证中 `xarchive-storage::completes_archive_directly_from_sidecar_result` 偶发报 Windows 路径不存在；目标测试单独重跑及串行完整 workspace 均通过 | 需后续观察 |
| Rust 格式 | Windows `cargo fmt --check` 通过 | 已完成 |
| Rust lint | `large_enum_variant`、Telegram formatter 的 `single_char_add_str` 和 Tauri aria2 路径扫描的 `clippy::collapsible_if` 均已修复；Windows 严格 workspace clippy re-validation 通过 | 已完成 |
| Python Sidecar | `.venv` + editable 安装，gallery-dl 1.32.11，10 个测试全部通过 | 已完成基础验证 |
| Sidecar 路径兼容 | 中文、空格、Unicode 路径下完成 JSONL `ready → started → log → failed` 流程 | 已完成基础验证 |
| 示例 X URL | 返回 `EXTRACT_OR_DOWNLOAD_FAILED` | 已记录，不能视为认证下载成功 |
| Edge Cookie/真实 X | 尚未使用明确账号环境验证 | 外部账号环境阻塞 |
| Native Messaging framing | Chromium 4 字节 little-endian framing、1 MiB payload 限制、JSON 读写和错误边界已在跨平台 Rust crate 中实现并测试 | 跨平台代码已完成，Windows Edge/Chrome 实机待验证 |
| Named Pipe | 对应 Windows transport 尚未实现 | 待开发，不是测试失败 |
| Retry/TagEngine/用户目录 | retry/backoff、TagEngine、Windows-safe 用户目录名和 users/user_names/tags/tweet_tags Repository 已在跨平台 Rust 中实现并测试 | 跨平台代码已完成，Windows 文件系统/并行故障注入待验证 |
| Telegram contract | SecretStore abstraction、Bot API request models、metadata formatter、UTF-8 continuation、media group 分组和 `reqwest 0.13.4` + Rustls HTTPS transport 已在跨平台 Rust 中实现并测试；fake-server 已覆盖四种 Bot API 方法及 HTTP/API 错误；发送状态持久化与幂等补传已作为跨平台代码实现并通过 Linux 测试 | 跨平台 transport 与发送状态持久化已完成；Windows 平台验证 `WINDOWS_VERIFICATION_PENDING`；Credential Manager 和真实账号发送仍待平台/账号验证 |
| Windows 构建依赖 | Visual Studio BuildTools/MSVC、Windows SDK、MSBuild、WebView2 可用；`aria2c`、`cmake`、`ninja` 不在 PATH | 工具链已完成，aria2c artifact/进程集成待实现 |
| Tauri Desktop 脚手架 | Tauri CLI 2.11.4 已由项目依赖安装；Windows `npm run dev:tauri` 已启动 Vite、Rust Debug 和 Desktop 可执行文件，`npm run build:tauri` 已生成 Release 可执行文件；当前未启用 bundle，真实 externalBin/安装包仍未配置 | 开发启动/构建已完成；GUI/打包待验证 |
| 执行过程错误与警告 | 首次未设置 `PYTHON` 时 Rust Supervisor 两个真实 Worker 测试因默认 `python3` 不存在而报 `NotRunning`，显式使用项目 `.venv\Scripts\python.exe` 后复验通过；Rust/Tauri 构建输出 MSVC linker stdout `#[warn(linker_messages)]` 非阻塞警告；`npm ci` 提示 `esbuild` postinstall script 尚未批准；停止 Tauri 开发进程时出现 Chromium `Error = 1411` 注销警告 | 已处理环境错误；其余为不阻塞警告 |

## 上一轮 Windows 平台验证记录（2026-09-09）

### Validation Environment

| 项目 | 实际值 |
|---|---|
| Linux source branch/revision | `main` / `f70f91399a0866ca8ee35741481e805a71556dc7`，包含 working tree changes |
| Linux working tree | 验证开始前已存在 `README.md`、`aidlc-docs/aidlc-state.md`、`crates/xarchive-download/src/lib.rs`、多份开发/架构文档的未提交修改，以及未跟踪的 `AGENTS.md`、`docs/development/cross-platform-validation.md`、`docs/validation/`；本轮仅修改本验证文档，未修改业务代码 |
| Windows 工作副本 | `E:\Shiraishi\VSCode Workspace\Tw2Tg` |
| Windows 系统/架构 | Windows 11 Insider Preview `10.0.29661` / 64 位 |
| 运行时/工具链 | Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、rustfmt/clippy `1.9.0`、Python `3.14.7`、Tauri CLI `2.11.4` |
| 验证日期 | 2026-09-09 |

同步方向为 Linux source → E: Windows validation workspace。同步排除了 `.git`、`target`、`node_modules`、`.venv`、`dist` 和缓存/数据库文件，未删除 E 盘额外文件；同步后 `rsync --checksum` 内容校验无差异。

### Validation Results

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Node 依赖 | PASS | `npm ci`；安装 70 个依赖，审计 0 个漏洞 |
| Node 检查/测试/构建 | PASS | `npm run check`、`npm run test`、`npm run build`；Desktop 0 项 Node 测试，Extension 6 项全部通过 |
| Rust 格式/编译 | PASS | `cargo fmt --all -- --check`、`cargo check --workspace` |
| Rust 全量测试（环境修正后） | PASS | 当前进程 `PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe` 后执行 `cargo test --workspace`；60 个 crate 单元测试全部通过，含 Telegram 8 项测试 |
| Rust clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings` |
| Python Sidecar 测试 | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Sidecar Windows 路径/JSONL 进程链路 | PASS | 项目 `.venv` 在含空格、中文和 `Ω` 的临时路径启动；输出 `ready → started → failed` 及 `INVALID_JSON`，进程 exit 0 |
| Tauri CLI/Release 构建 | PASS | `npm exec --workspace desktop -- tauri --version`；`npm run build:tauri`；生成 `target\release\xarchive-desktop.exe` |
| Tauri 开发启动 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 `target\debug\xarchive-desktop.exe` |
| Tauri GUI 视觉验收 | BLOCKED | GUI 自动化 helper 未提供可用的 native app target；仅确认启动日志，未将其当作 GUI 验收通过 |

### Errors

| 失败/警告 | 分类与原因 | 对后续验证的影响 |
|---|---|---|
| 默认 `cargo test --workspace` 的两个 Supervisor 测试报 `NotRunning` | Windows 环境配置/测试 harness 前提：测试默认调用 `python3`，但 Windows PATH 中不存在；设置当前进程 `PYTHON` 指向项目 `.venv\Scripts\python.exe` 后复验通过 | 不阻塞其他测试；后续开发事项是让 Windows 测试显式配置 Python 或改善默认探测，本次不修改代码 |
| `npm ci` 提示 `esbuild@0.28.2` postinstall script 未被 `allowScripts` 批准 | 依赖安装安全策略警告；本轮构建和测试均通过 | 不阻塞当前验证；是否批准该脚本需后续依赖策略决定 |
| Rust/Tauri 构建输出 MSVC linker stdout `#[warn(linker_messages)]` | 工具链非阻塞 warning，未导致构建失败 | 不阻塞 |
| 停止 Tauri 开发进程时出现 Chromium `Error = 1411`，进程返回 `STATUS_CONTROL_C_EXIT` | 主动 Ctrl+C 停止开发进程时的窗口类注销/终止警告；启动阶段已正常运行 | 不影响启动验证；GUI 视觉状态仍未确认 |

### Not Executed / Blocked / Not Applicable

| 验证项目 | 状态 | 原因 |
|---|---|---|
| Edge Cookie、真实 X 认证和真实媒体归档 | BLOCKED | 缺少明确账号/Profile；公开示例 URL 的失败链路不能替代认证验收 |
| Named Pipe transport、Native Host Registry/manifest、Edge/Chrome 实机 Extension | NOT RUN | Named Pipe/Registry 相关 Windows 集成功能尚未实现；浏览器实机链路依赖这些前置项 |
| Tauri GUI 前后端交互、视觉和资源路径人工验收 | BLOCKED | GUI 自动化 helper 无法提供可用 native app target；本轮只能确认进程启动日志 |
| Tauri bundle、安装器、升级/卸载、真实 externalBin Sidecar | NOT RUN | `bundle.active=false`，externalBin 和安装器 artifact 尚未配置/实现 |
| Windows 长路径、文件锁、磁盘不足、Tray/Single Instance/Autostart、Credential Manager | NOT RUN | 项目功能或 Windows backend 尚未实现，不能用基础构建结果替代专项验收 |
| aria2 管理 UI/检测/版本 allowlist | LINUX_VERIFIED | Desktop 已增加 PATH、程序目录、应用数据目录检测、版本显示、官方版本选择、SHA-256 allowlist 和下载按钮；Linux 已通过 Desktop Rust/Vite 构建验证 |
| aria2c executable/进程集成 | WINDOWS_VERIFICATION_PENDING | 本轮同步的 Windows 副本已包含当前 Linux 的 `Aria2Supervisor` 和 aria2 管理 UI，但 Windows 环境中 `aria2` 不在 PATH，也未发现项目提供的 `aria2c.exe`；因此未执行真实下载、PowerShell 解压、进程生命周期、大文件断点、崩溃恢复、`.aria2` 清理、Unicode staging 路径和 403 回退 gallery-dl 验证 |
| 真实 Telegram 账号发送与持久化 | BLOCKED | 真实账号/网络发送环境及发送状态持久化验收尚未提供；本轮仅覆盖 HTTPS contract/fake-server 单元测试 |
| Linux-only 的 pytest/cargo-clippy 重新执行 | NOT APPLICABLE | 本任务目标是 Windows validation；Windows 对应的 pytest 和 clippy 已实际执行 |

本节记录 2026-09-09 从当前 Linux working tree 同步后的 Windows 复验。默认 `python3` 测试探测、GUI automation target、Named Pipe/Registry、externalBin/安装器和 aria2c.exe Windows 集成仍应作为后续开发/验证事项处理。

### Linux Follow-up

| 后续事项 | 原因 |
|---|---|
| Windows Rust Supervisor 测试的 Python 前置配置 | 默认探测 `python3` 在 Windows PATH 中不可用；本轮需显式设置项目 `.venv\Scripts\python.exe` 才能通过，建议后续开发/测试流程明确 Python 解析规则 |
| `aria2c.exe` Windows 实际集成 | 当前 Linux 的 supervisor 修改已同步，但 Windows 缺少可执行文件；需提供或安装受控 artifact 后再验证版本/hash、生命周期、断点、崩溃恢复、`.aria2` 和 Unicode 路径 |
| GUI、Named Pipe/Registry、externalBin/安装器和真实账号链路 | 分别受 UI automation target、尚未实现的 Windows backend、未配置发布 artifact 和凭据/外部服务限制 |

本轮未为通过验证而修改业务代码、依赖或系统设置；以上事项作为后续 Linux 开发/验证任务保留。

### Linux Reconciliation（2026-09-09）

依据最新 Windows 结果和 `docs/development/cross-platform-validation.md` 重新评估当前 Plan：

| 事项 | 状态 | 当前事实 |
|---|---|---|
| Windows Rust fmt/check/test/完整 clippy、Node、Sidecar 基础链路、Tauri Debug/Release 构建 | WINDOWS_PASS | 已在 Windows 实际执行并通过；Supervisor 测试需要显式使用项目 `.venv\\Scripts\\python.exe` |
| Telegram HTTPS transport | LINUX_VERIFIED | Linux 已实现 `reqwest 0.13.4` + Rustls transport，并通过 fake-server 测试；真实账号发送仍为 `WINDOWS_BLOCKED`/账号环境事项，不因 Linux 测试改写为 Windows PASS |
| Telegram 发送状态持久化与幂等补传 | 单元层 WINDOWS_PASS / 应用层 WINDOWS_VERIFICATION_PENDING / 真实账号 BLOCKED | Windows 当前 revision 的 `cargo test --workspace` 通过 69 项，其中 storage 16 项、telegram 12 项覆盖状态往返、重试计数、迁移重开和幂等发送（实际执行，单元层可记 PASS）；但现有测试使用 in-memory SQLite，基于文件的 SQLite Windows 路径行为、应用重启现场恢复和 0001→0002 迁移升级仍需专项验证，不得整体标记 PASS；真实账号发送、Credential Manager 另受账号/Windows backend 限制 |
| aria2 supervisor core | LINUX_VERIFIED | Linux 已实现并验证配置校验、aria2 参数构造、进程启动失败映射、RPC 就绪检查和 secret 脱敏 |
| aria2c.exe Windows 实际集成 | WINDOWS_VERIFICATION_PENDING | 本轮已将当前 Linux working tree 同步到 Windows 工作副本；UI 和 Rust 下载管理命令已实现，但环境中没有 `aria2c.exe`，且尚未执行官方 ZIP 下载/PowerShell 解压；仍需提供受控 artifact 后验证版本/hash、安装目录检测、进程生命周期、断点、崩溃恢复、`.aria2` 清理、Unicode staging 和 403 回退 |
| Windows GUI 视觉验收 | WINDOWS_BLOCKED | GUI automation helper 仍未提供可用 native app target |
| Edge Cookie、真实 X、Named Pipe、Registry、externalBin、安装器、Credential Manager、Tray/Autostart | WINDOWS_BLOCKED / NOT_RUN | 依赖账号、Windows backend、发布 artifact 或尚未实现的前置功能 |

本轮 Linux 开发修改了 `crates/xarchive-download/src/lib.rs` 及相关 Plan/架构文档，未修改 Windows 工作副本代码；本轮已重新同步并执行可用的 Windows 验证。Linux 端已执行相关 regression tests；aria2c.exe 相关项目仍保持 `WINDOWS_VERIFICATION_PENDING`，直到实际进程集成验证通过。

---

## Linux Reconciliation（2026-09-10）

依据最新 Windows 结果和 `docs/development/cross-platform-validation.md` 重新评估当前 Plan。本轮 Linux 端完成了以下 bug 修复和清理工作：

### 本轮 Linux 变更

| 变更 | 文件 | 说明 |
|---|---|---|
| `record_event` SQL 参数修复 | `crates/xarchive-storage/src/lib.rs` | 修复 `INSERT INTO events` 语句缺少 `params!` 宏导致的 SQL 执行失败 |
| `archive_tweet` 所有权重构 | `desktop/src-tauri/src/lib.rs` | 修复 `database` 在闭包中移动后再次使用的编译错误，正确处理 `ArchiveService` 所有权转移 |
| `stop_sidecar` SidecarCommand 初始化 | `desktop/src-tauri/src/lib.rs` | 添加缺失的 `executable`, `browser`, `profile` 字段 |
| 未使用 import 清理 | `desktop/src-tauri/src/lib.rs` | 移除未使用的 `xarchive_core::JobState` 导入 |
| 跨平台验证规则更新 | `AGENTS.md`, `docs/development/cross-platform-validation.md`, `docs/validation/windows.md` | 新增"批量开发、集中验证"工作流规则 |
| Windows Validation Queue 整理 | `docs/development/windows-validation.md` | 新增 11 个队列项目，全部 `WINDOWS_VERIFICATION_PENDING` |

### 当前验证状态

| 项目 | 状态 | 说明 |
|---|---|---|
| Linux 编译 | PASS | `cargo check --workspace` 无错误无警告 |
| Linux 单元测试 | PASS | 79 个 crate 单元测试全部通过（11 core + 5 desktop + 16 download + 8 Native Host + 7 protocol + 4 supervisor + 16 storage + 12 Telegram） |
| Linux 格式检查 | PASS | `cargo fmt --all -- --check` 通过 |
| Windows 基础工具链 | WINDOWS_PASS | 已在 2026-09-09 Windows 验证中确认通过 |
| Windows 单元测试 | WINDOWS_PASS | 69 个 crate 单元测试通过（环境修正后） |
| Windows GUI 视觉验收 | WINDOWS_BLOCKED | GUI automation helper 不可用 |
| Edge Cookie/真实 X | WINDOWS_BLOCKED | 缺少测试账号 |
| Named Pipe/Registry | NOT RUN | 功能尚未实现 |
| aria2c.exe Windows 集成 | WINDOWS_VERIFICATION_PENDING | Windows 环境中无 aria2c.exe |
| Telegram 真实账号 | WINDOWS_BLOCKED | 缺少账号/网络环境 |

### Windows Validation Queue 更新

本轮 Linux 变更未新增 Windows 验证项目。现有 11 个队列项目保持 `WINDOWS_VERIFICATION_PENDING`，无 `WINDOWS_VERIFICATION_BLOCKING`。

### 下一步

根据 deferred Windows validation 规则：
1. 当前无 Linux-only 开发项需要继续
2. 所有非 Windows-dependent 开发工作已完成
3. Linux 端验证已全部通过
4. Windows 验证项目已累计记录，等待集中执行

当前可进入 Windows Validation Preparation 阶段，但无新的 blocking 项需要立即处理。

---

## P0：运行链路

### W-P0-01 工具链

**验证方式：** Windows 实机 + Windows CI；**状态：** 基础构建、测试和 lint 完成，GUI/打包仍待验证。

确认 Rust/Cargo、rustfmt、clippy、Node/npm、Python、Visual Studio C++ Build Tools、Windows SDK 和 x64 target 可用。开发阶段 `icons/icon.ico` 已补齐；Windows workspace 的 rustfmt、check、test、完整 clippy、Debug/Release 和 Tauri Release 构建均通过。Supervisor 测试首次因默认 `python3` 不在 Windows PATH 而报 `NotRunning`，改用项目 `.venv\Scripts\python.exe` 后通过。E 盘首次 Node 检查因缺少 `vite`，本轮执行项目内 `npm ci` 后复验通过；npm 另提示 `esbuild` postinstall script 尚未批准。

```powershell
rustc --version
cargo --version
rustfmt --version
cargo clippy --version
node --version
npm --version
py --version
```

最低验收：

```text
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
npm run check
npm run test
npm run build
```

### W-P0-02 Python Sidecar

**验证方式：** Windows 实机；**状态：** 基础测试和本地进程链路已完成，真实 X 提取和打包待实现。

验证 `.venv`、editable 安装、Worker 启动、`hello`、`download`、`shutdown`、stdout JSONL、stderr 日志、退出码，以及工作目录含空格/中文/Unicode 时的行为。

验收：Rust Supervisor 能启动 Worker；`hello → ready`、`download → started → complete/failed`、`shutdown → exit` 全部成立；不依赖全局 Python 包。当前已使用项目 `.venv`、gallery-dl 1.32.11 验证真实 sidecar 在含中文、空格和 Unicode 的工作路径中输出 JSONL；示例 URL 因 X 提取错误返回 `EXTRACT_OR_DOWNLOAD_FAILED`，真实账号下载仍待验证。

### W-P0-03 Edge Profile/Cookie

**验证方式：** Windows Edge 实机；**状态：** 待验证，依赖明确账号环境。

验证 Edge `Default` Profile、浏览器运行中/关闭后的 Cookie 读取、无效 Profile、Cookie 失效、登录可见内容、敏感内容和受保护账号内容。

安全验收：Cookie 不进入 Extension、Rust IPC、SQLite、日志、`tweet.json` 或 aria2 参数；失效映射为 `AUTH_REQUIRED`。

### W-P0-04 真实 X 本地归档

**验证方式：** Windows 实机；**状态：** 待验证，依赖 Edge Cookie 和可用 X 账号。

```text
真实 X URL → gallery-dl → Python Sidecar → Rust Supervisor
→ ArchiveService → SQLite → staging → 最终目录
```

至少覆盖：无媒体、单图、多图、视频、图文混合、Quote、Reply、不可访问 Tweet、重复点击、下载中关闭 Desktop、本地文件已存在。

验收：Tweet ID 幂等；文件可打开；JSON/TXT 正确；大小和 SHA-256 由 Rust 实际校验；重启后 Job 可恢复。

---

## P1：Desktop 与浏览器集成

### W-P1-01 Tauri Desktop

**验证方式：** Linux 开发环境 + Windows 实机/CI；**状态：** CLI 入口、Debug/Release 编译和开发启动已完成，Windows GUI/打包待验证。

当前已完成：Vite/React 前端、Tauri 2 Rust crate、状态/Job/目录/Sidecar commands、启动时 SQLite 初始化、Sidecar `hello → ready` 握手、最近 Job 查询、跨平台打开归档目录、本地 shadcn/ui 组件和 Dashboard、基础 capabilities、开发阶段 PNG/ICO 图标、Windows Debug/Release Rust 构建、项目内 Tauri CLI 入口，以及 Windows `npm run dev:tauri` 启动和 `npm run build:tauri` Release 构建。启动日志确认 Vite、Rust Debug 和 Desktop 可执行文件均启动；停止开发进程时出现 Chromium `Error = 1411` 注销警告，最终以 Ctrl+C 终止。当前尚未配置真实 `externalBin` Sidecar；仍需验证前后端通信、资源路径、打包后 Sidecar 启动、安装到含空格/非 ASCII 路径及非系统盘。UI 自动化 helper 初始化失败，因此本轮未完成 GUI 视觉和安装器验证。

当前 `bundle.active=false`，图标仍为开发阶段 PNG/ICO 资源；正式打包前必须替换正式图标集、启用 bundle 并完成安装器测试。

### W-P1-02 Named Pipe

**验证方式：** Windows 实机；**状态：** Windows transport 待实现。

目标：

```text
\\.\pipe\xarchive-v1
```

验证 Server 启动、Native Host 连接/重连、多连接、request_id 路由、批量 `query_status`、Desktop 退出、ACL、消息大小限制、非法 JSON/协议版本/action 拒绝。

### W-P1-03 Native Messaging Host

**验证方式：** 跨平台代码测试 + Windows Edge/Chrome 实机；**状态：** 跨平台代码已完成，Windows 集成待验证。

跨平台已验证 Chromium 长度前缀 framing、stdin/stdout 二进制读写、stdout 机器协议、stderr 诊断和大 payload 拒绝；Windows 仍需验证 origin allowlist、Host manifest、Desktop 离线、Host 反复启动/关闭和实际 Edge/Chrome 连接。

### W-P1-04 Registry 与 Host manifest

**验证方式：** Windows 实机；**状态：** 待实现。

确认 Chrome/Edge 注册路径、固定 Extension ID、安装/升级/卸载、管理员/非管理员权限、安装目录移动、路径含空格时的行为。

### W-P1-05 MV3 Extension

**验证方式：** Windows Edge/Chrome 实机；**状态：** Host/Extension 跨平台代码已完成，浏览器集成待验证。

跨平台代码已覆盖 Tweet ID/URL/metadata 提取、MutationObserver、按钮去重、Service Worker request_id 路由、Native Host 断线错误处理和最小消息边界；Windows 仍需验证开发版加载、Extension ID、Timeline、Tweet Detail、SPA 路由、虚拟滚动、多标签同步、Service Worker 重启和 Native Messaging 重连。

场景：

```text
x.com / twitter.com / Timeline / Detail / Quote / Reply
刷新 / 多标签 / Desktop 未启动
```

### W-P1-06 aria2c

**验证方式：** Linux/跨平台 fake server + Windows 实机/CI；**状态：** 基础跨平台 supervisor 已实现并完成 Linux 验证，Windows aria2c.exe 集成和恢复验证待执行（`WINDOWS_VERIFICATION_PENDING`）。

跨平台已验证 loopback HTTP RPC、Secret 参数、`addUri/tellStatus/pause/unpause/remove` 请求和响应、HTTP/RPC 错误映射，以及 `Aria2Supervisor` 的配置校验、aria2 启动参数、进程启动失败映射和 RPC 就绪检查；仍需在 Windows 验证随应用提供的 `aria2c.exe`、版本/hash、进程生命周期、大文件断点、崩溃恢复、`.aria2` 清理、Unicode staging 路径和 403 回退 gallery-dl。

优先使用本地 HTTP 测试服务器，不直接依赖 X CDN。

---

## P1：文件系统与生命周期

### W-P1-07 Windows 文件系统

**验证方式：** Windows 实机；**状态：** 逻辑已测试，Windows 待验证。

覆盖系统盘/非系统盘、空格、中文/Unicode、Windows 保留字符和文件名、`CON/PRN/AUX/NUL`、超长路径、磁盘不足、文件锁、目标目录已存在、遗留 staging 和异常退出恢复。

### W-P1-08 Tray/Single Instance/Autostart

**验证方式：** 跨平台代码测试 + Windows 实机；**状态：** 跨平台规则和抽象已完成，Windows backend/实机待验证。

验证 Tray 启动、关闭隐藏、打开/退出菜单、第二次启动激活已有实例、登录自启动、禁用自启动、后台 Sidecar 工作和关机安全退出。

### W-P1-09 Secret Store

**验证方式：** 跨平台代码测试 + Windows 实机；**状态：** SecretStore abstraction 已完成，Windows backend 待实现/验证。

跨平台已完成 `SecretStore` abstraction、内存测试实现、BotToken 脱敏、Telegram request contract 和基于 `reqwest 0.13.4` + Rustls 的 HTTPS transport；仍需实现 Windows Credential Manager 或 Stronghold backend，验证应用重启读取、删除/更新、日志/SQLite/Extension/Sidecar 隔离和 Windows 用户边界。真实 Telegram 账号发送、API 限制和发送状态持久化仍待账号/业务环境验证。

### W-P1-10 GUI 源码审查结论

**验证方式：** Linux 源码审查 + Windows 实机/CI；**状态：** Linux 源码审查已完成（2026-09-09），结论是当前 GUI 为较高完成度的开发 Dashboard 原型，尚不符合直接视觉验收的正式界面。GUI 核心修补、按平台展示、焦点/键盘可访问性和对比度仍需完成；Windows WebView2/DPI/Narrator/NVDA 真实渲染、Tab/焦点可见性、键盘流程、命中目标和最终对比度验证必须在 Windows 实机或 Windows CI 上完成，不能仅靠 Linux 静态审查替代。

参见 `docs/development/roadmap.md` M6 GUI 的“当前 GUI 设计评估”部分。

### W-P1-11 Tauri GUI 视觉与交互人工验收

**验证方式：** Windows 实机/CI + GUI automation target；**状态：** 待验证，受限于 UI automation target 是否可用。

验证侧栏导航名/键盘聚焦/焦点可见性、错误与成功反馈、加载与空状态、统计语义、aria2 按平台展示、主按钮层级、紧凑断点可操作性、Windows 缩放与显示缩放下的可读性、帮助文本对比度、实时任务列表更新的屏幕阅读器反馈和操作恢复。

Linux 侧仅能通过构建、静态可访问性检查和样式正文推断覆盖这些项，最终验收以 Windows 真实渲染为准。

### Linux GUI redesign reconciliation（2026-09-10）

Linux 已将 Desktop GUI 重设计为白色主色调、Vercel 风格的本地控制台：白色背景、细灰边框、近黑主按钮、浅色语义 Badge、结构化最近任务、运行环境/归档位置卡片、Skeleton 加载状态和更适合 Desktop/DPI 的字号层级。此次修改保留现有 Tauri commands、Widget 级错误隔离/重试、`aria2` Windows gating、`<ul>/<li>`/`<time>` 语义、`role="alert"`、`aria-label`、`:focus-visible` 和 reduced-motion 约束。

Linux 端适用的 Vite check/build、Extension 静态检查、Rust fmt/check/test 已通过。由于本轮没有可用的 Windows native GUI automation target，以下项目不得提前标记为 `WINDOWS_PASS`，当前保持 `WINDOWS_VERIFICATION_PENDING` / `BLOCKED`：WebView2 白底真实渲染、100%/125%/150% DPI、Tab 顺序、键盘操作、Focus-visible、Narrator/NVDA、真实对比度、命中区域、最小窗口布局和中文/Unicode 长路径显示。

---

## P2：安装与发布

### W-P2-01 安装器

**验证方式：** Windows 实机 + CI；**状态：** 待实现。

验证全新安装、覆盖升级、自定义路径、非 ASCII 路径、Sidecar/aria2 资源、Native Host 注册、失败回滚、卸载保留/删除 `X-Archive` 数据。

### W-P2-02 签名与杀毒软件

**验证方式：** Windows 实机/发布环境；**状态：** 待实现。

验证安装包、Sidecar、Native Host 签名策略，SmartScreen、Windows Defender、实时扫描导致的文件锁、重试和日志脱敏。

### W-P2-03 Tauri Updater

**验证方式：** Windows 实机 + CI；**状态：** 待实现。

验证签名更新、更新前关闭子进程、失败回滚、保留数据库/归档、schema migration、Native Host 注册保持有效、组件版本可追踪。

### W-P2-04 第三方许可证

**验证方式：** CI + 发布审核；**状态：** 文档已建立，扫描待实现。

检查 gallery-dl、aria2、Python、Rust crates、npm packages、可选 yt-dlp/ffmpeg 的许可证、安装包副本、源代码获取方式和闭源/商业发行法律审查。

---

## 推荐执行顺序

```text
W-P0-01 → W-P0-02
→ W-P0-03（准备账号/Profile）→ W-P0-04（真实归档）
→ W-P1-01（Windows Tauri 复验）→ W-P1-02 → W-P1-03 → W-P1-04 → W-P1-05
→ W-P1-07 → W-P1-08 → W-P1-09 → W-P1-06
→ W-P2-01 → W-P2-02 → W-P2-03 → W-P2-04
```

如果当前阶段没有可用于 X 的测试账号，W-P0-03/W-P0-04 应保持为“外部环境阻塞”，不要用公开示例 URL 失败结果替代认证验证。

## Windows CI 最低工作流

```powershell
npm ci
npm run check
npm run test
npm run build
cargo check --workspace
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
py -m venv .venv
.\.venv\Scripts\python.exe -m pip install --upgrade pip
.\.venv\Scripts\python.exe -m pip install -e .\sidecar pytest
.\.venv\Scripts\pytest.exe .\sidecar\tests -q
```

rustfmt/clippy 应在项目专用 CI/toolchain 中安装，不要求修改开发者全局工具链。

## Windows MVP 通过标准

- 全部 P0 项目完成。
- W-P1-01 至 W-P1-05 完成并通过。
- Edge Profile 至少完成图片和视频归档验证。
- Desktop 重启不重复下载。
- Native Host 只提供 allowlist 业务能力。
- 本地文件、SQLite 和 Job 状态一致。
- Token、Cookie、RPC Secret 不泄露。
- 安装、升级和卸载不会意外删除用户归档。
- Windows CI 的格式、lint、构建和测试全部通过。

## 本轮 Windows 平台复验（2026-09-09，Linux revision `add84c0`）

### Validation Environment

| 项目 | 实际值 |
|---|---|
| Linux source branch/revision | `main` / `add84c0950436b912671c5a451b2e3090300cb9f`；验证开始前 working tree clean |
| Linux working tree | 验证开始前无未提交修改；本轮仅修改本验证文档，未修改业务代码 |
| Windows 工作副本 | `E:\Shiraishi\VSCode Workspace\Tw2Tg` |
| Windows 系统/架构 | Windows 11 专业工作站版 Insider Preview `10.0.29661` / 64 位 |
| 运行时/工具链 | Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、rustfmt/clippy `1.9.0`、Python `3.14.7`、Tauri CLI `2.11.4` |
| 验证日期 | 2026-09-09 |

同步方向为 Linux source → E: Windows validation workspace。同步排除了 `.git`、`node_modules`、`.venv`、`target`、`dist` 和缓存/数据库文件，也排除了 Linux 端验证结果文档；未删除 E 盘本地依赖和构建产物。同步后对其余项目内容执行 `rsync --checksum`，无差异。

本次复验说明：同步前发现 E 盘副本曾落后于该 Linux revision，因此本轮先重新单向同步，再对最新副本重新执行全部适用命令；结果与既有复验一致，未产生新的 FAIL。

### Validation Results

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Node 依赖 | NOT RUN | 本轮未重复执行 `npm ci`；E 盘工作副本已有依赖且 `package-lock.json` 未变化。上一次成功安装和审计结果保留在当前基线 |
| Node 检查/测试/构建 | PASS | `npm run check`、`npm run test`、`npm run build`；Desktop 0 项 Node 测试，Extension 6 项通过 |
| Rust 格式/编译 | PASS | `cargo fmt --all -- --check`、`cargo check --workspace` |
| Rust 全量测试 | PASS | 当前进程设置 `PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe` 后执行 `cargo test --workspace`；68 个 crate 单元测试全部通过，storage 16 项、telegram 12 项及 Desktop aria2 allowlist/SHA-256 5 项测试通过 |
| Telegram 发送状态持久化与幂等补传单元覆盖 | PASS | Windows workspace 测试通过 `telegram_send_state_round_trip`、重试/未发送列表、已发送状态约束，以及幂等发送的失败重试、已送达稳定性和已发送跳过 transport 测试；真实 Telegram 账号发送仍未执行 |
| Rust clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；Linux let-chain 修复已在 Windows 当前 revision 上复验通过 |
| Python Sidecar 测试 | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Sidecar Windows 路径/JSONL 进程链路 | NOT RUN | 本轮执行了 `.venv\Scripts\pytest.exe sidecar\tests -q` 并通过，但未重复执行独立的 Unicode 路径 JSONL 进程链路；既有通过证据保留在历史基线 |
| Tauri CLI/Release 构建 | PASS | `npm exec --workspace desktop -- tauri --version`、`npm run build:tauri`；生成 `target\release\xarchive-desktop.exe` |
| Tauri 开发启动 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 `target\debug\xarchive-desktop.exe` |
| Tauri GUI 视觉验收 | BLOCKED | GUI 自动化 helper 无可用 native app target；仅确认启动日志 |

### Errors

| 失败/警告 | 分类与原因 | 对后续验证的影响 |
|---|---|---|
| 默认 Python 探测 | 已知环境问题；Windows PATH 中没有 `python3`，本轮在执行 Rust workspace 测试前显式设置项目 `.venv\Scripts\python.exe`，未再复现 `NotRunning` | 不阻塞本轮验证；后续仍应明确 Windows 测试的 Python 解析规则 |
| 受限沙箱直接执行 Node 脚本报 `EPERM: operation not permitted, lstat 'E:\\Shiraishi\\VSCode Workspace'` | Windows 工作区父目录的沙箱访问边界；使用受控权限重新执行后 `npm run check` 通过，不属于项目代码失败 | 不阻塞；后续 Windows 验证需保留该权限前提 |
| 上一次 `npm ci` 提示 `esbuild@0.28.2` postinstall script 未被 `allowScripts` 批准 | 依赖安装安全策略警告；本轮未重复执行 `npm ci`，Node 构建测试通过 | 不阻塞 |
| Rust/Tauri 构建输出 MSVC linker stdout `#[warn(linker_messages)]` | 工具链非阻塞 warning | 不阻塞 |
| 停止 Tauri 开发进程时出现 Chromium `Error = 1411`、`STATUS_CONTROL_C_EXIT` | 主动 Ctrl+C 停止时的窗口类注销/终止警告；启动阶段正常 | 不影响启动结论；GUI 视觉仍未确认 |

### Not Executed / Blocked / Not Applicable

| 验证项目 | 状态 | 原因 |
|---|---|---|
| Tauri GUI 前后端交互、视觉和资源路径人工验收 | BLOCKED | GUI 自动化 helper 无法提供可用 native app target |
| Edge Cookie、真实 X 认证和真实媒体归档 | BLOCKED | 缺少明确账号/Profile；公开 URL 失败不能替代认证验收 |
| Named Pipe transport、Native Host Registry/manifest、Edge/Chrome 实机 Extension | NOT RUN | Windows backend/Registry 前置功能尚未实现 |
| Tauri bundle、安装器、升级/卸载、真实 externalBin Sidecar | NOT RUN | `bundle.active=false`，externalBin 和安装器 artifact 尚未配置 |
| aria2 官方 ZIP 下载、PowerShell 解压、`aria2c.exe` 检测和真实进程集成 | NOT RUN | 当前环境无 `aria2c.exe`，项目也未提供受控 artifact；本轮未下载/安装外部 artifact 或修改系统设置 |
| aria2 断点、崩溃恢复、`.aria2` 清理、Unicode staging、403 回退 gallery-dl | BLOCKED | 依赖上一项真实 `aria2c.exe` 集成通过 |
| Windows 长路径、文件锁、磁盘不足、Tray/Single Instance/Autostart、Credential Manager | NOT RUN | 项目功能或 Windows backend 尚未具备专项验收前置条件 |
| 真实 Telegram 账号发送与持久化 | BLOCKED | 缺少真实账号、凭据和发送状态持久化环境；仅覆盖 HTTPS fake-server contract |
| Linux-only 的 pytest/cargo-clippy 重新执行 | NOT APPLICABLE | 本轮目标为 Windows；Windows 对应 pytest 和 clippy 已实际执行，Linux-only 重复执行不属于本轮范围 |

### Linux Follow-up

| 后续事项 | 原因 |
|---|---|
| Windows clippy re-validation（PASS） | Linux 已将 `desktop/src-tauri/src/lib.rs:83` 的嵌套 `if let` 改为 let-chain；Windows 当前 revision 的严格 workspace clippy 已通过 |
| 提供受控 `aria2c.exe` artifact 并完成 Windows 集成验证 | 当前新增下载管理 UI/命令和 Rust supervisor 仅完成单元/构建层验证，真实下载、解压、版本/hash、生命周期和恢复仍未执行 |
| GUI、Named Pipe/Registry、externalBin/安装器和真实账号链路 | 分别受 UI automation target、尚未实现的 Windows backend、未配置发布 artifact 和凭据/外部服务限制 |

本轮未为通过验证而修改业务代码、依赖或系统设置；以上事项交由 Linux 后续开发/验证任务处理。

### Linux Follow-up after Windows result reconciliation (2026-09-09)

> 注：本节为 Windows clippy re-validation 之前的历史记录，其中 `WINDOWS_VERIFICATION_PENDING` 状态已被下文 "Windows re-validation after Linux clippy fix" 与 "Linux reconciliation after Windows clippy re-validation" 小节更新为已通过；本节内容按原文保留。

| 项目 | 结果 | 说明 |
|---|---|---|
| `candidate_aria2_paths` clippy fix | LINUX_VERIFIED | 将 `desktop/src-tauri/src/lib.rs:83` 的嵌套 `if let` 改为 let-chain；`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace` 通过 |
| Linux Rust workspace clippy | NOT RUN | 当前 Linux toolchain 未安装 `cargo-clippy`；命令返回 `cargo-clippy is not installed for the toolchain stable-x86_64-unknown-linux-gnu` |
| Linux Node workspace | PASS | `npm run check`、`npm run test`、`npm run build` 和 Extension check/test 通过；Extension 6 项测试通过 |
| Linux Python/schema checks | PASS | `python3 -m compileall -q sidecar` 和 shared JSON/schema 解析通过 |
| Windows clippy re-validation | WINDOWS_VERIFICATION_PENDING | Linux 修复尚未在当时的 Windows 工作副本重新执行；本历史表格保留原始 pending 记录 |

本轮 reconciliation 结论：Windows 已通过项目仍保持其原 PASS 记录；Windows clippy 的历史 FAIL 仍保留，原因已在 Linux 修复，但需要下一轮 Windows workspace clippy re-validation。aria2 官方 ZIP 下载、PowerShell 解压、`aria2c.exe` 实际检测/进程生命周期、断点恢复、崩溃恢复、`.aria2` 清理、Unicode staging、403 回退、GUI、Named Pipe/Registry、externalBin/安装器和真实账号项目继续保持 `WINDOWS_VERIFICATION_PENDING`、`WINDOWS_BLOCKED`、`NOT RUN` 或 `BLOCKED`，不提前标记 PASS。

### Windows re-validation after Linux clippy fix（2026-09-09）

当前 Linux let-chain 修复已同步至 E 盘，并完成 Windows re-validation；上一轮 clippy FAIL 记录保留为历史记录。

本轮重新同步后复验（2026-09-09）：针对 Linux revision `add84c0950436b912671c5a451b2e3090300cb9f` 重新执行 `npm run check/test/build`、`cargo fmt --all -- --check`、`cargo check --workspace`、设置项目 `PYTHON` 后的 `cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`.venv\Scripts\pytest.exe sidecar\tests -q`、`npm run build:tauri` 和 `npm run dev:tauri`；结果全部为 PASS。Rust workspace 68 项测试全部通过，严格 clippy 复验通过。

| 验证项目 | 状态 | 关键结果 |
|---|---|---|
| Node check/test/build | PASS | Vite 构建通过；Extension 6 项测试通过 |
| Rust fmt/check/test | PASS | `cargo test --workspace` 通过，68 项测试全部通过 |
| Rust clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings` 通过 |
| Python sidecar | PASS | 10 passed |
| Tauri Release/dev | PASS | Release executable 生成，Vite、Rust Debug、Desktop 启动成功 |
| GUI 视觉验收 | BLOCKED | GUI automation helper 无可用 native app target |

aria2 官方 ZIP 下载、解压、`aria2c.exe` 检测和真实进程/恢复验证仍为 `NOT RUN`，原因是当前环境没有 `aria2c.exe`，本轮未安装外部 artifact 或修改系统设置。Edge/真实 X/Telegram、Named Pipe/Registry、externalBin/安装器仍为 `BLOCKED` 或 `NOT RUN`，原因分别是凭据缺失、Windows backend 未实现或发布 artifact 未配置。

本轮验证错误仅包括 MSVC linker stdout `#[warn(linker_messages)]` 和停止 Tauri 时的 Chromium `Error = 1411`/`STATUS_CONTROL_C_EXIT`，均不影响构建或启动结论。

Linux 后续处理：提供受控 aria2c artifact 并完成真实下载/解压/生命周期/恢复验证；继续实现 GUI、Named Pipe/Registry、externalBin/安装器和凭据相关链路。clippy 修复已完成 Windows re-validation，无需继续作为失败项处理。

本轮基线：Linux `main` / `add84c0950436b912671c5a451b2e3090300cb9f`，验证开始前 working tree clean；Windows `E:\Shiraishi\VSCode Workspace\Tw2Tg`，Windows 11 Insider Preview `10.0.29661` / 64 位，Node `v24.19.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。同步排除 `.git`、依赖、缓存、构建产物和 Linux 验证文档，其他内容 `rsync --checksum` 校验通过。

### Linux reconciliation after Windows clippy re-validation (2026-09-09)

本轮 Linux 重新读取上述 Windows re-validation 结果并按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成 reconciliation。上一轮 `collapsible_if` FAIL 的处理链路已闭环，历史 FAIL 记录保留：

```text
Previous Windows validation: FAIL (clippy::collapsible_if at desktop/src-tauri/src/lib.rs:83)

Linux fix: let-chain rewrite of candidate_aria2_paths

Linux verification: PASS (cargo fmt --all -- --check, cargo check --workspace,
cargo test --workspace, npm run check/test/build, Extension tests,
python3 -m compileall, JSON/schema parse)

Current Windows status: WINDOWS_PASS (strict workspace clippy re-validation)
```

| 项目 | 结果 | 说明 |
|---|---|---|
| Desktop aria2 `candidate_aria2_paths` clippy 修复 | WINDOWS_PASS | Windows `cargo clippy --workspace --all-targets -- -D warnings` re-validation 通过；let-chain 修复已在 Windows 确认，clippy 失败链路闭环 |
| 本轮 Linux 代码修改 | 无新增 | Windows 结果未引入新的代码失败；本轮仅做文档 reconciliation，未修改业务代码 |
| Linux 适用回归 | PASS | 重新执行 Rust fmt/check/test、Node check/test/build、Extension 测试、Python compileall 和 JSON/schema 解析，全部通过 |
| Linux cargo clippy | NOT RUN | Linux toolchain 未安装 `cargo-clippy`；clippy 结论以 Windows 严格 clippy re-validation 为准 |
| aria2c.exe 实际下载/解压/进程生命周期/断点与崩溃恢复/`.aria2` 清理/Unicode staging/403 回退 | NOT RUN | 依赖受控 `aria2c.exe` artifact 与 Windows 环境授权；状态保持待下一轮 Windows 验证 |
| GUI 视觉验收 | BLOCKED | UI automation helper 仍无可用 native app target |
| Named Pipe/Registry、externalBin/安装器、Edge Cookie/真实 X、Credential Manager、真实 Telegram 发送 | BLOCKED / NOT RUN | 分别受未实现 Windows backend、未配置发布 artifact 和凭据/外部服务限制 |

下一轮 Windows 验证重点保持不变：在获得受控 `aria2c.exe` artifact 后执行官方 ZIP 下载、PowerShell 解压、版本/hash 校验、进程生命周期、断点/崩溃恢复、`.aria2` 清理、Unicode staging 和 403 回退 gallery-dl；GUI 视觉验收、Named Pipe/Registry、externalBin/安装器和真实账号链路按各自前置条件推进。当前没有因 Windows 验证结果产生的待修复 Linux 代码问题。
### Linux reconciliation after Windows re-validation of revision `add84c0`（2026-09-09）

Windows 针对干净 working tree 的 Linux revision `add84c0950436b912671c5a451b2e3090300cb9f` 重新同步并完成全量复验。Linux 重新读取结果并按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows Node check/test/build、Rust fmt/check/test、严格 clippy `-D warnings`、`.venv` pytest、`build:tauri`、`dev:tauri` | WINDOWS_PASS | 全部在 Windows 实际执行并通过；clippy 失败链路（`collapsible_if`）在当前 revision 上确认闭环，无遗留待修复 lint |
| Telegram 发送状态持久化与幂等补传（单元层：migration 应用、`SendStateStore` 语义、`send_idempotently`） | WINDOWS_PASS（单元层） | Windows `cargo test --workspace` 实际包含 storage 16 项、telegram 12 项并通过 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 现有测试使用 in-memory SQLite，未执行基于文件 DB 和应用重启的专项验证；保持 pending，不得提前标记 PASS |
| 真实 Telegram 账号发送、Credential Manager | BLOCKED | 依赖真实账号/凭据和未实现的 Windows backend，本轮无变化 |
| 测试计数差异 | 待复核 | Windows 报告 `cargo test --workspace` 共 68 项；Linux 同一 revision 复测为 69 项（11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram）。Windows 自报分项（storage 16、telegram 12）与 Linux 一致，差异最可能为计数笔误，但未经 Windows 端确认前按差异记录；下一轮 Windows 验证需按 crate 重新清点并回填 |
| Sidecar 手动 Unicode 路径 JSONL 进程链路 | NOT RUN（保留） | 本轮 pytest 10 项通过，但未重复独立手动链路；既有通过证据保留在历史基线 |
| Windows 环境记录（python3 缺失、沙箱 `EPERM lstat` 父目录、esbuild postinstall 警告） | 环境事项 | 分别通过显式 `PYTHON`、受控权限复跑处理或为非阻塞警告；不属于项目代码失败，无需 Linux 代码修改 |

本轮 Plan 重新评估结论：原 Plan（Telegram 发送状态持久化 + 幂等补传）已完成实现、Linux 验证和 Windows 单元级验证，Plan 无剩余步骤；Windows 结果未引入任何属于项目代码的 FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。下一轮 Windows 验证重点：按 crate 清点测试总数并回填差异；对发送状态持久化执行基于文件 SQLite、应用重启恢复和迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成、GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件继续 `WINDOWS_VERIFICATION_PENDING` / `WINDOWS_BLOCKED` / `NOT RUN`。

### Windows validation rerun for Linux revision `4d4b3f5`（2026-09-09）

本轮针对最新 Linux revision `4d4b3f5a16584cf209edefa94688554443fc8ff6` 执行验证。该 revision 仅为上一轮 Windows 结果的文档 reconciliation；验证开始前 Linux working tree 只有本验证文档未提交修改，业务代码无新增改动。E 盘副本由 Linux source 重新单向同步，验证文档本身按规则排除。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | `E:\Shiraishi\VSCode Workspace\Tw2Tg`；排除 `.git`、`node_modules`、`.venv`、`target`、`dist`、缓存/数据库和验证文档后，`rsync --checksum` 无差异 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6 项测试通过 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | `cargo test --workspace`；按 crate 清点 69 项全部通过：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Telegram 发送状态持久化与幂等补传单元覆盖 | PASS | storage/telegram migration、状态往返、重试计数和幂等发送测试通过；真实账号及应用级文件 DB 重启恢复仍未执行 |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | Tauri CLI 2.11.4；`npm run build:tauri` 生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 Desktop executable；Ctrl+C 停止后无遗留进程 |

验证环境：Windows 11 Insider Preview `10.0.29661` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘已有依赖且 lockfile 未变化；未安装外部 artifact 或修改系统设置。

本轮错误和未执行项：

- 初次同步后的终审发现 E 盘副本仍有三份开发文档落后于 Linux revision；重新执行同一 Linux→E: 同步后，排除本地依赖/构建产物/验证文档的 `rsync --checksum` 复核通过。该问题属于同步工作流/环境状态，不属于项目代码失败。
- Windows PATH 没有 `python3`，因此 Rust 测试显式使用项目 `.venv\Scripts\python.exe`；未产生测试失败。
- Rust/Tauri 构建出现 MSVC linker stdout `#[warn(linker_messages)]`，属于非阻塞 warning。
- `aria2c.exe` 不在 PATH，且项目未提供受控 artifact；aria2c 下载/解压/进程生命周期/恢复为 `NOT RUN`，依赖项为 `BLOCKED`。
- GUI 视觉验收为 `BLOCKED`，缺少可用 GUI automation native app target。
- Edge Cookie、真实 X、真实 Telegram 为 `BLOCKED`，缺少账号/Profile/凭据。
- Named Pipe、Registry/Host manifest、浏览器 Extension 实机、externalBin、安装器、Tray/Autostart、Credential Manager 为 `NOT RUN` 或 `BLOCKED`，相关 Windows backend、发布 artifact 或专项前置条件尚未具备。

本轮已按 crate 清点确认 Windows 测试总数为 69 项，解决此前文档中的 68/69 计数差异。未发现属于项目代码的 Windows `FAIL`。Linux 后续事项保持为：提供受控 `aria2c.exe` artifact，完成文件 SQLite/应用重启恢复专项，以及 GUI、Windows backend、externalBin/安装器、凭据和真实账号链路验证。

### Linux reconciliation after Windows validation of revision `4d4b3f5`（2026-09-09）

Windows 针对最新 Linux revision `4d4b3f5a16584cf209edefa94688554443fc8ff6`（仅含上一轮文档 reconciliation，无业务代码改动）重新同步并完成全量复验。Linux 重新读取结果并按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows Node check/test/build、Rust fmt/check/严格 clippy、`cargo test --workspace`、`.venv` pytest、`build:tauri`、`dev:tauri` | WINDOWS_PASS | 全部实际执行并通过；`4d4b3f5` 与 `add84c0` 业务代码一致，结论可覆盖两者 |
| 68/69 测试计数差异 | 已关闭 | Windows 本轮按 crate 清点确认 69 项（11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram），与 Linux 计数一致；上一轮差异确认为计数笔误，历史"待复核"记录保留 |
| Telegram 发送状态持久化与幂等补传（单元层） | WINDOWS_PASS | storage 16 项、telegram 12 项在 Windows 实际执行并通过，状态与上一轮一致 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 本轮仍未执行专项验证，状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成及依赖项（断点/崩溃恢复/`.aria2` 清理/Unicode staging/403 回退） | NOT RUN / BLOCKED | 环境仍无 `aria2c.exe` 且未提供受控 artifact |
| GUI 视觉验收、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | BLOCKED / NOT RUN | 各自前置条件（automation target、Windows backend、发布 artifact、账号/凭据）均未具备，无变化 |
| 首次同步后 E 盘三份开发文档落后于 Linux revision | 已修复的环境事项 | 重新执行同一 Linux→E: 同步后 `rsync --checksum` 复核通过；属于同步工作流/环境状态，不属于项目代码失败，无需 Linux 代码修改 |

本轮 Plan 重新评估结论：原 Plan 已无剩余步骤；Windows 复验结果未引入任何属于项目代码的 FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。

### Windows validation rerun for latest Linux revision `facd8d7`（2026-09-09）

本轮针对最新 Linux revision `facd8d70017bafbc695384a19b21812b72d5339e` 执行验证。该 revision 仅包含上一轮 Windows 验证结果的文档 reconciliation，验证开始前 Linux working tree clean，业务代码无新增改动。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；排除 `.git`、`node_modules`、`.venv`、`target`、`dist`、缓存/数据库和验证文档后，`rsync --checksum` 无差异 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6 项测试通过 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | `cargo test --workspace`；按 crate 清点 69 项全部通过：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Telegram 发送状态持久化与幂等补传单元覆盖 | PASS | storage/telegram 的状态、重试、migration 和幂等发送测试全部通过；真实账号及应用级文件 DB 重启恢复未执行 |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | Tauri CLI 2.11.4；`npm run build:tauri` 生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 Desktop executable；Ctrl+C 停止后无遗留进程 |

验证环境：Windows 11 Insider Preview `10.0.29661` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘已有依赖且 lockfile 未变化；未安装外部 artifact 或修改系统设置。

本轮错误和未执行项：

- Windows PATH 没有 `python3`，Rust 测试显式使用项目 `.venv\Scripts\python.exe`，未产生测试失败。
- Rust/Tauri 构建出现 MSVC linker stdout `#[warn(linker_messages)]`，属于非阻塞 warning。
- `aria2c.exe` 不在 PATH，项目未提供受控 artifact；aria2c 下载、解压、进程生命周期、断点/崩溃恢复和 403 回退为 `NOT RUN` 或依赖阻塞。
- GUI 视觉验收为 `BLOCKED`，缺少可用 GUI automation native app target。
- Edge Cookie、真实 X、真实 Telegram 为 `BLOCKED`，缺少账号/Profile/凭据。
- Named Pipe、Registry/Host manifest、浏览器 Extension 实机、externalBin、安装器、Tray/Autostart、Credential Manager 为 `NOT RUN` 或 `BLOCKED`，相关 Windows backend、发布 artifact 或专项前置条件尚未具备。

本轮确认 Windows workspace 测试总数为 69 项，未发现项目代码导致的 Windows `FAIL`。Linux 后续事项保持为：提供受控 `aria2c.exe` artifact，完成文件 SQLite/应用重启恢复专项，以及 GUI、Windows backend、externalBin/安装器、凭据和真实账号链路验证。

### Windows validation rerun for Linux revision `facd8d7`（2026-09-09）

本轮针对最新 Linux revision `facd8d70017bafbc695384a19b21812b72d5339e` 执行验证。该 revision 仅包含上一轮 Windows 结果的文档 reconciliation；验证开始前 Linux working tree clean，业务代码无新增改动。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；排除 `.git`、依赖、缓存、构建产物和验证文档后，`rsync --checksum` 无差异 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6 项测试通过 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | `cargo test --workspace`；按 crate 清点 69 项全部通过：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Telegram 发送状态持久化与幂等补传单元覆盖 | PASS | storage/telegram 状态、重试、迁移和幂等发送测试全部通过；真实账号及应用级文件 DB 重启恢复未执行 |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | Tauri CLI 2.11.4；`npm run build:tauri` 生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 Desktop executable；主动 Ctrl+C 停止后未发现遗留进程 |

验证环境：Windows 11 Insider Preview `10.0.29661` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘已有依赖且 lockfile 未变化；未安装外部 artifact 或修改系统设置。

本轮未发现项目代码导致的 Windows `FAIL`。环境/阻塞事项如下：

- Windows PATH 没有 `python3`，Rust 测试显式使用项目 `.venv\Scripts\python.exe`，未产生测试失败。
- Rust/Tauri 构建出现 MSVC linker stdout `#[warn(linker_messages)]`，属于非阻塞 warning。
- `aria2c.exe` 不在 PATH，项目未提供受控 artifact；aria2c 下载、解压、进程生命周期、断点/崩溃恢复和 403 回退为 `NOT RUN` 或依赖阻塞。
- GUI 视觉验收为 `BLOCKED`，缺少可用 GUI automation native app target。
- Edge Cookie、真实 X、真实 Telegram 为 `BLOCKED`，缺少账号/Profile/凭据。
- Named Pipe、Registry/Host manifest、浏览器 Extension 实机、externalBin、安装器、Tray/Autostart、Credential Manager 为 `NOT RUN` 或 `BLOCKED`，相关 Windows backend、发布 artifact 或专项前置条件尚未具备。

本轮确认 Windows workspace 测试总数为 69 项，上一轮 68/69 计数差异已关闭。Linux 后续事项保持为：提供受控 `aria2c.exe` artifact，完成基于文件 SQLite/应用重启恢复专项，并继续推进 GUI、Windows backend、externalBin/安装器、凭据和真实账号链路验证。
### Linux reconciliation after Windows validation of revision `facd8d7`（2026-09-09）

Windows 针对最新 Linux revision `facd8d70017bafbc695384a19b21812b72d5339e`（仅含上一轮文档 reconciliation，无业务代码改动）重新同步并完成全量复验。Linux 重新读取结果并按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows Node check/test/build、Rust fmt/check/严格 clippy、`cargo test --workspace`、`.venv` pytest、`build:tauri`、`dev:tauri` | WINDOWS_PASS | 全部实际执行并通过；`facd8d7` 业务代码与 `4d4b3f5`/`add84c0` 一致，结论可覆盖三者 |
| 测试计数 | 已确认一致 | Windows 按 crate 清点确认 69 项（11/5/10/4/7/4/16/12），与 Linux 复测一致；68/69 差异维持关闭状态 |
| 发送状态持久化（单元层） | WINDOWS_PASS | storage 16 项、telegram 12 项在 Windows 实际执行并通过，连续三轮保持一致 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 本轮仍未执行专项验证，状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成及依赖项、GUI 视觉、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | NOT RUN / BLOCKED | 各自前置条件均未具备，无变化 |
| Windows 写回记录重复 | 文档事项 | 本轮 Windows 结果在同一文档中写入了两个内容相同的 `facd8d7` 复验小节（标题措辞略异）；按历史保留规则两节均不删除，仅在此记录重复事实。属于验证文档书写习惯问题，不属于项目代码或验证结论问题 |

本轮 Plan 重新评估结论：原 Plan 仍无剩余步骤；Windows 复验未引入任何属于项目代码的 FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。当前状态对 `add84c0`、`4d4b3f5`、`facd8d7` 三个 revision 保持一致。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。另建议 Windows 端后续写回结果时避免为同一 revision 重复创建小节。

### Windows validation rerun for latest Linux revision `5fbc675`（2026-09-09）

本轮针对最新 Linux revision `5fbc675fd6dec4a415776c868591bf268da53a53` 执行验证。该 revision 仅包含上一轮 Windows 验证结果的文档 reconciliation；验证开始前 Linux working tree clean，业务代码无新增改动。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；排除 `.git`、依赖、缓存、构建产物和验证文档后，`rsync --checksum` 无差异 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6 项测试通过 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | `cargo test --workspace`；按 crate 清点 69 项全部通过：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Telegram 发送状态持久化与幂等补传单元覆盖 | PASS | storage/telegram 状态、重试、migration 和幂等发送测试全部通过；真实账号及应用级文件 DB 重启恢复未执行 |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | Tauri CLI 2.11.4；`npm run build:tauri` 生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 Desktop executable；主动 Ctrl+C 停止后未发现遗留进程 |

验证环境：Windows 11 Insider Preview `10.0.29661` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘已有依赖且 lockfile 未变化；未安装外部 artifact 或修改系统设置。

### Linux reconciliation after Windows validation of revision `5fbc675`（2026-09-09）

Windows 针对最新 Linux revision `5fbc675fd6dec4a415776c868591bf268da53a53`（仅含上一轮文档 reconciliation）重新同步并完成全量复验，本轮写回为单一小节，无重复。Linux 按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows Node check/test/build、Rust fmt/check/严格 clippy、`cargo test --workspace`、`.venv` pytest、`build:tauri`、`dev:tauri` | WINDOWS_PASS | 全部实际执行并通过；按 crate 清点 69 项与 Linux 一致 |
| 发送状态持久化（单元层） | WINDOWS_PASS | storage 16 项、telegram 12 项连续四轮在 Windows 实际执行并通过 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 仍未执行专项验证，状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成、GUI 视觉、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | NOT RUN / BLOCKED | 前置条件均未具备，无变化 |

流程效率说明：经 Linux 端核实，`add84c0..HEAD` 业务代码（`crates`、`desktop/src-tauri/src`、`desktop/src`）为零改动，`add84c0`、`4d4b3f5`、`facd8d7`、`5fbc675` 四轮 Windows 复验对象为同一业务代码状态，结论一致。后续 Windows 轮次对仅含文档 reconciliation 的 revision 无需重复执行全量复验，可将验证资源集中于：含业务代码改动的 revision、新解除阻塞的专项（send-state 应用层验证、aria2c artifact 集成）或此前 `BLOCKED`/`NOT RUN` 项的前置变化。

本轮 Plan 重新评估结论：原 Plan 仍无剩余步骤；Windows 复验未引入任何属于项目代码的 FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。

本轮未发现项目代码导致的 Windows `FAIL`。Windows PATH 没有 `python3`，故 Rust 测试显式使用项目 `.venv\Scripts\python.exe`；构建出现 MSVC linker stdout `#[warn(linker_messages)]` 非阻塞 warning。`aria2c.exe` 不在 PATH 且未提供受控 artifact，相关下载/解压/生命周期/恢复项目为 `NOT RUN` 或 `BLOCKED`；GUI、Edge Cookie、真实 X/Telegram、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager 等因缺少 automation target、backend、artifact 或凭据而为 `BLOCKED` / `NOT RUN`。

Linux 后续事项：提供受控 `aria2c.exe` artifact，完成基于文件 SQLite/应用重启恢复/迁移升级专项，并继续推进 GUI、Windows backend、externalBin/安装器、凭据和真实账号链路验证。

### Windows validation of latest Linux revision `f3faea3`（2026-09-09）

本轮针对最新 Linux revision `f3faea35b9bf836518ff753dc39675bbe56bd5dc` 执行 Windows 验证。该 revision 相比已完成 Windows 全量验证的 `5fbc675` 仅包含文档 reconciliation，`crates`、`desktop/src-tauri/src`、`desktop/src`、`extension`、`sidecar` 和 `shared` 均无业务代码差异；验证开始前 Linux working tree clean。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；保留 Windows 本地 `.venv`、`node_modules`、`target`、`desktop\dist`，排除依赖、缓存、构建产物和验证文档后 checksum dry-run 通过 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6 项测试通过，Desktop Node tests 0 项 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | `cargo test --workspace`；69 项全部通过：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | Tauri CLI 2.11.4；`npm run build:tauri` 生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动 | PASS | `npm run dev:tauri` 成功启动 Vite、Rust Debug 和 `target\debug\xarchive-desktop.exe`；主动 Ctrl+C 停止，未发现残留进程 |

验证环境：Windows 11 Insider Preview `10.0.29661` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘已有依赖且 lockfile 未变化；未安装外部 artifact 或修改系统设置。

### Linux reconciliation after Windows validation of revision `f3faea3`（2026-09-09）

Windows 针对最新 Linux revision `f3faea35b9bf836518ff753dc39675bbe56bd5dc`（仅含上一轮文档 reconciliation，单一小节写回，无重复）完成全量复验。Linux 按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows Node check/test/build、Rust fmt/check/严格 clippy、`cargo test --workspace`、`.venv` pytest、`build:tauri`、`dev:tauri` | WINDOWS_PASS | 全部实际执行并通过；69 项测试按 crate 清点与 Linux 一致（11/5/10/4/7/4/16/12） |
| 发送状态持久化（单元层） | WINDOWS_PASS | storage 16 项、telegram 12 项连续五轮在 Windows 实际执行并通过 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 本轮仍未执行专项验证（`NOT RUN`），状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成、GUI 视觉、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | NOT RUN / BLOCKED | 前置条件均未具备，无变化 |

流程效率说明（延续上一轮结论）：Linux 端核实 `add84c0..f3faea3` 全部业务源码目录（`crates`、`desktop/src-tauri/src`、`desktop/src`、`extension`、`sidecar`、`shared`）diff 为空，本轮复验对象与此前四轮为同一业务代码状态，结论一致。再次明确：后续 Windows 轮次对仅含文档 reconciliation 的 revision 无需重复执行全量复验；仅在出现含业务代码改动的 revision、新解除阻塞的专项（send-state 应用层验证、aria2c artifact 集成）或 `BLOCKED`/`NOT RUN` 项前置变化时执行相应验证。

本轮 Plan 重新评估结论：原 Plan 仍无剩余步骤；Windows 复验未引入任何属于项目代码的 FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。

本轮未发现项目代码导致的 Windows `FAIL`。Windows PATH 没有 `python3`，使用项目 `.venv\Scripts\python.exe` 后 pytest 和 Rust 测试均通过；Rust/Tauri 构建的 MSVC linker stdout `#[warn(linker_messages)]` 为非阻塞 warning。`aria2c.exe` 不在 PATH 且未提供受控 artifact，aria2c 下载/解压/生命周期/断点恢复/崩溃恢复/403 回退为 `NOT RUN` 或 `BLOCKED`。基于文件 SQLite 的应用级重启恢复和迁移升级为 `NOT RUN`；GUI 视觉、Edge Cookie、真实 X/Telegram、Named Pipe/Registry、浏览器 Extension 实机、externalBin/安装器、Tray/Autostart、Credential Manager 因缺少 automation target、backend、artifact 或凭据而为 `BLOCKED` / `NOT RUN`。Sidecar 手动 Unicode JSONL 链路本轮未重复执行，既有通过证据保持有效。

Linux 后续事项：继续提供受控 `aria2c.exe` artifact，完成文件 SQLite/应用重启恢复/迁移升级专项，并推进 GUI、Windows backend、externalBin/安装器、凭据、真实账号及其他缺失前置条件的验证。由于本轮没有业务代码改动，不需要 Linux 代码修复或扩大 Plan。

### Linux reconciliation after Windows validation of revision `55bcdc8`（2026-09-09）

Windows 针对最新 Linux revision `55bcdc80ab82357983bcfbdd8350a343f72615c1`（仅含上一轮文档 reconciliation）按轻量模式复核：执行同步 checksum、`npm run check`、`cargo fmt/check` 和 Tauri CLI 版本检查（PASS），并将 Node test/build、Rust 全量测试、严格 clippy、pytest 和 Tauri Release/Debug 标记为 `NOT APPLICABLE`——因业务代码与已全量验证的 `f3faea3` 完全一致，既有通过证据继续有效。Linux 按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows 轻量 check（同步一致性、Node check、Rust fmt/check、Tauri CLI 入口） | WINDOWS_PASS | 实际执行并通过；全量套件按效率指引标记 `NOT APPLICABLE`，非未执行遗漏 |
| 发送状态持久化（单元层） | WINDOWS_PASS（继承） | `f3faea3` 全量证据对相同业务代码继续有效；Linux 同步复测 69/69 通过 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 专项仍未执行（`NOT RUN`），状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成、GUI 视觉、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | NOT RUN / BLOCKED | 前置条件均未具备，无变化 |

本轮确认 Windows 端已采纳「纯文档 revision 轻量复核」的工作方式，文档与验证成本显著降低，且未牺牲结论有效性。本轮 Plan 重新评估结论：原 Plan 仍无剩余步骤；无任何属于项目代码的 Windows FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。

### Windows validation review for latest Linux revision `55bcdc8`（2026-09-09）

本轮针对最新 Linux revision `55bcdc80ab82357983bcfbdd8350a343f72615c1` 执行 Windows 平台复核。该 revision 相比已完成全量 Windows 验证的 `f3faea3` 仅包含验证结果的文档 reconciliation；业务源码目录无变化，Linux working tree 在验证开始前 clean。

| 验证项目 | 状态 | 实际命令/关键证据 |
### Linux reconciliation after Windows validation of revision `8318569`（2026-09-09）

Windows 针对最新 Linux revision `831856946e76785d4efd9a531a6e9062e41fef52`（仅含上一轮文档 reconciliation）延续轻量复核：同步 checksum、`npm run check`、`cargo fmt/check` 和 Tauri CLI 版本检查 PASS；全量测试套件因业务代码与已全量验证的 `f3faea3` 一致标记 `NOT APPLICABLE`，既有证据继续有效。Linux 按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows 轻量 check（同步一致性、Node check、Rust fmt/check、Tauri CLI 入口） | WINDOWS_PASS | 实际执行并通过；全量套件 `NOT APPLICABLE`，非未执行遗漏 |
| 发送状态持久化（单元层） | WINDOWS_PASS（继承） | Linux 同步复测 69/69 通过；`f3faea3` 全量证据对相同业务代码继续有效 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 专项仍未执行（`NOT RUN`），状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成、GUI 视觉、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | NOT RUN / BLOCKED | 前置条件均未具备，无变化 |

本轮 Plan 重新评估结论：原 Plan 仍无剩余步骤；无任何属于项目代码的 Windows FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围。轻量复核模式运转正常。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。

|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；保留 E 盘 `.venv`、`node_modules`、`target`、`desktop\dist`，排除 `.git`、依赖、缓存、构建产物、数据库和验证文档后 checksum dry-run 通过 |
| 当前 Windows 轻量 check | PASS | `npm run check`、`cargo fmt --all -- --check`、`cargo check --workspace`、`npm exec --workspace desktop -- tauri --version`；Vite 35 modules、Rust check 和 Tauri CLI 2.11.4 均通过 |
| Node test/build、Rust workspace tests、严格 clippy、Python pytest、Tauri Release/Debug | NOT APPLICABLE | `f3faea3` 已对相同业务代码全量执行并通过（Rust 69 项、sidecar 10 项、Release/Debug）；`55bcdc8` 无业务源码或配置变化，按项目文档不重复执行 |
| Telegram 发送状态持久化单元层 | NOT APPLICABLE | 与上一轮相同业务状态，既有 Windows storage 16 项和 telegram 12 项通过证据继续有效 |
### Linux reconciliation after Windows validation of revision `5d9dbd9`（2026-09-09）

Windows 针对最新 Linux revision `5d9dbd9720f72642fe594969b92ce075db765253`（仅含上一轮文档 reconciliation）延续轻量复核：同步 checksum、`npm run check`、`cargo fmt/check` 和 Tauri CLI 版本检查 PASS；全量测试套件因业务代码与已全量验证的 `f3faea3` 一致标记 `NOT APPLICABLE`，既有证据继续有效。Linux 按 [`cross-platform-validation.md`](cross-platform-validation.md) 完成本轮 reconciliation：

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows 轻量 check（同步一致性、Node check、Rust fmt/check、Tauri CLI 入口） | WINDOWS_PASS | 实际执行并通过；全量套件 `NOT APPLICABLE`，非未执行遗漏 |
| 发送状态持久化（单元层） | WINDOWS_PASS（继承） | Linux 同步复测 69/69 通过；`f3faea3` 全量证据对相同业务代码继续有效 |
| 发送状态持久化（应用层：基于文件的 SQLite Windows 路径行为、应用重启现场恢复、0001→0002 迁移升级） | WINDOWS_VERIFICATION_PENDING | 专项仍未执行（`NOT RUN`），状态不变，不得提前标记 PASS |
| aria2c.exe artifact 集成、GUI 视觉、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie/真实 X/真实 Telegram | NOT RUN / BLOCKED | 前置条件均未具备，无变化 |

本轮 Plan 重新评估结论：原 Plan 仍无剩余步骤；无任何属于项目代码的 Windows FAIL，本轮无必要的 Linux 代码修改，不扩大 Plan 范围；轻量复核模式运转正常。下一轮 Windows 验证重点不变：发送状态持久化的基于文件 SQLite、应用重启现场恢复和 0001→0002 迁移升级专项验证（前置：受控 artifact 与验证设计）；aria2c.exe artifact 集成；GUI 视觉、Named Pipe/Registry、externalBin/安装器与真实账号链路按各自前置条件推进。

| 文件 SQLite 应用重启恢复、迁移升级 | NOT RUN | 专项仍未执行，缺少相应应用级验证场景 |
| aria2c artifact 集成及下载/恢复链路 | NOT RUN / BLOCKED | `aria2c.exe` 不在 PATH，项目未提供受控 artifact |
| GUI、Windows backend、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | automation target、backend、发布 artifact、账号或凭据等前置条件仍未具备 |

验证环境：Windows 11 Insider Preview `10.0.29661.0` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘依赖和 lockfile 未变化；未安装外部 artifact、未修改系统设置。

本轮没有项目代码导致的 Windows `FAIL`。Windows PATH 仍没有 `python3`，但本轮轻量 check 未依赖该命令；历史 Rust/Tauri MSVC linker stdout warning 为非阻塞环境输出。Linux 后续仅需继续处理 aria2c artifact、文件 SQLite/重启恢复/迁移专项，以及 GUI/backend、安装器、凭据和真实账号验证前置条件；不需要因本轮文档-only revision 修改业务代码。

### Windows validation review for latest Linux revision `8318569`（2026-09-09）

本轮针对最新 Linux revision `831856946e76785d4efd9a531a6e9062e41fef52` 执行 Windows 平台复核。该 revision 相比已完成全量 Windows 验证的 `f3faea3` 仍仅包含验证文档 reconciliation；业务源码目录无变化，Linux working tree 在验证开始前 clean。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；保留 E 盘依赖、`.venv`、Rust target 和 desktop 构建目录，checksum dry-run 通过 |
| 当前 Windows 轻量 check | PASS | `npm run check`、`cargo fmt --all -- --check`、`cargo check --workspace`、`npm exec --workspace desktop -- tauri --version`；Vite 35 modules、Rust check 和 Tauri CLI 2.11.4 均通过 |
| Node test/build、Rust workspace tests、严格 clippy、Python pytest、Tauri Release/Debug | NOT APPLICABLE | 同一业务代码状态已在 `f3faea3` 全量通过（Rust 69 项、sidecar 10 项、Release/Debug）；当前 revision 无业务源码或配置变化，按项目文档不重复执行 |
| Telegram 发送状态持久化单元层 | NOT APPLICABLE | 既有 Windows storage 16 项和 telegram 12 项通过证据继续有效 |
| 文件 SQLite 应用重启恢复、迁移升级 | NOT RUN | 缺少应用级专项验证场景，状态不变 |
| aria2c artifact 集成及下载/恢复链路 | NOT RUN / BLOCKED | `aria2c.exe` 不在 PATH，项目未提供受控 artifact |
| GUI、Windows backend、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | automation target、backend、发布 artifact、账号或凭据等前置条件仍未具备 |

验证环境沿用并复核为：Windows 11 Insider Preview `10.0.29661.0` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘依赖和 lockfile 未变化；未安装外部 artifact、未修改系统设置。

本轮没有项目代码导致的 Windows `FAIL`。Windows PATH 没有 `python3`，但本轮轻量 check 未依赖该命令；既有 MSVC linker stdout warning 为非阻塞环境输出。Linux 后续继续处理 aria2c artifact、文件 SQLite/重启恢复/迁移专项，以及 GUI/backend、安装器、凭据和真实账号验证前置条件；不扩大为开发任务。

### Windows validation review for latest Linux revision `5d9dbd9`（2026-09-09）

本轮针对最新 Linux revision `5d9dbd9720f72642fe594969b92ce075db765253` 执行 Windows 平台复核。该 revision 仅包含上一轮轻量 Windows 验证结果的文档 reconciliation；业务源码目录无变化，Linux working tree 在验证开始前 clean。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；保留 E 盘依赖、`.venv`、Rust target 和 desktop 构建目录，checksum dry-run 通过 |
| 当前 Windows 轻量 check | PASS | `npm run check`、`cargo fmt --all -- --check`、`cargo check --workspace`、`npm exec --workspace desktop -- tauri --version`；Vite 35 modules、Rust check 和 Tauri CLI 2.11.4 均通过 |
| Node test/build、Rust workspace tests、严格 clippy、Python pytest、Tauri Release/Debug | NOT APPLICABLE | `f3faea3` 已对相同业务代码全量通过（Rust 69 项、sidecar 10 项、Release/Debug）；当前 revision 无业务源码或配置变化，按项目文档不重复执行 |
| Telegram 发送状态持久化单元层 | NOT APPLICABLE | 既有 Windows storage 16 项和 telegram 12 项通过证据继续有效 |
| 文件 SQLite 应用重启恢复、迁移升级 | NOT RUN | 缺少应用级专项验证场景，状态不变 |
| aria2c artifact 集成及下载/恢复链路 | NOT RUN / BLOCKED | `aria2c.exe` 不在 PATH，项目未提供受控 artifact |
| GUI、Windows backend、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | automation target、backend、发布 artifact、账号或凭据等前置条件仍未具备 |

验证环境：Windows 11 Insider Preview `10.0.29661.0` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘依赖和 lockfile 未变化；未安装外部 artifact、未修改系统设置。

本轮没有项目代码导致的 Windows `FAIL`。Windows PATH 没有 `python3`，但轻量 check 未依赖该命令；既有 MSVC linker stdout warning 为非阻塞环境输出。Linux 后续继续处理 aria2c artifact、文件 SQLite/重启恢复/迁移专项，以及 GUI/backend、安装器、凭据和真实账号验证前置条件；不扩大为开发任务。

### Windows validation review for latest Linux revision `d239a1d`（2026-09-09）

本轮针对最新 Linux revision `d239a1dbb7bc8ee9f3b851ba2378a915e3312017` 执行 Windows 平台复核。该 revision 仅包含上一轮轻量 Windows 验证结果的文档 reconciliation；业务源码目录无变化，验证开始前仅有既存验证文档 working-tree 改动。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与内容一致性 | PASS | 单向同步至 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；保留 E 盘依赖、`.venv`、Rust target 和 desktop 构建目录，checksum dry-run 通过 |
| 当前 Windows 轻量 check | PASS | `npm run check`、`cargo fmt --all -- --check`、`cargo check --workspace`、`npm exec --workspace desktop -- tauri --version`；Vite 35 modules、Rust check 和 Tauri CLI 2.11.4 均通过 |
| Node test/build、Rust workspace tests、严格 clippy、Python pytest、Tauri Release/Debug | PASS | 用户授权下载组件后复跑；Node Extension 6 项、Rust workspace 69 项、sidecar pytest 10 项、严格 clippy、Release/Debug 均通过 |
| Telegram 发送状态持久化单元层 | PASS | storage 16 项、telegram 12 项 Windows 单元测试通过 |
| aria2 官方 Windows x64 artifact 下载、SHA-256、解压和版本 | PASS | 下载 `aria2-1.37.0-win-64bit-build1.zip`；SHA-256 `67D015301EEF0B612191212D564C5BB0A14B5B9C4796B76454276A4D28D9B288` 与 allowlist 一致；解压后 `aria2c --version` 为 1.37.0 |
| aria2 RPC、Unicode/空格路径下载、暂停/恢复、Range 续传和 hash | PASS | 使用本地 Range-capable HTTP fixture；`aria2.getVersion`、本地下载、`pause`/`unpause`、16 MiB Release artifact Range 续传及 SHA-256 全部通过 |
| aria2 进程中断恢复与 `.aria2` 清理 | PASS | 模拟终止 aria2 进程后重新启动并续传；最终 SHA-256 一致，`.aria2` 控制文件完成后清理，验证进程正常停止 |
| 文件 SQLite 应用重启恢复、迁移升级 | NOT RUN | 缺少应用级专项验证场景，状态不变 |
| aria2 403 回退 gallery-dl、真实 X/CDN | BLOCKED | 仅使用本地 HTTP fixture；真实外部服务、凭据和回退链路未具备 |
| GUI、Windows backend、Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 Telegram | BLOCKED / NOT RUN | automation target、backend、发布 artifact、账号或凭据等前置条件仍未具备 |

验证环境：Windows 11 Insider Preview `10.0.29661.0` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。本轮未执行 `npm ci`，因 E 盘依赖和 lockfile 未变化；未安装系统组件、未修改 PATH 或系统设置。官方 aria2 ZIP 和本轮测试 artifact 后续已移动至 E 盘独立开发目录 `E:\Shiraishi\VSCode Workspace\Tw2Tg-Windows-DevArtifacts\aria2\`。

本轮发现并已隔离的验证脚本问题：首次暂停尝试使用 Python 标准 HTTP server，因路径参数转义失败；修正后又因该 server 不支持 Range 响应导致 aria2 `Invalid range header`，另一次人为设置 `always-resume=false` 的暂停脚本触发 aria2 `Piece.cc:309` assertion。上述均属于测试 fixture/参数设置问题；使用正确的 Range server 和 aria2 默认续传设置重跑后，RPC、暂停/恢复、断点续传、进程中断恢复和 `.aria2` 清理均 PASS，未归类为项目代码 FAIL。

Linux 后续事项：继续完成文件 SQLite/重启恢复/迁移专项，补齐 aria2 403 回退、GUI/backend、安装器、凭据和真实账号验证前置条件；aria2 官方 artifact 的基础 Windows 集成与恢复验证已完成，不需要因本轮结果修改业务代码。

### Windows aria2 artifact 半永久保存（2026-09-09）

用户授权后，本轮将验证所需文件从 Windows 验证副本的 `E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\aria2\` 移动至独立开发 artifact 目录：

`E:\Shiraishi\VSCode Workspace\Tw2Tg-Windows-DevArtifacts\aria2\`

保存内容包括：官方 `aria2-1.37.0-win-64bit-build1.zip`、解压后的 `aria2c.exe` 及许可证/说明文件、Range-capable 本地测试 server、aria2 RPC/恢复日志、session、下载结果和失败尝试日志。ZIP SHA-256 为 `67D015301EEF0B612191212D564C5BB0A14B5B9C4796B76454276A4D28D9B288`，与项目 allowlist 一致；`aria2c.exe --version` 为 `1.37.0`。

该目录属于 E 盘 Windows 本地开发/验证 artifact，不纳入 Linux→Windows 源同步，不反向同步到 WSL，不写入 PATH，也不安装系统服务。旧验证副本中的 `validation-artifacts\aria2\` 已确认不存在。

### Windows validation for latest Linux revision `34b5c67`（2026-09-10）

本轮针对最新 Linux revision `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6` 执行 Windows 平台验证。该 revision 包含 Dashboard 的加载状态、系统健康判断、错误播报、紧凑导航可访问名称和仅 Windows 显示 aria2 面板修补，以及对应路线图更新；验证开始前 Linux `main` working tree clean，未包含未提交修改。

### Validation Environment

- Windows 11 Insider Preview `10.0.29661.0` / 64 位。
- Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。
- Windows 工作副本：`E:\Shiraishi\VSCode Workspace\Tw2Tg`；该目录为非 Git 验证副本。
- 同步方向：WSL `/home/shiraishi/VSCode Workspace/Tw2Tg` → Windows E 盘；使用 Robocopy `/E /XJ /FFT /COPY:DAT /DCOPY:DAT`，未使用镜像删除。
- 排除 `.git`、`node_modules`、`.venv`、`target`、Desktop 构建目录、缓存、数据库、`.env` 和本地验证报告；E 盘既有依赖/构建产物/本地验证目录保持不变。

### Validation Results

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux → Windows 同步与关键文件一致性 | PASS | Robocopy 单向同步完成；`desktop/src/main.jsx`、`desktop/src/style.css`、`docs/development/roadmap.md` 与 WSL 源文件内容一致，依赖、缓存、构建产物和本地 artifact 未被覆盖 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules 构建通过，Extension 6 项测试通过，Desktop Node 测试为 0 项且无失败 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | 设置 `PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe` 后 `cargo test --workspace` 通过 69 项：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | `npm exec --workspace desktop -- tauri --version` 为 2.11.4；`npm run build:tauri` 成功生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动与进程清理 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 `target\debug\xarchive-desktop.exe`；启动期间进程可见，Ctrl+C 后 Tw2Tg 相关进程为 0 |
| aria2 官方 artifact 与下载/恢复链路 | NOT APPLICABLE | aria2 逻辑和 artifact 自上一轮 `d239a1d` Windows 实测后未变化；既有 aria2 RPC、Unicode/空格路径、Range 续传、进程恢复和 `.aria2` 清理 PASS 证据继续有效，本轮变化仅涉及 GUI 展示条件 |
| GUI 视觉、WebView2/DPI、键盘和辅助技术人工验收 | BLOCKED | 当前环境没有可用 GUI automation native app target；本轮只能确认构建和 Debug 启动，不能据此宣称真实渲染验收通过 |
| 文件 SQLite 应用重启恢复、迁移升级 | NOT RUN | 当前仍缺少应用级专项场景；Rust repository 单元测试通过不替代 Desktop 现场重启/迁移验证 |
| Named Pipe、Native Host/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | Windows backend、发布 artifact、浏览器实机、automation target、账号或凭据等前置条件仍未具备 |

### Errors

- 首次直接执行 `cargo test --workspace` 时，`xarchive-sidecar-supervisor` 的 `spawn_ready_completes_the_hello_handshake` 和 `communicates_with_a_real_python_worker_when_available` 报 `NotRunning`。原因是 Windows 环境未显式设置项目要求的 `PYTHON`，不是业务代码失败；设置 `PYTHON` 指向项目 `.venv\Scripts\python.exe` 后，目标测试 4/4 和完整 workspace 69/69 均通过。
- Rust/Tauri 构建出现 MSVC linker stdout `#[warn(linker_messages)]`，只记录生成 `.lib/.exp` 的非阻塞 warning，不影响构建或测试结论。
- 停止 Tauri Debug 时记录 Chromium `Failed to unregister class Chrome_WidgetWin_0. Error = 1412` 和 `STATUS_CONTROL_C_EXIT`。这是 Ctrl+C 主动终止开发进程时的 Windows 运行时清理输出；最终无 Tw2Tg 遗留进程，未导致启动验证失败。若未来要求无告警退出，应作为独立 Windows 生命周期问题调查，不在本轮修改业务代码。

### Linux Follow-up

本轮没有发现属于项目代码的 Windows `FAIL`，无需因验证结果修改业务代码。Linux 后续处理事项为：

- 在具备 GUI automation native app target 后，重新验证最新 Dashboard 的真实 WebView2/DPI 渲染、初始加载/错误播报、紧凑导航 aria-label、aria2 仅 Windows 显示、键盘焦点和辅助技术反馈；
- 设计并执行基于文件 SQLite 的 Desktop 应用重启恢复和 `0001 → 0002` 迁移专项；
- 继续补齐 Named Pipe/Registry、真实 externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram 等各自前置条件；
- 可选地单独调查 Tauri Ctrl+C 后的 Chromium `Error = 1412` 清理警告。

### Linux reconciliation after Windows validation of revision `34b5c67`（2026-09-10）

Linux 重新读取了本轮 Windows 验证结果和当前 Plan。Windows 针对 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6` 的 Node、Rust fmt/check/clippy/test、Python Sidecar、Tauri Debug/Release 构建和进程清理均通过；aria2 相关验证继承上一轮 `d239a1d` 的实际 Windows PASS。本轮没有项目代码导致的 Windows `FAIL`。

| 事项 | 状态 | 当前处理 |
|---|---|---|
| `34b5c67` 已同步的 GUI 修补：systemReady、initialLoad、错误播报、紧凑导航 `aria-label`、aria2 Windows gating、最近任务命名 | WINDOWS_PASS | Windows 已对这些 revision 中的实际代码执行构建/测试；真实 GUI 渲染结论仍不由构建结果替代 |
| Linux 后续 GUI 语义修补：Widget 级错误恢复、未实现导航项移除、Job `<ul>/<li>`、`<time>`、共享 `:focus-visible`、`prefers-reduced-motion` | LINUX_VERIFIED / WINDOWS_VERIFICATION_PENDING | Linux 已通过 Vite check/build、Desktop Node test、Rust fmt/check/test；这些新增代码尚未在 Windows 复验，不提前标记 `WINDOWS_PASS` |
| GUI 真实 WebView2/DPI、Tab 顺序、Focus-visible 实际表现、命中目标、Narrator/NVDA 和最终对比度 | WINDOWS_VERIFICATION_PENDING | 本轮仍因 GUI automation native app target 不可用而 BLOCKED；下一次具备 target 后执行 W-P1-11 |
| 文件 SQLite Desktop 应用重启、遗留 staging、`0001 → 0002` 迁移升级 | NOT RUN | 仍缺少应用级专项验证场景；Repository 单元测试不能替代现场恢复验证 |
| Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | WINDOWS_VERIFICATION_PENDING / BLOCKED / NOT RUN | 依赖 Windows backend、发布 artifact、浏览器实机、automation target、账号或凭据；不因本轮基础测试通过而关闭 |

本轮 Linux Plan 重新评估：systemReady、initialLoad、错误播报、Widget 级错误恢复/重试、用户级错误区域标题、紧凑导航可访问名称、未实现导航项移除、aria2 Windows gating、最近任务命名、Job/时间语义、Focus-visible 和 reduced-motion 基础修补均已完成；随后 Windows 使用 `/IS /IT` 强制同步并完成当前 working tree 的构建/测试复验。真实 GUI 渲染和辅助技术验收仍保持 `WINDOWS_VERIFICATION_PENDING`/`BLOCKED`，文件 SQLite 应用级专项保持 `NOT RUN`。Windows-only 项目不得提前标记为 `WINDOWS_PASS`。

### Current Linux working tree after Windows validation of `34b5c67`（2026-09-10）

在 `34b5c67` Windows 验证完成后，Linux 完成了不依赖 Windows 的 GUI 代码收口：Widget 级错误分离与重试、移除未实现的禁用主导航、Job `<ul>/<li>`/`<time>` 语义、Focus-visible/reduced-motion 和用户级错误区域标题。随后 Windows 使用 `/IS /IT` 强制同步并实际完成构建/测试复验；这些代码的当前状态为 `LINUX_VERIFIED`，真实 GUI 渲染/交互验收仍为 `WINDOWS_VERIFICATION_PENDING`/`BLOCKED`。

### Preliminary Windows validation for Linux working tree based on `34b5c67`（2026-09-10）

本轮实际验证对象为 Linux `main` 的 HEAD `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6` 加验证开始前已存在的 working-tree changes，而不是纯 Git commit。验证开始时 `git status --short` 显示以下未提交修改：`desktop/src/main.jsx`、`desktop/src/style.css`、`docs/development/non-windows-completion.md`、`docs/development/risk-register.md`、`docs/development/roadmap.md`、`docs/development/testing-strategy.md` 以及既有的本验证报告。当前 GUI working-tree changes 包括 Widget 级错误分离/重试、移除未实现禁用导航、Job `<ul>/<li>` 与 `<time>` 语义、Focus-visible 和 reduced-motion 样式；这些改动是本轮验证目标。

> 同步复核说明：本节最初执行的测试发生在 Robocopy 按时间/大小判断而遗漏同尺寸 working-tree 文件的旧副本上，不能作为最新 working-tree 的正式证据。随后已使用 `/IS /IT` 强制同步，并对最新副本完整重跑；正式结论见下方“after forced sync”小节。

### Validation Environment

- Windows 11 Insider Preview `10.0.29661.0` / 64 位。
- Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`。
- Windows 工作副本：`E:\Shiraishi\VSCode Workspace\Tw2Tg`；未建立 Git checkout。
- 同步方向：WSL `/home/shiraishi/VSCode Workspace/Tw2Tg` → Windows E 盘；使用 Robocopy `/E /XJ /FFT /COPY:DAT /DCOPY:DAT`，未使用镜像删除。
- 排除 `.git`、`node_modules`、`.venv`、`target`、Desktop 构建目录、缓存、数据库、`.env` 和验证报告；E 盘本地依赖、缓存、构建产物与 `validation-artifacts` 均保留。

### Validation Results

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| Linux working tree → Windows 同步与关键 GUI 文件一致性 | PASS | 当前 working-tree 的 `desktop/src/main.jsx`、`desktop/src/style.css`、`docs/development/roadmap.md` 已同步到 E 盘并参与测试；本地依赖/缓存/构建产物未被覆盖 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules 构建通过，Extension 6 项测试通过，Desktop Node 测试 0 项且无失败 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | 设置 `PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe` 后 `cargo test --workspace` 通过 69 项：11 core、5 desktop、10 download、4 Native Host、7 protocol、4 supervisor、16 storage、12 Telegram |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri CLI/Release 构建 | PASS | `npm exec --workspace desktop -- tauri --version` 为 2.11.4；`npm run build:tauri` 成功生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动与进程清理 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 Desktop executable；启动期间进程可见，Ctrl+C 后 Tw2Tg 相关进程为 0 |
| 最新 GUI working-tree 修补的真实 WebView2/DPI、键盘、焦点、屏幕阅读器和对比度验收 | BLOCKED | 当前环境没有可用 GUI automation native app target；构建和启动 PASS 不能替代真实渲染/交互验收 |
| aria2 官方 artifact 与下载/恢复链路 | NOT APPLICABLE | 本轮改动未触及 aria2 核心逻辑；`d239a1d` 已完成官方 artifact、RPC、Unicode/空格路径、Range 续传、进程恢复和 `.aria2` 清理的实际 Windows 验证，证据继续有效 |
| 文件 SQLite 应用重启恢复、遗留 staging、`0001 → 0002` 迁移 | NOT RUN | 仍缺少应用级专项场景；Repository 单元测试不能替代 Desktop 现场恢复验证 |
| Named Pipe、Native Host/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | Windows backend、发布 artifact、浏览器实机、账号/凭据和自动化前置条件仍未具备 |

### Errors

- 首次直接执行 `cargo test --workspace` 时，`xarchive-sidecar-supervisor` 的 2 项真实 Python worker/hello 测试因未显式设置 `PYTHON` 报 `NotRunning`。这是 Windows 环境配置前置问题，不是业务代码 FAIL；设置 `PYTHON` 指向项目 `.venv\Scripts\python.exe` 后，目标测试 4/4 和完整 workspace 69/69 均通过。
- Rust/Tauri 构建出现 MSVC linker stdout `#[warn(linker_messages)]`，属于生成 `.lib/.exp` 的非阻塞 warning。
- 停止 Tauri Debug 时记录 Chromium `Failed to unregister class Chrome_WidgetWin_0. Error = 1412` 和 `STATUS_CONTROL_C_EXIT`；最终 Tw2Tg 进程已清理，未影响启动结论。若要求无告警退出，应作为独立 Windows 生命周期事项调查。

### Linux Follow-up

本轮没有发现属于项目代码的 Windows `FAIL`。但由于本轮验证对象包含未提交 working-tree changes，Linux 后续应：

- 在可用 GUI automation native app target 后，对最新 GUI working-tree changes 执行 W-P1-11 的真实 WebView2/DPI、Tab/键盘、Focus-visible、aria-live/错误恢复、aria2 Windows gating、屏幕阅读器和对比度验证；
- 完成文件 SQLite Desktop 应用重启、遗留 staging 恢复和 `0001 → 0002` 迁移专项；
- 继续推进 Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram 等前置条件；
- 可选地单独调查 Tauri Ctrl+C 后 Chromium `Error = 1412` 清理警告；
- 在提交或继续修改 GUI 前，重新执行本轮代码变更对应的 Windows 验证，不能将本轮结果外推到后续未同步的 working-tree changes。

### Windows re-validation after forced sync of latest Linux working tree（2026-09-10）

本节是本轮正式结论。验证对象仍为 Linux `main` HEAD `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6` 加验证开始前已有的未提交 GUI/文档 working-tree changes；通过 Robocopy `/IS /IT` 强制同步后，最新文件才进入 Windows 工作副本并重新执行全部适用验证。

| 验证项目 | 状态 | 实际命令/关键证据 |
|---|---|---|
| 最新 Linux working tree → Windows 同步 | PASS | 使用 `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT` 单向覆盖受控源文件；排除 `.git`、依赖、缓存、构建产物、数据库、`.env` 和验证报告；关键 GUI 文件哈希复核一致 |
| Node workspace | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules、Extension 6 项测试通过，Desktop Node 测试 0 项且无失败 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings` 全部通过 |
| Rust workspace tests | PASS | 设置 `PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe` 后 `cargo test --workspace`；69 项全部通过：11/5/10/4/7/4/16/12 |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10 passed |
| Tauri Release 构建 | PASS | `npm run build:tauri`；35 modules 构建并成功生成 `target\release\xarchive-desktop.exe` |
| Tauri Debug 启动与进程清理 | PASS | `npm run dev:tauri` 启动 Vite、Rust Debug 和 Desktop executable；进程检查可见，停止后 Tw2Tg 相关进程为 0 |
| 最新 GUI working-tree 的真实 WebView2/DPI、键盘、焦点、屏幕阅读器、对比度验收 | BLOCKED | 当前没有可用 GUI automation native app target；构建/启动结果不替代真实渲染和交互验收 |
| aria2 官方 artifact 与下载/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑；沿用 `d239a1d` 已完成的官方 artifact、RPC、Range/恢复和 `.aria2` 清理 Windows 证据 |
| 文件 SQLite Desktop 应用重启、遗留 staging、`0001 → 0002` 迁移 | NOT RUN | 仍没有应用级专项测试场景 |
| Named Pipe、Native Host/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 相应 backend、发布 artifact、浏览器实机、自动化 target、账号或凭据前置条件缺失 |

### Re-validation Errors and Follow-up

- 首次 Robocopy 同步未使用 `/IS /IT`，虽然返回允许的退出码 3，但同尺寸/近似时间戳文件未被覆盖；后续哈希检查发现 `desktop/src/main.jsx`、`desktop/src/style.css` 及文档存在差异。该同步过程错误已通过 `/IS /IT` 修正，旧副本测试结果作废，强制同步后的结果才是本轮有效证据。
- 强制同步后的 Rust 测试从一开始显式设置 `PYTHON`，未再出现 `NotRunning`；完整 workspace 69/69 通过。MSVC linker stdout 的 `#[warn(linker_messages)]` 仍为非阻塞 warning。
- Ctrl+C 停止 Tauri Debug 时出现 Chromium `Failed to unregister class Chrome_WidgetWin_0. Error = 1411` 与 `STATUS_CONTROL_C_EXIT`；进程最终清理干净。该 Windows 生命周期警告可另立事项调查，本轮不修改业务代码。

Linux 后续处理：本轮未发现项目代码导致的 Windows `FAIL`。需在具备 GUI automation native app target 后重新执行最新 working-tree 的 W-P1-11 真实渲染/交互验收；继续设计文件 SQLite 应用级恢复/迁移专项，以及 Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie 和真实账号链路。后续若 GUI working tree 再变化，必须重新执行受控同步和对应 Windows 验证。

### Historical Linux follow-up before latest Windows working-tree re-validation（2026-09-10）

依据上一节强制同步后的 Windows 正式结果，Node/Rust/Sidecar/Tauri 基础验证已经覆盖当时的 GUI working tree；真实 GUI 验收仍为 `BLOCKED`，文件 SQLite 应用级恢复/迁移仍为 `NOT RUN`。Linux 随后只继续了一个不依赖 Windows 的小范围改动：为 Widget 错误增加用户级区域标题，例如“任务列表加载失败”“归档文件夹打开失败”，并保留原始技术详情和局部重试。

该段记录的是本轮强制同步前的状态快照；随后已通过 `/IS /IT` 将该错误文案改动同步到 Windows 并完成正式复验，正式结果见下方最新条目。除实际同步并验证的 working tree 外，不外推此前 Windows 结果。

### Final Windows re-validation of latest Linux working tree（2026-09-10）

本轮最终验证对象为 Linux `main` HEAD `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6` 加验证开始前已有的未提交 GUI/文档 working-tree changes，包括 Widget 级错误分离、用户级区域标题与重试、未实现导航项移除、Job/时间语义、Focus-visible 和 reduced-motion。使用 `/IS /IT` 强制同步后，以下结果覆盖了该完整 working tree。

| 验证项目 | 状态 | 实际命令/结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；关键源文件哈希一致，未覆盖依赖、缓存、构建产物、数据库、`.env` 或独立 artifact |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6，Desktop Node 测试无失败 |
| Rust fmt/check/clippy/test | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、严格 clippy、`cargo test --workspace`；69/69 通过，Rust 测试显式使用项目 `.venv\Scripts\python.exe` |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10/10 通过 |
| Tauri Release/Debug | PASS | `npm run build:tauri` 生成 Release executable；`npm run dev:tauri` 启动 Vite/Rust/Desktop，停止后 Tw2Tg 进程为 0 |
| GUI 真实 WebView2/DPI/键盘/焦点/屏幕阅读器/对比度 | BLOCKED | 无可用 GUI automation native app target；构建和启动不能替代真实渲染验收 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 已完成的 Windows 实测证据 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 缺少应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | Windows backend、发布 artifact、浏览器实机、账号/凭据或自动化前置条件缺失 |

正式结论：本轮没有项目代码导致的 Windows `FAIL`。唯一的同步过程问题是初次 Robocopy 未强制覆盖同尺寸文件，已由 `/IS /IT` 修正并重新执行全部适用测试；Tauri Ctrl+C 仍记录 Chromium `Error = 1411`/`STATUS_CONTROL_C_EXIT`，但进程清理正常。Linux 已有 storage 层文件数据库重开与 migration 幂等测试，但这不替代 Windows Desktop 应用现场重启、路径行为、遗留 staging 和迁移升级验证。Linux 后续继续处理 GUI 真实渲染验收、文件 SQLite 应用级恢复/迁移专项及其余 Windows backend/账号前置条件；本轮不修改业务代码。

### Windows validation repeat for current Linux working tree（2026-09-10）

本轮按用户要求再次验证当前 Linux 最新状态。HEAD 仍为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`，验证开始时 working tree 仍包含既有 GUI/文档未提交修改；未发现新的业务代码提交。使用 Robocopy `/IS /IT` 将当前 WSL working tree 单向同步到 `E:\Shiraishi\VSCode Workspace\Tw2Tg`，关键文件哈希一致。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；排除 `.git`、依赖、缓存、构建产物、数据库、`.env` 和验证报告；本地额外目录保留 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules、Extension 6/6 通过 |
| Rust fmt/check/test/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`、严格 clippy；69/69 通过，显式设置项目 `.venv\Scripts\python.exe` |
| Python sidecar | PASS | `.venv\Scripts\pytest.exe sidecar\tests -q`；10/10 通过 |
| Tauri Release/Debug | PASS | `npm run build:tauri` 成功生成 Release executable；`npm run dev:tauri` 启动 Vite/Rust/Desktop，停止后相关进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 当前 working tree 未修改 aria2 核心逻辑，沿用 `d239a1d` 的实际 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/辅助技术/对比度 | BLOCKED | 无 GUI automation native app target；构建和启动不能替代真实渲染验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | backend、发布 artifact、浏览器实机、账号/凭据和自动化前置条件缺失 |

本轮无项目代码导致的 Windows `FAIL`。构建仍有非阻塞 MSVC linker stdout warning；主动 Ctrl+C 停止 Tauri 时出现 Chromium `Error = 1411`/`STATUS_CONTROL_C_EXIT`，但相关进程已清理。Linux 后续继续处理 GUI 真实验收、文件 SQLite 应用级恢复/迁移及其余 Windows backend/账号前置条件；本轮不扩大为开发任务。

### Latest Linux reconciliation after Windows repeat（2026-09-10）

最新 Windows repeat 已覆盖当前 Linux working tree，包含全部 GUI/文档未提交修改；Node、Rust、Sidecar、Tauri 构建和启动清理均为 `PASS`，本轮没有项目代码导致的 Windows `FAIL`。因此当前 Plan 不再继续 GUI 源码修补，也不重复已有 storage 层文件数据库重开测试。

当前仍需保持的状态：

- GUI 真实 WebView2/DPI、键盘/焦点、屏幕阅读器和对比度：`WINDOWS_VERIFICATION_PENDING` / `BLOCKED`；
- 文件 SQLite Desktop 应用重启、遗留 staging、迁移升级：`NOT RUN`；
- Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram：`BLOCKED / NOT RUN`。

Linux 本轮适用验证已通过：workspace Node check/test/build、Rust fmt/check/test、Python `compileall` 和 7 个 JSON/schema 文件解析。由于 Linux 环境缺少 `cargo-clippy` 与 `pytest`，不将其本地结果标记为 PASS；Windows 对应结果继续以最新验证记录为准。

### Windows validation repeat after current-state resynchronization（2026-09-10）

本轮针对当前 Linux 最新 working tree 重新执行 Windows 验证。Linux source 为 branch `main`、HEAD `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，包含此前已有的 GUI/文档未提交修改，本轮未新增业务代码修改。Windows 工作副本为 `E:\Shiraishi\VSCode Workspace\Tw2Tg`。环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。使用 Robocopy `/IS /IT` 完成 Linux → Windows 单向同步；6 个关键文件哈希一致，Windows 本地依赖、缓存、构建产物和独立测试辅助文件按规则保留。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | `robocopy /E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；排除 `.git`、`node_modules`、`.venv`、Rust target、构建缓存、数据库、`.env` 和验证报告；Robocopy exit 3 表示复制文件并保留目标额外目录，非错误 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules 构建通过，Extension 6/6 通过 |
| Rust fmt/check/test/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`；workspace 测试 69/69 通过 |
| Python sidecar | PASS | `.venv\\Scripts\\pytest.exe sidecar\\tests -q`；10/10 通过 |
| Tauri Release/Debug | PASS | `npm run build:tauri` 成功生成 `target\\release\\xarchive-desktop.exe`；`npm run dev:tauri` 启动成功，停止后相关进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮 working tree 未修改 aria2 核心逻辑；沿用 `d239a1d` 的实际 Windows PASS 证据。独立保存的 aria2 测试辅助文件仍位于 `E:\Shiraishi\VSCode Workspace\Tw2Tg-Windows-DevArtifacts\aria2` |
| GUI 真实 WebView2/DPI/键盘/辅助技术/对比度 | BLOCKED | 当前没有可用的 GUI automation native-app target；构建和启动不能替代真实渲染、输入法、屏幕阅读器和对比度验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前环境未执行 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 分别缺少对应 Windows backend、发布/安装验证、浏览器实机、账号或凭据及自动化前置条件 |

本轮未出现项目代码导致的 Windows `FAIL`。构建输出中的 MSVC linker warning 为非阻塞警告；主动 Ctrl+C 停止 Tauri 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，但相关进程已清理。需要 Linux 后续处理的事项仍为 GUI 真实验收、文件 SQLite 应用级恢复/迁移，以及其余 Windows backend、发布安装和真实账号/凭据场景。本轮不扩大为开发任务。

### Latest Linux reconciliation after Windows results（2026-09-10）

Linux 重新读取了当前 `git diff`、最新 Windows 验证记录、现有 Plan、`AGENTS.md` 和跨平台验证规范。Windows 最新事实为：针对 `main` / `34b5c67` 加 working-tree changes 的 Node、Rust fmt/check/clippy/test、Python Sidecar、Tauri Debug/Release 构建和进程清理均通过；aria2 基础 artifact/RPC/恢复链路沿用既有 Windows PASS。GUI automation native-app target 仍不可用，因此真实 WebView2/DPI、Tab 顺序、键盘、Focus-visible 实际表现、命中区域、Narrator/NVDA 和最终对比度没有完成实际验收。文件 SQLite Desktop 应用级重启、遗留 staging 和 `0001 → 0002` 迁移也没有执行。

本轮 Linux 端没有发现需要继续修改的业务代码。已有 GUI 源码修补和白色 Dashboard 重设计已收口，不机械重复 GUI 重构；`roadmap.md` 的 M6 完成标准已改为明确区分“Windows 构建/启动已验证”和“真实 GUI 验收仍待验证”。

| 当前事项 | 状态 | 当前结论 |
|---|---|---|
| GUI Widget 错误分离/重试、用户级错误标题、语义 Job 列表/时间、Focus-visible、reduced-motion、aria2 Windows gating、白色 Dashboard | `LINUX_VERIFIED`；Windows 构建/启动证据有效 | Linux 回归通过；不把构建/启动替代真实 GUI 验收 |
| Windows WebView2/DPI、Tab/键盘、Focus-visible、Narrator/NVDA、命中区域、最终对比度 | `WINDOWS_VERIFICATION_PENDING` / `BLOCKED` | 缺少 GUI automation native-app target；具备 target 后执行 W-P1-11 |
| 文件 SQLite Desktop 应用重启、遗留 staging、`0001 → 0002` 迁移 | `NOT RUN` | 需要受控 Desktop 应用级专项场景；Repository 单元测试不能替代现场恢复验证 |
| Named Pipe/Registry、externalBin/安装器、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | `WINDOWS_VERIFICATION_PENDING` / `WINDOWS_BLOCKED` / `NOT RUN` | 依赖尚未实现的 Windows backend、发布 artifact、浏览器实机、账号或凭据 |

### Latest Linux validation record（2026-09-10）

| 验证项目 | 状态 | 实际命令/结果 |
|---|---|---|
| Node workspace | PASS | `npm run check && npm run test && npm run build`；Desktop Vite 构建通过，Extension 6/6，通过 |
| Desktop Tauri Linux 构建 | PASS | `npm run build:tauri`；生成 `/home/shiraishi/VSCode Workspace/Tw2Tg/target/release/xarchive-desktop` |
| Rust workspace | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`；69/69 通过 |
| Python Sidecar 静态验证 | PASS | `python3 -m compileall -q sidecar/src sidecar/tests`；通过 |
| Rust clippy | NOT RUN | 当前 Linux 未安装 `cargo-clippy`；沿用最新 Windows 严格 clippy PASS，不将其改记为 Linux PASS |
| Python pytest | NOT RUN | 当前 Linux 未安装 pytest；沿用最新 Windows Sidecar 10/10 PASS，不将其改记为 Linux PASS |

本轮 Linux 验证未发现失败。下一轮 Windows 仍不得提前标记上述真实 GUI 项目为 `WINDOWS_PASS`；只有在 Windows 实际执行并通过对应验收后才能关闭 `WINDOWS_VERIFICATION_PENDING`。

### Linux business backend development follow-up（2026-09-10）

Linux 继续实现当前文档明确缺失的业务后端，新增 `xarchive-download::DownloadRouter`。该 Router 是纯 Rust、无新增依赖的策略编排层：默认执行 gallery-dl；仅当 gallery-dl 返回稳定错误码 `EXTRACT_OR_DOWNLOAD_FAILED`、配置允许 aria2 且调用方提供新鲜 `AddUriRequest` 时才进入 aria2；`AUTH_REQUIRED`、`RATE_LIMITED` 和 `TWEET_NOT_FOUND` 不自动回退。Router 同时保留 gallery-dl 与 aria2 的双失败原因。

| 项目 | 状态 | 当前事实 |
|---|---|---|
| DownloadRouter 策略与错误模型 | `IMPLEMENTED` / `LINUX_VERIFIED` | `cargo test -p xarchive-download` 16/16 通过；覆盖默认 gallery-dl、下载失败 fallback、认证错误不回退、禁用 fallback、aria2 未配置和双失败 |
| Router 与真实 Sidecar/Job 调度接入 | `NOT RUN` | 当前仍需在 Desktop/归档 Job 执行路径中接入；本轮没有把闭包级策略测试误记为端到端完成 |
| 403 后 gallery-dl 重新提取、真实 aria2 transfer 生命周期 | `WINDOWS_VERIFICATION_PENDING` | 需要真实 media URL、Sidecar/aria2 进程和 Windows externalBin/aria2c.exe 场景；Linux Router 单元测试不能替代 Windows 实机验证 |

本轮没有修改 Windows 工作副本代码，也没有将该 Router 标记为 `WINDOWS_PASS`。下一步 Linux 开发应把 Router 接入实际 Job/Sidecar 调度，并补充 fake Sidecar + fake aria2 的端到端测试；接入完成后再执行 Linux 全量回归，并将新增 Windows 平台集成项目继续标记为 `WINDOWS_VERIFICATION_PENDING`。

### Linux Native Host forwarding development follow-up（2026-09-10）

Linux 继续实现当前文档明确缺失的 Native Host 转发后端。`xarchive-native-host` 新增 `forward_request`、`error_response` 和 `request_id` 公共编排函数；主程序读取 `XARCHIVE_PIPE_ENDPOINT`，在未配置时返回 `NATIVE_PIPE_UNAVAILABLE`，配置后尝试以读写方式连接 Desktop endpoint，并将连接/协议错误映射为 `NATIVE_PIPE_ERROR`。Linux fake duplex transport 已验证合法请求转发、响应保留以及非法请求在写入 transport 前被拒绝。

| 项目 | 状态 | 当前事实 |
|---|---|---|
| Native Host framing、BrowserRequest 校验和可插拔请求/响应转发 | `IMPLEMENTED` / `LINUX_VERIFIED` | Native Host crate 8/8 通过；本轮新增后 Rust workspace 最新总数为 79/79；包含 fake transport 成功转发、非法请求边界、request_id 不匹配和协议版本不匹配测试 |
| Windows Named Pipe server/client、endpoint 连接、ACL、生命周期 | `WINDOWS_VERIFICATION_PENDING` / `WINDOWS_BLOCKED` | 当前只实现 endpoint client boundary；Windows Named Pipe 服务端、ACL、多连接、重连和退出行为尚未在 Windows 实机验证 |
| Native Host manifest、Registry、Edge/Chrome 实机加载 | `WINDOWS_VERIFICATION_PENDING` / `NOT RUN` | 仍缺少 manifest/Registry 安装与浏览器实机前置条件 |

本轮 Linux 端没有实现 Windows 专用 server、ACL 或 Registry 代码，也没有把 fake transport 结果标记为 Windows PASS。下一步应在 Linux 补充 Native Host 与 fake Desktop endpoint 的多请求/错误响应测试；在 Windows 前置条件具备后，执行 W-P1-02/W-P1-05 的 Named Pipe、ACL、重连、Registry 和浏览器集成验证。

### Windows validation repeat after current Linux state（2026-09-10）

本轮重新读取并验证当前 Linux 状态。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，既有未提交修改仍包含 GUI 和文档文件，本轮未修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`，环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。使用 Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT` 完成 Linux → Windows 单向同步，6 个关键文件 SHA-256 哈希一致；独立 aria2 测试辅助文件仍保存在 `E:/Shiraishi/VSCode Workspace/Tw2Tg-Windows-DevArtifacts/aria2`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | 首次受限执行环境访问 WSL/E: 被拒绝，使用授权执行后同步成功；Robocopy exit 3 为复制文件并保留目标额外目录的允许结果 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6，无失败 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`；全部通过 |
| Rust workspace tests | PASS（设置环境后） | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后 `cargo test --workspace`；69/69 通过 |
| Python sidecar | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release 构建 | PASS | `npm run build:tauri`；成功生成 `target/release/xarchive-desktop.exe` |
| Tauri Debug 启动与进程清理 | PASS | `npm run dev:tauri` 启动 Vite、Rust 和 Desktop；主动停止后没有 Tw2Tg 相关残留进程 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 仅发现浏览器目标，没有可用的 GUI automation native-app target；构建和启动不能替代真实 GUI 验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前未执行 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

本轮错误与分析：

- 首次在受限执行环境中运行 Robocopy 和 Node 命令时出现 `Access denied` / 找不到 `C:/package.json`；这是执行环境访问边界，不是项目错误。授权后从 E 盘工作副本重跑，结果为 PASS。
- 首次未设置 `PYTHON` 运行 `cargo test --workspace` 时，`xarchive-sidecar-supervisor` 的 `spawn_ready_completes_the_hello_handshake` 和 `communicates_with_a_real_python_worker_when_available` 失败并报 `NotRunning`。最可能原因是 sidecar Python 路径前置条件缺失，非 Windows 业务代码兼容性问题；设置项目 `.venv` Python 后 69/69 通过。
- Rust/Release 构建有非阻塞 MSVC linker stdout warning。Ctrl+C 停止 Tauri Debug 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，随后进程清理正常；本轮不将其判定为项目 FAIL。

本轮没有项目代码导致的 Windows FAIL。Linux 后续事项仍为 GUI 原生实机验收、文件 SQLite Desktop 应用级恢复/迁移，以及 Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie 和真实账号链路。若后续 GUI working tree 或业务代码继续变化，需重新同步并执行对应 Windows 验证；本轮不扩大为开发任务。

### Windows validation repeat after latest Linux-state sync（2026-09-10）

本轮再次针对当前 Linux 最新 working tree 执行验证。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，既有 GUI/文档未提交修改仍在，本轮未修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`，环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。通过 Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT` 完成 Linux → Windows 单向同步，6 个关键文件 SHA-256 哈希一致。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy exit 3；复制文件并保留 E 盘本地额外目录，未同步 `.git`、依赖、缓存、target、数据库、`.env` 或验证报告 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6，均无失败 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`；全部通过 |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后 `cargo test --workspace`；69/69 通过 |
| Python sidecar | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release 构建 | PASS | `npm run build:tauri`；成功生成 `target/release/xarchive-desktop.exe` |
| Tauri Debug 启动与进程清理 | PASS | `npm run dev:tauri` 成功启动 Vite、Rust 和 Desktop；停止后无 Tw2Tg 项目进程残留 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据；独立 aria2 辅助文件仍保存在 E:/Shiraishi/VSCode Workspace/Tw2Tg-Windows-DevArtifacts/aria2 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 返回无可用原生应用目标，并报告浏览器策略加载错误；无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前未执行 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

本轮没有项目代码导致的 Windows `FAIL`。MSVC linker stdout warning 为非阻塞警告；主动 Ctrl+C 停止 Tauri Debug 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，但进程清理正常。Linux 后续仍需处理 GUI 原生实机验收、文件 SQLite Desktop 应用级恢复/迁移及其余 Windows backend/账号前置条件；本轮不扩大为开发任务。

### Windows validation of latest Linux working tree with protocol/backend changes（2026-09-10）

本轮针对当前 Linux 最新 working tree 执行集中 Windows 验证。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，包含既有 GUI、Rust crate、Tauri、协议 schema、文档修改和未跟踪的 `OPENAI_CODEX_WRITING_RULES.md`。本轮不修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`，环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`，exit 3；新增 Rust/Tauri/协议/文档文件和未跟踪规则文件均已同步，关键文件哈希一致，未覆盖依赖、缓存、target、数据库、`.env` 或验证报告 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6 |
| Rust fmt/check | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`；通过 |
| Rust clippy | FAIL | `cargo clippy --workspace --all-targets -- -D warnings`；`desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 有 8 个参数，触发 `clippy::too_many_arguments`（8/7） |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后 `cargo test --workspace`；79/79 通过，包含新增 download fallback、Native Host 和协议回归测试 |
| Python sidecar | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release 构建 | PASS | `npm run build:tauri`；成功生成 `target/release/xarchive-desktop.exe`；构建不执行 clippy，因此不抵销 clippy FAIL |
| Tauri Debug 启动与进程清理 | PASS | `npm run dev:tauri` 启动 Vite、Cargo 和 `xarchive-desktop.exe`；停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 返回 `apps=[]`，没有可用原生 GUI automation target；无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

### Errors and Linux follow-up

- `cargo clippy --workspace --all-targets -- -D warnings` 的 FAIL 属于项目代码质量门禁，不是 Windows-only 环境问题；建议 Linux 后续重构 `run_sidecar_download` 参数对象/上下文，或根据项目规范处理该 lint。由于本轮是验证任务，不在此处修改代码。该 FAIL 不阻塞独立的 Node、Python、workspace test、Tauri build/start 验证，但阻塞“严格 clippy 全通过”的结论。
- Rust/Release 构建的 MSVC linker stdout warning 为非阻塞 warning。Ctrl+C 停止 Tauri Debug 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，但项目进程清理正常。
- GUI 原生验证仍受 Computer Use 无原生 app target 阻塞；文件 SQLite 应用级恢复/迁移、Windows backend、发布安装和真实账号链路仍未执行。

本轮最终结论：适用自动化验证中，Node、Rust fmt/check、workspace tests、Python sidecar、Tauri build/start 均 PASS；严格 Rust clippy 为 FAIL。Linux 后续必须处理 `desktop/src-tauri/src/lib.rs:299` 的 too-many-arguments 问题，然后重新执行 Linux lint 和 Windows clippy；同时继续安排 GUI 原生实机、SQLite 应用级恢复/迁移和其余 Windows backend/账号验证。本轮不扩大为开发任务。

### Windows validation repeat after latest Linux-state sync（第二次重跑，2026-09-10）

本轮再次读取当前 Linux source、Plan 和验证文档，并针对同一最新 working tree 执行 Windows 验证。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，既有 GUI/文档未提交修改仍在，本轮未修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`，环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3 为允许结果，6 个关键文件 SHA-256 哈希一致，Windows 本地目录保留 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6 |
| Rust fmt/check/clippy/test | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、严格 clippy、设置 `PYTHON` 后 `cargo test --workspace`；69/69 通过 |
| Python sidecar | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release/Debug | PASS | `npm run build:tauri` 生成 `target/release/xarchive-desktop.exe`；`npm run dev:tauri` 启动 Vite/Rust/Desktop，停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据；独立辅助文件仍保存在 E:/Shiraishi/VSCode Workspace/Tw2Tg-Windows-DevArtifacts/aria2 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 返回无可用原生应用目标，无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

本轮未发现项目代码导致的 Windows `FAIL`。Rust/Release 构建仍有非阻塞 MSVC linker stdout warning；Ctrl+C 停止 Tauri Debug 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，但进程清理正常。Linux 后续需处理 GUI 原生实机验收、文件 SQLite Desktop 应用级恢复/迁移及其余 Windows backend/账号前置条件；本轮不扩大为开发任务。

### Windows validation repeat after latest Linux working-tree update（第四次重跑，2026-09-10）

本轮重新读取当前 Linux source、Plan 和验证文档，并验证最新 working tree。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，既有 GUI/文档未提交修改仍在，本轮未修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`，环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`，exit 3；6 个关键文件 SHA-256 哈希一致，保留 E 盘本地目录和独立 aria2 辅助文件 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6 |
| Rust fmt/check/clippy/test | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、严格 clippy、设置 `PYTHON` 后 `cargo test --workspace`；69/69 通过 |
| Python sidecar | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release/Debug | PASS | `npm run build:tauri` 生成 `target/release/xarchive-desktop.exe`；`npm run dev:tauri` 启动成功，停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 无可用原生应用目标；无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

本轮未发现项目代码导致的 Windows `FAIL`。MSVC linker stdout warning 仍为非阻塞警告；主动 Ctrl+C 停止 Tauri Debug 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，但项目进程清理正常。Linux 后续仍需处理 GUI 原生实机验收、文件 SQLite Desktop 应用级恢复/迁移及其余 Windows backend/账号前置条件；本轮不扩大为开发任务。

### Windows validation repeat after latest Linux working-tree update（第五次重跑，2026-09-10）

本轮重新读取当前 Linux source、Plan 和验证文档，并针对最新 working tree 完成 Windows 验证。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；验证开始时 working tree 非 clean，既有 GUI/文档未提交修改仍在，本轮未修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`，环境为 Windows NT 10.0.29661.0、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`，exit 3；6 个关键文件 SHA-256 哈希一致，保留 E 盘本地目录和 aria2 辅助文件 |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 6/6 |
| Rust fmt/check/clippy/test | PASS | `cargo fmt --all -- --check`、`cargo check --workspace`、严格 clippy、设置 `PYTHON` 后 `cargo test --workspace`；69/69 通过 |
| Python sidecar | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release/Debug | PASS | `npm run build:tauri` 生成 `target/release/xarchive-desktop.exe`；`npm run dev:tauri` 启动成功，停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前返回 `apps=[]`，没有可用原生 GUI automation target；无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

本轮未发现项目代码导致的 Windows `FAIL`。MSVC linker stdout warning 为非阻塞警告；主动 Ctrl+C 停止 Tauri Debug 时出现 Chromium `Error = 1411` / `STATUS_CONTROL_C_EXIT`，但项目进程清理正常。Linux 后续仍需处理 GUI 原生实机验收、文件 SQLite Desktop 应用级恢复/迁移及其余 Windows backend/账号前置条件；本轮不扩大为开发任务。

### Windows validation after latest Linux working-tree update（2026-09-10）

本轮再次以当前 Linux source 为唯一来源读取仓库状态、Plan 和验证文档，并将最新 working tree 单向同步到 Windows 验证副本。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；working tree 非 clean，包含既有 Rust、Tauri、前端、协议、文档和未跟踪 `OPENAI_CODEX_WRITING_RULES.md` 修改。本轮没有修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows NT `10.0.29661.0`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3（仅表示存在复制/跳过项，允许）；本轮关键源文件 SHA-256 哈希一致，保留 E 盘本地依赖、缓存、验证产物和 aria2 辅助文件 |
| Node check | PASS | `npm run check`；完成且 exit 0 |
| Node tests | PASS | `npm run test`；Extension 6/6，Desktop Node 测试无失败 |
| Node production build | PASS | `npm run build`；Vite 35 modules，产物构建完成 |
| Rust formatter | FAIL | `cargo fmt --all -- --check`；`crates/xarchive-core/src/job.rs` 与 `crates/xarchive-storage/src/lib.rs` 存在 rustfmt 差异 |
| Rust check | PASS | `cargo check --workspace`；完成且 exit 0 |
| Rust strict clippy | FAIL | `cargo clippy --workspace --all-targets -- -D warnings`；`desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 有 8 个参数，触发 `clippy::too_many_arguments`（8/7） |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后执行 `cargo test --workspace`；79/79 通过，doc-tests 通过 |
| Python sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit 0 |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；开发应用成功启动；Ctrl+C 后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前返回 `apps=[]`，没有可用原生 GUI automation target；无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前仅完成库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

#### 错误分析与 Linux 后续事项

1. `cargo fmt --check` 的 FAIL 是当前 working tree 的格式未整理问题，涉及 `xarchive-core/src/job.rs` 和 `xarchive-storage/src/lib.rs`；不是 Windows 专属问题。Linux 后续需先按项目约定重新运行 formatter 并确认 diff。
2. 严格 clippy 的 FAIL 是项目代码质量门禁问题，不是 Windows 环境缺陷。Linux 后续需处理 `desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 参数过多（建议重构参数上下文或采用项目批准的 lint 处理），然后重新执行 Linux lint 和 Windows clippy；本轮不直接修复。
3. Tauri Release/Debug 构建和启动均通过。MSVC linker stdout warning 为非阻塞警告；停止 Debug 时出现 Chromium `Error = 1411`，并返回 `STATUS_CONTROL_C_EXIT`，但这是主动 Ctrl+C 停止产生的现象，项目进程已清理。
4. GUI 原生实机、SQLite Desktop 应用级恢复/迁移、Named Pipe/Registry/安装器/Tray、浏览器 Cookie、真实 X/Telegram 账号链路仍需具备相应 Windows 实机和凭据后再执行。本轮未将这些前置条件不足误记为 PASS，也未扩大为开发任务。

### Windows validation after latest Linux working-tree sync（2026-09-10 22:54）

本轮再次按 `docs/development/cross-platform-validation.md` 读取 Linux source、Plan、Windows 验证规范和既有报告，并针对当前 working tree 执行 Windows 验证。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；working tree 非 clean，包含既有 Rust、Tauri、前端、协议、文档和未跟踪 `OPENAI_CODEX_WRITING_RULES.md` 修改。本轮没有修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows NT `10.0.29661.0`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3 为允许结果；本轮同步后关键源文件 SHA-256 哈希全部一致，保留 E 盘本地 `.venv`、`node_modules`、`target`、验证产物和 aria2 辅助文件 |
| Node check | PASS | `npm run check`；exit 0 |
| Node tests | PASS | `npm run test`；Extension 6/6，Desktop Node 测试无失败 |
| Node production build | PASS | `npm run build`；Vite 35 modules，构建完成 |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit 0 |
| Rust check | PASS | `cargo check --workspace`；exit 0 |
| Rust strict clippy | FAIL | `cargo clippy --workspace --all-targets -- -D warnings`；`desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 有 8 个参数，触发 `clippy::too_many_arguments`（8/7） |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后执行 `cargo test --workspace`；79/79 通过，doc-tests 通过 |
| Python sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit 0 |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动；停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前无可用原生应用目标（`apps=[]`；仅有浏览器标签），无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前仅完成库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

#### 错误分析与 Linux 后续事项

1. 严格 clippy 的 FAIL 是项目代码质量门禁问题，不是 Windows 环境缺陷。当前 Plan 曾记录 Windows clippy 已通过，但本轮针对最新 working tree 的实际结果为 FAIL；Linux 后续必须重新 reconcile Plan/代码状态，处理 `desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 参数过多问题，然后重新执行 Linux lint 和 Windows clippy。本轮不直接修复。
2. Tauri Release/Debug 构建和启动均通过。构建期间的 MSVC linker stdout warning 为非阻塞警告；主动 Ctrl+C 停止 Debug 时出现 Chromium `Error = 1411`，并返回 `STATUS_CONTROL_C_EXIT`，但项目进程已清理。
3. GUI 原生实机、SQLite Desktop 应用级恢复/迁移、Named Pipe/Registry/安装器/Tray、浏览器 Cookie、真实 X/Telegram 账号链路仍需相应 Windows 实机和凭据。本轮未将这些前置条件不足误记为 PASS，也未扩大为开发任务。

### Windows validation after Linux revision update（2026-09-11 10:03）

本轮再次按 `docs/development/cross-platform-validation.md` 读取 Linux source、Plan、Windows 验证规范和既有报告，并针对最新 Linux revision 执行 Windows 验证。source branch 为 `main`，HEAD 为 `9334d1472843babae8910c02cb93fd7039e82387`；验证开始时 working tree 仅有未跟踪 `OPENAI_CODEX_WRITING_RULES.md`，本轮没有修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows NT `10.0.29661.0`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3 为允许结果；关键源文件 SHA-256 哈希全部一致，保留 E 盘本地 `.venv`、`node_modules`、`target`、验证产物和 aria2 辅助文件 |
| Node check | PASS | `npm run check`；完成且无错误 |
| Node tests | PASS | `npm run test`；Extension 6/6，Desktop Node 测试无失败 |
| Node production build | PASS | `npm run build`；Vite 35 modules，构建完成 |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit 0 |
| Rust check | PASS | `cargo check --workspace`；exit 0 |
| Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；exit 0 |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后执行 `cargo test --workspace`；79/79 通过，doc-tests 通过 |
| Python sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit 0 |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动；停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前无可用原生应用目标（`apps=[]`；仅有浏览器标签），无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前仅完成库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

#### 错误分析与 Linux 后续事项

1. 本轮适用的 Node、Rust、Python 和 Tauri 自动化验证均无 FAIL；之前的 `run_sidecar_download` clippy 问题已随当前 Linux revision 处理，Plan 中对应的 Windows clippy PASS 已得到实际重新验证。
2. Tauri 构建期间出现 MSVC linker stdout warning，为非阻塞警告；主动 Ctrl+C 停止 Debug 时出现 Chromium `Error = 1411`，并返回 `STATUS_CONTROL_C_EXIT`，但项目进程已清理，未观察到持续性 Windows 运行时故障。
3. Linux 后续无需因本轮自动化 FAIL 修复业务代码；仍需在具备 Windows 原生 GUI 自动化目标、Desktop 应用级 SQLite 场景、发布/安装实机、浏览器 Cookie 环境及 X/Telegram 账号凭据后，补做上述 BLOCKED / NOT RUN 项目。本轮不扩大为开发任务。

### Windows validation after Linux revision update（2026-09-11 09:33）

本轮再次按 `docs/development/cross-platform-validation.md` 读取 Linux source、Plan、Windows 验证规范和既有报告，并针对最新 Linux revision 执行 Windows 验证。source branch 为 `main`，HEAD 为 `9334d1472843babae8910c02cb93fd7039e82387`；working tree 仅有未跟踪 `OPENAI_CODEX_WRITING_RULES.md`，本轮没有修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows NT `10.0.29661.0`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3 为允许结果；关键源文件 SHA-256 哈希全部一致，保留 E 盘本地 `.venv`、`node_modules`、`target`、验证产物和 aria2 辅助文件 |
| Node check | PASS | `npm run check`；exit 0 |
| Node tests | PASS | `npm run test`；Extension 6/6，Desktop Node 测试无失败 |
| Node production build | PASS | `npm run build`；Vite 35 modules，构建完成 |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit 0 |
| Rust check | PASS | `cargo check --workspace`；exit 0 |
| Rust strict clippy | FAIL | `cargo clippy --workspace --all-targets -- -D warnings`；`desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 有 8 个参数，触发 `clippy::too_many_arguments`（8/7） |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后执行 `cargo test --workspace`；79/79 通过，doc-tests 通过 |
| Python sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit 0 |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动；停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前无可用原生应用目标（`apps=[]`；仅有浏览器标签），无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前仅完成库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

#### 错误分析与 Linux 后续事项

1. 严格 clippy 的 FAIL 是项目代码质量门禁问题，不是 Windows 环境缺陷。Plan 仍记录 Windows clippy 已通过，但本轮针对最新 revision 的实际结果仍为 FAIL；Linux 后续需 reconcile Plan/代码状态，处理 `desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 参数过多问题，再重新执行 Linux lint 和 Windows clippy。本轮不直接修复。
2. Tauri Release/Debug 构建和启动均通过。MSVC linker stdout warning 为非阻塞警告；主动 Ctrl+C 停止 Debug 时出现 Chromium `Error = 1411`，并返回 `STATUS_CONTROL_C_EXIT`，但项目进程已清理。
3. GUI 原生实机、SQLite Desktop 应用级恢复/迁移、Named Pipe/Registry/安装器/Tray、浏览器 Cookie、真实 X/Telegram 账号链路仍需相应 Windows 实机和凭据。本轮未将这些前置条件不足误记为 PASS，也未扩大为开发任务。

### Windows validation after latest Linux working-tree sync（2026-09-10 23:06）

本轮再次按 `docs/development/cross-platform-validation.md` 读取 Linux source、Plan、Windows 验证规范和既有报告，并针对当前 working tree 执行 Windows 验证。source branch 为 `main`，HEAD 为 `34b5c6786ebb5f31621a45d332c8bf2b51c06bd6`；working tree 非 clean，包含既有 Rust、Tauri、前端、协议、文档和未跟踪 `OPENAI_CODEX_WRITING_RULES.md` 修改。本轮没有修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows NT `10.0.29661.0`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3 为允许结果；关键源文件 SHA-256 哈希全部一致，保留 E 盘本地 `.venv`、`node_modules`、`target`、验证产物和 aria2 辅助文件 |
| Node check | PASS | `npm run check`；exit 0 |
| Node tests | PASS | `npm run test`；Extension 6/6，Desktop Node 测试无失败 |
| Node production build | PASS | `npm run build`；Vite 35 modules，构建完成 |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit 0 |
| Rust check | PASS | `cargo check --workspace`；exit 0 |
| Rust strict clippy | FAIL | `cargo clippy --workspace --all-targets -- -D warnings`；`desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 有 8 个参数，触发 `clippy::too_many_arguments`（8/7） |
| Rust workspace tests | PASS | 设置 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后执行 `cargo test --workspace`；79/79 通过，doc-tests 通过 |
| Python sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit 0 |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动；停止后项目进程为 0 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前无可用原生应用目标（`apps=[]`；仅有浏览器标签），无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前仅完成库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

#### 错误分析与 Linux 后续事项

1. 严格 clippy 的 FAIL 是项目代码质量门禁问题，不是 Windows 环境缺陷。当前 Plan 仍记录 Windows clippy 已通过，但本轮针对最新 working tree 的实际结果为 FAIL；Linux 后续必须 reconcile Plan/代码状态，处理 `desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 参数过多问题，然后重新执行 Linux lint 和 Windows clippy。本轮不直接修复。
2. Tauri Release/Debug 构建和启动均通过。MSVC linker stdout warning 为非阻塞警告；主动 Ctrl+C 停止 Debug 时出现 Chromium `Error = 1411`，并返回 `STATUS_CONTROL_C_EXIT`，但项目进程已清理。
3. GUI 原生实机、SQLite Desktop 应用级恢复/迁移、Named Pipe/Registry/安装器/Tray、浏览器 Cookie、真实 X/Telegram 账号链路仍需相应 Windows 实机和凭据。本轮未将这些前置条件不足误记为 PASS，也未扩大为开发任务。

### Linux reconciliation after Windows clippy failure（2026-09-11）

Linux 已按 `docs/development/cross-platform-validation.md` 重新读取本轮 Windows 结果，并处理其中唯一明确属于项目代码的失败：`desktop/src-tauri/src/lib.rs:299` 的 `run_sidecar_download` 因 8 个参数触发 `clippy::too_many_arguments`。该问题不是 Windows-specific 行为，也不是 `WINDOWS_VERIFICATION_BLOCKING`；它已在 Linux 侧通过 `SidecarDownloadRequest` 参数上下文结构完成最小重构，保持 Sidecar 命令字段、事件过滤和归档行为不变。

本轮 Linux 验证结果：

| 验证项目 | 状态 | 实际命令/结果 |
|---|---|---|
| Rust formatter | PASS | `cargo fmt --all -- --check` |
| Rust workspace check | PASS | `cargo check --workspace` |
| Rust workspace tests | PASS | `cargo test --workspace`；79 项 crate 测试及 doc-tests 通过 |
| Node check/build | PASS | `npm run check`、`npm run build` |
| Extension tests | PASS | `npm test` |
| Linux clippy | NOT RUN | 本机 stable toolchain 未安装 `cargo-clippy` |
| Git diff check | PASS | `git diff --check` |

该修复尚未在 Windows 实际重新执行严格 clippy，因此相关状态更新为：

| Windows 项目 | 状态 | 说明 |
|---|---|---|
| Windows strict clippy after `SidecarDownloadRequest` refactor | `WINDOWS_VERIFICATION_PENDING` | 必须在 Windows 工作副本同步最新 Linux 状态后执行 `cargo clippy --workspace --all-targets -- -D warnings`；Linux 通过不能替代 Windows 复验 |
| Windows Node/Rust build/test、Python Sidecar、Tauri build/start | `WINDOWS_VERIFICATION_PENDING` | 当前 Linux working tree 含业务代码变更，上一轮对旧状态的 PASS 不能自动覆盖本轮新 revision |
| GUI、文件 SQLite 应用级恢复/迁移、Named Pipe/Registry、externalBin/安装器、Credential Manager、Edge Cookie、真实 X/Telegram | `WINDOWS_VERIFICATION_PENDING` / `BLOCKED` / `NOT RUN` | 继续受各自 automation、backend、artifact、浏览器实机或账号前置条件约束 |

本轮没有 `WINDOWS_VERIFICATION_BLOCKING` 项目。Windows 严格 clippy 复验是下一次集中 Windows validation 的必做项，但不是继续 Linux 设计或实现的硬性前置；其余可在 Linux 完成的工作仍可按当前 Plan 继续。历史 Windows clippy `FAIL` 记录保留，不被本次 Linux 修复改写为 Windows `PASS`。

### Linux reliability batch after latest Windows results（2026-09-11）

根据上一轮 Windows 验证暴露的项目代码问题和当前 Plan，Linux 侧继续完成了一个不依赖 Windows 结果的可靠性批次：

- `run_sidecar_download` 已使用 `SidecarDownloadRequest` 上下文结构收敛参数，修复 Windows 严格 clippy 报告的 `too_many_arguments`；
- `archive_tweet` 不再忽略 `DownloadRouter` 返回值，也不再通过 `expect` 处理下载失败；
- gallery-dl/aria2 路由失败会映射到 `AUTH_REQUIRED` 或 `FAILED`，并写入 `last_error_code`/`last_error_message`；
- 归档流程新增 `JOB_CREATED`、`DOWNLOAD_STARTED`、`DOWNLOAD_FAILED` 和 `DOWNLOAD_COMPLETED` 事件持久化；
- Router 仍使用默认 gallery-dl 路径，真实 aria2 `AddUriRequest`、403 后重新提取 URL 和 transfer 生命周期尚未实现，不将本批次视为 Windows 端到端通过。

本轮 Linux 验证：`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace`（79 项 crate 测试及 doc-tests）、`npm test`（Extension 6/6）、`npm run check`、`npm run build` 和 `git diff --check` 均通过；本机未安装 `cargo-clippy`，Linux clippy 为 `NOT RUN`。

受本批次业务代码变更影响的 Windows 项目必须在下一次同步后重新执行，并保持 `WINDOWS_VERIFICATION_PENDING`：

| 项目 | 状态 | 下一步 |
|---|---|---|
| Windows strict clippy after `SidecarDownloadRequest` refactor | `WINDOWS_VERIFICATION_PENDING` | 执行 `cargo clippy --workspace --all-targets -- -D warnings`；不得使用 Linux 结果替代 |
| Desktop Router/Sidecar/Job failure and event persistence | `WINDOWS_VERIFICATION_PENDING` | 使用 Windows Desktop/Sidecar 场景确认无 panic、失败状态、事件历史和进程清理 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | `WINDOWS_VERIFICATION_PENDING` | 准备受控 `aria2c.exe`、media server 和可重复的过期 URL 场景 |

本轮仍无 `WINDOWS_VERIFICATION_BLOCKING`。GUI、文件 SQLite 应用级恢复/迁移、Named Pipe/Registry、externalBin/安装器、Credential Manager、Edge Cookie 和真实 X/Telegram 等项目继续按既有前置条件保持 `WINDOWS_VERIFICATION_PENDING`、`BLOCKED` 或 `NOT RUN`，不得提前标记为 `WINDOWS_PASS`。

### Windows validation of committed Linux reliability batch（2026-09-11 13:00）

本轮针对 Linux 最新提交重新执行 Windows 验证。source branch 为 `main`，HEAD 为 `040b98232f8475c9c4f19b98679e808aed800ab5`；验证开始时 working tree 仅有未跟踪 `OPENAI_CODEX_WRITING_RULES.md`，本轮没有修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows 11 Insider Preview `10.0.29661.0`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`、Cargo `1.98.0`、Python `3.14.7`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux source → Windows E: 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3 为允许结果；关键源文件 SHA-256 比对 `KEY_HASH_MISMATCHES=0`，保留 E 盘本地 `.venv`、`node_modules`、`target`、验证产物和独立 aria2 辅助目录 |
| Node check | PASS | `npm run check`；Vite 35 modules，exit 0 |
| Node tests | PASS | `npm run test`；Extension 6/6，Desktop Node 测试 0 failures |
| Node production build | PASS | `npm run build`；Vite 35 modules，exit 0 |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit 0 |
| Rust check | PASS | `cargo check --workspace`；exit 0 |
| Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；exit 0 |
| Rust workspace tests（默认 Windows 命令环境） | FAIL | `cargo test --workspace`；`xarchive-sidecar-supervisor` 的 `spawn_ready_completes_the_hello_handshake` 与 `communicates_with_a_real_python_worker_when_available` 返回 `NotRunning`，exit 101 |
| Rust workspace tests（项目 Python 环境） | PASS | 设置进程级 `PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后执行 `cargo test --workspace`；79/79 crate 测试通过，doc-tests 通过 |
| Python sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit 0 |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动；主动停止后项目进程为 0 |
| Tauri installer/package | NOT APPLICABLE | 当前 `desktop/src-tauri/tauri.conf.json` 为 `bundle.active=false`，本轮没有启用安装包产物 |
| Desktop Router/Sidecar/Job failure and event persistence | NOT RUN | 当前自动化只覆盖 crate/unit 测试和 Desktop 启动，缺少应用级失败状态、事件历史和无 panic 场景 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改既有 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据仅覆盖既有 aria2 核心 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | NOT RUN | 缺少受控 `aria2c.exe`、媒体服务和可重复的过期 URL 场景；当前 reliability batch 尚未实现完整真实链路 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前无可用原生应用目标（`apps=[]`；仅有浏览器标签），无法执行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前仅完成库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

#### 错误分析与 Linux 后续事项

1. 默认 Windows 命令环境下的 `cargo test --workspace` 是可复现的环境配置 FAIL：`PYTHON` 未设置时，测试默认调用 `python3`；本机 `where.exe python3` 仅解析到 `C:\Users\Shiraishi\AppData\Local\Microsoft\WindowsApps\python3.exe`，直接执行返回 9009。Windows venv Python 直接探测正常，设置项目级 `PYTHON` 后 supervisor 两个握手测试和完整 79 项 workspace 测试均通过。因此当前主要分类为 Windows 环境/测试默认命令解析问题，不据此修改业务代码；Linux 后续应在开发或 CI 配置中明确 Windows Python 解释器前置条件，并在后续 Windows 验证中继续使用项目 Python 环境复验。
2. Rust/Release 构建期间出现 MSVC linker stdout warning（生成 `.lib/.exp` 文件），为非阻塞警告。主动 Ctrl+C 停止 Tauri Debug 时出现 Chromium `Error = 1411`，并返回 `STATUS_CONTROL_C_EXIT`；这是主动中止烟测的退出信号，项目进程随后已清理，未观察到持续性运行时故障。
3. Linux 后续需处理 Desktop Router/Sidecar/Job 失败与事件持久化的应用级场景，并准备受控 `aria2c.exe`、媒体服务和过期 URL 场景，验证真实 aria2 fallback/403 refresh/transfer lifecycle；同时在具备原生 GUI、Desktop 应用级 SQLite、Named Pipe/Registry、安装器、浏览器和账号凭据前置条件后补做相应项目。本轮不扩大为开发任务。

本轮没有稳定的项目代码 `FAIL`；唯一失败是默认 `python3` Windows 别名导致的测试环境问题。Linux 源与 E 盘关键文件已完成哈希一致性检查，验证期间未向 Linux 反向同步代码或构建产物。

### Windows validation of Linux reliability batch（2026-09-11 10:09）

本轮针对 Linux 最新 reliability batch 重新执行 Windows 验证。source branch 为 `main`，HEAD 为 `9334d1472843babae8910c02cb93fd7039e82387`；验证时 working tree 包含 `SidecarDownloadRequest`、下载失败状态/事件持久化等未提交代码和文档修改，以及未跟踪 `OPENAI_CODEX_WRITING_RULES.md`。这些 working-tree changes 在本轮验证操作之外产生，本轮没有修改业务代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows NT `10.0.29661.0`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`、Python `3.14.7`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux working tree → Windows 同步 | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit 3 为允许结果；关键源文件 SHA-256 哈希不匹配数为 0，保留 E 盘本地 `.venv`、`node_modules`、`target`、验证产物和 aria2 辅助文件 |
| Node check | PASS | `npm run check`；exit 0 |
| Node tests | PASS | `npm run test`；Extension 6/6，Desktop Node 测试无失败 |
| Node production build | PASS | `npm run build`；Vite 35 modules，构建完成 |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit 0 |
| Rust check | PASS | `cargo check --workspace`；exit 0 |
| Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；exit 0 |
| Rust workspace tests | PASS（含一次间歇性 FAIL） | 首次完整运行有 2 个 sidecar supervisor 握手测试超时；定向重跑 `cargo test -p xarchive-sidecar-supervisor --lib -- --nocapture` 为 4/4，随后再次完整 `cargo test --workspace` 为 79/79，doc-tests 通过 |
| Python sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；10/10 通过 |
| Python hello 直连复现 | PASS | 使用 Windows venv Python 直接发送 hello JSONL，正确返回 `ready`，exit 0 |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit 0 |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动；停止后项目进程为 0 |
| Desktop Router/Sidecar/Job failure and event persistence | NOT RUN | 当前仅覆盖 crate/unit 测试和 Desktop 启动，缺少应用级失败状态、事件历史和无 panic 场景 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改既有 aria2 核心逻辑，沿用 `d239a1d` 的 Windows PASS 证据仅覆盖既有 aria2 核心 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | NOT RUN | 缺少受控 `aria2c.exe`、媒体服务和可重复的过期 URL 场景；且当前 batch 尚未实现完整真实链路 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer Use 当前无可用原生应用目标（`apps=[]`；仅有浏览器标签），无法进行真实 GUI 渲染和交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 当前仅完成库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin/安装器、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号/凭据或自动化前置条件 |

#### 错误分析与 Linux 后续事项

1. 首次 `cargo test --workspace` 的 `xarchive-sidecar-supervisor` 中 `communicates_with_a_real_python_worker_when_available` 和 `spawn_ready_completes_the_hello_handshake` 失败，分别表现为未收到 `Ready` 和 `HandshakeTimeout`。Windows venv Python 直连 hello 正常，定向重跑及随后完整 workspace 重跑均通过，因此当前判断为未稳定复现的进程启动/握手时序或环境瞬态问题，不能据此认定稳定的业务代码 FAIL；仍保留该失败证据，后续若复现应重点检查 Windows child-process 启动和握手等待路径。
2. Tauri Release/Debug 构建和启动均通过。MSVC linker stdout warning 为非阻塞警告；主动 Ctrl+C 停止 Debug 时出现 Chromium `Error = 1411`，并返回 `STATUS_CONTROL_C_EXIT`，但项目进程已清理。
3. Linux 后续无需因本轮最终自动化结果修改业务代码；仍需准备受控 `aria2c.exe`、媒体服务和过期 URL 场景，验证真实 aria2 fallback/403 refresh/transfer lifecycle，并在具备原生 GUI、Desktop 应用级失败/事件和 SQLite 场景、安装器、浏览器及账号凭据后完成相应 BLOCKED / NOT RUN 项目。本轮不扩大为开发任务。

### Windows validation of Linux HEAD 040b982（2026-09-11 13:45 +08:00）

本轮重新针对 Linux 最新状态执行 Windows 验证。source branch 为 `main`，HEAD 为 `040b98232f8475c9c4f19b98679e808aed800ab5`；验证开始时 working tree 包含既有 `docs/development/windows-validation.md` 文档修改和未跟踪 `OPENAI_CODEX_WRITING_RULES.md`，无未提交业务代码，本轮没有修改 Linux 源代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows 11 专业工作站版 Insider Preview `10.0.29661`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`、Cargo `1.98.0`、Python `3.14.7`、pytest `9.1.1`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux source → Windows E: 同步 | PASS | 使用 Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`，exit `3`；无 failed files/mismatches，关键源文件 SHA-256 比对 `KEY_HASH_MISMATCHES=0`；保留 E 盘 `.venv`、`node_modules`、`target`、`validation-artifacts`、`X-Archive`、生成目录和未知本地文件 |
| Node check | PASS | `npm run check`；Vite `35 modules`，exit `0` |
| Node tests | PASS | `npm run test`；Extension `6/6`，Desktop Node 测试 `0` 项且无失败 |
| Node production build | PASS | `npm run build`；Vite `35 modules`，exit `0` |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit `0` |
| Rust check | PASS | `cargo check --workspace`；exit `0` |
| Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；exit `0` |
| Rust workspace tests（默认 Windows 命令环境） | FAIL | `cargo test --workspace`；未设置 `PYTHON` 时 `xarchive-sidecar-supervisor` 的 `communicates_with_a_real_python_worker_when_available`、`spawn_ready_completes_the_hello_handshake` 返回 `NotRunning`，exit `1` |
| Rust workspace tests（项目 Python 前置条件） | PASS | `$env:PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe; cargo test --workspace`；79/79 crate tests 通过，doc-tests 通过 |
| Python Sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；`10 passed` |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，约 `16.6 MB`，exit `0` |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动并观察到 `xarchive-desktop.exe`，受控停止后项目进程数为 `0` |
| Tauri installer/package | NOT APPLICABLE | `desktop/src-tauri/tauri.conf.json` 的 `bundle.active=false`，本轮没有启用安装包产物 |
| Desktop Router/Sidecar/Job failure and event persistence | NOT RUN | 现有自动化覆盖 crate/unit tests 和 Desktop 启动，但没有应用级失败状态、事件历史和无 panic 场景；不以启动烟测替代 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改既有 aria2 核心逻辑，沿用历史 Windows PASS 证据；不重复执行 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | NOT RUN | 缺少受控 `aria2c.exe`、媒体服务和可重复的过期 URL 场景，且完整真实业务链路仍未实现 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer-use helper 两次启动均因 `helper_unknown_error: setup refresh had errors` 异常退出，当前没有可验证的原生 GUI target；未进行交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 本轮仅有库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号或凭据前置条件 |

#### 错误分析与 Linux 后续事项

1. 默认 Windows 命令环境下的 workspace test `FAIL` 属于环境前置条件问题：`PYTHON` 为空，真实 worker 测试无法启动解释器并返回 `NotRunning`。同一 E: 工作副本设置 `PYTHON` 指向项目 `.venv` 后 79/79 通过，因此本轮不修改业务代码；Linux/CI 后续应明确 Windows Python 解释器前置条件，并继续保留“默认环境 FAIL、项目环境 PASS”的证据边界。
2. Release/Debug 构建和启动均通过。MSVC linker stdout 的 `.lib/.exp` 生成信息为非阻塞 warning；主动 Ctrl+C 停止 Debug 时出现 Chromium `Error = 1411` 和 `STATUS_CONTROL_C_EXIT`，但项目进程已清理，未观察到持续运行时故障。
3. 本轮没有稳定的项目代码 `FAIL`。Linux 后续仍需为 Desktop Router/Sidecar/Job 失败状态与事件历史补充应用级 Windows 场景，并准备受控 aria2/media-server/expired-URL 场景；具备原生 GUI、应用级 SQLite、Named Pipe/Registry、安装器、浏览器和账号凭据前置条件后，再执行当前 BLOCKED/NOT RUN 项目。

本轮未将默认 Python 环境失败误记为业务代码失败，也未将 Linux 单元测试、Tauri 启动或静态 GUI 结构检查提升为真实 Windows GUI/端到端验收；验证期间没有向 Linux 反向同步代码或构建产物。

### Windows validation repeat of Linux HEAD 040b982（2026-09-11 23:36 +08:00）

本轮再次针对 Linux 最新状态执行 Windows 验证。source branch 为 `main`，HEAD 为 `040b98232f8475c9c4f19b98679e808aed800ab5`；验证开始时 working tree 包含既有 `docs/development/windows-validation.md` 文档修改和未跟踪 `OPENAI_CODEX_WRITING_RULES.md`，无未提交业务代码，本轮没有修改 Linux 源代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows 11 专业工作站版 Insider Preview `10.0.29661`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`、Cargo `1.98.0`、Python `3.14.7`、pytest `9.1.1`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux source → Windows E: 同步 | PASS | 先 dry-run，再使用 Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit `3`，无 failed files/mismatches，关键源文件 SHA-256 比对 `KEY_HASH_MISMATCHES=0`；保留 E 盘 `.venv`、`node_modules`、`target`、`validation-artifacts`、`X-Archive`、生成目录和未知本地文件 |
| Node check | PASS | `npm run check`；Vite `35 modules`，exit `0` |
| Node tests | PASS | `npm run test`；Extension `6/6`，Desktop Node 测试 `0` 项且无失败 |
| Node production build | PASS | `npm run build`；Vite `35 modules`，exit `0` |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit `0` |
| Rust check | PASS | `cargo check --workspace`；exit `0` |
| Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；exit `0` |
| Rust workspace tests（默认 Windows 命令环境） | FAIL | `cargo test --workspace`；`PYTHON` 为空，`xarchive-sidecar-supervisor` 的 `communicates_with_a_real_python_worker_when_available`、`spawn_ready_completes_the_hello_handshake` 返回 `NotRunning`，命令非零退出 |
| Rust workspace tests（项目 Python 前置条件） | PASS | `$env:PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe; cargo test --workspace`；79/79 crate tests 通过，doc-tests 通过 |
| Python Sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；`10 passed` |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，约 `16.6 MB`，exit `0` |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动并观察到 `xarchive-desktop.exe`，受控停止后项目进程数为 `0` |
| Tauri installer/package | NOT APPLICABLE | `desktop/src-tauri/tauri.conf.json` 的 `bundle.active=false`，本轮没有启用安装包产物 |
| Desktop Router/Sidecar/Job failure and event persistence | NOT RUN | 现有自动化覆盖 crate/unit tests 和 Desktop 启动，但没有应用级失败状态、事件历史和无 panic 场景；不以启动烟测替代 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改既有 aria2 核心逻辑，沿用历史 Windows PASS 证据；不重复执行 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | NOT RUN | 缺少受控 `aria2c.exe`、媒体服务和可重复的过期 URL 场景，且完整真实业务链路仍未实现 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer-use helper 首次调用和重置后重试均因 `helper_unknown_error: setup refresh had errors` 异常退出；未进行 GUI 交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 本轮仅有库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号或凭据前置条件 |

#### 错误分析与 Linux 后续事项

1. 默认 Windows 命令环境下的 workspace test `FAIL` 仍是环境前置条件问题：`PYTHON` 未设置，真实 worker 测试无法启动并返回 `NotRunning`；设置项目 `.venv` Python 后同一工作副本的 79/79 workspace tests 通过。本轮不修改业务代码；Linux/CI 后续应明确 Windows Python 解释器前置条件。
2. Release/Debug 构建和启动均通过。MSVC linker stdout 的 `.lib/.exp` 生成信息为非阻塞 warning；主动 Ctrl+C 停止 Debug 时出现 Chromium `Error = 1411` 和 `STATUS_CONTROL_C_EXIT`，但项目进程已清理，未观察到持续运行时故障。
3. 本轮没有稳定的项目代码 `FAIL`。Linux 后续仍需补充 Desktop Router/Sidecar/Job 失败状态与事件历史的应用级 Windows 场景，准备受控 aria2/media-server/expired-URL 场景，并在具备原生 GUI、应用级 SQLite、Named Pipe/Registry、安装器、浏览器和账号凭据前置条件后重新执行当前 BLOCKED/NOT RUN 项目。

本轮未将默认 Python 环境失败误记为业务代码失败，也未将 Linux 单元测试、Tauri 启动或静态 GUI 结构检查提升为真实 Windows GUI/端到端验收；验证期间没有向 Linux 反向同步代码或构建产物。
### Windows validation repeat of Linux HEAD 040b982（2026-09-12 09:55 +08:00）

本轮再次针对 Linux 最新状态执行 Windows 验证。source branch 为 `main`，HEAD 为 `040b98232f8475c9c4f19b98679e808aed800ab5`；验证开始时 working tree 包含既有 `docs/development/windows-validation.md` 文档修改和未跟踪 `OPENAI_CODEX_WRITING_RULES.md`，无未提交业务代码，本轮没有修改 Linux 源代码。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`；环境为 Windows 11 专业工作站版 Insider Preview `10.0.29661`、AMD64、Node `v24.19.0`、npm `11.17.0`、Rust `1.98.0`、Cargo `1.98.0`、Python `3.14.7`、pytest `9.1.1`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux source → Windows E: 同步 | PASS | 先 dry-run，再使用 Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；exit `3`，无 failed files/mismatches，关键源文件 SHA-256 比对 `KEY_HASH_MISMATCHES=0`；保留 E 盘 `.venv`、`node_modules`、`target`、`validation-artifacts`、`X-Archive`、生成目录和未知本地文件 |
| Node check | PASS | `npm run check`；Vite `35 modules`，exit `0` |
| Node tests | PASS | `npm run test`；Extension `6/6`，Desktop Node 测试 `0` 项且无失败 |
| Node production build | PASS | `npm run build`；Vite `35 modules`，exit `0` |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit `0` |
| Rust check | PASS | `cargo check --workspace`；exit `0` |
| Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；exit `0` |
| Rust workspace tests（默认 Windows 命令环境） | FAIL | `cargo test --workspace`；`PYTHON` 为空，`xarchive-sidecar-supervisor` 的 `communicates_with_a_real_python_worker_when_available`、`spawn_ready_completes_the_hello_handshake` 返回 `NotRunning`，命令非零退出 |
| Rust workspace tests（项目 Python 前置条件） | PASS | `$env:PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe; cargo test --workspace`；79/79 crate tests 通过，doc-tests 通过 |
| Python Sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；`10 passed` |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，约 `16.6 MB`，exit `0` |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；Vite/Rust/Desktop 成功启动并观察到 `xarchive-desktop.exe`，受控停止后项目进程数为 `0` |
| Tauri installer/package | NOT APPLICABLE | `desktop/src-tauri/tauri.conf.json` 的 `bundle.active=false`，本轮没有启用安装包产物 |
| Desktop Router/Sidecar/Job failure and event persistence | NOT RUN | 现有自动化覆盖 crate/unit tests 和 Desktop 启动，但没有应用级失败状态、事件历史和无 panic 场景；不以启动烟测替代 |
| aria2 核心 artifact/RPC/恢复链路 | NOT APPLICABLE | 本轮未修改既有 aria2 核心逻辑，沿用历史 Windows PASS 证据；不重复执行 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | NOT RUN | 缺少受控 `aria2c.exe`、媒体服务和可重复的过期 URL 场景，且完整真实业务链路仍未实现 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer-use helper 首次调用和重置后重试均因 `helper_unknown_error: setup refresh had errors` 异常退出；未进行 GUI 交互验收 |
| 文件 SQLite 应用级重启、遗留 staging、迁移 | NOT RUN | 本轮仅有库级测试，缺少 Desktop 应用级专项场景 |
| Named Pipe、Registry、externalBin、Tray、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、发布/安装实机、浏览器实机、账号或凭据前置条件 |

#### 错误分析与 Linux 后续事项

1. 默认 Windows 命令环境下的 workspace test `FAIL` 仍是环境前置条件问题：`PYTHON` 未设置，真实 worker 测试无法启动并返回 `NotRunning`；设置项目 `.venv` Python 后同一工作副本的 79/79 workspace tests 通过。本轮不修改业务代码；Linux/CI 后续应明确 Windows Python 解释器前置条件。
2. Release/Debug 构建和启动均通过。MSVC linker stdout 的 `.lib/.exp` 生成信息为非阻塞 warning；主动 Ctrl+C 停止 Debug 时出现 Chromium `Error = 1411` 和 `STATUS_CONTROL_C_EXIT`，但项目进程已清理，未观察到持续运行时故障。
3. 本轮没有稳定的项目代码 `FAIL`。Linux 后续应将路线图和非 Windows 完成清单中仍写作“Windows 严格 clippy 复验待执行”的陈旧状态，与本轮及前两轮实际 PASS 结果对齐；同时补充 Desktop Router/Sidecar/Job 失败状态与事件历史的应用级 Windows 场景，准备受控 aria2/media-server/expired-URL 场景，并在具备原生 GUI、应用级 SQLite、Named Pipe/Registry、安装器、浏览器和账号凭据前置条件后重新执行当前 BLOCKED/NOT RUN 项目。
### Linux reconciliation after Windows validation repeats（2026-09-12）

Linux 重新读取 2026-09-11 13:00、23:36 与 2026-09-12 09:55 三轮 Windows 验证结果。关键事实：

- Windows strict clippy 已针对 HEAD `040b982` 多轮实际执行并通过（`cargo clippy --workspace --all-targets -- -D warnings`，exit 0）；
- 默认 Windows 命令环境下的 `cargo test --workspace` `FAIL` 是 `PYTHON` 前置条件问题，设置项目 `.venv` Python 后 79/79 crate tests 与 doc-tests 均通过，不是稳定项目代码 FAIL；
- Node check/test/build、Python Sidecar 10/10、Tauri Release build 与 Debug startup/cleanup 均 PASS；
- GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度仍 BLOCKED；文件 SQLite 应用级重启、遗留 staging、迁移仍 NOT RUN；Desktop Router/Sidecar/Job 应用级失败状态与事件持久化、真实 aria2 fallback / 403 refresh / transfer lifecycle 仍 NOT RUN。

本轮 Linux 处理：

- 将 `docs/development/roadmap.md`、`docs/development/non-windows-completion.md`、`docs/development/testing-strategy.md` 中仍写作"Windows 严格 clippy 复验待执行 / `WINDOWS_VERIFICATION_PENDING`"的陈旧状态，与本轮及前两轮实际 `WINDOWS_PASS` 结果对齐，明确区分"Windows 已实际执行并通过"与"Linux 侧未安装 clippy 记为 NOT RUN"；
- 未修改任何业务代码。

Linux 端验证（2026-09-12）：`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo test --workspace --no-fail-fast`（79 项 crate 测试及 doc-tests）、`npm test`（Extension 6/6）、`npm run check`、`npm run build`、`git diff --check` 均通过。

| 项目 | 状态 | 说明 |
|---|---|---|
| Windows strict clippy（HEAD `040b982`） | WINDOWS_PASS | 已多轮实际执行并通过；后续新增代码仍需保持严格 clippy 通过 |
| Windows Node/Rust build/test、Python Sidecar、Tauri build/start（HEAD `040b982`） | WINDOWS_PASS | 已实际执行并通过；后续业务代码变更需重新同步并复验 |
| Desktop Router/Sidecar/Job failure and event persistence 应用级 | WINDOWS_VERIFICATION_PENDING | 现有自动化仅覆盖 crate/unit 测试与 Desktop 启动，缺少应用级失败状态、事件历史和无 panic 场景 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | WINDOWS_VERIFICATION_PENDING | 缺少受控 `aria2c.exe`、media server 与可重复的过期 URL 场景，且完整真实链路仍未实现 |
| GUI 真实 WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | Computer-use helper 因 setup refresh 错误无法提供原生 GUI target |
| Installer/signing/updater/externalBin、Tray/Autostart、Credential Manager | NOT RUN / BLOCKED | `bundle.active=false`，发布 artifact、签名、浏览器实机等前置条件未具备 |
| Edge Cookie、真实 X/Telegram 账号链路 | BLOCKED | 缺少账号/Profile/凭据，公开示例失败链路不能替代认证验收 |

本阶段仍无 `WINDOWS_VERIFICATION_BLOCKING`。未实际执行的项目保持 `WINDOWS_VERIFICATION_PENDING`、`BLOCKED` 或 `NOT RUN`，不得提前标记为 `WINDOWS_PASS`。

### 本批次 Windows Validation Queue（M5 Quote/Reply 建模，2026-09-12，working tree 未提交）

本批 Linux 开发在 HEAD `040b982` 之后新增 working tree 改动（未提交）：`xarchive-protocol` 嵌套 `quoted_tweet`/`reply_to` 校验、shared JSON Schema 同步、Extension DOM 嵌套引用提取、`0003_quote_reply_relationships.sql` 迁移与 `tweet_relationships`/`update_tweet_metadata` 关系持久化、Desktop `archive_tweet` 的 `merge_browser_relationships`、Sidecar `models.py` 引用元数据归一化。Linux 验证：`cargo fmt --all -- --check`、`cargo check --workspace --all-targets`（无警告）、`cargo test --workspace` 87 项全通过、`npm test`（Extension 7/7）、`python3 -m compileall sidecar/src` 通过；Linux 无 clippy，保持 NOT RUN。以下为需要 Windows 复验的合并队列（已按场景去重分组）：

| ID | 分组 | 验证项目 | 关联修改 | 状态 | 前置条件 | 精确验证行为 | 预期结果 | 优先级 | 阻塞后续 Linux 开发 |
|---|---|---|---|---|---|---|---|---|---|
| WQ-M5-01 | Build/Toolchain | Windows workspace fmt/check/clippy/test 复验（含 M5 改动） | protocol/core/storage/desktop 新增代码 | WINDOWS_VERIFICATION_PENDING | 同步当前 working tree；进程级 `PYTHON` 指向项目 `.venv\\Scripts\\python.exe` | `cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`（含新增 `persists_reply_and_quote_relationships`、`upgrades_existing_database_with_relationship_columns`） | 全部 exit 0；workspace 总数 ≥ 87 | P1 | 否 |
| WQ-M5-02 | Build/Toolchain | Windows Node check/test/build 复验（含 DOM 引用提取） | `extension/src/content-core.js`、`extension/tests/content.test.js`（新增 1 项测试） | WINDOWS_VERIFICATION_PENDING | 同步 working tree | `npm run check`、`npm run test`（Extension 7/7）、`npm run build` | 全部 exit 0 | P1 | 否 |
| WQ-M5-03 | Build/Toolchain | Python Sidecar 复验（引用元数据归一化） | `sidecar/src/xarchive_downloader/models.py` | WINDOWS_VERIFICATION_PENDING | 项目 `.venv` | `.venv/Scripts/pytest.exe sidecar/tests -q` | 10/10 通过 | P2 | 否 |
| WQ-M5-04 | Build/Toolchain | Tauri Release build / Debug startup 复验 | desktop `archive_tweet` 合并逻辑随二进制进入构建 | WINDOWS_VERIFICATION_PENDING | 同步 working tree | `npm run build:tauri`、`npm run dev:tauri` 启动并受控停止 | Release exe 生成 exit 0；启动/停止无残留进程 | P1 | 否 |
| WQ-M5-05 | Runtime/Integration | 真实浏览器端到端：quote tweet DOM 提取 → Native Messaging → Desktop `archive_tweet` → SQLite 关系列持久化 → `tweet_relationships` 查询 | Extension + protocol + storage + desktop 合并逻辑 | WINDOWS_VERIFICATION_PENDING | Edge/Chrome 实机、Native Host 注册、真实 X 账号或公开 quote tweet | 归档一条真实 quote tweet，检查 `tweets.reply_to_tweet_id`/`quoted_tweet_id` 与 `tweet.json` 中 `quoted_tweet` 嵌套一致；reply tweet 同理 | 关系列与 DOM 证据一致；Sidecar 数据存在时不被 DOM 覆盖 | P1 | 否 |
| WQ-M5-06 | Runtime/Integration | 真实 gallery-dl payload 的 `merge_browser_relationships` 不覆盖语义 | desktop `archive_tweet` | WINDOWS_VERIFICATION_PENDING | 同 WQ-M5-05（需 gallery-dl 实际返回含引用信息的 payload） | 对比 `merged_metadata_json` 与 Sidecar 原始 payload；确认 Sidecar 已有 reply/quote 数据优先于 DOM | 不覆盖 Sidecar 数据；Sidecar 缺失时以 DOM 补齐；`post` 可被 DOM 升级为 reply/quote | P2 | 否 |
| WQ-M5-07 | Filesystem | SQLite 迁移升级 0001→0003（Windows 文件数据库） | `migrations/0003_quote_reply_relationships.sql` + storage 迁移逻辑 | WINDOWS_VERIFICATION_PENDING | 存在仅含 0001/0002 的旧版 Windows 文件数据库副本 | 用旧库启动 Desktop，验证 `ALTER TABLE ADD COLUMN` 幂等应用、`schema_migrations` 到 3、旧 tweet 关系列为 NULL | 迁移成功且无数据丢失；与 Linux `upgrades_existing_database_with_relationship_columns` 行为一致 | P1 | 否 |
| WQ-M5-08 | Filesystem | Unicode/空格路径下的 staging commit 与 `tweet.json` 写入 | storage `ArchiveService`（关系元数据写入路径） | WINDOWS_VERIFICATION_PENDING | Windows 归档根目录含 Unicode/空格 | 在含中文/空格路径归档带 quoted_tweet 的 tweet | `tweet.json` UTF-8 无损；目录 commit 成功 | P2 | 否 |
| WQ-M5-09 | Packaging | Tauri installer/package（externalBin/manifest 随包） | desktop 构建配置 | WINDOWS_VERIFICATION_PENDING | 临时启用 `bundle.active=true` 或按 M7 专项 | 打包并安装后归档 quote tweet | 安装包内 Sidecar/Native Host 可用，归档链路完整 | P3 | 否 |

不重复入队（沿用既有队列状态）：GUI WebView2/DPI/辅助技术验收（BLOCKED）、Desktop Router/Sidecar/Job 应用级失败状态与事件历史（PENDING）、真实 aria2 fallback/403 refresh/transfer lifecycle（PENDING）、Tray/Autostart/Credential Manager/Named Pipe/Registry（NOT RUN/BLOCKED）、真实 Telegram 账号链路（BLOCKED）。

本批次仍无 `WINDOWS_VERIFICATION_BLOCKING`：所有 Windows 复验均不阻塞后续 Linux 开发，可在本批次提交后集中执行。优先顺序：WQ-M5-01/02/04（可全自动）→ WQ-M5-07（需旧库样本）→ WQ-M5-03 → WQ-M5-05/06（需账号环境）→ WQ-M5-08 → WQ-M5-09。

本轮未将默认 Python 环境失败误记为业务代码失败，也未将 Linux 单元测试、Tauri 启动或静态 GUI 结构检查提升为真实 Windows GUI/端到端验收；验证期间没有向 Linux 反向同步代码或构建产物。
### Windows validation of Linux working tree（M5 Quote/Reply，2026-09-12 11:40–11:48 +08:00）

本轮针对 Linux 当前 working tree 执行 Windows 验证。source branch 为 `main`，HEAD 为 `040b98232f8475c9c4f19b98679e808aed800ab5`；验证开始时已有未提交的 M5 业务改动（protocol/core/storage/desktop、Extension、Sidecar 及 shared schema）、文档改动，以及未跟踪的 `desktop/src-tauri/migrations/0003_quote_reply_relationships.sql` 和 `OPENAI_CODEX_WRITING_RULES.md`。本轮未修改或修复这些业务改动；验证覆盖的是同步后的 working tree，而不是纯 Git commit。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`。

Validation environment：Windows 11 专业工作站版 Insider Preview `10.0.29661`，AMD64；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、系统 Python `3.14.7`、项目 pytest `9.1.1`。项目 Python 为 `E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux source → Windows E: 同步 | PASS | 先 dry-run，再使用 Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；实际 exit `1`（有文件复制，非失败），无 failed/mismatch；关键源码、M5 迁移、Schema 和验证文档 SHA-256 均匹配；保留 E 盘 `.venv`、`node_modules`、`target`、`validation-artifacts`、`X-Archive`、`desktop/src-tauri/gen` 及本地文件，排除 `.git`、依赖、缓存、数据库和 secrets |
| Node check | PASS | `npm run check`；Vite 转换 `35 modules`，exit `0` |
| Node tests | PASS | `npm run test`；Extension `7/7`（含 nested quoted tweet），Desktop Node 测试 `0` 项且无失败 |
| Node production build | PASS | `npm run build`；Vite `35 modules`，exit `0` |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit `0` |
| Rust check | PASS | `cargo check --workspace --all-targets`；exit `0` |
| Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；exit `0` |
| Rust workspace tests（默认 Windows 命令环境） | FAIL | 清空 `PYTHON` 后执行 `cargo test --workspace`；`xarchive-sidecar-supervisor` 的 `communicates_with_a_real_python_worker_when_available`、`spawn_ready_completes_the_hello_handshake` 均为 `NotRunning`，exit `1`；其余已执行 crate 测试通过 |
| Rust workspace tests（项目 Python 前置条件） | PASS | `$env:PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe; cargo test --workspace`；crate tests `87/87` 通过，所有 doc-tests 通过 |
| Python Sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；`10 passed` |
| Python compile check | PASS | `.venv/Scripts/python.exe -m compileall -q sidecar/src sidecar/tests`；exit `0` |
| M5 relation persistence targeted test | PASS | `cargo test -p xarchive-storage persists_reply_and_quote_relationships`；`1 passed` |
| M5 migration targeted test | PASS | `cargo test -p xarchive-storage upgrades_existing_database_with_relationship_columns`；`1 passed` |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit `0` |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；观察到 `xarchive-desktop.exe`，窗口标题 `XArchive`，进程 `Responding=True`；受控停止后项目进程为 `0` |
| Tauri installer/package（WQ-M5-09） | NOT APPLICABLE | `desktop/src-tauri/tauri.conf.json` 中 `bundle.active=false`，当前轮次没有安装包目标 |
| WQ-M5-05 真实浏览器 → Native Messaging → Desktop → SQLite 端到端 | BLOCKED | Computer Use helper 初始化和重置后重试均因 `helper_unknown_error: setup refresh had errors` 异常退出；同时缺少可用浏览器实机、Native Host 注册及真实账号/公开受控样本 |
| WQ-M5-06 真实 gallery-dl payload merge | NOT RUN | 没有受控 gallery-dl 实际返回 payload 和可重复的真实归档链路；库级 `merge_browser_relationships` 测试已包含在 Rust 87/87 中 |
| WQ-M5-07 Desktop 应用级旧文件数据库 0001→0003 升级 | NOT RUN | `upgrades_existing_database_with_relationship_columns` 的 Windows 库级测试通过，但没有准备仅含 0001/0002 的 Desktop 应用数据库副本，未将库级证据提升为应用级验收 |
| WQ-M5-08 Unicode/空格路径下带引用 tweet 的 staging commit | NOT RUN | 缺少 Desktop 应用级归档驱动和可核验的 Windows 用户数据场景；未以静态路径检查代替真实写入验收 |
| WQ-M5-10 profile 文件 Windows 写入与 SQLite 迁移升级 | WINDOWS_VERIFICATION_PENDING | profile 文件实现（`UserProfileSnapshot`/`UserProfileFile`、`Database::user_profile()`、`FileStore::write_user_profile()`、`ArchiveService::refresh_author_profile()`、`upsert_user` 名称去重） | 在 Windows 归档根目录含 Unicode/空格路径环境下，归档一条含 `user_id` 的 tweet，验证 `Users/<stable>/profile.json` 写入成功且内容（schema_version/user_id/stable_directory_name/username/display_name/updated_at/names）正确；旧版 Windows 文件数据库（仅含 0001/0002）启动后 `schema_migrations` 到 3 且 `profile.json` 可正常读写 | profile.json UTF-8 无损；迁移成功无数据丢失 | P2 | 否 |
| WQ-M5-11 profile 文件名称去重与 tweet-user 关联 | WINDOWS_VERIFICATION_PENDING | `upsert_user` 名称去重逻辑 + `set_tweet_user` 关联 | 同一用户多次归档时 `user_names` 表不重复记录（username 与 display_name 均未变化时）；`tweets.user_id` 正确关联到 `users` 表；`user_profile()` 返回最新名称 + 完整历史 | 名称去重；tweet-user 关联正确；profile 快照与数据库一致 | P2 | 否 |
| Desktop Router/Sidecar/Job 应用级失败状态与事件持久化 | NOT RUN | 本轮只执行 crate/unit tests 与启动检查，未覆盖应用级失败、事件历史和无 panic 场景 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | NOT RUN | 缺少受控 `aria2c.exe`、media server 和可重复过期 URL 场景；本轮未扩大为开发任务 |
| GUI WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | 原生交互 helper 不可用，未进行真实窗口渲染与交互验收 |
| Named Pipe、Registry、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、安装/发布实机、浏览器 Profile、账号或凭据前置条件 |

#### 错误分析与 Linux 后续事项

1. 默认 Windows 命令环境下 `PYTHON` 为空导致两项 Sidecar supervisor 真实 worker 测试返回 `NotRunning`，属于环境前置条件 FAIL；同一工作副本设置项目 `.venv` 后完整 `87/87` 通过，当前没有稳定可归因于 M5 业务代码的 Windows 自动化 FAIL。Linux/CI 后续应明确 Windows Python 解释器前置条件，并保留默认环境 FAIL 与项目环境 PASS 的证据边界。
2. Rust/Tauri 输出的 MSVC linker `.lib/.exp` 信息是非阻塞 warning。Debug 受控 Ctrl+C 停止时出现 `STATUS_CONTROL_C_EXIT`，但 `xarchive-desktop` 已退出且没有残留项目进程，不构成运行失败。
3. 初次使用 `-- --exact` 的两个针对性过滤命令因未包含完整测试路径而选择了 `0` 项；该结果未作为证据，随后用普通过滤器重新执行并分别得到 `1/1` PASS。没有因该命令调整业务代码或测试配置。
4. M5 的协议嵌套校验、DOM 提取、关系持久化、迁移库级行为、Sidecar 归一化和 profile 文件（`UserProfileSnapshot`/`UserProfileFile`、`Database::user_profile()`、`FileStore::write_user_profile()`、`ArchiveService::refresh_author_profile()`、`upsert_user` 名称去重）已通过 Windows 自动化/针对性检查；仍未完成真实浏览器、Native Messaging、Desktop 应用级 SQLite、Unicode 用户路径、真实 gallery-dl payload 和安装包链路。GUI helper 失败是本轮原生交互验证的外部阻塞。
5. Linux 后续处理：在合并当前 working tree 前保留本轮结果并决定 M5 提交边界；补齐 WQ-M5-05/06/07/08/09 及 Desktop Router/Job、真实 aria2、GUI、Native Host/Registry/Tray、Edge/X/Telegram 的前置条件后再执行。以上项目均不阻塞后续 Linux 开发，本轮不修改业务代码。

本轮最终没有向 Linux 反向同步代码、依赖或构建产物；仅计划将本节验证记录写回 Linux 的 `docs/development/windows-validation.md`。
### Windows validation of Linux working tree（M5 User Profile，2026-09-12 12:45–12:54 +08:00）

本轮针对 Linux 当前 working tree 执行 Windows 验证。source branch 为 `main`，HEAD 为 `040b98232f8475c9c4f19b98679e808aed800ab5`；验证开始时已有未提交的 M5 Quote/Reply 及 User Profile 业务改动（protocol/core/storage/desktop、Extension、Sidecar、shared schema）、文档改动，以及未跟踪的 `desktop/src-tauri/migrations/0003_quote_reply_relationships.sql` 和 `OPENAI_CODEX_WRITING_RULES.md`。本轮未修改或修复这些业务改动；验证覆盖同步后的 working tree，而不是纯 Git commit。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`。

Validation environment：Windows 11 专业工作站版 Insider Preview `10.0.29661`，AMD64；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、系统 Python `3.14.7`、项目 pytest `9.1.1`。项目 Python 为 `E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux source → Windows E: 同步 | PASS | 先 dry-run，再使用 Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；实际 exit `3`，failed/mismatch 为 `0`；16 个关键源码、迁移、Schema 和文档文件 SHA-256 全部匹配；保留 E 盘 `.venv`、`node_modules`、`target`、`validation-artifacts`、`X-Archive`、`desktop/src-tauri/gen` 及本地文件，排除 `.git`、依赖、缓存、数据库和 secrets |
| Node check | PASS | `npm run check`；Vite 转换 `35 modules`，exit `0` |
| Node tests | PASS | `npm run test`；Extension `7/7`（含 nested quoted tweet），Desktop Node 测试 `0` 项且无失败 |
| Node production build | PASS | `npm run build`；Vite `35 modules`，exit `0` |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit `0` |
| Rust check | PASS | `cargo check --workspace --all-targets`；exit `0` |
| Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；exit `0` |
| Rust workspace tests（默认 Windows 命令环境） | FAIL | 清空 `PYTHON` 后执行 `cargo test --workspace`；`xarchive-sidecar-supervisor` 的 `communicates_with_a_real_python_worker_when_available`、`spawn_ready_completes_the_hello_handshake` 返回 `NotRunning`，exit `1`；其余已执行 crate 测试通过 |
| Rust workspace tests（项目 Python 前置条件） | PASS | `$env:PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe; cargo test --workspace`；crate tests `88/88` 通过，所有 doc-tests 通过；storage `19/19` |
| Python Sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；`10 passed` |
| Python compile check | PASS | `.venv/Scripts/python.exe -m compileall -q sidecar/src sidecar/tests`；exit `0` |
| M5 profile history targeted test | PASS | `cargo test -p xarchive-storage persists_user_name_history_and_stable_directory_name`；`1 passed` |
| M5 profile name dedup targeted test | PASS | `cargo test -p xarchive-storage does_not_duplicate_name_history_when_names_are_unchanged`；`1 passed` |
| M5 profile JSON/archive targeted test | PASS | `cargo test -p xarchive-storage completes_local_archive_and_writes_portable_metadata`；`1 passed` |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit `0` |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；观察到 `xarchive-desktop.exe`，窗口标题 `XArchive`，进程 `Responding=True`；受控停止后项目进程为 `0` |
| Tauri installer/package（WQ-M5-09） | NOT APPLICABLE | `desktop/src-tauri/tauri.conf.json` 中 `bundle.active=false`，当前轮次没有安装包目标 |
| WQ-M5-05 真实浏览器 → Native Messaging → Desktop → SQLite 端到端 | BLOCKED | Computer Use helper 初始化和重置后重试均因 `helper_unknown_error: setup refresh had errors` 异常退出；同时缺少可用浏览器实机、Native Host 注册及真实账号/公开受控样本 |
| WQ-M5-06 真实 gallery-dl payload merge | NOT RUN | 没有受控 gallery-dl 实际返回 payload 和可重复的真实归档链路；库级合并测试已包含在 Rust 88/88 中 |
| WQ-M5-07 Desktop 应用级旧文件数据库 0001→0003 升级 | NOT RUN | storage 库级迁移测试随 Rust 88/88 通过，但没有仅含 0001/0002 的 Desktop 应用数据库副本，未将库级证据提升为应用级验收 |
| WQ-M5-08 Unicode/空格路径下带引用 tweet 的 staging commit | NOT RUN | profile/归档单元测试通过，但缺少 Desktop 应用级归档驱动和可核验的 Windows 用户数据场景 |
| WQ-M5-10 Desktop 应用级 profile 文件写入与迁移升级 | NOT RUN | storage 层 profile JSON 与迁移相关测试通过，但未执行 Windows Desktop 实际归档、Unicode/空格路径和旧库启动场景 |
| WQ-M5-11 Desktop 应用级名称去重与 tweet-user 关联 | NOT RUN | storage 层名称历史、去重及归档关联测试通过，但未执行 Desktop 应用级多次归档和 profile/SQLite 联动验收 |
| Desktop Router/Sidecar/Job 应用级失败状态与事件持久化 | NOT RUN | 本轮只执行 crate/unit tests 与启动检查，未覆盖应用级失败、事件历史和无 panic 场景 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | NOT RUN | 缺少受控 `aria2c.exe`、media server 和可重复过期 URL 场景；本轮未扩大为开发任务 |
| GUI WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | 原生交互 helper 不可用，未进行真实窗口渲染与交互验收 |
| Named Pipe、Registry、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、安装/发布实机、浏览器 Profile、账号或凭据前置条件 |

#### 错误分析与 Linux 后续事项

1. 默认 Windows 命令环境下 `PYTHON` 为空导致两项 Sidecar supervisor 真实 worker 测试返回 `NotRunning`，属于环境前置条件 FAIL；同一工作副本设置项目 `.venv` 后完整 `88/88` 通过，当前没有稳定可归因于 User Profile 或 Quote/Reply 业务代码的 Windows 自动化 FAIL。Linux/CI 后续应明确 Windows Python 解释器前置条件，并保留默认环境 FAIL 与项目环境 PASS 的证据边界。
2. Rust/Tauri 输出的 MSVC linker `.lib/.exp` 信息是非阻塞 warning。Debug 受控 Ctrl+C 停止时出现 `STATUS_CONTROL_C_EXIT` 和 Chromium 注销警告，但 `xarchive-desktop` 已退出且没有残留项目进程，不构成运行失败。
3. M5 User Profile 的 `UserProfileSnapshot`/`UserProfileFile`、profile JSON 写入、名称历史去重和 tweet-user 关联已通过 Windows Rust storage 单元/针对性测试；这些结果不等同于 Windows Desktop 应用级 Unicode/空格路径、旧数据库迁移和多次归档验收。
4. 原生交互 helper 两次初始化均因 `helper_unknown_error: setup refresh had errors` 失败，导致真实 GUI 与浏览器端到端项目 BLOCKED；未通过静态结构、启动进程或单元测试替代交互验收。
5. Linux 后续处理：在合并当前 working tree 前保留本轮结果并决定 M5 User Profile 提交边界；补齐 WQ-M5-05/06/07/08/09/10/11 及 Desktop Router/Job、真实 aria2、GUI、Native Host/Registry/Tray、Edge/X/Telegram 的前置条件后再执行。以上项目均不阻塞后续 Linux 开发，本轮不修改业务代码。

本轮最终没有向 Linux 反向同步代码、依赖或构建产物；仅计划将本节验证记录写回 Linux 的 `docs/development/windows-validation.md`。
### Windows validation repeat of current Linux working tree（M5 User Profile，2026-09-12 13:05–13:12 +08:00）

本轮按用户要求重新针对 Linux 当前 working tree 执行 Windows 验证。source branch 为 `main`，HEAD 为 `040b98232f8475c9c4f19b98679e808aed800ab5`；验证开始时 working tree 仍包含未提交的 M5 Quote/Reply 与 User Profile 业务改动、相关文档改动，以及未跟踪的 `desktop/src-tauri/migrations/0003_quote_reply_relationships.sql` 和 `OPENAI_CODEX_WRITING_RULES.md`。Plan 已记录上一轮 88/88 自动化验证，但本轮仍重新执行；本轮没有修改业务代码，验证对象是同步后的 working tree 而不是纯 Git commit。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`。

Validation environment：Windows 11 专业工作站版 Insider Preview `10.0.29661`，AMD64；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、系统 Python `3.14.7`、项目 pytest `9.1.1`。项目 Python 为 `E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux source → Windows E: 同步 | PASS | 先 dry-run，再使用 Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；实际 exit `3`，failed/mismatch 为 `0`；16 个关键源码、迁移、Schema 和文档文件 SHA-256 全部匹配；保留 E 盘 `.venv`、`node_modules`、`target`、`validation-artifacts`、`X-Archive` 及本地文件，排除 `.git`、依赖、缓存、数据库和 secrets |
| Node check | PASS | `npm run check`；Vite 转换 `35 modules`，exit `0` |
| Node tests | PASS | `npm run test`；Extension `7/7`，Desktop Node 测试 `0` 项且无失败 |
| Node production build | PASS | `npm run build`；Vite `35 modules`，exit `0` |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit `0` |
| Rust check | PASS | `cargo check --workspace --all-targets`；exit `0` |
| Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；exit `0` |
| Rust workspace tests（默认 Windows 命令环境） | FAIL | 清空 `PYTHON` 后执行 `cargo test --workspace`；`xarchive-sidecar-supervisor` 的 `communicates_with_a_real_python_worker_when_available`、`spawn_ready_completes_the_hello_handshake` 返回 `NotRunning`，exit `1`；其余已执行 crate 测试通过 |
| Rust workspace tests（项目 Python 前置条件） | PASS | `$env:PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe; cargo test --workspace`；crate tests `88/88` 通过，所有 doc-tests 通过；storage `19/19` |
| Python Sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；`10 passed` |
| Python compile check | PASS | `.venv/Scripts/python.exe -m compileall -q sidecar/src sidecar/tests`；exit `0` |
| M5 profile history/name dedup/archive tests | PASS | `cargo test -p xarchive-storage persists_user_name_history_and_stable_directory_name`、`does_not_duplicate_name_history_when_names_are_unchanged`、`completes_local_archive_and_writes_portable_metadata`；各 `1 passed` |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit `0` |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；观察到 `xarchive-desktop.exe`，窗口标题 `XArchive`，进程 `Responding=True`；受控停止后项目进程为 `0` |
| Tauri installer/package（WQ-M5-09） | NOT APPLICABLE | `desktop/src-tauri/tauri.conf.json` 中 `bundle.active=false`，当前轮次没有安装包目标 |
| WQ-M5-05 真实浏览器 → Native Messaging → Desktop → SQLite 端到端 | BLOCKED | Computer Use helper 首次调用和重置后重试均因 `helper_unknown_error: setup refresh had errors` 异常退出；同时缺少可用浏览器实机、Native Host 注册及真实账号/公开受控样本 |
| WQ-M5-06 真实 gallery-dl payload merge | NOT RUN | 没有受控 gallery-dl 实际返回 payload 和可重复的真实归档链路；库级合并测试已包含在 Rust 88/88 中 |
| WQ-M5-07 Desktop 应用级旧文件数据库 0001→0003 升级 | NOT RUN | storage 库级迁移测试随 Rust 88/88 通过，但没有仅含 0001/0002 的 Desktop 应用数据库副本 |
| WQ-M5-08 Unicode/空格路径下带引用 tweet 的 staging commit | NOT RUN | profile/归档单元测试通过，但缺少 Desktop 应用级归档驱动和可核验的 Windows 用户数据场景 |
| WQ-M5-10 Desktop 应用级 profile 文件写入与迁移升级 | NOT RUN | storage 层 profile JSON 与迁移相关测试通过，但未执行 Windows Desktop 实际归档、Unicode/空格路径和旧库启动场景 |
| WQ-M5-11 Desktop 应用级名称去重与 tweet-user 关联 | NOT RUN | storage 层名称历史、去重及归档关联测试通过，但未执行 Desktop 应用级多次归档和 profile/SQLite 联动验收 |
| Desktop Router/Sidecar/Job 应用级失败状态与事件持久化 | NOT RUN | 本轮只执行 crate/unit tests 与启动检查，未覆盖应用级失败、事件历史和无 panic 场景 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | NOT RUN | 缺少受控 `aria2c.exe`、media server 和可重复过期 URL 场景；本轮未扩大为开发任务 |
| GUI WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | 原生交互 helper 不可用，未进行真实窗口渲染与交互验收 |
| Named Pipe、Registry、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、安装/发布实机、浏览器 Profile、账号或凭据前置条件 |

#### 错误分析与 Linux 后续事项

1. 默认 Windows 命令环境下 `PYTHON` 为空导致两项 Sidecar supervisor 真实 worker 测试返回 `NotRunning`，属于环境前置条件 FAIL；同一工作副本设置项目 `.venv` 后完整 `88/88` 通过，当前没有稳定可归因于 M5 业务代码的 Windows 自动化 FAIL。Linux/CI 后续应明确 Windows Python 解释器前置条件，并保留默认环境 FAIL 与项目环境 PASS 的证据边界。
2. Rust/Tauri 输出的 MSVC linker `.lib/.exp` 信息是非阻塞 warning。Debug 受控 Ctrl+C 停止时出现 `STATUS_CONTROL_C_EXIT` 和 Chromium 注销警告，但 `xarchive-desktop` 已退出且没有残留项目进程，不构成运行失败。
3. M5 User Profile 的 storage 层测试和 Rust 88/88 已在 Windows 通过；这只证明跨平台库级行为，不能替代 WQ-M5-10/11 所要求的 Desktop 应用级 profile、Unicode/空格路径和旧数据库启动验收。
4. 原生交互 helper 两次初始化均因 `helper_unknown_error: setup refresh had errors` 失败，真实 GUI 与浏览器端到端继续 BLOCKED；没有将静态结构、进程启动或单元测试提升为 GUI/端到端 PASS。
5. Linux 后续处理：本轮没有新增业务代码问题；保留并决定当前 M5 User Profile working tree 的提交边界，补齐 WQ-M5-05/06/07/08/09/10/11 及 Desktop Router/Job、真实 aria2、GUI、Native Host/Registry/Tray、Edge/X/Telegram 的前置条件后再执行。以上项目均不阻塞后续 Linux 开发，本轮不修改业务代码。

本轮最终没有向 Linux 反向同步代码、依赖或构建产物；仅计划将本节验证记录写回 Linux 的 `docs/development/windows-validation.md`。
### Windows validation of current Linux working tree（M5 Quote/Reply + User Profile，2026-09-12 13:34–13:42 +08:00）

本轮再次以 Linux 当前 working tree 为唯一源执行 Windows 平台验证。source branch 为 `main`，HEAD 为 `040b98232f8475c9c4f19b98679e808aed800ab5`；验证开始时 working tree 为 dirty，包含 14 个已跟踪文件的 M5 Quote/Reply、User Profile 及相关文档修改，以及未跟踪的 `desktop/src-tauri/migrations/0003_quote_reply_relationships.sql` 和 `OPENAI_CODEX_WRITING_RULES.md`。本轮未修改这些业务文件，验证对象为同步后的 working tree 而不是纯 Git commit。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`。

Validation environment：Windows 11 专业工作站版 Insider Preview `10.0.29661`，x64；WebView2 Runtime `152.0.4191.66`；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、系统 Python `3.14.7`、项目 pytest `9.1.1`。项目 Python 为 `E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux source → Windows E: 受控同步 | PASS | 先 dry-run，再使用 Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT /R:1 /W:1`；实际 exit `3`，failed/mismatch 均为 `0`；15 个关键源码、迁移、Schema 和文档文件 SHA-256 全部匹配；保留 E: `.venv`、`node_modules`、`target`、`validation-artifacts`、`X-Archive`、`desktop/src-tauri/gen` 及本地文件，排除 `.git`、依赖、缓存、数据库、日志、PDF、secrets 和 build/dist |
| Node check | PASS | `npm run check`；Desktop Vite 转换 35 modules，Extension `node --check` 全部通过 |
| Node tests | PASS | `npm run test`；Extension `7/7`，Desktop Node 测试 `0` 项且无失败 |
| Node production build | PASS | `npm run build`；Vite 转换 35 modules，exit `0` |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit `0` |
| Rust check | PASS | `cargo check --workspace --all-targets`；exit `0` |
| Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；exit `0` |
| Rust workspace tests（默认 Windows 命令环境） | FAIL | 清空 `PYTHON` 后执行 `cargo test --workspace`；`xarchive-sidecar-supervisor` 的 `spawn_ready_completes_the_hello_handshake` 与 `communicates_with_a_real_python_worker_when_available` 返回 `NotRunning`，exit `1` |
| Rust workspace tests（项目 Python 前置条件） | PASS | `$env:PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe; cargo test --workspace`；crate tests `88/88` 通过，所有 doc-tests 通过 |
| Python Sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；`10 passed` |
| Python compile check | PASS | `.venv/Scripts/python.exe -m compileall -q sidecar/src sidecar/tests`；exit `0` |
| M5 Quote/Reply 协议/存储定向回归 | PASS | `validates_browser_archive_request_with_quoted_tweet`、`round_trips_quoted_tweet_without_losing_nested_data`、`persists_reply_and_quote_relationships` 各 `1 passed` |
| M5 User Profile 定向回归 | PASS | `persists_user_name_history_and_stable_directory_name`、`does_not_duplicate_name_history_when_names_are_unchanged`、`completes_local_archive_and_writes_portable_metadata` 各 `1 passed` |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit `0` |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；`xarchive-desktop.exe` 为 `Responding=True`，窗口标题 `XArchive`；受控停止后项目 Tauri 进程为 `0` |
| Tauri installer/package（WQ-M5-09） | NOT APPLICABLE | `desktop/src-tauri/tauri.conf.json` 的 `bundle.active=false`，当前没有安装包目标 |
| WQ-M5-05 真实浏览器 → Native Messaging → Desktop → SQLite 端到端 | BLOCKED | 原生交互 helper 首次初始化及重置后重试均异常退出（`helper_unknown_error: setup refresh had errors`）；同时缺少可用浏览器实机、Native Host 注册、真实账号和公开受控样本 |
| WQ-M5-06 真实 gallery-dl payload merge | NOT RUN | 缺少受控 gallery-dl 实际 payload 和可重复的真实归档链路；库级合并测试已包含在 `88/88` 中 |
| WQ-M5-07 Desktop 应用级旧文件数据库 `0001 → 0003` 升级 | NOT RUN | storage 库级迁移测试通过，但没有仅含 `0001/0002` 的 Desktop 应用数据库副本 |
| WQ-M5-08 Unicode/空格路径下带引用 tweet 的 staging commit | NOT RUN | Quote/Reply、profile 和归档库级测试通过，但缺少 Desktop 应用级归档驱动和可核验的 Windows 用户数据场景 |
| WQ-M5-10 Desktop 应用级 profile 文件写入与迁移升级 | NOT RUN | storage 层 profile JSON 与迁移相关测试通过，但未执行 Desktop 实际归档、Unicode/空格路径和旧库启动场景 |
| WQ-M5-11 Desktop 应用级名称去重与 tweet-user 关联 | NOT RUN | storage 层名称历史、去重及归档关联测试通过，但未执行 Desktop 多次归档和 profile/SQLite 联动验收 |
| Desktop Router/Sidecar/Job 应用级失败状态与事件持久化 | NOT RUN | 本轮只执行 crate/unit tests 与启动检查，未覆盖应用级失败、事件历史和无 panic 场景 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | NOT RUN | 缺少受控 `aria2c.exe`、media server 和可重复过期 URL 场景；本轮未扩大为开发任务 |
| GUI WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | 原生交互 helper 不可用，未进行真实窗口渲染与交互验收；未用静态结构、进程启动或单元测试替代人工验收 |
| Named Pipe、Registry、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、Host 注册/安装实机、浏览器 Profile、账号或凭据前置条件 |

#### 错误分析与 Linux 后续事项

1. 默认 Windows 命令环境未设置 `PYTHON` 时，两个真实 Python worker handshake 测试返回 `NotRunning`；同一副本显式设置项目 `.venv` 后完整 `88/88` 通过。因此这是 Windows 环境前置条件 FAIL，不是当前 M5 业务代码 FAIL。Linux/CI 后续应明确 Windows Python 解释器前置条件，并保留默认环境 FAIL 与项目环境 PASS 的证据边界。
2. Rust/Tauri 输出的 MSVC linker `.lib/.exp` 为非阻塞 warning。Debug 受控停止时出现 `STATUS_CONTROL_C_EXIT` 和 Chromium `Error = 1411` 注销警告，但 `xarchive-desktop.exe` 已退出且无残留项目进程，不构成应用启动失败。
3. 原生交互 helper 两次初始化均失败，故真实 GUI、WebView2/DPI、键盘/辅助技术和浏览器 Native Messaging 端到端仍为 BLOCKED；没有把启动、静态检查或库级测试提升为 GUI/端到端 PASS。
4. Linux 后续处理：补充并固化 Windows `PYTHON` 前置条件；决定当前 M5 working tree（含未跟踪 migration）的提交边界；准备受控 gallery-dl、旧版本 SQLite、Desktop 应用级归档/重启、aria2 fallback、Native Host/Named Pipe/Registry、GUI 自动化、Edge/X/Telegram 的前置条件后再执行对应队列项。本轮没有发现需要立即修复的 Windows 业务代码问题，以上未执行项目均不阻塞后续 Linux 开发。

本轮 Windows 仅产生验证副本中的依赖、缓存、构建产物和临时日志；没有向 Linux 反向同步代码、依赖或构建产物，仅追加本验证记录。

### Windows validation of current Linux working tree（M5 Quote/Reply + User Profile，2026-09-12 14:14–14:22 +08:00）

本轮以 Linux 当前 working tree 为唯一源重新执行 Windows 验证。source branch 为 `main`，HEAD 为 `040b98232f8475c9c4f19b98679e808aed800ab5`；开始时 working tree 为 dirty，包含 14 个已跟踪修改和 2 个未跟踪文件（包括 `desktop/src-tauri/migrations/0003_quote_reply_relationships.sql`）。本轮没有修改业务代码，验证对象是同步后的 working tree，不是纯 Git commit。Windows 工作副本为 `E:/Shiraishi/VSCode Workspace/Tw2Tg`。

Validation environment：Windows 11 专业工作站版 Insider Preview `10.0.29661`，x64；WebView2 `152.0.4191.66`；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、系统 Python `3.14.7`、项目 pytest `9.1.1`。项目 Python 为 `E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe`。

| 验证项目 | 状态 | 实际命令/关键结果 |
|---|---|---|
| Linux source → Windows E: 受控同步 | PASS | 先 dry-run，再使用 Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT /R:1 /W:1`；实际 exit `3`，failed/mismatch 均为 `0`；15 个关键源码、迁移、Schema 和文档文件 SHA-256 全部匹配；保留 E: `.venv`、`node_modules`、`target`、`validation-artifacts`、`X-Archive`、`desktop/src-tauri/gen` 及本地文件，排除 `.git`、依赖、缓存、数据库、日志、PDF、secrets 和 build/dist |
| Node check | PASS | `npm run check`；Vite 转换 `35 modules`，Extension 检查通过，exit `0` |
| Node tests | PASS | `npm run test`；Extension `7/7`，Desktop Node 测试 `0` 项且无失败 |
| Node production build | PASS | `npm run build`；Vite `35 modules`，exit `0` |
| Rust formatter | PASS | `cargo fmt --all -- --check`；exit `0` |
| Rust check | PASS | `cargo check --workspace --all-targets`；exit `0` |
| Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings`；exit `0` |
| Rust workspace tests（默认 Windows 命令环境） | FAIL | 清空 `PYTHON` 后执行 `cargo test --workspace`；`xarchive-sidecar-supervisor` 的 `communicates_with_a_real_python_worker_when_available`、`spawn_ready_completes_the_hello_handshake` 返回 `NotRunning`，exit `1` |
| Rust workspace tests（项目 Python 前置条件） | PASS | 设置 `$env:PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe` 后执行 `cargo test --workspace`；crate tests `88/88` 通过，所有 doc-tests 通过 |
| Python Sidecar tests | PASS | `.venv/Scripts/pytest.exe sidecar/tests -q`；`10 passed` |
| Python compile check | PASS | `.venv/Scripts/python.exe -m compileall -q sidecar/src sidecar/tests`；exit `0` |
| M5 Quote/Reply 与 User Profile 定向回归 | PASS | 六项定向 `cargo test`（协议 quoted tweet 两项；storage 关系、名称历史、名称去重、portable metadata 各一项）均为 `1 passed` |
| Tauri Release build | PASS | `npm run build:tauri`；生成 `target/release/xarchive-desktop.exe`，exit `0` |
| Tauri Debug startup/cleanup | PASS | `npm run dev:tauri`；观察到 `xarchive-desktop.exe` 为 `Responding=True`；受控停止后项目进程数为 `0` |
| Tauri installer/package（WQ-M5-09） | NOT APPLICABLE | `desktop/src-tauri/tauri.conf.json` 中 `bundle.active=false`，当前没有安装包目标 |
| Computer Use Edge 浏览器只读 AX 探测 | PASS | `cua.getState()` / 选定 Edge tab 后 `getAXState({emit:false})` 成功；本轮未点击、输入或改变浏览器状态；该结果不代表原生桌面 GUI 验收 |
| Computer Use 原生 Windows 桌面枚举/交互 | BLOCKED | `@oai/sky` 初始化后 `sky.list_apps()` 返回 `Trusted RPC service is not configured: sky`；当前 CUA 状态 `apps: []`，无法安全进行 Tauri 原生窗口的真实交互验收 |
| WQ-M5-05 真实浏览器 → Native Messaging → Desktop → SQLite 端到端 | BLOCKED | 原生桌面 Computer Use 服务不可用；同时缺少 Native Host 注册、真实账号和公开受控样本 |
| WQ-M5-06 真实 gallery-dl payload merge | NOT RUN | 缺少受控 gallery-dl 实际 payload 和可重复的真实归档链路；库级合并测试已包含在 `88/88` 中 |
| WQ-M5-07 Desktop 应用级旧文件数据库 `0001 → 0003` 升级 | NOT RUN | storage 库级迁移测试随 `88/88` 通过，但没有仅含 `0001/0002` 的 Desktop 应用数据库副本 |
| WQ-M5-08 Unicode/空格路径下带引用 tweet 的 staging commit | NOT RUN | 缺少 Desktop 应用级归档驱动和可核验的 Windows 用户数据场景 |
| WQ-M5-10 Desktop 应用级 profile 文件写入与迁移升级 | NOT RUN | storage 层 profile JSON/迁移测试通过，但未执行 Desktop 实际归档、Unicode/空格路径和旧库启动场景 |
| WQ-M5-11 Desktop 应用级名称去重与 tweet-user 关联 | NOT RUN | storage 层测试通过，但未执行 Desktop 多次归档和 profile/SQLite 联动验收 |
| Desktop Router/Sidecar/Job 应用级失败状态与事件持久化 | NOT RUN | 本轮仅执行 crate/unit tests 与启动检查，未覆盖应用级失败、事件历史和无 panic 场景 |
| Real aria2 fallback / 403 refresh / transfer lifecycle | NOT RUN | 缺少受控 `aria2c.exe`、media server 和可重复过期 URL 场景；本轮未扩大为开发任务 |
| GUI WebView2/DPI/键盘/焦点/辅助技术/对比度 | BLOCKED | 原生 Windows Computer Use 服务不可用，未进行真实窗口渲染与交互验收 |
| Named Pipe、Registry、Tray/Autostart、Credential Manager、Edge Cookie、真实 X/Telegram | BLOCKED / NOT RUN | 缺少对应 backend、Host 注册/安装实机、浏览器 Profile、账号或凭据前置条件 |

#### 错误分析与 Linux 后续事项

1. 默认 Windows 命令环境未设置 `PYTHON` 时，两个 Sidecar supervisor 真实 worker handshake 测试返回 `NotRunning`；同一副本显式设置项目 `.venv` 后通过。该项是环境前置条件 FAIL，不是当前 M5 业务代码 FAIL。Linux/CI 后续应明确 Windows Python 解释器前置条件，并保留默认环境 FAIL 与项目环境 PASS 的证据边界。
2. Rust/Tauri 输出的 MSVC linker `.lib/.exp` 是非阻塞 warning。Debug 受控停止时出现 `STATUS_CONTROL_C_EXIT` 及 Chromium 注销警告，但 `xarchive-desktop.exe` 已退出且无残留项目进程，不构成启动失败。
3. Computer Use 的浏览器只读 AX 探测可用，但原生 Windows 桌面服务未配置，不能继续 Tauri GUI、WebView2/DPI、键盘/辅助技术或浏览器 Native Messaging 的交互验收；没有用启动、静态检查或库级测试替代这些验收。
4. Linux 后续处理：固化 Windows `PYTHON` 前置条件；决定当前 M5 working tree（含未跟踪 migration）的提交边界；准备受控 gallery-dl、旧版本 SQLite、Desktop 应用级归档/重启、aria2 fallback、Native Host/Named Pipe/Registry、GUI Computer Use、Edge/X/Telegram 的前置条件后再执行对应队列项。以上项目均不阻塞后续 Linux 开发，本轮不修改业务代码。

本轮仅将本验证记录写回 Linux 的 `docs/development/windows-validation.md`；没有向 Linux 反向同步代码、依赖或构建产物。

### Windows validation of latest Linux working tree（Security hardening，2026-09-12 18:14–18:26 +08:00）

本轮以 Linux 当前 working tree 为唯一源执行 Windows 验证。source branch 为 `main`，HEAD 为 `463acd864cf1136d44f4ca42647815a47b68140a`，相对 `origin/main` ahead 1；验证开始时 working tree 为 dirty，包含 7 个已跟踪修改（protocol、sidecar supervisor、storage、Desktop 及三份开发文档）和未跟踪的 `OPENAI_CODEX_WRITING_RULES.md`。本轮未修改业务代码，验证对象是同步后的 working tree，不是纯 Git commit。

Validation environment：Windows 11 专业工作站版 Insider Preview `10.0.29661`，AMD64；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、系统及项目 Python `3.14.7`、项目 pytest `9.1.1`、Tauri CLI `2.11.4`、Visual Studio Build Tools `17.14.40`、WebView2 `152.0.4191.66`。Windows 工作副本为 `E:\Shiraishi\VSCode Workspace\Tw2Tg`（当前任务目录 `Tw2Tg-CodexAlias` 为其 junction）。

| Scope | Category | Command / evidence | Working directory | Status | Summary |
|---|---|---|---|---|---|
| Linux source → Windows E: 受控同步 | Required | `robocopy /E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT /R:1 /W:1`；exit `3`，`FAILED=0`、`Mismatch=0`；19 个关键源码、迁移、Schema 和文档 SHA-256 全匹配 | `W:\home\shiraishi\VSCode Workspace\Tw2Tg` → `E:\Shiraishi\VSCode Workspace\Tw2Tg` | PASS | 未使用镜像删除；保留 E: `.venv`、`node_modules`、`target`、`validation-artifacts`、`X-Archive`、Tauri 生成目录和目标额外文件；未复制 `.git`、依赖、缓存、构建产物或用户数据 |
| Node check | Required | `npm run check` | `E:\Shiraishi\VSCode Workspace\Tw2Tg` | PASS | Vite 35 modules；Extension syntax check exit `0` |
| Node tests | Required | `npm run test` | 同上 | PASS | Extension `7/7`；Desktop Node test `0` 项且无失败 |
| Node production build | Required | `npm run build` | 同上 | PASS | Vite 35 modules，exit `0` |
| Rust formatter | Required | `cargo fmt --all -- --check` | 同上 | PASS | exit `0` |
| Rust workspace check | Required | `cargo check --workspace --all-targets` | 同上 | PASS | exit `0` |
| Rust strict clippy | Required | `cargo clippy --workspace --all-targets -- -D warnings` | 同上 | FAIL | `crates/xarchive-storage/src/lib.rs:962` 的 `complete_sidecar_archive` 为 8 个参数，Rust 1.98 报 `clippy::too-many-arguments (8/7)`，exit `101` |
| Rust workspace tests（默认 Windows 命令环境） | Required | 清除 `PYTHON` 后 `cargo test --workspace --no-fail-fast` | 同上 | FAIL | 92 项 crate tests 中 90 项通过；`xarchive-sidecar-supervisor` 的 2 个真实 Python worker handshake 测试返回 `NotRunning`，exit `101`；doc-tests 通过 |
| Rust workspace tests（项目 Python 前置条件） | Required | `$env:PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe; cargo test --workspace --no-fail-fast` | 同上 | PASS | crate tests `92/92`，所有 doc-tests 通过 |
| Python Sidecar tests | Applicable | `.venv\Scripts\pytest.exe sidecar/tests -q` | 同上 | PASS | `10 passed` |
| Python compile check | Applicable | `.venv\Scripts\python.exe -m compileall -q sidecar/src sidecar/tests` | 同上 | PASS | exit `0` |
| Security targeted Rust regression | Applicable | 协议 URL/Tweet ID 拒绝 2 项；storage metadata ID、非法 settings、路径逃逸各 1 项定向测试 | 同上 | PASS | 实际运行 `5/5` 通过；完整 workspace 也覆盖 settings、metadata identity 和 Windows reparse 分支编译 |
| Shared protocol Schema JSON parse | Applicable | PowerShell `ConvertFrom-Json` 解析 archive/browser/download 三份 schema | 同上 | PASS | JSON 可解析；但见下方 `executable` 契约残留错误 |
| Tauri Release build | Required | `npm run build:tauri` | 同上 | PASS | 生成 `target\release\xarchive-desktop.exe`，exit `0` |
| Tauri Debug startup | Required | `npm run dev:tauri`；观察 `xarchive-desktop.exe`、窗口标题和 Vite 1420 端口 | 同上 | PASS | `Responding=True`，标题 `XArchive`，端口监听；受控停止后项目进程数 `0`、1420 端口不再监听 |
| Tauri installer/package | Not applicable | `desktop/src-tauri/tauri.conf.json` | 同上 | NOT APPLICABLE | `bundle.active=false` 且 `createUpdaterArtifacts=false`，当前没有 installer/package 目标 |
| WQ-P1-12 安全边界回归（聚合） | Applicable | 上述定向测试、Tauri handler/结构静态审查、Schema 审查 | 同上 | FAIL | Rust/Tauri 已无 per-request `executable` 字段、未注册 settings IPC，identity/settings/path 基础测试通过；但 `shared/protocol-schema/download-command.schema.json:15` 仍声明 `executable` property，且 Windows symlink/junction/reparse 专项未实际执行，聚合预期尚未满足 |
| WQ-P1-13 归档数据 ACL 跨用户验收 | Required | 只读 `Get-Acl` 检查 `X-Archive`、SQLite/WAL/SHM | 同上 | NOT RUN | 当前用户 ACL 未见 `Everyone` ACE，数据文件位于 `X-Archive`；但无第二个本地用户、空白 profile 或跨用户读取尝试，不能替代完整验收 |

#### 错误分析与 Linux 后续事项

1. **Rust strict clippy FAIL（项目代码，非 Windows 专属）：**本轮安全改动给 `complete_sidecar_archive` 增加了 `expected_tweet_id`，当前签名达到 8 个参数，触发 `-D warnings`。建议 Linux 将该参数并入已有请求/上下文结构或采用等价的低复杂度重构；修复后先跑 Linux fmt/check/test，再重新执行 Windows clippy。验证期间未直接修改此代码。
2. **默认 `PYTHON` 前置条件 FAIL（环境问题，非业务代码 FAIL）：**清除环境变量后两个真实 worker handshake 测试返回 `NotRunning`；设置项目 `.venv\Scripts\python.exe` 后 92/92 通过。Linux/CI 后续应固化 Windows Python 解释器前置条件，同时保留默认环境 FAIL 与项目环境 PASS 的证据边界。
3. **共享 Schema 契约残留（项目代码/契约问题）：**Rust `SidecarCommand` 和 Tauri `ArchiveTweetRequest` 已移除 per-request `executable`，但 `shared/protocol-schema/download-command.schema.json` 仍允许该字段。Linux 后续应删除该 property、更新 schema fixtures/校验测试，并重新做跨层验证；在此之前不得宣称 executable override 安全边界完整闭环。
4. **reparse/link 专项未完成：**Windows 分支的 `symlink_metadata`/`FILE_ATTRIBUTE_REPARSE_POINT` 代码已通过 check/test/build 编译，但当前仓库没有直接调用该 API 的 Windows symlink/junction harness。本轮不能把普通文件、路径逃逸测试或静态代码存在性提升为 reparse PASS。
5. **Tauri 停止时的非阻塞警告：**受控 Ctrl+C 停止返回 `STATUS_CONTROL_C_EXIT`，并出现 Chromium `Failed to unregister class Chrome_WidgetWin_0. Error = 1411`；应用已退出且无残留进程，因此不构成启动失败。Cargo/MSVC 还输出生成 `.lib/.exp` 的 linker stdout warning，不影响构建结果。
6. **Windows Computer Use / GUI：**Computer Use helper 初始化及重试均返回 `helper_unknown_error: setup refresh had errors`，因此真实 WebView2/DPI、Tab/键盘/焦点、辅助技术、对比度和原生 Tauri 窗口交互均为 `BLOCKED`。没有用静态结构、启动进程或 Node/Rust 测试替代 GUI 验收。

#### Not Executed / Blocked

- WQ-P1-13 的第二用户/非管理员 ACL 读取、不同 working directory/盘符的应用级路径稳定性：`NOT RUN`，缺少第二用户、空白 profile 和独立受控应用场景。
- WQ-P0-02/WQ-M5-05 真实 Edge/X 归档、Quote/Reply DOM → Native Messaging → Desktop → SQLite：`BLOCKED`，缺少 Native Host 注册、可用浏览器实机/公开受控样本、真实账号，且原生 GUI Computer Use 不可用。
- WQ-M5-06 真实 gallery-dl payload merge：`NOT RUN`，缺少可重复的真实 payload 和 Desktop 归档驱动；库级逻辑已在 92/92 中通过。
- WQ-M5-07/WQ-M5-10 旧版 SQLite `0001/0002 → 0003` 的 Desktop 应用级升级：`NOT RUN`，没有对应旧数据库副本；storage 层 migration 已由 `crates/xarchive-storage/migrations/` 自主管理，库级测试通过。
- WQ-M5-08 Unicode/空格路径的 Desktop 应用级带引用归档、WQ-M5-11 profile/SQLite 联动：`NOT RUN`，缺少应用级驱动和可核验用户数据场景。
- Router/aria2 真实 fallback、403 后重新提取、transfer lifecycle、Named Pipe/Registry/Tray/Autostart/Credential Manager、真实 Telegram：`NOT RUN` 或 `BLOCKED`，所需 artifact、backend、凭据或外部服务未具备；本轮未扩大为开发任务。
- GUI WebView2/DPI/键盘/屏幕阅读器/对比度：`BLOCKED`，原生自动化 target 不可用。

#### Linux reconciliation / next actions

- 本轮 Linux 后续必须优先处理 clippy 8 参数 FAIL 和共享 `download-command.schema.json` 的 `executable` 残留；两项完成后重新执行 Linux fmt/check/test，并将 WQ-P1-12 置回待 Windows 复验而非直接标记通过。
- 为 WQ-P1-12 增加可重复的 Windows symlink/junction/reparse、超长 settings JSON 和合法 Unicode 文件场景；为 WQ-P1-13 准备第二用户/非管理员 ACL 验证环境。
- 固化 Windows `PYTHON` 前置条件；准备旧版 SQLite、Desktop 应用级归档/重启、gallery-dl/aria2、Native Host/Named Pipe、GUI automation、Edge/X/Telegram 的受控前置条件后再执行对应队列项。
- 本轮没有向 Linux 反向同步代码、依赖或构建产物；Linux 源 `git diff --check` 通过，源 working tree 除本次验证文档追加外未产生修改。

### Linux reconciliation after latest Windows security-hardening validation（2026-09-12）

Linux 已重新读取本轮 Windows 验证结果、当前 `git diff`、现有 Plan 和安全加固代码。最新 Windows 结果不是全量 PASS：`complete_sidecar_archive` 的 8 参数 clippy FAIL 属于项目代码问题，`download-command.schema.json` 和 Python Worker 仍保留 per-request `executable` 属于跨层契约残留；WQ-P1-12 聚合项因此保持 FAIL，不能被 Linux 验证直接改写为 Windows PASS。

本轮 Linux reconciliation 已完成：

1. 新增 `SidecarArchiveRequest` 上下文结构，收敛 `ArchiveService::complete_sidecar_archive` 参数并保持 Tweet ID/metadata identity binding。
2. 删除 `shared/protocol-schema/download-command.schema.json` 的 `executable` property。
3. 删除 Python Worker 从 Sidecar command 读取 `executable` 的逻辑及对应测试输入；内部 `GalleryDlConfig.executable` 仍仅作为可信 Sidecar 配置，不属于跨进程用户输入。
4. 保留 Windows reparse/junction、ACL、真实 Edge/X、GUI、Native Host、应用级 SQLite 和真实 Telegram 等未完成项目，不扩大为本轮 Linux 阻塞。

Linux 验证结果：

| 验证项目 | 状态 | 结果 |
|---|---|---|
| Rust formatter | PASS | `cargo fmt --all -- --check` |
| Rust workspace check | PASS | `cargo check --workspace` |
| Rust workspace tests | PASS | `cargo test --workspace --no-fail-fast`；全部 workspace tests/doc-tests 通过 |
| Rust clippy | NOT RUN | Linux stable toolchain 未安装 `cargo-clippy`；不得用此结果替代 Windows strict clippy |
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build` |
| Python compile | PASS | `python3 -m compileall -q sidecar/src` |
| Python pytest | NOT RUN | 当前 Linux 环境未安装 `pytest` |
| Shared schema JSON parse | PASS | `archive`、`browser`、`download` schema 全部可解析 |
| Git diff check | PASS | `git diff --check` |

下一步 Windows 状态：

| 项目 | 状态 | 下一步 |
|---|---|---|
| WQ-P1-12 安全边界回归 | WINDOWS_VERIFICATION_PENDING | 同步最新 Linux working tree 后重新执行 strict clippy、Schema/Worker 契约审查，并在可用时执行 symlink/junction/reparse 专项 |
| WQ-P1-13 归档数据 ACL | WINDOWS_VERIFICATION_PENDING | 准备第二用户/非管理员账户、空白 profile 和跨用户读取场景 |
| 真实 X、Edge Cookie、GUI、Named Pipe、Registry、Credential Manager、应用级 SQLite、aria2 fallback、真实 Telegram | WINDOWS_VERIFICATION_PENDING / BLOCKED / NOT RUN | 继续等待各自 backend、artifact、automation、浏览器或凭据前置条件 |

本轮没有 Windows-specific blocking 项。此前 Windows report 中的 FAIL 保留为历史事实；Linux 修复仅使相关项目重新进入待复验状态，不提前宣称 Windows PASS。

### Windows validation of latest Linux working tree（Security hardening + storage migration ownership，2026-09-12 20:26–20:36 +08:00）

本轮再次以 Linux 当前 working tree 为唯一源执行 Windows 平台验证。source branch 为 `main`，HEAD 为 `463acd864cf1136d44f4ca42647815a47b68140a`，相对 `origin/main` ahead 1；开始时 working tree 为 dirty，包含 16 个已跟踪修改（包括 protocol、sidecar supervisor、storage、Desktop、迁移归属及相关文档）和 2 个未跟踪条目：`OPENAI_CODEX_WRITING_RULES.md`、`crates/xarchive-storage/migrations/` 下的 3 个迁移文件。本轮没有修改业务代码，验证对象是同步后的 working tree，不是纯 Git commit。

Validation environment：Windows 11 专业工作站版 Insider Preview `10.0.29661`，x64；WebView2 `152.0.4191.66`；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、系统及项目 Python `3.14.7`、项目 pytest `9.1.1`、Tauri CLI `2.11.4`。项目 Python 为 `E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe`。Windows 工作副本为 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；当前任务目录 `Tw2Tg-CodexAlias` 为该目录的 junction。

| Scope | Category | Command / evidence | Working directory | Status | Summary |
|---|---|---|---|---|---|
| Linux source → Windows E: 受控同步 | Required | `robocopy /E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT /R:1 /W:1`；exit `3`，failed/mismatch `0` | `W:\home\shiraishi\VSCode Workspace\Tw2Tg` → `E:\Shiraishi\VSCode Workspace\Tw2Tg` | PASS | 先确认 Linux 已删除旧 `desktop/src-tauri/migrations/0001–0003`，再将 E: 中同名陈旧文件移入 `validation-artifacts/stale-source-files-2026-09-12`，随后同步新增 `crates/xarchive-storage/migrations/0001–0003`；未使用镜像删除，依赖、缓存、数据库、用户数据和 `.git` 未同步 |
| Node check | Required | `npm run check` | 同上 | PASS | Vite 转换 35 modules；Extension `node --check` 通过，exit `0` |
| Node tests | Required | `npm run test` | 同上 | PASS | Extension `7/7`；Desktop Node test `0` 项且无失败 |
| Node production build | Required | `npm run build` | 同上 | PASS | Vite 35 modules，exit `0` |
| Rust formatter | Required | `cargo fmt --all -- --check` | 同上 | PASS | exit `0` |
| Rust workspace check | Required | `cargo check --workspace --all-targets` | 同上 | PASS | exit `0` |
| Rust strict clippy | Required | `cargo clippy --workspace --all-targets -- -D warnings` | 同上 | PASS | exit `0`；此前 `complete_sidecar_archive` 的 8 参数问题已不再复现 |
| Rust workspace tests（默认 Windows 命令环境） | Required | 清除 `PYTHON` 后 `cargo test --workspace --no-fail-fast` | 同上 | FAIL | `xarchive-sidecar-supervisor` 的 `communicates_with_a_real_python_worker_when_available`、`spawn_ready_completes_the_hello_handshake` 返回 `NotRunning`；exit `101`，其余 crate tests 通过 |
| Rust workspace tests（项目 Python 前置条件） | Required | `$env:PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe; cargo test --workspace --no-fail-fast` | 同上 | PASS | crate tests `92/92`，所有 doc-tests 通过；storage `23/23`，Sidecar supervisor `4/4` |
| Python Sidecar tests | Applicable | `.venv\Scripts\python.exe -m pytest sidecar/tests` | 同上 | PASS | `10 passed`，win32/Python 3.14.7 |
| Python compile check | Applicable | `.venv\Scripts\python.exe -m compileall -q sidecar/src` | 同上 | PASS | exit `0` |
| Security / migration targeted regression | Applicable | 协议 URL/Tweet ID 2 项；storage metadata/path 2 项；migration upgrade/reopen 2 项定向 `cargo test` | 同上 | PASS | 实际运行 `6/6` 通过；完整 workspace 亦覆盖 settings、metadata identity、path escape 和迁移回归 |
| Schema / Worker / migration ownership static audit | Applicable | PowerShell `ConvertFrom-Json`、输入契约字符串审查、迁移路径存在性/旧路径缺失检查 | 同上 | PASS | download schema 可解析且无 per-request `executable`；Worker 不再读取 command `executable`；新迁移文件存在，旧 Desktop 迁移路径不存在；剩余 `GalleryDlConfig.executable` 仅为可信内部配置 |
| Tauri Release build | Required | `npm run build:tauri` | 同上 | PASS | 生成 `target\release\xarchive-desktop.exe`，exit `0` |
| Tauri Debug startup/cleanup（进程级） | Required | `npm run dev:tauri`；检查 Vite 1420 端口、`xarchive-desktop.exe`、WebView2，再受控停止 | 同上 | PASS | Vite `localhost:1420` 就绪，Desktop/WebView2 进程出现；停止后相关进程为 `0`、1420 端口不再监听；这不是 GUI 交互验收 |
| Tauri installer/package | Not applicable | `desktop/src-tauri/tauri.conf.json` | 同上 | NOT APPLICABLE | `bundle.active=false` 且 `createUpdaterArtifacts=false`，当前无 installer/package 目标 |
| WQ-P1-12 自动化安全边界子集 | Applicable | 上述定向测试、Schema/Worker/迁移静态审查、Tauri handler 结构审查 | 同上 | PASS | clippy、身份绑定、settings、路径逃逸、Schema/Worker `executable` 契约和迁移归属均通过 |
| WQ-P1-12 symlink/junction/reparse 及长 JSON/Unicode 专项 | Applicable | 项目当前无可重复的 Windows 专项 harness；本轮未伪造替代测试 | 同上 | NOT RUN | Windows reparse/junction、超长 settings JSON、合法 Unicode 文件场景仍未直接执行；WQ-P1-12 总体继续 `WINDOWS_VERIFICATION_PENDING` |
| WQ-P1-13 归档数据 ACL 跨用户验收 | Required | 需要第二本地用户/非管理员和空白 profile 的跨用户读取 | 同上 | NOT RUN | 本轮未具备第二用户和独立应用场景；不以当前用户只读 ACL 代替完整验收 |
| Desktop 应用级旧库迁移/重启、Unicode/空格归档 | Applicable | 需要旧数据库副本及 Desktop 应用级驱动 | 同上 | NOT RUN | storage 库级迁移测试通过，但未执行实际 Desktop 归档、重启和用户数据场景 |
| 真实 Edge/X、Native Messaging、gallery-dl、aria2、Telegram、Named Pipe/Registry/Tray/Credential Manager | Applicable | 需要对应 backend、注册、artifact、账号、凭据或外部服务 | 同上 | BLOCKED / NOT RUN | 缺少受控外部前置条件；本轮未扩大为开发任务 |
| GUI WebView2/DPI/键盘/焦点/辅助技术/对比度 | Applicable | 原生 Computer Use helper 初始化 | 同上 | BLOCKED | `cua.getState()` 后 trusted Node process 异常退出并 reset，无法安全枚举或交互原生窗口；未将进程启动、静态审查或库级测试提升为 GUI PASS |

#### 错误分析与 Linux 后续事项

1. **默认 `PYTHON` 前置条件 FAIL（环境问题，非业务代码 FAIL）：**清除环境变量后两个真实 Python worker handshake 测试返回 `NotRunning`；显式设置项目 `.venv\Scripts\python.exe` 后完整 `92/92` 通过。Linux/CI 后续应固化 Windows Python 解释器前置条件，并保留默认环境 FAIL 与项目环境 PASS 的证据边界。
2. **此前 clippy/schema/Worker 问题已在本轮消除：**`SidecarArchiveRequest` 使 strict clippy 通过；download schema 与 Python Worker 的 per-request `executable` 残留已消除。`GalleryDlConfig.executable` 仍是可信 Sidecar 内部配置，不应与跨进程 command contract 混同。
3. **迁移归属已在验证副本生效：**新的 `crates/xarchive-storage/migrations/0001–0003` 随源码同步并通过 storage workspace、upgrade/reopen 测试；E: 中 Linux 已删除的旧 Desktop 迁移文件仅被移入排除的本地验证归档，没有反向写回 Linux 代码。
4. **非阻塞工具输出：**MSVC linker 输出生成 `.lib/.exp` 的 warning；Tauri 受控 Ctrl+C 停止使 dev 命令返回非零终端状态，但应用进程、WebView2 和 1420 端口均已清理，不构成启动失败。
5. **Computer Use / GUI BLOCKED：**当前调用返回 `trusted Node process exited unexpectedly; kernel reset, rerun your request`，无法安全完成原生 Tauri 窗口、WebView2/DPI、键盘/焦点、辅助技术或浏览器 Native Messaging 的交互验收。

#### Not Executed / Blocked

- WQ-P1-12 的 Windows symlink/junction/reparse、超长 settings JSON、合法 Unicode 文件专项：`NOT RUN`，当前仓库没有可重复 harness；自动化契约、身份绑定、settings、路径逃逸和迁移子集已 `PASS`，但不能替代这些专项。
- WQ-P1-13 的第二用户/非管理员 ACL 读取、不同 working directory/盘符的应用级路径稳定性：`NOT RUN`，缺少第二用户、空白 profile 和独立 Desktop 应用场景。
- WQ-M5-07/WQ-M5-10 旧版 SQLite `0001/0002 → 0003` 的 Desktop 应用级升级、WQ-M5-08 Unicode/空格路径归档、WQ-M5-11 profile/SQLite 联动：`NOT RUN`，storage 层测试通过但没有旧数据库副本和应用级驱动。
- 真实 Edge/X、Native Host/Named Pipe/Registry/Tray/Credential Manager、gallery-dl、aria2、Telegram 和 GUI：`BLOCKED` 或 `NOT RUN`，缺少对应 backend、注册、artifact、账号、凭据或可用自动化；本轮未扩大为开发任务。

#### Linux follow-up / next actions

- WQ-P1-12 总体继续 `WINDOWS_VERIFICATION_PENDING`：为 symlink/junction/reparse、超长 settings JSON、合法 Unicode 路径准备可重复的 Windows harness 后再复验。
- WQ-P1-13 继续待第二用户/非管理员 ACL 场景；不要用当前用户只读 ACL 或库级测试替代跨用户验收。
- 确认当前 working tree 中新增的 `crates/xarchive-storage/migrations/*.sql` 与删除的 `desktop/src-tauri/migrations/*.sql` 的提交边界，并在 Linux/CI 固化 Windows `PYTHON` 前置条件。
- 准备旧版 SQLite、Desktop 应用级归档/重启、gallery-dl/aria2、Native Host/Named Pipe、GUI automation、Edge/X/Telegram 的受控前置条件后再执行对应队列项。
- 本轮未发现需要立即修复的 Windows 业务代码问题；上述未执行/阻塞项目不阻塞后续 Linux 开发。本轮只需将本验证记录写回 Linux，未反向同步代码、依赖或构建产物。

### Windows aria2 artifact revalidation using persistent development artifacts（2026-09-12 20:56–21:03 +08:00）

用户指定的 E: 本地半永久化 artifact 目录为 `E:\Shiraishi\VSCode Workspace\Tw2Tg-Windows-DevArtifacts\aria2`。本轮没有修改 Linux 源码、没有把 artifact 目录加入 PATH，也没有安装系统服务；仅在该目录下创建新的隔离测试目录 `runtime-test-2026-09-12-2110` 和 `runtime-test-2026-09-12-2145`。测试使用本地 Range-capable Python server，不访问外部网络或真实账号。

Artifact integrity：官方 ZIP SHA-256 为 `67D015301EEF0B612191212D564C5BB0A14B5B9C4796B76454276A4D28D9B288`，与项目 allowlist 一致；`aria2c --version` 为 `1.37.0`。16 MiB 本地 fixture SHA-256 为 `080ACF35A507AC9849CFCBA47DC2AD83E01B75663A516279C8B9D243B719643E`。

| 验证项目 | 状态 | 实际证据 |
|---|---|---|
| aria2 artifact integrity/version | PASS | `aria2c.exe --version`；ZIP/hash allowlist 匹配 |
| Local Range server + JSON-RPC | PASS | `range_server.py`；RPC `getVersion`、`addUri`、`tellStatus`、`pause`、`unpause`；loopback ports `18932/18933/18935/18936` |
| Unicode/空格路径基础下载 | PASS | `runtime-test-2026-09-12-2110`；`下载 Unicode 空格\归档 文件.bin` 16 MiB，SHA-256 与 fixture 一致 |
| Pause/resume | PASS | `runtime-test-2026-09-12-2145`；暂停前 `active`、已完成 `311296` bytes；暂停后 `paused`；暂停期间观察到 `.aria2`；恢复完成后 hash 一致且 `.aria2` 消失 |
| Forced process interruption/resume | PASS | 中断前 `active`、已完成 `311296` bytes；强制停止后观察到 `.aria2`；重启 aria2 并重新提交同一目标后完成，hash 一致且 `.aria2` 消失 |
| `.aria2` cleanup | PASS | Pause/resume 和 crash/resume 两个最终输出目录均无残留 `.aria2`；无残留 `aria2c.exe` 或 Range server 进程 |
| Test wrapper exit status | FAIL（harness only） | 最终测试断言和 hash 均通过，但 PowerShell 外层命令返回 `1`；清理后仅保留本地 TCP `TIME_WAIT`，没有 aria2 进程或活动监听端口。该退出码归因于 wrapper 清理/退出处理，不归因于 aria2 或项目业务代码 |
| Project `DownloadRouter` / Desktop Job real aria2 fallback | NOT RUN | 本轮验证的是独立 aria2 artifact/RPC/恢复子集；尚未把真实 `aria2c.exe` 接入当前 Desktop Job、gallery-dl 403 refresh 和 transfer lifecycle，因此完整项目级 aria2 队列仍保持 `NOT RUN` |

#### 过程中的 harness 问题

1. 第一次测试命令在 PowerShell 解析阶段失败，未启动进程、未产生测试结果。
2. 第二次尝试的 `Start-Process -ArgumentList` 未对含空格/Unicode 的路径逐项加引号，导致 aria2 将路径尾部误识别为 URI，Range server 也未正确接收 root 参数；该次结果为 harness FAIL，随后改用显式引号并重新执行。
3. 修正后的有效测试中，强制中断下载会使本地 Range server 记录 `ConnectionResetError / WinError 10054`；这是客户端主动断开连接的预期 fixture 噪声。aria2 最终文件 hash、大小和 `.aria2` 清理均通过。

#### 结论与后续

- 现有半永久化 artifact 已足以将 aria2 官方 artifact、Windows x64 版本、loopback RPC、Unicode/空格路径、暂停/恢复、进程中断恢复和 `.aria2` 清理记录为 `PASS`。
- 不应把该结果扩大为当前项目的 `DownloadRouter`、Desktop Job、gallery-dl 403 后重新提取 URL 或真实 transfer lifecycle `PASS`；这些仍需项目级接入和受控过期 URL/media-server 场景。
- 本轮新增的测试目录和日志均保留在 `E:\Shiraishi\VSCode Workspace\Tw2Tg-Windows-DevArtifacts\aria2`，不反向同步到 Linux 源，也未修改业务代码。

### Windows validation of latest Linux working tree（当前追加复验，2026-09-12 约 21:54 +08:00）

本次追加复验仍以 Linux/WSL 源为唯一事实来源：branch 为 `main`，HEAD 为 `463acd864cf1136d44f4ca42647815a47b68140a`，相对 `origin/main` ahead 1，working tree 为 dirty。当前状态包含 16 个已跟踪修改（其中包含旧 Desktop migration 删除和验证文档修改），以及未跟踪的 `OPENAI_CODEX_WRITING_RULES.md`、`crates/xarchive-storage/migrations/`（3 个 SQL 文件）和 `desktop/src-tauri/src/archive_application.rs`。验证对象是该 dirty working tree，不是纯 commit。

本轮再次执行 Linux → E: 单向同步。目标实际工作副本为 `E:\Shiraishi\VSCode Workspace\Tw2Tg`；Codex 任务使用的 `Tw2Tg-CodexAlias` 是指向它的 junction。Robocopy 使用 `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`，退出码为 3（含更新/复制，不代表失败），failed/mismatch 均为 0；未同步 `.git`、依赖、`target`、缓存、用户数据、`validation-artifacts` 和本地配置。新增 `archive_application.rs`、storage migrations、schema、storage/lib.rs 等关键文件逐一 hash 对齐。

| 验证项目 | 状态 | 命令/证据与结果 |
|---|---|---|
| Node check/test/build | PASS | `npm run check`、`npm run test`、`npm run build`；Vite 35 modules，Extension 7/7，exit 0 |
| Rust fmt/check/clippy | PASS | `cargo fmt --all -- --check`、`cargo check --workspace --all-targets`、`cargo clippy --workspace --all-targets -- -D warnings`，均 exit 0 |
| Rust workspace tests（未设置 `PYTHON`） | FAIL（环境前置） | `cargo test --workspace --no-fail-fast`；两个 sidecar supervisor worker handshake 测试为 `NotRunning`，exit 101；不是业务代码失败 |
| Rust workspace tests（项目 Python） | PASS | 设置 `PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe` 后 crate tests 92/92 和全部 doc-tests 通过；Python 3.14.7 |
| Sidecar pytest/compile | PASS | 项目 venv `pytest 9.1.1`；`pytest sidecar/tests` 为 10 passed，`compileall -q sidecar/src` exit 0 |
| 安全/迁移定向回归及静态审查 | PASS | browser request 2 项、metadata/path 2 项、migration upgrade/reopen 2 项，共 6/6；schema 无 per-request `executable`，Worker 不读取该字段，新迁移路径存在且旧 Desktop 路径为空 |
| Tauri Release build（实际 E: 路径） | PASS | 在 `E:\Shiraishi\VSCode Workspace\Tw2Tg` 执行 `npm run build:tauri`；Vite 与 Rust release 均完成，生成 `target\release\xarchive-desktop.exe`，exit 0 |
| Tauri Release build（Codex junction 别名） | FAIL（路径入口） | 在 `Tw2Tg-CodexAlias` 执行同一命令时，Vite/Rollup 将 emitted HTML 资源名识别为 `../../Tw2Tg/desktop/index.html` 并报 “must be strings that are neither absolute nor relative paths”；未修改配置以规避 |
| Tauri Debug 启动/进程清理 | PASS（进程级） | `npm run dev:tauri` 在实际 E: 路径启动 Vite `localhost:1420`、`xarchive-desktop.exe` 和 WebView2；受控 Ctrl+C 后相关进程为 0、1420 端口不再监听 |
| Tauri GUI/WebView2/DPI/键盘/辅助技术 | 跳过（不进行验证） | 按本轮范围决定不启动原生 GUI 验收；既有 Computer Use helper 错误保留在历史记录，不作为本轮待验证项 |
| aria2 工件完整性/版本复核 | PASS | 半永久化目录存在；`aria2c` 1.37.0，ZIP SHA-256 `67D015301EEF0B612191212D564C5BB0A14B5B9C4796B76454276A4D28D9B288`，EXE SHA-256 `BE2099C214F63A3CB4954B09A0BECD6E2E34660B886D4C898D260FEBFE9D70C2` |
| aria2 RPC/Unicode/暂停恢复/中断恢复 | PASS（已有同日有效证据） | `runtime-test-2026-09-12-2110`、`runtime-test-2026-09-12-2145` 的本地 Range/RPC、Unicode/空格、pause/resume、强制中断后恢复和 `.aria2` 清理均通过；外层 wrapper 的清理退出码问题仍按既有记录标为 harness-only |
| 项目 DownloadRouter/Desktop Job 真实 aria2 fallback | NOT RUN | 本轮没有把独立工件接入真实 Job、gallery-dl 403 refresh 或 transfer lifecycle；不将独立 aria2 PASS 扩大为项目集成 PASS |
| WQ-P1-12 自动化安全边界子集 | PASS | clippy、身份绑定、路径逃逸、settings、schema/Worker 契约和 storage migration 子集通过 |
| WQ-P1-12 reparse/junction/长 JSON/Unicode 专项 | NOT RUN | 当前仓库没有可重复的 Windows 专项 harness；未用普通路径测试替代 |
| WQ-P1-13 跨用户 ACL、Desktop 应用级旧库迁移 | 跳过（不进行验证） | 按本轮范围决定不创建第二用户、不准备旧数据库副本；storage 库级测试已通过，但不扩大为跨用户或 Desktop 应用级 PASS |
| 真实 Edge/X、Native Messaging、gallery-dl、Telegram、Named Pipe/Registry/Tray/Credential Manager | BLOCKED / NOT RUN | 缺少浏览器/注册/账号/凭据/外部服务等受控前置条件 |
| Tauri installer/package | NOT APPLICABLE | 当前 `bundle.active=false` 且 `createUpdaterArtifacts=false` |
| 未接入的 `desktop/src-tauri/src/archive_application.rs` | NOT RUN | 文件虽已随 Linux working tree 同步，但未被 `desktop/src-tauri/src/lib.rs` 引用，当前 Cargo 构建不会编译它；本轮不临时接线以扩大开发范围 |

#### 本轮错误分析

1. 默认 Windows 命令环境没有 `PYTHON`，sidecar supervisor 无法找到 worker，产生两个 `NotRunning`；指定项目 venv 后 92/92 通过，结论为环境前置 FAIL，不是业务代码 FAIL。
2. Junction 别名入口触发 Vite/Rollup 的真实路径输出名错误；实际 E: 工作副本构建通过，因此当前记录为“别名入口 FAIL、规范工作副本 PASS”。后续应固定使用 canonical E: 路径，或由 Linux 开发任务评估工具链对 junction 的兼容性。
3. Debug 停止时有 Chromium `Chrome_WidgetWin_0 Error = 1411` 和 `STATUS_CONTROL_C_EXIT`，但应用进程、WebView2 和端口均已清理；另有 MSVC 生成 `.lib/.exp` 的 linker stdout warning，均不阻塞构建/进程级结论。
4. `archive_application.rs` 是当前 dirty tree 的未跟踪代码范围，未接入构建不是 Windows 编译失败，而是当前仓库集成边界未确定；未对其做临时编译或修改。
5. GUI helper 仍不可用，因此不能把进程启动、静态审查或库级测试当作真实 GUI/可访问性验收。

#### Linux 后续处理

- 明确 `desktop/src-tauri/src/archive_application.rs` 是否属于本轮应提交的功能；若属于，应在独立 Linux 开发任务中完成模块接入、调用链和测试，再重新执行 Windows 验证；若不属于，应由维护者决定其保留边界。此项本轮未代为修改。
- 在 Linux/CI/Windows runbook 中固化项目 `.venv\Scripts\python.exe` 的 `PYTHON` 前置条件，保留“默认环境 FAIL / 正确项目环境 PASS”的证据区分。
- Windows 验证应使用 canonical `E:\Shiraishi\VSCode Workspace\Tw2Tg`，并另行评估 junction 别名入口的 Vite/Rollup 兼容性。
- 为 WQ-P1-12 补充可重复的 symlink/junction/reparse、超长 settings JSON 和合法 Unicode 文件 harness；为 WQ-P1-13 准备第二用户/非管理员 ACL 场景。
- 继续准备旧 SQLite 的 Desktop 应用级迁移/重启、真实 DownloadRouter/aria2 fallback、gallery-dl、Native Host、Edge/X、GUI 和 Telegram 的受控前置条件。本轮不扩大为开发任务。

本轮没有修改业务代码、依赖或架构；仅追加本验证记录。验证总体仍为部分通过，WQ-P1-12/WQ-P1-13 和真实集成/GUI 项目不得标记为全量 Windows PASS。

### 本轮范围调整与手动验证指南（2026-09-12）

本节是当前状态的覆盖说明。历史验证记录保留原状；以下“跳过”是本轮用户明确要求的范围决定，不是 PASS、FAIL 或 BLOCKED。其余项目保持 `NOT RUN（提供手册）`，本轮没有实际执行。

#### 已跳过（不进行验证）

| 项目 | 当前状态 | 跳过范围与原因 |
|---|---|---|
| 原生 GUI / WebView2 / DPI / 键盘 / 辅助技术 | 跳过（不进行验证） | 不启动原生 GUI 验收流程，不检查视觉渲染、DPI 缩放、Tab/Focus-visible、屏幕阅读器、对比度或命中区域。既有 Computer Use helper 阻塞记录保留为历史事实，但本轮不再把它作为待验证项。 |
| WQ-P1-13 跨用户 ACL | 跳过（不进行验证） | 不创建第二用户、不切换非管理员账户、不读取或修改跨用户 ACL；不对 `X-Archive`、SQLite、WAL/SHM 或 staging 做跨用户访问结论。 |
| Desktop 应用级旧数据库迁移 | 跳过（不进行验证） | 不准备旧版 Desktop 数据库，不执行应用启动时的 `0001/0002 → 0003` 或旧 schema 升级，不以 storage 库级 migration 测试替代应用级迁移验收。storage 层已有测试结果仍保留，但不扩大为 Desktop PASS。 |

#### 保留为 NOT RUN 的项目及手动步骤

以下步骤是未来在明确恢复验证范围、具备相应 harness/权限后使用的操作指南。执行时必须使用专用临时目录，不能使用真实用户归档或生产 SQLite；每项完成后记录实际命令、路径、日志和 PASS/FAIL。

##### 1. symlink / junction / reparse 拒绝

目的：确认归档 staging 或 archive root 内出现 Windows link/reparse 对象时，系统拒绝处理，不跟随链接写入 archive root 外部。

前置条件：canonical E: 工作副本、PowerShell、可创建 symbolic link/junction 的权限（Developer Mode 或管理员权限）、一个专用临时目录。下面命令只建立 fixture，不代表验证已经执行：

~~~powershell
$manual = 'E:\Shiraishi\VSCode Workspace\Tw2Tg-Windows-DevArtifacts\manual-windows-scope-2026-09-12'
$root = Join-Path $manual 'archive-root'
$outside = Join-Path $manual 'outside'
New-Item -ItemType Directory -Force -Path (Join-Path $root 'staging'), (Join-Path $outside 'payload') | Out-Null
Set-Content -LiteralPath (Join-Path $outside 'payload\marker.txt') -Value 'outside-marker' -NoNewline
New-Item -ItemType SymbolicLink -Path (Join-Path $root 'staging\symbolic-file.txt') -Target (Join-Path $outside 'payload\marker.txt')
New-Item -ItemType Junction -Path (Join-Path $root 'staging\junction-dir') -Target (Join-Path $outside 'payload')
Get-Item -LiteralPath (Join-Path $root 'staging\symbolic-file.txt'), (Join-Path $root 'staging\junction-dir') | Format-List FullName,LinkType,Attributes,Target
fsutil reparsepoint query (Join-Path $root 'staging\symbolic-file.txt')
fsutil reparsepoint query (Join-Path $root 'staging\junction-dir')
~~~

随后使用未来的 WQ-P1-12 Windows harness 或已批准的 Desktop archive path，提交包含这两个对象的 staging/job。预期结果：在读取、复制、commit 或 metadata conversion 前明确返回 link/reparse 拒绝错误；`outside\payload\marker.txt` 不被修改或追加；archive root 内不生成跟随链接后的副本；日志不泄露不必要的绝对敏感路径。使用 `Get-ChildItem -Force` 和 SHA-256 对比确认结果。没有 harness 时，不要把 `fsutil` 的对象识别结果当作业务 PASS。

清理时仅删除上述专用 `$manual` 目录，并先确认解析后的绝对路径仍位于 `Tw2Tg-Windows-DevArtifacts` 下；不要对 E: 根目录或真实 `X-Archive` 使用递归删除。

##### 2. 长 JSON / settings allowlist

目的：确认 settings 只接受 `ui.*`/`download.*`，值必须是合法 JSON，且 UTF-8 字节数不超过当前代码规定的 16 KiB 上限。

当前 Desktop 已移除 generic settings Tauri IPC，因此不能通过不存在的 UI 命令伪造手动结果。恢复该验证时，应使用受控 storage integration harness 或明确批准的测试入口调用 `Database::set_setting`；直接编辑 SQLite 只能检查数据库文件，不能证明输入校验。

生成边界 fixture：

~~~powershell
function New-JsonAtBytes([int]$bytes) {
  $prefix = '{"value":"'
  $suffix = '"}'
  $n = $bytes - [Text.Encoding]::UTF8.GetByteCount($prefix + $suffix)
  if ($n -lt 0) { throw 'requested size is too small' }
  return $prefix + ('a' * $n) + $suffix
}
$json16384 = New-JsonAtBytes 16384
$json16385 = New-JsonAtBytes 16385
[Text.Encoding]::UTF8.GetByteCount($json16384)
[Text.Encoding]::UTF8.GetByteCount($json16385)
~~~

对受控 harness 依次执行：`ui.manual` + `$json16384` 应接受；`ui.manual` + `$json16385` 应拒绝；`ui.manual` + 未闭合 JSON 应拒绝；`telegram.bot_token` 即使 JSON 合法也应拒绝；合法 `download.*` JSON 应接受。记录返回的稳定错误类别、SQLite 是否保持旧值、是否产生部分写入。Unicode boundary fixture 必须按 UTF-8 字节数而不是 PowerShell 字符数计算。

##### 3. Unicode / 空格路径与普通重启

目的：在不进行 GUI 视觉验收的前提下，确认应用工作目录、archive root、SQLite、staging、archives 和 profile JSON 在 Unicode/空格路径下能定位并可在重启后复用。

准备专用运行目录并使用 canonical E: 构建出的 Desktop executable：

~~~powershell
$runRoot = 'E:\Shiraishi\VSCode Workspace\Tw2Tg-Windows-DevArtifacts\manual-windows-scope-2026-09-12\归档 空格'
New-Item -ItemType Directory -Force -Path $runRoot | Out-Null
$exe = 'E:\Shiraishi\VSCode Workspace\Tw2Tg\target\debug\xarchive-desktop.exe'
$p = Start-Process -FilePath $exe -WorkingDirectory $runRoot -PassThru
~~~

使用可重复的受控 archive fixture 或已批准的 browser/Native Host 链路完成一条归档。当前实现以 process current directory 加 `X-Archive` 作为默认 archive root，因此应检查：

~~~powershell
$archiveRoot = Join-Path $runRoot 'X-Archive'
Test-Path (Join-Path $archiveRoot '_database\archive.sqlite3')
Get-ChildItem -LiteralPath $archiveRoot -Recurse -Force
~~~

预期：SQLite、`archives\<tweet-id>\tweet.json`、媒体文件、`Users\<stable>\profile.json` 均位于该 Unicode/空格 root 内；JSON 以 UTF-8 无损读取；没有生成工作目录外的 staging 或归档文件；同一个 tweet 不产生非预期重复 job。先正常停止进程，再确认无残留 `xarchive-desktop.exe`/WebView2 和数据库锁定；用同一 `$runRoot` 再启动，检查 archive root、数据库 schema、已有 job 和 profile 仍可读取。此处是普通重启/路径行为，仍不包含本轮明确跳过的旧版数据库迁移。

##### 4. 真实归档链路

目的：确认真实或受控 browser request → Native Host/Sidecar → DownloadRouter → staging → SQLite/archive files 的完整行为。它不是 GUI 视觉验收，但需要真实功能入口或批准的集成 harness。

前置条件：测试用 Edge Profile 或公开受控样本、已注册 Native Host（如使用）、项目 Python/Sidecar、可重复媒体源或本地 fake media server、专用 `$runRoot` 和可清理的测试 tweet。没有这些前置条件时继续标记 `NOT RUN`，不要使用个人账号替代。

步骤：

1. 记录 source/working copy、archive root、测试 tweet ID、URL、运行时间和 Sidecar/aria2 版本。
2. 启动 Desktop，确认进程级 health/Sidecar ready；通过已有 Extension/Native Host 或批准的 harness 提交一条归档请求。
3. 观察 `created → download started → complete/failed` 事件、job 状态和 Sidecar 日志；检查 URL/Tweet ID、metadata identity、Quote/Reply 关系和文件 SHA-256。
4. 检查 `X-Archive\_database\archive.sqlite3`、`archives\<tweet-id>`、`Users\<stable>\profile.json` 和 staging cleanup；确认失败时不留下伪成功的完成状态。
5. 停止并重启应用后重新查询同一 job，确认状态一致、不会无意重新下载；最后保存摘要日志并清理专用运行目录。

预期结果：合法请求只写入 archive root，metadata/文件/SQLite 一致，失败可诊断且不泄露 token/cookie；真实 gallery-dl、Native Host、Telegram 或 aria2 fallback 未执行前，不得把此项写成 PASS。

##### 5. 未被 lib.rs 引用的 archive_application.rs

当前文件是 dirty working tree 中的未跟踪文件，desktop/src-tauri/src/lib.rs 没有 `mod archive_application;` 或对应调用，因而当前 Cargo/Tauri 构建不会编译它。本轮不通过临时加 `mod`、复制文件或修改业务代码来制造验证条件，状态为 `NOT RUN（集成边界未确定）`。

手动复核步骤：

~~~powershell
rg -n 'mod archive_application|archive_application::|ArchiveApplication' E:\Shiraishi\VSCode Workspace\Tw2Tg\desktop\src-tauri\src E:\Shiraishi\VSCode Workspace\Tw2Tg\crates
Get-FileHash E:\Shiraishi\VSCode Workspace\Tw2Tg\desktop\src-tauri\src\archive_application.rs -Algorithm SHA256
~~~

预期当前结果是只能找到文件自身定义，找不到 lib.rs 的 module declaration。Linux 维护者若确认它应进入产品：应在独立开发任务中完成 module wiring、缺失 import/type/API 对齐、调用链替换和专门测试；先在 Linux 执行 `cargo fmt --all -- --check`、`cargo check --workspace --all-targets`、`cargo test --workspace`、`cargo clippy --workspace --all-targets -- -D warnings`，再同步到 canonical E: 路径重复同一套检查并做 Tauri build/startup。若确认它不是当前范围，则由维护者决定保留或移除；本验证任务不替其做决定。

以上手册仅定义未来验证方法。本轮实际结论为：前三项明确跳过；symlink/reparse、长 JSON、Unicode/普通重启、真实归档和 archive_application.rs 仍为 `NOT RUN`，不代表通过。

## 2026-09-12 Follow-up — Storage Archive Regression and Reparse Fixture Assessment

### Windows 归档流程回归

在 Windows 工作副本 `E:\Shiraishi\VSCode Workspace\Tw2Tg` 中执行：

- `rejects_sidecar_path_escape`：`PASS`
- `completes_archive_directly_from_sidecar_result`：`PASS`
- `converts_sidecar_files_using_actual_size_and_hash`：`PASS`
- `protects_files_and_commits_staging`：`PASS`

MSVC linker 的 `linker stdout` 警告和 `22 filtered out` 均不构成测试失败。

### 手工 reparse 夹具

已创建并检查：

- `junction-dir`
- `symbolic-dir`
- `symbolic-file.txt`
- `regular.txt`

外部 marker 文件内容仍为 `OUTSIDE-MARKER`，未发现意外归档或写入。

但现有 Rust 测试没有证据读取：

```text
manual-reparse-2026-09-12\archive-root\staging
```

但现有 Rust test suite 没有证据读取 `manual-reparse-2026-09-12\archive-root\staging` 目录之外的目标内容，也没有拒绝 `junction-dir`、`symbolic-dir` 或 `symbolic-file.txt` 这些 reparse 链接对象的断言。当前手动夹具仅能确认外部 marker 文件内容仍为 `OUTSIDE-MARKER`，未发现意外归档或写入，但这不构成 test 层面拒绝链接对象的证据。

本轮结论为：symlink/junction/reparse point 拒绝逻辑在 Windows 平台已有**手动**回归证据，但**自动化 test** 仍标记为 `WINDOWS_VERIFICATION_PENDING`，等待可重复的 Windows harness 验证。WQ-P1-12 仍需重新执行，以确认 `protects_files_and_commits_staging` 和 `rejects_sidecar_path_escape` 在 reparse fixture 下的行为，不能将手动记录替代自动化 test。

## 结论

本轮 Windows 追加复验（2026-09-12 约 21:54 +08:00）针对 Linux dirty working tree（branch `main`，HEAD `463acd8`，ahead `origin/main` 1，包含 16 个已跟踪修改）完成：

### 本轮 PASS（Linux reconciliation 对应）

- `cargo fmt --all -- --check`：PASS (Linux 验证)
- `cargo check --workspace --all-targets`：PASS (Linux 验证)
- `cargo clippy --workspace --all-targets -- -D warnings`：PASS (Windows re-validation)
- `cargo test --workspace`（项目 Python 前置）：PASS (92/92 crate tests + doc-tests)
- `npm run check/test/build`：PASS (Extension 7/7)
- `pytest sidecar/tests`：PASS (10 passed)
- `Tauri Release build (实际 E: 路径)`：PASS
- `Tauri Debug 启动/进程清理`：PASS
- `rejects_sidecar_path_escape`：PASS
- `completes_archive_directly_from_sidecar_result`：PASS
- `converts_sidecar_files_using_actual_size_and_hash`：PASS
- `protects_files_and_commits_staging`：PASS
- `browser request 2 项、metadata/path 2 项、migration upgrade/reopen 2 项`：PASS (6/6)

### 本轮 SKIPPED

- `GUI WebView2/DPI/键盘/辅助技术`：跳过（不进行验证）

### 本轮 BLOCKED

- `Tauri Release build（Codex junction 别名)`：FAIL（路径入口） — Vite/Rollup junction 别名解析问题，不是业务代码失败

### 本轮 NOT RUN / NOT APPLICABLE

- `Tauri installer/package`：NOT APPLICABLE (`bundle.active=false`)
- `Desktop 应用级旧数据库迁移`：跳过
- `WQ-P1-13 跨用户 ACL`：跳过
- `archive_application.rs`：NOT RUN

### 仍需 Windows 验证的项目

| 项目 | 状态 | 原因 |
|---|---|---|
| WQ-P1-12 安全边界回归 | WINDOWS_VERIFICATION_PENDING | 手动 reparse fixture 已有证据，但自动化 test 和 symlink/junction/reparse fixture 的可重复验证仍缺少 |
| WQ-P1-13 跨用户 ACL、Desktop 应用级旧库迁移 | 跳过（不进行验证） | 不创建第二用户、不准备旧数据库副本 |
| Windows workspace clippy | PASS | 已完成 re-validation |
| Linux clippy | NOT RUN | Linux stable toolchain 未安装 cargo-clippy |
| Linux pytest | NOT RUN | Linux 环境未安装 pytest |

**本轮没有修改业务代码、依赖或架构；仅追加验证记录。**

### Linux reconciliation

1. Windows 最新验证暴露的 `complete_sidecar_archive` clippy 8 参数问题已在 Linux 用 `SidecarArchiveRequest` 上下文结构修复。
2. `download-command.schema.json`/Python Worker 的 `executable` 契约残留已在 Linux 删除。
3. Linux `cargo fmt/check/test`、Node check/test/build、Python compileall 和 schema JSON parse 均已通过。
4. Windows strict clippy 对最新 Linux working tree 已 PASS，包括 `SidecarArchiveRequest` 修复。
5. WQ-P1-12 仍需 Windows re-validation，不能因 Linux fix 而标记 PASS。
6. `archive_application.rs` 移除为 untracked，清理完成，不在 Windows scope。

### Windows validation of latest Linux working tree (R1 executor contract batch, 2026-09-13 19:10–19:25 +08:00)

本轮按 [`cross-platform-validation.md`](cross-platform-validation.md)、[`../validation/windows.md`](../validation/windows.md) 和当前 Windows Validation Queue 读取 Linux source、Plan、working tree、变更模块和历史结果后执行。验证对象是 Linux `dev` 分支当前 dirty working tree，不是纯 Git commit：

- branch: `dev`
- HEAD: `e8069596722d1484ed36b57b90a8341f93dd8b23`
- working tree: dirty，包含 9 个已跟踪修改和未跟踪的 `desktop/src-tauri/src/executor.rs`；其中包括 R1 executor、Job 状态/Storage event contract 及相关文档变更
- Linux source: `W:\\home\\shiraishi\\VSCode Workspace\\Tw2Tg`
- Windows canonical validation workspace: `E:\\Shiraishi\\VSCode Workspace\\Tw2Tg`
- `E:\\Shiraishi\\VSCode Workspace\\Tw2Tg-CodexAlias` 是指向上述 canonical workspace 的 junction；构建和测试使用 canonical 路径，以避免将 junction 入口的历史 Vite/Rollup 路径问题混入标准验证结论

#### Validation Environment

| 项目 | 实际值 |
|---|---|
| Windows | `Microsoft Windows NT 10.0.29667.0`, x64 |
| Node/npm | `v24.19.0` / `11.17.0` |
| Rust/Cargo | `1.98.0` |
| System Python | `3.14.7` |
| Project Python/pytest | `.venv\\Scripts\\python.exe`, `3.14.7` / `pytest 9.1.1` |
| Python 前置 | Rust Sidecar integration test 使用 `E:\\Shiraishi\\VSCode Workspace\\Tw2Tg\\.venv\\Scripts\\python.exe` |
| Linux source 状态 | `dev`, `e806959…`, dirty；验证包含未提交修改 |
| Windows 工作副本 | `E:\\Shiraishi\\VSCode Workspace\\Tw2Tg`；无 `.git` 同步 |

#### Linux pre-validation

在进入 Windows 阶段前，对同一 Linux working tree 执行：

| 项目 | 状态 | 结果 |
|---|---|---|
| `cargo fmt --all -- --check` | PASS | 无格式差异 |
| `cargo check --workspace --all-targets` | PASS | workspace check 完成 |
| `cargo test --workspace --no-fail-fast` | PASS | 136 个 crate tests 全部通过，doc-tests 通过；其中 Desktop 51、Storage 23、Sidecar Supervisor 4 |
| Linux Node workspace checks | NOT RUN | 当前 WSL shell 没有 `node` 命令；本轮不把历史 Linux Node 结果冒充为当前环境结果 |

#### Linux → Windows synchronization

| 项目 | 状态 | 命令/证据 |
|---|---|---|
| 受控单向同步 | PASS | `robocopy W:\\home\\shiraishi\\VSCode Workspace\\Tw2Tg E:\\Shiraishi\\VSCode Workspace\\Tw2Tg /E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT /R:1 /W:1`，返回码 `3`；`FAILED=0`、`Mismatch=0` |
| 同步范围 | PASS | 未同步 `.git`、`node_modules`、`.venv`、`target`、build/dist、验证产物、`X-Archive`、数据库/日志/`.env`；目标中已有 `.venv`、依赖、target、验证产物、`X-Archive` 和额外本地文件均保留 |
| 关键源文件一致性 | PASS | `Cargo.toml`、`Cargo.lock`、workspace/package 配置、`job.rs`、Storage `jobs.rs`、Desktop `lib.rs`/`executor.rs`、状态/队列文档 SHA-256 均匹配 |

#### Validation Results

| 类别 | 项目 | 状态 | 实际命令/结果摘要 |
|---|---|---|---|
| Build/Toolchain | Node workspace check | PASS | `npm run check`；Desktop Vite build 和 Extension syntax check 通过 |
| Build/Toolchain | Node workspace test | PASS | `npm run test`；Extension `7/7`，Desktop `0` tests、无失败 |
| Build/Toolchain | Node workspace build | PASS | `npm run build`；Vite production build 通过 |
| Build/Toolchain | Rust formatter | PASS | `cargo fmt --all -- --check` |
| Build/Toolchain | Rust workspace check | PASS | `cargo check --workspace --all-targets` |
| Build/Toolchain | Rust strict clippy | FAIL on validated revision; Linux fix applied, re-validation pending | Windows validation of `e806959` dirty working tree reported `crates/xarchive-storage/src/database/jobs.rs:151` `clippy::type-complexity` for the anonymous event tuple. Linux changed the API to named `JobEventRecord`; Linux fmt/check/test passed, but the current Linux dirty working tree has not yet been revalidated on Windows. |
| Build/Toolchain | Rust workspace tests, default Windows environment | FAIL | 清除 `PYTHON` 后 `cargo test --workspace --no-fail-fast`；其余 crate tests 通过，但 Sidecar Supervisor 的 `communicates_with_a_real_python_worker_when_available` 和 `spawn_ready_completes_the_hello_handshake` 返回 `NotRunning` |
| Build/Toolchain | Rust workspace tests, project Python | PASS | 设置 `$env:PYTHON` 为项目 `.venv\\Scripts\\python.exe` 后 `cargo test --workspace --no-fail-fast`；136/136 crate tests 通过，doc-tests 通过 |
| Build/Toolchain | Python Sidecar tests | PASS | `.venv\\Scripts\\python.exe -m pytest sidecar/tests -q`；`10 passed` |
| Packaging/Build | Tauri Release build | PASS | `npm run build:tauri`；产物 `target\\release\\xarchive-desktop.exe` 生成 |
| Runtime | Tauri Debug startup/cleanup | PASS | `npm run dev:tauri` 启动 Vite、编译并运行 `target\\debug\\xarchive-desktop.exe`；Ctrl+C 后无残留 `xarchive-desktop.exe`，1420 端口无监听残留 |
| Runtime/Regression | R1 executor pure-Rust contract model | PASS for validated revision | 已包含在 Windows 136/136 workspace tests；Desktop executor 51 tests 覆盖 submit/query/cancel/shutdown/recovery/completion/event ordering/SQLite adapter 等当前 contract 范围；不包含 production RuntimeState/Tauri/Sidecar/FileStore integration |
| Packaging | Tauri installer/package | NOT APPLICABLE | `desktop/src-tauri/tauri.conf.json` 中 `bundle.active=false` 且 `createUpdaterArtifacts=false` |
| Runtime/Integration | R1 production executor integration | NOT RUN | `ExecutorRuntime`、`get_app_status` 和最小 Tauri executor control commands 已接入；Linux contract 已覆盖 `EXECUTOR_UNAVAILABLE` compensation 与 shutdown interruption ordering，但尚未接入真实 Sidecar/FileStore worker I/O 或 completion/recovery path；本轮不临时接线以制造验证条件 |
| Filesystem | Desktop 应用级旧 SQLite 迁移、重启恢复、跨用户 ACL | NOT RUN | 当前没有受控旧库副本、Desktop 应用级归档驱动或第二用户/非管理员 ACL 场景；Storage 库级测试通过不能扩大为应用级 Windows PASS |
| Filesystem/Security | symlink/junction/reparse、超长 settings JSON、Unicode 文件专项 | NOT RUN | 当前仓库没有可重复 Windows harness；未用普通文件测试替代 link/reparse 或 UTF-8 边界证据 |
| Integration | 真实 Edge Cookie/X archive、真实 gallery-dl/DownloadRouter fallback、Telegram account | BLOCKED / NOT RUN | 缺少受控浏览器 profile、账号/凭据、真实外部服务或项目级归档入口；不使用个人账号替代 |
| Integration/Packaging | Named Pipe、Native Host Registry/manifest、Tray/Autostart、Credential Manager、externalBin | NOT RUN | 对应 Windows 专属实现或打包前置尚未完成；本轮不扩大为开发任务 |
| Regression | 原生 GUI/WebView2/DPI/键盘/屏幕阅读器/对比度 | BLOCKED | Computer Use helper 初始化返回 `helper_unknown_error: setup refresh had errors`；未把进程启动、静态审查或 Node/Rust tests 当作 GUI 验收 |

#### Errors and classification

1. **Rust strict clippy FAIL — project code, not Windows-specific.** Windows validation of the `e806959` dirty working tree reported `xarchive-storage`'s anonymous event tuple as `clippy::type-complexity`. Linux has now replaced that tuple with the named `JobEventRecord` type and revalidated fmt/check/test; the current revision remains `WINDOWS_VERIFICATION_PENDING` until Windows strict clippy is rerun. The historical FAIL is retained and must not be rewritten as PASS for the new revision.
2. **Default `PYTHON` FAIL — environment prerequisite, not business-code FAIL.** 未设置 `PYTHON` 时两个真实 Python worker handshake tests 返回 `NotRunning`；显式使用项目 `.venv` 后完整 136/136 通过。Linux/CI 应固化解释器前置条件，同时保留默认环境失败与项目环境通过的证据边界。
3. **Linker stdout warnings — non-blocking.** MSVC linker 输出 `正在创建库 ... .lib/.exp` warning；Rust tests、Tauri build 和 Debug startup 均完成，未观察到由该输出导致的构建或运行失败。
4. **GUI helper BLOCKED — test infrastructure/environment.** 当前无法建立原生桌面自动化目标，因此不对 WebView2 实际视觉、DPI、键盘、辅助技术或对比度作结论。

#### Linux Follow-up

- Linux 已处理 `crates/xarchive-storage/src/database/jobs.rs:151` 的 `clippy::type-complexity`：使用命名 `JobEventRecord` 表达 nullable event payload，并通过 Linux fmt/check/test；本地未安装 cargo-clippy，故 Linux clippy 为 `NOT RUN`。Windows strict clippy 对当前 dirty revision 仍需重新执行，WQ-P0-01 保持 `WINDOWS_VERIFICATION_PENDING`。
- 固化 Windows `PYTHON` 指向项目 `.venv\\Scripts\\python.exe` 的运行前置，保留默认环境 FAIL 与正确项目环境 PASS 的区别；本问题不应通过修改业务逻辑解决。
- R1 production executor integration 已完成 `RuntimeState` → `ExecutorRuntime` 的 worker ownership/database context boundary、State-independent `ArchiveExecutionContext` 和 fake `JobExecution` port contract；仍需独立 Linux 开发任务将真实 bundle 接入 worker，完成 Tauri/SidecarSupervisor/FileStore 的真实 worker I/O、completion/recovery path。完成后再执行 WQ-P1-14 的 Windows runtime/recovery 回归。本轮不临时接线。
- 为 WQ-P1-12/WQ-P1-13 准备可重复的 reparse/long-JSON/Unicode harness、旧 SQLite 应用级 fixture 和第二用户/ACL 环境；为真实 Edge/X、aria2 project integration、Native Host、GUI、Credential Manager 和 Telegram 准备受控前置。
- 当前没有因 Windows 专属行为而阻塞后续 Linux 开发；WQ-P0-01、WQ-P1-12、WQ-P1-14 及真实集成/GUI 项目保持 pending/blocked/not-run，不提前改写为全量 Windows PASS。

### Current Linux reconciliation after runner-owned executor wiring (2026-09-14)

本轮 Linux development 已完成不依赖 Windows 的 R1 阶段二主体，未启动新的 Windows validation phase。

| 项目 | Linux 状态 | Windows 状态 |
|---|---|---|
| `archive_job_requests` immutable execution spec | PASS；SQLite migration、request JSON/schema/request_id persistence 已通过 workspace tests | `WINDOWS_VERIFICATION_PENDING`；需验证 Windows 文件 SQLite、重启和旧库升级 |
| `ExecutorConfig` / `ProductionExecutionFactory` | PASS；runner 按 `job_id` 加载 spec，独立创建 Database/FileStore/Sidecar/ArchiveExecutionJob | `WINDOWS_VERIFICATION_PENDING`；需验证 Sidecar executable、资源路径、进程和环境变量 |
| single active runner / bounded control worker | PASS；query/cancel/shutdown 不由长 I/O 直接阻塞 control loop | `WINDOWS_VERIFICATION_PENDING`；需验证 Windows runtime timing、进程清理和并发场景 |
| attempt fencing / late result | PASS；late result 不覆盖新的 attempt 状态 | `WINDOWS_VERIFICATION_PENDING`；需验证应用关闭、文件锁和重启竞态 |
| startup recovery scan / missing spec failure | PASS；缺失 spec 明确写入 `EXECUTION_SPEC_MISSING` | `WINDOWS_VERIFICATION_PENDING`；真实 staging/final/restart facts 归入 WQ-P1-15 |
| running Sidecar cancellation | NOT COMPLETE；当前仅有 interruption/late-result fencing，尚未实现主动 Sidecar interrupt | `NOT RUN`；依赖 Linux cancellation contract 完成后再验证 |
| Extension/Native Host user-entry switch | NOT COMPLETE；保留 `archive_tweet` fallback 和显式 executor command | `NOT RUN`；依赖已验证 Desktop transport adapter |

#### Current BLOCKED / NOT RUN manual steps

1. **WQ-P1-14 executor runtime:** 在最终 Windows artifact 和项目 `.venv` 可用后，启动 Desktop；设置 `XARCHIVE_SIDECAR_PROGRAM`/`XARCHIVE_SIDECAR_ARGS`；提交两个不同 Tweet，查询第一个 Job 时让第二个保持等待；执行 cancel/shutdown；检查 SQLite attempt_count、state/event/error 顺序、runner-owned Sidecar/FileStore、无 RuntimeState 长锁和无残留进程。
2. **WQ-P1-14 restart:** 提交 Job 后在 `DOWNLOADING` 或 staging 阶段关闭应用；重启应用；确认 execution spec 可加载，active/INTERRUPTED Job 不凭空构造 request，晚到结果不会把 `INTERRUPTED` 恢复为 `COMPLETE`。
3. **WQ-P1-15 filesystem recovery:** 准备 `DOWNLOADED + staging`、`DOWNLOADED + final`、`DOWNLOADED + neither`、`COMPLETE + final`、`COMPLETE + missing` fixtures；分别制造文件锁、异常退出、重启和 reparse/junction 条件；记录 recovery decision、SQLite state/event/error 与文件结果。
4. **BLOCKED GUI/IPC/credentials:** 若 WebView2、Named Pipe server、Registry/manifest、第二用户、测试账号或 GUI automation target 缺失，跳过对应项，状态记录为 `BLOCKED`/`NOT RUN`，保留前置缺失原因，不用 Linux contract 或静态检查替代 Windows 结论。

本轮未修改业务代码、依赖或架构；仅将本验证记录追加到 Linux `docs/development/windows-validation.md`，未从 Windows 反向同步代码、依赖、target、日志或用户数据。

### Windows validation of latest Linux commit (R1 executor control and execution contracts, 2026-09-13 23:54–2026-09-14 00:09 +08:00)

本轮按 cross-platform-validation.md、../validation/windows.md、当前 Plan/状态文档和 Windows Validation Queue，对 Linux dev 分支最新提交执行集中 Windows 验证。验证开始时 Linux source 为 clean Git commit；验证结果完成后才向 Linux source 写回本报告和队列状态。

- Linux source: W:/home/shiraishi/VSCode Workspace/Tw2Tg
- branch: dev
- HEAD: 5f18ae0ebd5aeea33fb2da68a8b888c5bc371826
- 验证开始时 working tree: clean (## dev...origin/dev)，不是 dirty working tree
- 变更范围: R1 executor lifecycle/status、executor control commands、独立 SQLite executor context、ArchiveExecutionContext/ArchiveExecutionJob contract，以及 JobEventRecord 命名类型；当前 status/roadmap 明确 real Sidecar/FileStore worker I/O 和 completion/recovery wiring 仍未完成
- Windows canonical validation workspace: E:/Shiraishi/VSCode Workspace/Tw2Tg
- E:/Shiraishi/VSCode Workspace/Tw2Tg-CodexAlias 仍是 junction，但本轮没有使用 alias 入口
- 本轮未修改业务代码、依赖、架构或 Windows 本地配置；只写回 Linux 验证文档

#### Validation Environment

| 项目 | 实际值 |
|---|---|
| Windows | Microsoft Windows NT 10.0.29667.0, x64 |
| Node/npm | v24.19.0 / 11.17.0 |
| Rust/Cargo | 1.98.0 |
| System Python | 3.14.7 |
| Project Python/pytest | E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe, 3.14.7 / pytest 9.1.1 |
| Tauri CLI | local 2.11.4 |
| Linux source | dev, 5f18ae0…, 验证开始时 clean |
| Windows 工作副本 | E:/Shiraishi/VSCode Workspace/Tw2Tg；无 .git |

#### Scope determination

| 类别 | 本轮范围 | 依据 |
|---|---|---|
| Required | Rust fmt/check/test/clippy、Node check/test/build、Sidecar pytest、Tauri build/start/cleanup | docs/development/testing.md、setup.md、windows-queue.md 的 WQ-P0-01 |
| Applicable | R1 executor contract regression、Sidecar subprocess handshake、Tauri Windows process startup/cleanup、当前变更模块的 Windows compile/build | 当前 HEAD 的变更模块和 status/roadmap |
| Not applicable | Tauri installer/package/updater | desktop/src-tauri/tauri.conf.json 为 bundle.active=false、createUpdaterArtifacts=false |
| Deferred prerequisite | 真实 Edge/X、aria2/DownloadRouter、Telegram、Native Host/Named Pipe、应用级旧库迁移/ACL、reparse harness、GUI accessibility、R1 real worker integration | 当前队列前置或实现/artifact 尚未具备；本轮不扩大为开发任务 |

#### Linux pre-validation

Windows 阶段前，在同一 Linux source 执行：

| 项目 | 状态 | 结果 |
|---|---|---|
| cargo fmt --all -- --check | PASS | 无格式差异 |
| cargo check --workspace --all-targets | PASS | workspace check 完成 |
| cargo test --workspace --no-fail-fast | PASS | 142/142 crate tests 通过，doc-tests 通过；Desktop 57、Sidecar Supervisor 4 |
| python3 -m compileall -q sidecar/src sidecar/tests | PASS | 完成 |
| Linux cargo clippy --workspace --all-targets -- -D warnings | NOT RUN | Linux stable toolchain 未安装 cargo-clippy |
| Linux python3 -m pytest sidecar/tests -q | NOT RUN | Linux /usr/bin/python3 未安装 pytest |
| Linux Node checks | NOT RUN | 当前 WSL shell 没有 node 命令 |

#### Linux → Windows synchronization

| 项目 | 状态 | 命令/证据 |
|---|---|---|
| 目标目录安全检查 | PASS | canonical E:/Shiraishi/VSCode Workspace/Tw2Tg 已存在；未删除未知文件；确认 .venv、node_modules、validation-artifacts、X-Archive 及额外本地文件需保留 |
| 受控单向同步 | PASS | robocopy W:/home/shiraishi/VSCode Workspace/Tw2Tg E:/Shiraishi/VSCode Workspace/Tw2Tg /E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT /R:1 /W:1 /XD .git node_modules .venv target build dist validation-artifacts X-Archive /XF .env .env.* *.db *.sqlite *.sqlite3 *.log；返回码 3，FAILED=0、Mismatch=0，146 total / 145 copied / 1 skipped / 6 extras |
| 排除与本地文件保护 | PASS | 未同步 .git、依赖、.venv、Rust target、build/dist、验证产物、X-Archive、数据库/日志/.env；目标 extras 未删除或覆盖 |
| 关键源文件一致性 | PASS | 17 个关键源文件/配置/文档 SHA-256 与 Linux source 匹配，KEY_HASH_MISMATCHES=0；source 仍保持 clean |
| 备注 | — | 本次命令未额外排除 __pycache__/*.pyc，因此有少量 Python bytecode 随源树复制；它们未作为验证证据，也未反向同步到 Linux |

#### Validation Results

| 类别 | 项目 | 状态 | 实际命令/结果摘要 |
|---|---|---|---|
| Build/Toolchain | Node workspace check | PASS | npm run check；Vite 与 Extension syntax check 通过 |
| Build/Toolchain | Node workspace test | PASS | npm run test；Extension 7/7，Desktop 0 tests，无失败 |
| Build/Toolchain | Node workspace build | PASS | npm run build；Vite production build 通过 |
| Build/Toolchain | Rust formatter | PASS | cargo fmt --all -- --check |
| Build/Toolchain | Rust workspace check | PASS | cargo check --workspace --all-targets |
| Build/Toolchain | Rust strict clippy | PASS | cargo clippy --workspace --all-targets -- -D warnings；当前 JobEventRecord 修复后的提交无 clippy 失败 |
| Build/Toolchain | Rust workspace tests，默认 Windows environment | FAIL | 清除 PYTHON 后执行 cargo test --workspace --no-fail-fast；除 Sidecar 外其余测试通过，spawn_ready_completes_the_hello_handshake 与 communicates_with_a_real_python_worker_when_available 返回 NotRunning |
| Build/Toolchain | Rust workspace tests，项目 Python | PASS | 设置 PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe 后执行同一命令；142/142 crate tests 通过，doc-tests 通过 |
| Build/Toolchain | Python Sidecar tests | PASS | .venv/Scripts/python.exe -m pytest sidecar/tests -q；10 passed |
| Packaging/Build | Tauri Release build | PASS | npm run build:tauri；生成 target/release/xarchive-desktop.exe；release profile 完成 |
| Runtime | Tauri Debug startup/cleanup | PASS | npm run dev:tauri 启动 Vite http://localhost:1420/ 并运行 target/debug/xarchive-desktop.exe；Ctrl+C 后 xarchive-desktop 进程数为 0、1420 监听数为 0 |
| Regression | R1 executor contract model | PASS | 已包含在 Windows 142/142 workspace tests；Desktop 57 tests 覆盖 submit/query/cancel/shutdown、SQLite adapter、completion/failure、terminal skip 和 event ordering；不扩大为真实 worker acceptance |
| Packaging | Tauri installer/package/updater | NOT APPLICABLE | bundle.active=false 且 createUpdaterArtifacts=false |
| Runtime/Integration | R1 production executor integration | NOT RUN | 只完成了 ExecutorRuntime/control-plane/ArchiveExecutionContext contract 的静态范围核对；真实 Sidecar/FileStore worker I/O、completion/recovery 和 fallback 切换尚未接入，本轮不临时接线 |
| Filesystem | Desktop 应用级旧 SQLite migration/restart/recovery | NOT RUN | 没有受控旧库副本、Desktop 应用级归档驱动和可制造异常退出的 fixture；Storage 库级测试不能替代应用级结论 |
| Filesystem/Security | symlink/junction/reparse、长 JSON、Unicode/长路径专项 | NOT RUN | 当前没有可重复 Windows harness；未用普通文件测试替代这些边界 |
| Privacy/Filesystem | 跨用户 ACL 和 user-data boundary | NOT RUN | 没有第二用户、非管理员 ACL fixture 或受控共享目录场景 |
| Integration | 真实 Edge/X Cookie archive | BLOCKED | 缺少专用 Edge profile、测试账号/凭据、真实网络和受控 archive fixture |
| Integration | 真实 gallery-dl/DownloadRouter/aria2 fallback | BLOCKED | 缺少受控 aria2c.exe、media server、真实媒体 URL 和项目级归档入口 |
| Integration | Telegram account flow/Credential Manager | BLOCKED | 缺少测试 Bot/chat、真实凭据 backend 和外部服务授权 |
| Integration | Native Host/Named Pipe/Registry/manifest | NOT RUN | 当前没有最终 Named Pipe server、manifest/Registry artifact 或安装入口 |
| Runtime/Packaging | bundled Sidecar/externalBin、Tray、Autostart、Single Instance | NOT RUN | 当前没有可验证的 bundle artifact；Debug 启动不等价于打包后进程生命周期验收 |
| Regression | 原生 GUI/WebView2/DPI/键盘/屏幕阅读器/对比度 | BLOCKED | Computer Use helper 初始化返回 helper_unknown_error: setup refresh had errors；没有可用 GUI automation target |

#### Errors and classification

1. 默认 PYTHON 测试 FAIL：环境前置，不是当前业务代码 FAIL。未设置 PYTHON 时两个真实 Python worker handshake 测试返回 NotRunning；显式使用项目 .venv 后完整 142/142 通过。setup.md 已规定 Windows 使用项目虚拟环境解释器，后续应固化测试入口/CI 前置并保留两种结果的证据边界。
2. MSVC linker stdout warning：非阻塞。Tauri Release/Debug 和 Rust tests 输出 正在创建库 ... .lib/.exp，但编译、运行和清理均完成，未观察到由该输出导致的失败。
3. Debug 受控退出提示：非业务失败。Ctrl+C 退出时出现 Chrome_WidgetWin_0 ... Error = 1411 和 STATUS_CONTROL_C_EXIT；随后进程和 1420 端口均为零，符合受控终止后的清理检查。
4. GUI BLOCKED：验证基础设施/环境问题。helper 初始化失败，故不对 WebView2 的实际视觉、DPI、键盘、屏幕阅读器、对比度或命中区域作结论。
5. 当前提交未发现新的项目代码 Windows 编译/测试失败。历史 xarchive-storage clippy::type-complexity 已由 Linux 的命名 JobEventRecord 修复，并在本轮 Windows strict clippy 通过；该历史问题不应继续记为当前提交 FAIL。

#### Queue reconciliation

- WQ-P0-01 已更新为 WINDOWS_PASS：所有文档定义且当前前置满足的 Windows baseline 在项目 .venv 下通过；默认未设置 PYTHON 的负向诊断单独保留为环境 FAIL。
- WQ-P0-02、WQ-P0-03、WQ-P0-04、WQ-P1-01、WQ-P1-02、WQ-P1-03、WQ-P1-04、WQ-P1-05、WQ-P1-12、WQ-P1-13、WQ-P1-14、WQ-P1-15、WQ-P2-01、WQ-P2-02 仍保持 WINDOWS_VERIFICATION_PENDING；本轮的具体 prerequisite 缺失分别记录在上表。
- 当前没有 WINDOWS_VERIFICATION_BLOCKING 项目；这些未完成项不阻塞后续 Linux 开发，但在相应功能或验证前置具备后必须重新执行。

#### Linux Follow-up

- 固化 Windows Rust/Sidecar 测试入口对项目 .venv/Scripts/python.exe 的选择，或在测试/CI 文档中明确自动设置 PYTHON；不要通过修改业务逻辑掩盖 NotRunning。
- 继续独立 Linux 开发任务：将真实 Sidecar/FileStore I/O 接入 ExecutorRuntime/ArchiveExecutionJob，完成 completion/recovery、worker ownership 和 synchronous fallback 切换；完成后再执行 WQ-P1-14 的 Windows runtime/recovery 回归。
- 为 WQ-P0-03/WQ-P1-12/WQ-P1-13/WQ-P1-15 准备可重复的 Windows 旧 SQLite、异常退出、reparse、长 JSON/Unicode、ACL 和跨用户 fixtures/harness。
- 在不使用个人账号的前提下准备专用 Edge/X、aria2/media server、Telegram 测试环境，并在 Native Host/packaging artifact 形成后执行对应集成验证。
- 恢复可用的 Windows GUI automation target 后，补做 WebView2/DPI/键盘/屏幕阅读器/对比度验证。
- 本轮没有因 Windows 专属行为阻塞后续 Linux 实现；验证范围之外的问题不在本任务中修复。

本轮实际验证结论：当前 5f18ae0 Linux commit 的 Windows baseline 在正确项目 Python 前置下通过；默认 Python 环境有可复现的 NotRunning 前置失败，GUI 和真实外部/应用级集成仍未完成，不能将本轮结论扩大为全量 Windows acceptance。

### Windows validation of latest Linux dirty working tree (R1 stage-two executor and SQLite migration, 2026-09-14 15:00–15:20 +08:00)

本轮按 [`cross-platform-validation.md`](cross-platform-validation.md)、[`../validation/windows.md`](../validation/windows.md)、当前 Plan/状态文档和 Windows Validation Queue，对 Linux `dev` 分支最新 dirty working tree 执行集中 Windows 验证。当前 Plan/状态以 `aidlc-docs/aidlc-state.md`、`docs/development/status.md` 和 `docs/development/roadmap.md` 为准：R1 executor 阶段二 Linux 主体已完成，最终用户入口切换及若干真实 Windows/外部集成仍 deferred。验证对象不是纯 Git commit：

- branch: `dev`
- HEAD: `5f18ae0ebd5aeea33fb2da68a8b888c5bc371826`
- working tree: dirty，22 个已跟踪修改和未跟踪 `crates/xarchive-storage/migrations/0004_archive_job_requests.sql`；验证包含这些未提交修改
- Linux source: `W:\home\shiraishi\VSCode Workspace\Tw2Tg`
- Windows canonical validation workspace: `E:\Shiraishi\VSCode Workspace\Tw2Tg`
- `E:\Shiraishi\VSCode Workspace\Tw2Tg-CodexAlias` 是 canonical workspace 的 junction；本轮构建和测试使用 canonical 路径
- 本轮未修改业务代码、依赖、架构或 Windows 本地配置；Windows 仅产生 `target`、`dist`、Python bytecode 等工作副本产物

#### Validation Environment

| 项目 | 实际值 |
|---|---|
| Windows | `Microsoft Windows NT 10.0.29667.0`, x64 |
| Node/npm | `v24.19.0` / `11.17.0` |
| Rust/Cargo | `rustc 1.98.0 (88d9e12ae 2026-08-18)` / `cargo 1.98.0 (797e8a9bc 2026-08-05)` |
| System Python | `3.14.7` |
| Project Python/pytest | `.venv\Scripts\python.exe`, `3.14.7` / `pytest 9.1.1` |
| Tauri CLI | `2.11.4`; application Tauri runtime `2.11.5`; MCP Bridge `0.13.0` |
| Linux source | `dev`, `5f18ae0…`, dirty；验证包含未提交修改 |
| Windows 工作副本 | `E:\Shiraishi\VSCode Workspace\Tw2Tg`；无 `.git` 同步 |

#### Linux pre-validation

Windows 阶段前，在同一 Linux dirty working tree 执行项目规定的适用 gate：

| 项目 | 状态 | 结果 |
|---|---|---|
| `cargo fmt --all -- --check` | PASS | 无格式差异 |
| `cargo check --workspace --all-targets` | PASS | workspace check 完成 |
| `cargo clippy --workspace --all-targets -- -D warnings` | PASS | 当前 WSL stable clippy 可用并通过 |
| `cargo test --workspace --no-fail-fast` | PASS | 144 个 crate tests 通过，doc-tests 通过；Desktop 58、Storage 24 |
| `.venv/bin/python -m compileall -q sidecar/src sidecar/tests` | PASS | 完成 |
| `.venv/bin/python -m pytest sidecar/tests -q` | PASS | 10 passed |
| Linux Node workspace checks | NOT RUN | 当前 WSL shell 没有 `node` 命令 |

#### Linux → Windows synchronization

| 项目 | 状态 | 命令/证据 |
|---|---|---|
| 目标目录安全检查 | PASS | canonical E: 目录已存在；保留 `.venv`、`node_modules`、`target`、`validation-artifacts`、`X-Archive`；目标额外的 `desktop/src/main.js`、`windows-schema.json`、`archive_application.rs` 未删除 |
| 受控单向同步 | PASS | `robocopy W:\home\shiraishi\VSCode Workspace\Tw2Tg E:\Shiraishi\VSCode Workspace\Tw2Tg /E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT /R:1 /W:1 /XD .git node_modules .venv target build dist validation-artifacts X-Archive __pycache__ .pytest_cache /XF .env .env.* *.db *.sqlite *.sqlite3 *.log *.pyc`；返回码 `3`，目标未报告 failed/mismatch |
| 排除与本地文件保护 | PASS | 未同步 `.git`、依赖、`.venv`、Rust `target`、build/dist、验证产物、`X-Archive`、数据库、日志、`.env`；目标 extras 未删除或覆盖 |
| 同步后 dry-run | PASS | 正常变更模式 `Mismatch=0`、`FAILED=0`；仅发现目标 Tauri 生成的 `windows-schema.json` 和历史额外文件，以及 source 较旧的 generated `desktop-schema.json`，均未反向同步 |
| 关键源文件一致性 | PASS | `Cargo.toml`、`Cargo.lock`、workspace/package 配置、`0004_archive_job_requests.sql`、Storage `jobs.rs/lib.rs`、Desktop `executor.rs/lib.rs`、状态/队列/验证文档 SHA-256 共 13/13 匹配 |

#### Scope determination

| 类别 | 本轮范围 | 依据 |
|---|---|---|
| Required | Rust fmt/check/test/clippy、Node check/test/build、Sidecar compileall/pytest、Tauri build/start/cleanup | `docs/development/testing.md`、`setup.md`、`windows-queue.md` 的 WQ-P0-01 |
| Applicable | R1 executor contract regression、SQLite migration compile/test、Sidecar subprocess handshake、Tauri Windows startup/cleanup、Tauri MCP runtime probe | 当前 HEAD 的变更模块、status/roadmap 和可用 Windows 前置 |
| Not applicable | Tauri installer/package/updater | `desktop/src-tauri/tauri.conf.json` 的 `bundle.active=false`、`createUpdaterArtifacts=false` |
| Deferred prerequisite | 真实 Edge/X、aria2/DownloadRouter 业务 fallback、Telegram/Credential Manager、Named Pipe/Registry、应用级旧库重启/ACL/reparse harness、bundled externalBin、最终 executor real-worker acceptance | 当前队列前置、artifact 或应用入口尚未形成；本轮不扩大为开发任务 |

#### Validation Results

| 类别 | 项目 | 状态 | 实际命令/结果摘要 |
|---|---|---|---|
| Build/Toolchain | Node workspace check | PASS | `npm run check`；Desktop Vite build 和 Extension syntax check 通过 |
| Build/Toolchain | Node workspace test | PASS | `npm run test`；Extension `7/7`，Desktop `0` tests、无失败 |
| Build/Toolchain | Node workspace build | PASS | `npm run build`；Vite production build 通过 |
| Build/Toolchain | Rust formatter | PASS | `cargo fmt --all -- --check` |
| Build/Toolchain | Rust workspace check | PASS | `cargo check --workspace --all-targets` |
| Build/Toolchain | Rust strict clippy | PASS | `cargo clippy --workspace --all-targets -- -D warnings` |
| Build/Toolchain | Rust workspace tests，默认 Windows environment | FAIL | 清除 `PYTHON` 后 `cargo test --workspace --no-fail-fast`；除 Sidecar 外通过，`spawn_ready_completes_the_hello_handshake` 和 `communicates_with_a_real_python_worker_when_available` 返回 `NotRunning` |
| Build/Toolchain | Rust workspace tests，项目 Python | PASS | 设置 `PYTHON=E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe` 后同一命令；144/144 crate tests 通过，doc-tests 通过 |
| Build/Toolchain | Python compileall | PASS | `.venv\Scripts\python.exe -m compileall -q sidecar/src sidecar/tests` |
| Build/Toolchain | Python Sidecar tests | PASS | `.venv\Scripts\python.exe -m pytest sidecar/tests -q`；10 passed |
| Packaging/Build | Tauri Release build | PASS | `npm run build:tauri`；Vite build 完成并生成 `target\release\xarchive-desktop.exe`，约 17.1 MB |
| Runtime | Tauri Debug startup/cleanup | PASS | `npm run dev:tauri` 启动 Vite `http://localhost:1420/`、`xarchive-desktop.exe` 和 MCP bridge `127.0.0.1:9223`；Tauri MCP stop 后 Ctrl+C 清理，应用/WebView/Vite/MCP 进程及 1420/9223 listeners 均为 0 |
| Runtime/Regression | Tauri MCP backend/window smoke | PASS | driver session 成功连接；backend state 报告 Windows/debug、`com.tw2tg.xarchive`、窗口 `main` 可见且响应；窗口 resize `720x540` logical 和 focus 调用成功 |
| Regression | R1 executor/Storage contract model | PASS | 已包含在 Windows 144/144 workspace tests；Desktop 58 tests 覆盖 executor lifecycle/control、SQLite adapter、recovery decision/action、completion/failure、cancellation fencing 和 event ordering；不扩大为真实应用级 acceptance |
| Packaging | Tauri installer/package/updater | NOT APPLICABLE | `bundle.active=false` 且 `createUpdaterArtifacts=false` |
| Runtime/Integration | R1 production executor real worker integration | NOT RUN | 当前没有可重复的应用级 submit/query/cancel/restart fixture；Tauri MCP 的应用 IPC invoke 受 WebView timeout，且本轮不临时接线或修改入口来制造验证条件 |
| Filesystem | Desktop 应用级旧 SQLite migration/restart/recovery | NOT RUN | 没有受控旧库副本、应用级归档驱动和异常退出/文件锁 fixture；Storage 库级 migration/reopen tests 不能替代应用级 Windows 结论 |
| Filesystem/Security | symlink/junction/reparse、长 JSON、Unicode/长路径专项 | NOT RUN | 当前没有可重复 Windows harness；未用普通文件测试替代 link/reparse 或边界证据 |
| Privacy/Filesystem | 跨用户 ACL 和 user-data boundary | NOT RUN | 没有第二用户、非管理员 ACL fixture 或受控共享目录 |
| Integration | 真实 Edge/X Cookie archive | BLOCKED | 缺少专用 Edge profile、测试账号/凭据、真实网络和受控 archive fixture |
| Integration | 真实 gallery-dl/DownloadRouter/aria2 fallback | BLOCKED | 缺少受控 `aria2c.exe`、media server、真实媒体 URL 和项目级归档入口 |
| Integration | Telegram account flow/Credential Manager | BLOCKED | 缺少测试 Bot/chat、真实凭据 backend 和外部服务授权 |
| Integration | Native Host/Named Pipe/Registry/manifest | NOT RUN | 当前没有最终 Named Pipe server、manifest/Registry artifact 或安装入口 |
| Runtime/Packaging | bundled Sidecar/externalBin、Tray、Autostart、Single Instance | NOT RUN | 当前没有可验证 bundle artifact；Debug 启动不等价于打包后生命周期验收 |
| Regression | 原生 GUI/WebView2/DPI/键盘/屏幕阅读器/对比度 | BLOCKED | Tauri MCP bridge 可连接，但 DOM accessibility/structure、截图、console logs 和 `get_app_status` IPC 均返回 `WebView execution failed: Request timeout after 2000ms`；`allowScreenCapture=true` 也失败 |

#### Errors and classification

1. **默认 `PYTHON` 测试 FAIL — environment prerequisite，不是业务代码 FAIL。** 未设置 `PYTHON` 时两个真实 Python worker handshake 测试返回 `NotRunning`；显式使用项目 `.venv` 后完整 144/144 通过。后续应固化 Windows 测试入口对项目解释器的选择，不通过修改业务逻辑掩盖该问题。
2. **MSVC linker stdout warning — non-blocking。** Rust tests/Tauri build 输出 `正在创建库 ... .lib/.exp`，但编译、测试、启动和清理均完成，未观察到由该输出导致的失败。
3. **Tauri release warning — non-blocking follow-up。** Release build 输出 `desktop/src-tauri/src/lib.rs:31` 的 `unused_mut` warning；strict clippy 通过，产物已生成。本任务不为消除 warning 修改业务代码，建议后续 Linux hygiene task 按 debug/release cfg 检查。
4. **Debug Ctrl+C exit — expected controlled termination。** `npm run dev:tauri` 的 wrapper 在 Ctrl+C 后报告 `STATUS_CONTROL_C_EXIT`/exit code `1`，但随后 `xarchive-desktop`、Vite、WebView、1420 和 9223 均为 0；因此 startup/cleanup 项目仍为 PASS，不把受控终止提示记为业务 FAIL。
5. **GUI WebView BLOCKED — test infrastructure/environment。** Tauri MCP driver/backend/window management 可用，但 WebView eval 层统一 2 秒 timeout；无法对真实 DOM、视觉、DPI、键盘、辅助技术、对比度或应用 IPC 作结论。

#### Queue reconciliation

- WQ-P0-01 继续为 `WINDOWS_PASS`：当前 `5f18ae0` + dirty working tree 的文档定义 baseline 在项目 `.venv` Python 前置下通过；默认无 `PYTHON` 的两项 `NotRunning` 单独保留为 environment FAIL。
- WQ-P0-02、WQ-P0-03、WQ-P0-04、WQ-P1-01、WQ-P1-02、WQ-P1-03、WQ-P1-04、WQ-P1-05、WQ-P1-12、WQ-P1-13、WQ-P1-14、WQ-P1-15、WQ-P2-01、WQ-P2-02 继续为 `WINDOWS_VERIFICATION_PENDING`；本轮没有足够的真实外部、应用级、IPC、ACL、reparse 或 packaging 前置关闭这些项目。
- 当前没有 `WINDOWS_VERIFICATION_BLOCKING` 项目；本轮 NOT RUN/BLOCKED 项目不阻塞后续 Linux 开发，但相应 artifact/fixture/账号/automation target 具备后必须重新执行。

#### Linux Follow-up

- 固化 Windows Rust/Sidecar 测试入口使用项目 `.venv\Scripts\python.exe`，保留默认环境 FAIL 与正确项目环境 PASS 的证据边界；不要通过业务逻辑修改掩盖 `NotRunning`。
- 继续独立 Linux 开发任务：完成 R1 executor 的最终用户入口切换及真实 Sidecar/FileStore worker I/O、completion/recovery 与 cancellation 接入；完成后再执行 WQ-P1-14 的 Windows runtime/recovery 回归。本轮没有临时接线。
- 为 WQ-P0-03/WQ-P1-12/WQ-P1-13/WQ-P1-15 准备可重复的 Windows 旧 SQLite、异常退出/文件锁、reparse、长 JSON/Unicode、ACL 和跨用户 fixtures/harness。
- 在不使用个人账号的前提下准备专用 Edge/X、aria2/media server、Telegram 测试环境，并在 Native Host/packaging artifact 形成后执行对应集成验证。
- 恢复可用的 WebView GUI automation target 后，补做 DOM/accessibility、WebView2/DPI、键盘、屏幕阅读器和对比度验证；也应补执行真实 Tauri IPC application-command probe。
- 处理 `desktop/src-tauri/src/lib.rs:31` 的 release-only `unused_mut` warning（独立 Linux hygiene task）；不把本轮 warning 当作 Windows 业务失败。
- 本轮相对验证开始状态仅新增本验证记录；未向 Linux 反向同步代码、依赖、target、dist、日志或用户数据。

本轮实际验证结论：当前 `5f18ae0` Linux commit 加未提交的 R1 executor/SQLite migration working tree 在 canonical E: 工作副本上完成 Windows baseline 验证；项目 Python 前置下所有适用静态、构建和自动化测试通过，默认 Python 环境保留两个可复现的 `NotRunning` FAIL，Tauri MCP WebView 层 BLOCKED，真实应用级/外部服务/Windows 专属集成仍为 NOT RUN 或 BLOCKED，不能扩大为全量 Windows acceptance。

### Windows aria2 半永久化目录更新后的复验（2026-09-14 15:52–15:53 +08:00）

用户确认当前 Windows aria2 半永久化目录为 E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2。本次只在该目录下创建隔离测试目录 runtime-test-2026-09-14-aria2-revalidation 并重新执行 aria2 独立工件/RPC/恢复子集；没有修改 Linux 业务代码、没有把目录加入 PATH、没有安装系统服务，也没有反向同步 Windows 生成的工件、日志或用户数据。此前 Tw2Tg-Windows-DevArtifacts\aria2 的记录保留为历史证据，不再作为当前目录。

#### Validation Environment

- Linux source：branch dev，HEAD 5f18ae0ebd5aeea33fb2da68a8b888c5bc371826，working tree dirty，验证对象包含当前未提交修改。
- Windows workspace：E:\Shiraishi\VSCode Workspace\Tw2Tg；项目 Python：E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe。
- aria2：Windows x64 aria2c.exe 1.37.0；ZIP SHA-256 67D015301EEF0B612191212D564C5BB0A14B5B9C4796B76454276A4D28D9B288；EXE SHA-256 BE2099C214F63A3CB4954B09A0BECD6E2E34660B886D4C898D260FEBFE9D70C2。
- Local fixture：16 MiB，SHA-256 080ACF35A507AC9849CFCBA47DC2AD83E01B75663A516279C8B9D243B719643E；Range server 仅监听 loopback，端口 19102，JSON-RPC 端口 19103/19104。
- Test artifact/log root：E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2\runtime-test-2026-09-14-aria2-revalidation。

#### Validation Results

| 验证项目 | 状态 | 实际命令/结果摘要 |
|---|---|---|
| aria2 artifact integrity/version | PASS | 对官方 ZIP、EXE 和 16 MiB fixture 执行 Get-FileHash -Algorithm SHA256；aria2c.exe --version 为 1.37.0，hash 与 allowlist/fixture 预期一致 |
| Local Range server | PASS | 使用 E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2\range_server.py 和项目 .venv\Scripts\python.exe，loopback 19102 启动成功 |
| Loopback JSON-RPC | PASS | aria2.getVersion、addUri、tellStatus、pause、unpause、changeOption 请求成功 |
| Unicode/空格路径下载 | PASS | 输出到 暂停 Unicode 空格\暂停 文件.bin，16 MiB 文件 hash 与 fixture 一致 |
| Pause/resume | PASS | 观察到 active → paused，暂停期间存在 .aria2 控制文件；恢复完成后 hash 一致且 .aria2 消失 |
| Forced process interruption/resume | PASS | 中断前已有下载进度并存在 .aria2；强制停止 aria2c 后重新启动并提交同一目标，续传完成且 hash 一致 |
| .aria2 cleanup/process cleanup | PASS | 两个输出目录最终无 .aria2；aria2c/Range server 无残留进程；19103/19104 无活动监听，19102 仅保留正常 TCP TIME_WAIT |
| Test wrapper exit status | PASS | 本次干净隔离复验外层命令返回 0，输出 WRAPPER=PASS |
| Project DownloadRouter / Desktop Job real aria2 fallback | NOT RUN | 本次仍是独立 aria2 artifact/RPC/恢复子集；未把真实 aria2c.exe 接入当前 Desktop Job，也未执行 gallery-dl 403 refresh 和 transfer lifecycle，因此 WQ-P1-01 继续 pending |

#### Errors and classification

1. aria2 在 loopback 测试启动时记录未设置 --rpc-secret 的安全警告。本次 RPC 只绑定本机回环接口，属于测试配置提示，不是项目业务代码失败；正式集成验证仍应使用受控 secret/权限配置。
2. 强制中断场景的 Range server 日志包含 ConnectionResetError: [WinError 10054]，原因是客户端进程被主动终止后连接重置；最终续传、hash 和 .aria2 清理均通过，归类为预期 fixture 噪声，不记为 FAIL。

#### Linux Follow-up

- 将当前 aria2 半永久化验证目录固定记录为 E:\Shiraishi\VSCode Workspace\Tw2Tg\aria2；不把该机器路径写入业务运行时配置或 PATH。
- WQ-P1-01 仍需 Linux 后续开发/集成任务提供项目级 DownloadRouter、Desktop Job、gallery-dl 403 refresh 和 transfer lifecycle 的可重复入口；本次独立 aria2 PASS 不能关闭该队列项。
- 当前没有由本次 aria2 复验发现的 Linux 业务代码 FAIL；本次只新增验证文档记录。

### Linux reconciliation of Windows results（2026-09-14）

按 `docs/development/cross-platform-validation.md` 重读 2026-09-14 Windows 轮记录并核对当前工作树后，结论如下：

- **验证对象一致。** Windows 轮验证对象为 `5f18ae0` + 未提交 working tree，该 working tree 已包含 MCP Bridge `0.13.0`、R1 executor recovery/late-result/cancellation 和 SQLite migration；Windows 环境表亦记录 `MCP Bridge 0.13.0`，与当前 Linux 工作树一致。Linux 预检与 Windows 结果一致（workspace `144/144` tests、Desktop `58`）。
- **保留为 PASS 的证据边界。** `WQ-P0-01` 保持 `WINDOWS_PASS`；默认无 `PYTHON` 的两项 `NotRunning` 保留为 environment FAIL，会在不设置 `PYTHON` 的任何平台复现，故不关闭。
- **关闭无关 Linux hygiene item。** Windows 轮要求后续 Linux 处理 `desktop/src-tauri/src/lib.rs:31` 的 release-only `unused_mut` warning。已在本轮 Linux 用 `#[cfg(debug_assertions)]` shadowing 注册 MCP Bridge 消除该 warning；`cargo check --release -p xarchive-desktop` 无 warning，`cargo check --workspace`、`cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings` 通过。该修复属于 warning-only/behavior-invariant，不影响 Windows 已验证的 Debug MCP runtime 行为，但最终 commit 对应的 Windows 复验仍安排在后续 Windows 轮。
- **不提前标记 PASS。** `WQ-P0-02/03/04`、`WQ-P1-01/02/03/04/05/12/13/14/15`、`WQ-P2-01/02` 保持 `WINDOWS_VERIFICATION_PENDING`、`NOT RUN` 或 `BLOCKED`：Tauri MCP 的 WebView eval 层 2 秒 timeout 意味着 DOM、真实 WebView2 渲染/DPI/键盘/辅助技术、截图和应用 IPC probe 仍未获得 Windows 证据；aria2 独立 PASS 不关闭 `WQ-P1-01` 的项目级业务链路；应用级 executor real-worker、应用级旧库重启/恢复、ACL/reparse/长路径和 bundle/packaging 仍缺 fixture/artifact。
- **Linux 当前不新增开发项。** 阶段三用户入口切换保持暂缓，等待已验证的 Desktop transport adapter；其余 Windows 前置齐全后再统一执行集中式 handoff。

### Linux → Windows WebdriverIO native smoke 验证（2026-09-14 23:35–23:38 +08:00）

本轮按 docs/development/cross-platform-validation.md 执行：先检查 Linux 源，再将 Linux 源单向同步到 canonical E: Windows 验证副本；只把本验证记录写回 Linux，未反向同步 Windows 依赖、target、日志、缓存或用户数据。

#### Validation Environment

- Linux source：/home/shiraishi/VSCode Workspace/Tw2Tg（Windows 映射为 W:\home\shiraishi\VSCode Workspace\Tw2Tg），branch dev，HEAD 15ab756d74aabebb86229cea59076af54c8e5ab7，working tree dirty；验证对象包含当前未提交的 R1 executor/transport、文档和本轮 WDIO 配置变更。
- Windows workspace：E:\Shiraishi\VSCode Workspace\Tw2Tg；canonical E: 目录，不是 Git source of truth。
- Windows：Microsoft Windows NT 10.0.29667.0，AMD64；Node v24.19.0；npm 11.17.0；WDIO CLI 9.31.9；WebView2 152.0.4191.66。
- WDIO packages：@wdio/cli 9.31.9、@wdio/local-runner 9.31.9、@wdio/mocha-framework 9.31.9、@wdio/spec-reporter 9.31.2、@wdio/tauri-service 1.4.0。
- tauri-driver：由 service 自动通过 cargo 安装到 C:\Users\Shiraishi\.cargo\bin\tauri-driver.exe；Windows external provider 使用端口 4444。
- Tauri artifact：E:\Shiraishi\VSCode Workspace\Tw2Tg\target\release\xarchive-desktop.exe，17,054,208 bytes，生成时间 2026-09-14 23:22:15。
- tauri-plugin-wdio：当前未注册；本轮只验证 WebDriver DOM smoke，不宣称 browser.tauri、mock 或 frontend/backend log capture 可用。

#### Linux pre-validation

| 项目 | 状态 | 实际命令/结果 |
|---|---|---|
| Rust formatter/workspace tests | PASS | cargo fmt --all -- --check；cargo test --workspace --no-fail-fast；Desktop 64 tests，workspace 其他 crate tests 和 doc-tests 全部通过 |
| Node test | PASS | npm run test；Extension 7/7，Desktop 0 tests |
| WDIO source syntax | PASS | node --check desktop/wdio.conf.mjs；node --check desktop/e2e/specs/dashboard.e2e.mjs |
| Linux Node check/build | NOT RUN | WSL 源目录未安装 frontend dependencies，npm run check/build 报 vite 不是可识别命令；对应 Windows check/build 已执行并通过 |

#### Sync verification

| 项目 | 状态 | 实际命令/结果 |
|---|---|---|
| Windows target safety check | PASS | E: 目标存在；.venv、node_modules、target、validation-artifacts、X-Archive 和目标 extra files 保留 |
| Controlled one-way sync | PASS | robocopy W:\home\shiraishi\VSCode Workspace\Tw2Tg E:\Shiraishi\VSCode Workspace\Tw2Tg /E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT /R:1 /W:1，排除 .git、依赖、虚拟环境、target、build/dist、验证产物、用户数据、数据库、日志和 .env；ROBOCOPY_EXIT=3，FAILED=0，Mismatch=0 |
| Key WDIO source parity | PASS | desktop/package.json、package-lock.json、desktop/wdio.conf.mjs、desktop/e2e/specs/dashboard.e2e.mjs 及相关文档已更新到 E: |

#### Validation Results

| 类别 | 项目 | 状态 | 实际命令/结果摘要 |
|---|---|---|---|
| Build/Toolchain | WDIO dependency install | PASS | npm ci --no-audit --no-fund；依赖安装完成，npm 的 deprecated/allow-scripts 提示未阻塞运行 |
| Build/Toolchain | WDIO package resolution | PASS | npm ls --workspace desktop；五个 WDIO 直接依赖版本与 lockfile 一致；npx --no-install wdio --version 为 9.31.9 |
| Build/Toolchain | Windows Node check | PASS | npm run check --workspace desktop；Vite 7.3.6 build 通过 |
| Build/Toolchain | Windows Node test | PASS | npm run test；Extension 7/7，Desktop 0 tests |
| Build/Toolchain | Windows Node build | PASS | npm run build；Vite production build 和 Extension syntax checks 通过 |
| Packaging/Build | Tauri release build | PASS | npm run build:tauri；生成 target\release\xarchive-desktop.exe；当前 bundle.active=false，不生成 installer |
| Runtime | Edge WebDriver preparation | PASS | service 检测 WebView2 152.0.4191.66，自动下载并配置匹配的 msedgedriver 152.0.4191.66 |
| Runtime | tauri-driver preparation | PASS | service 在 WDIO_AUTO_INSTALL_TAURI_DRIVER 默认开启时通过 cargo 安装 tauri-driver，并在 127.0.0.1:4444 监听 |
| Regression/Automation | Native WDIO dashboard smoke | PASS | npm run test:e2e:windows --workspace desktop；1 spec、2 tests、2 passing，真实 Tauri release window 建立 WebDriver session，Dashboard heading、main、主导航和归档概览检查通过 |
| Runtime | Driver/app cleanup | PASS | WDIO 结束后 tauri-driver、msedgedriver、xarchive-desktop 无残留进程；4444、9223、1420 无监听 |
| Packaging | Installer/updater | NOT APPLICABLE | tauri.conf.json 中 bundle.active=false 且 createUpdaterArtifacts=false |

#### Errors and classification

1. 初次 WDIO 尝试 FAIL：缺少 @wdio/local-runner，WDIO 报 Couldn't find plugin local runner。原因是配置新增时漏列 local runner；已补入 desktop/package.json 和 package-lock.json，并重新同步。修正后的安装和 native smoke PASS。
2. 修正前 tauri-service onPrepare FAIL：未安装 tauri-driver。原因是初始配置将 autoInstallTauriDriver 默认设为 false；已改为默认开启，并保留 WDIO_AUTO_INSTALL_TAURI_DRIVER=0 作为已有 driver/CI 的关闭开关。修正后的自动安装 PASS。
3. 最终运行保留非阻塞 warning：diagnostics 报 Disk Space: Could not determine disk space；未影响 driver、应用或测试。
4. 未注册 tauri-plugin-wdio 时 service 仍会探测 plugin，并在 teardown 输出 Failed to clear mock store: A sessionId is required for this command；该 warning 不影响 session、2/2 测试或清理，但使本轮 smoke 日志较噪。当前只把它归类为 service/plugin 配置限制，不扩大为应用 FAIL。
5. npm ci 输出部分依赖 deprecated 与 allow-scripts 提示；本轮所需 Vite、WDIO、Tauri build 和 smoke 均通过，未观察到由该提示导致的功能失败。

#### Not Executed / Blocked

- Native Host Named Pipe、Registry、manifest、安装入口：NOT RUN；当前对应产品实现/Windows artifact 尚未形成。
- Desktop 应用级 executor/transport real-worker IPC、submit/query/cancel/restart/recovery：NOT RUN；本轮 smoke 只验证 renderer DOM，不临时接线或修改业务入口制造 acceptance。
- browser.tauri execute、Tauri command mocking、backend/frontend log capture：NOT RUN；项目尚未注册 tauri-plugin-wdio。
- WebView2 DPI、键盘焦点、屏幕阅读器、对比度、视觉截图和跨用户 ACL/reparse/长路径：BLOCKED 或 NOT RUN；本轮没有相应人工/fixture/harness，DOM 可见性通过不能替代这些结论。
- 真实 Edge/X、gallery-dl/aria2 DownloadRouter、Telegram/Credential Manager 和生产网络：BLOCKED；缺少专用账号、凭据、外部服务和受控数据。
- Tauri installer/package/updater：NOT APPLICABLE；当前 bundle 配置关闭。

#### Queue reconciliation

- WQ-P1-16 已更新为 WINDOWS_PASS，范围严格限于 external provider、Edge WebDriver/tauri-driver lifecycle 和 dashboard DOM smoke；不等价于全量 Windows GUI 或应用集成 acceptance。
- WQ-P0-02、WQ-P0-03、WQ-P0-04、WQ-P1-01、WQ-P1-02、WQ-P1-03、WQ-P1-04、WQ-P1-05、WQ-P1-12、WQ-P1-13、WQ-P1-14、WQ-P1-15、WQ-P2-01、WQ-P2-02 保持 pending、NOT RUN 或 BLOCKED，未因本轮 smoke 通过而提前关闭。
- 本轮没有 WINDOWS_VERIFICATION_BLOCKING；未执行项不会阻塞继续 Linux 开发，但在 artifact、fixture、账号和 plugin 前置具备后必须重新验证。

#### Linux Follow-up

- 如后续需要 browser.tauri、mock、窗口级 IPC 或日志捕获，单独评估并注册 tauri-plugin-wdio；该项会扩大 Rust/Tauri 配置和验证范围，不在本轮临时接入。
- 继续完成 R1 executor 最终用户入口、Native Host/Named Pipe/Registry 和真实应用级 transport integration；形成可重复 fixture 后再执行 WQ-P1-14/WQ-P0-04。
- 为 WQ-P1-03 及相关 GUI 项准备真实 WebView2/DPI/键盘/辅助技术验证；为 WQ-P0-02、WQ-P1-01、WQ-P1-15 等准备受控账号、旧库、文件锁、ACL、reparse 和外部服务 fixture。
- 对 CI 或预装环境可使用 WDIO_AUTO_INSTALL_TAURI_DRIVER=0，避免每台机器重复 cargo 编译；默认开启行为用于新 Windows 验证副本的可复现准备。
- 本轮未将任何 Windows 代码、依赖、target、dist、日志或用户数据反向同步到 Linux。

本轮结论：15ab756d74aabebb86229cea59076af54c8e5ab7 加 dirty working tree 已同步到 canonical E: Windows 副本；WDIO 依赖、Tauri release artifact、Edge WebDriver、tauri-driver 和真实 dashboard native smoke 均通过。WQ-P1-16 可关闭为 WINDOWS_PASS；tauri-plugin-wdio 相关 API、GUI 深度验收、Native Host/Named Pipe、真实应用 IPC 和外部账号链路仍明确为 NOT RUN/BLOCKED。

### tauri-plugin-wdio 注册方案与后续操作（2026-09-15）

本节记录在上一轮 Windows WDIO 结果之后完成的 Linux 端配置工作；本轮没有执行 Windows native E2E，也没有将 Windows 工作副本中的依赖、target、日志或用户数据反向同步到 Linux。

#### 当前配置结果

- WebdriverIO 9 与 @wdio/tauri-service 的 external provider 基线已落在 Linux 源项目：wdio.conf.mjs、dashboard.e2e.mjs、workspace scripts、package.json 和 package-lock.json 均已配置。
- 当前基础 smoke 使用 Tauri release artifact，只检查真实窗口的 heading、main、主导航和归档概览；高级插件使用独立 `wdio-e2e` feature、`wdio.json` capability、`tauri.wdio.conf.json`、`withGlobalTauri` 和前端 guest JS 入口。
- external provider 不需要 tauri-plugin-wdio-webdriver；只有切换 embedded provider 时才评估该 Rust-only 插件。

#### 当前结论

- 这是“Linux 端 tauri-plugin-wdio 注册/构建/普通 release 与高级 artifact 边界”的配置完成，不等于 Windows 端 WDIO 验证完成。
- WQ-P1-16 在随后同一修订版的 Windows 复验中为 `WINDOWS_FAIL`（Dashboard DOM 断言通过，但 service ACL warning 与 driver cleanup 未满足队列中启动/连接/关闭的完整预期）；此前历史 PASS 保留为历史证据，不覆盖本轮结果。
- WQ-P1-17 在随后同一修订版的 Windows 复验中为 `WINDOWS_FAIL`（专用 artifact、plugin 初始化、wdioTauri/execute、mock 子项和日志生成有通过证据，但官方 wrapper、源 spec、teardown 和清理仍失败）；且当前 Linux WSL 缺少 Node，因此无法在此完成 Node 门禁。Linux 端注册已准备，但 Windows 插件验收仍待修复后重验。

#### Linux 端已完成配置

1. 已加入可选 `tauri-plugin-wdio = 1.4.0` 和 `wdio-e2e` Cargo feature；仅 feature-enabled 构建注册 `tauri_plugin_wdio::init()`。
2. 已加入 `capabilities/wdio.json`、`tauri.wdio.conf.json` 和 `build.rs` capability pattern；普通构建只选择 `default`，高级构建只选择 `wdio`。
3. 已安装 `@wdio/tauri-plugin = 1.4.0`，在 `desktop/src/main.jsx` 导入 guest JS，并启用 `withGlobalTauri`。
4. 已增加 `wdio-plugin.e2e.mjs`，覆盖 plugin availability、frontend execute、command mocking 和 cleanup；高级 npm script 使用跨平台 Node wrapper。
5. Linux 已通过 formatter、普通/feature-enabled Cargo check、strict Clippy、workspace tests、Node check/test/build、JSON/config 解析和依赖锁更新检查；Linux 无 GUI，因此未执行原生 WDIO E2E。

#### Windows 后续验证

1. 在 Windows 工作副本执行 `npm ci`。
2. 执行 `npm run build:tauri:wdio --workspace desktop`，确认专用 artifact 成功生成。
3. 执行 `npm run test:e2e:windows:advanced --workspace desktop`，确认 `window.wdioTauri`、execute、mocking、cleanup、日志捕获和无 session teardown warning。
4. 随后执行普通 `npm run build:tauri --workspace desktop` 与 `npm run test:e2e:windows --workspace desktop`，确认普通 artifact 不包含 WDIO plugin，WQ-P1-16 仍通过。

WQ-P1-17 已完成 Linux 配置，仍为 `WINDOWS_VERIFICATION_PENDING`，且不阻塞当前 Linux 其他开发。不得用 Linux 构建通过或 WQ-P1-16 的 DOM smoke 代替 Windows 插件验证。

### Windows 进一步验证：wdio-e2e 专用构建与 plugin follow-up（2026-09-15 10:49–11:17 +08:00）

本轮以 Linux 源项目为唯一 source of truth，将最新提交单向同步至 canonical E: Windows 验证副本；未将 Windows 依赖、target、dist、日志、验证产物或用户数据反向同步到 Linux。本轮没有修改 Linux 业务代码。

#### Validation Environment

- Linux source：/home/shiraishi/VSCode Workspace/Tw2Tg；branch dev；HEAD 3f70894bce7ee0ff1f464c131b41fbda90ced59a；进入 Windows 阶段时 working tree clean。
- Windows workspace：E:\Shiraishi\VSCode Workspace\Tw2Tg；无 .git，用作验证工作副本。
- Windows：Microsoft Windows 11 专业工作站版 Insider Preview，10.0.29667，64 位。
- Node v24.19.0；npm 11.17.0；Rust 1.98.0；Cargo 1.98.0；WDIO CLI 9.31.9。
- WebView2/Edge：152.0.4191.66；tauri-driver：C:\Users\Shiraishi\.cargo\bin\tauri-driver.exe。
- 关键依赖：@wdio/tauri-plugin 1.4.0、@wdio/tauri-service 1.4.0、@wdio/local-runner 9.31.9、Tauri CLI 2.11.4。

#### Linux pre-validation

| 项目 | 状态 | 实际命令/结果 |
|---|---|---|
| Rust formatter | PASS | cargo fmt --all -- --check |
| Linux plugin feature compile | PASS | cargo check -p xarchive-desktop --features wdio-e2e --all-targets |
| Linux workspace tests | PASS | cargo test --workspace --no-fail-fast；150 个 crate tests 通过，doc-tests 通过 |
| Linux strict Clippy | PASS | cargo clippy --workspace --all-targets --all-features -- -D warnings |
| Linux Node checks | PASS | `node --check`、`npm run check`、`npm run test`、`npm run build`；当前 Node v26.7.0/npm 11.19.0 |

#### Sync verification

| 项目 | 状态 | 实际命令/结果 |
|---|---|---|
| Windows target safety check | PASS | E: 目标存在；node_modules、.venv、target、validation-artifacts、X-Archive、aria2、.codex 和其他目标 extra files 保留 |
| Controlled one-way sync | PASS | robocopy W:\home\shiraishi\VSCode Workspace\Tw2Tg E:\Shiraishi\VSCode Workspace\Tw2Tg /E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT /R:1 /W:1，并排除 .git、node_modules、.venv、target、build/dist、validation-artifacts、X-Archive、Agent 本地目录、数据库、日志和环境覆盖；返回码 1，151 files copied，FAILED=0，Mismatch=0，目标 extra 未删除 |
| Key source parity | PASS | package.json、package-lock.json、Cargo.toml、Cargo.lock、lib.rs、build.rs、wdio capability/config、main.jsx、advanced spec、wrapper 和相关文档 SHA-256 全部匹配 |

#### Validation Results

| 类别 | 项目 | 状态 | 实际命令/结果摘要 |
|---|---|---|---|
| Build/Toolchain | npm ci | PASS | npm ci --no-audit --no-fund；547 packages added；仅有 deprecated/allow-scripts warning |
| Build/Toolchain | WDIO dependency resolution | PASS | npm ls --workspace desktop；WDIO 与 tauri-plugin 版本解析符合 lockfile |
| Packaging/Build | Dedicated wdio-e2e Tauri build | PASS | npm run build:tauri:wdio --workspace desktop；tauri-plugin-wdio 1.4.0 实际编译并生成 release exe；仅有 linker stdout warning |
| Packaging/Build | Ordinary Tauri release build | PASS | npm run build:tauri --workspace desktop；生成 target\release\xarchive-desktop.exe，17,104,896 bytes |
| Runtime | Edge WebDriver and tauri-driver preparation | PASS | service 检测 WebView2 152.0.4191.66，自动下载匹配 msedgedriver，找到 tauri-driver 并建立 4444/4445 session |
| Regression/Automation | Official advanced npm wrapper | FAIL | npm run test:e2e:windows:advanced --workspace desktop；run-wdio-advanced.mjs 的 spawnSync wdio.cmd 在 Windows 返回 EINVAL，未进入 WDIO 测试 |
| Regression/Automation | Direct advanced run, current source spec | FAIL | 等价执行 node wrapper 环境变量下的 node_modules\.bin\wdio.cmd；dashboard 2/2 通过，plugin spec 1 passing、1 failing；失败为 browser.tauri.isTauriApiAvailable is not a function，WDIO 进程错误返回 code 0，按实际 reporter 结果判 FAIL |
| Regression/Automation | Existing mock/execute path | PASS | 聚焦执行 mock test 通过；mock 后的 browser.tauri.execute 返回预期结果，restore interception 完成 |
| Regression/Automation | Corrected wdioTauri/execute probe | PASS | 临时 E 盘 probe 使用 browser.tauri.execute 检查 wdioTauri in window 并读取 h1；1/1 passing；临时文件已删除，不属于 Linux source |
| Integration | Frontend/service log capture | PASS | WDIO_CAPTURE_LOGS=1 生成 E:\Shiraishi\VSCode Workspace\Tw2Tg\desktop\logs 下日志，包含 Tauri ready、guest JS、frontend log 和 backend listener registered 记录；未宣称真实业务 backend log assertion |
| Regression/Automation | Ordinary release WQ-P1-16 DOM smoke | PASS（子项） | npm run test:e2e:windows --workspace desktop；dashboard 2/2 通过；默认生成 capability schema 不含 wdio permission |
| Cleanup | Driver/app cleanup | FAIL | 每次 WDIO 结束后 tauri-driver 和 msedgedriver 仍监听 4444/4445；已按已核实 PID 手动清理，最终 process_count=0、listen_count=0 |

#### Errors and classification

1. run-wdio-advanced.mjs 在 Windows 直接 spawnSync wdio.cmd 返回 EINVAL。分类：项目测试 wrapper / Windows process invocation；阻塞官方高级 npm 入口。Linux 后续应改用 Windows 可执行脚本的安全调用方式，并在 Linux 检查后重新同步。
2. wdio-plugin.e2e.mjs 调用 browser.tauri.isTauriApiAvailable，但当前 @wdio/tauri-service 1.4.0 没有该方法。分类：项目 E2E spec API 使用错误；官方可用性检查应通过 browser.tauri.execute 检查 window.wdioTauri。临时 probe 已证明正确路径可用。
3. 普通 release 未注册 Rust wdio plugin，但前端始终导入 @wdio/tauri-plugin；service hook 仍调用 plugin:wdio|execute，Windows 日志反复出现 Command plugin:wdio|execute not allowed by ACL。分类：发布/测试边界配置问题；需要 Linux 决定 guest JS 的 feature/build 条件或普通路径的 service hook 行为。
4. 高级和普通运行均出现 Failed to clear mock store: A sessionId is required for this command，且 driver 没有自动退出。分类：@wdio/tauri-service teardown/test infrastructure；不属于应用业务逻辑 PASS，需在 Linux 评估 service 生命周期/版本兼容或显式清理策略；本轮不升级依赖。
5. diagnostics 报 Disk Space: Could not determine disk space，Node 报 shell option deprecation，npm 报 deprecated/allow-scripts；均未阻塞构建或断言，但保留为非阻塞 warning。

#### Queue reconciliation

- WQ-P1-16：WINDOWS_FAIL。Dashboard DOM 断言 2/2 通过，但 service ACL warning 和 driver cleanup 未满足队列中“启动/连接/关闭”的完整预期；此前历史 PASS 保留为历史证据，不覆盖本轮结果。
- WQ-P1-17：WINDOWS_FAIL。专用 artifact、plugin 初始化、wdioTauri/execute、mock 子项和日志生成有 PASS 证据，但官方 wrapper、源 spec、teardown 和清理仍失败。
- 其他 Native Host/Named Pipe、真实应用 executor/transport IPC、DPI/键盘/屏幕阅读器、ACL/reparse/长路径、真实账号/外部服务和 installer/updater 继续为 NOT RUN、BLOCKED 或 NOT APPLICABLE。
- 本轮没有 WINDOWS_VERIFICATION_BLOCKING；Linux 修复和 Windows re-validation 是当前自动化范围的后续要求。

#### Linux follow-up

1. 已修复 run-wdio-advanced.mjs 的 Windows wdio.cmd 调用，并保留真实退出码；仍需补充 Windows wrapper smoke。
2. 已将不存在的 isTauriApiAvailable 断言改为官方 execute/wdioTauri 检查；仍需 Windows 运行高级 spec 验证失败传播。
3. 已将普通 release 的 guest JS 与 wdio-e2e 专用构建边界分离；仍需 Windows 重新验证普通 artifact 不产生 WDIO ACL warning。
4. 处理 tauri-service teardown 的 sessionId/driver 残留问题；修复后验证 4444、4445 和应用进程自动清理，不依赖手工 Stop-Process。
5. 当前 revision 的 Node check/test/build、wrapper syntax checks 和 Linux Rust gates 已通过；下一步是单向同步并在 Windows 重验 WQ-P1-16/WQ-P1-17。
6. 在下一轮 Windows 验证中追加可观察的 backend log assertion；当前仅证明日志捕获机制和 frontend/listener 记录，不证明业务 backend 日志完整性。

本轮最终 Windows 状态：专用 wdio-e2e 构建、正确的 plugin execute probe、mock 子项、日志生成和普通 DOM 断言均有通过证据；但 WQ-P1-16 与 WQ-P1-17 的完整验收分别因 cleanup/ACL 和 wrapper/spec/teardown 问题为 WINDOWS_FAIL，必须完成 Linux 修复后再重验。
### 当前 Linux 端后续验证与改动登记（Windows 复验后）

以下项目是本轮 Windows 结果直接要求 Linux 源项目处理的内容；它们不是新的 Windows 队列项，也不改变历史报告。

| ID | 优先级 | 类型 | 当前状态 | Linux 端动作 | 完成标准 |
|---|---|---|---|---|---|
| LINUX-WDIO-01 | P1 | 配置/测试 wrapper | DONE-LINUX | 已改为当前 Node 进程加载 workspace WDIO CLI，并透传真实退出码；Windows wrapper smoke 仍待执行 | Windows advanced npm 入口不再因 `spawnSync wdio.cmd` 在测试前失败 |
| LINUX-WDIO-02 | P1 | E2E spec | DONE-LINUX | 已改为通过 `browser.tauri.execute` 检查 `window.wdioTauri`；保留 execute、mock restore 和失败传播断言 | Windows advanced spec 可实际进入插件断言 |
| LINUX-WDIO-03 | P1 | 构建边界 | DONE-LINUX | 普通 release 不加载 WDIO guest JS；专用构建通过 `VITE_WDIO_E2E=1` 注入 guest JS，并保留 wdio-e2e feature/capability | Windows 重验普通 release 无 ACL warning，专用 artifact 可 execute/mock/log |
| LINUX-WDIO-04 | P1 | 生命周期/兼容性 | WINDOWS_REVALIDATION_PENDING | Linux 配置已完成；仍需确认 session 创建、mock cleanup、service/driver shutdown 顺序 | 不再出现 sessionId warning；应用、4444 tauri-driver、4445 msedgedriver 自动回收 |
| LINUX-WDIO-05 | P1 | Linux 门禁 | DONE-LINUX | Node v26.7.0/npm 11.19.0 下 Node check/test/build、wrapper syntax/config check、Rust fmt/check/test/clippy 均通过 | Linux 门禁和 capability 边界静态证据完成 |
| LINUX-WDIO-06 | P1 | 回归编排 | WINDOWS_REVALIDATION_PENDING | 需将本次 Linux 状态单向同步到 E:，先 advanced 后 ordinary smoke，并回写队列状态 | WQ-P1-17 先满足高级 API、日志、teardown/cleanup；WQ-P1-16 再满足 DOM、ACL clean 和 driver cleanup |

#### Linux 端验证命令

当前 Linux 源目录已完成以下 Node 门禁；后续配置或依赖变更后按需复跑：

    node --version
    npm --version
    npm ci --no-audit --no-fund
    npm run check
    npm run test
    npm run build
    node --check desktop/scripts/run-wdio-advanced.mjs

代码/配置修正后执行：

    cargo fmt --all -- --check
    cargo check --workspace --all-targets
    cargo check -p xarchive-desktop --features wdio-e2e --all-targets
    cargo test --workspace --no-fail-fast
    cargo clippy --workspace --all-targets --all-features -- -D warnings

另需检查：普通构建生成的 capability schema 不含 wdio permission；wdio-e2e 构建包含该 permission；普通 feature tree 不意外引入 tauri-plugin-wdio；JSON 与 lockfile 可解析。Linux 无可用 GUI 时，原生 WDIO 仍记为 NOT RUN，不以 mock 或构建成功代替。

#### Windows handoff 入口

当前 Linux 配置完成后的 Windows 详细执行步骤已集中记录在 [`../validation/windows-wdio-handoff.md`](../validation/windows-wdio-handoff.md)，包括同步安全检查、专用 artifact、advanced plugin E2E、session/driver cleanup、普通 release smoke、结果记录和队列回写。该 handoff 不改变当前 `WQ-P1-16`、`WQ-P1-17` 的 `WINDOWS_FAIL` 状态；只有实际 Windows 重验满足全部预期后才可更新队列。

#### 暂不需要在 Linux 端处理的项目

Native Host/Named Pipe、真实 executor/transport IPC、WebView2/DPI/键盘/屏幕阅读器、Windows ACL/reparse/长路径、真实账号/外部服务、installer/updater 仍属于 Windows 队列。它们不因本轮 Linux 门禁通过而自动关闭，也不应在本轮通过修改业务代码“顺带解决”。

### Windows 进一步验证：Linux 最新配置同步、专用 plugin E2E 与普通 release 边界（2026-09-15）

本轮依据 Linux 源仓库最终提交 0537d32c9b4d2ef71ec508467d75378a767a34e7 执行单向同步和 Windows 验证。进入同步阶段时 Linux branch 为 dev、working tree clean；Windows E: 目录仍为验证工作副本，不是 source of truth。本轮未把 Windows 的依赖、target、dist、日志、验证产物或用户数据反向同步到 Linux，也未修改业务代码。

#### Validation Environment

- Linux source：/home/shiraishi/VSCode Workspace/Tw2Tg；branch dev；HEAD 0537d32c9b4d2ef71ec508467d75378a767a34e7；同步前 working tree clean。
- Windows workspace：E:\Shiraishi\VSCode Workspace\Tw2Tg；无 .git，仅作为验证副本。
- Windows：Windows 11 专业工作站版 Insider Preview，10.0.29667，AMD64。
- Node v24.19.0；npm 11.17.0；Rust/Cargo 1.98.0；WDIO CLI 9.31.9；WebView2/Edge 152.0.4191.66。
- WDIO/Tauri 依赖：@wdio/tauri-plugin 1.4.0、@wdio/tauri-service 1.4.0、@wdio/local-runner 9.31.9、Tauri CLI 2.11.4；tauri-driver 位于 C:\Users\Shiraishi\.cargo\bin\tauri-driver.exe。
- 普通 release artifact：E:\Shiraishi\VSCode Workspace\Tw2Tg\target\release\xarchive-desktop.exe；17096704 bytes；SHA-256 6DF52770B82F174EBB47A17C50A4092800383FE17580308139EC32EB28E8A84E。

#### Linux pre-validation

| 项目 | 状态 | 实际命令/结果 |
|---|---|---|
| cargo fmt --all -- --check | PASS | 通过 |
| cargo check --workspace --all-targets | PASS | 通过 |
| cargo check -p xarchive-desktop --features wdio-e2e --all-targets | PASS | wdio-e2e feature 编译通过 |
| cargo test --workspace --no-fail-fast | PASS | 150 个 crate tests 通过，doc-tests 通过 |
| cargo clippy --workspace --all-targets --all-features -- -D warnings | PASS | 通过 |
| Linux Node check/test/build 与 syntax/config checks | NOT RUN | WSL 当前 node 不可用；node --version 返回 bash: node: command not found。npm 实际调用 Windows npm 11.17.0，UNC 工作目录回退到 C:\Windows 并因缺少 package.json 报 ENOENT，因此不能把该结果记为 Linux Node PASS |
| Linux native WebView/WDIO | NOT RUN | Linux 环境无可用原生 GUI 验收条件；不以构建或 mock 代替 |

#### Sync verification

| 项目 | 状态 | 实际命令/结果 |
|---|---|---|
| E: 目标安全检查 | PASS | 目标存在；.codex、.venv、aria2、node_modules、target、validation-artifacts、X-Archive 等 Windows 本地目录均保留 |
| Controlled one-way sync | PASS | 使用 robocopy 从 W:\home\shiraishi\VSCode Workspace\Tw2Tg 到 E:\Shiraishi\VSCode Workspace\Tw2Tg；排除 .git、依赖、虚拟环境、target、build/dist、验证产物、数据库、日志、secrets 和 agent-local 文件；返回码 3，153 files copied，FAILED=0，Mismatch=0，未删除目标 extra |
| Key source parity | PASS | desktop/package.json、package-lock.json、wdio.conf.mjs、Tauri Cargo.toml/Cargo.lock、lib.rs、build.rs、tauri.wdio.conf.json、wdio capability、main.jsx、advanced spec、两个 wrapper 和 handoff 文档 SHA-256 均匹配 |
| E: local preservation | PASS | 同步后 .codex、.venv、aria2、node_modules、target、validation-artifacts、X-Archive 仍存在 |

robocopy 返回码 3 表示复制成功并检测到目标端 extra/mismatch 类目录状态；本次输出明确为 FAILED=0、Mismatch=0，且没有执行删除目标 extra 的参数。

#### Windows Validation Results

| 类别 | 项目 | 状态 | 实际命令/结果摘要 |
|---|---|---|---|
| Build/Toolchain | npm ci | PASS | npm ci --no-audit --no-fund；547 packages added；仅有 deprecated 与 allow-scripts warning |
| Build/Toolchain | WDIO dependency resolution | PASS | npm ls --workspace desktop；WDIO/Tauri plugin 依赖按 lockfile 解析 |
| Build/Toolchain | Node workspace checks | PASS | npm run check、npm run test、npm run build；Extension tests 7/7 |
| Build/Toolchain | wrapper/spec syntax | PASS | node --check desktop/scripts/run-wdio-advanced.mjs、build-tauri-wdio.mjs、e2e/specs/wdio-plugin.e2e.mjs |
| Packaging/Build | Dedicated wdio-e2e Tauri build | PASS | npm run build:tauri:wdio --workspace desktop；tauri-plugin-wdio 1.4.0 实际编译，专用 release artifact 生成 |
| Packaging/Build | Ordinary Tauri release build | PASS | npm run build:tauri --workspace desktop；普通 artifact 生成，linker stdout warnings 未阻塞 |
| Packaging/Build | Ordinary artifact static boundary | PASS | 生成 capability schema 不含 wdio permission；ordinary desktop/dist JS 不含 wdioTauri marker |
| Runtime | Edge WebDriver/tauri-driver preparation | PASS | service 建立 Windows native session，使用 WebView2 152.0.4191.66 与匹配 driver；WDIO 运行时端口可用 |
| Regression/Automation | Advanced official entry | PASS（功能项） | npm run test:e2e:windows:advanced --workspace desktop；wrapper 已由 Node 直接加载本地 WDIO CLI，Dashboard 2/2、plugin spec 2/2 通过 |
| Regression/Automation | Advanced plugin API | PASS（功能项） | window.wdioTauri 存在；browser.tauri.execute 返回预期结果；mock command 与 restoreAllMocks 子项通过 |
| Integration | Frontend/service log capture | PASS（机制项） | WDIO_CAPTURE_LOGS=1 生成 desktop/logs 日志，包含 Tauri ready、guest JS、frontend log 和 backend listener registered；本轮未宣称真实业务 backend log assertion |
| Cleanup | Advanced session/mock/driver teardown | FAIL | teardown 输出 Failed to clear mock store: Error: A sessionId is required for this command；tauri-driver/msedgedriver 曾在测试返回后残留，需按已核实 PID 手工 Stop-Process；最终已清至 process_count=0、listen_count=0，但手工清理不满足 PASS |
| Regression/Automation | Ordinary release native smoke | FAIL | npm run test:e2e:windows --workspace desktop 在普通 artifact 上反复 executeAsyncScript/RESULT false，并输出 tauri-service:window: Failed to get window states: Error: Tauri plugin not available. Make sure @wdio/tauri-plugin is installed and registered in your Tauri app. 本轮中断该轮轮询；未将 dashboard/进程关闭记为完整 PASS |
| Regression/Automation | Windows Rust gates | PASS | cargo fmt --all -- --check、cargo check --workspace --all-targets、cargo clippy --workspace --all-targets --all-features -- -D warnings 通过 |
| Runtime/Integration | Windows workspace tests | PASS | 设置 E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe 为 PYTHON 后 cargo test --workspace --no-fail-fast 通过，150 tests/doc-tests 通过 |
| Runtime/Integration | Sidecar pytest | PASS | .\.venv\Scripts\python.exe -m pytest sidecar/tests -q；10 passed in 0.40s |
| Cleanup | Final environment cleanup | PASS | 最终无 tauri-driver.exe、msedgedriver.exe、xarchive-desktop.exe 残留；4444、4445、9223、1420 无监听；E: 本地专用目录保留 |

#### Errors and classification

1. Ordinary release service/plugin boundary：普通构建确实没有 wdio capability 和 guest JS，但当前 wdio.conf.mjs 仍无条件启用 tauri-service；service hook 因找不到 plugin 反复轮询并输出 Tauri plugin not available，导致普通 native smoke 无法干净完成。分类为项目 WDIO 配置边界问题，不是业务代码问题；阻塞 WQ-P1-16 完整 PASS，也应由 Linux 后续配置/测试编排修复后重验。
2. Advanced teardown：tauri-service 结束时输出 A sessionId is required for this command，且 driver 进程未自动回收。分类为 @wdio/tauri-service 生命周期/测试基础设施兼容性问题；阻塞 WQ-P1-17 完整 PASS。手工 Stop-Process 仅用于恢复验证环境，不是验收结果。
3. Plugin console warning：日志出现 Invoke interception via defineProperty failed; mock routing via window.__wdio_mocks__ remains active。mock 功能断言仍通过；目前归类为 plugin 的兼容性 warning，需后续确认是否可接受，不改写为错误或 PASS 的完整替代证据。
4. Linux Node prerequisite：Linux WSL 缺少 node，Windows npm 在 UNC 工作目录中回退 C:\Windows 并报 package.json ENOENT。分类为验证环境缺失，不是项目业务失败；Linux Node 门禁必须在补齐 Linux Node 后重新执行。
5. 非阻塞 warning：npm 的 deprecated/allow-scripts、Tauri linker stdout warning 和 WDIO diagnostics 的 Disk Space warning 未阻塞已通过的构建/测试，但应保留在日志中。

#### Not Executed / Blocked

- Linux Node check/test/build、Linux WDIO 配置加载/dry-run：NOT RUN；缺少 Linux node/npm 运行时，且 Windows npm UNC fallback 不能替代 Linux 验证。
- Linux native WebView/WDIO：NOT RUN；无 GUI。
- Native Host/Named Pipe、Registry、browser installation、真实 Edge/X Cookie archive、aria2/DownloadRouter 真实链路、Telegram/Credential Manager：NOT RUN 或 BLOCKED；本轮没有实现、账号、凭据或外部服务前置。
- Desktop executor/transport real-worker IPC、submit/query/cancel/restart/recovery、旧 SQLite 应用迁移、Windows ACL/reparse/长路径、DPI/键盘/屏幕阅读器：NOT RUN 或 BLOCKED；本轮 WDIO smoke 不等价于应用级验收。
- Tauri installer/signing/updater：NOT APPLICABLE；当前范围未生成 bundle/installer。
- WQ-P1-16/WQ-P1-17：不是 NOT RUN；两项均已执行并按实际失败边界保持 WINDOWS_FAIL。

#### Queue reconciliation

- WQ-P1-16：WINDOWS_FAIL。普通 artifact 静态隔离通过，但 service 无条件探测 plugin 并使普通 smoke 中断；driver cleanup 也未满足自动关闭标准。
- WQ-P1-17：WINDOWS_FAIL。专用 artifact 的 build、plugin API、mock、日志机制和 2/2 功能断言通过，但 teardown sessionId warning 与 driver 残留仍存在。
- 本轮没有 WINDOWS_VERIFICATION_BLOCKING；失败项需要 Linux 配置/测试编排修复和 Windows 重验，但不阻塞其他 Linux 业务开发。

#### Linux follow-up

1. 为普通 release 建立与专用 wdio-e2e artifact 相匹配的 WDIO 配置边界：普通 smoke 不应无条件执行依赖 tauri-plugin-wdio 的 service hook；可采用条件 service、分离 basic/advanced config 或其他项目内可维护方式，但需保持普通 artifact 不加载 guest JS/capability。
2. 追踪 @wdio/tauri-service 的 session 创建、mock-store cleanup 与 driver shutdown 顺序；修复后必须验证不再出现 sessionId warning，并确认应用、tauri-driver、msedgedriver 和 4444/4445/1420/9223 自动清理，不依赖 Stop-Process。
3. 在 Linux 安装/启用项目要求的 Node/npm 后重跑 npm ci、npm run check/test/build、三个 node --check、WDIO config load/dry-run 与普通/专用静态 capability boundary；保留本轮 Linux Rust PASS，不因 Node NOT RUN 误报全门禁 PASS。
4. 重同步并按 advanced → teardown → ordinary 顺序重验 WQ-P1-17/WQ-P1-16；如果普通 smoke 仍受 service hook 影响，继续保持 WINDOWS_FAIL。
5. 日志捕获机制已有证据，但若要关闭 WQ-P1-17，还需增加可观察的 backend log assertion，并确认日志不包含 Cookie/token/secret。
6. Native Host、executor/transport、真实 filesystem/ACL、GUI accessibility、账号链路和 packaging 继续按其他 Windows queue 项独立准备，不由本轮 WDIO 结果代替。

#### Linux follow-up implementation after the 2026-09-15 Windows results

Linux 已完成以下配置，不代表对应 Windows 队列项已通过：

- 新增 `desktop/scripts/wdio-tauri-service.mjs`，复用 `@wdio/tauri-service` 的官方 launcher，仅覆盖 worker 生命周期；
- `beforeCommand` 不再为单窗口 smoke 调用依赖 `plugin:wdio` 的 `get_window_states`；普通 release 因此不会通过 service hook 轮询不存在的 WDIO plugin；
- `afterSession` 只在有效 session 存在时删除 WebDriver session，不重复调用 mock restore；高级 spec 继续负责显式 `restoreAllMocks()`；
- `desktop/wdio.conf.mjs` 已改用项目 service adapter；未升级 WDIO 依赖，未修改业务 Rust 或普通 Tauri capability；
- 本轮现场复核发现 Linux Node 前置并未恢复：WSL 无 Linux node，npm 来自 Windows 挂载路径；Node workspace check/test/build、wrapper/spec/config syntax 和 WDIO config load 当前均为 NOT RUN；Rust 门禁保持通过。

对应状态：LINUX-WDIO-07、LINUX-WDIO-08 为 DONE-LINUX / WINDOWS_REVALIDATION_PENDING；LINUX-WDIO-09 当前为 NOT RUN；LINUX-WDIO-10 仍为 WINDOWS_REVALIDATION_PENDING。WQ-P1-16/WQ-P1-17 继续保持 WINDOWS_FAIL，必须在 Windows 重验满足完整验收条件后更新。

#### Current conclusion

Linux 最新配置已安全单向同步到 E:，代码/配置关键文件校验一致；Windows 构建、Rust/Sidecar 门禁、专用 plugin 功能子项和日志机制均有通过证据。Linux service adapter 配置已完成，但 Linux Node 门禁当前为 NOT RUN；Windows 仍需重验普通 smoke 的 ACL 边界以及 advanced teardown 的 sessionId/driver 生命周期。WQ-P1-16/WQ-P1-17 均保持 WINDOWS_FAIL；Windows 工作副本中的进程已清理。

### Linux 端剩余工作复核（2026-09-15）

本节从最近一次 Windows 结果反推 Linux 当前仍需处理的事项，不能把它们误读为新的 Windows PASS。

| ID | 当前状态 | 证据边界 | Linux 后续动作 |
|---|---|---|---|
| LINUX-WDIO-07 | DONE-LINUX / WINDOWS_REVALIDATION_PENDING | `desktop/wdio.conf.mjs` 已改用项目 service adapter；普通 release 不再通过 worker focus probe 轮询 `window.wdioTauri` | Windows 重验普通 smoke、ACL 边界和自动 cleanup；保持普通 artifact 安全边界 |
| LINUX-WDIO-08 | DONE-LINUX / WINDOWS_REVALIDATION_PENDING | service adapter 的 `afterSession` 只删除有效 session，不重复执行 mock restore；高级 spec 保留显式 restore | Windows 重验 session teardown、driver shutdown 和端口清理；自动清理仍是验收条件 |
| LINUX-WDIO-09 | 历史快照：NOT RUN；当前 DONE-LINUX | 本轮现场快照曾发现 WSL 无 Linux node，npm 来自 Windows 挂载路径；后续 Linux reconciliation 已完成 Node workspace check/test/build、syntax 和 WDIO config load | 当前结果以 `DONE-LINUX` 为准；不把 Windows npm UNC fallback 计为 Linux PASS |
| LINUX-WDIO-10 | WINDOWS_REVALIDATION_PENDING | 专用功能子项已通过，但两个 Windows 队列项完整验收仍失败 | Linux 修改和门禁完成后重新同步 E:，advanced → teardown → ordinary 重验 |

#### Linux 端执行顺序

1. 安装或启用 Linux Node/npm，确认 command -v node/npm 不指向 Windows 挂载路径。
2. 执行 Node workspace 门禁、wrapper/spec syntax、WDIO config load/dry-run。
3. 若修改 wdio.conf.mjs 或相关测试编排，执行 Rust fmt/check/feature check/test/clippy 和普通/专用 Tauri 静态构建边界检查。
4. 检查普通 artifact 不注册 wdio、专用 artifact 才加载 guest JS/capability。
5. 将修复后的 Linux working tree 重新单向同步到 E:，再执行 WQ-P1-17 与 WQ-P1-16。
6. 仍未具备 Linux GUI 时，Linux native WDIO 标记 NOT RUN，不用 mock 或静态构建替代。

#### 明确不应在当前 Linux 阶段做的操作

不应为消除 Windows warning 而修改业务 Rust、普通 Tauri capability、用户数据目录或生产入口；不应升级 @wdio/tauri-service、改用 embedded provider 或通过手工杀进程掩盖生命周期问题。若确认是第三方 service 兼容性，应记录版本、最小复现和候选升级评估，另行决定是否升级。

### Windows 进一步验证：Linux 最新 working tree 同步与 WDIO adapter revalidation（2026-09-15 16:06–16:22 +08:00）

本轮针对 Linux 端新增的 desktop/scripts/wdio-tauri-service.mjs 与 desktop/wdio.conf.mjs 配置执行单向同步和 Windows 验证。Linux 是 source of truth；本轮没有把 E: 的依赖、target、日志、验证产物或用户数据反向同步到 Linux，也没有修改业务 Rust、前端业务逻辑或依赖版本。

#### Validation Environment

- Linux source：/home/shiraishi/VSCode Workspace/Tw2Tg，branch dev，HEAD 0537d32c9b4d2ef71ec508467d75378a767a34e7；本轮开始时 working tree 为 dirty，包含用户新增的 service adapter、wdio 配置和既有文档改动。
- Windows workspace：E:/Shiraishi/VSCode Workspace/Tw2Tg；无 .git，仅作为 Windows validation copy。
- Windows：Windows 11 专业工作站版 Insider Preview 10.0.29667，AMD64。
- Node v24.19.0、npm 11.17.0、Rust/Cargo 1.98.0、WDIO CLI 9.31.9、Tauri CLI 2.11.4、WebView2/Edge 152.0.4191.66；tauri-driver.exe 位于 C:/Users/Shiraishi/.cargo/bin/tauri-driver.exe。

#### Linux pre-validation and source parity

| 项目 | 状态 | 实际命令/结果 |
|---|---|---|
| Linux Rust fmt/check/feature check/test/clippy | PASS | cargo fmt --all -- --check、workspace/all-targets check、xarchive-desktop --features wdio-e2e check、cargo test --workspace --no-fail-fast、strict clippy 均通过；workspace tests 和 doc-tests 通过 |
| Linux Node/syntax/config load | NOT RUN | node --version 在 WSL 返回 bash: node: command not found；因此 Linux Node workspace check/test/build、wrapper/spec syntax 和 WDIO config load 未执行 |
| Linux native WebView/WDIO | NOT RUN | 当前 Linux 环境无可用 GUI 原生验收条件 |
| Controlled sync | PASS | 首次使用不可用的 W: 路径返回 robocopy ERROR 3，未改动目标；随后从 UNC Linux source 重试，robocopy 返回码 1，154 files copied、FAILED=0、Mismatch=0，未删除 E: extra |
| Key source parity | PASS | wdio.conf.mjs、wdio-tauri-service.mjs、两个 wrapper、plugin spec、Cargo/Tauri capability 配置及相关开发/验证文档 SHA-256 与 Linux 源一致 |
| E: local preservation | PASS | .codex、.venv、node_modules、target、validation-artifacts、X-Archive、aria2 和其他 Windows extra 保留 |

#### Windows Validation Results

| 类别 | 项目 | 状态 | 实际命令/结果摘要 |
|---|---|---|---|
| Build/Toolchain | npm dependencies | PASS | npm ci --no-audit --no-fund；547 packages added；仅有 deprecated 与 allow-scripts warning |
| Build/Toolchain | Node workspace checks | PASS | npm run check、npm run test、npm run build；Extension tests 7/7，Desktop Node tests 0 |
| Build/Toolchain | WDIO/config syntax | PASS | node --check 通过 adapter、wdio.conf.mjs、两个 wrapper 和 wdio-plugin.e2e.mjs |
| Packaging/Build | Dedicated wdio-e2e Tauri build | PASS | npm run build:tauri:wdio --workspace desktop；tauri-plugin-wdio 1.4.0 编译并生成 release artifact |
| Packaging/Build | Ordinary Tauri release build | PASS | npm run build:tauri --workspace desktop；生成 target/release/xarchive-desktop.exe；仅有 linker stdout warning |
| Packaging/Build | Ordinary static boundary | PASS（静态子项） | ordinary desktop/dist JS 不含 wdioTauri marker；普通 tauri.conf.json 选择 default，专用 tauri.wdio.conf.json 选择 wdio。生成的 capability schema 是两套 profile 的 inventory，不能单独据此宣称 ordinary binary 已完成 native 验收 |
| Regression/Automation | Advanced WDIO E2E | FAIL | npm run test:e2e:windows:advanced --workspace desktop；Dashboard 与 plugin 两个 worker 均在 POST http://127.0.0.1:4444/session 失败：session not created: DevToolsActivePort file doesn't exist；0 passed, 2 failed |
| Regression/Automation | Ordinary WDIO native smoke | FAIL | 首次 npm run test:e2e:windows --workspace desktop 因无法联网下载 Edge 152.0.4191.66 driver，随后 tauri-driver 以 code 1 退出；重试使 tauri-driver 成功监听 4444 后，WDIO worker 报 SystemError: uv_os_get_passwd returned ENOMEM (not enough memory)，未进入 Dashboard spec |
| Runtime/Integration | Windows Rust fmt/check/WDIO feature/clippy | PASS | Windows E: 副本中 cargo fmt --all -- --check、workspace/all-targets check、wdio-e2e feature check、strict clippy 通过 |
| Runtime/Integration | Windows workspace tests | PASS | 首次非提升运行因 Python worker 启动受沙箱权限影响而出现 2 个 NotRunning；随后在同一 E: 副本、PYTHON=E:/Shiraishi/VSCode Workspace/Tw2Tg/.venv/Scripts/python.exe 下重跑，全部 150 tests/doc-tests 通过 |
| Runtime/Integration | Sidecar pytest | PASS | 项目 .venv 下 sidecar 测试 10 passed（历史同版本证据，本轮 adapter 不触及 Python 代码） |
| Cleanup | Validation process/ports | PASS（恢复性清理） | advanced/ordinary 失败后按本轮已核实 PID 清理残留 tauri-driver/msedgedriver；最终无 tauri-driver.exe、msedgedriver.exe、xarchive-desktop.exe，1420/4444/4445/9223 无监听。手工清理不满足队列的自动 cleanup PASS |

#### Errors and classification

1. Advanced DevToolsActivePort：两个 spec 都未能创建 WebDriver session。分类为 Windows WebView2/EdgeDriver/Tauri native session 启动或测试环境问题；未将其误判为 plugin API 功能通过，也未修改业务代码。需要在稳定 driver、无残留用户数据和可用 Windows GUI 条件下取得最小复现。
2. Edge driver 下载失败：@wdio/tauri-service 尝试自动下载匹配 driver 时报告无法连接到远程服务器；E: 临时缓存中虽有匹配 msedgedriver 152.0.4191.66，但 service 的自动发现依赖 Windows where msedgedriver.exe，不能把临时缓存存在视为可用系统前置。分类为网络/Windows 工具链前置，不是业务代码失败。
3. Node uv_os_get_passwd ENOMEM：普通 smoke 在 worker 启动阶段失败，直接运行 node 的 os.userInfo() 也复现同一错误。分类为 Windows Node/OS process environment issue，阻塞 WDIO worker，不是 Dashboard 断言失败。
4. Non-blocking warnings：npm deprecated/allow-scripts、Tauri MSVC linker stdout warning、WDIO diagnostics 的 Disk Space warning 和 Node child-process shell deprecation 未阻塞构建或 Rust/Node 单测；保留为 warning。

#### Not Executed / Blocked / Not Applicable

- Linux Node check/test/build、Node syntax/config load：已在后续 Linux reconciliation 中补做并通过；本轮 Windows 报告中的 `NOT RUN` 仅表示当时验证快照的历史状态。Linux native WebView/WDIO 仍为 NOT RUN，因为当前 Linux 环境无原生 GUI；Windows npm fallback 不计为 Linux PASS。
- WQ-P1-16/WQ-P1-17：历史轮次均已执行并记录为 WINDOWS_FAIL；最新轮次主要受 driver/Node worker/native session 前置阻断。当前队列改为 `WINDOWS_VERIFICATION_PENDING`，等待前置稳定后的重新验证，不视为 PASS。
- Native Host/Named Pipe/Registry、真实 Edge/X、aria2/DownloadRouter 业务链路、Telegram/Credential Manager、executor/transport real IPC、ACL/reparse/长路径、DPI/键盘/屏幕阅读器和 installer/signing/updater：本轮 NOT RUN 或 BLOCKED，缺少对应 artifact、账号、GUI/硬件或外部服务前置。
- Tauri installer/signing/updater：NOT APPLICABLE，当前范围未生成 bundle/installer。

#### Queue reconciliation and Linux follow-up

- WQ-P1-16：WINDOWS_FAIL。普通 artifact 的静态隔离子项通过，但 native smoke 未进入 Dashboard 断言，driver/worker/自动 cleanup 完整条件未满足。
- WQ-P1-17：WINDOWS_FAIL。adapter 已同步、专用 build 通过，但本轮两个 advanced worker 均未创建 session；上一轮 plugin API/mock/log 子项 PASS 不覆盖本轮 session/teardown 验收。
- 当前没有 WINDOWS_VERIFICATION_BLOCKING。

此前报告中的 Linux 后续只需处理以下前置和重验，不应借机修改业务实现：

1. 历史记录曾显示 Linux Node/npm 前置不可用；当前 Linux reconciliation 已确认 Node v26.7.0/npm 11.19.0 来自 Linux nvm 路径，并完成 Node check/test/build、adapter/wrapper/spec node --check 和 WDIO config load/dry-run。该历史 `NOT RUN` 不再代表当前状态。
2. Windows 端准备与 Edge/WebView2 匹配且能被 where msedgedriver.exe 发现的稳定 driver；调查 Node uv_os_get_passwd ENOMEM 与 DevToolsActivePort 的最小环境复现。
3. 保持当前 adapter、普通/专用 capability 和 guest JS 隔离；不通过手工杀进程、延长等待、吞掉 warning、升级依赖或切换 embedded provider 伪造 PASS。
4. 前置稳定后重新同步 Linux working tree，按 advanced → teardown/automatic cleanup → ordinary smoke 顺序重验；在完整条件满足前，WQ-P1-16/WQ-P1-17 继续为 WINDOWS_FAIL。

#### Linux follow-up closure after the latest Windows environment failures

Linux 端已完成本轮可执行的后续配置和门禁：

- Node v26.7.0/npm 11.19.0 已由 Linux 环境直接提供；`npm ci --no-audit --no-fund`、workspace `check/test/build`、WDIO adapter/config/spec syntax check 和 WDIO config load 已通过；
- Rust fmt、workspace/all-targets check、`wdio-e2e` feature check、workspace tests 和 strict Clippy 已通过；
- `desktop/wdio-tauri-service.mjs`、普通/专用 capability、条件 guest JS 和 wrapper 不再有新的 Linux 侧配置动作；
- Windows `DevToolsActivePort file doesn't exist`、Edge driver 下载/发现失败和 `uv_os_get_passwd returned ENOMEM` 均记录为 Windows 环境/工具链/native session 前置问题，不修改业务代码或升级依赖。

当前 reconciliation：`LINUX-WDIO-09 = DONE-LINUX`；`LINUX-WDIO-10 = WINDOWS_VERIFICATION_PENDING`。历史 WQ-P1-16/WQ-P1-17 的 `WINDOWS_FAIL` 保留在历史结果中；当前队列状态为 `WINDOWS_VERIFICATION_PENDING`，等待 Windows driver、native session 和 Node worker 前置稳定后重验。

### Linux 平台详细测试收口（2026-09-15）

本节记录当前 Linux source 的实际测试结果；它不替代 Windows native/WebView2 结论，也不把 Linux build 或静态检查扩大为 `WINDOWS_PASS`。

#### Validation Environment

- Source：`/home/shiraishi/VSCode Workspace/Tw2Tg`
- Branch：`dev`
- Revision：`0537d32c9b4d2ef71ec508467d75378a767a34e7`
- Working tree：dirty；验证包含当前未提交的 WDIO 实现和文档修改
- Node/npm：v26.7.0 / 11.19.0；路径来自 Linux nvm
- Rust/Cargo：1.98.0 / 1.98.0
- Python：3.14.4；使用 `/home/shiraishi/VSCode Workspace/Tw2Tg/.venv/bin/python`
- Linux GUI：`DISPLAY=:0`、`WAYLAND_DISPLAY=wayland-0`，但未发现 `webkit2gtk-driver`、Chrome/Chromium 或 `xvfb-run`
- 结果日志：`/tmp/tw2tg-linux-validation-20260915/`（临时目录，不提交）

#### Validation Results

| ID | Layer | Status | Command / evidence | Result |
|---|---|---|---|---|
| LNX-STATIC-01 | static | `PASS` | `node --check`：service adapter、WDIO config、advanced/build wrapper、plugin spec | 所有脚本语法通过 |
| LNX-STATIC-02 | static | `PASS` | WDIO config load 和 adapter/provider/app binary/spec probe | service adapter、external provider、`autoDownloadEdgeDriver=false`、Linux binary path 和 ordinary spec 均正确 |
| LNX-NODE-01 | unit/integration | `PASS` | `npm run check`、`npm run test`、`npm run build` | Node check/build 通过；Extension 7/7；Desktop Node test 0/0 |
| LNX-RUST-01 | unit/integration | `PASS` | `cargo fmt --all -- --check`、workspace/all-targets check、`cargo check -p xarchive-desktop --features wdio-e2e --all-targets`、workspace test、strict Clippy | workspace tests 150/150；各 crate/doc-test 无失败；Clippy `-D warnings` 通过 |
| LNX-PY-01 | unit/integration | `PASS` | `.venv/bin/python -m compileall -q sidecar/src sidecar/tests`、`.venv/bin/python -m pytest sidecar/tests -q` | compileall 通过；pytest 10/10 |
| LNX-BUILD-01 | native build | `PASS` | `npm run build:tauri --workspace desktop` | 生成 `target/release/xarchive-desktop`，18,513,752 bytes；SHA-256 `e8fa38e90e834c565c054025dafdf3cc1881ee2085df8e0fa25b1d5c9fa76267` |
| LNX-BUILD-02 | native build | `PASS` | `npm run build:tauri:wdio --workspace desktop` | `wdio-e2e` Linux release build 完成 |
| LNX-BROWSER-01 | browser_e2e | `NOT APPLICABLE` | 检查 `desktop/` scripts/config/capabilities | 当前仓库没有独立 Browser Mode 配置或脚本，未临时创建测试架构 |
| LNX-NATIVE-01 | native_e2e | `BLOCKED_AUTOMATION` | `npm run test:e2e --workspace desktop`，单次低成本尝试 | `webkit2gtk-driver` 缺失；自动安装的 `tauri-driver` 启动 code 1；未进入 spec/session/teardown |

#### Test Summary

```json
{
  "summary": {
    "total": 9,
    "pass": 7,
    "pass_flaky": 0,
    "pass_after_fix": 0,
    "fail_product": 0,
    "fail_test": 0,
    "blocked_env": 0,
    "blocked_automation": 1,
    "skipped_platform": 0,
    "needs_review": 0,
    "not_applicable": 1
  },
  "regression_status": "PASS_WITH_ISSUES",
  "remaining_windows_validation": ["WQ-P1-16", "WQ-P1-17", "Windows-only platform queue"],
  "manual_tests_required": [],
  "development_followups": [],
  "environment_issues": ["Linux webkit2gtk-driver/tauri-driver native startup prerequisite"],
  "flaky_tests": []
}
```

### BLOCKED_ENV / BLOCKED_AUTOMATION blocker recovery analysis (2026-09-15 21:37 +08:00)

本节记录针对当前阻塞项的诊断和低风险修复尝试。Linux source 仍为唯一事实来源；本轮没有修改业务代码、依赖版本或生产 Tauri 配置。诊断期间在 E: 验证副本临时生成的 probe/config 已删除，普通 release artifact 已重新构建。

#### Root-cause classification

| 项目 | 当前状态 | 已确认原因 | 结论 |
|---|---|---|---|
| WQ-P1-16 ordinary native session | `BLOCKED_ENV` | WDIO session 创建前先报 `Chrome instance exited`，随后为 `DevToolsActivePort file doesn't exist`；spec 未执行 | 目前没有产品 DOM/业务断言证据；阻塞点在 Windows WebView2/Tauri native session 启动链 |
| WQ-P1-17 advanced native session | `BLOCKED_ENV` | 同一错误在 advanced 两个 spec 上复现；独立 HTTP probe 也在 45 秒内超时 | plugin API、mock、IPC 和日志断言没有被执行；不能把历史子项 PASS 延伸到本轮 |
| WQ-P1-16/WQ-P1-17 failed-run teardown | `BLOCKED_AUTOMATION` | WDIO 失败后曾留下 tauri-driver/msedgedriver 及监听端口，只能用已核实 PID 手工停止 | 手工恢复环境不等于自动 teardown PASS；需要测试基础设施生命周期修复或上游兼容性修复 |
| GUI/DPI/accessibility fallback | `BLOCKED_AUTOMATION` | Computer Use 返回 `Trusted RPC service is not configured: sky`，native inventory 为 `apps=[]` | 当前工具链没有可操作的 Windows native app target；不能从浏览器 tab 代替 GUI 验收 |

#### Recovery attempts and results

| 尝试 | 实际操作 | 结果 |
|---|---|---|
| Driver executable/version | 本地 `msedgedriver 152.0.4191.66` 启动；`4555/status` 返回 `ready=true` | `PASS`；driver 自身不是当前根因 |
| tauri-driver proxy | `tauri-driver --port 4544 --native-port 4555`；`4544/status` 能透传同版本 driver | `PASS`；代理监听和 native driver 链路可用 |
| Direct app startup | 单独启动当前 `target\\release\\xarchive-desktop.exe`，进程保持存活；随后关闭已核实 PID | `PASS`（仅进程级）；不代表 WebDriver session 或 GUI 验收通过 |
| Fresh WebView2 profile | 设置 `WEBVIEW2_USER_DATA_FOLDER` 指向 E: 新目录后运行最小 session probe | `NOT FIXED`；Tauri artifact 仍使用 identifier 对应的 profile，probe 仍超时 |
| Isolated application identifier | 在 E: 临时配置 `com.tw2tg.xarchive.wdio-validation` 并重建，再运行最小 probe；确认新 profile 被创建 | `NOT FIXED`；session 仍在 45 秒内超时，不能归因于默认 profile 单一冲突；已恢复普通 release artifact |
| Explicit driver path | 同一 driver 临时复制到无空格目录并以 `--native-driver` 显式指定；运行最小 probe | `NOT FIXED`；仍超时；临时 driver 副本已删除 |
| Computer Use recovery | 重新检查 native app inventory | `BLOCKED_AUTOMATION`；`sky` trusted RPC 未配置，无法由仓库命令修复 |

#### Current interpretation

以上结果将 `BLOCKED_ENV` 收敛为“应用可单独启动、Edge WebDriver 可用、tauri-driver 可代理，但 WebDriver 创建 Tauri WebView2 session 失败”。profile 隔离、PATH/driver 选择和 driver proxy 已做过受控排除；仍不能确定是 Windows/WebView2 运行时、tauri-driver 2.0.6 与当前 Tauri artifact 的组合、单实例/启动参数，还是该机器的 native session 条件。Windows Application event log 未发现对应的 XArchive 崩溃事件，因此本轮没有把它升级为 `FAIL_PRODUCT`。

自动清理属于独立的 `BLOCKED_AUTOMATION`：失败路径的 driver 残留已能人工恢复，但没有证据证明正常路径和异常路径都会由 WDIO/service 自动回收。不能用本轮手工 `taskkill` 或 `Stop-Process` 改记为 PASS。

#### Manual recovery guide

在一台允许 native GUI 操作、且没有其他 XArchive 验证进程的 Windows 账户上，按以下顺序执行：

1. 关闭本账户中由本次验证启动的 `xarchive-desktop.exe`、`tauri-driver.exe` 和 `msedgedriver.exe`。先用 `Get-CimInstance Win32_Process` 检查完整路径和命令行，只停止已确认属于本轮的 PID；不要按名称误杀用户已有的 Edge/WebView2/Codex 进程。不要删除 `C:\Users\\<user>\\AppData\\Local\\com.tw2tg.xarchive`，它可能包含应用用户状态。
2. 检查 WebView2 runtime、driver 和工具链版本：`C:\\Program Files (x86)\\Microsoft\\EdgeWebView\\Application\\<version>\\msedgewebview2.exe`、`msedgedriver.exe --version`、`tauri-driver --help`。本轮已确认 `152.0.4191.66` 的 runtime/driver 匹配；若机器版本不同，重新取得同一 WebView2 major/minor/build 的 driver，并确认 `where.exe msedgedriver.exe` 的第一项就是该文件。
3. 在 E: 副本执行普通构建和最小 smoke：

   ```powershell
   Set-Location 'E:\Shiraishi\VSCode Workspace\Tw2Tg'
   npm run build:tauri --workspace desktop
   $env:Path = 'E:\Shiraishi\VSCode Workspace\Tw2Tg\desktop\test-artifacts\msedgedriver\152.0.4191.66;' + $env:Path
   $env:WDIO_APP_BINARY = (Join-Path (Get-Location) 'target\release\xarchive-desktop.exe')
   $env:WDIO_CAPTURE_LOGS = '1'
   $env:WDIO_LOG_DIR = (Join-Path (Get-Location) 'desktop\logs\wdio-manual-retry')
   npm run test:e2e:windows --workspace desktop
   ```

4. 若 session 仍报 `DevToolsActivePort`，不要继续反复重跑；保存 `desktop\\logs`、失败时间、app/driver/tauri-driver 完整命令行和 `ports 4444/4445` 证据，交给 Linux 后续任务调查 `@wdio/tauri-service`/tauri-driver lifecycle、Tauri WebView2 启动参数和单实例行为。只有真正创建 session 后，才按 advanced → automatic teardown → ordinary smoke 顺序重验 WQ-P1-17/WQ-P1-16。
5. 若要完成 GUI/DPI/accessibility 项目，需要先让 Computer Use 的 trusted `sky` RPC 和 native app target 可见，或由人工在前台打开 E: 的 `xarchive-desktop.exe`。在 100%/125%/150% DPI 下检查 dashboard、最小窗口、Tab/Shift+Tab、Enter/Escape、focus-visible、错误提示、屏幕阅读器名称/角色/状态和对比度，并保存截图/录屏；该步骤不能由当前浏览器 tab 替代。

本轮结论：没有可安全确认的 blocker 修复；`WQ-P1-16`、`WQ-P1-17` 继续保持 `WINDOWS_FAIL`/待前置稳定后的重验，GUI/DPI/辅助技术继续 `BLOCKED_AUTOMATION`。需要 Linux 后续处理的是测试基础设施和 Windows native session 条件，不是本轮的业务代码修复。


#### Errors and classification

1. `LNX-NATIVE-01`：`@wdio/tauri-service` 首次提示 `WebKitWebDriver not found`，随后安装的 `tauri-driver` 在启动阶段以 code 1 退出。分类为 `BLOCKED_AUTOMATION`，不是 `FAIL_PRODUCT`；未重试，因为缺失 driver/启动前置是稳定可解释的环境问题，且 Linux native E2E 不影响独立测试结果。
2. 本轮没有发现确定性产品缺陷、测试缺陷或 flaky 用例；没有进行修复、删断言、延长等待或修改业务代码。

#### Windows 必须执行的后续验证

1. 同步当前 dirty Linux working tree 到 `E:\Shiraishi\VSCode Workspace\Tw2Tg`，记录 branch/revision/dirty 状态，并排除依赖、target、日志、secrets 和机器本地配置。
2. 准备可被 `where.exe msedgedriver.exe` 发现的 Edge driver，检查版本与 Edge/WebView2 匹配；调查并消除历史 `DevToolsActivePort file doesn't exist` 和 `uv_os_get_passwd returned ENOMEM`。
3. 执行专用 `wdio-e2e` build 和 advanced WDIO，验证 `window.wdioTauri`、`browser.tauri.execute`、invoke interception、mock/restore、frontend/backend logs、session teardown 和失败退出码。
4. 无论 advanced 成功或失败，检查应用、`tauri-driver`、`msedgedriver` 及 1420/4444/4445/9223 端口自动清理；手动杀进程只能恢复环境，不构成 PASS。
5. 执行普通 release build/smoke，验证 Dashboard DOM、普通 artifact 不含 WDIO guest JS/capability，且无 ACL warning；再按需要执行 Windows-only 路径、WebView2、ACL/reparse/长路径、Native Host/Named Pipe、executor real IPC、真实账号、installer 和 Computer Use 场景。

### Windows 再验证：同步后当前 WDIO driver 前置（2026-09-15 17:20 +08:00）

本节记录本轮在 Linux 最新 working tree 完成再次同步后，针对普通与 advanced WDIO 入口的最新 Windows 验证。Linux 仍是唯一 source of truth；本轮没有将 E: 的依赖、缓存、target、日志、验证产物或用户数据反向同步到 Linux，也没有修改业务代码、依赖版本或 Tauri 生产配置。

#### Validation Environment

- Linux source：/home/shiraishi/VSCode Workspace/Tw2Tg，branch dev，HEAD 0537d32c9b4d2ef71ec508467d75378a767a34e7；working tree dirty，包含用户的 WDIO 配置、service adapter 和文档改动。
- Windows validation copy：E:/Shiraishi/VSCode Workspace/Tw2Tg，无 .git，仅用于 Windows 验证。
- Windows：Windows 11 专业工作站版 Insider Preview 10.0.29667，AMD64；Node v24.19.0、npm 11.17.0、Rust/Cargo 1.98.0、WDIO CLI 9.31.9、Tauri CLI 2.11.4、Edge/WebView2 152.0.4191.66。
- 同步方式：从 Linux source 经 WSL /mnt/e 使用受控 rsync -a --checksum 单向同步；排除 .git、依赖、虚拟环境、Rust target、构建/测试产物、日志、用户数据和 secrets；未使用 delete，E: 本地目录保留。

#### Linux pre-validation and sync

| 项目 | 状态 | 结果 |
|---|---|---|
| Linux Rust fmt/check/feature check/test/clippy | PASS | 当前 HEAD 的 Rust 门禁已有通过证据；未因 Windows 前置问题修改 Rust/业务实现 |
| Linux Node/npm 门禁 | NOT RUN | 现场 command -v node 无输出，node --version 为 bash: node: command not found；npm 来自 /mnt/c/Program Files/nodejs/npm，不是 Linux Node |
| Linux native WebView/WDIO | NOT RUN | 当前 Linux 环境无可用原生 GUI 验收条件 |
| Linux→E 同步与关键文件一致性 | PASS | desktop/wdio.conf.mjs、desktop/scripts/wdio-tauri-service.mjs、wrapper、plugin spec、Cargo/Tauri 配置及验证文档已同步；关键文件 SHA-256 与 Linux source 一致 |
| E: 本地验证环境保留 | PASS | .venv、node_modules、target、validation-artifacts、X-Archive、.codex 等 Windows 本地目录未被删除或反向写入 Linux |

#### Windows Validation Results

| 类别 | 项目 | 状态 | 实际命令/结果摘要 |
|---|---|---|---|
| Build/Toolchain | Node workspace checks | PASS | npm run check、npm run test、npm run build；Extension tests 7/7，Desktop Node tests 0 |
| Build/Toolchain | WDIO/config syntax | PASS | node --check 通过 wdio.conf.mjs、service adapter、两个 wrapper 和 plugin spec |
| Runtime/Integration | Rust static gates | PASS | cargo fmt --all -- --check、workspace/all-targets check、wdio-e2e feature check、strict Clippy 通过 |
| Packaging/Build | Dedicated/ordinary Tauri builds | PASS | npm run build:tauri:wdio --workspace desktop 与 npm run build:tauri --workspace desktop 均通过；专用构建编译 tauri-plugin-wdio 1.4.0 |
| Regression/Automation | Advanced WDIO E2E | FAIL | npm run test:e2e:windows:advanced --workspace desktop；在 onPrepare 先尝试下载 Edge 152.0.4191.66 的 msedgedriver，报“无法连接到远程服务器”，随后 tauri-driver code 1 异常退出；0 个 spec 执行 |
| Regression/Automation | Ordinary WDIO native smoke | FAIL | npm run test:e2e:windows --workspace desktop；同样在 onPrepare 受 driver 下载网络失败影响，随后 tauri-driver code 1 退出；未进入 Dashboard spec |
| Cleanup | 失败路径进程/端口核对 | PASS（诊断清理） | 验证结束后无 tauri-driver.exe、msedgedriver.exe、xarchive-desktop.exe；1420/4444/4445/9223 无监听。该结果不等于 WDIO 自动 teardown 已满足队列验收 |

#### Errors and classification

1. 当前首要阻塞：匹配 Edge driver 下载失败。@wdio/tauri-service 检测到 Edge/WebView2 152.0.4191.66 后尝试自动下载对应 driver，PowerShell 报 Error downloading msedgedriver: 无法连接到远程服务器。这是 Windows 网络/driver 前置问题，不是业务代码失败。
2. 连带阻塞：tauri-driver 启动失败。driver 前置失败后，C:/Users/Shiraishi/.cargo/bin/tauri-driver.exe 在启动阶段以 code 1 退出，WDIO 在 onPrepare 终止，两个入口均没有执行 spec。
3. 先前环境证据仍保留：在使用缓存 driver 使 tauri-driver 进入监听后，ordinary worker 曾复现 uv_os_get_passwd returned ENOMEM；另一次 advanced session creation 曾报 DevToolsActivePort file doesn't exist。本轮因更早的 driver/onPrepare 阻塞，未重新触达这两个阶段，不能宣称 adapter、窗口 DOM 或 plugin API 已重新通过。
4. 非阻塞 warning：npm deprecated/allow-scripts、Tauri MSVC linker stdout warning 和 Node child-process shell deprecation 不影响静态门禁或构建结果。

#### Queue reconciliation

- WQ-P1-16：继续 WINDOWS_FAIL。普通 artifact 的静态隔离和构建通过，但本轮 native smoke 未进入 spec，driver/worker/自动 cleanup 完整条件未满足。
- WQ-P1-17：继续 WINDOWS_FAIL。专用 artifact/build 已通过，但本轮 advanced 未创建 session，不能用历史 plugin 子项结果覆盖当前 native session/teardown 验收。
- 当前没有 WINDOWS_VERIFICATION_BLOCKING；本轮失败属于 Windows driver、Node worker 和 native session 前置，不阻塞 Linux 其他业务开发。

#### Follow-up

1. 在 Windows 安装与 Edge/WebView2 152.0.4191.66 匹配、且可由 where msedgedriver.exe 发现的稳定 driver；不要把 Temp 缓存存在误记为系统前置已满足。
2. 在 driver 前置稳定后，调查并最小化复现 tauri-driver code 1、uv_os_get_passwd returned ENOMEM 与 DevToolsActivePort file doesn't exist，确认无残留 Edge/Tauri session。
3. 重新按 advanced → automatic teardown → ordinary smoke → capability/guest-JS boundary 顺序验证；在真实 session、自动 cleanup 和完整断言通过前，保持 WQ-P1-16/WQ-P1-17 = WINDOWS_FAIL。
4. Linux 端当前没有新的业务配置动作；Linux Node 文档状态应保持 NOT RUN，待真正安装 Linux Node/npm 后再执行 Linux Node 门禁和 WDIO config load/dry-run。

### Windows WDIO 重试：手动 driver 后续验证（2026-09-15 17:38–17:42 +08:00）

针对上次自动下载 msedgedriver 的网络失败，本轮从 Microsoft 官方地址手动下载并解压了 Edge/WebView2 对应的 driver：

- Edge/WebView2：152.0.4191.66
- msedgedriver：152.0.4191.66
- 版本命令输出：Microsoft Edge WebDriver 152.0.4191.66
- 验证副本路径：E:/Shiraishi/VSCode Workspace/Tw2Tg/desktop/test-artifacts/msedgedriver-152.0.4191.66/msedgedriver.exe
- 通过临时 PATH 注入确认 where msedgedriver.exe 可发现；driver 未同步回 Linux source。

#### Retry results

| 项目 | 状态 | 实际命令/结果 |
|---|---|---|
| Advanced WDIO retry | FAIL | 手动 driver 后 tauri-driver 成功监听 4444/4445，WDIO 启动 2 个 workers；两个 worker 均在 spec 前因 uv_os_get_passwd returned ENOMEM 失败，0 passed、2 failed |
| Ordinary WDIO retry | FAIL | 手动 driver 后 tauri-driver 成功监听动态端口 54876/54877；worker 在 spec 前因 uv_os_get_passwd returned ENOMEM 失败，0 passed、1 failed |
| Native driver startup | PASS（前置子项） | 手动 msedgedriver 版本正确，tauri-driver 可启动并报告 ready；不等于 WebView2 session、DOM 或 plugin API PASS |
| Cleanup | PASS（手工恢复） | 失败后核对发现 2 个 tauri-driver 和 2 个 msedgedriver 残留，已按核实 PID 停止；最终无相关进程和监听端口。自动 teardown 仍未满足验收 |

#### New evidence and classification

1. 网络下载阻塞已通过手动下载绕过，证明对应版本 driver 文件可用，且 tauri-driver 能够启动。
2. @wdio/tauri-service 仍尝试自动下载：当前版本的 findMsEdgeDriver 版本解析期待 MSEdgeDriver 文本，而当前 driver 输出为 Microsoft Edge WebDriver 152.0.4191.66；因此 service 没有把 PATH 中的手动 driver 识别为已匹配版本。这是当前测试依赖版本的兼容性问题，未修改 node_modules 或业务代码。
3. 剩余主要阻塞为 Windows Node worker 的 uv_os_get_passwd returned ENOMEM；advanced/ordinary 均未进入真实 spec。WQ-P1-16/WQ-P1-17 继续 WINDOWS_FAIL。
4. 手工停止残留进程只恢复验证环境，不作为自动 cleanup PASS；下一轮仍需确认正常和失败路径自动清理。

#### Follow-up

- 保留手动 driver 作为可复用的 E: 本地验证前置，但不要把 test-artifacts driver 目录提交或同步到 Linux source。
- 评估升级/修复 @wdio/tauri-service 的 Edge driver 版本识别，或在项目配置中提供明确的 autoDownloadEdgeDriver 控制；本轮不直接改依赖或 node_modules。
- 优先解决 Windows Node v24.19.0 的 os.userInfo()/uv_os_get_passwd ENOMEM，再重跑 advanced → teardown → ordinary。

### Windows WDIO revalidation from current dirty Linux source (2026-09-15 19:17–20:18 +08:00)

本节记录本轮对 Linux 最新 working tree 的实际 Windows 验证。Linux 仍是唯一 source of truth；本轮未修改业务代码、依赖版本、Tauri 生产配置，也未将 E: 的依赖、缓存、target、日志、test-artifacts 或用户数据反向同步到 Linux。

#### Validation Environment

- Linux source：`/home/shiraishi/VSCode Workspace/Tw2Tg`，branch `dev`，HEAD `0537d32c9b4d2ef71ec508467d75378a767a34e7`；working tree dirty，包含 `desktop/wdio.conf.mjs`、未跟踪的 `desktop/scripts/wdio-tauri-service.mjs` 以及既有文档/WDIO 修改。验证明确包含这些 dirty changes。
- Windows validation copy：`E:/Shiraishi/VSCode Workspace/Tw2Tg`，无 `.git`，仅作为验证工作副本。
- Windows：Windows 11 专业工作站版 Insider Preview `10.0.29667`，AMD64。
- Runtime/toolchain：Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Python `3.14.7`、Tauri CLI `2.11.4`、Edge/WebView2 `152.0.4191.66`。
- Edge driver：`msedgedriver 152.0.4191.66`，E: 本地副本经 `where.exe` 可发现，SHA-256 为 `9E9B1F048D2CC781DEEE084E6CB6E9F2F3417A33ED45D96CF7C34BE4EB23077B`。WDIO service 本轮仍自动下载同版本 driver 到 Temp，下载本身成功。
- Sync：使用 `rsync -a --checksum` 从 Linux source 单向同步到 `/mnt/e/Shiraishi/VSCode Workspace/Tw2Tg/`，不使用 `--delete`；排除 `.git`、`.venv`、`node_modules`、Rust target、dist/build、`desktop/test-artifacts`、日志、数据库、用户数据和 secrets。关键 WDIO/config/spec/lockfile 的 source/E: SHA-256 一致；E: `.venv`、`node_modules`、`target`、`desktop/test-artifacts`、`validation-artifacts`、`X-Archive`、`aria2` 和 `.codex` 保留。
- Validation date：2026-09-15。

#### Linux pre-validation and sync

| 项目 | 状态 | 证据摘要 |
|---|---|---|
| Linux source status/diff check | `PASS` | branch/HEAD/dirty 状态已记录；`git diff --check` 通过 |
| Linux→E sync and key-file identity | `PASS` | `desktop/wdio.conf.mjs`、service adapter、WDIO specs、专用 Tauri config 和 `package-lock.json` 的 source/E: SHA-256 一致 |
| Windows-local preservation | `PASS` | 未删除或覆盖 E: 依赖、虚拟环境、target、driver、验证产物、用户数据和机器本地目录 |

#### Windows Validation Results

| Scope | Category | Command / working directory | Status | Summary |
|---|---|---|---|---|
| WDIO-W-01 | Build/Toolchain | `npm ci --no-audit --no-fund` / `E:\Shiraishi\VSCode Workspace\Tw2Tg` | `PASS` | 锁定依赖安装完成；仅有 deprecated 与 npm allow-scripts warnings |
| WQ-P0-01 | Build/Toolchain | `npm run check`; `npm run test`; `npm run build` | `PASS` | Vite/Extension 静态检查通过；Extension 7/7，Desktop Node tests 0/0，workspace frontend build 通过 |
| WQ-P0-01 | Sidecar | `.venv\\Scripts\\python.exe -m compileall -q sidecar\\src sidecar\\tests`; `.venv\\Scripts\\python.exe -m pytest sidecar\\tests -q` | `PASS` | compileall 通过；pytest 10 passed |
| WQ-P0-01 | Rust | `cargo fmt --all -- --check`; `cargo check --workspace --all-targets`; `cargo check -p xarchive-desktop --features wdio-e2e --all-targets`; `cargo test --workspace --no-fail-fast`; `cargo clippy --workspace --all-targets -- -D warnings` | `PASS` | workspace tests 150/150；fmt/check/feature check/strict Clippy 通过；MSVC linker stdout warning 不影响结果 |
| WDIO-W-02 | Packaging/Build | `npm run build:tauri:wdio --workspace desktop` | `PASS` | 专用 artifact 生成；`target\\release\\xarchive-desktop.exe` 17,631,744 bytes，SHA-256 `4C221B60508BEF34D1A840F9BEF6E8F2908F87D7ED999D0E7F042107A2B511D6`；专用 dist 含 `__wdio_mocks__`，wdio capability 为 `wdio:default` |
| WDIO-W-05 | Packaging/Build | `npm run build:tauri --workspace desktop` | `PASS` | 普通 artifact 生成；`target\\release\\xarchive-desktop.exe` 17,096,704 bytes，SHA-256 `78C3BBAB92F04A90204E2631282A6AE66FA3C7071ABEAC92FD9DA289B1B5E76C`；显式 `EXIT_CODE=0` |
| WDIO static | Build/Toolchain | `node --check`（wdio config、service adapter、两个 wrapper、两个 spec）；Node ESM config load probe | `PASS` | syntax/config load 通过；service 指向项目 adapter、external provider 和当前 release binary |
| WDIO artifact boundary | Security/Regression | 普通 `default.json`/专用 `wdio.json` 权限探针；普通 `desktop\\dist` marker probe | `PASS` | 普通 capability 不含 `wdio:default`，普通 dist 不含 `wdioTauri`、`@wdio/tauri-plugin`、`__wdio_mocks__` 或 `plugin:wdio`；专用构建含 WDIO guest/mock marker |
| WQ-P1-17 | Runtime/Regression | 设置 `WDIO_APP_BINARY`、`WDIO_ADVANCED=1`、`WDIO_CAPTURE_LOGS=1`、`WDIO_LOG_DIR` 后执行 `npm run test:e2e:windows:advanced --workspace desktop` | `FAIL` | 2 workers/2 spec 均未创建 session；driver 下载成功后 WebDriver 三次尝试均报 `session not created: DevToolsActivePort file doesn't exist`，`0 passed, 2 failed`，`EXIT_CODE=1` |
| WQ-P1-16 | Runtime/Regression | 清理 advanced 变量后执行 `npm run test:e2e:windows --workspace desktop` | `FAIL` | Dashboard spec 未创建 session；三次尝试均报 `session not created: DevToolsActivePort file doesn't exist`，`0 passed, 1 failed`，`EXIT_CODE=1` |
| WDIO-W-04 | Runtime/Cleanup | 失败后检查 `xarchive-desktop`、`tauri-driver`、`msedgedriver` 与 1420/4444/4445/9223/61725/61726 端口 | `FAIL` | advanced/ordinary 失败路径均留下 driver/端口监听；必须按精确 PID 手工停止。手工停止后最终无相关验证进程和已知端口，但不构成自动 teardown PASS |
| GUI fallback | Runtime/Automation | Computer Use inventory；`cua.getState()` / native app discovery | `BLOCKED` | `sky` 返回 `Trusted RPC service is not configured: sky`，inventory `apps=[]`；没有进行 GUI 输入。WebView2/DPI/键盘/屏幕阅读器/视觉验收未执行 |
| Browser Mode | Regression | 当前仓库 Browser Mode 配置/脚本检查 | `NOT APPLICABLE` | 仓库没有独立 Browser Mode 配置；未临时创建测试架构 |
| WQ-P2-01 | Packaging | 当前 `desktop/src-tauri/tauri.conf.json` 检查 | `NOT APPLICABLE` | 当前 `bundle.active=false` 且 `createUpdaterArtifacts=false`；本轮无 installer/signing/updater artifact |

#### Errors and classification

| Step | Error / evidence | Fine-grained classification | Windows-specific | Blocks other validation | Follow-up |
|---|---|---|---|---|---|
| Advanced/ordinary WebDriver session | `session not created: Chrome instance exited`，最终为 `DevToolsActivePort file doesn't exist`；spec 未进入 | `BLOCKED_ENV` / native session prerequisite | yes | Blocks WQ-P1-16、WQ-P1-17 的 DOM/API/IPC 断言 | 调查 Edge/WebView2 native session 启动、临时 user-data-dir、driver/tauri-driver 组合；稳定后重跑 advanced → teardown → ordinary |
| Failed-run cleanup | advanced 后观测到 `tauri-driver` PIDs `48348/51828`、`msedgedriver` PIDs `51640/19136` 与 4444/4445/61725/61726；ordinary 后观测到 `48436/25268` 与 4444/4445 | `BLOCKED_AUTOMATION`（自动 teardown 未满足） | yes | Blocks complete WQ-P1-16/WQ-P1-17 acceptance | 先保存 PID/path/log，再验证正常和失败路径自动清理；`Stop-Process` 仅为恢复环境，不是 PASS 证据 |
| Service diagnostics | `Disk Space: Could not determine disk space`；Node child-process shell deprecation；MSVC linker stdout | warning-only | no/yes | no | 保留记录；不将 warning 误判为产品失败 |
| Advanced plugin log | 日志出现 `Invoke interception via defineProperty failed; mock routing via window.__wdio_mocks__ remains active`，但 session 未建立、spec 未执行 | `NEEDS_REVIEW` after native session recovery | yes | Blocks plugin API conclusion | session 前置恢复后再确认 execute/invoke interception、mock restore、backend log 和 session teardown |
| Log capture | advanced service log：`E:\Shiraishi\VSCode Workspace\Tw2Tg\desktop\logs\wdio-2026-09-15T11-47-52-045Z.log`；ordinary log：`...\\wdio-2026-09-15T12-01-46-314Z.log`（0 bytes）；近期日志 marker probe 未发现 token/cookie/password/secret/authorization/bearer | `PASS` for bounded marker probe, not full privacy acceptance | no | no | 保留日志路径；真实账号/隐私链路仍需独立验证 |
| Computer Use | `Trusted RPC service is not configured: sky`；`cua.getState()` returned no native apps | `BLOCKED_AUTOMATION` | yes | Blocks GUI-only items | 按下方人工步骤在 native app target 可用的 Windows 环境执行 |

#### Not Executed / Blocked

以下项目属于当前 Windows queue，但本轮没有用静态测试或历史结果冒充通过：

| 项目 | 状态 | 原因 |
|---|---|---|
| WQ-P0-02 真实 X/Edge Cookie archive；WQ-P1-04 Credential Manager/Telegram | `BLOCKED` | 缺少受控非个人测试账号、空白 Edge profile、Credential Manager 测试凭据和外部服务授权；本轮不接触真实账号 |
| WQ-P0-03 文件 SQLite 应用级恢复；WQ-P1-14 executor production integration；WQ-P1-15 commit recovery | `NOT RUN` | 当前 native session 无法建立，且本轮 handoff 没有可独立执行的受控 restart/crash/SQLite fixture harness；Rust/storage 单测不能替代 Desktop 文件应用验证 |
| WQ-P0-04 Named Pipe；WQ-P1-02 Native Host browser installation | `NOT RUN` / `BLOCKED` | Windows Named Pipe server、manifest、Registry/浏览器安装前置尚未形成最终 artifact；framing/transport 单测不能替代端到端验证 |
| WQ-P1-01 真实 aria2/DownloadRouter/Job integration | `NOT RUN` | 当前队列仍缺少最终 Sidecar/Job 业务接入、真实 media server 和 403 重新提取场景；本轮只执行其 Rust/Sidecar 单元层回归 |
| WQ-P1-03 GUI rendering/accessibility；DPI、键盘、屏幕阅读器、对比度 | `BLOCKED_AUTOMATION` | native GUI automation target 不可用；Computer Use 没有 apps/window，未执行输入或视觉验收 |
| WQ-P1-05 externalBin/Tray/Autostart/Single Instance | `NOT RUN` | 当前 bundle inactive，未提供 bundled Sidecar、Tray/Autostart 或 installer artifact |
| WQ-P1-12 security boundary；WQ-P1-13 user-data ACL；WQ-P2-02 filesystem stress | `NOT RUN` | 本轮没有可重复的 Windows reparse/junction/ACL/长路径/锁/磁盘压力 harness；静态安全单测和 build 证据不替代目标文件系统验证 |

#### Queue reconciliation and Linux follow-up

- `WQ-P0-01`：本轮 workspace/Sidecar/Rust/build gates 均 `PASS`；保留 queue 的 `WINDOWS_PASS` 基线，但不把 WDIO native session 失败覆盖为 PASS。
- `WQ-P1-16`：更新为 `WINDOWS_FAIL`。普通 artifact build 和静态 WDIO boundary 通过，但 native smoke 未创建 session，且失败路径自动 cleanup 未满足。
- `WQ-P1-17`：更新为 `WINDOWS_FAIL`。专用 artifact build 通过，但 advanced plugin API、日志桥接、mock cleanup 和 session teardown 未能在真实 session 中验收。
- 当前没有 `WINDOWS_VERIFICATION_BLOCKING`；上述问题属于 Windows native session/automation lifecycle，不阻塞 Linux 其他业务开发。
- 需要 Linux 后续处理：调查 `DevToolsActivePort file doesn't exist`/`Chrome instance exited` 的最小复现和 `@wdio/tauri-service`/tauri-driver 生命周期；在不放宽普通 capability、guest-JS 或生产安全边界的前提下修复或调整测试基础设施。若涉及依赖升级或配置变化，先完成 Linux 静态回归，再将 WQ-P1-16/WQ-P1-17 置回 `WINDOWS_VERIFICATION_PENDING` 并重验。
- 不需要 Linux 后续处理的本轮环境项：driver 下载网络问题已不再是当前阻塞（同版本下载成功）；`Disk Space`、Node shell deprecation、MSVC linker stdout 属于 warning。不要通过修改业务代码规避它们。

#### Manual validation fallback for blocked GUI/native scenarios

在具备 native app/GUI automation target 后执行：使用当前普通 `target\\release\\xarchive-desktop.exe` 启动应用，分别设置 100%、125%、150% DPI，验证最小窗口、Dashboard heading/main、Tab/Shift+Tab、Enter/Escape、focus-visible、错误提示、屏幕阅读器角色/名称/状态/焦点和对比度；保存截图/录屏、窗口信息、WebView2/Edge 版本和 frontend/backend logs。若要验证 installer/tray/Native Host，先提供对应 artifact、manifest、Registry/ACL 和专用非个人账号，再执行全新安装/卸载、Named Pipe `\\.\\pipe\\xarchive-v1` request/response、多连接/重连/非法消息/权限拒绝和进程清理。PASS 标准是所有预期状态可见、无敏感数据泄露、正常与失败路径自动清理；FAIL 标准是断言不符、状态丢失、权限绕过、残留进程/端口或需要手工 Stop-Process 才能恢复。

#### Structured summary

```json
{
  "summary": {
    "total": 31,
    "pass": 17,
    "pass_flaky": 0,
    "pass_after_fix": 0,
    "fail_product": 0,
    "fail_test": 0,
    "blocked_env": 3,
    "blocked_automation": 3,
    "skipped_platform": 0,
    "needs_review": 1,
    "not_run": 5,
    "not_applicable": 2
  },
  "regression_status": "FAIL_WITH_ENVIRONMENT_BLOCKERS",
  "remaining_windows_validation": [
    "WQ-P1-16 ordinary native smoke and automatic cleanup",
    "WQ-P1-17 advanced plugin/session/log/mock teardown",
    "WQ-P0-03/WQ-P1-14/WQ-P1-15 application-level file/restart/recovery",
    "WQ-P0-04/WQ-P1-02 Named Pipe and browser installation",
    "WQ-P1-03 GUI/DPI/accessibility",
    "WQ-P0-02/WQ-P1-04 real-account flows"
  ],
  "manual_tests_required": [
    "Native GUI WebView2/DPI/keyboard/accessibility fallback",
    "Native Host/Named Pipe and installer flows when final artifacts exist"
  ],
  "development_followups": [
    "Investigate DevToolsActivePort/Chrome instance exit and automatic driver teardown; do not modify business code in this validation task"
  ],
  "environment_issues": [
    "Windows native session did not start; Computer Use sky service/native app inventory unavailable"
  ],
  "flaky_tests": []
}
```

### Windows 最新验证：Linux `dev` HEAD `cb1e5816bcef7480c46b255782c586682ceab16c`（2026-09-16）

本节是当前最新 Windows 验证快照。Linux/WSL 源项目仍是唯一 source of truth；本轮源项目 branch 为 `dev`，HEAD 为 `cb1e5816bcef7480c46b255782c586682ceab16c`（`feat: portable Windows runtime layout, YAML config, and log management`），working tree clean。验证工作副本为 `E:\Shiraishi\VSCode Workspace\Tw2Tg`，包含该 commit 的代码和文档，没有将 E: 的依赖、缓存、target、driver、日志、验证产物或用户数据反向同步到 Linux。

#### Validation environment and synchronization

- Windows：`Microsoft Windows NT 10.0.29667.0`，AMD64；Edge/WebView2 版本由 WDIO service 检测为 `153.0.4234.32`。
- Node/npm：`v24.19.0` / `11.17.0`；Rust/Cargo：`1.98.0`；Tauri CLI：`2.11.4`；WebdriverIO CLI：`9.31.9`；项目 Python：`3.14.7`，pytest `9.1.1`。
- 同步命令族：`robocopy W:\home\shiraishi\VSCode Workspace\Tw2Tg E:\Shiraishi\VSCode Workspace\Tw2Tg /E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`，排除 `.git`、`.venv`、`node_modules`、`target`、`validation-artifacts`、`X-Archive`、缓存/构建/driver/test-artifacts、日志、数据库和 secrets；不删除目标端 extra 文件。
- Robocopy exit code 为 `3`；`FAILED=0`、`MISMATCH=0`。关键源文件 SHA-256 比对 `24/24` 匹配；E: 本地 `.venv`、`node_modules`、`target`、`validation-artifacts`、`X-Archive`、`aria2`、`desktop\test-artifacts` `7/7` 保留。

#### Validation results

| ID / 项目 | 状态 | 实际命令或证据 | 结论边界 |
|---|---|---|---|
| SYNC-2026-09-16 | `PASS` | 受控 Linux → E: Robocopy；关键文件 SHA-256 `24/24` | E: 对应当前 Linux clean HEAD；本地依赖和验证资料未被覆盖 |
| WIN-NODE-01 | `PASS` | `npm run check`、`npm run test`、`npm run build` | Desktop Vite build 通过；Extension `7/7`，Desktop Node test `0/0` |
| WIN-WDIO-STATIC | `PASS` | 6 个 `node --check`、WDIO ESM config load | service adapter、spec、wrapper、当前 binary/spec/provider 配置均可解析 |
| WIN-RUST-01 | `PASS` | `cargo fmt --all -- --check`、`cargo check --workspace --all-targets`、`cargo check -p xarchive-desktop --features wdio-e2e --all-targets`、strict Clippy | 无编译或 `-D warnings` 错误；仅 MSVC linker stdout warning |
| WIN-RUST-TEST | `PASS` | 项目 `.venv\Scripts\python.exe` + `cargo test --workspace --no-fail-fast` | crate tests 合计 `156 passed, 0 failed`；所有 doc-tests `0 failed` |
| WIN-SIDECAR-01 | `PASS` | 项目 Python `-m compileall -q sidecar/src`、`-m pytest sidecar/tests -q` | `10 passed`；pytest cache 写入权限 warning 不影响测试结果 |
| WIN-SCHEMA-01 | `PASS` | 项目 Python 解析 `shared/protocol-schema/**/*.json` | `7` 个 JSON schema/fixture 成功解析 |
| WIN-TAURI-RELEASE | `PASS` | `npm run build:tauri` | `target\release\xarchive-desktop.exe` 生成；仅 linker stdout warning |
| WIN-TAURI-DEBUG | `PASS` | `npm run dev:tauri`；检查 `xarchive-desktop`、1420、9223；结束后清理 | 进程和端口实际启动；MCP bridge 在 `127.0.0.1:9223` 监听；清理后进程/端口均为 0 |
| PORTABLE-W-01 | `PASS` | `npm run build:portable:windows --workspace desktop` | 便携 artifact 生成；包含 `cache/`、`config/`、`extension/`、`logs/`、`sidecar/` 和 `.exe`；不预创建 `download/`/`telegram/` |
| PORTABLE-W-02-init | `PASS` | 从 `dist-portable\XArchive` 启动最终普通 `.exe` 8 秒后检查并清理 | 进程存活；创建 `config\archive.sqlite3`、`cache/`、`logs/`，日志为 `application runtime initialized`；清理后无残留进程 |
| WQ-P1-17-advanced-spec | `PASS` | `npm run test:e2e:windows:advanced --workspace desktop`（专用 wdio build） | WebView2 native session 建立；Dashboard `2/2`、plugin `2/2`，`window.wdioTauri`、`browser.tauri.execute`、mock/restore 通过，exit `0` |
| WQ-P1-17-advanced-cleanup | `FAIL` | advanced 成功退出后检查进程/端口 | `tauri-driver` PID `2796`、`msedgedriver` PID `34932` 仍存在，4444/4445 仍监听；只按精确 PID 人工清理后恢复，不计为自动 teardown PASS |
| WQ-P1-16-ordinary-spec | `PASS` | 恢复普通 `npm run build:tauri` 后 `npm run test:e2e:windows --workspace desktop` | native Dashboard `2/2` 通过，exit `0`；无 session/DOM 失败 |
| WQ-P1-16-ordinary-cleanup | `FAIL` | ordinary 成功退出后检查进程/端口 | `tauri-driver` PID `36640`、`msedgedriver` PID `61008` 仍存在，4444/4445 仍监听；人工清理后恢复 |
| WDIO-log-marker | `PASS`（有限范围） | 扫描本轮 advanced service log `wdio-2026-09-16T02-08-41-343Z.log` | 未命中 token/cookie/password/secret/authorization/bearer；不是完整真实账号隐私验收，ordinary log 为 0 bytes |
| WQ-P1-18-interactive-setup | `NOT RUN` | 便携首启 GUI 目录选择、`config.yaml` 持久化、`download/` 创建/fallback、跨卷 staging→final | 缺少可重复的下载目录 fixture/交互步骤；本轮只完成进程/SQLite/log 初始化 smoke |
| WQ-P1-19-log-rotation | `NOT RUN` | Release/Debug 日志等级、YAML/GUI 覆盖、`max_files` 轮转和只读目录 | 未执行专用 config/轮转 fixture；不能以单次启动日志替代 |
| WQ-P2-01-installer | `NOT APPLICABLE` | `desktop/src-tauri/tauri.conf.json`：`bundle.active=false` | 当前 revision 不生成 installer/signing/updater artifact |
| Browser Mode | `NOT APPLICABLE` | 仓库没有独立 Browser Mode 配置或脚本 | 未临时创建测试架构 |

#### Errors and classification

1. **WDIO/service 自动清理失败（`FAIL`，测试基础设施/生命周期）**：两套原生 spec 都已成功建立 session 并通过断言，但 onComplete 输出 `Stopping 1 driver(s)...` 后仍留下 `tauri-driver`、`msedgedriver` 和 4444/4445。手工 `Stop-Process` 只用于恢复后续验证环境，不是 PASS 证据。没有产品 DOM、IPC 或业务逻辑失败证据；WQ-P1-16/WQ-P1-17 整体继续 `WINDOWS_FAIL`。
2. **Edge driver 版本变化（非失败）**：WDIO service 本轮检测到 WebView2 `153.0.4234.32`，自动下载同版本 driver 成功；E: 保存的 `152.0.4191.66` driver 不能作为当前版本匹配证据，但没有阻塞本轮 spec。
3. **非阻塞 warning**：WDIO 诊断无法确定磁盘空间；Rust/MSVC linker 输出 warning；Sidecar pytest 无法写入既有 `sidecar\.pytest_cache`。这些均未导致验证退出失败。
4. **系统 Python 前置差异**：系统 `python -m pytest` 缺少 pytest；使用项目 `.venv` 后 Sidecar compileall/pytest 通过。分类为环境前置，不是项目代码失败。
5. **GUI/辅助技术自动化边界（`BLOCKED`）**：Computer Use helper 当前仍返回 `helper_unknown_error: setup refresh had errors`，因此 DPI、键盘焦点、屏幕阅读器、原生文件选择器、托盘/通知等不由本轮 WDIO DOM smoke 替代验收。

#### Not executed / blocked inventory

- `BLOCKED`：GUI/DPI/键盘/屏幕阅读器/原生文件选择器/托盘等，需要可用 Computer Use/native accessibility target。
- `NOT RUN`：真实 Edge/X Cookie archive、真实 Telegram/Credential Manager、Native Host/Named Pipe/Registry、aria2 业务级 fallback/403 refresh、executor real-worker/restart/recovery、应用级 SQLite migration/restart、ACL/reparse/长路径、externalBin/Tray/Autostart、filesystem stress、便携 interactive setup、log rotation；分别缺少测试账号/外部服务、最终 artifact、fixture、第二用户或可重复交互路径。
- `NOT APPLICABLE`：当前 `bundle.active=false` 的 installer/signing/updater；独立 Browser Mode。

#### Linux follow-up required

- 处理 WDIO service/tauri-driver 生命周期：成功和失败路径都必须自动关闭 tauri-driver、msedgedriver、应用进程及 4444/4445 端口；不得用手工杀进程掩盖问题。完成后先做 Linux 静态/相关回归，再重验 WQ-P1-16/WQ-P1-17。
- 保持普通 release 的 capability/guest-JS 隔离；本轮没有发现需要修改业务 Rust、前端业务逻辑、生产 Tauri capability 或依赖版本的问题。
- 为 WQ-P1-18/WQ-P1-19 提供受控 Windows 交互/fixture 后，再验证 `config.yaml`、下载目录 fallback、跨卷提交、日志等级与轮转；在此之前保持 `WINDOWS_VERIFICATION_PENDING`。
- 为真实账号、Named Pipe、ACL/reparse/长路径、应用级 recovery、externalBin 和发布包补齐前置后再执行对应队列项；本轮不将静态、单测或 DOM smoke 外推为这些项目的 PASS。

### Windows 最新验证：Linux `dev` HEAD `a20027455651ef5f4f9faed527948bc1830375a6`（2026-09-16）

本节记录 Linux 最新 clean 状态在 Windows 验证副本上的实际结果。验证任务不修改业务代码；Linux 源项目仍是唯一 source of truth。

#### Validation environment and synchronization

- Linux source：branch `dev`，HEAD `a20027455651ef5f4f9faed527948bc1830375a6`，提交前 working tree clean；提交信息为 `fix: enforce WDIO driver process-tree teardown and record 2026-09-16 Windows results`。
- Windows：Windows 10 Pro for Workstations，x64，Windows build 2009；WDIO service 检测 Edge/WebView2 `153.0.4234.32`。
- Toolchain：Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0` MSVC、Tauri CLI `2.11.4`、WDIO CLI `9.31.9`、项目 Python `3.14.7`、pytest `9.1.1`。
- Windows worktree：`E:\Shiraishi\VSCode Workspace\Tw2Tg`。
- 使用 Linux → E: 的受控 `robocopy /E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT` 同步；排除 `.git`、`.venv`、`node_modules`、`target`、验证产物、用户数据、缓存、日志、数据库和 test artifacts，且不删除目标端 extra。Robocopy exit `3`、`FAILED=0`、`MISMATCH=0`；关键源码/测试/文档 SHA-256 与 Linux 源一致。

#### Validation results

| ID / 项目 | 状态 | 实际命令或证据 | 结果边界 |
|---|---|---|---|
| SYNC-2026-09-16 | `PASS` | 受控 Linux → E: Robocopy；关键文件哈希一致 | 验证副本对应当前 Linux clean HEAD；本地依赖、缓存和用户验证资料保留 |
| WIN-TOOLCHAIN-01 | `PASS` | Windows/Node/npm/Rust/Python/Tauri/WDIO 版本探针 | 项目前置工具可用；WDIO 自动下载匹配的 Edge driver 成功 |
| WIN-NODE-CHECK | `PASS` | `npm run check`；各 WDIO 脚本 `node --check` | Vite 和 Extension 语法通过；WDIO service/config/wrapper 可解析 |
| WIN-NODE-TEST | `FAIL` | `npm run test`；单独 `node --test desktop/test/wdio-tauri-service.test.mjs` | Extension `7/7` 通过；Desktop 新增测试 `7 passed, 1 failed`，失败为 `killTree` 子进程终止测试 |
| WIN-RUST-01 | `PASS` | `cargo fmt --all -- --check`；workspace/all-targets check；`wdio-e2e` feature check；strict Clippy | 编译和 `-D warnings` 通过；仅 MSVC linker stdout warning |
| WIN-RUST-TEST | `PASS` | `.venv\Scripts\python.exe` + `cargo test --workspace --no-fail-fast` | `156 passed, 0 failed`；doc-tests 全部通过 |
| WIN-SIDECAR-01 | `PASS` | `.venv\Scripts\python.exe -m compileall -q sidecar`；pytest `sidecar/tests -q --basetemp E:\Tw2Tg-pytest-temp` | `10 passed` |
| WIN-SCHEMA-01 | `PASS` | 6 个 schema JSON 与 3 个 JSONL fixture 文件解析 | JSON schema 全部可解析；JSONL records `3+1+4` 全部可解析 |
| WIN-TAURI-WDIO-BUILD | `PASS` | `npm run build:tauri:wdio --workspace desktop` | 专用 WDIO release artifact 生成 |
| WIN-TAURI-RELEASE | `PASS` | `npm run build:tauri --workspace desktop` | ordinary `target\release\xarchive-desktop.exe` 生成 |
| WIN-TAURI-ORDINARY-BOUNDARY | `PASS` | ordinary `desktop\dist` 扫描 | 未发现 `wdioTauri`、`__wdio_mocks__`、`plugin:wdio` 等 WDIO-only 标记 |
| PORTABLE-W-01/02 | `PASS` | `npm run build:portable:windows --workspace desktop`；启动 portable exe 8 秒后检查 | exe 启动；创建 `config\archive.sqlite3`、同级 `logs\xarchive-*.log`；未预创建 `download` 符合脚本约定；本次进程树清理后无残留 |
| WQ-P1-17 advanced spec | `PASS` | `npm run test:e2e:windows:advanced --workspace desktop` | 2 spec、4 tests 全部通过；native session、Dashboard、plugin API、execute、mock/restore 通过 |
| WQ-P1-17 advanced cleanup | `FAIL` | 同一 advanced run 的 `onComplete` | tracked survivors `23148, 40748`；hook 报 PID `23148` 未在 5 秒内确认退出，不能以最终环境已恢复代替自动 cleanup PASS |
| WQ-P1-16 ordinary spec | `PASS` | `npm run test:e2e:windows --workspace desktop` | 1 spec、2 tests 全部通过；native Dashboard session 成功 |
| WQ-P1-16 ordinary cleanup | `FAIL` | 同一 ordinary run 的 `onComplete` | tracked survivors `49032, 45032`；hook 报 PID `45032` 未在 5 秒内确认退出 |
| WQ-P1-18 interactive setup | `NOT RUN` | 首次下载目录选择、`config.yaml` 持久化、download fallback、跨卷提交 | 缺少可重复的交互/文件系统 fixture；本轮只做 portable 进程/SQLite/log smoke |
| WQ-P1-19 log rotation | `NOT RUN` | 日志等级、YAML/GUI 覆盖、max_files、只读目录 | 未提供专用 config/轮转 fixture；单次启动日志不能替代 |
| GUI/DPI/accessibility | `BLOCKED` | WebView2 DPI、键盘、屏幕阅读器、原生选择器 | 当前无可用 native GUI automation target；WDIO DOM smoke 不外推为 GUI 验收 |
| Real account / Telegram / Credential Manager | `BLOCKED` | 真实 Edge/X Cookie、Telegram 和 Windows secret flow | 缺少专用非个人账号、凭据和外部服务授权 |
| Named Pipe / Native Host / Registry | `NOT RUN` | `\\.\pipe\xarchive-v1`、manifest、Registry 安装 | 当前未提供最终 Native Host/manifest/installer artifact |
| Application-level recovery / ACL / reparse / stress | `NOT RUN` | Windows 文件 SQLite 重启恢复、权限和压力场景 | 缺少受控 crash/lock/ACL/reparse fixture；单元测试不替代 |
| Installer/signing/updater/Browser mode | `NOT APPLICABLE` | `bundle.active=false`；无独立 Browser Mode 配置 | 当前 revision 不生成这些 artifact |

#### Errors and classification

1. **Windows WDIO cleanup：`FAIL`，测试基础设施/生命周期问题。** advanced 和 ordinary 的业务/DOM 断言均通过，但 `onComplete` 在上游 teardown 后发现两个 tracked driver PID；执行 `taskkill /T /F` 后仍有一个 PID 在 5 秒确认窗口内被 `isPidAlive` 判定存活，hook 抛出 `failed to tree-kill driver process(es) after teardown`。两次命令外层最终退出码显示为 `0`，但 hook 错误仍按项目规范记为 FAIL。退出后立即复查已无 `tauri-driver`、`msedgedriver`、`xarchive-desktop` 和 4444/4445/1420/9223 LISTEN；这是环境最终恢复证据，不是自动 teardown 成功证据。
2. **新增 `killTree` 单测：`FAIL`，Windows 测试/实现边界待查。** `node --test desktop/test/wdio-tauri-service.test.mjs` 为 `7 passed, 1 failed`，失败位于 `test/wdio-tauri-service.test.mjs:72` 的 spawned child termination 断言，约 5.3 秒后 `false !== true`；未发现测试子进程残留。该结果不足以证明 Linux 实现已可靠覆盖 Windows taskkill 的时序/进程状态语义。
3. **非阻塞 warning：** WDIO 诊断无法确定磁盘空间；WDIO/Node 报 shell 参数 deprecation；MSVC linker 输出 warning；这些未导致其它验证退出失败。

#### Not executed / blocked inventory

- `BLOCKED`：真实账号、Credential Manager、Telegram、GUI/DPI/键盘/屏幕阅读器/原生文件选择器。
- `NOT RUN`：Native Host/Named Pipe/Registry、应用级 SQLite/restart/recovery、aria2 业务级 fallback、ACL/reparse/长路径/压力、portable interactive setup、log rotation、externalBin/Tray/Autostart。
- `NOT APPLICABLE`：当前 `bundle.active=false` 的 installer/signing/updater；仓库未定义独立 Browser Mode。

#### Linux follow-up required

- 处理 `desktop/scripts/wdio-tauri-service.mjs` 的 Windows 进程清理判定：复现 `killTree` 单测和两种 WDIO `onComplete` 的 survivor PID 行为，明确 PID 快照、`taskkill /T /F`、进程退出确认和 PID reuse/race 的边界；修复后先做 Linux 相关回归，再重验 WQ-P1-16/WQ-P1-17。验证阶段未修改业务代码。
- 保持 ordinary release capability/guest-JS 隔离；本轮未发现需要修改业务 Rust、前端业务逻辑、生产 capability 或依赖版本的问题。
- 为 WQ-P1-18/WQ-P1-19、Native Host/Named Pipe、真实账号、应用级 recovery 和 GUI 验收补齐专用 Windows 前置后再执行；在此之前继续保持相应 `BLOCKED`/`NOT RUN`/`WINDOWS_VERIFICATION_PENDING`，不把单测或 DOM smoke 外推为完整通过。

### Windows validation after Sidecar unknown-field hardening (2026-09-16)

本轮以 Linux 源项目最新 clean revision `fe185a262258cedbde78e481de479a69848caf11`（`dev`，ahead of `origin/dev` 1）执行增量 Windows 验证。该 revision 只新增 `SidecarCommand` 的 `serde(deny_unknown_fields)` 和 Rust targeted regression；因此本轮范围收敛为 WQ-P1-12 的跨层 Sidecar command contract，不重复执行与当前 diff 无交集的 WDIO、便携交互、GUI、真实账号、Native Host、应用级 recovery 和 installer 项目。

#### Validation environment and synchronization

- Windows: Microsoft Windows 11 Insider Preview `10.0.29667`, AMD64.
- Node/npm: `v24.19.0` / `11.17.0`; Rust/Cargo: `1.98.0`; Python: `3.14.7`; project Python: `E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe`.
- Linux source: `/home/shiraishi/VSCode Workspace/Tw2Tg`; branch `dev`; HEAD `fe185a262258cedbde78e481de479a69848caf11`; working tree clean, ahead of `origin/dev` by 1 committed revision.
- Windows worktree: `E:\Shiraishi\VSCode Workspace\Tw2Tg`.
- Used one-way `robocopy W:\home\shiraishi\VSCode Workspace\Tw2Tg E:\Shiraishi\VSCode Workspace\Tw2Tg /E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT /R:1 /W:1`, excluding `.git`, `.venv`, `node_modules`, `target`, validation/user data, logs, databases, build/test artifacts and machine-local directories; target extras were not deleted. Robocopy exit `3`, `FAILED=0`, `MISMATCH=0`; 8 key source/target SHA-256 pairs matched; target `.venv`, `node_modules`, `target`, `validation-artifacts`, `X-Archive`, `aria2` and `desktop\test-artifacts` were preserved.

#### Validation scope and results

| ID / 项目 | 状态 | Actual command or evidence | Boundary |
|---|---|---|---|
| SYNC-2026-09-16-PROTOCOL | `PASS` | Controlled Linux -> E: Robocopy; all key SHA-256 pairs matched | Copy corresponds to Linux clean HEAD; local dependencies/validation data were not overwritten |
| WIN-WQ-P1-12-PROTOCOL | `PASS` | `cargo test -p xarchive-protocol --no-fail-fast` | `11 passed, 0 failed`, including unknown `executable` rejection regression |
| WIN-WQ-P1-12-SIDECAR-SUPERVISOR | `PASS` | `$env:PYTHON=.venv\Scripts\python.exe; cargo test -p xarchive-sidecar-supervisor --no-fail-fast` | `4 passed, 0 failed`; real Python worker handshake passed |
| WIN-WQ-P1-12-DESKTOP-CONSUMER | `PASS` | `cargo test -p xarchive-desktop --no-fail-fast` | `70 passed, 0 failed` |
| WIN-WQ-P1-12-FMT | `PASS` | `cargo fmt --all -- --check` | Passed |
| WIN-WQ-P1-12-CLIPPY | `PASS` | `cargo clippy -p xarchive-protocol -p xarchive-sidecar-supervisor -p xarchive-desktop --all-targets -- -D warnings` | Passed; only non-blocking MSVC linker stdout warning |
| WIN-SIDECAR-PYTEST | `PASS` | `.venv\Scripts\python.exe -m pytest sidecar/tests -q --basetemp E:\Tw2Tg-pytest-temp` | `10 passed` |
| WIN-WQ-P1-12-PYTHON-UNKNOWN-FIELD | `FAIL` | Sent `hello` JSONL containing `executable` to `.venv\Scripts\python.exe -m xarchive_downloader`, then `shutdown` | Worker returned `{"event":"ready"...}` and exit `0`; schema-forbidden field was not rejected. WQ-P1-12 remains `WINDOWS_FAIL` |
| WQ-P1-12-path/reparse | `NOT RUN` | Windows path permissions, ordinary files, symlink/junction/reparse, long JSON and real Sidecar download | No controlled reparse/permission fixture; unit/protocol tests cannot substitute for application-level Windows acceptance |

#### Errors and classification

1. **Python Sidecar did not reject unknown fields: `FAIL` / `FAIL_PRODUCT_NEEDS_DEVELOPMENT`.** With schema-forbidden `executable`, `sidecar/src/xarchive_downloader/__init__.py` `run_worker`/`handle_command` parsed a dict and read only needed keys, returning `ready` instead of enforcing `additionalProperties: false`. This is a cross-platform Python consumer security-contract gap confirmed by the Windows worker probe, not a Windows filesystem or environment false positive; it does not block unrelated Linux development.
2. **Initial supervisor attempt: `BLOCKED_ENV`, resolved by the documented prerequisite.** Without `PYTHON`, the test reported `NotRunning` (2/4); with the project `.venv` interpreter, `4/4` passed. The final supervisor result is `PASS`, while the environment diagnostic is retained here.
3. **Non-blocking warning:** MSVC linker stdout warning; no test exit code or assertion was affected.

#### Not executed / blocked inventory

- `FAIL`: WQ-P1-12 Python consumer unknown-field rejection; Rust consumer, direct Rust consumers, strict Clippy and Sidecar tests passed.
- `NOT RUN`: WQ-P1-12 real Sidecar download, Windows archive-root/permission, symlink/junction/reparse, long-path/long-JSON and application-level path commit; controlled fixtures were unavailable, so protocol tests were not extrapolated.
- Existing items with no intersection with the current diff retain their historical states: WDIO WQ-P1-16/WQ-P1-17 cleanup, portable interactive setup/log rotation, GUI/DPI/accessibility, real account/Telegram/Credential Manager, Native Host/Named Pipe/Registry, executor/restart/recovery and installer were not re-run and are not changed by this result.

#### Linux follow-up required

- Linux follow-up completed the Python consumer fix: `run_worker` now rejects unknown command fields, including `executable`, before dispatch and has a direct regression test. WQ-P1-12 is returned to `WINDOWS_VERIFICATION_PENDING`; rerun the Python worker unknown-field and valid-command checks plus controlled Windows path/reparse fixtures before any `WINDOWS_PASS` decision.
- This Linux follow-up changed only the Python Sidecar consumer, its direct regression test and validation records; no Windows capability, frontend behavior or dependency version was changed.

### Linux Plan reconciliation after the latest Windows result (2026-09-16)

- 最新 Windows 结果只对 `fe185a2` 的 WQ-P1-12 增量范围产生新事实：Python worker 未拒绝未知 `executable` 字段；该缺口已在 Linux `bc7f613` 修复，并通过 worker targeted pytest、Sidecar 全部 pytest、compileall、Rust protocol/supervisor/Desktop 直接消费者测试和 fmt 验证。
- 当前没有新的 Linux 业务代码 FAIL。Desktop 已在 Linux/Unix 上注册 Unix domain socket transport endpoint，Native Host 在 Linux 上改用 `UnixStream::connect` 连接；`desktop/src-tauri/src/transport.rs` 已成为 Desktop 生产 endpoint 注册的一部分，不再是仅作为 contract adapter。Native Host 在 Linux/Unix 上通过配置的 `XARCHIVE_PIPE_ENDPOINT` 使用 Unix stream 连接 Desktop，Windows 下仍保留 `OpenOptions` 文件打开路径，Desktop 不注册 Named Pipe server。保留同步 `archive_tweet` fallback。
- R2 仍未完成。`ArchiveExecutionContext::download_sidecar` 当前只通过 `DownloadRouter` 包裹 gallery-dl 执行并统一失败映射，尚未提供 fresh media URL、aria2 backend 注入、transfer polling/completion 或 403 重新提取。直接接线会违反 roadmap 完成标准，因此本轮不修改业务代码。
- 按增量策略，本轮 Linux 验证范围为 Python worker、Sidecar 直接消费者、Rust protocol/supervisor/Desktop 直接消费者、fmt、compileall、Schema parse 和文档链接/diff 检查；未运行无交集的 WDIO、GUI、真实账号、installer 或 Windows full suite。
- 当前队列事实：WQ-P1-01、WQ-P1-12、WQ-P1-14、WQ-P1-15、WQ-P1-16、WQ-P1-17、WQ-P1-18 和 WQ-P1-19 均等待各自 Windows 重验，保持 `WINDOWS_VERIFICATION_PENDING`；历史 Windows FAIL 章节继续保留，不代表本轮已通过。

#### BLOCKED / NOT RUN handoff

若进入 Windows 阶段仍缺少 endpoint、受控 Sidecar/media fixture、旧库/文件锁/reparse harness、第二用户、账号或 GUI automation target，则跳过对应项目并记录 `BLOCKED` 或 `NOT RUN`。手工步骤使用 `../validation/windows-queue.md` 的“当前 BLOCKED / NOT RUN 手工验证步骤”，不以 Linux contract、fake transport 或静态检查替代 Windows 结论。

### Windows validation of latest Linux working tree（2026-09-16 22:14–22:48 +08:00）

#### Validation environment and synchronization

- Linux source: `dev`, HEAD `ec58a200eef3f19e9ae89419b5b605ecc4ff5fad`, working tree dirty with 11 modified tracked files (including GUI, portable/config/commands, Native Host entry and validation docs); no active app Plan file or goal was present, so `roadmap.md`, `status.md`, the Windows queue and current handoff were used as the plan/state sources.
- Windows: Windows 11 Pro for Workstations Insider Preview `10.0.29667`, x64; Edge `154.0.4258.18`; Node `v24.19.0`; npm `11.17.0`; Rust/cargo `1.98.0`; usable validation Python `3.12.14`; worktree `E:\Shiraishi\VSCode Workspace\Tw2Tg`.
- One-way sync completed from `W:\home\shiraishi\VSCode Workspace\Tw2Tg` to E: with dependencies, caches, target, user data, logs, driver and test artifacts excluded; Robocopy final exit `3`, `FAILED=0`, `MISMATCH=0`, 8 key SHA-256 pairs matched. The requested alias `Tw2Tg-CodexAlias` is a junction to the real E: path; running Vite from the alias first produced a path-resolution error, while the real path passed.
- Sync audit: the first exclusion attempt mistakenly used target absolute paths, briefly copied an accidental target `.git` and touched the existing target `.venv`; the accidental `.git` was removed before validation and the corrected sync preserved local directories. The target `.venv` remains invalid as a Windows environment (`/usr/bin\\python.exe`), so an isolated `.venv-windows-validation` was created without overwriting it.

#### Validation results

| ID / 项目 | 状态 | Command / evidence |
|---|---|---|
| SYNC-WT-2026-09-16 | `PASS` | Corrected one-way Robocopy; 8 key SHA-256 pairs matched; target local extras retained |
| WIN-NODE-CHECK | `PASS` | `npm run check` from real E: path; Vite and Extension checks passed |
| WIN-NODE-TEST | `PASS` | `npm test`; Desktop 8/8 and Extension 7/7 |
| WIN-NODE-BUILD | `PASS` | `npm run build`; Vite and Extension build passed |
| WIN-RUST-FMT | `PASS` | `cargo fmt --all -- --check` |
| WIN-RUST-CHECK | `PASS` | `cargo check --workspace --all-targets`; only existing `runtime.rs:77 unused_mut` warning |
| WIN-RUST-TEST | `PASS` | `$env:PYTHON=<usable Windows Python>; cargo test --workspace --no-fail-fast`; 158 passed, 0 failed |
| WIN-RUST-CLIPPY | `FAIL` | `cargo clippy --workspace --all-targets -- -D warnings`; existing `desktop/src-tauri/src/runtime.rs:77` `unused_mut` promoted to error |
| WIN-SIDECAR | `PASS` | Isolated venv: `compileall` and `pytest sidecar/tests -q`; 11 passed |
| WIN-TAURI-RELEASE | `PASS` | `npm run build:tauri --workspace desktop`; release exe generated |
| WIN-TAURI-WDIO-BUILD | `PASS` | `npm run build:tauri:wdio --workspace desktop`; dedicated artifact generated |
| WIN-WDIO-ADVANCED | `PASS` | `npm run test:e2e:windows:advanced --workspace desktop`; Dashboard 2/2, plugin API/execute 1/1, mock/restore 1/1, exit 0 |
| WIN-WDIO-ORDINARY | `PASS` | `npm run test:e2e:windows --workspace desktop`; Dashboard 2/2, exit 0 |
| WIN-WDIO-TEARDOWN | `PASS with safety-net evidence` | Each run required safety-net tree-kill for 2 survivors; post-run `tauri-driver`/`msedgedriver` absent and 4444/4445 had no listeners |
| WIN-PORTABLE-BUILD | `PASS` | `npm run build:portable:windows --workspace desktop`; exe 17,460,736 bytes, Extension files present |
| WIN-PORTABLE-START | `PASS` | Fresh portable exe stayed alive for 6 seconds and created `config/archive.sqlite3`; process then cleaned up |
| WIN-NODE-SYNTAX | `PASS` | Node syntax checks for WDIO wrappers/specs/config; config load succeeded |

#### Errors and classification

1. `FAIL` — strict workspace Clippy rejects existing `desktop/src-tauri/src/runtime.rs:77` `let mut state`; not part of the current Linux diff. Classification: existing project lint debt, not Windows-specific and not caused by this GUI/portable batch. Linux follow-up should remove the unnecessary mutability or otherwise reconcile the lint; do not alter it during this validation.
2. `BLOCKED_ENV` then resolved — the existing target `.venv` points to `/usr/bin\\python.exe`, causing the first Rust workspace run's two Sidecar handshake tests to return `NotRunning`. With the supported `PYTHON` override and isolated Windows validation venv, the targeted and full Rust tests passed. Keep the broken `.venv` as a machine/setup issue until its ownership is decided.
3. `BLOCKED/FAIL` boundary — GUI settings/DPI/keyboard/accessibility and Extension missing-file interaction could not be exercised because the local GUI automation/native target was unavailable. No product failure is inferred from this absence.
4. `NEEDS_REVIEW` — `build-portable-windows.mjs` explicitly pre-creates `config`, `cache`, `logs` and `sidecar`, while the older handoff text says these should not exist before first launch. The observed fresh output follows the script. Align the acceptance wording/documentation with the intended product contract in a Linux follow-up.
5. `PASS with warning` — WDIO service upstream teardown left two driver survivors in each run; the project safety-net tree-killed them, and final process/port checks were clean. This is not a product DOM failure, but future teardown diagnostics should explain why upstream cleanup still misses the tree.

6. `BLOCKED_ENV` then resolved — `npm ci --ignore-scripts` hit `ENOTEMPTY` while removing an existing `node_modules\\mocha\\node_modules\\yargs\\locales`; the subsequent `npm install --ignore-scripts --no-audit --no-fund` completed and all Node checks/tests/builds passed. This is a validation-workspace dependency-cache issue, not a product failure.

#### Not executed / blocked / not applicable

- `BLOCKED_AUTOMATION`: GUI settings/Extension flow, 100/125/150% DPI, keyboard/focus, screen reader/contrast and native file/folder picker interactions; the CUA/native target was unavailable.
- `BLOCKED`: real X/Edge Cookie, Telegram, Credential Manager and external network/account flows; no controlled accounts or credentials were supplied.
- `NOT RUN`: Windows Named Pipe/ACL, Native Host manifest/Registry/browser installation, real executor transport/restart/recovery, SQLite migration/reparse/ACL/long-path fixtures, real aria2 business integration, cross-volume commit, sidecar artifact installation and installer/signing/updater/Tray/Autostart. Required endpoints, fixtures or release artifacts are not present.
- `NOT APPLICABLE`: Browser Mode has no independent configuration in the current repository; installer validation is not applicable to the current bundle-disabled release scope, though future packaging remains `NOT RUN`.

#### Queue reconciliation and Linux follow-up

- WQ-P0-01 is `WINDOWS_FAIL` because strict Clippy did not pass; fix or explicitly reconcile `runtime.rs:77` in a separate Linux task, then rerun the Windows baseline.
- WQ-P1-16 and WQ-P1-17 are `WINDOWS_PASS` for this dirty working-tree validation, with the safety-net warning and clean post-run ports retained as evidence.
- WQ-P1-18 and WQ-P1-19 remain `WINDOWS_VERIFICATION_PENDING`; complete interactive setup, `config.yaml` persistence, Downloads fallback, cross-volume commit, sidecar/Extension distribution, log levels/rotation and read-only permissions on a controlled Windows fixture.
- New GUI settings/visual/accessibility and Extension native-loading queue items remain `WINDOWS_VERIFICATION_PENDING`/`BLOCKED_AUTOMATION`; do not infer them from Dashboard smoke.

### Linux reconciliation after 2026-09-16 22:14 working-tree validation（2026-09-17）

Previous Windows validation:

- `WQ-P0-01`: `WINDOWS_FAIL` — strict workspace Clippy failed only at pre-existing `desktop/src-tauri/src/runtime.rs:77` `unused_mut`; all other baselines passed.
- `WQ-P1-16` / `WQ-P1-17`: `WINDOWS_PASS` for the dirty working-tree run, with safety-net warning retained.
- `WQ-P1-18` / `WQ-P1-19` and new GUI/Extension items: `WINDOWS_VERIFICATION_PENDING` / `BLOCKED_AUTOMATION`.

Linux fix:

- Restricted the post-construction mutation of `RuntimeState` to Unix builds only: `let state` remains immutable on Windows, while Unix uses a `#[cfg(unix)]`-gated `let mut state` before assigning `transport_server`. This is a warning-only, behavior-invariant change; Unix transport startup, recovery scan, feature-gated fields and public APIs are unchanged.

Linux verification:

- `PASS`: `cargo fmt --all -- --check`; `cargo clippy -p xarchive-desktop --all-targets -- -D warnings`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test -p xarchive-desktop --all-targets --no-fail-fast` (71 passed); `cargo test -p xarchive-native-host --all-targets --no-fail-fast` (8 passed); `cargo check -p xarchive-desktop --all-targets`; `npm run check --workspace desktop`; `npm run test --workspace desktop` (8/8); `npm run check --workspace extension`; `npm run test --workspace extension` (7/7); `git diff --check`.

Current Windows status:

- `WQ-P0-01`: `WINDOWS_VERIFICATION_PENDING` — history retains the prior `WINDOWS_FAIL`; Linux fixed the Clippy cause locally, but the result must not be promoted to `WINDOWS_PASS` until strict workspace Clippy actually reruns on the fixed revision in Windows.
- `WQ-P1-16` / `WQ-P1-17`: unchanged `WINDOWS_PASS`; current Linux diff does not touch the WDIO service adapter, specs, build scripts, capabilities, GUI entry or plugin registration.
- Other pending/blocked items: unchanged; GUI settings/visual/accessibility, Extension native loading, real accounts/credentials, IPC/filesystem fixtures, installer/packaging and complete portable setup still require Windows evidence or controlled fixtures.

- No business code was modified. The Linux follow-up set is: (a) resolve the existing strict-Clippy lint, (b) reconcile portable directory acceptance wording vs script behavior, (c) provide a GUI/native automation target and controlled fixtures for remaining portable/security/IPC/credential checks, and (d) rerun WQ-P0-01 after (a).

### Windows validation of latest Linux working tree（2026-09-17 18:04–18:23 +08:00）

本轮验证 Linux 源项目 dev 分支 HEAD a5f42ccc4b6d661e3cf80338b44859e5178e8480。working tree dirty：27 个已跟踪文件修改，另有 workflow、portable 脚本、GUI 页面/组件、测试和 PyInstaller spec；验证包含 working-tree changes，不代表纯 commit。Plan/state 来源为 roadmap.md、status.md、windows-queue.md、本文件及项目 instructions；无独立 active Plan 或 Codex goal。

#### Validation environment and synchronization

- Windows 11 Insider Preview 10.0.29667, AMD64；Edge/WebView2 target 153.0.4234.32。
- Node/npm v24.19.0 / 11.17.0；Rust/Cargo 1.98.0；validation Python 3.12.14；PyInstaller 6.22.3。
- Linux source /home/shiraishi/VSCode Workspace/Tw2Tg；Windows copy E:\Shiraishi\VSCode Workspace\Tw2Tg。
- One-way Robocopy from W: source to E: target，exit 3，FAILED=0，MISMATCH=0；未删除 18 个目标额外目录，.venv*、node_modules、target、日志、测试产物、用户数据和缓存保留。

#### Validation results

| ID / 项目 | 状态 | Actual command or evidence | 说明 |
|---|---|---|---|
| SYNC-WT-2026-09-17 | PASS | Controlled Linux → E: Robocopy | 目标对应 dirty working tree；本地目录保留 |
| WIN-NODE-CHECK | PASS | npm run check | Vite 与 Extension syntax check 通过 |
| WIN-NODE-TEST | PASS | npm test | Desktop 30 passed；Extension 7 passed |
| WIN-NODE-BUILD | PASS | npm run build | Desktop/Extension build 通过 |
| WIN-RUST-FMT | PASS | cargo fmt --all -- --check | 通过 |
| WIN-RUST-CHECK | PASS | cargo check --workspace --all-targets | 通过 |
| WIN-RUST-CLIPPY | PASS | cargo clippy --workspace --all-targets -- -D warnings | 通过；仅非阻塞 MSVC linker warning |
| WIN-RUST-TEST | FAIL | PYTHON=.venv-windows-validation\Scripts\python.exe; cargo test --workspace --no-fail-fast | 163 passed, 1 failed；Desktop 76/77 |
| WIN-RUST-DESKTOP-REST | PASS | cargo test -p xarchive-desktop --lib -- --skip configured_sidecar_args_passes_portable_gallery_dl_path_to_worker | 76 passed；仅用于隔离已知测试问题 |
| WIN-SIDECAR-PYTEST | PASS | compileall；pytest sidecar/tests -q --basetemp validation-artifacts\pytest-current | 12 passed |
| WIN-WORKER-ARTIFACT | FAIL | PyInstaller spec build；workflow 等价路径检查 | spec 输出 sidecar\dist\xarchive-downloader.exe，workflow 要求嵌套目录同名 exe |
| WIN-WORKER-PROTOCOL | PASS | staged worker --help、JSONL hello 和未知 executable probe | ready 正常；未知字段返回 INVALID_COMMAND |
| WIN-TAURI-RELEASE | PASS | npm run build:tauri --workspace desktop | 退出 0，生成 target\release\xarchive-desktop.exe |
| WIN-TAURI-WDIO-BUILD | PASS | npm run build:tauri:wdio --workspace desktop | 专用 wdio-e2e artifact 生成 |
| WIN-WDIO-ADVANCED | PASS | npm run test:e2e:windows:advanced --workspace desktop | 2 specs、4 tests 通过 |
| WIN-WDIO-ORDINARY | PASS | npm run test:e2e:windows --workspace desktop | Dashboard 2/2 通过 |
| WIN-WDIO-TEARDOWN | PASS（带 warning） | 两次运行后的进程/端口检查 | 每次有 2 个 driver survivor，由 safety-net tree-kill；最终无 driver 和 4444/4445 listener |
| WIN-PORTABLE-FULL | BLOCKED | npm run build:portable:windows --workspace desktop | worker staging 后缺少必需 sidecar\gallery-dl；没有受控来源 |
| WIN-PORTABLE-CORE | PASS | PORTABLE_PACKAGE_TYPE=core、独立输出目录后运行 portable script | manifest、目录和 worker 契约通过 |
| WIN-PORTABLE-START | PASS | Core portable exe 运行 8 秒 | 进程存活并创建 config\archive.sqlite3/日志；download 未提前创建，随后关闭精确 PID |
| WIN-GUI-SETTINGS-INTERACTION | BLOCKED | 未执行设置页、日志页、aria2/Extension 导入和原生选择器交互 | 无稳定 Computer Use/native manual target；Dashboard smoke 不足以替代 |

#### Errors and classification

1. FAIL / FAIL_TEST：runtime Windows 单测把 /tmp/xarchive with spaces 写死为期望值；Windows 实际词法路径使用反斜杠。建议 Linux 后续改为平台路径 fixture 或平台无关比较，再重跑完整 workspace tests。
2. FAIL / packaging contract：PyInstaller spec 的 one-file 输出与 workflow/portable 约定的嵌套目录不一致；阻塞 worker artifact 发布和 Full portable 组装。建议统一 spec、workflow 和 portable script 的 layout。
3. BLOCKED / missing artifact：Full portable 还缺少受控 sidecar\gallery-dl 发布物或构建步骤；Core PASS 不外推为 Full PASS。
4. 非阻塞 warning：WDIO 磁盘空间诊断、Node DEP0190、MSVC linker 输出 warning；未影响通过项。WDIO teardown 的 survivor 已由安全网清理，保留为生命周期 warning。

#### Not executed / blocked / not applicable

- BLOCKED_AUTOMATION：设置/日志/aria2/Extension UI、系统剪贴板/文件夹选择器、DPI、键盘焦点、读屏和对比度。
- BLOCKED：真实 X/Edge Cookie、Telegram、Credential Manager、外部网络和账号。
- NOT RUN：Named Pipe/ACL、Native Host manifest/Registry/browser installation、executor/restart/recovery、SQLite migration/reparse/ACL/长路径、aria2 业务 fallback、跨卷提交、Full 分发、log rotation、installer/signing/updater/Tray/Autostart；缺少 endpoint、fixture、artifact 或证书。
- NOT APPLICABLE：当前 bundle.active=false 的 installer/signing/updater；仓库没有独立 Browser Mode 配置。

#### Queue reconciliation and Linux follow-up

- WQ-P0-01 更新为 WINDOWS_FAIL：Clippy 已通过，但完整 workspace test 有 1 个 Windows 路径断言失败；修复测试后重跑完整 baseline。
- WQ-P1-16 / WQ-P1-17 保持 WINDOWS_PASS：ordinary/advanced native smoke 通过，teardown warning 和最终清理证据保留。
- WQ-P1-12 保持 WINDOWS_VERIFICATION_PENDING：worker unknown-field 和 Rust 直接消费者通过，但 path/reparse/ACL/长 JSON/真实下载未执行。
- WQ-P1-18 / WQ-P1-19、WQ-REL-DB-01、WQ-REL-SETTINGS-02、WQ-REL-LOG-03、WQ-REL-CONSOLE-04 保持 pending；本轮只有 Core 首启和日志文件生成证据。
- Linux 后续仅处理：runtime Windows 测试的 POSIX 硬编码期望；PyInstaller worker 的 spec/workflow/portable layout 契约和 gallery-dl Full artifact 来源。修复后先做 Linux regression，再重验 WQ-P0-01、worker artifact 和 Full portable。
- 本轮没有修改 Linux 业务代码、测试代码或配置，只回写验证文档和队列状态。

### Linux reconciliation after the 2026-09-17 Windows result（2026-09-17）

依据上一节 Windows working-tree 结果，Linux 侧只处理两个已明确、可独立验证的问题，并顺带关闭一个潜在的 portable 契约漏洞：

1. `desktop/src-tauri/src/runtime.rs` 的 `configured_sidecar_args` 测试不再硬编码 POSIX `/` 路径，改用 `PathBuf::join` 和 `display()` 构造期望值。生产行为未改变。
2. `sidecar/pyinstaller/xarchive-downloader.spec` 统一为 one-dir 构建：`EXE(exclude_binaries=True)` + `COLLECT`，输出 `dist/xarchive-downloader/xarchive-downloader.exe`，与 Windows workflow、portable 目录和默认配置路径一致。
3. `desktop/scripts/portable-package.mjs` / `build-portable-windows.mjs` 将组件 presence 明确为 `required` / `optional` / `excluded` 三态；Core 明确排除 `sidecar/gallery-dl`，不会因 Linux 工作树中存在该目录而错误复制到 Core 包。worker 仍 required，aria2 保持 optional，Full gallery-dl required。
4. PyInstaller `entrypoint.py` 增加 `if __name__ == "__main__": main()`，确保 one-dir artifact 的直接启动入口明确。

Linux verification after reconciliation:

- `cargo fmt --all -- --check`：PASS；
- `cargo clippy --workspace --all-targets -- -D warnings`：PASS；
- Desktop runtime tests：2/2 PASS；config tests：5/5 PASS；
- `npm test --workspace desktop`：31/31 PASS；Desktop Vite build：PASS；
- Extension check/test：PASS，7/7；
- Sidecar spec/entrypoint compileall：PASS；`pytest sidecar/tests -q`：12/12 PASS；
- `node --check`、spec Python syntax check、`git diff --check`：PASS。

Windows status after reconciliation:

- `WQ-P0-01`：`WINDOWS_VERIFICATION_PENDING`。历史 Windows 路径断言失败已由平台无关测试修复，但必须在修复后的 Windows working tree 重跑完整 workspace tests 后才能改为 `WINDOWS_PASS`。
- `WQ-WORKER-BUILD-01`：`WINDOWS_VERIFICATION_PENDING`。spec 已改为 workflow 约定的 one-dir 输出，仍需 Windows runner 真实构建、`--help`、JSONL probe、目录清单和 SHA-256 证据。
- `WQ-PACKAGE-FULL-01`：`WINDOWS_VERIFICATION_PENDING`。layout 契约已修复，但仍缺受控 gallery-dl artifact/source 和真实 Full portable smoke；不得由 Core 结果外推。
- `WQ-PACKAGE-CORE-02`：`WINDOWS_VERIFICATION_PENDING`。Core gallery-dl 显式排除已在 Linux 契约测试覆盖，仍需 Windows package 目录、GUI 设置和真实外部 gallery-dl 验证。
- `WQ-P1-16` / `WQ-P1-17`：继续保留上一轮 `WINDOWS_PASS`；本轮没有命中其 WDIO service/spec/capability 影响面。
- WQ-P1-12、WQ-GALLERY-CORE-03、WQ-EXT-CORE-04、WQ-WEBVIEW2-05、WQ-RELEASE-06 及 GUI、IPC、ACL/reparse、真实账号和 installer 项目继续按队列保持 `WINDOWS_VERIFICATION_PENDING`、`WINDOWS_BLOCKED` 或 `NOT RUN`，不因 Linux 回归通过而提升状态。

#### Windows revalidation handoff

1. 在 Windows runner 执行 `Windows Sidecar Worker Artifact` workflow，确认 `sidecar\\dist\\xarchive-downloader\\xarchive-downloader.exe` 存在，运行 `--help` 和 JSONL hello，记录 artifact zip、文件清单和 SHA-256。
2. 将 worker one-dir 目录放入 portable staging，重跑 `WQ-P0-01` 的完整 workspace Rust tests；只有全量通过后才可关闭历史 FAIL。
3. 在同一 Windows working tree 准备受控 `gallery-dl.exe`、Desktop release executable 和 Extension 目录，分别执行 Full/Core portable build；确认 Core 不含 `sidecar\\gallery-dl`，Full 包含该目录及 Extension。
4. 对仍为 `BLOCKED_AUTOMATION` 的 GUI/设置/文件选择器/剪贴板项目跳过自动化，按 `docs/validation/windows-queue.md` 中的手工步骤执行并记录截图、日志、版本和实际状态；不得把跳过记为 PASS。

本次 Linux reconciliation 未执行 Windows 验证、未生成 Windows artifact、未同步 Windows 工作副本，也没有修改与上述 follow-up 无关的业务逻辑。

### Windows revalidation after Linux follow-up（2026-09-17 19:00–20:02 +08:00）

本轮针对 Linux follow-up 后的最新 working tree 执行 Windows 重验。Linux 源仍为 branch dev、HEAD a5f42ccc4b6d661e3cf80338b44859e5178e8480，working tree dirty；因此本报告不把结果表述为纯 commit 验证。

#### Validation environment

- Windows 11 Insider Preview 10.0.29667，AMD64。
- Node/npm 24.19.0 / 11.17.0；Rust/Cargo 1.98.0；validation Python 3.12.14；PyInstaller 6.22.3。
- Linux source：/home/shiraishi/VSCode Workspace/Tw2Tg。
- Windows validation copy：E:\Shiraishi\VSCode Workspace\Tw2Tg。
- 使用受控 Linux → E: Robocopy 单向同步：exit 3，Files copied=21、skipped=178、mismatch=0、failed=0；目标额外目录/文件共 24/19 项保留，未使用 purge。同步排除了 .git、虚拟环境、node_modules、target、dist-portable、validation-artifacts、日志、用户数据和机器本地配置。
- worker artifact 由当前 Windows 副本内的 PyInstaller 6.22.3 重新生成；portable staging 只使用 E: 本地构建产物，没有反向同步到 Linux。

#### 本轮适用范围

依据当前 roadmap 和 queue reconciliation，本轮只重验 WQ-P0-01、WQ-WORKER-BUILD-01、WQ-PACKAGE-FULL-01、WQ-PACKAGE-CORE-02，以及受当前 release/portable 修改直接影响的构建与启动 smoke。未重复没有命中当前 diff 的 WDIO ordinary/advanced、GUI 手工交互、账户、ACL/reparse、installer 和签名项目。

#### Validation results

| ID / 项目 | 状态 | Actual command or evidence | 简要结果 |
|---|---|---|---|
| SYNC-WT-2026-09-17-R2 | PASS | Linux → E: Robocopy | 当前 dirty working tree 已同步；Windows 本地验证目录保留 |
| WIN-NODE-CHECK-R2 | PASS | npm run check | Vite build 与 Extension syntax check 通过 |
| WIN-NODE-TEST-R2 | FAIL | npm test | Desktop 30/31；Extension 7/7。失败为 portable-package.test.mjs 的 sidecar/gallery-dl 路径使用 / 后缀匹配 Windows 反斜杠 |
| WIN-NODE-BUILD-R2 | PASS | npm run build | Desktop/Extension build 通过 |
| WIN-RUST-FMT-R2 | PASS | cargo fmt --all -- --check | 通过 |
| WIN-RUST-CHECK-R2 | PASS | cargo check --workspace --all-targets | 通过 |
| WIN-RUST-CLIPPY-R2 | PASS | cargo clippy --workspace --all-targets -- -D warnings | 通过；仅非阻塞 MSVC linker stdout warning |
| WIN-RUST-TEST-R2 | FAIL | PYTHON=.venv-windows-validation\Scripts\python.exe; cargo test --workspace --no-fail-fast | 164 项中 163 passed、1 failed；xarchive-desktop 77/78 |
| WIN-SIDECAR-PYTEST-R2 | PASS | compileall -q sidecar；pytest sidecar/tests -q --basetemp validation-artifacts\pytest-current-2 | 12 passed |
| WIN-WORKER-ARTIFACT-R2 | PASS | PyInstaller spec；检查 sidecar\dist\xarchive-downloader\xarchive-downloader.exe、SHA-256、--help | one-dir 嵌套 artifact 存在；SHA-256 449880D9D765901F099E1F40F12A0854E4CE3594970CDC9438002BEB3CCBE48E；--help exit 0 |
| WIN-WORKER-PROTOCOL-R2 | PASS | worker JSONL hello + shutdown；未知 executable 字段 probe | hello 返回 ready；未知字段返回 INVALID_COMMAND；两次进程 exit 0 |
| WIN-TAURI-RELEASE-R2 | PASS | npm run build:tauri --workspace desktop | exit 0；生成 target\release\xarchive-desktop.exe |
| WIN-PORTABLE-FULL-R2 | BLOCKED | npm run build:portable:windows --workspace desktop | worker 已 staging，但必需的 sidecar\gallery-dl 不存在；脚本明确报 Required portable component is missing |
| WIN-PORTABLE-CORE-R2 | PASS | PORTABLE_PACKAGE_TYPE=core、独立输出目录、npm run build:portable:windows --workspace desktop | Core 包生成；manifest、release exe、nested worker 存在；gallery-dl 未包含 |
| WIN-PORTABLE-START-R2 | PASS | 启动 Core portable exe 8 秒，检查 PID/SQLite/日志后精确关闭 | PID 43248 持续运行；创建 config\archive.sqlite3 和 logs；download 未提前创建；随后已关闭 |
| WIN-GUI-UNCHANGED-R2 | NOT RUN | 未执行设置页/日志页/原生选择器等手工交互 | 当前 roadmap 明确本轮不重复无交集 GUI 项；已有自动化能力也不能替代这些手工验收 |

#### Errors and classification

1. WIN-RUST-TEST-R2 是测试契约 FAIL，不是已观测到的生产功能崩溃。失败测试仍以 Path::new("/tmp/xarchive with spaces") 作为 root；生产词法路径在 Windows 变为 \tmp\...，而测试期望保留 /tmp... 前缀。Linux 侧此前加入 PathBuf::join/display 仍未消除 root fixture 的 POSIX 硬编码。建议改用平台路径 fixture 或比较规范化后的 PathBuf，再重跑完整 workspace tests。
2. WIN-NODE-TEST-R2 是测试路径断言 FAIL。新增 portable Core contract test 对字符串调用 endsWith("sidecar/gallery-dl")；Windows 生成路径为 sidecar\gallery-dl。建议使用 path.resolve/path.join 或统一分隔符后比较，再重跑 npm test。
3. WIN-PORTABLE-FULL-R2 为 BLOCKED：当前仓库/Windows 副本没有受控的 gallery-dl.exe 发布物或构建步骤。Core 通过不能外推 Full 通过；需要 Linux/发布流程提供并记录可信 artifact 来源。
4. MSVC linker stdout、Node DEP0190 和 npm 新版本提示均为非阻塞 warning；没有证据表明它们导致失败。
5. 本轮未修改业务代码、测试代码或项目配置；只在 E: 产生正常 build/test/artifact/log 文件并回写 Linux 验证文档。

#### Not executed / blocked / not applicable

- BLOCKED_AUTOMATION：设置页、运行日志实时交互、剪贴板、文件夹选择器、DPI、键盘焦点、读屏和对比度；本轮未启动新的 WebView2 手工验收。
- BLOCKED：Full portable 的 gallery-dl 前置 artifact。
- NOT RUN：真实 gallery-dl 下载、真实 Extension 导入、Named Pipe/ACL、symlink/junction/reparse、长路径/长 JSON、SQLite migration、aria2 fallback、真实 Telegram/Edge Cookie、installer/signing/updater/Tray/Autostart；本轮 scope 未命中或缺少受控 endpoint/fixture/证书/账号。
- NOT APPLICABLE：当前 bundle.active=false 的 installer/signing/updater；仓库没有独立 Browser Mode 配置。

#### Queue result and Linux follow-up

- WQ-P0-01：更新为 WINDOWS_FAIL；两个测试层面的 Windows 路径断言尚未修复。
- WQ-WORKER-BUILD-01：更新为 WINDOWS_PASS；one-dir artifact、--help、JSONL hello/unknown-field probe 和 SHA-256 已有当前副本证据。
- WQ-PACKAGE-FULL-01：更新为 WINDOWS_BLOCKED；缺少 gallery-dl artifact。
- WQ-PACKAGE-CORE-02：更新为 WINDOWS_PASS（包边界/启动 smoke）；设置页外部路径和 Extension 导入仍不在本轮结论内。
- WQ-P1-16/WQ-P1-17：保留上一轮 WINDOWS_PASS；本轮按 roadmap 的无交集规则 KEEP_VALID，未重复执行。
- WQ-P1-12：保留 WINDOWS_VERIFICATION_PENDING；本轮只确认 worker unknown-field probe，路径权限、reparse、长 JSON 和真实下载仍未执行。
- WQ-REL-DB-01：保留 WINDOWS_VERIFICATION_PENDING；Core 首启已证明 SQLite 文件生成，但未完成 UI 任务列表验收。
- Linux 后续必须处理：runtime 测试 root fixture 的 POSIX 硬编码、portable-package test 的路径分隔符断言，并补齐 Full 所需 gallery-dl artifact/来源；之后先做 Linux regression，再重跑 WQ-P0-01 和 Full portable。不要把这两个测试修复扩大为业务代码开发。

### Linux reconciliation after Windows revalidation R2（2026-09-17）

根据 `Windows revalidation after Linux follow-up` 的失败结果，Linux 侧仅修复两个测试 fixture 契约：

1. `desktop/src-tauri/src/runtime.rs` 将 `configured_sidecar_args` 测试的 root 从 `/tmp/xarchive with spaces` 改为相对路径 `xarchive with spaces`，保留 `PathBuf::join` 断言逻辑。这样测试不再依赖 POSIX 根路径，同时继续覆盖包含空格的 portable 路径。
2. `desktop/test/portable-package.test.mjs` 使用 `join("sidecar", "gallery-dl")` 构造目录后缀，覆盖 POSIX 和 Windows 分隔符。

Linux verification after R2 reconciliation：

- `cargo fmt --all -- --check`：PASS；
- `cargo test -p xarchive-desktop runtime::tests --lib --no-fail-fast`：2/2 PASS；
- `cargo test --workspace --all-targets --no-fail-fast`：日志中所有 crate 测试套件均为 `test result: ok`，无 `FAILED`/`error`；
- `npm test --workspace desktop`：31/31 PASS；`npm run check --workspace desktop`：PASS；
- `npm run check --workspace extension`：PASS；`npm test --workspace extension`：7/7 PASS；
- `compileall` / Sidecar pytest：12/12 PASS；PyInstaller spec/entrypoint syntax：PASS；`git diff --check`：PASS。

Current Windows state：

- `WQ-P0-01`：`WINDOWS_VERIFICATION_PENDING`。旧 Windows R2 的 runtime test FAIL 已完成 Linux 修复，但必须重跑 Windows workspace tests；不能提前提升为 PASS。
- `WQ-WORKER-BUILD-01`：`WINDOWS_VERIFICATION_PENDING`。旧 R2 的 one-dir artifact、`--help`、JSONL 和 SHA-256 证据保留为旧 working-tree PASS；当前 dirty diff 命中相关构建/测试契约，需重新确认。
- `WQ-PACKAGE-CORE-02`：`WINDOWS_VERIFICATION_PENDING`。旧 R2 package/start PASS 保留为历史证据；本轮 portable contract test 发生修改，需重新确认 Core 目录边界和启动。
- `WQ-PACKAGE-FULL-01`：`WINDOWS_BLOCKED`。仍缺少受控 `gallery-dl.exe` artifact/source；不得用 Core PASS 替代 Full 验证。
- 其他 GUI、真实账号、Named Pipe/Registry、ACL/reparse、SQLite migration/recovery、installer/signing/updater 和 Extension 实际加载项目继续按 queue 保持 `WINDOWS_VERIFICATION_PENDING`、`WINDOWS_BLOCKED` 或 `NOT RUN`。

本次 reconciliation 未执行 Windows 验证、未生成 Windows artifact、未同步 Windows 工作副本；下一次 Windows handoff 只需重验命中本轮 diff 的 P0 baseline、worker artifact 和 Core portable，并继续跳过无交集项目。

### Windows revalidation after R2 Linux fixture fixes（2026-09-17 20:28–20:40 +08:00）

本轮针对 Linux 侧修复 runtime root fixture 和 portable-package 路径断言后的最新 working tree 执行 Windows 重验。Linux 源为 branch dev、HEAD a5f42ccc4b6d661e3cf80338b44859e5178e8480，working tree 仍 dirty；结果不代表纯 commit 验证。

#### Validation environment

- Windows 11 Insider Preview 10.0.29667，AMD64。
- Node/npm 24.19.0 / 11.17.0；Rust/Cargo 1.98.0；validation Python 3.12.14；PyInstaller 6.22.3。
- Linux source：/home/shiraishi/VSCode Workspace/Tw2Tg。
- Windows validation copy：E:\Shiraishi\VSCode Workspace\Tw2Tg。
- 最新一次受控 Linux → E: Robocopy：exit 3，Files copied=13、skipped=186、mismatch=0、failed=0；目标额外内容 24 个目录/15 个文件保留，未使用 purge。
- worker artifact 和 portable staging 仅在 E: 生成，未反向同步到 Linux。

#### 本轮适用范围

依据当前 roadmap，本轮执行 WQ-P0-01、WQ-WORKER-BUILD-01、WQ-PACKAGE-CORE-02，并重新确认 Full portable 的既有前置阻塞。WDIO native session、GUI 手工交互、真实账号、ACL/reparse、installer/signing 等无交集或缺少受控前置的项目未重复。

#### Validation results

| ID / 项目 | 状态 | Actual command or evidence | 简要结果 |
|---|---|---|---|
| SYNC-WT-2026-09-17-R3 | PASS | Linux → E: Robocopy | 最新 dirty working tree 已同步；Windows 本地依赖/缓存/用户数据保留 |
| WIN-NODE-CHECK-R3 | PASS | npm run check | Vite 与 Extension syntax check 通过 |
| WIN-NODE-TEST-R3 | FAIL | npm test | Desktop 的 30 个非 killTree 测试通过；killTree Windows 子进程清理测试约 10 秒后失败并使整套命令挂起，手工中止；Extension 单独 npm test 7/7 |
| WIN-NODE-CONTRACT-REST-R3 | PASS | node --test desktop/test/log-lines.test.mjs desktop/test/portable-package.test.mjs desktop/test/ui-state.test.mjs desktop/test/ui-wiring.test.mjs | 23/23 通过；路径分隔符修复有效 |
| WIN-NODE-BUILD-R3 | PASS | npm run build | Desktop/Extension build 通过 |
| WIN-RUST-FMT-R3 | PASS | cargo fmt --all -- --check | 通过 |
| WIN-RUST-CHECK-R3 | PASS | cargo check --workspace --all-targets | 通过 |
| WIN-RUST-CLIPPY-R3 | PASS | cargo clippy --workspace --all-targets -- -D warnings | 通过；仅非阻塞 MSVC linker stdout warning |
| WIN-RUST-TEST-R3 | PASS | PYTHON=.venv-windows-validation\Scripts\python.exe; cargo test --workspace --all-targets --no-fail-fast | 所有 workspace crate test suites 均 ok；Desktop 78/78，runtime fixture 通过 |
| WIN-SIDECAR-PYTEST-R3 | PASS | compileall -q sidecar；pytest sidecar/tests -q --basetemp validation-artifacts\pytest-current-3 | 12/12；仅 pytest cache WinError 5 warning |
| WIN-WORKER-ARTIFACT-R3 | PASS | PyInstaller spec；one-dir path、SHA-256、--help | sidecar\dist\xarchive-downloader\xarchive-downloader.exe 存在；SHA-256 148EBDE72B5447FACB95B733B0C56A4154D1F7769197D288AE9583B1CFDDBEC2；--help exit 0 |
| WIN-WORKER-PROTOCOL-R3 | PASS | 独立 JSONL hello/shutdown 与 unknown executable probe | hello 返回 ready；unknown field 返回 INVALID_COMMAND；两个 probe 正常结束且无残留 worker |
| WIN-TAURI-RELEASE-R3 | PASS | npm run build:tauri --workspace desktop | exit 0；生成 target\release\xarchive-desktop.exe |
| WIN-PORTABLE-FULL-R3 | BLOCKED | npm run build:portable:windows --workspace desktop | worker 已 staging，但必需 sidecar\gallery-dl 缺失；脚本明确报 Required portable component is missing |
| WIN-PORTABLE-CORE-R3 | PASS | PORTABLE_PACKAGE_TYPE=core、独立输出目录、portable script | Core 包生成；release exe、manifest、nested worker 存在；gallery-dl 未包含 |
| WIN-PORTABLE-START-R3 | PASS | 启动 dist-portable\XArchive-core-current-4\xarchive-desktop.exe 8 秒 | PID 60516 存活；config\archive.sqlite3 与 logs 生成；download 未提前创建；随后已精确关闭 |
| WIN-GUI-UNCHANGED-R3 | NOT RUN | 未启动 GUI 手工交互 | 当前 Plan 明确本轮不重复无交集 GUI 项；自动化契约测试不能替代手工验收 |

#### Errors and classification

1. WIN-NODE-TEST-R3：失败集中在 desktop/test/wdio-tauri-service.test.mjs 的 killTree test。测试启动 Node 子进程后调用 Windows taskkill，并等待 exit/close 证据；约 10 秒后失败，随后测试进程仍不退出，需要中止。其余 23 个当前 Desktop contract/UI 测试通过。当前证据更符合 Windows Node subprocess/test lifecycle 或测试自身等待契约问题，不是应用业务功能失败；建议 Linux 后续单独调查 killTree 的 Windows 子进程退出证据和测试清理，必要时将环境/自动化阻塞与产品验收分离。
2. WIN-PORTABLE-FULL-R3：BLOCKED，不是构建脚本回归。当前 Linux/Windows 工作副本均没有受控 gallery-dl.exe artifact/source；不能用 Core PASS 推断 Full PASS。
3. Pytest cache WinError 5、MSVC linker stdout、npm update notice 为非阻塞环境 warning；不影响相应 PASS 项。
4. 本轮没有修改业务代码、测试代码或项目配置，只生成 E: 验证产物并回写 Linux 验证文档。

#### Not executed / blocked / not applicable

- BLOCKED：Full portable 的 gallery-dl artifact 前置。
- BLOCKED_AUTOMATION / NOT RUN：设置页和日志页手工操作、剪贴板/原生选择器、DPI/accessibility、真实 gallery-dl/Extension 导入、Named Pipe/ACL、symlink/junction/reparse、长路径/长 JSON、SQLite migration/recovery、aria2 fallback、真实 Telegram/Edge Cookie、installer/signing/updater/Tray/Autostart。
- NOT APPLICABLE：当前 bundle.active=false 的 installer/signing/updater，以及仓库没有独立 Browser Mode 配置。

#### Queue result and Linux follow-up

- WQ-P0-01：保持 WINDOWS_FAIL；Rust P0 已通过，但 Node workspace 全测仍被 killTree Windows test failure 拦截。
- WQ-WORKER-BUILD-01：保持 WINDOWS_PASS；当前 one-dir artifact、SHA-256、help 和协议探针均通过。
- WQ-PACKAGE-CORE-02：保持 WINDOWS_PASS（包边界与启动 smoke）；设置页外部路径和 Extension 导入仍未验收。
- WQ-PACKAGE-FULL-01：保持 WINDOWS_BLOCKED；需要受控 gallery-dl artifact/source。
- WQ-P1-16/WQ-P1-17：按当前 roadmap 的 KEEP_VALID 规则保持上一轮结论，本轮未重复 native session。
- Linux 后续只需：调查并修正或重新分类 killTree Windows 测试生命周期问题；提供 Full 所需 gallery-dl artifact/source；随后重跑 npm 全测和 Full portable。不要为通过验证扩大业务开发范围。
### Windows 最新 working-tree 验证结论（2026-09-17，本次实际验证）

本轮针对 Linux 最新 dirty working tree 执行受控 Windows 验证。Linux source 为 `dev` / HEAD `a5f42ccc4b6d661e3cf80338b44859e5178e8480`，包含未提交修改；实际 Windows 验证目录为 `E:\Shiraishi\VSCode Workspace\Tw2Tg`，`Tw2Tg-CodexAlias` 仅为指向该目录的 reparse alias。

同步使用 Linux → E: Robocopy，`/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`，排除依赖、缓存、target、portable/validation artifacts、用户数据、日志/数据库和二进制；Robocopy exit `3`，`FAILED=0`、`MISMATCH=0`，代表性源/目标 SHA-256 14/14 匹配，未反向同步。

#### Validation results

| ID / 项目 | 状态 | Actual command or evidence | 简要结果 |
|---|---|---|---|
| SYNC-WT-2026-09-17-R4 | PASS | Linux → E: Robocopy + SHA-256 | dirty working tree 已同步；本地依赖和验证目录保留 |
| WIN-NODE-CHECK-R4 | PASS | `npm run check`（实际 E: 目标目录） | Vite 与 Extension syntax check 通过 |
| WIN-NODE-TEST-R4 | PASS | `npm run test`（实际 E: 目标目录） | Desktop 31/31、Extension 7/7；killTree 通过 |
| WIN-NODE-BUILD-R4 | PASS | `npm run build` | Desktop/Extension build 通过 |
| WIN-RUST-FMT/CHECK/CLIPPY-R4 | PASS | `cargo fmt --all -- --check`；`cargo check --workspace --all-targets`；strict Clippy | 全部通过；仅非阻塞 MSVC linker stdout warning |
| WIN-RUST-TEST-R4 | PASS | `cargo test --workspace --no-fail-fast`，使用 `.venv-windows-validation\\Scripts\\python.exe` | workspace tests 全部通过：12+78+16+8+11+4+25+12；doctests 0 项 |
| WIN-SIDECAR-R4 | PASS | compileall + `pytest sidecar/tests -q` | compileall 通过；pytest 12/12 |
| WIN-TAURI-RELEASE/START-R4 | PASS | `npm run build:tauri`；release exe 启动 8 秒后精确关闭 | 生成 exe；启动和清理成功 |
| WIN-WORKER-HELP-R4 | FAIL | `sidecar\\xarchive-downloader\\xarchive-downloader.exe --help` | 现有 E: artifact 缺少 `...\\_internal\\python312.dll`；SHA-256 `148EBDE72B5447FACB95B733B0C56A4154D1F7769197D288AE9583B1CFDDBEC2` |
| WIN-WORKER-ARTIFACT-R4 | BLOCKED | Windows worker workflow 本轮未执行 | 缺少可确认有效的 CI/发布 artifact；现有本地 artifact 的 help 已失败 |
| WIN-PORTABLE-CORE-R4 | PASS（边界） | `PORTABLE_PACKAGE_TYPE=core npm run build:portable:windows --workspace desktop` | manifest/目录边界正确；未含 gallery-dl/Extension，未预创建 `download/`；worker runtime 仍受上项影响 |
| WIN-PORTABLE-FULL-R4 | BLOCKED | `PORTABLE_PACKAGE_TYPE=full npm run build:portable:windows --workspace desktop` | 缺少必需 `sidecar\\gallery-dl`，脚本明确拒绝组装 |
| WIN-WDIO-ORDINARY-R4 | PASS | `npm run test:e2e:windows --workspace desktop` | WebView2 native session；Dashboard 2/2；exit 0 |
| WIN-WDIO-ADVANCED-R4 | PASS | `npm run build:tauri:wdio`；`npm run test:e2e:windows:advanced --workspace desktop` | 2 specs / 4 tests 通过；plugin execute、mock/restore 通过；exit 0 |
| WIN-WDIO-CLEANUP-R4 | PASS（含诊断警告） | 两次 WDIO 后复查 | upstream teardown 各有 2 个 driver survivor，safety-net tree-kill 后相关进程和 4444/4445 均为 0 |
| WIN-WORKSPACE-ALIAS-R4 | FAIL（验证环境） | 经 `Tw2Tg-CodexAlias` 运行 Vite check/build | Rollup 将入口解析为 `../../Tw2Tg/desktop/index.html`；实际 E: 目标目录重跑通过，不判为产品代码失败 |

#### Not executed / blocked / not applicable

- `BLOCKED_AUTOMATION`：设置页、运行日志、剪贴板、原生选择器、DPI、键盘焦点、Narrator/NVDA、对比度和视觉布局；Computer Use 初始化返回 `helper_unknown_error: setup refresh had errors`。
- `NOT RUN`：真实 gallery-dl 下载、真实 Extension 导入/浏览器加载、Named Pipe/Registry/ACL、symlink/junction/reparse、长路径/长 JSON、SQLite migration/restart/recovery、aria2 fallback、真实 Telegram/Edge Cookie、installer/signing/updater/Tray/Autostart。
- `NOT APPLICABLE`：当前 `bundle.active=false`，正式 installer/signing/updater 不适用；仓库没有独立 Browser Mode 配置。

#### Queue result and Linux follow-up

- `WQ-P0-01`：`WINDOWS_PASS`；实际 E: 目标目录的 Node/Rust/Sidecar baseline 全部通过。
- `WQ-P1-16` / `WQ-P1-17`：`WINDOWS_PASS`；ordinary/advanced native smoke 和最终清理均有本轮证据，保留 safety-net warning。
- `WQ-WORKER-BUILD-01`：当前轮 `WINDOWS_BLOCKED`；需重新生成有效 Windows worker artifact。历史 PASS 记录保留。
- `WQ-PACKAGE-CORE-02`：边界 PASS，整体 `WINDOWS_VERIFICATION_PENDING`；worker runtime、设置页外部 gallery-dl 配置和 Extension 导入未验收。
- `WQ-PACKAGE-FULL-01`：`WINDOWS_BLOCKED`；需要受控 `gallery-dl.exe` artifact/source 后重跑。
- `WQ-REL-DB-01`、`WQ-REL-SETTINGS-02`、`WQ-REL-LOG-03`、`WQ-REL-CONSOLE-04`、`WQ-P1-12`：保持 `WINDOWS_VERIFICATION_PENDING`/`NOT RUN`，未以 release process smoke 外推完整应用验收。

Linux 后续只需提供可运行的 Windows worker one-dir artifact、受控 Full `gallery-dl.exe` 来源，并使用实际 E: 目录而非 reparse alias 运行验证；随后重跑 worker/Full portable 和手工 GUI 项目。不要为通过验证修改业务代码。
### Full portable retry with provided gallery-dl artifact (2026-09-17)

用户提供的 E: 本地 artifact 为 `E:\Shiraishi\VSCode Workspace\Tw2Tg\gallery-dl\gallery-dl.exe`。本轮仅在 Windows 验证副本中将其复制到脚本要求的 `sidecar\\gallery-dl\\gallery-dl.exe`；该 artifact 未同步到 Linux source，也未修改业务代码。

| 项目 | 状态 | 实际证据 |
|---|---|---|
| `gallery-dl.exe` artifact | PASS | 版本 `1.32.12`；`--version`/`--help` exit 0；SHA-256 `0B36AE6734ED41E12BE6BE1B33D3165A450B3E0A811FC1B8C664C032F7F13B2C` |
| Full portable assembly | PASS | `PORTABLE_PACKAGE_TYPE=full npm run build:portable:windows --workspace desktop` exit 0；输出 `validation-artifacts\\portable-full-r5` |
| Full manifest/boundary | PASS | manifest 标记 `package_type=full`、`gallery_dl_bundled=true`、`extension.bundled=true`；gallery-dl、worker、Extension 均存在；`download/` 未预创建 |
| Full portable startup | PASS | Full exe 启动 8 秒，PID 存活并精确关闭；`config\\archive.sqlite3`、`logs` 创建；`download`、`telegram` 未创建 |
| Bundled worker smoke | FAIL | `sidecar\\xarchive-downloader\\xarchive-downloader.exe --help` 仍因缺少 `...\\_internal\\python312.dll` 失败；SHA-256 `148EBDE72B5447FACB95B733B0C56A4154D1F7769197D288AE9583B1CFDDBEC2` |
| Full Sidecar handshake/real download | BLOCKED | bundled worker 无法启动，不能执行 `hello → ready` 或受控下载；不归因于 gallery-dl artifact |

本次重试解除的是 Full package 的 `gallery-dl` 缺失阻塞；Full 目录组装和启动 smoke 已通过，但完整 Full runtime 仍等待有效的 Windows worker one-dir artifact。下一步只需重新生成包含 `_internal\\python312.dll` 等全部依赖的 worker，再重跑 worker `--help`、JSONL handshake、Full portable startup 和受控下载。

### Linux follow-up after latest worker runtime failure (2026-09-17)

依据最新 Windows 实际结果，Linux 端没有继续推进旧 GUI/WDIO 计划，而是只处理可独立确认的构建契约：删除 `sidecar/pyinstaller/entrypoint.py` 重复的 `main()` 调用；在 Windows worker workflow 上传前检查 one-dir artifact 必须包含 `_internal\\python312.dll`；将 Core portable manifest 的 `extension.user_importable` 改为 `false`，与当前设置页仅提供 GitHub Extension 外链和浏览器指南的实现一致。

Linux verification：`cargo fmt --all -- --check`、`cargo check -p xarchive-storage -p xarchive-desktop --all-targets`、`cargo test -p xarchive-storage --lib --no-fail-fast`（25/25）、`npm test --workspace desktop`（31/31）、`npm run build --workspace desktop`、Python `compileall`、Node script syntax checks 和 `git diff --check` 均通过。

Windows queue reconciliation：`WQ-WORKER-BUILD-01`、`WQ-PACKAGE-CORE-02`、`WQ-PACKAGE-FULL-01` 均保持 `WINDOWS_VERIFICATION_PENDING`，等待包含完整 `_internal` runtime 的新 worker artifact 后重验；worker artifact 缺失/无法生成时使用现有 `WINDOWS_BLOCKED` 手工步骤。普通/高级 WDIO 结果未受本轮 service/spec/capability diff 影响，继续按上一轮证据 `KEEP_VALID`，不重复执行。
### 2026-09-19 U7 latest dirty working-tree Windows validation

本轮针对 Linux 最新 dirty working tree 执行。Linux source 为 `feature/u7-desktop-production-integration` / HEAD `79232f24641f88e11b077611723f0af5cd92760e`，包含未提交的 U7 实现与文档修改；Windows 实际工作副本为 `E:\Shiraishi\VSCode Workspace\Tw2Tg`，未使用 `Tw2Tg-CodexAlias`。Windows 11 Insider Preview `10.0.29671` / 64 位；Node `v24.19.0`、npm `11.17.0`、Rust/Cargo `1.98.0`、Tauri CLI `2.11.4`、Python `3.12.14`。

同步方向为 Linux source → E: validation workspace。Robocopy exit `3`，`FAILED=0`、`MISMATCH=0`，未 purge 或反向同步；排除了 `.git`、依赖、virtualenv、target、dist/validation artifacts、缓存、日志、用户数据和 E: 本地 artifact。代表性 `production.rs`、`executor.rs`、`status.md` SHA-256 均 source/target 匹配。worker、gallery-dl 和 portable/validation artifacts 未回写 Linux。

#### 本轮适用范围

当前 diff 命中 Desktop production executor、Sidecar v2 supervisor、download plan/refresh 和 Windows packaged worker/portable runtime，因此执行 Node workspace、Rust fmt/check/test/clippy/Tauri build、受影响 Rust crates、Python Sidecar、worker artifact probe 及 Core/Full portable manifest 组装。WDIO service/spec/capability 未变化，ordinary/advanced native session 不重复；真实 WebView2、GUI、账号、Named Pipe/Registry、ACL/reparse、aria2、signed URL、installer/signing 等按前置条件保持 BLOCKED/NOT RUN。

#### Validation results

| 验证项目 | 状态 | 实际命令/证据 | 结果摘要 |
|---|---|---|---|
| Linux → Windows 同步与代表性 hash | PASS | Robocopy `/E /XJ /FFT /IS /IT /COPY:DAT /DCOPY:DAT`；SHA-256 | `FAILED=0`、`MISMATCH=0`；本地依赖、artifact、缓存和用户数据保留 |
| Node workspace check | PASS | `npm run check` | Vite 与 Extension syntax check 通过 |
| Node workspace tests | PASS | `npm test` | Desktop 33/33、Extension 7/7 |
| Node workspace build | PASS | `npm run build` | Desktop/Extension build 通过 |
| Rust formatter | PASS | `cargo fmt --all -- --check` | 通过 |
| Rust workspace check | FAIL | `cargo check --workspace --all-targets` | Windows `transport.rs` 报 `error[E0425]: cannot find type PathBuf in this scope` |
| Rust workspace tests | FAIL | `cargo test --workspace --no-fail-fast` | 同一 `PathBuf` Windows 编译错误 |
| Rust strict Clippy | FAIL | `cargo clippy --workspace --all-targets -- -D warnings` | 同一 `PathBuf` Windows 编译错误 |
| U7 xarchive-download crate | PASS | `cargo test -p xarchive-download` | 23 unit + 7 integration 全部通过 |
| U7 sidecar-supervisor crate | PASS | 设置 `.venv-windows-validation\Scripts\python.exe` 后 `cargo test -p xarchive-sidecar-supervisor` | 5/5 通过 |
| Sidecar compileall | PASS | `.venv-windows-validation\Scripts\python.exe -m compileall -q sidecar\src` | 通过 |
| Sidecar full pytest | FAIL | `.venv-windows-validation\Scripts\python.exe -m pytest sidecar\tests -q` | 29 passed、4 failed；4 个 fixture 执行 POSIX `#!/bin/sh`，触发 `WinError 193` |
| Sidecar Windows-compatible remainder | PASS | 同上 `-k` 排除 4 个 POSIX fixture | 29/29 通过；不替代 full pytest FAIL |
| Existing worker `--help` | PASS | `sidecar\dist\xarchive-downloader\xarchive-downloader.exe --help` | exit 0；`_internal\python312.dll` 存在 |
| Packaged worker v2 hello/shutdown | FAIL | v2 JSONL `hello` + `shutdown` probe | artifact 返回 `protocol_version:1`、`UNSUPPORTED_PROTOCOL_VERSION`；是旧 v1 worker |
| Tauri release build | FAIL | `npm run build:tauri --workspace desktop` | Vite 通过，Rust app build 因同一 `PathBuf` 错误失败 |
| Core portable manifest/目录边界 | PASS | `PORTABLE_PACKAGE_TYPE=core ... build:portable:windows --workspace desktop` | Core 正确排除 gallery-dl 与 Extension |
| Full portable manifest/目录边界 | PASS | `PORTABLE_PACKAGE_TYPE=full ... build:portable:windows --workspace desktop` | Full manifest、Extension、E: 本地 gallery-dl 均存在 |
| Current-source Tauri/portable runtime acceptance | BLOCKED | 依赖 Tauri 编译与 v2 worker artifact | 目录组装 PASS 不能外推为 current-source runtime PASS |

#### Errors and classification

1. **Windows product compile FAIL：**`transport.rs` 将 `PathBuf` 与 `Path` 一并放在 `#[cfg(unix)]` import 中，但 `BrowserTransportAdapter` 在 Windows 也无条件引用 `PathBuf`。Linux check 通过，Windows check/test/clippy/Tauri build 均复现 `E0425`；本轮未修改代码。
2. **Packaged worker protocol FAIL：**现有 one-dir worker `--help` 和 DLL 完整性通过，但 v2 probe 返回 v1/`UNSUPPORTED_PROTOCOL_VERSION`。需要重新生成当前 source 对应的 Windows worker artifact。
3. **Sidecar pytest fixture FAIL：**4 个失败均为 Windows `CreateProcess` 启动无 `.exe` 的 POSIX fake script 触发 `WinError 193`；排除这些 fixture 后 29/29 通过，当前证据指向测试 fixture 缺口。
4. **Supervisor 初次 FAIL 已被环境前置消除：**未设置 `PYTHON` 时 2 个真实 worker handshake 测试为 `NotRunning`；使用项目 venv 后 5/5 通过，最终 crate 状态按显式前置后的 PASS 记录。

#### Not executed / blocked / not applicable

- `BLOCKED`：当前-source Tauri app startup、U7 production executor、Sidecar v2 extraction、aria2 multi-GID/refresh、staging→ArchiveService commit、cancel/shutdown/recovery/late-result fencing；前置为 Desktop 编译、当前 worker、aria2c 和受控 fixtures。
- `BLOCKED_AUTOMATION` / `NOT RUN`：真实 WebView2 GUI、原生选择器、日志/设置页、DPI/键盘/辅助技术、Extension/Native Host、Named Pipe/Registry/ACL、reparse、长路径、SQLite restart、真实 X/Telegram/Edge Cookie、installer/signing/updater/Tray。
- `NOT APPLICABLE`：当前 `bundle.active=false` 下的正式 installer/signing/updater 验收。
- WDIO ordinary/advanced 保留历史 KEEP_VALID 证据，但不能替代 U7 production runtime 验收；本轮未因 service/spec/capability 无交集重复执行。

#### Queue result and Linux follow-up

- `WQ-P0-01`：`WINDOWS_FAIL`；Node 和独立受影响 crates 通过，但 workspace/Tauri compile 被 `PathBuf` 条件导入阻塞。
- `WQ-U7-01` / `WQ-ARCH-01`：`FAIL` / `BLOCKED`；旧 worker 仅通过 v1 响应，当前 v2 artifact 需重生成。
- `WQ-U7-02` / `WQ-U7-03` / `WQ-U7-04` / `WQ-U7-05`：`BLOCKED`，等待 Desktop compile、aria2c、signed URL、filesystem 和 SQLite fixtures。
- `WQ-WORKER-BUILD-01`：`WINDOWS_FAIL`；`--help`/DLL 通过但 v2 protocol 不匹配。
- `WQ-PACKAGE-CORE-02` / `WQ-PACKAGE-FULL-01`：`WINDOWS_VERIFICATION_PENDING`；manifest 边界通过，完整 runtime 未验收。

Linux 后续只需：修复 `transport.rs` 的 Windows `PathBuf` 导入条件并完成 Linux fmt/check/test；将 Sidecar POSIX fake executable 改为 Windows-compatible fixture 或显式平台分类；重新生成 v2 worker artifact；然后重跑 workspace/Tauri、WQ-U7-01 至 WQ-U7-05 和 Core/Full runtime。不要通过修改业务逻辑、放宽安全边界或沿用旧 artifact 制造 PASS。
### 2026-09-19 current-revision Windows revalidation after Linux follow-up

本轮重新验证 Linux 最新 dirty working tree。Linux source 为 feature/u7-desktop-production-integration / HEAD 79232f24641f88e11b077611723f0af5cd92760e，包含未提交 U7 实现与文档修改。Windows 副本为 E:\Shiraishi\VSCode Workspace\Tw2Tg；Robocopy exit 3，FAILED=0、MISMATCH=0；transport.rs、test_protocol_v2.py、production.rs 代表性 SHA-256 与 Linux source 匹配。E: worker、gallery-dl、target、依赖和 validation artifacts 未反向同步。

#### Validation results

| 验证项目 | 状态 | 实际命令/证据 | 结果摘要 |
|---|---|---|---|
| Linux → Windows 同步 | PASS | 受控 Robocopy + SHA-256 | 单向同步完成；本地依赖、缓存、artifact 和用户数据保留 |
| Node workspace check/test/build | PASS | npm run check；npm test；npm run build | Vite/Extension check 通过；Desktop 33/33、Extension 7/7；build 通过 |
| Rust fmt/check/clippy | PASS | cargo fmt --all -- --check；cargo check --workspace --all-targets；strict Clippy | PathBuf Windows 编译问题已消除；均 exit 0 |
| Rust workspace tests | PASS | 使用 .venv-windows-validation Python 的 cargo test --workspace --no-fail-fast | 190 个 crate tests 全部通过：13+82+23+7+8+15+5+25+12 |
| Tauri release build | PASS | npm run build:tauri --workspace desktop | 当前 source release exe 构建成功；仅非阻塞 MSVC linker warning |
| Sidecar compileall | PASS | Python -m compileall -q sidecar\src | 通过 |
| Sidecar full pytest | FAIL | Python -m pytest sidecar\tests -q | 31 passed、2 failed；test_extraction_only.py 的无扩展名 POSIX fake executable 触发 WinError 193 |
| Sidecar remainder | PASS | 排除上述 2 个 fixture 后 pytest | 31/31 通过；不替代 full pytest FAIL |
| Current worker PyInstaller build | PASS | workflow 等价 PyInstaller spec 构建 | one-dir、python312.dll、--help 通过；SHA-256 4C868A591C28CBC5559095F88321B81DD291397934005D8D2303E11C0EA74963 |
| Current packaged worker v2 protocol | FAIL | v2 JSONL hello、unknown-field、shutdown probe | executable 仍输出 protocol_version:1；v2 hello/shutdown 返回 UNSUPPORTED_PROTOCOL_VERSION |
| Core portable assembly/boundary | PASS | PORTABLE_PACKAGE_TYPE=core portable script | Core manifest/目录边界正确；无 gallery-dl/Extension |
| Full portable assembly/boundary | PASS | PORTABLE_PACKAGE_TYPE=full portable script | Full manifest、Extension、worker、受控 gallery-dl 均存在 |
| Full portable startup smoke | PASS | current release Full exe 启动 8 秒后精确关闭 | SQLite/logs 创建，download 未预创建，进程可清理 |
| U7 production runtime/controlled download | BLOCKED | 依赖 v2 worker、aria2c、signed URL 和 filesystem fixture | 不能由 assembly/startup smoke 外推为 production PASS |

#### Errors and follow-up

1. 上一轮 transport.rs PathBuf Windows compile FAIL 已修复；workspace check/test/clippy/Tauri build 当前均 PASS。
2. Sidecar full pytest 剩余 2 项仍直接将无扩展名 POSIX shell script 交给 Windows CreateProcess，触发 WinError 193；其余 31/31 通过。这是测试 fixture Windows 兼容性问题。
3. 当前 source 重新生成的 worker 仍打包 xarchive_downloader.main v1 entrypoint；worker_v2 只是 hidden import，未成为 executable entrypoint。因此 v2 probe 仍 FAIL，已排除“旧 E: artifact”原因，属于当前 packaging/entrypoint 问题。
4. aria2 multi-GID、expired URL refresh、Windows file lock/reparse、ArchiveService commit、cancel/shutdown/recovery/late-result fencing、真实 GUI/Native Host/账号和 installer 等保持 BLOCKED、NOT RUN 或 NOT APPLICABLE。

Linux 后续只需：修复 test_extraction_only.py 的两个 Windows fixture；调整 worker packaging entrypoint 使 artifact 启动 worker_v2；先做 Linux regression，再重跑 worker probe、WQ-U7-01 至 WQ-U7-05 和完整 portable runtime。不要扩大为无关业务开发。

### 2026-09-19 Linux reconciliation after current-revision Windows results

已处理上一节指出的两个 Linux 可修复问题：

1. `test_extraction_only.py` 的两个 POSIX fake executable 已改为由 `sys.executable` 启动的 Python fixture；
2. PyInstaller spec 已切换到 `entrypoint_v2.py`，该入口启动 `worker_v2` 并支持 `--gallery-dl`；legacy v1 入口单独保留在 `entrypoint_v1.py`。

Linux verification：Sidecar pytest 33/33、compileall、entrypoint/spec syntax、Rust fmt/check/strict Clippy、workspace tests/doc-tests、Desktop 82/82、Node Desktop 33/33、Extension 7/7、Node build 和 `git diff --check` 全部通过。

这些 Linux 结果不能替代 Windows artifact 证据。`WQ-U7-01`、`WQ-ARCH-01` 和 `WQ-WORKER-BUILD-01` 继续为 `WINDOWS_VERIFICATION_PENDING`，必须用当前 revision 重建 worker 并执行 v2 hello/capability/unknown-field/shutdown/extract probe。`WQ-U7-02` 至 `WQ-U7-05` 的 aria2、signed URL、filesystem、commit、cancel/shutdown/recovery 和 late-result fencing 仍为 `WINDOWS_VERIFICATION_PENDING` 或 `WINDOWS_BLOCKED`，不得提前标记 PASS。
### 2026-09-19 current-revision Windows revalidation after v2 worker and fixture follow-up

本轮验证 Linux 最新 dirty working tree。Linux source 为 feature/u7-desktop-production-integration / HEAD 79232f24641f88e11b077611723f0af5cd92760e，包含未提交修改；Windows 验证副本为 E:\Shiraishi\VSCode Workspace\Tw2Tg。同步方向为 Linux source → E:，Robocopy exit 3，Files copied=150、skipped=81、FAILED=0、MISMATCH=0；未删除或反向同步 E: 的依赖、缓存、target、gallery-dl、worker 和 validation artifacts。代表性 spec、v2 entrypoint、extraction fixture SHA-256 与 Linux source 匹配。

#### Validation environment

- Windows 工作副本：E:\Shiraishi\VSCode Workspace\Tw2Tg
- Python：.venv-windows-validation
- Node/npm：使用 E: 副本现有 Node workspace
- Rust/Tauri：Windows MSVC toolchain；Tauri release executable 构建成功
- Sidecar worker：当前 source 的 PyInstaller one-dir spec；python312.dll 位于 sidecar\dist\xarchive-downloader\_internal
- 验证日期：2026-09-19

#### Validation results

| 验证项目 | 状态 | 实际命令/证据 | 结果摘要 |
|---|---|---|---|
| Linux → Windows 同步与代表性 hash | PASS | 受控 Robocopy；SHA-256 | FAILED=0、MISMATCH=0；E: 本地依赖和验证产物保留 |
| Node workspace check | PASS | npm run check | Vite 与 Extension syntax check 通过 |
| Node workspace tests | PASS | npm test | Desktop 33/33、Extension 7/7 |
| Node workspace build | PASS | npm run build | Desktop/Extension build 通过 |
| Rust formatter/check | PASS | cargo fmt --all -- --check；cargo check --workspace --all-targets | 通过 |
| Rust workspace tests | PASS | 设置 PYTHON 为 .venv-windows-validation\Scripts\python.exe 后 cargo test --workspace --no-fail-fast | 190 个 crate tests 全部通过；doc-tests 0 |
| Rust strict Clippy | PASS | cargo clippy --workspace --all-targets -- -D warnings | 通过；仅有非阻塞 MSVC linker stdout |
| Sidecar compileall | PASS | .venv-windows-validation\Scripts\python.exe -m compileall -q sidecar\src | 通过 |
| Sidecar full pytest | PASS | .venv-windows-validation\Scripts\python.exe -m pytest sidecar\tests -q | 33 passed in 1.11s；此前 WinError 193 fixture 问题不再复现 |
| Current worker PyInstaller build | PASS | .venv-windows-validation\Scripts\python.exe -m PyInstaller --noconfirm --clean --distpath sidecar\dist --workpath sidecar\build sidecar\pyinstaller\xarchive-downloader.spec | 使用 entrypoint_v2.py；one-dir artifact 构建成功，_internal\python312.dll 存在 |
| Packaged worker help | PASS | sidecar\dist\xarchive-downloader\xarchive-downloader.exe --help | exit 0；显示 XArchive Sidecar protocol v2 worker |
| Packaged worker v2 JSONL probe | PASS | protocol_version=2 的 hello、unknown-field、shutdown probe | hello 返回 ready/capabilities；unknown field 返回 INVALID_COMMAND；进程 exit 0 |
| Tauri release build | PASS | npm run build:tauri --workspace desktop | release xarchive-desktop.exe 构建成功；MSVC linker stdout warning 不阻塞 |
| Core portable assembly/boundary | PASS | PORTABLE_PACKAGE_TYPE=core；npm run build:portable:windows --workspace desktop | manifest 正确排除 gallery-dl 和 Extension；worker 存在 |
| Full portable assembly/boundary | PASS | PORTABLE_PACKAGE_TYPE=full；npm run build:portable:windows --workspace desktop | manifest、Extension、E: 受控 gallery-dl 和 worker 均存在 |
| Full portable startup smoke | PASS | Full xarchive-desktop.exe 启动 8 秒后清理 | 进程保持存活并写入 application runtime initialized；logs 可见；未将短窗口内未创建 download/DB 目录外推为完整首次运行 |
| U7 production runtime/controlled download | BLOCKED | 需要 aria2c、signed URL、实际 media fixture、Windows file/restart fixtures | 当前仅完成 worker handshake、assembly 和 startup smoke，不能外推为真实下载/恢复 PASS |
| WQ-P1-16 / WQ-P1-17 native WDIO | PASS（KEEP_VALID） | 沿用历史 session 证据 | 本轮 service/spec/capability 未改动；不外推为 U7 production runtime PASS |

#### Errors and classification

1. 本轮没有新的 Windows product compile FAIL。上一轮 transport.rs 的 Windows PathBuf 条件导入问题已由 Linux follow-up 修复，并由当前 Windows workspace check、tests、strict Clippy 和 Tauri release build 复核通过。
2. 本轮没有复现 Sidecar fixture 的 WinError 193。两个 extraction fixture 已由 Linux follow-up 改为使用 sys.executable 启动的 Python fixture；Windows full pytest 33/33 通过。
3. 本轮没有复现 packaged worker v1/protocol mismatch。PyInstaller spec 已使用 entrypoint_v2.py；v2 probe 实际返回 ready 和 INVALID_COMMAND，确认本轮不是沿用旧 E: worker。
4. Full startup smoke 的 8 秒窗口只证明进程初始化和可清理，不证明 aria2 下载、Sidecar extraction、SQLite restart 或首次运行目录生命周期。

#### Not executed / blocked / not applicable

- BLOCKED：WQ-U7-02 aria2 multi-GID/progress/cancel/cleanup；WQ-U7-03 expired signed URL refresh；WQ-U7-04 staging → ArchiveService commit、Windows file lock/reparse；WQ-U7-05 cancel/shutdown/recovery/late-result fencing。缺少 aria2c、受控 signed URL/media fixture、Windows filesystem 和 restart fixtures。
- NOT RUN / BLOCKED_AUTOMATION：真实 WebView2 GUI、native picker、设置/日志页、DPI/键盘/辅助技术、Extension/Native Host、Named Pipe/Registry/ACL、真实 X/Telegram/Edge Cookie、installer/signing/updater/Tray。
- NOT APPLICABLE：当前 bundle.active=false 下的正式 installer/signing/updater 验收。
- WDIO ordinary/advanced 保留历史 KEEP_VALID 证据；本轮没有影响其 service/spec/capability 的修改，未重复执行。

#### Queue result and Linux follow-up

- WQ-P0-01：WINDOWS_PASS；Node、Rust workspace、Sidecar full pytest 和 Tauri release baseline 均通过。
- WQ-U7-01 / WQ-ARCH-01：WINDOWS_PASS（当前范围限于 packaged v2 handshake）；hello/capability/unknown-field/shutdown probe 通过，尚未覆盖真实 extraction。
- WQ-WORKER-BUILD-01：WINDOWS_PASS；当前 v2 one-dir artifact、_internal\python312.dll、--help 和 JSONL probe 均通过。
- WQ-U7-02 / WQ-U7-03 / WQ-U7-04 / WQ-U7-05：BLOCKED；等待 Windows 专用运行前置与受控 fixtures。
- WQ-PACKAGE-CORE-02 / WQ-PACKAGE-FULL-01：WINDOWS_VERIFICATION_PENDING；manifest、组件边界、Full startup smoke 已通过，但完整 production runtime、受控下载、GUI 和设置/Extension 手工验收仍未完成。
- WQ-P1-16 / WQ-P1-17：WINDOWS_PASS（KEEP_VALID），不替代 U7 production runtime 验收。

需要 Linux 后续处理的问题：准备并接入 aria2c、signed URL/media、Windows file-lock/reparse、SQLite restart/recovery 和 late-result fencing 的受控验证 fixtures，然后重跑 WQ-U7-02 至 WQ-U7-05；补做真实 GUI/Native Host/installer 等尚未执行的验收。当前不需要为本轮 Windows baseline、worker packaging 或两个 fixture 继续修改业务代码，也不应把 startup smoke 或 handshake PASS 扩大解释为完整 U7 runtime PASS。

### 2026-09-19 Linux reconciliation after current-revision Windows PASS/blocked results

最新 Windows 结果已确认上一轮 Linux follow-up 生效：`transport.rs` 条件导入、Windows-compatible extraction fixtures 和 v2 PyInstaller entrypoint 均通过目标环境复验。本轮没有新的 Linux product code failure，也不继续修改 worker/packaging。

Linux 侧已完成并保留的适用验证：Sidecar 33/33、Rust workspace 190-test equivalent、Desktop 82/82、strict Clippy、Node Desktop 33/33、Extension 7/7、fmt/check/build 和 `git diff --check`。

当前计划重新收敛为：

- `WQ-P0-01`、`WQ-U7-01`（仅 packaged handshake 范围）、`WQ-WORKER-BUILD-01` 和 Sidecar full pytest 可记录 Windows PASS；
- `WQ-U7-02` 至 `WQ-U7-05` 仍为 `WINDOWS_BLOCKED`，原因是 aria2c、signed URL/media、Windows filesystem/restart fixtures 缺失或尚未执行；
- `WQ-PACKAGE-CORE-02`、`WQ-PACKAGE-FULL-01` 继续 `WINDOWS_VERIFICATION_PENDING`，因为 assembly/startup smoke 不等于完整 runtime；
- 真实 extraction/download、refresh、staging/commit、file lock/reparse、cancel/shutdown/recovery、GUI/Native Host/installer 仍不得标记为 Windows PASS。

本轮没有 `WINDOWS_VERIFICATION_BLOCKING`，也没有必要新增 Linux 业务开发。下一步应准备 Windows 专用 U7 fixtures 并集中执行 blocked runtime 项目，而不是继续修改已通过的 worker/packaging 代码。
### 2026-09-20 current HEAD Windows validation: U8-U14 and release scope

#### Validation Scope

Linux source was clean before this documentation write-back. Current source is feature/u7-desktop-production-integration / HEAD 4812f29847a6c2ae77eed608e91ff4c2d4bc4769. The HEAD commit is documentation-only and records the pre-release workflow result; the current source tree includes the U8-U14 implementation already present at the tagged pre-release source. This round selected the current-source Windows baseline, v2-only worker, Tauri/WDIO E2E, Core/Full packaging, and release/asset boundary checks. Real aria2 transfer, component activation, Registry/browser integration, final release assets, and filesystem recovery remain separate prerequisites.

Selected Required/Applicable checks were Node workspace check/test/build, Rust fmt/check/workspace tests/strict Clippy, Sidecar compileall/current v2-only pytest/entrypoint/PyInstaller/JSONL probe, Tauri release and E2E builds, ordinary and advanced WDIO, Core/Full portable assembly and startup. Full Node/Rust/Sidecar applicable suites were run. The full release workflow, final asset hash/license/signature and real archive runtime were not run because no final Windows release asset set or controlled external fixtures were available.

#### Validation Environment

- Windows: Windows-11-10.0.29671-SP0
- WebView2/Edge: 153.0.4234.48
- Python: 3.12.14
- PyInstaller: 6.22.3
- Linux source branch: feature/u7-desktop-production-integration
- Linux source commit: 4812f29847a6c2ae77eed608e91ff4c2d4bc4769
- Linux working tree changes included: no, clean before this documentation write-back
- Windows workspace: E:\Shiraishi\VSCode Workspace\Tw2Tg
- Validation date: 2026-09-20

#### Synchronization

Controlled Linux → E: Robocopy completed with exit code 3, Files copied=164, skipped=70, MISMATCH=0, FAILED=0. Local node_modules, Python environments, target, gallery-dl, aria2, logs, caches and validation artifacts were retained. Representative SHA-256 checks for release-assets.mjs, offline-bundle-package.mjs, native-host-package.mjs, components.rs and commands.rs matched Linux source.

The E: copy contained stale files from earlier U8 synchronization that were absent from the Linux tracked-file list. They were not deleted: ten stale source/test files were moved into E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\stale-sync-20260920 for recoverability. Current-source tests were rerun against the cleaned validation tree.

#### Validation Results

| Scope | Category | Command/evidence | Status | Summary |
|---|---|---|---|---|
| Source sync and parity | Required | Robocopy + representative SHA-256 | PASS | FAILED=0, MISMATCH=0; local Windows state retained |
| Node workspace | Required | npm run check; npm test; npm run build | PASS | Desktop 44/44, Extension 7/7, build passed; U9-U13 contract tests included |
| Rust toolchain | Required | cargo fmt --all -- --check; cargo check --workspace --all-targets; cargo clippy --workspace --all-targets -- -D warnings | PASS | All passed; MSVC import-library linker stdout was non-blocking |
| Rust workspace tests | Required | PYTHON=.venv-windows-validation\Scripts\python.exe cargo test --workspace --no-fail-fast | PASS | 188 crate tests passed; doc-tests 0 |
| U8 current v2-only source surface | Required | rg legacy-symbol check after stale-file isolation | PASS | No legacy v1 command/router/schema/entrypoint symbols remained in the current-source validation tree |
| Sidecar current source | Required | Python compileall; pytest sidecar\tests -q; pytest test_entrypoint.py -q | PASS | compileall passed; 21/21 and entrypoint 5/5 passed |
| Current v2 worker artifact | Required | PyInstaller current spec; --help; v2 hello/unknown-field/legacy-v1/shutdown probe | PASS | entrypoint_v2 used; ready/capabilities, INVALID_COMMAND, v1 rejection and clean shutdown; _internal\python312.dll present |
| Tauri release build | Required | npm run build:tauri --workspace desktop | PASS | Current Windows release executable built |
| Tauri E2E artifact | Required | npm run build:tauri:wdio --workspace desktop | PASS | Test-only Tauri plugin artifact built |
| Ordinary WDIO | Applicable | npm run test:e2e --workspace desktop | PASS | dashboard smoke 2/2 |
| Advanced WDIO | Applicable | npm run test:e2e:windows:advanced --workspace desktop | PASS | dashboard and wdio-plugin specs 4/4 after required E2E artifact build |
| Core portable boundary | Required | PORTABLE_PACKAGE_TYPE=core build:portable:windows | PASS | worker/DLL present; gallery-dl, Extension and component files absent; manifest correct |
| Full portable boundary | Required | PORTABLE_PACKAGE_TYPE=full build:portable:windows | PASS | worker/DLL, E: gallery-dl and Extension present; manifest correct |
| Core/Full startup smoke | Applicable | current Core and Full exe, 8-second process smoke | PASS | Both stayed alive and wrote application runtime initialized; not setup/download acceptance |
| U7 aria2/extraction/refresh/commit/recovery | Required | No controlled aria2/media/signed-URL/filesystem/restart fixtures | BLOCKED | No real transfer or recovery conclusion |
| U9 asset activation/filesystem | Required | No real catalog/assets/ACL/reparse/lock fixture | BLOCKED | Empty embedded catalog is expected until U11 assets exist |
| U10 Bootstrap setup/manual GUI | Required | Native GUI helper unavailable; no controlled Downloads/ACL/marker fixture | BLOCKED | Process smoke and WDIO dashboard pass do not cover Settings Bootstrap/manual setup |
| U12 Registry/ACL/browser/Native Host | Required | No fixed release host asset, browser profile or Registry/ACL fixture | BLOCKED | Contract tests pass; real browser endpoint not verified |
| U13 Offline Bundle parity/signature | Required | No final six-component Windows asset set or signed bundle | BLOCKED | Contract tests pass; actual bundle/license/hash/signature not verified |
| v0.2.0-pre.3 external Release workflow | Required | Historical run 35492155317 remains the recorded evidence | NOT RUN | External workflow not retriggered; historical tag build FAIL remains |
| v0.2.0-pre.3 release asset hash/license/parity | Required | No assets produced by failed workflow | NOT RUN | No exe/7z/SHA256SUMS/final bundle available |
| Native Computer Use GUI | Applicable | CUA helper initialization failed twice with helper_unknown_error | BLOCKED | Manual steps remain required; no product failure inferred |

#### Errors and Classification

| Step | Error summary | Classification | Blocks other validation | Follow-up |
|---|---|---|---|---|
| Initial Sidecar full pytest | E: stale gallery.py/test_gallery.py/test_worker.py/test_scaffold.py and old entrypoints were collected; ImportError and legacy-surface assertion occurred | FAIL_TEST / sync-workspace contamination | Sidecar tests only | Current-source rerun passed 21/21; keep stale-file audit |
| Initial advanced WDIO | Production binary lacked test-only Tauri plugin API; two wdio-plugin tests failed | BLOCKED_ENV prerequisite | Advanced WDIO only | Build E2E artifact first; rerun passed 4/4 |
| Native Computer Use | helper_unknown_error on both initialization attempts | BLOCKED_AUTOMATION | Native GUI/manual scenarios only | Execute documented manual WebView2/selector/ACL steps when helper is available |
| Historical pre.3 Release workflow | Run 35492155317 failed in Rust tests with two sidecar v2 handshake timeouts; no assets uploaded | External CI/release failure | Release asset checks | Re-run final tagged workflow after release/CI decision; do not infer from local build |
| WDIO diagnostics | Disk space could not be determined; other diagnostics passed | Environment warning | No | No product impact observed |

#### Not Executed / Blocked

- WQ-U7-02 through WQ-U7-05: aria2 multi-GID, expired URL refresh, staging/commit, cancel/shutdown/recovery and late-result fencing; controlled fixtures unavailable.
- WQ-U9-02/WQ-U9-03: ACL, reparse/junction, file-lock/atomic activation and real component executable probes; real catalog/assets unavailable.
- WQ-U10-02/WQ-U10-03: Known Downloads, cross-volume/readonly/ACL setup and component activation; manual fixtures and real assets unavailable.
- WQ-U12-02 through WQ-U12-04: Registry/ACL, Edge/Chrome developer mode, Native Host reconnect and browser restart; fixed host asset/browser environment unavailable.
- WQ-U13-01 through WQ-U13-04: actual Offline Bundle assembly, extraction/path/license scan, bootstrap parity, signing and upload; final component set/certificate unavailable.
- WQ-U11-01 external release workflow was not retriggered; the documented pre.3 FAIL remains current evidence for that tag.
- WQ-U11-02 through WQ-U11-04 were not run because the failed workflow produced no release assets.

#### Queue Result and Linux Follow-up

- WQ-P0-01: WINDOWS_PASS for the current local Windows baseline.
- WQ-U8-01: WINDOWS_PASS for current v2-only source/worker/WDIO scope after stale E: files were isolated.
- WQ-U7-02 through WQ-U7-05: WINDOWS_BLOCKED.
- WQ-U9-01: WINDOWS_VERIFICATION_PENDING; Windows build/startup and contract tests pass, real catalog/asset validation pending.
- WQ-U9-02/WQ-U9-03: WINDOWS_BLOCKED; WQ-U9-04 remains WINDOWS_VERIFICATION_PENDING.
- WQ-U10-01/WQ-U10-04: WINDOWS_VERIFICATION_PENDING; process/WDIO smoke pass but Bootstrap screen, marker and manual setup acceptance were not completed.
- WQ-U10-02/WQ-U10-03: WINDOWS_BLOCKED.
- WQ-U11-01: WINDOWS_FAIL for the recorded v0.2.0-pre.3 GitHub Actions release workflow; current local Tauri/WDIO/build checks do not clear the tag-level CI failure.
- WQ-U11-02 through WQ-U11-04: NOT RUN.
- WQ-U12-01: WINDOWS_VERIFICATION_PENDING; contract tests pass but real release package and host executable are absent.
- WQ-U12-02 through WQ-U12-04: WINDOWS_BLOCKED.
- WQ-U13-01 through WQ-U13-04: WINDOWS_BLOCKED.
- No WINDOWS_VERIFICATION_BLOCKING item was introduced.

Linux follow-up tasks:
1. Keep the stale-file audit in the Linux→E synchronization procedure; do not treat preserved E: extras as current source.
2. Provide or schedule a final tagged Windows workflow run for v0.2.0-pre.3 or a replacement tag, then inspect the Rust v2 handshake failure before publishing assets.
3. Prepare real U9/U10 component catalog/assets and Windows filesystem/ACL/reparse/marker fixtures.
4. Prepare aria2, signed URL, restart/recovery and late-result fixtures for U7.
5. Prepare fixed Native Host/Extension/browser assets and a signed Offline Bundle for U12/U13.
6. No business-code change was made in this validation round; do not expand the task into feature development merely to clear blocked release or manual scenarios.
### 2026-09-20 current dirty UI/Extension/Native Host/release revalidation

#### Validation scope

本轮以 Linux source 为唯一事实源，基于当前 Plan、working tree diff、Windows queue、历史 Windows 结果和项目脚本合并确定范围。当前改动命中 Desktop UI/layout/interaction、Extension status/reconnect、Native Host portable packaging、Windows release workflow 与相关 Node contract tests，因此重新执行 Node workspace、Tauri build、Windows worker v2 probe、Core/Full portable assembly、Full startup smoke 和 ordinary/advanced native WDIO。Rust/Sidecar 全量逻辑测试仅在没有 Rust/Sidecar diff 的前提下沿用最近有效结论；本轮用 Tauri/native-host Windows build 复核实际编译链。

本轮不把历史 pre4 GitHub Actions 成功外推到当前 dirty workflow，也不把 local portable assembly 外推为最终 Release asset hash/license/signature/parity。U7 真实 extraction/transfer/recovery、U9/U10 component activation、U12 Registry/browser/Named Pipe、U13 final Offline Bundle 继续按前置条件单独处理。

#### Validation environment and source state

- Windows: Microsoft Windows 11 Professional Workstations Insider Preview, 10.0.29671, build 29671, x64.
- Node/npm: Node v24.19.0, npm 11.17.0.
- Rust: rustc 1.98.0 (88d9e12ae 2026-08-18), cargo 1.98.0 (797e8a9bc 2026-08-05).
- WebView2/Edge target used by tauri-service: 153.0.4234.48; matching msedgedriver was downloaded by the test service.
- Linux source branch: feature/u7-desktop-production-integration.
- Linux source HEAD: bd3e58ddf064ab015a3c04036086a01a871062e6, docs: record successful pre4 Windows release.
- Working tree: dirty; 19 tracked files contained pre-existing UI/Extension/Native Host/release and documentation changes before this report. This validation therefore covers working-tree changes and is not a pure commit validation.
- Windows validation workspace: E:\Shiraishi\VSCode Workspace\Tw2Tg.
- One-way sync: Linux UNC source to E: with controlled Robocopy; exit code 3, 171 files copied, 68 skipped, 0 mismatch, 0 failed. .git, dependencies, virtual environments, Rust target, caches, logs, validation-artifacts and machine-local directories were excluded and preserved. Validation docs remained Linux-only.
- Representative SHA-256 checks for the workflow, portable scripts, UI state/settings and Extension background/test files matched between source and E:.

#### Validation results

| Validation item | Status | Actual command / evidence | Result |
|---|---|---|---|
| Linux to Windows controlled sync and source parity | PASS | Robocopy plus representative SHA-256 | E: current source matches selected Linux files; local dependencies and artifacts preserved |
| Node workspace check | PASS | npm run check | Vite build and Extension syntax checks passed |
| Node workspace tests | PASS | npm test | Desktop 46/46 and Extension 10/10 passed |
| Node workspace build | PASS | npm run build | Desktop and Extension build passed |
| Tauri Windows release build | PASS | npm run build:tauri | release xarchive-desktop.exe built; MSVC linker emitted non-blocking stdout notes |
| Tauri WDIO build | PASS | npm run build:tauri:wdio --workspace desktop | E2E-featured release binary built successfully |
| Windows ordinary native WDIO smoke | FAIL | npm run test:e2e --workspace desktop | tauri-driver and WebView2 session started, but h1 never became visible within 20.1 seconds; 0 passed, 1 failed |
| Windows advanced native WDIO | FAIL | npm run test:e2e:windows:advanced --workspace desktop | dashboard.e2e.mjs and wdio-plugin.e2e.mjs both failed before assertions because the dashboard did not become visible; 0 passed, 2 failed |
| Native Host Windows release build | PASS | cargo build -p xarchive-native-host --release | xarchive-native-host compiled successfully |
| Core portable assembly | PASS | PORTABLE_PACKAGE_TYPE=core; npm run build:portable:windows --workspace desktop | executable, v2 worker and Core manifest created; gallery-dl, Extension and Native Host excluded |
| Full portable assembly | PASS | PORTABLE_PACKAGE_TYPE=full; local test Extension ID; npm run build:portable:windows --workspace desktop | executable, worker, gallery-dl, Extension, Native Host and host manifest created; local synthetic ID only, not release-secret validation |
| Full portable startup smoke | PASS | start validation-artifacts/current-dirty-20260920/portable-full/xarchive-desktop.exe; observe 8 seconds; close/cleanup | process remained alive for 8 seconds and was fully cleaned up |
| Current packaged worker help | PASS | sidecar/xarchive-downloader/xarchive-downloader.exe --help | exit 0; reports XArchive Sidecar protocol v2 worker |
| Current packaged worker v2 JSONL | PASS | v2 hello, unknown executable field, shutdown | hello returned ready/capabilities; unknown field returned INVALID_COMMAND; process exit 0 |
| Rust/Sidecar full suites | PASS (KEEP_VALID) | recent Windows evidence; no Rust/Sidecar source diff in this round | latest applicable Windows Rust/Sidecar results remain valid; not redundantly rerun |
| External v0.2.0-pre.4 release workflow for current dirty tree | NOT RUN | no external GitHub runner invocation in this round | historical pre4 run remains valid for its recorded source/tag, but does not validate current dirty release workflow |
| Final release asset hash/size, license/source scan, signature and Core/Full/Offline parity | NOT RUN | no final asset set was downloaded into this validation workspace | local assembly is insufficient for final release acceptance |
| U7 extraction/transfer/refresh/commit/recovery | BLOCKED | controlled aria2, signed URL/media, Windows file-lock/reparse and restart fixtures unavailable | handshake and startup do not prove production runtime |
| U9/U10 activation, ACL, marker, rollback and known Downloads setup | BLOCKED | real versioned component assets and Windows filesystem fixtures unavailable | no Windows activation or rollback evidence |
| U12 Registry/ACL, Edge/Chrome load and Native Host reconnect | BLOCKED | fixed release Extension ID, browser profile and Registry/Named Pipe environment unavailable | contract/package checks do not prove browser integration |
| Manual settings/path/DPI/accessibility GUI acceptance | BLOCKED | native GUI automation helper was unavailable; WDIO stopped before dashboard render | requires a usable Windows GUI/manual session after render issue is diagnosed |
| Formal installer/updater/signing acceptance | NOT APPLICABLE | current project workflow defines portable EXE/7z assets, not a separate installer/updater target in this round | no installer-specific command was defined for current scope |

#### Errors and analysis

1. The only new current-scope product-facing failure is native WDIO rendering. Both ordinary and advanced runs successfully created a WebView2 153.0.4234.48 session and initialized tauri-driver, then repeatedly received no h1 element and failed the dashboard wait. Teardown reported surviving driver processes and tree-killed them. The same symptom reproduced in two independent commands.
2. Node tests, Vite build, Tauri release build, worker probe, portable assembly and short startup all passed. Therefore the evidence narrows the issue to the Windows native WebView2 render/startup path or the interaction between the current UI/E2E build and the native test harness; it does not prove whether the root cause is UI code, E2E feature injection, packaged asset loading or machine-local WebView2 state.
3. Relevant logs are retained in E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\current-dirty-20260920\wdio-smoke.log and wdio-advanced.log. Build/probe logs are in the same directory. No business-code fix was attempted.
4. Full package used the test-only ID abcdefghijklmnopabcdefghijklmnop because the release secret was not available in this local validation session. The resulting package proves local manifest/layout assembly only; it is not evidence for the real release Extension ID or browser registration.
5. No new Rust/Sidecar failure was observed; prior Windows full-suite and current v2 worker evidence remain scoped to their recorded revisions and prerequisites.

#### Queue result and Linux follow-up

- WQ-P0-01: WINDOWS_PASS for the current local Node/Tauri/Native Host build and contract scope; native WDIO GUI regression is separately FAIL.
- WQ-P1-16 / WQ-P1-17: WINDOWS_FAIL for current dirty UI/native WDIO scope. The previous KEEP_VALID result is invalidated because the current diff directly changes UI layout and interaction.
- WQ-U7-01 / WQ-ARCH-01: WINDOWS_PASS limited to the current packaged v2 hello/capability/unknown-field/shutdown probe; real extraction remains blocked.
- WQ-U7-02 through WQ-U7-05: WINDOWS_BLOCKED; controlled aria2, signed URL/media, Windows filesystem and restart/recovery fixtures are missing.
- WQ-U9-01/U9-04 and WQ-U10-01/U10-04: WINDOWS_VERIFICATION_PENDING or BLOCKED as applicable; local build/startup does not establish component activation, marker, setup or full GUI acceptance.
- WQ-U11-01: NOT RUN for the current dirty workflow; historical v0.2.0-pre.4 WINDOWS_PASS remains tied to run 35497313604 and its recorded source/tag. WQ-U11-02/U11-03/U11-04: NOT RUN until real release assets are available.
- WQ-U12-01: WINDOWS_PASS limited to local Full package/manifest assembly with a synthetic ID; WQ-U12-02/U12-03/U12-04: WINDOWS_BLOCKED.
- WQ-U13-01 through WQ-U13-04: NOT RUN or BLOCKED pending final Offline Bundle, catalog/hash/license/signature and parity evidence.
- No WINDOWS_VERIFICATION_BLOCKING item was introduced.

需要 Linux 后续处理的问题：

1. 优先诊断当前 dirty UI/native WDIO FAIL：在 Windows 上取得前端 console/backend log、确认 E2E-featured binary 的 asset loading 和 VITE_WDIO_E2E plugin 注入，并用人工 GUI 复核 dashboard 是否实际白屏；本轮不直接修改业务代码。
2. 若确认是项目问题，再在 Linux 修复并执行对应 Linux 回归后重新同步和重跑 ordinary/advanced WDIO。
3. 提供真实 release Extension ID/secret 与发布 runner，重跑当前 release workflow；随后执行 pre4/current asset hash/size、license/source、signature 和 Core/Full/Offline parity。
4. 准备 U7 aria2/signed URL/media/file-lock/reparse/restart fixtures，及 U9/U10 activation/ACL/marker/rollback fixtures。
5. 准备真实 Edge/Chrome developer-mode、Registry/Named Pipe/Native Host reconnect 和手工 Settings/setup/DPI/accessibility 环境；不要把 local package/handshake/startup PASS 扩大解释为这些系统级验收。

### Linux reconciliation after 2026-09-20 current-dirty Windows validation

Linux source 当前为 branch `feature/u7-desktop-production-integration` / HEAD `bd3e58ddf064ab015a3c04036086a01a871062e6`，working tree dirty，包含本轮 UI、Extension、Native Host packaging、release workflow 和文档修改。根据最新 current-dirty Windows 结果完成 reconciliation：

| 范围 | 当前结论 | Linux action |
|---|---|---|
| UI / Extension / path controls | Linux implementation 已完成；Windows native WDIO ordinary/advanced FAIL，Dashboard `h1` 未渲染 | 不修改生产 UI 以猜测修复；执行 Linux targeted regression；等待 Windows render/session 诊断 |
| NativeBridge | Linux bridge regression 通过；Windows 真实 Registry/browser/reconnect 未执行 | 保持实现；真实集成继续 `WINDOWS_BLOCKED`/`WINDOWS_VERIFICATION_PENDING` |
| Native Host package | Windows local package boundary 通过，但使用 synthetic Extension ID | 保留 package contract；真实 release ID、Registry 和 browser integration 不标 PASS |
| Release workflow/assets | 当前 dirty workflow 未在 Windows runner 执行；pre.4 PASS 属于历史 tag/source | 不把历史 release PASS 外推到当前 diff；等待最终 source/tag workflow |

本轮没有确认新的 Linux 产品缺陷。Linux 适用验证完成后，继续保留 WQ-P1-16/WQ-P1-17 的 `WINDOWS_FAIL`，并将 Registry/Edge/Chrome/Named Pipe/real archive request/release parity 项目按现有原因保持 pending/blocked/not run。
### 2026-09-20 incremental Windows revalidation after Linux reconciliation

#### Scope decision

当前 Linux HEAD 仍为 bd3e58ddf064ab015a3c04036086a01a871062e6，branch 仍为 feature/u7-desktop-production-integration。与上一轮 current-dirty Windows 验证相比，没有新的业务代码、依赖、工具链或 Windows 影响区变化；新增变化仅为 Linux 验证文档 reconciliation。因此按 cross-platform-validation.md 的增量规则重新同步并重跑受影响的 Node/Rust/Native Host 门禁，保留 ordinary/advanced WDIO 的当前 FAIL 结论，不重复执行相同的原生 GUI 场景。

#### Validation environment and synchronization

- Windows workspace: E:\Shiraishi\VSCode Workspace\Tw2Tg.
- Linux source branch/HEAD: feature/u7-desktop-production-integration / bd3e58ddf064ab015a3c04036086a01a871062e6.
- Linux working tree: dirty with the same UI, Extension, Native Host packaging, release workflow and validation-document changes; no new business-code change since the previous Windows run.
- One-way Robocopy: exit 3, 171 copied, 68 skipped, 0 mismatch, 0 failed. .git, node_modules, virtual environments, target, caches, logs, validation-artifacts and Windows machine-local data remained excluded/preserved.
- Representative hashes for the workflow, portable scripts, UI state/settings and Extension files matched source and E:.

#### Results

| Validation item | Status | Evidence |
|---|---|---|
| Linux to Windows source sync/parity | PASS | Robocopy plus representative SHA-256 parity |
| Node workspace check | PASS | npm run check; Vite and Extension syntax passed |
| Node workspace tests | PASS | npm test; Desktop 46/46 and Extension 10/10 |
| Node workspace build | PASS | npm run build |
| Rust formatter | PASS | cargo fmt --all -- --check |
| Rust workspace compile check | PASS | cargo check --workspace --all-targets |
| Native Host tests | PASS | cargo test -p xarchive-native-host --no-fail-fast; 8 passed |
| Tauri release build | PASS (KEEP_VALID) | no Rust/UI source impact since last successful Windows release build |
| Worker v2 probe and Core/Full package assembly | PASS (KEEP_VALID) | no worker/package source impact since last successful probe and assembly |
| Ordinary/advanced native WDIO | FAIL (carried forward) | previous current-dirty run reproduced no dashboard h1 in ordinary 0/1 and advanced 0/2; not rerun because the exact impact area and prerequisites are unchanged |
| U7 real extraction/transfer/recovery | BLOCKED | controlled aria2, signed URL/media, filesystem and restart fixtures still unavailable |
| U9/U10 activation, ACL, marker, rollback and setup GUI | BLOCKED | real assets and Windows filesystem/manual GUI prerequisites still unavailable |
| U12 Registry/Edge/Chrome/Named Pipe/reconnect | BLOCKED | release Extension ID, browser profile and system integration environment still unavailable |
| Current dirty external release workflow and final asset hash/license/parity | NOT RUN | no external runner or final asset set in this round |
| Separate installer/updater acceptance | NOT APPLICABLE | current workflow scope remains portable EXE/7z |

#### Errors and Linux follow-up

本轮没有新的 Windows compile/test failure。当前唯一未解决的 FAIL 仍是 native WDIO dashboard render failure；它已在上一轮 ordinary 与 advanced 两条独立路径复现，根因仍不能确定为业务 UI、E2E feature injection、asset loading 或机器级 WebView2 状态。没有为了通过验证修改业务代码。

Linux 后续处理：

1. 继续诊断 WDIO/WebView2 dashboard 不渲染问题；若确认是产品问题，再在 Linux 修复并执行针对性回归。
2. 提供最终 release runner、真实 Extension ID/secret 和资产集，执行 release workflow、hash、license、signature 与 parity。
3. 准备 U7、U9、U10、U12/U13 所需 Windows 专用 fixtures 和人工 GUI/浏览器环境。

本轮没有 WINDOWS_VERIFICATION_BLOCKING 项。
### 2026-09-20 Windows WDIO native-render diagnostic and Native Host integration audit

本轮验证继续以 Linux source 为唯一事实来源，并在同步后使用 E: Windows 工作副本执行。Linux source branch 为 feature/u7-desktop-production-integration，HEAD 为 bd3e58ddf064ab015a3c04036086a01a871062e6（docs: record successful pre4 Windows release）；working tree 在同步前包含既有 dirty changes，未将其伪装成纯 commit。同步方向为 \\wsl.localhost\Ubuntu\home\shiraishi\VSCode Workspace\Tw2Tg → E:\Shiraishi\VSCode Workspace\Tw2Tg。Robocopy 结果为 239 files、171 copied、68 skipped、0 mismatch、0 failed；desktop/package.json、desktop/wdio.conf.mjs、desktop/src-tauri/tauri.conf.json、desktop/src/main.jsx、extension/src/background.js 和 docs/development/status.md 的 SHA-256 均与 Linux source 一致。Windows 本地 node_modules、target、validation-artifacts、logs/config/cache 等未被同步覆盖；两份 Linux 验证文档保持 Linux 侧写回。

Validation environment:
- Windows 工作副本：E:\Shiraishi\VSCode Workspace\Tw2Tg
- Windows architecture：win32 x64
- Node：v24.19.0
- Edge executable：C:\Program Files (x86)\Microsoft\Edge\Application\154.0.4258.24\msedge.exe；file/product version 154.0.4258.24
- WebView2 Evergreen registry version：153.0.4234.48
- tauri-driver：C:\Users\Shiraishi\.cargo\bin\tauri-driver.exe
- msedgedriver：由项目服务按 WebView2 153.0.4234.48 自动下载
- Tauri application identifier：com.tw2tg.xarchive
- 项目 WDIO 配置未发现显式 WebView2 user-data/profile 覆盖；本轮在未修改项目代码的前提下额外设置 WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--enable-logging=stderr --v=1 进行诊断。
- 证据目录：E:\Shiraishi\VSCode Workspace\Tw2Tg\validation-artifacts\wdio-diagnostic-20260920

WDIO binary comparison:
- PASS — 普通 release binary。命令：npm run build:tauri --workspace desktop。产物复制为 validation-artifacts/wdio-diagnostic-20260920/ordinary-xarchive-desktop.exe；18241024 bytes；SHA-256 54FD3D09B65287E50BAB44B3103CEAD1C7039DE260E69C973F556763FFA5CBA5。前端构建生成 index.html、index-DQbVMiq2.css、index-D1s3bZ79.js。
- PASS — wdio-e2e binary build。命令：npm run build:tauri:wdio --workspace desktop。该脚本以 VITE_WDIO_E2E=1 和 wdio capability 构建；产物 wdio-e2e-xarchive-desktop.exe 为 18635264 bytes；SHA-256 B89ACD80401998CD0FC389C30D6EBEAB729BDF8E0A1F696034107FBBAD50EE7F。前端构建生成 index.html、index-DQbVMiq2.css、preload-helper-BXl3LOEh.js、event-C4MaYY9A.js、index-B7atKK1J.js、index-CBfXN_ns.js。两种 binary 不同，说明 E2E build 确实加入了额外前端/插件资产；不能据此推断运行时加载成功。

WDIO results:
- FAIL — WQ-P1-16 ordinary WDIO with ordinary release binary。命令：WDIO_APP_BINARY=.../ordinary-xarchive-desktop.exe、WDIO_CAPTURE_LOGS=1、WDIO_LOG_LEVEL=debug、npm run test:e2e --workspace desktop。tauri-driver session 创建成功，window handle 可取得，Tauri plugin initialization complete，WebView2/msedgedriver 为 153.0.4234.48；dashboard spec 等待 20 seconds 后 h1 仍不存在，错误为 XArchive dashboard heading did not become visible / no such element。记录：validation-artifacts/wdio-diagnostic-20260920/ordinary-binary/wdio-console.log。
- FAIL — ordinary WDIO with wdio-e2e binary，对比用以隔离 E2E build 影响。命令同上但 WDIO_APP_BINARY 指向 wdio-e2e-xarchive-desktop.exe。session 和 window handle 同样成功；20 seconds 后 h1 仍不存在。记录：validation-artifacts/wdio-diagnostic-20260920/e2e-binary/wdio-console.log。
- FAIL — WQ-P1-17 advanced WDIO with wdio-e2e binary。命令：WDIO_APP_BINARY=.../wdio-e2e-xarchive-desktop.exe、WDIO_ADVANCED=1、WDIO_CAPTURE_LOGS=1、npm run test:e2e:windows:advanced --workspace desktop。2 workers / 2 specs 均失败：dashboard.e2e.mjs 报 h1 未出现，wdio-plugin.e2e.mjs 报 dashboard 未出现；0 passed、2 failed、约 45 seconds。记录：validation-artifacts/wdio-diagnostic-20260920/advanced-e2e-binary/wdio-console.log。
- PASS — session/driver prerequisite。三次运行均完成 WebView2/tauri-driver session 建立，driver 与 WebView2 版本匹配；failure 发生在 DOM 渲染/元素可见性阶段，不是 session 创建失败。
- PASS with cleanup caveat — 失败路径最终清理。每次运行结束后没有残留 xarchive-desktop、tauri-driver、msedgedriver，TCP 4444/4445 无监听。项目 onComplete 日志显示 upstream teardown 曾留下 2 个 driver process，随后项目自有 tree-kill/reap 逻辑终止并释放端口；这证明最终清理有效，但也暴露出上游 teardown 不是单独可靠的清理保证。
- NOT RUN — Chrome developer-mode loading。当前环境未发现 Chrome executable/on-PATH，且没有真实 Extension ID。
- NOT RUN — 通过真实前端 console/WebView2 stderr 证明 asset load。WDIO console forwarding 注入脚本执行返回 null，未得到可用前端 console 内容；仅能确认 release binary 已构建并包含前端资产，不能确认 WebView2 runtime 实际成功加载这些资产。
- BLOCKED — 根因归属。普通 release 和 wdio-e2e binary 都失败，故当前证据不支持仅归因于 VITE_WDIO_E2E 或 guest JS；Rust/Tauri logs 仅有 application runtime initialized，没有 asset/JS/WebView2 error；前端/浏览器 console 未落盘。因此生产 UI、asset loading、Tauri/WebView2 和 machine-local profile/session 仍未被唯一排除，不能在本轮修改 UI assertion 或 capability。

WDIO logging evidence:
- WDIO 控制台记录了 WebView2 153.0.4234.48、session IDs、no such element 和最终失败。
- validation-artifacts/wdio-diagnostic-20260920/logs/xarchive-1789900143510.log、xarchive-1789900250271.log、xarchive-1789900324701.log、xarchive-1789900347541.log 各只包含 level=info 与 application runtime initialized。
- 项目服务显示 Log capture initialized: E:\Shiraishi\VSCode Workspace\Tw2Tg\desktop\logs；本轮自定义 logDir 产生了 WDIO console 文件，但没有产生有效的前端 console 或后端详细日志。该日志捕获缺口需后续诊断任务处理，不应解释成“没有前端错误”。

Native Host results:
- PASS — Windows cargo contract build/test。cargo build -p xarchive-native-host --release 成功；cargo test -p xarchive-native-host 为 8 passed、0 failed。
- PASS — Native Host package contract。node --test desktop/test/native-host-package.test.mjs 为 4 passed、0 failed；测试确认 fixed-length Extension ID、MV3 manifest、Native Messaging manifest 和 installation manifest 结构，且无 Registry side effect。
- PASS — synthetic Full package boundary。使用明确标注的 synthetic Extension ID aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa、普通 release binary 和 release Native Host binary 生成 validation-artifacts/wdio-diagnostic-20260920/synthetic-full；package-manifest.json、native-host/com.tw2tg.xarchive.json、extension 和 xarchive-native-host.exe 文件边界生成成功。此结果只证明 local package boundary，不代表真实发布 Extension ID 或真实浏览器集成。
- BLOCKED — WQ-U12-02 real user Registry registration/repair/unregister。XARCHIVE_EXTENSION_ID 未设置；HKCU/HKLM 下 Chrome 与 Edge 的 com.tw2tg.xarchive NativeMessagingHosts key 均不存在。项目当前只生成 manifest，没有本轮可执行的 Registry installer/repair/unregister 流程；未写入 Registry。
- BLOCKED — WQ-U12-03 real Extension ID and Edge/Chrome developer-mode loading。真实发布 Extension ID 不可得；Chrome executable 未发现，Edge 虽安装但没有真实 ID/package registration。未加载 synthetic ID 作为真实验收。
- BLOCKED — WQ-U12-04 Native Host → Desktop real Windows transport。crates/xarchive-native-host/src/main.rs 的 Windows 分支使用 OpenOptions 打开 XARCHIVE_PIPE_ENDPOINT；desktop/src-tauri/src/transport.rs 的 DesktopTransportServer 仅在 cfg(unix) 下实现 UnixListener，源码明确将 Windows Named Pipe backend 留作平台特定后续项。当前没有可验证的 Windows Named Pipe server。
- BLOCKED — WQ-U12-04 reconnect/pending request/query_status/archive_request。由于 Windows transport server 和真实 Registry/browser path 均缺失，无法执行真实 Native Host forwarding、断线重连、pending request、query_status 或 archive_request 创建 Job。Linux/unit contract 不升级为 Windows real-integration PASS。
- NOT APPLICABLE for this validation run — Registry repair/unregister side effects against a real published package。没有真实发布 ID、用户安装位置或允许写入的真实注册目标；强行写入 synthetic ID 会改变验证含义，故不执行。

Errors and likely causes:
1. WQ-P1-16/WQ-P1-17 的直接错误是 WebDriver no such element: h1，且 20 seconds 内元素始终为空。session、window handle、driver version 和 Tauri plugin initialization 均成功。最可能范围仍为 WebView2 页面 document/asset load、Tauri embedded frontend runtime 或 machine-local WebView2/session/profile；当前没有足够证据将其归给业务 UI、E2E injection 或 driver capability。
2. 前端 console/Rust/Tauri/WebView2 详细日志未被有效保存；仅有 application runtime initialized。该诊断基础设施缺口阻塞根因确认，但不是把测试标为 PASS 的理由。
3. WDIO upstream teardown 留下 2 个 driver，需要项目自有 tree-kill 才完成清理。最终资源状态干净，但应继续检查 process-tree ownership 和正常/异常路径 cleanup 的稳定性。
4. Native Host Windows 失败不是一次运行时失败，而是明确前置/实现缺失：real Extension ID、Registry lifecycle、Windows Named Pipe server、真实 browser load 与 transport wiring 当前不可用。

Linux 后续处理:
- 保持 WQ-P1-16/WQ-P1-17 为 WINDOWS_FAIL，不调整 h1 assertion、等待语义或 capability 来制造通过。
- 后续 Windows 诊断任务应先让前端 console、Rust/Tauri tracing 和 WebView2/msedgedriver stderr 落到每次运行独立且可读的目录；同时记录应用启动参数、WebView2 user-data/profile、application identifier 和 asset URL/load error。
- 对 ordinary 与 wdio-e2e binary 做同一套页面/asset 采样；只有发现项目代码或 build asset 逻辑错误后，才在 Linux 修复并重新同步。当前没有证据授权修改业务代码。
- 若仍需定位 machine-local profile/session，应在不影响用户 Edge profile 的隔离目录中复现，并保留成功/失败路径的进程树、session、端口和 cleanup 证据。
- 为 WQ-U12-02~U12-04 另开实现/集成任务：获取真实发布 Extension ID；实现并验证当前用户 Registry install/repair/unregister；实现 Windows Named Pipe Desktop server 与 ACL；再执行 portable move path repair、Edge/Chrome developer-mode load、Native Host forwarding、reconnect/pending request、真实 query_status 和 archive_request Job。
- 不把 Linux UnixStream/contract tests、synthetic package 或 host manifest generation 记录为真实 Windows Native Host 集成通过。
### 2026-09-20 Follow-up WDIO process/profile diagnosis

本轮再次使用 E:\Shiraishi\VSCode Workspace\Tw2Tg 上的 ordinary release binary 运行 ordinary WDIO，并将 stdout/stderr 保存到 validation-artifacts/wdio-cdp-20260920/wdio.stdout.log 与 wdio.stderr.log。

结果仍为 FAIL：WebView2/tauri-driver session 成功，Session ID 为 7257b05caaf3dcc8f0bbf7bffac8d4cf；dashboard.e2e.mjs 约 20.1 seconds 后报 XArchive dashboard heading did not become visible，WebDriver 返回 no such element。最终无残留 xarchive-desktop、tauri-driver、msedgedriver，4444/4445 无监听。

本轮从运行期间的 Win32 process command line 获得了更直接的环境证据：
- 目标进程为 E:\Shiraishi\VSCode Workspace\Tw2Tg\target\release\xarchive-desktop.exe，未误启动其他旧 binary。
- WebView2 使用独立临时 profile：C:\Users\SHIRAI~1\AppData\Local\Temp\scoped_dir93016_722944289\EBWebView。
- WebView2 renderer 带有 --embedded-browser-webview-dpi-awareness=2、--enable-automation、--test-type、--remote-debugging-port=0、--device-scale-factor=2。
- 当前 runner 没有向日志暴露可连接的 DevTools WebSocket；因此截图中类似 ws://127.0.0.1:63940/devtools/browser/... 的 endpoint 不能直接视为本轮 WDIO session 的 endpoint。下一次诊断必须在 runner 层显式暴露 CDP endpoint，或使用项目允许的独立 WebView2 调试启动方式，然后通过 CDP 采集 document、resource、console 和 runtime exception。
- Rust/Tauri 有效日志仍未包含 asset/JS 错误；本轮 WDIO stdout 只提供 WebDriver failure 证据。此前截图中的 DPI awareness failure 与 Edge LLM not supported on WebView2 应记录为 WebView2 环境噪声，当前不能单独作为应用渲染根因。

新的判断：应用 binary、driver session、临时 profile 和清理路径均已得到直接证据；未确认的核心仍是 WebView2 document/asset load 与前端 runtime。WQ-P1-16/WQ-P1-17 继续保持 FAIL，不修改断言或 capability。### 2026-09-20 Native Host real Windows integration conditions and execution method

当前 Windows 只能完成 local package/contract 验证，不能完成真实 Native Host 集成。进入真实集成验证前必须同时满足：

1. 发布流程提供真实 32 字符小写 Extension ID，并且 Extension manifest、Native Messaging manifest 的 allowed_origins、installation manifest 和浏览器加载的扩展 ID 完全一致；synthetic ID 不得作为真实验收依据。
2. Windows release package 包含 xarchive-desktop.exe、xarchive-native-host.exe、extension 和绝对路径 Native Messaging manifest；portable move 后必须有可验证的 manifest path repair。
3. Linux 侧先实现 Windows transport：Desktop 作为 Named Pipe server，Native Host Windows 分支作为 Named Pipe client；定义固定 pipe name、当前用户 ACL、连接失败、断线重连和 pending request 行为。当前代码仅有 UnixListener server，Windows Native Host 仍使用 OpenOptions endpoint，因此这项是实现前置，不是环境设置。
4. 提供当前用户 HKCU Chrome/Edge NativeMessagingHosts 的 install、repair、unregister 流程；manifest path 必须是当前 package 的绝对路径，Registry 操作必须可回滚。未明确要求时不写 HKLM。
5. Windows 环境安装 Edge 或 Chrome，并用真实发布 Extension ID 加载扩展；确认 MV3 service worker、nativeMessaging permission、content script 和 background forwarding 均运行。
6. Desktop 启动并监听 Named Pipe 后，使用真实扩展发送 query_status 和 archive_request，验证 request_id 保留、真实 Job 创建、状态查询和 response framing。
7. 分别测试 Desktop 重启、Native Host 重启、Pipe 断开、pending request、重复 archive_request、portable 目录移动后的 repair；每次记录 Registry、manifest path、pipe、process tree 和端口/句柄清理状态。
8. 测试必须使用隔离的 Windows 用户/profile 或明确的测试扩展安装目录；结束时删除测试 Registry keys、扩展加载状态和临时 package，不触碰用户已有浏览器配置。

推荐执行顺序：
- Linux：实现并测试 Windows Named Pipe server/client、Registry lifecycle 和 portable path repair。
- Linux：生成包含真实 Extension ID 的 release package，完成 Linux 文档与 queue 更新。
- Windows：单向同步最终 revision，构建 package，检查 manifest/absolute path/hash。
- Windows：注册 HKCU Native Messaging host，加载真实 Edge/Chrome extension，先验证 host handshake/framing，再验证 transport。
- Windows：执行 query_status、archive_request、reconnect/pending request 和 portable move repair。
- Windows：验证正常/异常路径 cleanup，最后回写 Linux validation documents。

在上述条件满足前，WQ-U12-02 至 WQ-U12-04 应保持 BLOCKED；不得用 synthetic ID、UnixStream 或 contract unit tests 提升为真实 Windows integration PASS。