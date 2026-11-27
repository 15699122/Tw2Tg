# 混合架构现状审计与分阶段路线图（2026-11-27）

本报告基于分支 `dev`（commit `16818c37`）的只读静态审查与双端无副作用诊断输出整理。本文件为**审计结论与规划基线**，不替代实现状态（[`../development/status.md`](../development/status.md)）或 Windows 实机验证记录（[`../validation/windows-queue.md`](../validation/windows-queue.md)）。

---

## 1. 执行摘要

1. **双端基线健康但存在静态检查盲区**：Rust 全工作区 `cargo check`/`clippy -D warnings`/`test --no-run` 零告警通过（共 897 项测试），Python `pytest` 105/105 通过；但 Python 环境未安装 `ruff`/`mypy`，导致 [`gallery_metadata.py`](../../sidecar/src/xarchive_downloader/gallery_metadata.py) 已出现漏写返回值与死代码的真实缺陷 `[已验证: sidecar/src/xarchive_downloader/gallery_metadata.py:L203-L209, L386]`。
2. **发现 3 处高危跨平台/进程边界脆弱点**：① 单帖提取硬编码 `Path("/tmp/xarchive-v2")`，在 Windows 污染盘符根目录、在 Linux 存在多实例互相清空与 `/tmp` 固定路径竞争 `[已验证: sidecar/src/xarchive_downloader/worker_v2.py:L273]`；② 全仓未设置 `PYTHONUTF8=1` 或流重配置，Windows（CP936/CP932）下管道读入含非 ASCII 路径的 JSONL 会触发未捕获 `UnicodeDecodeError` 致工作线程静默退出 `[已验证: sidecar/src/xarchive_downloader/worker_v2.py:L588-L613]`；③ 账号批次发现与手动启停未接入 Windows Job Object / Linux cgroup，终止或崩溃时 `gallery-dl` 会逃逸为孤儿进程 `[已验证: desktop/src-tauri/src/batch/dispatch.rs:L362-L370]`。
3. **跨语言契约与文档存在漂移**：JSON Schema、`crates/xarchive-protocol` 与 Python `protocol_v2.py` 在 `DownloadResult.result`、URL 身份校验强弱、`text` 4000 字符上限及已退役 `download` 能力上不一致 `[已验证: shared/protocol-schema/sidecar-v2-event.schema.json:L41, L97-L123]`；[`README.md`](../../README.md) 仍声明默认关闭 `aria2`，与生产代码强制 `aria2-only` 相悖 `[已验证: desktop/src-tauri/src/production.rs:L74-L95]`。
4. **核心建议**：按 **Phase 1（P0 编码/临时目录/契约对齐）→ Phase 2（子进程容器化统一与孤儿回收）→ Phase 3（Python 静态门禁、Tauri 最小权限与文档收敛）→ Phase 4（Windows 实机终验）** 顺序推进，遵守 Linux/Windows 双 Owner 交接边界 `[已验证: AGENTS.md:L1-L40]`。

---

## 2. 混合架构审计报告

### 2.1 项目上下文与平台归属

| 维度 | 审计推断结论 | 证据标注 |
|---|---|---|
| **项目定位与核心目标** | 本地优先的跨平台桌面端 X/Twitter 归档系统（XArchive / Tw2Tg）：Rust/Tauri 2 为唯一业务状态所有者（任务编排、SQLite、staging 哈希/重解析点校验、aria2 监督、Telegram 发送），React + Vite 提供桌面 UI，MV3 扩展采集页面入口，Python Sidecar 封装 `gallery-dl` 负责 X 元数据提取与账号候选发现 | `[已验证: README.md:L3-L68]`、`[已验证: docs/architecture/overview.md:L1-L39]` |
| **混合架构形态** | **多进程 Sidecar IPC 形态**（无 PyO3 进程内 FFI）：① MV3 扩展 ↔ Native Host（长度前缀 JSON）/ Loopback WebSocket ↔ Tauri Desktop；② Tauri Desktop ↔ Python Sidecar（`stdin/stdout` JSONL Protocol v2）；③ Tauri Desktop ↔ `aria2c`（Loopback HTTP JSON-RPC + 临时 `--conf-path` 密钥） | `[已验证: Cargo.toml:L1-L22]`、`[已验证: crates/xarchive-sidecar-supervisor/src/lib.rs:L238-L276]`、`[已验证: desktop/src-tauri/src/runtime.rs:L408-L454]` |
| **平台归属与环境约束** | 双 Owner Git 交接制：Linux 为 `Cross-platform Owner`（负责共享契约、核心逻辑与跨平台测试）；Windows 11 为 `Windows Platform Owner`（负责 Named Pipe、Win32 Job Object + IOCP、注册表、Credential Manager、PyInstaller/7z 便携包与 WebView2/Edge 实机验收） | `[已验证: AGENTS.md:L5-L75]`、`[已验证: docs/status/platform-handoff.md:L8-L28]` |
| **非目标（明确不做）** | 不支持 macOS 分发；Python 不直连主 SQLite 也不负责 Telegram；浏览器扩展不传 Cookie 或媒体字节；IDM 不进入核心下载链路；不提供动态 `latest` 未签名远程更新器 | `[已验证: README.md:L51-L67]`、`[已验证: docs/development/roadmap.md:L48-L60]` |
| **建议重构幅度** | 允许**单语言模块内修复 + 受控的跨语言契约对齐**（涉及共享契约须标记 `CROSS_PLATFORM_CHANGE_REQUIRED`；保持现有 10 个 Rust crate 边界与 `unsafe_code = "forbid"`，仅 `xarchive-win-job-iocp` 隔离封装 Win32 IOCP） | `[已验证: Cargo.toml:L21-L22]`、`[已验证: docs/architecture/MODULAR_DESIGN_POLICY.md:L20-L28]` |

