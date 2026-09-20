# 测试策略

## 测试 Agent 角色与核心原则

本项目采用 Linux 开发与 Windows 平台验证分工：

- **Linux + Cline**：负责功能开发、重构、测试维护，以及所有不依赖 Windows 的 unit、integration、frontend、WDIO Browser Mode 和 Linux 可执行 Native E2E 工作；当前阶段 Linux 工作完成后，再整理 Windows Validation Queue。
- **Windows + Codex**：负责真实 Windows 环境验证，优先执行共享 `@wdio/tauri-service`，随后执行 Windows-only 自动化测试；只有 WDIO 无法稳定覆盖的系统级 GUI 场景才使用 Computer Use，并将事实、失败原因和未执行原因写回 Linux 验证文档。

自动化测试优先于视觉 GUI 自动化，自动诊断优先于人工判断，局部失败不得无条件阻塞不相关测试。不得为了显示绿色而删除断言、降低标准、扩大生产 capability 或把环境问题写成产品失败。

## 目标架构验证矩阵（U1–U14）

下列项目是总体 Plan 的完成标准，不代表当前全部已实现。每个 Unit 进入开发后必须把对应 fixture、测试和验证结果落入代码与文档；未实现项目使用 `PLANNED`，Windows-only 项目使用 `WINDOWS_VERIFICATION_PENDING`。

| Unit | Linux 必须覆盖 | Windows/发布专项 |
|---|---|---|
| U1 | active→`CANCELLED`、active→`INTERRUPTED`、recovery、late-result fencing、cleanup warning、终态幂等 | 应用退出、重启和文件锁时序 |
| U2 | fake child/孙进程、cancel/shutdown、EOF、JSONL 串行化、timeout、无 extracted after cancel | Job Object/process tree、句柄、残留进程和 staging lock |
| U3 | v2 valid/invalid fixtures、legacy command rejection、unknown field、capability、Rust/Python/Schema round-trip | packaged worker handshake 与 artifact protocol probe |
| U4 | extraction-only 无媒体主体文件、稳定 media identity/order、filename、header allowlist、无 signed URL 持久化 | Edge Cookie、真实 X extraction 和 Windows worker |
| U5 | fake aria2 RPC/media server、multi-GID、状态映射、progress monotonicity、error/removed/timeout/cancel | aria2c.exe、进程清理、`.aria2`、Windows 路径 |
| U6 | 403 refresh、旧 GID remove、媒体匹配、refresh 上限、集合变化、敏感 URL 不进事件 | 真实 signed URL expiry 和 Windows filesystem recovery |
| U7 | extraction→plan→transfer→verify→commit、cancel/shutdown、staging/path/hash/identity | Tauri artifact、应用级 SQLite/restart/recovery |
| U9 | embedded catalog、hash/size/layout/license/probe、safe path、atomic activation、rollback | Windows filesystem/permission、EXE probe、真实 release assets |
| U10–U13 | 安全解压、Core/Offline layout parity、asset completeness、Bootstrap/Extension flow | Bootstrap、WebView2、Native Host、Registry、Extension reload |

目标验证不能通过恢复旧 fallback、降低断言或把 Linux 结果外推为 Windows PASS 来完成。U8 后不存在同步 `archive_tweet`、Sidecar v1 `download` command 或 `DownloadRouter` fallback；历史验证章节中的这些名称只表示当时的代码状态。

## 增量验证策略：最小必要范围

验证阶段默认采用**最小必要测试范围**：在保证对当前改动具有足够置信度的前提下，减少重复测试、无关模块构建、Windows 环境切换、GUI 自动化、全量回归和不必要的时间与资源消耗。不因为项目存在完整测试套件，就在每次修改后执行全部测试。

### 范围选择

对每次改动，先根据 git diff、changed files、affected modules、call/dependency graph、public API changes、platform-specific behavior、previous failures 和 test ownership 确定最小验证范围。优先顺序：

1. directly affected tests；
2. affected module tests；
3. affected integration boundary tests；
4. targeted regression tests；
5. broader subsystem tests；
6. full test suite。

只有前一级不足以建立合理置信度时，才扩大验证范围。局部修改默认执行：修改模块直接相关的 unit tests、对应 regression tests、必要的 lint/typecheck、必要的 compile/build check、与修改直接相关的 integration tests。不要默认执行全仓库测试、所有平台测试、完整 installer/package validation、所有 GUI 测试或与当前 diff 无关的模块测试。

