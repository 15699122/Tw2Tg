# 依赖与供应链安全整改计划

> 本文记录 2026-09-30 对 GitHub Security 页相关告警的评估与分批整改计划。
> 本文是**计划与当前状态**，不是验证证据；验证结果写入 [`../validation/windows-queue.md`](../validation/windows-queue.md) 和 [`../validation/windows-validation-history.md`](../validation/windows-validation-history.md)。
> 风险等级结论见 [`risk-register.md`](risk-register.md) RISK-023 与 RISK-026。

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

- 仓库缺少 `SECURITY.md`，公开 Security 页无可用的私密报告渠道。
- 仓库未配置 `.github/dependabot.yml`，安全更新无固定分组与节奏。
- 仓库 Action 权限设置当前为 `default_workflow_permissions: read`，`GITHUB_TOKEN` 默认只读。
- Secret scanning 与 push protection 已启用，公开 API 未返回未关闭的 secret 告警；这不等于证明无凭据泄露。

## 2. 结论与限制

- 没有证据表明项目已遭攻击，也没有证据表明这些依赖可直接被终端用户远程利用。
- 全部 7 条告警都落在**开发/测试期依赖或工作流**上；`extract-zip`、`ip-address`、`brace-expansion` 不进入发布产物，`glib` 是 Tauri 桌面运行时的传递依赖。
- 既有结论 [`risk-register.md`](risk-register.md) RISK-023 已接受 `extract-zip` 风险，并已确认 `desktop/wdio.conf.mjs` 的 `autoDownloadEdgeDriver: true` 使解压路径**可达**。本计划不推翻该结论，只补充复核触发条件与两条公告的区分。
- 本轮未完成：`glib` 的 Windows target 依赖树核对（离线缓存缺 `clipboard-win`，`cargo tree --target x86_64-pc-windows-msvc` 失败），因此该项不得标记为 Windows `NOT_APPLICABLE`。

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

- 先补齐 Linux 与 Windows 各 target 的依赖树核对（Windows 侧需可用的离线/在线 registry 缓存）。
- 调查上游 GTK 依赖是否真正调用受影响的字符串 Variant 迭代路径。
- 公告修复边界为 `0.20.0`，但当前 GTK 依赖链并不允许直接替换；不得通过新增 `glib 0.20` 而保留旧版的方式伪装消除。
- 若无法通过兼容升级消除，再评估经审查的回移或限期暂缓。

门槛：给出“已消除 / 已修补 / 有证据的限期暂缓”三选一的明确结论，并完成 **optimized/release profile** 构建验证；debug 构建或单元测试通过不足以判定完成。涉及共享依赖变更时安排 Windows 构建验证。

### 批次 D：工作流权限与安全治理（Owner：Windows Platform Owner + 仓库管理员）

- `windows-worker-artifact.yml` 显式声明 `contents: read`，并评估 `actions/checkout` 的 `persist-credentials: false`。
- 另行复核根级 `contents: write` 的发布类工作流是否可拆分权限；该建议不属于本轮已确认漏洞。
- 新增根目录 `SECURITY.md`：支持版本与平台、私密报告渠道、报告需附证据、禁止提交真实账号凭据、响应目标、Linux/Windows 责任划分。
- 新增 `.github/dependabot.yml`（npm、Cargo、GitHub Actions），patch/minor 与 major 分组。
- 为 npm 审计与已确认适用的 Rust 审计建立可重复的门禁，例外清单必须带到期日。

门槛：默认分支重扫后 Code scanning #2 关闭；`SECURITY.md` 与 Dependabot 配置上线。

## 4. 发布与交接约束

- 不因开发依赖告警就判定用户版必须紧急撤回。
- 若不可信 ZIP 可进入持有发布凭据的解压任务，应暂停该任务直到隔离或修复完成。
- 修复必须进入 `main` 及仍受支持的分支；停留在独立分支不得宣告告警处理完成。
- 告警关闭以默认分支重扫结果为准，不以本地命令成功为准。
- 正式交接按 [`git-platform-handoff.md`](git-platform-handoff.md) 记录分支、源提交、交接提交、未提交状态与 Owner。