### 2.2 双端基线体检与双生态依赖核查

| 端别 / 门禁 | 执行命令 | 状态 | 结果与覆盖基线摘要 | 证据标注 |
|---|---|---|---|---|
| **Rust 编译检查** | `cargo check --workspace --all-targets --offline` | **PASS** | `rustc 1.98.0`（与 [`rust-toolchain.toml`](../../rust-toolchain.toml) 一致），10 个工作区成员全部目标 0 错误、0 警告 | `[已验证: CLI cargo check]` |
| **Rust Clippy 静态检查** | `cargo clippy --workspace --all-targets --no-deps --offline -- -D warnings` | **PASS** | 全工作区 lib / bin / test / example 目标 0 警告 | `[已验证: CLI cargo clippy]` |
| **Rust 测试编译与基线** | `cargo test --workspace --no-run --offline` 及核心边界 `--lib` | **PASS** | 25 个测试二进制全部编译通过，全仓登记 **897** 项 Rust 测试；抽检 `xarchive-core` (49/49)、`xarchive-protocol` (31/31)、`xarchive-sidecar-supervisor` (17/17) 共 **97/97 PASS** | `[已验证: CLI cargo test]` |
| **Rust 安全审计** | `cargo audit --no-fetch` | **WARN (0 CVE)** | 扫描 [`Cargo.lock`](../../Cargo.lock) 603 个依赖（本地库 1277 条公告）：0 漏洞，9 条告警——1 条 `unsound`（`glib 0.18.5` `RUSTSEC-2024-0429`，仅 Linux GTK3 链可达，Windows WebView2 不含该包）、1 条 `yanked`（`clipboard-win 5.4.1`，经 `arboard 3.6.1` 引入）、7 条 `unmaintained`（`rustls-pemfile 2.2.0`、`proc-macro-error 1.0.4`、`unic-* 0.9.0`）；在线最新公告库同步因沙箱禁网未核验 | `[已验证: CLI cargo audit --no-fetch]`、`[已验证: Cargo.lock:L85-L95, L466-L472]`、`[未验证: 在线 RustSec 最新增量]` |
| **Python 语法与单测** | `.venv/bin/python -m compileall -q sidecar/src && .venv/bin/pytest sidecar/tests -q` | **PASS** | `.venv` 为 `Python 3.14.4`；收集并运行 **105 项测试全部通过（2.85s，0 失败，0 跳过）** | `[已验证: CLI .venv/bin/pytest]` |
| **Python Lint / 类型检查** | `which ruff mypy` | **MISSING** | [`sidecar/pyproject.toml`](../../sidecar/pyproject.toml) 配置了 `[tool.ruff]`，但 `.venv`、系统 `PATH` 与 `.github/workflows/` 均未安装或执行 `ruff` 与 `mypy` | `[已验证: sidecar/pyproject.toml:L21-L22]`、`[已验证: CLI which ruff mypy]` |
| **Python 依赖与锁文件** | `.venv/bin/pip list` / `uv tree --directory sidecar --offline` | **WARN** | `xarchive-downloader 0.1.0` 运行时 `dependencies = []`（纯标准库）；但 `sidecar/` 无 `uv.lock` 或 `requirements-dev.lock`，测试/打包工具（`pytest 9.1.1`、`jsonschema 4.26.0`、CI 中 `pyinstaller==6.22.3`）缺乏统一锁文件，且 PyPI 在线过期查询因沙箱禁网无法执行 | `[已验证: sidecar/pyproject.toml:L5-L11]`、`[已验证: .github/workflows/windows-worker-artifact.yml:L32-L43]`、`[未验证: pip list --outdated 在线版本对比]` |
| **架构与文档守卫** | `node scripts/check-architecture.mjs`、`node scripts/docs-audit.mjs` | **PASS** | 287 个受控源码文件 0 违规；181 篇 Markdown 0 死链 | `[已验证: CLI check-architecture / docs-audit]` |