### 改动分类与默认范围

| 类别 | 典型例子 | 默认验证 |
|---|---|---|
| A. Documentation-only | README、comments、project docs、validation records | 不运行功能测试；只检查文档格式、链接或相关静态检查 |
| B. Local implementation change | 单个内部函数或模块的 bug fix | affected unit tests、regression test、相关 lint/typecheck、必要时模块 compile/build；通常无需 full suite |
| C. Public API / shared library change | crate 公共 API、共享协议模型 | affected unit tests、direct consumers、相关 integration tests、依赖模块 compile/typecheck；必要时扩大到 subsystem tests |
| D. Cross-module behavior change | 跨模块行为变化 | involved modules、integration boundary、affected workflows、regression tests；影响范围难以确定时升级验证范围 |
| E. Security-sensitive change | 认证/授权、凭据、文件权限、命令执行、网络信任边界 | 直接测试之外，执行相关 security regression tests；必要时扩大到完整相关子系统 |
| F. Platform-specific change | Windows path handling、process spawning、GUI、packaging、sidecar | Linux 阶段：执行所有可在 Linux 完成的相关验证，Windows-only 项目加入 Windows Validation Queue；Windows 阶段：只执行当前改动相关的平台验证，不默认重跑整个 Windows test suite |
| G. Dependency / build-system change | Cargo.toml、package.json、pyproject.toml、lockfile、CI/build scripts | 至少执行 dependency resolution、relevant build、affected tests；必要时升级到 broader/full validation |

### 测试选择

优先寻找：与 changed file 同目录的测试、引用 changed module 的测试、针对 changed function/class/service 的测试、已有 regression test、与 changed API 直接关联的 integration tests。项目支持 test filtering 时优先使用（Rust package/module/test-name filtering、pytest file/class/test selection、workspace affected package filtering 等）。优先使用项目已有脚本，不自行创造新的测试流程。

### 升级规则

最小范围测试出现以下情况时扩大验证范围：direct test failure、unexpected build error、shared API affected、dependency graph 不明确、multiple modules affected、test result 与预期不一致、修改公共基础设施、出现新的 runtime behavior、修改可能影响多个平台、当前问题属于 regression、previous failures suggest broader impact。升级顺序为 `Targeted → Module → Subsystem → Full`，不要直接从 Targeted 跳到 Full，除非存在明确理由。

以下情况建议执行完整测试：release / pre-release、major refactor、architecture change、dependency overhaul、shared core library change、database schema/migration change、large cross-module diff、security-critical change、blast radius 难以确定、targeted tests 反复暴露无关失败、用户明确要求 full regression。日常小改动不满足这些条件时不默认 full test。

### GUI / Computer Use 测试

GUI 和 Computer Use 测试成本高，最后执行。只有当前改动涉及 layout、visual behavior、window lifecycle、interaction、native dialogs 或 GUI-driven workflow 时才默认执行相关 GUI 测试；后端、算法和纯数据层修改不应自动触发完整 GUI 回归。Computer Use 不可用时，相关测试标记 `BLOCKED`，继续其他验证，最后加入 Manual Windows Validation Queue。

### 验证记录

每轮验证记录：changed scope、selected tests、skipped tests、所选范围为何 sufficient、test results、是否需要更大范围验证。没有运行 full suite 时必须明确记录，例如：`Full test suite not run because current changes are limited to ...`；不得暗示已完成全量验证。验证结束后给出四类结论：**Validated**（本轮实际完成）、**Not required**（当前改动不会影响而未执行）、**Deferred**（计划在 Windows / release / full regression 阶段执行）、**Blocked**（当前无法执行）；若需要扩大范围，明确说明下一层验证范围（Escalation required）。

核心原则：**测试范围应与改动风险匹配，而不是与项目总规模匹配**。默认 `small change → small targeted validation`；只有证据表明影响面扩大时才 `small → module → subsystem → full`。

## 测试层级

### Unit

