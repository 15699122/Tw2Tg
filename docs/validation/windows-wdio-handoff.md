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

上述项目在实际执行前不得改成 `WINDOWS_PASS`。

## 1.1 Linux 复核结果（2026-09-18）

Linux 已完成本轮可执行门禁：Node v26.7.0/npm 11.19.0 下 workspace check/test/build，Rust fmt/check/`wdio-e2e` feature check/workspace test/strict Clippy，Python compileall/pytest 10/10，普通与 `wdio-e2e` Tauri build，以及 WDIO adapter、wrapper、spec syntax/config load 均通过。当前仓库没有独立 Browser Mode 配置，因此不创建临时 Browser Mode 测试。

Linux Native WDIO 初次尝试曾因旧包名 `webkit2gtk-driver` 不可用而阻塞；Ubuntu 26.04（`resolute`）实际使用发行版替代包 `webkitgtk-webdriver`（驱动 `/usr/bin/WebKitWebDriver`）后，`npm run test:e2e --workspace desktop` 已通过：Dashboard smoke 2/2，tauri-driver 正常启动并完成 session/teardown。该结果仅证明 Linux 原生 smoke 可执行，不替代 Windows WebView2 验证。

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

记录 Windows 版本、架构、Node/npm、Rust/Cargo、Python、Tauri CLI、WebView2/Edge 版本。

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