### 2.3 Rust-Python 边界与双平台风险清单

| 编号 | 风险域 | 等级 | 归属路由 | 具体脆弱点描述与触发链路 | 代码证据标注 |
|---|---|---|---|---|---|
| **BP-01** | Python 代码缺陷 / 契约类型违约 | **P0** | Linux (`Cross-platform Owner`) | `gallery_metadata.py` 中 `media_type_for_filename()` 仅处理 `video/*` 和 `image/*`，兜底 `return "unknown"` 被误移至文件末尾 `L386`（位于 `to_discovery_candidate` 的 `return` 之后成为死代码）。当媒体无已知 MIME 时返回 `None`，导致类型契约破损并在校验时触发失败。 | `[已验证: sidecar/src/xarchive_downloader/gallery_metadata.py:L203-L209, L386]` |
| **BP-02** | Windows/Linux 路径与并发隔离 | **P0** | Linux (`CROSS_PLATFORM_REVIEW_REQUIRED`) | `handle_extract` 硬编码 `Path("/tmp/xarchive-v2")`（对比 `handle_discover` 已使用 `tempfile.TemporaryDirectory`）：① Windows 下解析为当前盘符根 `<Drive>:\tmp\xarchive-v2`（受限盘符 `PermissionError` 或污染用户根目录）；② Linux 下为全局固定 `/tmp` 路径，存在本地符号链接抢占（TOCTOU）及多 Worker 并发时 `purge_metadata_files` 互删 `gallery-dl.jsonl` 竞态，且退出不清理。 | `[已验证: sidecar/src/xarchive_downloader/worker_v2.py:L273, L512-L513]`、`[已验证: sidecar/src/xarchive_downloader/extraction.py:L106-L110, L146]` |
| **BP-03** | Windows 管道编码与输入行界限 | **P0** | 双端 (`CROSS_PLATFORM_CHANGE_REQUIRED`) | Rust 父进程与 CI 均未注入 `PYTHONUTF8=1` / `PYTHONIOENCODING=utf-8`，Python 入口也未 `reconfigure(encoding="utf-8")`。在中文/日文 Windows（默认管道编码 CP936/CP932）下，Rust `serde_json` 向 `stdin` 写入原始 UTF-8 字节（如含中文路径的 `profile` 或参数）时，`for line in input_stream:` 会抛出**未捕获的 `UnicodeDecodeError`**（仅捕获了 `JSONDecodeError`），导致读取线程静默推入 `None` 终止 Sidecar；同时 `stdin` 读取无单行字节上限（对比 Rust `readers.rs` 限制 1 MiB）。 | `[已验证: sidecar/src/xarchive_downloader/worker_v2.py:L588-L613]`、`[已验证: sidecar/src/xarchive_downloader/__init__.py:L55-L72]`、`[已验证: crates/xarchive-sidecar-supervisor/src/readers.rs:L13, L100-L103]` |
| **BP-04** | 子进程生命周期与孤儿进程清理 | **P0** | 双端 (`CROSS_PLATFORM_CHANGE_REQUIRED`) | 单帖执行已走受控容器（Linux `LinuxContainment` cgroup / Windows `WindowsWriterJob` Job Object `KILL_ON_JOB_CLOSE` + IOCP），**但账号批次发现 `spawn_account_batch` 与手动命令 `start_sidecar` 仍直接调用非受控 `spawn_ready_v2_with_environment`**：① Windows 下无 Job Object，且 `terminate_process_tree` 仅 `child.kill()` 杀 Python 本体，不杀 `gallery-dl.exe` 子树，宿主崩溃或停止时留下孤儿进程；② Unix 非受控路径下 Rust 设 `process_group(0)` 而 Python `process.py` 又对 `gallery-dl` 设 `start_new_session=True` 脱离该进程组，Rust 25ms 后 `SIGKILL` Python，导致 `gallery-dl` 逃逸为孤儿进程。 | `[已验证: desktop/src-tauri/src/batch/dispatch.rs:L362-L370]`、`[已验证: desktop/src-tauri/src/commands/sidecar.rs:L71-L75]`、`[已验证: crates/xarchive-sidecar-supervisor/src/lib.rs:L604-L631]`、`[已验证: sidecar/src/xarchive_downloader/process.py:L28-L32, L68-L87]` |
| **BP-05** | 跨语言契约漂移（Schema vs Rust vs Python） | **P1** | 双端 (`CROSS_PLATFORM_CHANGE_REQUIRED`) | 三处契约存在 4 项不一致：① `sidecar-v2-event.schema.json` 限制 `extraction_result.text` `maxLength: 4000`，而 Rust `sidecar_v2.rs` 与 Python 均无上限（真实 X 长帖可达 25,000 字符）；② Rust `DownloadResult` 含可选 `result` 字段，但 JSON Schema `download_result` 设 `additionalProperties: false` 且不含 `result`；③ Python `validate_extraction_result` 仅用 `tweet_id in url` 子串判断，弱于 Rust `extract_tweet_id` 的严格域名+路径校验；④ 生产已强制 `aria2-only`，但 `REQUIRED_V2_CAPABILITIES` 与 Python Worker 仍暴露已退役的 `download` 命令面。 | `[已验证: shared/protocol-schema/sidecar-v2-event.schema.json:L41, L97-L123]`、`[已验证: crates/xarchive-protocol/src/sidecar_v2.rs:L24-L30, L130-L137, L743]`、`[已验证: sidecar/src/xarchive_downloader/protocol_v2.py:L10-L16, L208-L209]` |
| **BP-06** | Tauri 宿主权限面冗余 | **P1** | Linux (`CROSS_PLATFORM_REVIEW_REQUIRED`) | `capabilities/default.json` 在正式构建中默认授予 `"shell:default"` 与 `"mcp-bridge:default"`，同时 `lib.rs` 无条件注册 `tauri_plugin_shell::init()`；但代码库所有打开目录动作均由受控 Rust 命令实现，未使用前端 shell API，且 `mcp-bridge` 插件仅在 `debug_assertions` 注册。此外 `.gitignore` 未覆盖 `.mypy_cache/` 与 `sidecar/uv.lock`。 | `[已验证: desktop/src-tauri/capabilities/default.json:L1-L7]`、`[已验证: desktop/src-tauri/src/lib.rs:L80-L93]`、`[已验证: .gitignore:L13-L20]` |
| **BP-07** | 环境变量与系统代理跨平台差异 | **P2** | 双端（已部分缓解，需保持守卫） | Rust `SidecarSupervisor` 与 Python `spawn_env` 已实现大小写不敏感移除 `HTTP_PROXY` 等 10 个键，且 PAC/WPAD 多候选策略在 Rust 侧 fail-closed 为 `PROXY_POLICY_NOT_APPLICABLE`；但 `system` 模式下 `gallery-dl` 与 `aria2c` 各自解析 Windows 系统代理的行为一致性尚未在真实 PAC/系统代理切换场景完整实机闭环。 | `[已验证: sidecar/src/xarchive_downloader/gallery_commands.py:L20-L80]`、`[已验证: desktop/src-tauri/src/executor/factory.rs:L194-L216]`、`[已验证: docs/validation/windows-queue.md:L58-L65]` |

