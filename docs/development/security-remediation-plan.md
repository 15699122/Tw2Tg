# 依赖与供应链安全整改计划

> 本文记录 2026-09-30 对 GitHub Security 页相关告警的评估与分批整改计划。
> 本文是**计划与当前状态**，不是验证证据；验证结果写入 [`../validation/windows-queue.md`](../validation/windows-queue.md) 和 [`../validation/windows-validation-history.md`](../validation/windows-validation-history.md)。
> 风险等级结论见 [`risk-register.md`](risk-register.md) RISK-023 与 RISK-026。

## 6. 2026-10-04 跨平台复核与实施结果

本节记录在分支 `cross-platform/automatic-pairing-reconcile-20261002`、基线
`adedcceca5e01a8f0a2c9d2bfa9432308496d177` 上的续作。SEC-A 的 Windows
发布/打包实现不属于 Linux Owner，本轮不修改 Windows workflow；Windows
专属步骤汇总在 [`../validation/windows-queue.md`](../validation/windows-queue.md)。

### SEC-B：Sidecar 输入命令积压（共享实现）

- 复核确认 Rust supervisor 对 stdout/stderr 行已有 1 MiB 上限；gallery-dl
  stderr 使用临时文件并仅保留有界尾部。这些既有保护不限制 Sidecar stdin
  reader 在长 extraction/discovery 期间积累的 Python 命令队列。
- `run_v2_worker()` 原使用无界 `queue.Queue()`；metadata/event 及控制命令均
  可在队列中无限增长，且 shutdown sentinel 排在已读命令之后。此为已确认的
  共享资源耗尽路径，按 `CROSS_PLATFORM_REVIEW_REQUIRED` 修复：队列上限 32，
  满时 reader 阻塞形成背压；EOF sentinel 在 worker drain 过程中设置 reader
  stop event，避免继续读入无界数据。没有改变事件 schema 或协议版本。
- 回归测试覆盖超过队列容量的 metadata 积压、队列排空及 shutdown；测试不依赖
  外部账号或网络。真实 Windows stdin/进程关闭语义仍需目标平台验证。

### SEC-C：公告与依赖路径复核

- **glib：**2026-10-04 查阅 RustSec 官方 `RUSTSEC-2024-0429`，修复边界仍为
  `glib >=0.20.0`，受影响函数为 `VariantStrIter` 的迭代器方法。当前锁定
  `glib 0.18.5`。Linux `cargo tree --locked --target x86_64-unknown-linux-gnu
  -i glib` 确认经 Tauri/GTK3/WebKit2GTK 可达；Windows MSVC target 的同一反向
  查询无输出。依赖约束不支持安全单包升级，当前不伪装引入新主版本、不 fork；
  告警保持未解决，待上游 GTK4/WebKit6 迁移后重评。真实 Linux optimized
  release/GUI 验证未执行。
- **extract-zip：**2026-10-04 查阅 GitHub Advisory Database 的两条公告，均
  影响 `<=2.0.1` 且暂无 patched release；一条针对越界 symlink target，另一条
  针对同名 symlink 后普通文件写穿。当前 lock tree 仍为 `extract-zip 2.0.1`，
  由 WDIO Puppeteer browsers 路径引入；当前 `npm audit` 还报告其他更新公告，
  因而不能沿用旧的“审计只剩 extract-zip”快照。未改依赖、未宣称消除风险。
  普通 WDIO/Windows 执行入口和隔离解压的两类攻击回归需 Windows Owner 在
本机 `cargo audit --no-fetch --json` 报 `vulnerabilities: 0`，仍报告
unsound/unmaintained warnings；`--no-fetch` 使用缓存 advisory DB，不能作为
最新在线审计，也不能据此关闭 glib 风险。在线 `npm audit --json` 于本次执行
返回 20 条 HIGH 计数；其中包括 extract-zip/Wdio 链及其它当前 lockfile 命中，
不应与 2026-09-30 的审计快照混为一谈，也不等同于 GitHub 告警状态。
  条 HIGH 计数；此结果是本地 lockfile 的当前审计快照，不等同于 GitHub 告警状态。

### 当前状态

