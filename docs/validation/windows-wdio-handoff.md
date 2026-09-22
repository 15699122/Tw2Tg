# Windows WDIO 后续验证 Handoff

> 日期：2026-09-15
>
> 本文描述当前 Linux 配置完成后的 Windows 执行步骤，不代表这些步骤已经在 Windows 完成。Linux 源项目是唯一 source of truth；Windows 工作副本只能用于验证。

## 1. 当前范围与状态

Linux 端已完成：

- advanced wrapper 改为由当前 Node 进程加载本地 WDIO CLI，不直接执行 `wdio.cmd`；
- advanced spec 改为通过 `browser.tauri.execute` 检查 `window.wdioTauri`；
- 普通 release 不加载 `@wdio/tauri-plugin` guest JS；专用构建通过 `VITE_WDIO_E2E=1` 加载；
- Node、Rust、`wdio-e2e` feature build、workspace test 和 Clippy 门禁通过。

仍需 Windows 确认：

- WQ-P1-16 普通 native smoke 的 ACL 和 driver cleanup；
- WQ-P1-17 advanced plugin E2E、日志、mock cleanup、session teardown；
- `tauri-driver`、`msedgedriver`、应用进程和端口的自动清理。
- 固定工具链：`TAURI_DRIVER_VERSION=2.1.0-alpha.0`；workflow 使用 `cargo install tauri-driver --version ... --locked`，关闭 WDIO 自动安装/下载，要求 PATH 上的 `msedgedriver` 与 WebView2 runtime 共享 major（仅在需要精确固定时才设置 `EDGEDRIVER_VERSION`），并记录两个 executable 的路径、版本、banner 和 SHA-256。

上述项目在实际执行前不得改成 `WINDOWS_PASS`。

## 1.1 Linux 复核结果（2026-09-18）

Linux 已完成本轮可执行门禁：Node v26.7.0/npm 11.19.0 下 workspace check/test/build，Rust fmt/check/`wdio-e2e` feature check/workspace test/strict Clippy，Python compileall/pytest 10/10，普通与 `wdio-e2e` Tauri build，以及 WDIO adapter、wrapper、spec syntax/config load 均通过。当前仓库没有独立 Browser Mode 配置，因此不创建临时 Browser Mode 测试。

Linux Native WDIO 初次尝试曾因旧包名 `webkit2gtk-driver` 不可用而阻塞；Ubuntu 26.04（`resolute`）实际使用发行版替代包 `webkitgtk-webdriver`（驱动 `/usr/bin/WebKitWebDriver`）后，`npm run test:e2e --workspace desktop` 已通过：Dashboard smoke 2/2，tauri-driver 正常启动并完成 session/teardown。该结果仅证明 Linux 原生 smoke 可执行，不替代 Windows WebView2 验证。

## 1.2 状态更新（2026-09-22）

Linux revision 基线：`03332a1`（`fix: accept Microsoft Edge WebDriver banner in Tauri E2E harness`，18 files changed），已推送 `origin/feature/u7-desktop-production-integration`，working tree clean，`v0.2.0-pre.8` 未移动。第 1 节的“仍需 Windows 确认”清单已被 2026-09-22 实机结果部分覆盖：