### 2.4 性能与通信归因（待验证假设清单）

> 以下未经实测的通信/内存/吞吐风险均归类为**“待验证假设”**，须在完成对应基准测量后方可决定是否调整实现：

| 假设编号 | 待验证假设描述 | 涉及位置 | 可执行测量方案（Benchmark / Profile） |
|---|---|---|---|
| **HYP-01** | **单任务重复冷启动 PyInstaller Sidecar 开销假设**：`ProductionExecutionFactory::create_attempt_execution` 在每次归档任务中均重新拉起 `xarchive-downloader.exe` 并执行 `hello -> ready` 握手；Windows PyInstaller `onedir` 冷启动与防病毒扫描可能占据单帖总耗时的显著比例。 | `desktop/src-tauri/src/executor/factory.rs:L290-L336`、`sidecar/pyinstaller/xarchive-downloader.spec:L24-L49` | 在 Rust `create_attempt_execution` 与 `extract_v2` 埋入 `std::time::Instant` 四段计时（`keeper_spawn_ms`、`sidecar_handshake_ms`、`gallery_dl_extract_ms`、`aria2_transfer_ms`），在 Linux 与 Windows 11 便携版各采集 20 次样本对比 P50/P95。 |
| **HYP-02** | **Rust `SidecarSupervisor` 无界 `mpsc::channel` 内存积压假设**：Python 侧 `commands` 队列已限制 `MAX_QUEUED_COMMANDS = 32`，但 Rust 侧 `readers.rs` 将 stdout/stderr 持续推入无界 `std::sync::mpsc::channel()`；在大号账号执行数万条 `discover` 候选流且 SQLite 落库较慢时，可能产生驻留内存峰值。 | `crates/xarchive-sidecar-supervisor/src/lib.rs:L127-L131, L255-L259` | 构造合成 Sidecar 流式吐出 50,000 条 `DiscoveryCandidate` JSONL，使用 `criterion` 或 `/proc/self/status` `VmHWM`（Windows `GetProcessMemoryInfo`）测量消费延迟 5ms/条下的峰值 RSS，评估是否改用 `mpsc::sync_channel(256)`。 |
| **HYP-03** | **账号发现 `DiscoveryRunner` 轮询全量重读 `gallery-dl.jsonl` 开销假设**：`DiscoveryRunner.run` 在每 `0.1s` tick 中从头全量解析整个 `gallery-dl.jsonl` 文件，随发现条数增长呈 $O(N^2)$ 解析复杂度。 | `sidecar/src/xarchive_downloader/extraction.py:L222-L246`、`sidecar/src/xarchive_downloader/gallery_metadata.py:L96-L106` | 使用 `py-spy record -o discovery.svg -- .venv/bin/pytest` 搭配 5,000 行模拟 `gallery-dl.jsonl` 渐进写入基准测试，测量 CPU 时间与字节读取放大倍数；若确认显著，再改为按文件字节偏移增量读取。 |

