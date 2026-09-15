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

## 2. Handoff 清单

| ID | 类别 | 项目 | 目的 | 前置条件 | 优先级 | 人工交互 |
|---|---|---|---|---|---|---|
| WDIO-W-01 | Build/Toolchain | 同步与环境准备 | 确认 E: 工作副本对应当前 Linux revision | Windows 11、Node/npm、Rust/MSVC、Windows SDK、WebView2、`.venv` | P1 | no |
| WDIO-W-02 | Packaging/Build | 专用 `wdio-e2e` 构建 | 确认 plugin、capability 和 guest JS 只进入专用 artifact | WDIO-W-01 PASS | P1 | no |
| WDIO-W-03 | Runtime/Regression | Advanced plugin E2E | 验证 `window.wdioTauri`、execute、mock、日志和退出码 | WDIO-W-02 生成专用 exe | P1 | no |
| WDIO-W-04 | Runtime | Teardown/driver cleanup | 验证 session、mock store、应用和 driver 自动清理 | WDIO-W-03 完成或失败 | P1 | no |
| WDIO-W-05 | Packaging/Regression | 普通 release smoke | 确认普通 artifact 无 WDIO guest JS、capability 和 ACL warning | WDIO-W-04 检查完成 | P1 | no |
| WDIO-W-06 | Integration/Regression | 结果回写 | 更新验证报告、队列和当前状态 | 所有适用项完成 | P1 | yes |

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

预期：专用 executable 生成，plugin 和 `wdio` capability 可用。若 executable 已生成但 Tauri CLI 后处理返回非零，记为 `FAIL` 或 `BLOCKED`，不得记为完整 PASS。

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

如需手动清理，先记录 PID、进程路径、端口和日志，并将步骤记为 `FAIL` 或 `BLOCKED`。手动清理只能恢复环境，不能改变结果。

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