| 批次 | 本轮结果 |
|---|---|
| SEC-A | Windows-owned 发布/资源摘要校验和 job 权限隔离未实施；转入 Windows 手工队列 |
| SEC-B | 有界命令队列与 Linux 回归测试已实施；Windows stdin/shutdown 行为待验证 |
| SEC-C | 官方公告和锁文件路径已复核；glib 无安全兼容升级路径，保留未解决风险；网络隔离测试在 bwrap 内因无可用 `/tmp` 失败，未形成等价审计证据；未执行 optimized/release profile |

## 1. 评估基线

- 评估日期：2026-09-30；评估方式：GitHub API（Dependabot / Code scanning / Secret scanning）+ 本地依赖树核对。
- 评估分支：`origin/main`，基线提交 `c98d61061e3c09aa03ccd3609740fc5acb80cbe3`。
- 整改分支：`security/dependency-advisories-2026-09-30`，从 `origin/main` 创建。
- 本地 `release/v0.2.0`（`7a3374b`）已完全包含于 `origin/main`，不作为整改基线。
- 仓库默认分支为 `main`；Dependabot 告警与 CodeQL 告警均以 `main` 为准。

### 1.1 告警清单

| 告警 | GHSA | 依赖 | 公告等级 | 依赖链（本地核对） | 处置批次 |
|---|---|---|---|---|---|
| Dependabot #4 | GHSA-jmr9-qjv8-65gv | `extract-zip@2.0.1` | high | `@wdio/utils` → `@puppeteer/browsers@2.13.2` → `extract-zip` | B（既有接受风险，需复核触发条件） |
| Dependabot #6 | GHSA-7pqw-9j4j-h8q3 | `extract-zip@2.0.1` | high | 同上 | B（同上） |
| Dependabot #14 | GHSA-j6r3-76f7-8jcv | `ip-address@10.7.0` | medium | `webdriverio` → `@wdio/utils` → `proxy-agent` → `socks-proxy-agent` → `socks` → `ip-address` | A |
| Dependabot #11 | GHSA-q2hr-2g5m-vwhr | `brace-expansion@2.1.4` | medium | `minimatch@9.0.9`（`@wdio/config` → `glob`）、`minimatch@5.1.9`（`archiver`、`jake`）、`mocha` | A |
| Dependabot #12 | GHSA-q2hr-2g5m-vwhr | `brace-expansion@1.1.18` | medium | `recursive-readdir` → `minimatch@3.1.5` → `brace-expansion@1.1.18` | A |
| Dependabot #1 | GHSA-wrw7-89jp-8q8g | `glib@0.18.5` | medium | `tauri` / `tao` / `wry` / `webkit2gtk` → `gtk` / `gdk` → `glib` | C |
| Code scanning #2 | `actions/missing-workflow-permissions` | `.github/workflows/windows-worker-artifact.yml:14-75` | medium | 工作流缺少 `permissions` 声明 | D（Windows Platform Owner） |

### 1.2 治理缺口

- 基线仓库缺少 `SECURITY.md`；2026-09-30 通过 GitHub API `GET /repos/15699122/Tw2Tg/private-vulnerability-reporting` 核实 `enabled: true`，因此**私密报告功能原已启用**，缺失的是版本支持与报告说明文档。
- 仓库未配置 `.github/dependabot.yml`，安全更新无固定分组与节奏。
- 仓库 Action 权限设置当前为 `default_workflow_permissions: read`，`GITHUB_TOKEN` 默认只读。
- Secret scanning 与 push protection 已启用，公开 API 未返回未关闭的 secret 告警；这不等于证明无凭据泄露。

## 2. 结论与限制

- 没有证据表明项目已遭攻击，也没有证据表明这些依赖可直接被终端用户远程利用。
- 三种 npm 告警落在开发/测试期依赖，Code scanning 告警落在工作流；`glib` 则是 Linux Tauri 桌面运行时的传递依赖，不得归类为测试专用。
- 既有结论 [`risk-register.md`](risk-register.md) RISK-023 已接受 `extract-zip` 风险，并已确认 `desktop/wdio.conf.mjs` 的 `autoDownloadEdgeDriver: true` 使解压路径**可达**。本计划不推翻该结论，只补充复核触发条件与两条公告的区分。
- **`glib` 的平台范围已核实（2026-09-30，`cargo tree --locked`）：**`cargo tree --locked --target x86_64-pc-windows-msvc -i glib` 输出 `nothing to print`，且 Windows target 依赖树中 `gtk`/`glib` 计数为 0；`glib 0.18.5` 仅出现在 `x86_64-unknown-linux-gnu` target。Windows 走 WRY/WebView2，不引入 GTK。因此 Dependabot #1 是 **Linux-only** 依赖告警，Windows 不受该公告影响。
- 本轮 Linux 环境无 X server，**未执行真实 Linux GUI 运行验证**；上述结论仅基于依赖图，不等于 Linux 运行时行为已验证。