### 2.5 代码结构健康度与超 800 行文件评估

仓库已配置 `node scripts/check-architecture.mjs` 行数守卫（当前 0 违规 `[已验证: scripts/architecture-policy.json:L4-L37]`）。Python `sidecar/src/` 最大单文件为 `worker_v2.py`（671 行），无超 800 行生产文件。全仓共 32 个超过 800 行的源文件，按生产净行数归类如下 `[已验证: CLI git ls-files line analysis]`：

- **建议拆分/观察（3 项，P2）**：
  - `desktop/src-tauri/src/config.rs`（总 1136 / 生产 1108 / 测试 28）：可将 `ConfigurationExchange` 拆分为 `config/exchange.rs`。
  - `crates/xarchive-protocol/src/sidecar_v2.rs`（总 1148 / 生产 1071 / 测试 77）：随退役 `download` 契约清理可自然瘦身或按 command/event/validation 拆分。
  - `desktop/src-tauri/src/executor/runtime/startup.rs`（总 1135 / 生产 1135）：待 `Production V2` 门禁决策后按平台恢复适配拆分。
- **高内聚边界，暂不拆分（9 项）**：`application_service.rs` (1047)、`upload.rs` (1033)、`archive.rs` (1019 生产，私有 move-only `PendingArchivePublication`)、`executor.rs` (976)、`executor/runtime.rs` (895，控制面禁止拆散)、`download/supervisor.rs` (877)、`database/jobs.rs` (866)、`runtime.rs` (850 生产)、`linux_containment.rs` (824 生产)。
- **仅可外置测试（1 项，P2）**：`crates/xarchive-workflow/src/windows_containment.rs`（总 1422 / 生产 815 / 测试 607），Windows Owner 可按 P12 模式外置测试。
- **无需处理（19 项）**：8 个“薄生产根（84–781 行）+ 大内联测试”文件（如 `telegram_send.rs`、`batch.rs`、`commands/records.rs` 等）及 11 个纯测试文件（830–2280 行）。

### 2.6 未实现功能盘点（Gap Analysis）与文档漂移

全仓源码中 `unimplemented!()`、`todo!()`、`NotImplementedError` 及 `TODO`/`FIXME` 注释数量均为 **0** `[已验证: CLI git grep]`。主要能力缺口与文档漂移如下：

| 类别 | 模块 / 功能项 | 当前真实状态 | 证据标注 |
|---|---|---|---|
| **文档严重滞后** | **媒体传输后端 `aria2-only` 收敛** | 生产构建已强制 `use_aria2 = true` 且 `set_use_aria2(false)` 拒绝关闭；但 `README.md`、`overview.md`、`status.md` 仍写“默认 `use_aria2 = false` 走 gallery-dl 下载” | `[已验证: desktop/src-tauri/src/production.rs:L74-L95]`、`[已验证: README.md:L20-L41]` |
| **退役半残留** | **Python Sidecar `download` 命令与 `DownloadRunner`** | Rust 生产链路已废弃 `gallery-dl` 媒体下载（仅 `#[cfg(test)]` 保留），但 Python 与 JSON Schema 仍保留完整 `download` 实现 | `[已验证: desktop/src-tauri/src/production/sidecar_operations.rs:L126-L132]`、`[已验证: sidecar/src/xarchive_downloader/extraction.py:L314-L431]` |
| **Fail-Closed 未启用** | **Production V2（`InternalRenameV2`）** | 存储层存在 `v2_rename.rs`，但生产调度与 Windows `recover_v2_on_runner` 未接线，保持 fail-closed | `[已验证: crates/xarchive-storage/src/archive_service/internal_v1_publication.rs:L44-L47]` |
| **Fail-Closed 空占位** | **Core Bootstrap `embedded_catalog`** | `components.rs` 中 `embedded_catalog()` 为空数组（`2026-09-20-u9`），待正式 Release 资产哈希回填 | `[已验证: desktop/src-tauri/src/components.rs:L45-L55]` |
| **策略关闭功能** | **`gallery-dl` 自定义用户参数** | 因 `gallery-dl` allowlist 为空，所有非空 `gallery-dl` 用户参数均被拒绝（仅允许 7 个受控 `aria2` 参数） | `[已验证: crates/xarchive-download/src/downloader_args.rs:L37-L47]` |
| **Windows 实机待验收** | **Edge 轮询、多图/视频、Telegram 发送、Authenticode 签名** | 代码已合入 `dev`（`16818c37`），处于 `READY_FOR_WINDOWS` 待 Windows Platform Owner 执行实机队列验收 | `[已验证: docs/status/platform-handoff.md:L10-L28]` |