- 根因确认：已安装 `@wdio/tauri-service@1.4.0` 的 `findMsEdgeDriver` 只接受 `MSEdgeDriver x.y` banner，而实际 Edge WebDriver 输出 `Microsoft Edge WebDriver x.y`，导致 `Driver: unknown`。Linux 侧由 `desktop/scripts/patch-wdio-tauri-service.mjs`（根 `postinstall` + desktop `pretest:e2e*` hooks，幂等）在干净 `npm ci` 后自动修正。
- 已确认（受控本地范围）：clean-install ordinary Dashboard `3/3`、advanced Dashboard `3/3` + plugin `2/2` 通过，真实 session 创建成功，退出后无 `xarchive-desktop`/`tauri-driver`/`msedgedriver` 进程及 `1420/4444/4445/9223` 端口残留。因此 WQ-P1-16/WQ-P1-17 在 local v2.0.6 scope 为 `WINDOWS_PASS`。
- 仍为 PENDING/BLOCKED：exact pinned toolchain（`tauri-driver 2.1.0-alpha.0` 安装成功，但 `msedgedriver 152.0.4191.66` 与实机 Edge `154.0.4258.24` 不兼容，session 未创建 → WQ-P1-16 `WINDOWS_FAIL`、WQ-P1-17 `WINDOWS_BLOCKED`）；hosted/release-runner 重复、发布前同一最终 artifact 的 readiness gate、人工便携 setup（WQ-P1-18/WQ-P1-19）以及真实 Registry/browser/Named Pipe 集成。

下一轮执行前提：先解决 Edge/WebView2 与 pinned driver 的版本对齐（把 workflow pin 调整到实机可用版本，或提供受控 Edge 152 runtime），再按第 3–8 节流程执行；不得为通过而放宽断言、timeout 或静默回退到本地 v2.0.6 工具链。

## 2. Handoff 清单

| ID | 类别 | 项目 | 目的 | 前置条件 | 优先级 | 人工交互 |
|---|---|---|---|---|---|---|
| WDIO-W-01 | Build/Toolchain | 同步与环境准备 | 确认 E: 工作副本对应当前 Linux revision | Windows 11、Node/npm、Rust/MSVC、Windows SDK、WebView2、`.venv` | P1 | no |
| WDIO-W-02 | Packaging/Build | 专用 `wdio-e2e` 构建 | 确认 plugin、capability 和 guest JS 只进入专用 artifact | WDIO-W-01 PASS | P1 | no |
| WDIO-W-03 | Runtime/Regression | Advanced plugin E2E | 验证 `window.wdioTauri`、execute、mock、日志和退出码 | WDIO-W-02 生成专用 exe | P1 | no |
| WDIO-W-04 | Runtime | Teardown/driver cleanup | 验证 session、mock store、应用和 driver 自动清理 | WDIO-W-03 完成或失败 | P1 | no |
| WDIO-W-05 | Packaging/Regression | 普通 release smoke | 确认普通 artifact 无 WDIO guest JS、capability 和 ACL warning | WDIO-W-04 检查完成 | P1 | no |
| WDIO-W-06 | Integration/Regression | 结果回写 | 更新验证报告、队列和当前状态 | 所有适用项完成 | P1 | yes |
| PORTABLE-W-01 | Runtime/Filesystem | 便携目录布局与下载 setup | 验证 exe 同目录 `config/`、`cache/`、`download/`、`logs/`、`sidecar/`、`extension/`、首次下载目录选择和 Downloads fallback | `build:portable:windows` 便携产物 | P1 | yes |
| PORTABLE-W-02 | Runtime/Regression | 日志等级与轮转 | 验证默认等级、显式配置优先、等级过滤和 `xarchive-*.log` 轮转/权限 | PORTABLE-W-01 便携目录可运行 | P1 | no |

## 3. WDIO-W-01：同步与环境准备

同步前记录：

```text
branch: dev
revision: <git rev-parse HEAD>
working tree: clean 或列出 working tree changes
source: /home/shiraishi/VSCode Workspace/Tw2Tg
target: E:\Shiraishi\VSCode Workspace\Tw2Tg
```

如果 Linux working tree 有未提交修改，Windows 报告必须明确说明验证包含 working tree changes。

同步前确认目标目录存在，并列出目标目录中的 `node_modules`、`.venv`、`target`、`X-Archive`、`aria2`、数据库、日志、验证产物和其他 extra files。不得无条件删除未知文件或覆盖 Windows 本地配置。

同步只覆盖源代码、配置、lockfile、脚本和文档，排除 `.git`、依赖、虚拟环境、构建缓存、数据库、日志、secrets 和 agent-local files。