- Rust：Job 状态、重试、metadata、路径安全、hash、aria2-only plan/driver/refresh、TagEngine、Repository、Telegram formatter、发送状态，以及 Desktop R1 executor 的 submit/query/cancel/shutdown/recovery-scan、active/interrupted candidate、terminal skip、event-ordering、execution spec persistence、runner-owned ExecutorConfig/Database/FileStore/Sidecar context、attempt fencing、运行中 cancellation、late-result fencing、创建/下载开始/下载完成/下载失败/完成 lifecycle event mapping、fake Sidecar crash、事务性状态事件去重、queued/interrupted recovery source state、persisted cancel/shutdown/completion 幂等和状态边界、`DOWNLOADED → COMPLETE`、commit recovery decision/action、`CommitRecoveryFactsProvider` facts/snapshot 一致性、批量 mixed recovery、SQLite 状态/事件/错误字段顺序与单 Job 错误隔离、`EXECUTOR_UNAVAILABLE`/`EXECUTOR_SCHEDULE_FAILED` compensation、后台 executor failure persistence、shutdown interruption 与 worker shutdown 分离、独立 SQLite context、State-independent `ArchiveExecutionContext`、真实 `ArchiveExecutionJob` identity/download/commit/error mapping、runner spec missing failure、control worker 与 single active runner 分离、production Tauri submit wiring、`RuntimeState` ownership、`get_app_status` 生命周期状态、Tauri executor commands、JobSummary→JobSnapshot 字段投影、错误字段保留和 SQLite Job repository contract model、Browser transport adapter 的协议校验、request_id 路由、重复提交、状态查询和错误映射。
- Python：protocol v2 JSONL worker、extraction-only gallery-dl adapter、metadata 归一化、stable media identity/filename 和错误映射。
- JavaScript：DOM 提取、按钮去重、状态映射、Native Bridge、request_id 路由和断线处理。

### Contract

使用 `shared/protocol-schema/fixtures/` 验证 Rust、Python 和 JavaScript 对相同消息的兼容性。修改 Schema 时必须同步更新模型、fixture、producer、consumer 和测试。

### Integration

覆盖 Rust 与 Fake/Real Sidecar、临时 SQLite、临时 FileStore、Native Host fake transport、aria2 fake HTTP server、Telegram fake HTTPS server、Desktop commands 和 Tauri 构建。

### Platform

Windows 专属验证包括 Named Pipe、Registry、Edge/Chrome Native Host、WebView2、长路径、ACL、externalBin、installer、Credential Manager 和真实账号链路。Linux 测试不能替代这些结论。

## Linux 命令

以下命令是按需选用的验证工具箱，不是每次改动必须全部执行的 checklist。每轮验证按上文「增量验证策略：最小必要范围」结合当前 diff 选择其中相关命令；全量组合仅在 full suite 触发条件满足时执行。

### 环境预检