### 2.7 已识别的信息缺口（待补充确认）

1. **目标发布里程碑范围（`v0.2.1` GA 边界）**：`v0.2.1` 是否锁定 `InternalV1` 收口并继续保持 `InternalRenameV2` fail-closed？
2. **Windows 代码签名预期**：是否具备 Authenticode 证书，还是接受未签名 `.exe`/`.7z` + `SHA256SUMS` 作为 `v0.2.1` 发布标准？
3. **目标用户 Windows 区域与路径特征**：是否包含中文/日文用户名路径、非系统盘（无 `\tmp`）或企业 PAC 系统代理环境？
4. **单帖最长正文需求**：是否需支持 X Premium 长帖（4,000–25,000 字符）？（当前 JSON Schema 限制为 4,000 字符）
5. **Linux 桌面运行环境定位**：Linux 仅作为无头 CI/跨平台测试环境，还是同样需要打包运行 Linux WebKit2GTK 桌面 GUI（影响 `glib 0.18.5` 暴露面评估）？

---

## 3. 分阶段路线图（Roadmap）与优先级矩阵

### Phase 1：跨语言契约、UTF-8 编码与工作目录安全基线

| 任务 | 优先级 | 归属端 / Owner | 涉及文件 | 前置依赖 | 跨语言契约变更 | 验收标准 (DoD) | 验证方法（环境） | 工作量 | 回归风险 |
|---|---|---|---|---|---|---|---|---|---|
| **T1.1 修复 Python MIME 分类漏返与死代码** | **P0** | Python (`Cross-platform Owner`) | `sidecar/src/xarchive_downloader/gallery_metadata.py`、`sidecar/tests/test_extraction_only.py` | 无 | 无 | `media_type_for_filename` 在未知/空 MIME 时返回 `"unknown"`；移除 `L386` 不可达代码；新增回归单测 | **Linux**：`.venv/bin/pytest sidecar/tests/test_extraction_only.py -q` | S | 低 |
| **T1.2 消除硬编码 `/tmp/xarchive-v2` 固定工作目录** | **P0** | Python (`Cross-platform Owner` + `CROSS_PLATFORM_REVIEW_REQUIRED`) | `sidecar/src/xarchive_downloader/worker_v2.py`、`sidecar/tests/test_protocol_v2.py` | 无 | 无 | `handle_extract` 改用 `tempfile.TemporaryDirectory(prefix="xarchive-v2-extract-")` 每次调用创建独立临时目录并自动清理 | **Linux**：并发双 `handle_extract` 单测；**Windows**：非 `C:` 盘实测无 `\tmp\xarchive-v2` 生成 | S | 低 |
| **T1.3 强制双端管道 UTF-8 编码与 Sidecar 输入行上限** | **P0** | 双端 (`Cross-platform Owner`，`CROSS_PLATFORM_CHANGE_REQUIRED`) | `crates/xarchive-sidecar-supervisor/src/lib.rs`、`sidecar/src/xarchive_downloader/__init__.py`、`worker_v2.py` | 无 | 保持 JSONL v2，明确 UTF-8 字节流与单行上限 | Rust 注入 `PYTHONUTF8=1`/`PYTHONIOENCODING=utf-8`；Python 对 stdio `reconfigure(encoding="utf-8")`；`read_commands` 捕获 `UnicodeDecodeError` 返回结构化错误且限制单行字节上限 | **Linux**：注入非 UTF-8 字节/超长行/中文路径单测；**Windows**：CP936/CP932 含中文路径实机测试 | S | 低 |
| **T1.4 对齐 JSON Schema、Rust 与 Python 三方 Protocol v2 契约** | **P1** | 双端 (`Cross-platform Owner`，`CROSS_PLATFORM_CHANGE_REQUIRED`) | `shared/protocol-schema/sidecar-v2-*.schema.json`、`crates/xarchive-protocol/src/sidecar_v2.rs`、`sidecar/src/xarchive_downloader/protocol_v2.py` | T1.1 | **有**（统一 `text` 上限、消除 `DownloadResult.result` 冲突、对齐 URL 校验、处置退役 `download`） | 三方对同一组正向/负向共享夹具（含 >4000 字长帖、非法域名 URL）给出 100% 一致判定 | **Linux**：`cargo test -p xarchive-protocol --offline` + `.venv/bin/pytest sidecar/tests/test_protocol_v2.py` | M | 中 |