## 3. 批次与验收门槛

### 批次 A：可安全升级的 npm 依赖（Owner：Linux Cross-platform Owner）

范围：`package.json`、`package-lock.json`。

- `ip-address` 提升到不低于 `10.7.1`。
- `brace-expansion` 1.x 路径提升到不低于 `1.1.21`；2.x 路径提升到不低于 `2.1.7`；不得把不同主版本统一强制到同一版本。
- 优先使用定向 lock 更新；仅在父依赖约束阻止时才考虑最小范围 `overrides`，并记录移除条件。
- 不使用 `npm audit fix --force`，不夹带无关的大版本升级。

验证：`npm ci`；`npm ls ip-address brace-expansion --all`；`npm audit --json`；`npm run test --workspace desktop`；`npm run check --workspaces --if-present`；审查 `package-lock.json` diff。

门槛：#11、#12、#14 的受影响版本在依赖树中消失，且上述命令通过。`extract-zip` 仍可能使 `npm audit` 非零，必须按告警逐条判定，不得整体记为失败。

### 批次 B：`extract-zip` 两条 High 告警（Owner：Linux；Windows 协助验证）

- 先复核既有接受风险的**触发条件**是否仍成立：WDIO 全部入口（ordinary、advanced、本地、CI）的自动下载开关、下载地址来源、代理与缓存可配置性、相关任务是否持有发布凭据或写权限。
- 优先路径：升级到经核实已移除或修复该依赖的上游版本，且不破坏 `desktop/scripts/patch-wdio-tauri-service.mjs` 与现有驱动固定策略。
- 次选路径：强化隔离（禁止隐式下载并失败关闭、与发布签名任务隔离、一次性工作目录、预置产物校验来源与哈希）。该路径只能记为**缓解**，不得记为漏洞已修复。
- 不采用：把其它 ZIP 库 `override` 成 `extract-zip`；强推不满足 `@wdio/utils` `^2.2.0` 约束的 `@puppeteer/browsers` 3.x。
- 回归用例需分别覆盖两条公告：符号链接指向解压目录外；同名“符号链接 → 普通文件”条目；以及已存在的外部指向链接、正常 ZIP、禁网失败关闭。测试只在隔离临时目录进行。

门槛：优先实现不安全入口消除；若本轮无法消除，维持 RISK-023 接受状态并复核其触发条件是否需要更新。

### 批次 C：`glib` VariantStrIter（Owner：Linux Cross-platform Owner）

范围：`Cargo.lock`、`desktop/src-tauri/Cargo.toml`。

- **平台范围（已核实）：**该告警只影响 Linux target，Windows 的 WRY/WebView2 链路不含 `gtk`/`glib`，因此不需要 Windows 侧的依赖处置或构建回归。
- 调查上游 GTK 依赖是否真正调用受影响的字符串 Variant 迭代路径。
- 公告修复边界为 `0.20.0`，但当前 GTK 依赖链并不允许直接替换；不得通过新增 `glib 0.20` 而保留旧版的方式伪装消除。
- 若无法通过兼容升级消除，再评估经审查的回移或限期暂缓。

门槛：给出“已消除 / 已修补 / 有证据的限期暂缓”三选一的明确结论，并完成 **optimized/release profile** 构建验证；debug 构建或单元测试通过不足以判定完成。真实 Linux GUI 运行验证受限于本机无 X server，需与依赖图结论分开记录。

#### C1 兼容升级调查（2026-09-30）

