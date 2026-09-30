# 工程审查报告（2026-09-26）

本轮为只读工程审查。未修改产品代码、配置或依赖，未提交、推送或变更 PR 状态。修复计划见 [`../development/roadmap.md`](../development/roadmap.md) 的 R7。

## 审查基线

| 项 | 值 |
|---|---|
| 分支 | `security/tweet-url-host-validation` |
| Commit | `0321bf08e2c3dc51b28716a6a2020a3c6db636ae` |
| 基线分支 | 基于本地 `dev` `1786c6a` |
| 既存未跟踪内容 | `desktop/e2e/test-artifacts/`（非本轮产生，保留原状） |
| 交叉核对 | U7 分支 `feature/u7-desktop-production-integration` `6d60429` 的重点问题 |

当前工作区不是 U7 开发分支。报告以当前工作区实现为准；标注 U7 的条目只表示该问题在 U7 同样存在，不等于对 U7 完成了完整审查。

## 审查范围与限制

覆盖：架构与模块职责、安全、隐私、凭据、依赖供应链、配置默认值、日志与错误处理、跨平台、构建发布、测试、可观测性、技术债务。

方法是静态代码审查、依赖审计（`npm audit`、`cargo audit`）和选定 Linux 回归测试。**不是渗透测试或安全认证。**

未执行：Windows/macOS 平台、真实账号下载、真实 Telegram 发送、完整 Tauri 发布构建、Git 全历史秘密扫描、最终发布包内容检查。

## 结论摘要

组件划分（Extension → Native Host → Desktop → Sidecar/Storage）合理，SQL 参数化、aria2 SHA-256 校验、Token 脱敏、Extension URL 修复回归测试等防护已落地。但当前工作区**不适合作为直接发布基线**：存在打包脚本数据删除风险、当前分支 Windows 条件编译缺陷、文件与 IPC 边界不完整、发布可追溯性缺失，以及依赖审计实际命中的公告。

| 维度 | 评价 |
|---|---|
| Architecture | 总体边界合理；Desktop 编排层偏集中，部分生产路径残留测试时钟 |
| Security | 具备基础防护；文件系统与资源限制未形成完整边界 |
| Privacy | 本地优先明确；外部进程错误透传与删除策略需收口 |
| Maintainability | 测试基础可用；分支能力差异导致发布基线易混淆 |
| Release readiness | 建议先完成 P0/P1 再发布 |

## Critical / High

### ENG-01 打包输出目录未经保护即递归删除

- Category：Build / Filesystem
- Severity：HIGH；Confidence：HIGH；Status：Confirmed
- 位置：`desktop/scripts/build-portable-windows.mjs:10,24-29`

`outputRoot` 来自 `PORTABLE_OUTPUT_DIR`，随后直接 `rm(outputRoot, { recursive: true, force: true })`，未拒绝项目根目录、源码目录或用户目录。


## Architecture Findings

### ENG-06 生产持久化与传输路径使用固定时间

- Severity：MEDIUM；Confidence：HIGH；Status：Confirmed
- 位置：`desktop/src-tauri/src/executor.rs:647-649`、`desktop/src-tauri/src/transport.rs:306-309`

生产代码返回固定 `2026-09-13T00:00:00Z`，不是仅测试 fixture。影响任务时间、排序与诊断可信度；对由时间参与生成的任务标识还需验证重复提交与重试是否碰撞。

修复：注入时钟接口，生产用真实 UTC，测试用固定时钟。验证不同时间提交、重启恢复与重复任务。U7 仍保留固定时间。

### ENG-15 Executor 集中承担过多职责

- Severity：LOW；Confidence：HIGH；Status：Confirmed
- 位置：`desktop/src-tauri/src/executor.rs`（4446 行，含大量测试）

持久化适配、应用服务、调度与执行集中于一个模块。问题不是文件长度本身，而是多个变化原因耦合，修改持久化或调度策略时审查范围与回归成本偏大。

建议按持久化适配、应用服务、调度/执行端口做行为不变拆分，复用现有测试并登记 repository map。

未确认项：没有足够证据认定存在 crate 循环依赖或普遍跨层绕过，不列为确定缺陷。

## Security Findings

### ENG-03 文件路径防护只覆盖词法路径和部分末级文件

- Severity：MEDIUM；Confidence：HIGH；Status：Confirmed（实际利用条件需验证）
- 位置：`crates/xarchive-storage/src/file_store.rs:34-86,142-167`、`crates/xarchive-storage/src/metadata.rs:59-73`

