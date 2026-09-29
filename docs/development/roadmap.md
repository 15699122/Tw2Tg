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
| P1 | 发布可追溯性、依赖公告处置、文件/IPC/输出边界、错误脱敏、生产时钟、aria2 secret 传递 | ENG-03、ENG-04、ENG-05、ENG-06、ENG-07、ENG-08、ENG-09、ENG-10、ENG-11、ENG-12、ENG-13 |
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
| ENG-03 中间目录 symlink 逃逸 | Linux 已完成 | `resolve_within` 逐段 reparse 校验；storage 29/29（4 项新增）；旁路对照 2 项失败 |
| ENG-07 发布可追溯性 | Linux 已完成 | workflow 以 tag checkout 并校验 HEAD 一致；产物记录 commit 与 SHA-256；YAML 解析通过 |
| ENG-08 禁止默认复用旧二进制 | Linux 已完成 | 打包默认强制构建；复用需 `PORTABLE_ALLOW_BINARY_REUSE=1`；构建后校验二进制存在 |
| ENG-09 包内容排除规则 | Linux 已完成 | `filterPackageFiles` 排除 `.env`/SQLite/日志/缓存/`node_modules`/`target`/测试产物；`.env` 哨兵实测被排除且源文件保留 |
| ENG-16 工具链锁定（部分） | Linux 已完成 | 发布构建 `cargo --locked`；worker workflow 固定 `pyinstaller==6.22.3` |
| ENG-04 IPC 连接上限与读期限 | Linux 已完成 | 并发上限 64、每连接 15 秒读写期限；2 项 transport 测试（半帧连接突发后仍可服务、上限取值合理） |
| ENG-10 rustls TLS 补丁升级 | Linux 已完成 | `cargo update -p rustls --precise 0.23.45`；仅锁文件变更（版本+checksum），`Cargo.toml` 未动，`hyper-rustls` 无需连带升级；`cargo audit` 漏洞 0；workspace 182/182；telegram+download TLS 28/28；fmt 与严格 Clippy 通过。剩余 7 条为 unmaintained/unsound 警告，非漏洞 |
| ENG-11 Node 依赖链 | Linux 已完成，Windows 待验证 | 16 条报告条目中 3 条真实公告；`serialize-javascript`→7.1.2、内嵌 `deepmerge-ts` 7.1.6→hoist 8.0.2，经 19 组差分用例验证行为一致；`extract-zip` 无修复版本，登记为风险接受。`npm audit` 16→13。Windows 见 WQ-ENG-09 |
| P2 ENG-15 Executor 职责拆分 | Linux 已完成 | `executor.rs`（4449 行）按变化原因拆为 `executor/{mod,model,persistence,service,runtime,tests}.rs`；token 级比对证明除有意删去的冗余 `mod tests { }` 包装外内容完全一致；50/50 测试保留、workspace 187/187；fmt/严格 Clippy/check 全通过 |
| P2 ENG-16 工具链锁定 | Linux 已完成，Windows 待验证 | 新增 `rust-toolchain.toml` 固定 1.98.0（与当前 stable 一致，避免静默降级）并声明 rustfmt/clippy；5 个 GitHub Action 全部按 commit SHA 固定；`pyinstaller==6.22.3` 与 `cargo --locked` 此前已完成。Windows 构建解析结果见 WQ-ENG-10 |
| P1 ENG-13 aria2 RPC secret 传递 | Linux 已完成，Windows 待验证 | `--rpc-secret` 不再进入子进程 argv，改为 owner-only 短期 `--conf-path` 文件（Unix `0o600`，`create_new` 防竞态；spawn 失败/超时/`shutdown`/`drop` 均删除）；3 项新增测试（argv 无 secret、文件内容+权限+shutdown 删除、spawn 失败无残留）。workspace 189/189；fmt/严格 Clippy 通过。Windows 进程可见性与真实 aria2c.exe 行为见 WQ-ENG-12 |

### ENG-11 Node 依赖公告处置方案

本节记录 ENG-11 的处置方案、完成标准与已确认事实。`npm audit` 的报告条目不等于独立漏洞数：`deepmerge-ts`、`serialize-javascript`、`extract-zip` 是需分别处置的底层包，`mocha`、`@puppeteer/browsers` 与各 `@wdio/*` 包会因依赖关系被连带标记。结论以更新后的锁文件和复跑审计为准。

#### 已确认事实

