# 开发路线图

> 本文只记录未来方向、依赖和完成标准。当前实现事实以 [`status.md`](status.md) 为准；Windows 验证队列以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准。

## R1：应用编排与并发模型

### 目标

将归档请求从 Tauri command 中的长时间同步流程，演进为可观测、可取消、可恢复的后台 Job executor。

### 依赖

- 当前 `RuntimeState`、Database、SidecarSupervisor 和 FileStore 所有权梳理。
- 明确 Job 状态转换、取消语义和 Sidecar 生命周期。
- 先完成 application service API 和 ADR，再改变运行时行为。

### 完成标准

- Tauri command 只负责输入校验、创建/查询 Job 和发送控制请求。
- 网络、Sidecar、文件和 Telegram I/O 不在全局 RuntimeState 锁内执行。
- Job 状态、事件和错误在后台执行期间可查询。
- stop/cancel、Sidecar 崩溃和应用退出有明确结果。
- 增加并发、取消、失败恢复和重复请求测试。

## R2：DownloadRouter 与真实传输接入

### 目标

完成 gallery-dl 默认路径与 aria2 fallback 的应用级接入，而不仅是 crate 级路由策略。

### 依赖

- R1 的后台 Job executor。
- 受控的 aria2 executable 和本地 media fixture。
- 明确 403/过期 URL 的重新提取边界。

### 完成标准

- gallery-dl 仍是默认 extractor/downloader。
- 只有明确允许且有新鲜 URL 时才使用 aria2。
- 403 或 URL 过期时可重新提取，不复用过期 URL。
- aria2 transfer 状态、Job 状态、事件和文件提交一致。
- fallback 失败时保留 gallery-dl 与 aria2 两侧安全错误信息。

## R3：Native Host 与 Windows IPC

### 目标

完成 Native Host 到 Desktop 的 Windows Named Pipe、ACL、安装和浏览器连接链路。

### 依赖

- 跨平台 protocol/framing API 稳定。
- Windows Named Pipe server 实现和权限设计。
- Native Host manifest、Registry 和固定 Extension ID。

### 完成标准

- 合法 BrowserRequest 可转发到 Desktop 并返回匹配 request_id 的 BrowserResponse。
- 非法协议、越权连接、断线、重连和关闭有确定行为。
- Edge/Chrome 安装、升级和卸载路径可重复。
- Windows ACL 不允许无关进程访问业务管道。

## R4：真实账号与凭据边界

### 目标

完成 Edge Cookie、Credential Manager 和 Telegram 真实账号链路。

### 依赖

- 受控测试账号、Edge Profile、Telegram test chat 和可用网络。
- SecretStore 与 Windows Credential Manager adapter。
- 日志、错误 UI、SQLite 和进程输出脱敏审查。

### 完成标准

- Cookie、Bot Token 和 RPC secret 不进入 Extension、SQLite 或普通日志。
- AUTH_REQUIRED、限流、网络失败和重试状态可诊断。
- Telegram 发送状态跨重启可恢复且幂等。
- 真实账号验证结果与单元/fake-server 结果分开记录。

## R5：文件系统、恢复和隐私边界

### 目标

完成应用级文件数据库恢复、稳定数据目录、ACL 和异常文件系统场景。

### 依赖

- R1 的 Job executor 和应用退出语义。
- 明确 archive root、应用数据目录和用户选择目录的职责。
- Windows 文件数据库、第二用户和 reparse fixture。

### 完成标准

- 旧 migration 可在真实文件数据库中升级并保持数据一致。
- 应用重启、遗留 staging、WAL/SHM 和异常退出可恢复或明确失败。
- symlink/junction/reparse、长路径、Unicode、空格和文件锁场景有测试。
- 归档数据权限符合产品隐私约定。

## R6：打包、安装和桌面体验

### 目标

完成 externalBin、bundle、Native Host 安装、GUI Windows 验收和发布基础设施。

### 依赖

- R3、R4、R5 完成或有明确替代方案。
- Sidecar、aria2 和 Native Host 的分发许可证确认。
- Windows 签名、安装器、WebView2 和辅助技术环境。

### 完成标准

- 安装、升级、卸载、回滚和数据保留可重复。
- Sidecar、Native Host 和配置资源在安装后可定位。
- WebView2、DPI、键盘、Focus-visible、屏幕阅读器和对比度通过实机验收。
- 发布包包含完整许可证、版本和源代码获取信息。

## 依赖顺序

```text
R1 应用编排
  → R2 下载接入
  → R3 Native Host/IPC
  → R4 账号与凭据
  → R5 文件恢复与隐私
  → R6 打包与发布
```