`safe_child` 拒绝绝对路径与 `..`，但未约束中间目录 symlink/reparse point；metadata 校验检查末级文件，不能排除父目录指向根外。前提是攻击者或其他软件可修改归档/暂存目录，不能据此断言远程网页可任意写文件。

修复：约束根目录所有权与权限，逐层检查链接，敏感操作采用基于目录句柄的安全访问。验证父目录 symlink、Windows junction 与检查后替换竞态。U7 保留同类实现。

### ENG-04 Unix IPC 缺少连接数与读取期限限制

- Severity：MEDIUM；Confidence：HIGH；Status：Confirmed
- 位置：`desktop/src-tauri/src/transport.rs:209-237,267-296`

每连接一线程且阻塞读取，无连接上限与读超时，也未显式设置 endpoint 权限。半帧连接可长期占用线程。其他用户是否可连接取决于目录权限与 umask，不能统一断言跨用户开放。

修复：限制并发、设置读写期限、显式 endpoint 权限，明确同 UID 信任模型。验证大量空连接、半帧与客户端退出。消息大小限制不能替代连接限制。

### ENG-05 Sidecar 输出存在无界内存消耗路径

- Severity：MEDIUM；Confidence：HIGH；Status：Confirmed
- 位置：`crates/xarchive-sidecar-supervisor/src/readers.rs:8-25,45-50`、`sidecar/src/xarchive_downloader/gallery.py:59-73`

Rust 使用无单行上限的 `.lines()`；Python `capture_output=True` 捕获全部输出，`stderr[-4000:]` 截断发生在捕获之后，不限制峰值内存。


## Privacy Findings

敏感数据生命周期：

| 数据 | Source → Processing → Storage → Transmission → Deletion |
|---|---|
| Tweet 内容、作者、关系 | DOM / gallery-dl → Desktop 合并 → SQLite、JSON、TXT、媒体 → 主要本地归档，Telegram 库另有发送能力 → 未确认统一删除流程 |
| 浏览器 Cookie | 浏览器 profile → gallery-dl `--cookies-from-browser` → 外部工具内存与行为待核实 → 已认证请求 → 依赖进程退出与外部工具清理 |
| 浏览器 profile、本地路径 | 配置/UI → 命令参数与错误处理 → 配置及任务错误 → 子进程、UI、剪贴板 → 未确认统一保留期限 |
| Telegram Token | `SecretStore`/调用者 → HTTPS transport → 当前检查到内存实现 → Telegram API → 内存删除不等于安全清零 |
| 错误与诊断 | 子进程 stderr、Rust 错误 → 事件与任务错误 → SQLite 或显示层 → 用户复制或提交报告时可能外传 → 缺少统一脱敏与清理契约 |

### ENG-12 外部工具错误原样透传

- Severity：MEDIUM；Confidence：MEDIUM；Status：Potential
- 位置：`sidecar/src/xarchive_downloader/errors.py:18-29`、`sidecar/src/xarchive_downloader/__init__.py:136-149`、`crates/xarchive-storage/src/database/jobs.rs:207-224`

已确认：stderr 直接作为错误消息，数据库支持持久化任务错误文本。尚未确认：真实 Cookie/Token 是否泄露，以及哪些字段沿失败分支持久化。

修复：外部错误进入协议边界前统一脱敏与限长，用户提示使用稳定错误码。验证路径、认证头、Cookie 与带凭据 URL 的哨兵测试。

### ENG-13 aria2 RPC secret 进入进程参数

- Severity：LOW；Confidence：HIGH；Status：Confirmed
- 位置：`crates/xarchive-download/src/supervisor.rs:72-78`

`--rpc-secret=...` 出现在子进程命令行。暴露面是能查看进程信息的本地主体；本机同用户本就有较强权限，因此不评为 HIGH。修复方向为受限权限配置文件等传递方式、短生命周期随机 secret，并确保诊断不包含完整 argv。Windows 进程信息可见性需单独验证。

### 凭据检查结果

- 对当前 Git 跟踪文件的有限模式扫描未命中私钥、云访问密钥、GitHub Token 或 Telegram Token 模式。
- GitHub secret-scanning 开放列表为空。
- `.env` 等本地覆盖文件已有忽略规则。
- 未检查全部历史、真实机器凭据与发布包，不能承诺“从未提交过秘密”。
- Windows 凭据存储适配未在当前范围内得到完整实现证据，不能把内存 SecretStore 视为已完成的持久化安全存储。

## Dependency / Supply-chain Findings

### ENG-10 Rust 锁文件命中已知 TLS 公告

- Severity：MEDIUM；Confidence：HIGH；Status：Confirmed
- 位置：`Cargo.lock` 中 `rustls 0.23.43`