测试开始前只执行与当前范围相关的轻量预检：Git 工作区、Node/npm/pnpm 依赖、Rust/Cargo/Tauri 工具链、项目构建、WDIO 配置、E2E binary，以及适用的 WebView2/WebDriver/embedded driver 前置。预检失败时只阻塞依赖该前置的测试，继续执行独立的静态、unit 和 integration 测试；不要在预检阶段进行无关的大规模环境诊断。

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace --no-fail-fast
cargo clippy --workspace --all-targets -- -D warnings
npm run check
npm run test
npm run build
npm run build:portable:windows --workspace desktop
.venv/bin/python -m compileall -q sidecar/src sidecar/tests
.venv/bin/python -m pytest sidecar/tests -q
npm run test:e2e:windows --workspace desktop
```

如果当前环境没有 `cargo-clippy`，先执行 `rustup component add clippy`；如果没有 `.venv` 或 pytest，先执行 `python3 -m venv .venv` 和 `.venv/bin/python -m pip install -e sidecar pytest`。Sidecar 测试统一使用 `.venv/bin/python -m pytest`，不要依赖系统级 `pytest`。

便携目录组装是 Linux 可执行的 packaging smoke：`npm run build:portable:windows --workspace desktop` 只组装便携目录结构（exe/extension/可选 sidecar），不预创建 `download/`。每轮修改便携布局、配置模型或构建脚本后，应在临时输出目录（例如 `/tmp/tw2tg-portable-smoke`）执行一次组装，核对生成的目录结果并确认 `download/` 未被预创建。

便携 runtime 的 Linux 可验证边界：portable root 解析、路径派生、`config.yaml` YAML 模型、日志等级过滤与 `xarchive-*.log` 轮转、staging root 切换和归档目录命名已由 Rust 单元测试覆盖；`XARCHIVE_PORTABLE_ROOT`、exe 父目录解析、系统 Downloads fallback、跨卷 rename 提交、sidecar/Extension 实际分发和文件权限属于 Windows 行为，Linux 结果不能替代，对应 WQ-P1-18/WQ-P1-19 保持 `WINDOWS_VERIFICATION_PENDING`。

## 测试维护规则

- 新增功能必须增加与边界对应的测试。
- 业务逻辑测试不应依赖真实账号、用户目录或共享数据库。
- 使用临时目录、fake transport 或本地 server 隔离外部系统。
- 失败测试应记录错误分类和是否阻塞其他验证。
- 测试数量变化不是完成标准；行为覆盖和失败边界才是完成标准。
- 测试优先级为：静态/快速测试 → WDIO Browser Mode → Tauri Native E2E → Windows-only 测试 → Computer Use。Browser Mode 已充分覆盖的 UI 逻辑不重复使用 Computer Use。
- Tauri Native E2E 默认优先使用项目配置的 `driverProvider: embedded`；只有出现疑似驱动层问题且已有 fallback 配置时，才尝试 external provider，不临时改写测试架构。
- 单个测试失败时，先诊断再决定是否局部重试；不得无条件终止整个测试套件。

## 单测试结果分类与重试

单个测试用例不得只写含义模糊的 `FAIL` 或 `BLOCKED`。统一使用以下结果状态：

| 状态 | 含义 |
|---|---|
| `PASS` | 首次执行成功，行为、日志和关键异常均符合预期 |
| `PASS_FLAKY` | 初次失败但安全重试后成功，并记录不稳定证据 |
| `PASS_AFTER_FIX` | 修复明确的实现、配置或类型问题后通过相关回归 |
| `PASS_AFTER_TEST_FIX` | 修复测试代码问题后通过相关测试集 |
| `FAIL_PRODUCT` | 产品或平台实现确定性不符合要求 |
| `FAIL_PRODUCT_NEEDS_DEVELOPMENT` | 产品问题需要架构、需求判断或大范围开发评审 |
| `FAIL_TEST` | selector、fixture、mock、expectation、隔离或 WDIO 测试实现错误 |
| `BLOCKED_ENV` | 缺少依赖、服务、凭据、工具链或其他执行环境前置 |
| `BLOCKED_AUTOMATION` | WDIO、embedded/external driver、Computer Use 或 automation session 不可用，暂无产品缺陷证据 |
| `SKIPPED_PLATFORM` | 当前平台明确不适用，例如 Linux 跳过 Windows-only 用例 |
| `NEEDS_REVIEW` | 需求、文档、测试预期和当前实现存在无法自行裁决的冲突 |

失败诊断顺序固定为：测试步骤/断言 → WDIO 输出 → frontend console → Tauri IPC/invoke → Rust/backend 日志 → Windows 系统错误 → 测试代码 → 产品代码 → 环境/自动化基础设施。没有证据时不得直接修改产品代码。

仅对真实可能 transient/flaky 的问题重试，粒度优先为失败用例，其次为 spec，避免无条件重跑完整套件。最多重试 2 次（总执行最多 3 次），建议退避约 1 秒、3 秒；compiler error、确定性断言失败、panic、schema mismatch、权限错误和稳定 frontend exception 不自动重试。重试前只清理该测试自己的状态，不重建整个项目环境。

## 自动修复边界

允许优先修复明确的测试脚本、局部实现、类型/编译或配置问题；修复后必须重新运行失败用例及相关回归，不得只运行单个测试宣布整体解决。不得擅自进行架构调整、数据模型重大变更、安全模型变化、Tauri capability 扩大、删除安全检查、用户数据格式变化或 API breaking change；这些问题记录为 `NEEDS_DEVELOPMENT_REVIEW`，交由开发计划处理。

## 单测试证据与结构化记录

失败用例至少记录 `case_id`、测试名、平台、命令、时间、expected、actual、相关 WDIO 输出、frontend console、backend/Rust log 和 stack trace（如有）；GUI 问题按需记录 screenshot、窗口信息和 route。避免复制与问题无关的大量日志。

单测试结果可使用以下结构：

```json
{
  "case_id": "TC_001",
  "name": "Settings can be saved",
  "platform": "windows",
  "layer": "native_e2e",
  "status": "PASS",
  "category": "None",
  "retry_count": 0,
  "action_taken": "Executed native Tauri E2E through @wdio/tauri-service.",
  "details": {
    "summary": "Settings were saved successfully.",
    "expected": "Settings persist after Save.",
    "actual": "Settings persisted correctly.",
    "reproduction": null,
    "relevant_log": null,
    "root_cause": null,
    "affected_component": null
  },
  "evidence": {
    "frontend_log": null,
    "backend_log": null,
    "screenshot": null
  }
}
```

`layer` 只能使用 `static`、`unit`、`integration`、`browser_e2e`、`native_e2e`、`windows_native` 或 `computer_use`。

最终测试汇总应包含 total、各状态计数、`regression_status`、remaining Windows validation、manual tests、development follow-ups、environment issues 和 flaky tests；汇总不得把 `SKIPPED_PLATFORM`、`BLOCKED_ENV` 或 `BLOCKED_AUTOMATION` 统计为产品失败。

## Windows 验证

Windows 的执行流程、同步方向、状态定义和报告要求见 [`cross-platform-validation.md`](cross-platform-validation.md) 与 [`../validation/windows.md`](../validation/windows.md)。具体历史结果不写入本策略文档。
- WebdriverIO：desktop/e2e/specs/ 的 Tauri 原生窗口 smoke；验证真实窗口 DOM、可见性和稳定区域，不替代 Native Host、真实账号或应用级 IPC。
- 上一轮 Windows 进一步验证：专用 artifact 的 window.wdioTauri、browser.tauri.execute、Dashboard、mock 和日志捕获子项通过；但 advanced teardown 仍有 sessionId warning/driver 残留，普通 artifact 的 service 仍轮询不存在的 plugin，普通 native smoke 未形成干净通过。Linux follow-up 已移除 Windows .cmd 直接 spawn、改用 Node 直接加载 WDIO CLI，并将 availability 断言改为 window.wdioTauri 检查；详见 windows-validation.md。
- tauri-plugin-wdio 高级路径已完成 Linux 配置；使用 `wdio-e2e` feature、独立 capability 和 `wdio-plugin.e2e.mjs` 验证 `browser.tauri.execute`、mocking 与 cleanup。真实 Windows WebView2、日志转发和窗口生命周期仍作为独立 Windows 队列项验证。

Windows 验证优先级为共享 `@wdio/tauri-service` native E2E，其次 Windows-only WDIO，最后才是 Computer Use。Computer Use 仅用于原生文件选择器、通知、托盘、安装程序、原生窗口、DPI/多显示器、拖放、WDIO 无法稳定访问的系统组件和视觉布局验收。若 Computer Use 不可用，相关用例记录为 `BLOCKED_AUTOMATION`，给出手工步骤并继续其他测试；不得据此判定产品失败，也不得重复验证已由 WDIO 稳定覆盖的功能。

Windows-only 项目在 Linux 上应记录为 `SKIPPED_PLATFORM`，而不是 `FAIL_PRODUCT`。Windows 报告中的平台队列状态仍按 [`../validation/windows.md`](../validation/windows.md) 和 [`cross-platform-validation.md`](cross-platform-validation.md) 的 `WINDOWS_*` 状态维护。

## 人工验证 fallback

所有 `BLOCKED_AUTOMATION` 用例必须提供：前置条件、启动应用方式、点击/输入步骤、测试数据、预期结果、失败时应收集的日志、PASS 标准和 FAIL 标准。不得只写“需要人工测试”。

## Linux 端当前 WDIO follow-up

Windows 复验后，Linux 端的自动化工作按以下顺序处理：

1. 已修正 wrapper 对 Windows `.cmd` 的调用和退出码传播；需用 node --check、WDIO 配置加载/dry-run 做无 GUI 检查。
2. 已将 plugin availability 断言统一为通过 `browser.tauri.execute` 检查 `window.wdioTauri`；不得继续使用 `browser.tauri.isTauriApiAvailable`。
3. 已将 `@wdio/tauri-plugin` 的 guest JS 加载与 `VITE_WDIO_E2E=1` 专用构建边界对齐；仍需 Windows 验证普通 release 不触发 WDIO ACL 命令，专用 artifact 仍可 execute/mock/log。
4. 在当前 lockfile 下继续核对 service teardown 的 sessionId、mock store 和 driver 生命周期；该项需要 Windows native session 结果，暂不以手工杀进程替代修复。
5. Windows 端 npm run check/test/build、Tauri 专用/普通构建和 Rust fmt/check/test/clippy 已通过；当前最新 ordinary/advanced 的 Windows native session 仍需按 Windows 队列重验。2026-09-18 Linux 复核在 Ubuntu 26.04、Node v26.7.0/npm 11.19.0、Rust/Cargo 1.98.0、Python 3.14.4 下完成 Node/Rust/Python 门禁、普通/专用 Tauri build、WDIO syntax/config load；安装发行版替代包 `webkitgtk-webdriver` 后 Linux Native WDIO Dashboard smoke 2/2 PASS。R1 Browser transport/executor contract 通过 Desktop tests；R2 application-level aria2 fallback 未执行，因为当前 Sidecar failure contract 不提供 fresh media URL，不能用 fake URL 替代生产语义。仓库没有独立 Browser Mode 配置，记录为 `NOT APPLICABLE`，不临时创建测试架构。该状态取代本段此前的“Linux native WebView/WDIO 为 NOT RUN”表述；历史验证章节中的旧 `NOT RUN` 仍作为历史快照保留。

## Linux 端剩余验证清单（2026-09-15）

### 1. Linux Node 前置（已完成）

当前 Linux 已使用 Linux nvm 提供 Node/npm。已确认 `command -v node` 和 `command -v npm` 返回 Linux 路径，并在 Linux 源目录执行：

    node --version
    npm --version
    command -v node
    command -v npm
    npm ci --no-audit --no-fund
    npm run check
    npm run test
    npm run build
    node --check desktop/scripts/run-wdio-advanced.mjs
    node --check desktop/scripts/build-tauri-wdio.mjs
    node --check desktop/e2e/specs/wdio-plugin.e2e.mjs

还需执行一次 WDIO 配置的无 GUI 加载检查，确认 appBinaryPath、spec 选择、日志目录和 service option 可以被当前 Linux Node 解析；该检查不等于原生 WebView E2E。

### 2. 仅在配置修复后重跑 Linux 静态边界

当前 WDIO 配置通过 `desktop/scripts/wdio-tauri-service.mjs` 复用官方 launcher，并为本项目提供 worker 适配：单窗口 smoke 不执行依赖 `plugin:wdio` 的 focus probe，service 不再与高级 spec 重复执行 mock/session cleanup。普通 `tauri.conf.json`/default capability 仍不注册 wdio，专用构建才使用 `tauri.wdio.conf.json`、wdio capability 和 `VITE_WDIO_E2E=1`。

配置改动后执行：

    npm run build:tauri --workspace desktop
    npm run build:tauri:wdio --workspace desktop
    cargo fmt --all -- --check
    cargo check --workspace --all-targets
    cargo check -p xarchive-desktop --features wdio-e2e --all-targets
    cargo test --workspace --no-fail-fast
    cargo clippy --workspace --all-targets --all-features -- -D warnings

并分别检查普通/专用生成结果：普通 capability schema 不含 wdio permission，普通前端产物不含 wdioTauri；专用 artifact 含 wdio capability 且前端加载 guest JS。这里的静态检查不能替代 Windows WebView2 结论。

### 3. 处理 tauri-service 生命周期问题

项目适配层保留官方 launcher 的 driver/application 生命周期管理，但覆盖 worker 的 `beforeCommand` 和 `afterSession`：前者避免普通 artifact 对不存在的 `plugin:wdio|get_window_states` 发起调用，后者只在存在有效 session 时删除 WebDriver session，不重复触发 mock restore。高级 spec 继续显式调用 `restoreAllMocks()`。

2026-09-16 Windows 重验确认 spec 层（native session、Dashboard `2/2`、plugin API、mock/restore、exit 0）已通过，但成功退出后 `tauri-driver`/`msedgedriver` 与 4444/4445 仍残留。根因是 `@wdio/native-core` 的 `DriverProcess.stop()` 只 kill 直接子进程、无进程树清理。已实现修复：`wdio-tauri-service.mjs` 的 launcher 在上游 teardown 前快照 driver PID 与驱动端口占用者，teardown 后对幸存进程执行进程树 kill（Windows `taskkill /T /F`、POSIX `SIGKILL`），无法清理时使运行失败；netstat 解析与 PID 判定由 `desktop/test/wdio-tauri-service.test.mjs`（`node --test` 8/8）覆盖。修复目标不变：

第二轮 Windows 复验（同日）进一步确认：spec 全过且事后进程/端口均已干净，但 hook 的 5 秒固定 alive-check 在 Windows 误报——`taskkill /F` 成功后 OS 回收未完成，`kill(0)` 仍把已终止 PID 判为存活；同一语义也使 `killTree` 测试因 exit 事件监听挂晚而失败（`7 passed, 1 failed`）。判定规则已改为：child `exit` 事件优先 → 轮询（窗口 10s）→ 超时后以 tracked driver 端口 LISTEN 状态做最终仲裁；stale PID 无监听不再使运行失败。`killTree` 测试已改为在 `killTree` 之前挂 `exit`/`close` 监听。Windows 重验要求见 windows-queue.md「Linux 修复与队列状态更新（2026-09-16 第二轮）」。

- advanced run 不出现 A sessionId is required for this command；
- mock cleanup 只在有效 session 上执行且只执行一次；
- 应用、tauri-driver、msedgedriver 和 4444/4445/1420/9223 在正常及失败退出路径均自动清理；
- 不用 Stop-Process、延长等待或吞掉 warning 伪造 PASS。

### 4. 完成 Linux 后再进入 Windows re-validation

Linux 端 service adapter 等必要配置和 Linux 门禁已完成。Windows 前置稳定后重新执行单向同步，并按以下顺序重验：

1. 专用 wdio-e2e build；
2. advanced plugin E2E：window.wdioTauri、browser.tauri.execute、mock、日志和退出码；
3. session/mock/driver 自动清理；
4. 普通 release build 与 dashboard native smoke；
5. 普通 artifact 的 capability/guest JS/ACL 边界。

WQ-P1-16 与 WQ-P1-17 在 2026-09-16 teardown 修复后回到 `WINDOWS_VERIFICATION_PENDING`；Windows 重验必须覆盖上述顺序，并在成功与失败退出路径均确认自动清理，实际满足完整预期前不得改写为 WINDOWS_PASS。

当前 Windows 重验的前置阻塞包括 Edge/WebView2 native session 的 `DevToolsActivePort file doesn't exist`、Edge driver 自动下载/发现失败，以及 Node worker 的 `uv_os_get_passwd returned ENOMEM`。这些属于 Windows 工具链、进程环境或 native session 前置，不应通过 Linux 业务代码修改规避；前置稳定后才有意义重新判断 WDIO adapter 和应用行为。