Windows-specific 项目在 Linux 继续实现时统一加入 [`../validation/windows-queue.md`](../validation/windows-queue.md)，不得因普通 pending 项目提前中断 Linux development phase。
- 跨进程 Sidecar command 的字段边界必须由 Schema 和 Rust/Python consumer 同时拒绝未知字段；Linux contract 修复不等同于 Windows 真实 Sidecar、ACL 或 reparse 验证通过。

### 当前 Linux 执行顺序（2026-09-17 Windows reconciliation）

当前不机械执行旧的“入口切换 → R2”顺序。基于现有代码和最新 Windows 结果，下一批 Linux 工作按以下依赖执行：

1. **Windows-result Linux follow-up（本轮完成）**：修复 PyInstaller entrypoint 重复执行、workflow 的 `_internal/python312.dll` artifact 完整性检查，并将 Core manifest 的 Extension `user_importable` 与当前 GitHub 外链 UI 对齐；相关 Linux regression 全部通过。
2. **Windows incremental revalidation（下一步）**：仅重验命中本轮 worker workflow/portable manifest diff 的 WQ-WORKER-BUILD-01、WQ-PACKAGE-CORE-02、WQ-PACKAGE-FULL-01；不重复无交集的 WDIO、GUI、账号或 installer 项。
3. **R1 transport endpoint design/implementation（已完成）**：Desktop 已在 Linux/Unix 上注册 Unix domain socket transport endpoint，Native Host 在 Linux 上改用 `UnixStream::connect` 连接；Windows Named Pipe/ACL 仍属平台适配与验证项，不把 Unix socket 测试外推为 Windows PASS。
4. **R1 entry-switch regression（Linux 已完成）**：Unix transport endpoint 已接入 `BrowserTransportAdapter`，浏览器 `archive_request/query_status` 通过 `ArchiveApplicationService` 创建/复用 Job；request_id、重复提交、查询、协议错误和 executor error mapping 已有 contract tests。同步 `archive_tweet` fallback 继续保留，直到后续 Windows/runtime 证据完成。
5. **R2 fresh media URL contract / application integration（设计边界已识别，尚未完成）**：当前 Sidecar failure event 不返回可供 aria2 使用的 fresh media URL，`DownloadRouter` 只接受调用方提供的 `AddUriRequest`，因此不能安全地把 aria2 fallback 直接接入现有 `download_sidecar`。下一步必须先扩展 Sidecar/schema 的 extraction-result contract，明确 fresh URL 的来源、403/过期重新提取、aria2 transfer polling、cancel/shutdown、Job events/states 和 staging commit，再使用 fake HTTP/aria2/media fixtures 完成 Linux 验证；在该 contract 完成前不得声称 R2 已实现。

Windows revalidation 项目即使 Linux regression 通过，也必须保持 `WINDOWS_VERIFICATION_PENDING`，直到 Windows 真实 artifact/runtime 证据写回验证文档。R1 入口切换和 R2 真实传输接入仍按依赖顺序推进，不机械恢复旧 Plan。

## Portable Windows runtime and Settings/Download Management

状态：`IMPLEMENTED-LINUX / WINDOWS_VERIFICATION_PENDING`。Linux 可验证的便携路径、配置模型、下载目录 setup IPC、日志等级和便携构建组装已实现；Windows `.exe` 同目录、系统 Downloads、权限、sidecar artifact 和真实运行行为仍需验证。

- 当前阶段只构建 Windows 便携版 `.exe`，不生成 installer。
- 便携目录使用 `config/`、`cache/`、`download/`、`extension/`、`logs/` 和 `sidecar/`，不创建 `telegram/`。
- `config/config.yaml` 保存路径、下载目录、日志等级和日志数量；开发 Debug 默认日志等级为 `debug`，Release 默认 `info`。
- 应用启动时若 `download/` 不存在，通过 GUI 选择创建便携目录或使用 `Downloads/XArchive`。
- 最终归档写入 `download/`，临时 staging 写入 `cache/staging/`，日志写入同级 `logs/`。
- 使用 `npm run build:portable:windows --workspace desktop` 组装可移动目录。
- 仍需完成 Windows native portable runtime、跨盘提交、目录权限、辅助程序分发和 GUI 实机验证。
- 后续可继续增加“设置”页面中的 Sidecar、aria2 和自定义路径管理。
- 将 `gallery-dl` Sidecar 与 `aria2` 的相关配置集中放入设置页面；主页仅保留“启动”按钮和运行状态提示。
- 启动时自动检测 Sidecar 与 `aria2`，依次搜索 `PATH`、主程序所在目录及其子目录。
- 当对应程序不存在或不可用时，在设置页面显示明确提示，并提供实际解析到的程序路径与版本信息。
- 增加下载功能与自定义路径功能，允许用户选择其它目录中的相应文件使用。