`cargo audit` 命中 RUSTSEC-2026-0285（TLS 1.3 加密级别边界校验，修复版本 `>=0.23.45`）。公告说明握手 transcript 仍经认证，不能夸大为网络攻击者可篡改或完成握手。修复：单独升级兼容修复版本并跑 Rust/HTTPS/下载回归与审计；本轮未升级。

## Build / Packaging / Release Findings

### ENG-07 手动发布标签与 checkout 源码未绑定

- Severity：MEDIUM；Confidence：HIGH；Status：Confirmed
- 位置：`.github/workflows/windows-release.yml`

手动输入 `release_tag` 用于产物命名与上传，checkout 未显式使用该 tag。人工选择的 ref 与输入 tag 不一致时，可能向旧版本上传另一提交构建的产物，且使用 `--clobber`。修复：checkout 精确 tag、核对 HEAD 与 tag commit 并记录 SHA；用不匹配 ref/tag 演练确认发布被拒绝。

### ENG-08 打包直接复用已存在二进制

- Severity：MEDIUM；Confidence：HIGH；Status：Confirmed
- 位置：`desktop/scripts/build-portable-windows.mjs:24-26,34,53-54`

仅当二进制不存在才构建，无法证明现有二进制对应当前源码；manifest 版本还可独立指定。场景：源码已修复但打包仍携带旧程序。修复：发布默认强制构建，复用模式需验证 provenance。验证“修改源码但保留旧 exe”的负向测试。U7 仍有相同模式。

### ENG-09 递归复制整个组件目录，缺少发布文件允许列表

- Severity：MEDIUM；Confidence：HIGH；Status：Confirmed（机制；实际泄露未确认）
- 位置：`desktop/scripts/build-portable-windows.mjs:35-50`

Extension 与 sidecar 目录整体复制，文件复制不等于 Git tracked-file 导出，也不自动应用 `.gitignore`。组件目录内的本地 `.env`、调试文件、测试数据或凭据可能进入包。修复：明确允许列表并生成包内清单；用无敏感哨兵文件验证排除。未执行真实产物检查，不能报告已发生泄露。

## Logging and Error Handling

### ENG-14 日志仅限制文件数量，读取却全量加载

- Severity：MEDIUM；Confidence：HIGH；Status：Confirmed
- 位置：`desktop/src-tauri/src/logging.rs:32-49,52-73,99-123`

单个日志无大小上限；`read_recent` 全量读取并拆分后才保留最后若干行。长时间运行或高频日志可使磁盘与内存持续增长。修复：按字节轮转、限制单条日志、从尾部有界读取，并做大日志峰值内存测试。

运行时有 `.ok()` 与忽略恢复结果的路径，值得后续验证是否被健康状态或其他日志补偿；缺少完整错误传播证据时，不将所有忽略返回值判为独立缺陷。

## Platform-specific Risks

以下均未在 Windows 实机执行：

| 项目 | 标记 | 需验证行为 |
|---|---|---|
| 当前分支 Windows 编译 | Requires Windows validation | ENG-02 修复后 workspace/release 构建 |
| symlink / junction / reparse point | Requires platform-specific validation | 中间目录、竞态、实际 ACL |
| PowerShell 解压与可执行文件发现 | Requires Windows validation | 空格、Unicode、引号、路径搜索与执行失败 |
| 浏览器 Cookie / profile | Requires Windows validation | 权限拒绝、浏览器锁定、认证失败与错误脱敏 |
| Full/Core 包 | Requires Windows validation | 新机器、无开发依赖、移动目录后启动 |
| IPC / Native Messaging | Requires Windows validation | 当前分支 Windows 实现缺口，不能用 Unix 测试代替 |
| E2E / MCP | Requires platform-specific validation | 正式包未误启用自动化能力 |

## Tests and Verification

本轮实际结果：

| 检查 | 结果 |
|---|---|
| Extension Node tests | 13/13 PASS |
| Desktop Node tests | 33/33 PASS |
| Python Sidecar pytest | 12/12 PASS |
| Rust core library tests | 12/12 PASS |
| Rust protocol library tests | 11/11 PASS |
| Rust storage library tests | 25/25 PASS |
| `git diff --check` | PASS |
| `npm audit` | 16 个受影响依赖条目 |
| `cargo audit` | 1 条漏洞公告，另有警告 |
| 完整 Rust workspace / Tauri build | NOT RUN |
| Windows / macOS / 真实服务 / 发布包检查 | NOT RUN |

Desktop 测试首次以不存在的目录入口失败，随后枚举实际测试文件执行成功；该失败不计为产品失败。共 106 个选定测试通过，不代表完整覆盖率。