同步后核对 `desktop/package.json`、`package-lock.json`、两个 WDIO wrapper、`desktop/e2e/specs/wdio-plugin.e2e.mjs`、`desktop/src/main.jsx`、`tauri.wdio.conf.json` 和 `capabilities/wdio.json`。

PowerShell 环境检查：

```powershell
node --version
npm --version
rustc --version
cargo --version
python --version
where.exe node
where.exe cargo
where.exe python
Get-ChildItem Env:PYTHON,Env:XARCHIVE_SIDECAR_PROGRAM,Env:XARCHIVE_SIDECAR_ARGS -ErrorAction SilentlyContinue
$env:PYTHON = "E:\Shiraishi\VSCode Workspace\Tw2Tg\.venv\Scripts\python.exe"
```

记录 Windows 版本、架构、Node/npm、Rust/Cargo、Python、Tauri CLI、WebView2/Edge 版本，以及 `TAURI_DRIVER_VERSION`、`EDGEDRIVER_VERSION`、`tauri-driver.exe` 和 `msedgedriver.exe` 的路径、版本与 SHA-256。

Linux 本轮源状态为：

```text
branch: dev
revision: 0537d32c9b4d2ef71ec508467d75378a767a34e7
working tree: dirty（验证包含当前未提交修改）
source: /home/shiraishi/VSCode Workspace/Tw2Tg
target: E:\Shiraishi\VSCode Workspace\Tw2Tg
```

## 4. WDIO-W-02：专用 artifact 构建

在 `E:\Shiraishi\VSCode Workspace\Tw2Tg` 执行：

```powershell
npm ci --no-audit --no-fund
npm run check
npm run test
npm run build
npm run build:tauri:wdio --workspace desktop
```

记录 `target\release\xarchive-desktop.exe` 路径、大小、SHA-256、Tauri plugin 编译结果、`wdio:default` permission，以及 linker/capability/ACL/bundling 错误。

预期：专用 executable 生成，plugin 和 `wdio` capability 可用。若 executable 已生成但 Tauri CLI 后处理返回非零，按细粒度测试状态归类为 `FAIL_PRODUCT`、`FAIL_TEST`、`BLOCKED_ENV` 或 `BLOCKED_AUTOMATION`；不得记为完整 PASS。

## 5. WDIO-W-03：Advanced plugin E2E

```powershell
$env:WDIO_APP_BINARY = "E:\Shiraishi\VSCode Workspace\Tw2Tg\target\release\xarchive-desktop.exe"
$env:WDIO_ADVANCED = "1"
$env:WDIO_CAPTURE_LOGS = "1"
$env:WDIO_LOG_DIR = "E:\Shiraishi\VSCode Workspace\Tw2Tg\desktop\test-artifacts\wdio-advanced"
npm run test:e2e:windows:advanced --workspace desktop
```

逐项确认：

1. 不出现 `spawnSync wdio.cmd` 或 `EINVAL`；
2. `browser.tauri.execute` 确认 `window.wdioTauri` 和其 `execute` 函数；
3. Dashboard `h1` 断言通过；
4. `browser.tauri.mock("get_app_status")` 返回预期 mock 值；
5. `browser.tauri.restoreAllMocks()` 成功且没有 mock-store cleanup 错误；
6. 测试失败时 runner 返回非零退出码；
7. 日志写入指定目录，且不包含 Cookie、token、secret 或个人账号数据。

## 6. WDIO-W-04：session、mock store 和 driver cleanup

无论 WDIO-W-03 成功还是失败，都执行：

```powershell
Get-Process xarchive-desktop,tauri-driver,msedgedriver -ErrorAction SilentlyContinue |
  Select-Object Id,ProcessName,Path

Get-NetTCPConnection -State Listen -ErrorAction SilentlyContinue |
  Where-Object { $_.LocalPort -in 1420,4444,4445,9223 } |
  Select-Object LocalAddress,LocalPort,OwningProcess
```