- `@wdio/tauri-service@1.4.0` 已是当前可获得的最新版本，其固定依赖 `webdriverio 9.30.1` 带来内嵌 `deepmerge-ts 7.1.6`；升级项目顶层 WDIO 不会更新该内嵌副本。
- `@wdio/mocha-framework@9.32.0` 仍声明 `mocha: ^10.8.2`，而 Mocha 10.8.2 依赖 `serialize-javascript: ^6.0.2`；仅升级 WDIO 9 无法修复该项。
- `extract-zip` 最新版仍为 2.0.1，公告范围为 `*`，**目前没有修复版本**。上游 `@puppeteer/browsers` 3.x 已移除该依赖，但不满足 `@wdio/utils` 声明的 `^2.2.0`。
- `desktop/wdio.conf.mjs` 在 Windows 设置 `autoDownloadEdgeDriver: true`，因此浏览器/驱动下载路径**可达**，不得按“路径不可达”结案。

#### 处置方式

1. **`serialize-javascript`**：在根 `package.json` 使用 npm `overrides`，仅将 Mocha 使用的 `serialize-javascript` 指向已修复的 7.1.x，重新生成根 `package-lock.json`。这跨越 Mocha 10 的 `^6.0.2` 约束，必须以 reporter、失败输出和异常对象序列化的行为验证为准，不得仅凭 `npm audit` 数字下降判定。
2. **内嵌 `deepmerge-ts`**：对产生 `7.1.6` 的依赖链做定向 override 到 8.x，并确认锁文件中不再残留旧副本。重点验证 service 初始化、配置合并、session 生命周期以及 `desktop/scripts/wdio-tauri-service.mjs` 继承上游类所依赖的 `driverPool` 等内部接口行为。
3. **`extract-zip`**：不将其他 ZIP 库伪装为 `extract-zip`（API 与安全语义未经证明），也不强推 `@puppeteer/browsers` 3.x。当前措施为登记残留风险并跟踪上游替换；是否改用可信预置驱动以降低运行时暴露，需单独评估 `autoDownloadEdgeDriver` 改动及 CI 前置。关闭路径**不等于**从锁文件移除公告，两者分别记录。

#### 完成标准

- 每条残留审计项都有书面结论：已消除、路径受控、或带理由的风险接受；不以 `npm audit` 归零作为唯一标准。
- 跨主版本 override 均有行为验证证据；实验失败即回退，不为使审计变绿而保留未经证明的 override。
- Windows Tauri v2 原生 session、EdgeDriver 下载/预置行为、退出后 driver 进程与端口清理由 `WQ-ENG-09` 覆盖；自动化受阻时标记 `BLOCKED`，不得记为 PASS。

上述 Linux 结论不等于 Windows 通过：junction/reparse、MSVC 条件编译、真实账号错误内容、IPC 连接行为与发布包清单仍需 `WQ-ENG-01` 至 `WQ-ENG-08` 证据。

#### 执行结果（2026-09-26）

- **实现方式偏离初始设想**：npm 不会因 `overrides` 变化重新解析既有锁文件，`npm ci` 也会忽略 `overrides`；从零解析会连带升级 89 个无关包（含 `react 19.2.8→19.3.0`、`@tauri-apps/cli 2.11.4→2.12.0`、`undici 7.29.1→6.29.0` 降级）。因此改用 clean-room 解析得到的 integrity 精确改写锁文件，仅 3 处变更：`serialize-javascript` 条目、`@wdio/tauri-service` 嵌套 `deepmerge-ts` 条目、随之孤立的 `randombytes`。
- **已完成**：`npm ci` 通过且 `npm ls` 无 invalid；`npm audit` 16→13，剩余 13 条中仅 `extract-zip` 为真实公告，其余 12 条是其依赖传播元数据；WDIO `ConfigParser` 成功解析配置；adapter 继承上游 worker/launcher 完整且 `driverPool` 相关方法可达；Mocha 失败上报完整、退出码正确；`npm run check`/`test`/`build`、Extension 13/13、Sidecar compileall 与 pytest 19/19、Rust workspace 8 crates 均通过。
- **Windows 未执行**：Tauri v2 原生 session、EdgeDriver 下载或预置行为、退出后 driver 进程与端口清理由 `WQ-ENG-09` 覆盖，状态 `WINDOWS_VERIFICATION_PENDING`。

## R8：v0.2.0 整合与正式发布

> 状态：`PLANNED`（2026-09-29 计划定稿，尚未开始执行）。本节只记录发布计划、依赖和完成标准；当前实现事实以 [`status.md`](status.md) 为准，Windows 项目以 [`../validation/windows-queue.md`](../validation/windows-queue.md) 为准。