后续实现需补充：Windows 用户目录 API、跨卷 copy/verify fallback、便携 artifact 中实际 gallery-dl/aria2 文件、Extension 加载、日志权限/轮转实机验证，以及配置迁移和自定义路径回归验证。

## R7：工程审查整改（2026-09-26）

审查报告与证据见 [`../review/engineering-audit-2026-09-26.md`](../review/engineering-audit-2026-09-26.md)；风险登记见 [`risk-register.md`](risk-register.md) 的 RISK-014 至 RISK-022；Windows 项目见 [`../validation/windows-queue.md`](../validation/windows-queue.md)。

### 目标

在不改变产品功能与协议契约的前提下，消除审查确认的发布阻断项与高风险边界缺口，使正式发布基线具备可追溯的源码、产物与依赖证据。

### 依赖与执行顺序

1. 明确发布基线分支（当前安全修复分支 / dev / U7），不混用不同分支的构建与验证结论。
2. P0 必须先于任何发布构建。
3. P1 依赖 P0 完成后的稳定打包脚本。
4. P2 不阻塞发布，可并行推进。
5. 每项完成后执行 Linux 适用验证，并把需要 Windows 证据的项目加入 Windows 队列。

### 完成标准

- 打包脚本在输出目录越界时明确失败，且有负向回归测试。
- 当前发布分支可通过 Windows `cargo check --workspace --locked` 与发布构建。
- 发布产物可追溯到唯一 tag 与 commit，发布路径不默认复用未验证的既有二进制。
- 发布包内容有明确允许列表，不夹带本地配置、测试数据与调试文件。
- 依赖审计命中的公告有处置结论：升级、替代或带理由的风险接受。
- 文件、IPC 与子进程输出具备明确资源上限，并有对应测试。
- 生产路径不再使用固定测试时钟。
- 错误与日志在协议边界完成脱敏、限长，敏感数据生命周期有文档结论。

### 分级

| 阶段 | 范围 | 对应发现 |
|---|---|---|
| P0 | 打包输出目录删除保护、当前分支 Windows 编译 | ENG-01、ENG-02 |
| P1 | 发布可追溯性、依赖公告处置、文件/IPC/输出边界、错误脱敏、生产时钟 | ENG-03、ENG-04、ENG-05、ENG-06、ENG-07、ENG-08、ENG-09、ENG-10、ENG-11、ENG-12 |
| P2 | Executor 职责拆分、日志上限、工具链锁定 | ENG-14、ENG-15、ENG-16 |
| P3 | SBOM 与签名、诊断导出脱敏、发布能力矩阵 | 优化建议 |

本轮审查为只读，未修改产品代码；上述条目在实现并完成对应验证前不得记为已完成。

### 执行进度

| 阶段 | 状态 | 证据 |
|---|---|---|
| P0 ENG-01 打包输出目录删除保护 | Linux 已完成 | `validatePortableOutputDir`；Node 12/12；`PORTABLE_OUTPUT_DIR=.` 实测 exit 1 且项目目录完好；旁路对照 5 项失败 |
| P0 ENG-02 当前分支 Windows 编译 | Linux 已完成，Windows 待验证 | `PathBuf` 无条件导入；`cargo check`、`cargo fmt`、Desktop Rust 80/80 |
| ENG-06 生产固定时钟 | Linux 已完成 | 新增 `clock` 模块（无依赖 civil-from-days）；executor 7 处 + transport 1 处改用真实 UTC；6 项 clock 测试；任务 ID 实测为真实时间 |
| ENG-12 错误脱敏 | Linux 已完成 | `sanitize_error_text` 覆盖 Authorization/Bearer/多类 token/URL 凭据/query secret/cookie/绝对路径，限长 2000；7 项脱敏测试 |
| ENG-05 Sidecar 输出上限 | Rust/Python 已完成 | 单行 1 MiB 上限且分块扫描；gallery-dl 改用临时文件有界保留；9 项 supervisor 测试、19 项 sidecar 测试 |
| ENG-14 日志上限 | Linux 已完成 | 单行 16 KiB、单文件 8 MiB 轮转、换行折叠、尾部 512 KiB 有界读取；3 项日志测试 |
| P1 其余（ENG-03/04/07/08/09/10/11） | 未开始 | 见 RISK-016 至 RISK-018；ENG-10/11 涉及依赖变更，ENG-03/04 需平台语义判断 |
| P2 ENG-15、ENG-16 | 未开始 | Executor 拆分与工具链锁定 |

上述 Linux 结论不等于 Windows 通过：junction/reparse、MSVC 条件编译、真实账号错误内容、IPC 连接行为与发布包清单仍需 `WQ-ENG-01` 至 `WQ-ENG-08` 证据。