预期：应用、`tauri-driver`、`msedgedriver` 自动退出；端口 `1420`、`4444`、`4445`、`9223` 无本轮遗留监听；不出现 `A sessionId is required for this command`；不需要 `Stop-Process` 才能获得 PASS。

2026-09-16 Linux 修复后：`wdio-tauri-service.mjs` launcher 会在上游 teardown 后对幸存的 driver 进程执行进程树清理（Windows `taskkill /T /F`），并在无法清理时使运行失败。清理必须自动完成才算 PASS；safety-net 警告必须作为证据记录，仅上游 stop 成功（无警告）是理想结果；手工 `Stop-Process` 仍只能恢复环境、不能改变结果。

2026-09-16 第二轮修复：Windows 复验表明固定 alive-check 窗口在 Windows 误报（`taskkill /F` 成功后 OS 回收未完成，`kill(0)` 仍把已终止 PID 判活）。当前判定规则：child `exit` 事件优先 → 轮询（10s）→ 超时后以 tracked driver 端口（4444/4445）是否仍 LISTEN 做最终仲裁；「PID 未在窗口内退出」但端口已无监听时不再使运行失败，safety-net 警告仍需记录。若警告后 tracked 端口仍 LISTEN，仍判 FAIL。

如需手动清理，先记录 PID、进程路径、端口和日志，并将步骤记为对应的 `FAIL_PRODUCT`、`FAIL_TEST`、`BLOCKED_ENV` 或 `BLOCKED_AUTOMATION`。手动清理只能恢复环境，不能改变结果。

## 7. WDIO-W-05：普通 release 回归

```powershell
Remove-Item Env:VITE_WDIO_E2E -ErrorAction SilentlyContinue
Remove-Item Env:WDIO_ADVANCED -ErrorAction SilentlyContinue
Remove-Item Env:WDIO_CAPTURE_LOGS -ErrorAction SilentlyContinue
npm run build:tauri --workspace desktop
$env:WDIO_APP_BINARY = "E:\Shiraishi\VSCode Workspace\Tw2Tg\target\release\xarchive-desktop.exe"
npm run test:e2e:windows --workspace desktop
```

在普通和 advanced WDIO 命令前，必须确保以下环境已由 workflow 或手工步骤设置：

```powershell
$env:WDIO_AUTO_INSTALL_TAURI_DRIVER = "0"
$env:WDIO_AUTO_DOWNLOAD_EDGE_DRIVER = "0"
# 可选：仅在受控环境需要精确固定 driver 时设置；设置后 service 要求 driver 版本完全相等。
# 不设置时 service 只要求 driver 的 major 与 WebView2 runtime 的 major 一致。
# $env:EDGEDRIVER_VERSION = "<明确的 msedgedriver 版本>"
where.exe tauri-driver.exe
where.exe msedgedriver.exe
tauri-driver.exe --version
msedgedriver.exe --version
```

选择 driver 的规则（2026-09-22 复核，与 `@wdio/tauri-service` 实现一致）：

1. 不设置 `EDGEDRIVER_VERSION`：PATH 上的 `msedgedriver` 只要与 WebView2 runtime 共享 major 即可使用（Edge 与 driver 的 patch 版本本来就会漂移）；
2. 设置 `EDGEDRIVER_VERSION`：service 要求 driver 版本与该值完全相等，不满足则失败；
3. 关闭自动下载时，两条路径都不会回退到隐式下载，因此必须在执行前确认 `where.exe msedgedriver.exe` 命中正确的 driver，并记录路径、版本和 SHA-256；
4. Windows release workflow 使用同一规则：固定 `tauri-driver 2.1.0-alpha.0`，校验镜像/环境提供的 `msedgedriver` 与 WebView2 的 major 一致，并把版本、banner 与 SHA-256 写入 `toolchain.json`。

如果固定 driver 不存在或版本不匹配，状态为 `BLOCKED_ENV`，不得切换为自动下载后报告 PASS。