### 5. 当前不需要在 Linux 端做的工作

当前 revision 的 Rust fmt/check/feature check/test/clippy、Cargo wdio-e2e 注册、wdio capability、withGlobalTauri、条件 guest JS、跨平台 wrapper 和 plugin spec 已有通过或已完成配置证据；Linux Node 门禁和 WDIO config load 仍待 Linux Node/npm 前置。Native Host/Named Pipe、真实 executor/transport IPC、Windows ACL/reparse/长路径、DPI/键盘/屏幕阅读器、真实 X/Telegram、Credential Manager、installer/signing/updater 继续按 Windows 队列独立执行；本轮不修改业务代码、不升级依赖、不切换 embedded provider。

### Windows WDIO 网络重试补充（2026-09-15）

手动下载并放入临时 PATH 的 msedgedriver 152.0.4191.66 已使 tauri-driver 成功启动；但 @wdio/tauri-service 1.4.0 未识别其 Microsoft Edge WebDriver 版本输出，仍尝试联网下载。advanced 与 ordinary 随后均在 WDIO worker 的 uv_os_get_passwd returned ENOMEM 阶段失败，未进入 spec。WQ-P1-16/WQ-P1-17 仍为 WINDOWS_FAIL。
### Windows WDIO driver 本地前置

Windows WDIO 验证可复用 E: 验证副本中的 msedgedriver：

    $root = "E:\Shiraishi\VSCode Workspace\Tw2Tg"
    $driverDir = Join-Path $root "desktop\test-artifacts\msedgedriver\152.0.4191.66"
    $env:Path = "$driverDir;$env:Path"
    where.exe msedgedriver.exe
    msedgedriver.exe --version

当前 driver 版本为 152.0.4191.66，SHA-256 为 9E9B1F048D2CC781DEEE084E6CB6E9F2F3417A33ED45D96CF7C34BE4EB23077B。driver 目录仅存在于 E: Windows 验证副本的 ignored test-artifacts 下，不同步回 Linux。由于当前 @wdio/tauri-service 1.4.0 可能无法解析该 driver 的 Microsoft Edge WebDriver 版本输出，service 仍可能尝试自动下载；手动 driver 解决的是实际 driver 文件前置，不代表网络 warning、Node worker 或 WebView2 session 已通过。