### 目标

将分散在多条分支的完整功能（U7 Popup/Options + 安全修复 + Windows 打包/WDIO 修复）整合为统一基线，统一版本至 `v0.2.0`，完成 Linux 门禁与 Windows 集中验证，经 GitHub Actions 资产演练后发布正式 `v0.2.0`，并在发布后回合 `dev`/`main`。

### 发布口径（用户已确认，2026-09-29）

以下三条为发布范围约束，同时约束代码、README、Release notes 与测试措辞：

1. **不承诺** v0.2.0 真实 X 帖子归档成功；真实归档尝试的结果（无论成败）只作为有限环境观察记录，不作为发布 go/no-go 条件。
2. README 等用户文档强调**个人用途项目，不保证其它设备可用**（不保证在其他设备、账号、网络或浏览器配置下可用）。
3. **各资产不做用户承诺**；只有以下形态可直接运行或包含可直接运行内容：`.exe` 单文件、包含 `.exe` 的应用 `7z`、含 Extension/Sidecar 组件的完整 `7z`。Extension 独立包与 repository-dependencies 包仅定位为**组件/依赖分发包**。

“不做功能承诺”不等于降低真实性要求：被称为“可启动”的文件、每项资产内容和每条版本/校验信息，必须与最终实际发布物一致；损坏、来源不明或与说明不符的文件仍为 no-go。

### 资产格式契约（2026-09-29 用户决策：全部压缩包统一为 7z）

| 类型 | v0.2.0 格式 | 定位 |
|---|---|---|
| 独立桌面程序 | `.exe` | 可尝试直接启动；不含完整组件 |
| 应用包 | `.7z` | 解压后含可尝试启动的 `.exe` |
| Repository dependencies 包 | `.7z` | Extension、Sidecar 等组件/依赖分发包；不作为桌面程序包 |
| Full 包 | `.7z` | 解压后含可尝试启动的 `.exe` 及随附组件 |
| 独立 Extension 包 | **`.7z`（由 `.zip` 改为 `.7z`）** | 浏览器组件分发包；不是可直接运行的桌面程序 |
| Release manifest | `.json` | 校验与来源追溯材料 |
| SHA256SUMS | `.txt` | 校验与来源追溯材料 |

- 最终发布物为**一个 `.exe` + 四个真正的 `.7z` + 两个校验文件**（合计七个发布文件、五类载荷资产）；不得把七个文件都称为压缩包或可运行程序。
- Extension `.7z` 是分发容器：Edge/Chrome 不能直接导入 `.7z`，README 必须写明“先解压，再按受控测试过的方式加载其中的扩展目录”。
- 旧 `v0.2.0-pre.*` 中的 ZIP 保留为历史资产，不重写旧 tag 或 Release。
- `.7z` 改动必须覆盖整条契约：`desktop/scripts/release-assets.mjs` 的资产命名/解析、`.github/workflows/windows-release.yml` 的打包与解包验证（不能对 `.7z` 使用 ZIP 专用 `Expand-Archive`）、release manifest/`SHA256SUMS` 生成、GitHub Release 上传，以及对应测试；不能只改扩展名。

### 依赖

- 远端 `origin/feature/u7-desktop-production-integration`（Popup/Options + 七资产发布工具链 + `pre-release.yml`）与 `origin/security/tweet-url-host-validation`（安全/打包/WDIO 修复）的最终 revision。
- Windows 门禁项：WQ-PACKAGE-FULL-01-R2（P0）、WQ-ENG-09b-ORD-R2（P1 直连配方）、WQ-ENG-13-R2（91/91）、M11–M13 人工；WQ-P1-16/17 当前 `BLOCKED`（只写手工步骤，不记 PASS）。
- 新的、未使用过的演练 tag 与 GitHub Actions Windows runner。

### 执行阶段