普通和 advanced WDIO 命令前，还应检查 preflight `environment.json` 中的 `msedgedriver_banner_accepted`：

```powershell
(Get-Content (Join-Path $env:READINESS_DIAGNOSTICS "environment.json") -Raw | ConvertFrom-Json).msedgedriver_banner_accepted
(Get-Content (Join-Path $env:READINESS_DIAGNOSTICS "environment.json") -Raw | ConvertFrom-Json).msedgedriver_banner_reason
```

该字段只用于诊断。若 service 仍报告 `Driver: unknown`，保持 `BLOCKED_AUTOMATION`，不得据此标记原生 UI PASS。

确认普通 artifact 不加载 `@wdio/tauri-plugin` guest JS、不启用 `wdio:default`、不产生 `plugin:wdio|execute not allowed by ACL`；Dashboard heading、`main`、导航、归档概览和进程清理均通过。

## 8. WDIO-W-06：结果记录与回写

每个项目记录：实际命令和工作目录、Linux branch/revision/working tree、Windows/工具链版本、artifact 路径和 hash、日志目录、状态、错误分类、是否阻塞后续项目和建议。

结果写回顺序：

1. `docs/development/windows-validation.md` 增加本轮记录；
2. `docs/validation/windows-queue.md` 更新 WQ-P1-16/WQ-P1-17；
3. `docs/development/status.md` 更新当前事实；
4. Linux 代码问题只记录 reproduction、相关文件和建议修复，不在 Windows 验证阶段直接修改业务代码；
5. 不从 Windows 工作副本反向同步代码、依赖、构建产物、日志或凭据。

## 9. 状态判定

- `PASS`：命令实际执行且满足全部预期；
- `FAIL`：执行了但行为、输出或清理不符合预期；
- `BLOCKED`：缺少依赖、权限、账号、外部服务或前置 artifact；
- `NOT RUN`：本轮未执行但理论上适用，必须写明原因；
- `NOT APPLICABLE`：当前项目配置或范围明确不适用。

单个测试用例必须进一步使用 `PASS_FLAKY`、`PASS_AFTER_FIX`、`PASS_AFTER_TEST_FIX`、`FAIL_PRODUCT`、`FAIL_PRODUCT_NEEDS_DEVELOPMENT`、`FAIL_TEST`、`BLOCKED_ENV`、`BLOCKED_AUTOMATION`、`SKIPPED_PLATFORM` 或 `NEEDS_REVIEW`；不得用模糊的单独 `FAIL`/`BLOCKED` 隐藏诊断分类。
### 固定 msedgedriver 前置（Windows）

在执行 WQ-P1-16/WQ-P1-17 前，可使用 E: 验证副本中已保存的 driver：

    $root = "E:\Shiraishi\VSCode Workspace\Tw2Tg"
    $driverDir = Join-Path $root "desktop\test-artifacts\msedgedriver\152.0.4191.66"
    $env:Path = "$driverDir;$env:Path"
    where.exe msedgedriver.exe
    msedgedriver.exe --version

预期版本为 152.0.4191.66。该目录是 Windows 本地验证前置，不纳入 Git，也不反向同步到 Linux source。若 service 仍输出自动下载 warning，应记录该事实并继续观察 tauri-driver/worker；不能仅凭 PATH 命中宣称 WQ-P1-16 或 WQ-P1-17 通过。

版本兼容前置（2026-09-22 新增）：该 pinned driver 只支持 Edge 152，而验证机当前 Edge 为 `154.0.4258.24`，会导致 session 创建前的 `This version of Microsoft Edge WebDriver only supports Microsoft Edge version 152` 失败。因此执行 WQ-P1-16/WQ-P1-17 前必须先满足以下任一条件，并在结果中记录所选路径：

1. 把 workflow/本地 pin 调整为与实机 Edge/WebView2 版本匹配的官方 msedgedriver；或
2. 在验证环境提供受控 Edge 152 runtime（与 pinned driver 版本一致）。