最重要测试缺口：打包越界删除与旧二进制/产物夹带的负向测试；Windows 编译门禁；父目录 symlink/junction 与替换竞态；IPC 半帧与连接洪泛；Sidecar 超长输出内存上限；stderr 凭据哨兵端到端脱敏；生产时钟与任务排序；数据库与文件提交之间的中断恢复；删除后 SQLite/归档/缓存/日志一致性；发布包清单与源码 commit 一致性。

## 发现统计

| Severity | 数量 |
|---|---:|
| CRITICAL | 0 |
| HIGH | 2 |
| MEDIUM | 11 |
| LOW | 3 |

- Confirmed 15 项：缺陷机制或受影响依赖已确认，不等于全部完成攻击复现。
- Potential 1 项：ENG-12 的敏感信息实际泄露及完整持久化链路需进一步验证。
- 仍需验证：ENG-03 的实际攻击前提、ENG-09 的实际产物夹带、ENG-11 的项目可利用性。
- 未确认远程 RCE、认证绕过、SQL 注入或真实凭据泄露；未确认不等于已证明不存在。

推荐处理顺序：ENG-01 → ENG-02 → ENG-07/08/09 → ENG-10/11 → 文件与资源边界。


### ENG-11 Node 测试工具链命中多个受影响依赖

- Severity：MEDIUM（项目上下文）；Confidence：HIGH；Status：Confirmed（可利用性待验证）
- 位置：`package-lock.json`、`desktop/package.json`

`npm audit` 返回 16 个受影响依赖条目（工具标注 15 high、1 moderate），主要关联 WDIO、浏览器下载/解压与测试工具链。这是依赖传播后的条目数，**不是 16 个独立漏洞，也不是 15 个产品运行时 HIGH 漏洞**。

代表性公告包括 `extract-zip` 的 symlink 路径问题与 `serialize-javascript` 的代码执行问题，利用条件仍需映射到本项目实际调用。建议先追踪依赖链与输入来源再升级或替换，不执行未经评估的 `npm audit fix --force`。

### ENG-16 工具链锁定与发布依赖门禁不足

- Severity：LOW；Confidence：HIGH；Status：Confirmed
- 位置：`.github/workflows/windows-worker-artifact.yml`、`.github/workflows/windows-release.yml`、`sidecar/pyproject.toml`

Worker 构建升级 pip 并安装未固定 PyInstaller，Rust 使用 stable，Actions 使用可移动标签。锁文件存在不等于整个构建环境可复现。修复：锁定构建工具、使用 `--locked`、对 Action 固定提交并制定更新策略。Rust 审计另有维护性/健全性告警，需逐项判断。

修复：限制单行、单任务累计输出与事件队列容量，超限终止并返回稳定错误码。验证超长行与大量日志的假进程。

### 已确认的正向防护

- Extension URL 主机校验修复已有回归测试，本轮 13/13 通过。
- 已检查的 SQLite settings 路径使用参数化 SQL 与大小限制。
- gallery-dl 参数以数组传递，未发现该路径使用 `shell=True`。
- aria2 自动下载在解压前核对固定 SHA-256，不能描述为“下载后无校验执行”。
- MCP 插件注册受 `debug_assertions` 限制，不是正式版本默认开放。
- Telegram `BotToken` 的 Debug/Display 已脱敏。

上述结论不等于所有 SQL、命令调用与网络路径均已证明安全。

失败场景：已有应用二进制时误设 `PORTABLE_OUTPUT_DIR=.` 即删除当前项目。触发需要本地构建配置错误或构建环境被控制，不是远程未认证攻击，但数据损失严重。

修复：拒绝项目根/源码祖先/用户目录；只清理带标记的生成目录；先完成输入组件检查再删除；临时目录构建后原子替换。

验证：临时沙箱测试 `.`、`..`、绝对路径与 symlink 输入，确认越界被拒绝。U7 存在同类逻辑。

### ENG-02 当前基线存在 Windows 条件编译错误

- Category：Platform / Build
- Severity：HIGH；Confidence：HIGH；Status：Confirmed（静态代码）
- 位置：`desktop/src-tauri/src/transport.rs:11-12,28-44`

`PathBuf` 与 `Path` 同处 `#[cfg(unix)]` 导入，但非 Unix 专属结构体和方法仍使用 `PathBuf`。属发布阻断问题，不是安全漏洞。本轮未运行 Windows 编译。

U7 已修正该导入，当前分支未包含；不能用 U7 构建结论替代当前分支。

修复：`PathBuf` 无条件导入，仅 `Path` 保留条件导入。验证：Windows `cargo check --workspace --locked` 与发布构建。