1. **分支盘点 + 范围冻结**：`git fetch` 后全量枚举本地/远端分支，产出逐分支处置表（tip、独有提交、纳入/跳过理由）；确定整合起点 commit；把发布口径写成发布范围表。
2. **整合分支**：从 U7 远端 tip 新建 `release/v0.2.0`（或 `integration/v0.2.0`）；merge 安全分支，按协议安全 / Extension / 桌面与 Sidecar / 构建发布 / 文档 5 边界逐冲突裁决并分小批提交；旧 release、windows 专项分支按处置表选择性提取；确认 `feat/extraction-aria2-pipeline` 内容已在。
3. **Linux 收口**：统一 `0.2.0` 元数据（Cargo workspace、tauri.conf.json、根/desktop npm、Extension manifest+package、锁文件、GUI 侧栏硬编码版本、Popup 增加 `chrome.runtime.getManifest().version` 显示）；README 与相关文档加入“个人用途、不保证其它设备可用、归档成功不承诺、各资产定位与 7z 格式”声明；Extension ZIP→7z 契约改动及测试；跑全量 Linux 门禁（npm/cargo/pytest/fmt/clippy）。
4. **Windows 集中验证**：按 `cross-platform-validation.md` Git handoff，一次性排队 R2 项（P0 fresh Full build、P1 直连 E2E、91/91 计数、M11–M13 人工）+ Popup/Options/Extension 配对重验 + **四个 `.7z` 真实格式/解包/内容专项**；三种“可启动”形态分别取实际启动证据；BLOCKED 项写手工步骤，不记 PASS。
5. **Actions 演练**：用新演练 tag + 预发布 Release 跑 `windows-release.yml`（U7 七资产流程移植后），下载发布页实际文件核对资产名称/格式/内容/版本/SHA-256/manifest/source-tag parity；失败则修复后对最终 revision 重演练。
6. **go/no-go + 正式发布**：范围内 P0 通过、资产门禁通过、限制已按新措辞披露后，从已验收 commit 打 `v0.2.0` tag 并发布（非预发布），Release notes 按发布口径撰写。真实 X 归档尝试不成功本身不是 no-go 条件。
7. **发布后 merge**：tag 冻结 → 合入 `dev` 并跑门禁 → 合入 `main`（保持 main 与 tag 源码一致）→ 同步/归档 U7 与安全分支 → 不重写 pre.* tag；hotfix 从 tag 派生并回合。

### 完成标准

- 单一整合基线同时包含 Popup/Options、安全修复、Windows 打包/WDIO 修复；所有分支独有提交均有处置结论。
- 版本元数据在全部声明位置一致为 `0.2.0`，并有回归测试。
- 全量 Linux 门禁通过；适用 Windows 队列项取得 PASS 或有明确 `BLOCKED`/`NOT RUN` 原因。
- 发布页包含 1 个 `.exe`、4 个真实 `.7z`、release manifest 与 SHA256SUMS，命名、哈希、来源 tag 一致，无 ZIP 残留。
- 三种“可启动/含可启动”形态均有对应实际启动证据；组件包未被误标为可运行应用包。
- README/Release notes 明确个人用途、不保证其它设备可用、不承诺真实 X 归档成功、各资产定位。
- `v0.2.0` tag 与正式发布来自同一已验收 commit；`dev`/`main` 回合完成且合并后门禁通过。

### 执行进度

| 阶段 | 状态 | 证据 |
|---|---|---|
| 阶段 1 分支盘点 + 范围冻结 | 已完成（2026-09-29） | `git fetch --all --tags --prune` 后枚举；见下方处置表与 P0 门禁表 |
| 阶段 2 整合分支 | 未开始 | — |
| 阶段 3 Linux 收口 | 未开始 | — |
| 阶段 4 Windows 集中验证 | 未开始 | — |
| 阶段 5 Actions 演练 | 未开始 | — |
| 阶段 6 go/no-go + 正式发布 | 未开始 | — |
| 阶段 7 发布后 merge | 未开始 | — |

### 阶段 1 产出：分支处置表（2026-09-29，fetch 后实测）

整合起点：`origin/feature/u7-desktop-production-integration` = **`783a021`**。本地 U7 分支 `6d60429` 落后远端 10、领先 0，直接对齐远端即可。U7 与 security 的 merge-base 为 `1786c6a`（`origin/dev` tip）。