- `cargo tree --locked --offline --target x86_64-unknown-linux-gnu -i glib` 确认 `glib 0.18.5` 由 Tauri 的 GTK3/WebKit2GTK 链引入；Windows target 返回 `nothing to print`。
- 当前缓存中 `gtk 0.18.2`、`gdk 0.18.2` 的 manifest 约束均为 `glib = "0.18"`，`webkit2gtk 2.0.2` 为 `glib = "^0.18.0"`；`cargo update -p glib --dry-run --verbose --offline` 显示 `Locking 0 packages`，不能通过单包更新到修复边界 `0.20.0`。
- 上游 `tauri-apps/wry` issue #1474（GTK4/WebKit6 迁移）在核验时仍为 `open`。**经 crates.io API 核验（2026-09-30）：最新非撤回版本 `wry 0.57.0`、`0.56.1`、`0.56.0` 的 Linux 依赖仍为 `gtk ^0.18` + `webkit2gtk =2.0.2`**，即升级到当前最新 `wry` 也不会离开 `glib 0.18`。迁移目标 `gtk4`（最新 0.11.5）与 `webkit6`（最新 0.6.1）已发布，但本项目 Tauri 2 依赖链尚未采用。
- **C1 结论：本批次无法通过兼容升级消除该告警。** 本轮**没有更改 Cargo 依赖**；既不新增 `glib 0.20` 伪装消除，也不引入未经审查的 fork。后续路径为跟进上游 GTK4/WebKit6 迁移版本后重做 C1，或按“经审查的回移 / 限期暂缓”处理。
- `cargo audit --no-fetch --json` 对本机既有 advisory DB 返回 `vulnerabilities: []`，但 `warnings.unsound` 仍包含 `RUSTSEC-2024-0429`（`glib`），另有 6 条 `unmaintained`（`proc-macro-error`、`unic-*`）；不能把“漏洞数为零”误当作本告警解决。`--no-fetch` 不验证公告库为最新；CI 审计门禁和 Linux release/GUI 验证均未执行。

### 批次 D：工作流权限与安全治理（Owner：Windows Platform Owner + 仓库管理员）

- `windows-worker-artifact.yml` 显式声明 `contents: read`，并评估 `actions/checkout` 的 `persist-credentials: false`。
- 另行复核根级 `contents: write` 的发布类工作流是否可拆分权限；该建议不属于本轮已确认漏洞。
- 新增根目录 `SECURITY.md`：支持版本与平台、私密报告渠道、报告需附证据、禁止提交真实账号凭据、响应目标、Linux/Windows 责任划分。
- 新增 `.github/dependabot.yml`（npm、Cargo、GitHub Actions），patch/minor 与 major 分组。
- 为 npm 审计与已确认适用的 Rust 审计建立可重复的门禁，例外清单必须带到期日。

门槛：默认分支重扫后 Code scanning #2 关闭；`SECURITY.md` 与 Dependabot 配置上线。

#### 批次 D 执行结果（2026-09-30）

- **已完成（仓库管理员 / Linux 部分）：**新增根 `SECURITY.md`（支持范围为 `main` 与最新 pre-release；私密报告走 GitHub Security Advisories；含报告/不报告边界、响应目标与双 Owner 归属），新增 `.github/dependabot.yml`（npm、Cargo、GitHub Actions，weekly，npm/Cargo 按 patch/minor 与 security 分组）。YAML 解析通过，3 个 ecosystem；API 确认仓库现有 `dependencies` label，但未创建 `security`/`ci`，因此配置只引用现有 label；`python-pip` 仅作注释说明、未启用（当前 worker 为本地 editable 包，无已发布的固定 requirements 可审计）。
- **未完成（Windows Platform Owner）：**`windows-worker-artifact.yml` 的 `permissions` 声明属 Windows 打包/CI 资产，Linux 不自行修改，已登记为 WQ-SEC-PERMS-01。发布类工作流根级 `contents: write` 的权限拆分属进一步加固建议，未实施。
- **未实施的门禁：**npm 审计与 Rust 审计的 CI 门禁尚未建立。本机已安装 `cargo-audit 0.22.2` 并完成一次 `--no-fetch` 本地审计；这不等于仓库已集成 CI 或公告数据库已更新。门禁需要先明确 `extract-zip` 和 `glib` 当前接受/阻塞风险的可审计例外及期限，不得将 `warnings.unsound` 静默忽略。
- **设置核验：**私密漏洞报告 API 返回 `enabled: true`，与新 `SECURITY.md` 所述报告渠道一致；尚未用匿名访问者账号实测网页按钮。