### Phase 2：子进程容器化统一与孤儿进程治理

| 任务 | 优先级 | 归属端 / Owner | 涉及文件 | 前置依赖 | 跨语言契约变更 | 验收标准 (DoD) | 验证方法（环境） | 工作量 | 回归风险 |
|---|---|---|---|---|---|---|---|---|---|
| **T2.1 修复非受控监督模式的 `gallery-dl` 子树孤儿逃逸** | **P0** | 双端 (`Cross-platform Owner` + `Windows Platform Owner`) | `crates/xarchive-sidecar-supervisor/src/lib.rs`、`sidecar/src/xarchive_downloader/process.py` | T1.3 | 无 | `shutdown()` 先协作停止并给予宽限让 Python `terminate_tree()` 回收子会话，再升级强制信号；Windows 兜底清理子树 | **Linux**：fake 慢速 `gallery-dl` 孙进程退出确认；**Windows**：`stop_sidecar` 后无残留 `gallery-dl.exe` | M | 中 |
| **T2.2 将账号批次发现（`spawn_account_batch`）纳入平台进程容器管控** | **P0** | Rust（Linux 共享接线 + Windows Job Object 复验） | `desktop/src-tauri/src/batch/dispatch.rs`、`desktop/src-tauri/src/commands/sidecar.rs` | T2.1 | 无 | 批次发现 Sidecar 在 Windows 绑定 `KILL_ON_JOB_CLOSE` Job Object、在受控 Linux 绑定 cgroup，宿主异常退出时内核自动回收全树 | **Linux**：强杀父进程回归测试；**Windows**：发现中强杀 `xarchive-desktop.exe` 验证子进程归零 | M | 中 |
| **T2.3 通信与启动开销基准埋点及 `mpsc` 背压验证（HYP-01~03）** | **P1** | 双端 (`Cross-platform Owner`) | `crates/xarchive-sidecar-supervisor/src/lib.rs`、`desktop/src-tauri/src/executor/factory.rs`、`sidecar/src/xarchive_downloader/extraction.py` | T2.1 | 无 | 完成 HYP-01/02/03 实测；若实测确认峰值/复杂度超标，再落盘有界通道或增量偏移读取 | **Linux & Windows**：输出 P50/P95 耗时与峰值 RSS 验证记录 | M | 低 |

### Phase 3：双生态工具链门禁、权限收敛与文档卫生

| 任务 | 优先级 | 归属端 / Owner | 涉及文件 | 前置依赖 | 跨语言契约变更 | 验收标准 (DoD) | 验证方法（环境） | 工作量 | 回归风险 |
|---|---|---|---|---|---|---|---|---|---|
| **T3.1 补齐 Python `ruff` + `mypy` 静态门禁、锁文件与 `.gitignore`** | **P1** | Python / CI (`Cross-platform Owner`) | `sidecar/pyproject.toml`、`.gitignore`、`.github/workflows/architecture-check.yml` | T1.1 | 无 | `.gitignore` 增加 `.mypy_cache/`；配置 `mypy` 并固定开发依赖；CI 执行 `ruff check`、`mypy`、`pytest` | **Linux**：`ruff check`、`mypy`、`pytest`、`git status` 全绿 | S | 低 |
| **T3.2 收敛 Tauri 默认 Capability 最小权限与 `clipboard-win` 依赖核查** | **P1** | Rust (`Cross-platform Owner` + `Windows Platform Owner`) | `desktop/src-tauri/capabilities/default.json`、`desktop/src-tauri/src/lib.rs`、`desktop/src-tauri/Cargo.toml` | 无 | 无 | 生产 `default.json` 移除 `"mcp-bridge:default"`；移除未使用的 `tauri-plugin-shell`；评估 `arboard` 升级消除 `clipboard-win` yanked 告警 | **Linux**：`cargo check`/`clippy`/`audit`、`desktop test`；**Windows**：Release 启动与剪贴板实测 | S | 低 |
| **T3.3 修正 `README.md` / `overview.md` / `status.md` 架构文档漂移与退役 E2E 清理** | **P1** | 文档 / 前端测试 (`Cross-platform Owner`) | `README.md`、`docs/architecture/overview.md`、`docs/development/status.md`、`desktop/e2e/specs/` | T1.4 | 无 | 文档准确反映生产 `aria2-only` 现状，清理针对已退役 `use_aria2` 开关的历史 E2E 用例引用 | **Linux**：`node scripts/docs-audit.mjs`、`npm --prefix desktop test` PASS | S | 低 |
| **T3.4 模块卫生拆分（`config/exchange.rs` 与 `windows_containment` 测试外置）** | **P2** | Rust（Linux 拆 `config.rs`，Windows 拆 `windows_containment` 测试） | `desktop/src-tauri/src/config.rs`、`crates/xarchive-workflow/src/windows_containment.rs`、`scripts/architecture-baseline.json` | Phase 2 | 无 | `ConfigurationExchange` 独立为子模块，`windows_containment.rs` 测试外置，显式重密封架构基线 | **Linux**：`check-architecture.mjs`、`cargo test`；**Windows**：`cargo test -p xarchive-workflow` | M | 低 |