| 分支 | tip | 相对整合起点 | 处置 |
|---|---|---|---|
| `origin/feature/u7-desktop-production-integration` | `783a021` | 起点；含 `dev`、`feat/extraction-aria2-pipeline`、`pre.*` 发布史、Popup/Options、七文件发布工具链、`pre-release.yml` | **作为整合分支 `release/v0.2.0` 的起点** |
| `origin/security/tweet-url-host-validation` | `87e6b99` | 自 `dev` 分叉，40 独有提交 | **合入**：按协议安全 / Extension / 桌面与 Sidecar / 构建发布 / 文档 5 边界逐冲突裁决 |
| `origin/windows/webview2-readiness-gate` | `30b9ef8` | 于 `59c8221`（U7 历史内）分叉，仅 7 独有：`8a1714f`（WebView2 E2E 启动修复，改 patch 脚本 + `wdio.conf`）、`5a1ecf1`（WebDriver 端口清理 + 混入的孤儿 protocol/schema 文件）、5 个文档提交（09-23 前后） | **选择性提取**：先比对 `8a1714f`/`5a1ecf1` 的 WDIO 部分是否已被 security 批次 5–6 覆盖；`sidecar.rs`/`archive-*.schema.json` 在该分支 `lib.rs` 未声明 `mod`（孤儿文件）→ 跳过；文档以 security（09-29）与 U7（09-27）较新记录为准 |
| `origin/main` | `a472b4e` | 9 独有（CI lineage） | **已包含**：U7 `pre-release.yml` 与 main 仅差一个尾换行；`windows-release.yml` 由 U7 演进（5 类资产）。阶段 2 复核 `a472b4e`（sidecar install）、`6386935`/`93b123a`/`d7974f9`（GTK 依赖修复）在 U7 workflows 中等价存在 |
| `origin/dev` | `1786c6a` | U7 与 security 的共同祖先 | 已包含 |
| `origin/feat/extraction-aria2-pipeline` | `79232f2` | U7 祖先 | 已包含 |
| `origin/release/v0.1.1` | `9236027` | 2 独有：`2dca313`（版本准备）、`9236027`（PathBuf 修复） | 归档：U7 `windows_transport.rs` 为无条件 `use std::path::{Path, PathBuf}`（缺陷不存在），security 有 `3970c53` 等价修复；版本由阶段 3 统一为 0.2.0 |
| `origin/release/v0.2.0-pre.1` | `0105ce9` | 2 独有：`0a8a237`（版本准备）、`0105ce9`（同一 PathBuf 修复） | 归档，同上 |
| 本地 `feature/u7-desktop-production-integration` | `6d60429` | 落后远端 10、领先 0 | 对齐远端（快进） |
| 本地 `dev`/`main`/`release/*` | 落后各自远端 | — | 阶段 7 发布后统一同步 |

#### 阶段 1 识别的关键合并风险

1. **协议两代并存**：dev/security 线为 sidecar v1（`crates/xarchive-protocol/src/sidecar.rs`、`archive-request/archive-status/download-command/download-event` schema、`MessageType`）；U7 线已按 U8 legacy removal 迁移到 sidecar v2（`sidecar_v2.rs` + `media.rs`、`sidecar-v2-command/sidecar-v2-event` schema），**不再含 `sidecar.rs`**。security 的协议修复若落在 v1 文件，必须逐项判断并**重新映射到 v2**（含 Python worker 未知字段拒绝与 schema 命名差异），不得把 v1 文件整体拷回。
2. **同名修复双实现**：tweet link host 校验（security `0321bf0` vs U7 `6d60429`）、WDIO patch/直连配方（security 批次 5–6 vs U7 `pre.10` lineage 与 gate `8a1714f`）——逐文件 diff 裁决，保留行为更完整的一侧并补缺失测试。
3. **workflow 两条 lineage**：security `windows-release.yml` 为 dev 线（2 类资产），U7 为七文件线；合并以 U7 为基，移植 security 的 tag 绑定/本地文件排除、`.pyd` 排除、`npm.cmd` spawn 修复。
4. **文档双向演进**：`platform-handoff`、`windows-queue`、`windows-validation` 在两条线上均被改写，需按时间与内容合并，不得整文件单边覆盖。

#### 阶段 1 产出：Windows 发布门禁表（P0/P1）

| ID | 优先级 | 内容 | 状态/处理 |
|---|---|---|---|
| WQ-PACKAGE-FULL-01-R2 | P0 | 整合后 fresh Full `7z` 构建与内容验证（含四个 `.7z` 真实格式专项） | 阶段 4 排队 |
| WQ-ENG-09b-ORD-R2 | P1 | 直连 msedgedriver 配方 E2E（reviewed recipe） | 阶段 4 排队 |
| WQ-ENG-13-R2 | P1 | 91/91 测试计数复核 | 阶段 4 排队 |
| M11–M13 | 人工 | 手动 Windows 验证项 | 阶段 4 排队 |
| WQ-P1-16/17 | — | 当前 `BLOCKED` | 阶段 4 只补手工步骤，**不记 PASS** |
| 真实 X 帖子归档 | 不设门禁 | 按发布口径不承诺、不作为 go/no-go | 仅作有限环境观察记录 |