在版本对齐前，ordinary/advanced 的 pinned-toolchain 结果只能记录为 `FAIL`（`BLOCKED_ENV` 分类），不得记为产品缺陷，也不得静默使用本地 v2.0.6 工具链冒充 pinned 结果。

## 10. 便携版 Windows 验证步骤（PORTABLE-W-01/02，对应 WQ-P1-18/WQ-P1-19）

以下步骤针对 `npm run build:portable:windows --workspace desktop` 组装的便携目录；Linux 门禁和组装 smoke 不能替代其中任何一项。

### 10.1 便携目录布局与首启（PORTABLE-W-01）

```powershell
$root = "E:\Shiraishi\VSCode Workspace\Tw2Tg"
npm run build:portable:windows --workspace desktop
# 使用脚本实际输出的便携目录路径，例如：
$portable = "$root\desktop\portable\XArchive"
Get-ChildItem $portable
```

首启前目录检查：

1. 存在 `xarchive-desktop.exe`、`extension/`；可选存在 `sidecar/gallery-dl/`、`sidecar/aria2/`；
2. 不存在 `download/`、`config/`、`cache/`、`logs/`（由首启创建，构建脚本不预创建 `download/`）；
3. 不存在 `telegram/` 目录（当前便携布局不创建该目录）。

首启与下载目录选择：

1. 直接运行 exe，首次启动应出现下载目录选择面板；
2. 选择“便携目录”：确认生成 `config/`（含 `config.yaml` 与 `archive.sqlite3`）、`cache/staging/`、`download/` 和同级 `logs/`；
3. 重启应用：不再出现 setup 面板，`config.yaml` 保留下载目录设置；
4. 归档一条 Tweet，确认最终文件位于 `download/`，staging 位于 `cache/staging/` 且提交后清理。

跨路径与只读测试：

1. 将便携目录整体复制到另一盘符（例如 `D:\`）后运行，确认所有路径相对新位置派生，无旧位置绝对路径残留；
2. 将 `download/` 上级设为只读（或以拒绝创建的方式）重试 setup：选择便携目录失败时应 fallback 到系统 `Downloads/XArchive`，归档写入该目录且 `config.yaml` 记录所选模式；
3. sidecar 定位优先级：设置 sidecar/aria2 相关环境变量时优先于 `config.yaml` 的 `sidecar.gallery_dl`/`sidecar.aria2`；无环境变量时使用便携目录 `sidecar/` 下的程序（以 `commands.rs` 当前解析顺序为准）；
4. 若系统拒绝在便携目录创建文件，应用不得崩溃，错误应可诊断。

### 10.2 日志等级与轮转（PORTABLE-W-02）

1. Release 便携 exe 默认等级应为 `info`：`logs/xarchive-*.log` 首行为 `level=info`；
2. 通过 GUI 或 `config.yaml` 依次设为 `debug`、`warning`、`error`、`silent` 并重启，确认等级过滤行为；`silent` 不创建日志文件；
3. 将 `logging.max_files` 设为默认 5 及 1–100 边界值，反复重启生成超过上限的日志文件，确认最旧文件被删除且数量不超过上限；越界值（如 0、101）应回退默认并给出启动诊断；
4. 将 `logs/` 设为只读后启动：应用不得崩溃，权限错误应记录并可诊断。

### 10.3 人工 fallback 与结果规则

- 文件选择器、资源管理器“打开文件夹”、权限/UAC 提示等自动化无法稳定覆盖的步骤使用人工交互，按第 9 节状态判定记录；
- 每项记录实际命令、便携目录路径、Windows 版本、失败输出和相关日志路径；
- Linux 门禁通过不改变 WQ-P1-18/WQ-P1-19 状态；只有 Windows 实际满足全部预期后才可改写为 `WINDOWS_PASS`，结果按第 8 节顺序回写。