### Phase 4：Windows 11 端到端实机终验与发布收口

| 任务 | 优先级 | 归属端 / Owner | 涉及文件 | 前置依赖 | 跨语言契约变更 | 验收标准 (DoD) | 验证方法（环境） | 工作量 | 回归风险 |
|---|---|---|---|---|---|---|---|---|---|
| **T4.1 Windows 11 便携版全链路实机终验** | **P0** | Windows (`Windows Platform Owner`) | [`../validation/windows-queue.md`](../validation/windows-queue.md)、[`../status/platform-handoff.md`](../status/platform-handoff.md) | Phase 1–3 合入 `origin/dev` | 无 | 验证 Edge 自动轮询至 `● 已归档`、多图/视频扩展名与 SHA-256、Credential Manager + Telegram 发送、离线包 `embedded_catalog` 与 `SHA256SUMS` 一致性 | **Windows 11 实机**：回写精确 commit SHA、产物 SHA-256 与运行证据 | M | 低 |

---

## 4. 明确不建议做的事（Anti-Goals）

1. **将 Python Sidecar 改为 PyO3 进程内 FFI 绑定**：破坏按 Attempt 绑定 OS 容器强杀隔离的安全边界，并把 GIL 争用与 C 扩展崩溃风险引入 Tauri 宿主。
2. **引入 gRPC / Protobuf / ZeroMQ / Redis 替代 stdio JSONL**：当前控制流低频且 Python 侧零运行时依赖，重型 RPC 会破坏 PyInstaller 纯净单目录打包。
3. **按“800 行红线”机械强拆高内聚所有权模块（如 `archive.rs`、`executor/runtime.rs`）**：会迫使私有 move-only 字段提升可见性，削弱编译期状态机不变量保护。
4. **强行单包覆写升级 `glib 0.20` 或执行 `npm audit fix --force`**：`glib 0.18.5` 由 Tauri GTK3 链锁定（Windows 产物不含 `glib`），强制升级或全量 fix 会破坏依赖约束与 WDIO 补丁。
5. **恢复 `gallery-dl` 与 `aria2` 双媒体下载回退链路**：Phase 2.1 已完成 `aria2-only` 收敛，不应回退。

---

## 5. 待拍板决策

| 决策项 | `[推荐方案]` | `[备选方案]` | `[核心权衡]` | `[默认选择]` |
|---|---|---|---|---|
| **D1. 已退役 `download` 命令处置** | 彻底移除生产协议中的 `download` 命令、`DownloadRunner` 与 Schema 对应项 | 保留代码与 Schema，仅修补 `DownloadResult.result` 冲突并标注仅限测试夹具 | 彻底移除可消除 ~350 行死代码与 4 处漂移，但需改造旧夹具；保留改动小但背负双路径心智负担 | `v0.2.1` 选 `[备选方案]` 快速收口，`v0.3.0` 执行 `[推荐方案]` |
| **D2. `ExtractionResult.text` 长度上限** | 将 JSON Schema `text` 上限从 `4000` 上调至 `32768` 并双端对齐校验 | 维持 `4000` 上限，由 Sidecar 截断超长 X 推文 | X 长帖可达 25,000 字符；截断破坏归档完整性，32,768 完全在 1 MiB 行预算内 | `[推荐方案]` |
| **D3. 账号批次发现容器隔离级别** | 使用轻量级平台容器守护（Windows 匿名 `KILL_ON_JOB_CLOSE` Job Object；Linux 专属 cgroup 或协作清理） | 复用单帖归档重量级 `.record` + Desktop Keeper 进程架构 | 发现仅写临时目录并流式上报候选，无需跨崩溃发布证明，轻量容器即可防孤儿 | `[推荐方案]` |
| **D4. `v0.2.1` 的 `Production V2` 边界** | `v0.2.1` 锁定 `InternalV1` 收口，保持 `InternalRenameV2` fail-closed | 本轮追加实现 Windows `recover_v2_on_runner` 并切换至 `V2` | `InternalV1` 已有完整双平台崩溃恢复矩阵，强行切 `V2` 会显著推迟发布 | `[推荐方案]` |