## 4. 当前批次状态（2026-09-30，合并后）

分支 `security/dependency-advisories-2026-09-30`，从 `origin/main` `c98d6106` 创建，已通过 PR #9（squash `ed8c1fcad0403169ac230ebbe924dff01f907056`）与 PR #7（squash `f3c087697881f18bf1f6dda2381216647e052c59`）合并；合并后 `origin/main` 为 `f3c0876`。

| 批次 | 状态 | 结果 |
|---|---|---|
| A：npm 可安全升级依赖 | 已完成 | `ea465fa`，仅锁文件变更；`ip-address` 10.7.2、`brace-expansion` 2.1.7 / 1.1.21。**Dependabot #11、#12、#14 已由默认分支重扫关闭（2026-09-30T08:35Z），连带关闭 #7–#10、#13** |
| B：`extract-zip` 两条 High | 已复核，无代码变更 | 上游无修复版本，RISK-023 接受风险维持；发布流水线已关闭自动下载（缓解证据，非修复）。**#4、#6 仍为 open** |
| C：`glib` VariantStrIter | C1 已调查，**本批次无法消除** | 平台范围 Linux-only；最新 `wry 0.57.0` 仍为 `gtk ^0.18` + `webkit2gtk =2.0.2`，无兼容升级路径；未改 Cargo 依赖，**#1 仍为 open** |
| D：工作流权限与治理 | 部分完成 | `SECURITY.md` 与 `dependabot.yml` 已合并进 `main`；worker 工作流 `permissions` 属 Windows Platform Owner（WQ-SEC-PERMS-01），**Code scanning #2 仍为 open** |

### 合并后 Linux 验证（全部实际执行）

在 `main` `f3c0876` 的干净工作树上重跑：`npm ci` PASS；desktop **154/154**；extension **32/32**；`npm run check --workspaces --if-present` PASS；`npm run build --workspaces --if-present` PASS；`npm ls ip-address brace-expansion` 显示 10.7.2 / 2.1.7 / 1.1.21；`npm audit` 剩余 13 条全部源自 `extract-zip`；`cargo fmt --all -- --check` PASS；`cargo check --locked --all-targets` PASS；严格 Clippy `-D warnings` PASS；`cargo test --workspace --locked` **262/262**；`pytest sidecar/tests` **46/46**；5 个 workflow 与 `dependabot.yml` YAML 解析 PASS；`git diff --check` PASS。PR #9 的 CodeQL 四语言检查（actions/javascript-typescript/python/rust）全部 SUCCESS。

## 5. 发布与交接约束

- 不因开发依赖告警就判定用户版必须紧急撤回。
- 若不可信 ZIP 可进入持有发布凭据的解压任务，应暂停该任务直到隔离或修复完成。
- 修复必须进入 `main` 及仍受支持的分支；停留在独立分支不得宣告告警处理完成。
- 告警关闭以默认分支重扫结果为准，不以本地命令成功为准。
- 正式交接按 [`git-platform-handoff.md`](git-platform-handoff.md) 记录分支、源提交、交接提交、未提交状态与 Owner。


## Windows return — 2026-10-05

Exact aa8dabd native builds and targeted automated checks passed; acceptance remains WINDOWS_VERIFICATION_PENDING. Packaged queue fixture startup and new settings GUI matrix remain pending. Release trusted digest and job isolation static review did not meet criteria; implementation and isolated Actions remain pending. Current evidence, hashes and Owner routing are in ../validation/windows-validation-history.md and ../status/platform-handoff.md.


## Windows continuation — 2026-10-05 / 39f54e5

SEC-A trusted Windows asset pins and isolated read-only build / separate publish job are implemented. Negative fixtures and isolated Windows Actions run 37255273960 PASS; full production upload=false rehearsal and CodeQL evidence remain pending. Packaged queue control/worker-exit cases and scoped current-size settings GUI checks now PASS; subtree cleanup and full settings matrix remain open. Evidence and formal Owner routing: ../validation/windows-validation-history.md and ../status/platform-handoff.md. Real Telegram testing deferred by Owner for this local/CI batch.
