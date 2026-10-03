# 文档治理与 Agent 工具接入升级计划

Owner: Linux Cross-platform Owner。
Status: `IN_PROGRESS` — 原 Batch 0–7 于 2026-10-01 收口；追加的文档一致性 follow-up 于 2026-10-03 开始执行。本文件是计划文档，不是验证证据，也不是发布许可。

## 1. Objective

把当前分散、重叠且部分失真的文档收敛为“**一项事实或规则只有一个权威维护点**”的结构，同时升级 Cline / Codex 指令体系，使后续开发计划可执行、验证结果可复核。

本计划解决两个问题：

1. 文档职责不清、正文重复、当前状态与历史证据混杂；
2. Agent 规则把工具的运行时行为当成项目固定属性，导致工具一升级就要改仓库规则。

明确**不做**的事：

- 不实现代理 Batch B；
- 不修复 Windows 产品问题；
- 不执行“功能尚未实现”项目的实机测试；
- 不创建 tag、发布分支或 GitHub Pre-Release；
- 不自动同步 `main`；
- 不修改个人工具配置（`~/.codex/config.toml` 等），不安装或升级任何 CLI；
- 不变更根 `LICENSE`，不作出法律合规保证。

## 2. 基线与已确认问题

原 Batch 0 基线：`dev` 分支，HEAD `f4fef01`，工作区干净，本地 `origin/main` 落后 `origin/dev` 14 个提交；当时 Git 跟踪 Markdown 共 86 份。以上仅为原批次历史测量值，不是当前分支/文件数断言。Follow-up 基线另见 §2.1。

原 Batch 0 的 86 份文档分类为历史审计快照；不要将其作为当前清单。

本轮只读检查确认的问题：

| 问题 | 证据位置 | 处置批次 |
|---|---|---|

### 2.1 追加 follow-up 基线（2026-10-03）

本 follow-up 在原 Batch 0–7 完成后追加，不回写或撤销其历史验收结论。执行前核对：分支 `cross-platform/automatic-pairing-reconcile-20261002`，HEAD `a134f3cb1a9af2ea54fe1362d42fc38ee50709d0`，工作区干净，95 份 Git 跟踪 Markdown；结构审计 `node scripts/docs-audit.mjs` 通过。该 PASS 仅覆盖脚本明示的链接、索引、分支引用、skills frontmatter 与 release-note 索引检查，不证明语义状态一致、外链有效或代码与文档一致。

| ID | 确认的问题 | 权威处置位置 |
|---|---|---|
| DOC-FU-01 | `cross-platform-validation.md` 将完整历史正文标为过期，但仍有现行流程误指向 §16；历史正文含过期写入路由，且存在 Base64 编码污染 | Batch 8 第 4 项；导航页只保留当前有效入口，清除废弃正文 |
| DOC-FU-02 | 自动配对矩阵把 Windows pipe 接线标为未实现，但当前 Git 已含集成实现；Windows 验收仍待做 | `status.md` 能力矩阵；实现状态与验收状态分列 |
| DOC-FU-03 | Telegram 当前运行流把未接线 helper 描述为生产自动流程；状态分散在 status、Plan、handoff、queue 和 manual steps | `status.md` 为当前能力权威；Plan 记需求/阶段，handoff 记本批次与 source revision，queue 记验收状态，manual steps 仅记操作程序 |
| DOC-FU-04 | 当前状态页导语将历史与矩阵位置说反，含过期分支和固定测试计数 | `status.md` 只保留当前能力摘要并链接唯一状态源；逐轮记录保留为历史 |
| DOC-FU-05 | 测试指南绑定 Cline/Codex 名称并重述所有权 | `testing.md` 改用 Owner 名称，所有权与状态政策链接权威文档 |
| DOC-FU-06 | Telegram outbox 唯一键未在 Plan 解释 bot 身份如何进入逻辑幂等键；TG-06 剩余工作与“非 Windows 完成”口径不清 | Telegram Plan 明确 schema 与 key 生成契约、待办及 Owner；发现实现偏差另行立项，不以文档掩盖 |
| DOC-FU-07 | handoff 的源码 revision、验证记录 revision、文档 reconciliation revision 易混淆 | 当前 handoff 使用有语义的独立字段；历史 SHA 原样保留 |
| DOC-FU-08 | Windows validation history 连续重复 H1；历史 handoff 快照重复使用当前页 H1；queue 有重复通用标题 | 删除明确重复主标题；历史快照与同文件重复章节用唯一批次标题；queue 同一 Linux batch 内一个待办表与一个操作程序标题合并为单一编号入口 |
| DOC-FU-09 | 大量历史报告中的通用标题重复被简单重复标题扫描标记 | 仅处理连续重复/歧义性文档主标题；不同父章节、不同 run 的模板标题和历史证据允许重复 |

### 2.2 Follow-up 验收范围

1. 明确当前实现、目标设计、验证结果、发布状态的权威来源，按职责收敛 status、architecture、Plan、handoff、queue 与 manual steps；不得把 Linux/旧 artifact 的结果提升为集成源码的 Windows PASS。
2. 把遗留的 §16 有效流程从已作废正文中分离；删除确认的 Base64 正文污染，保留历史中仍有价值且不冲突的记录，并移除废弃操作入口。
3. 对 Telegram 状态逐项核对代码与 schema：区分共享 helper、生产接线、Windows 专属实现和验收；在 Plan 说明唯一约束及幂等键 bot 隔离语义。
4. 修复重复/歧义标题，检查受影响锚点；不得仅因标题相同而删除多轮验证正文。
5. 文档审计结果须注明覆盖边界。自动语义检查先以 warning 呈现，必须有合法历史重复的正反例，不自动改写产品状态。
6. 最终验证包含 targeted 文档审计/测试、链接与锚点、状态/Owner/revision/queue 对账、`git diff --check` 和人工 final diff review；不运行无关产品套件或 Windows 验收。
## 3. 目标权威职责

“无重复”不等于字面零重复。导航摘要、必要安全提醒和历史原始记录允许重复；禁止的是**多份文档独立定义同一规则**、**手工维护同一当前状态**、**在计划/README/handoff 复制测试流水账**、**复制外部内容却无来源与使用性质**。

| 类别 | 唯一职责 | 不再承载 |
|---|---|---|
| 根 `AGENTS.md` | 不变量、责任边界摘要、任务路由、完成条件 | 具体命令、当前版本、历史测试结果 |
| `docs/AGENTS.md` | 文档局部维护规则、来源与状态约束 | 重复根规则、旧历史入口 |
| `docs/README.md` | 文档入口与阅读路径 | 发布或测试流水账 |
| `docs/architecture/` | 当前结构、协议边界、数据模型、ADR | 每轮验证结果；未注明的未来实现 |
| `docs/development/status.md` | 当前实现能力与限制 | 历史测试计数 |
| `docs/development/roadmap.md` | 未来优先级、依赖、完成标准 | 已完成事项的详细日志 |
| `docs/development/platform-ownership.md` | Owner 权限、职责、问题路由 | Git 命令与验证步骤 |
| `docs/development/git-platform-handoff.md` | 正式 Git 交接、revision、scratch 边界 | 验证政策的第二份定义 |
| `docs/development/testing.md` | 测试入口、命令、fixture、模块映射 | 通用状态与风险政策的重复定义 |
| `docs/validation/validation-policy.md` | 验证范围、能力检测、结果语义、证据复用 | Windows 安装教程与历史结果 |
| `docs/validation/windows.md` | Windows 执行配方与环境排障 | 当前队列状态、另一套 Owner 规则 |
| `docs/validation/windows-queue.md` | 唯一当前待办与延期事项 | 已关闭条目的完整历史 |
| `docs/validation/windows-validation-history.md` | Windows 原始执行与 reconciliation 证据 | 当前任务指令源 |
| `docs/status/platform-handoff.md` | 当前批次、Owner、输入与下一步 | 多批次历史日志 |
| `docs/release/` | 政策、执行模板、版本索引、逐版本事实 | 开发通用规则 |
| `docs/references/` | 外部来源、许可证与使用性质登记 | 不记录本仓库自身实现事实 |
| 根 `THIRD_PARTY_NOTICES.md` | 第三方使用与分发索引 | 把计划清单当作实际发布清单 |
| `aidlc-docs/` | 初始设计历史快照 | 当前开发状态 |
| `.agents/skills/` | 可重复任务的操作入口 | 第二份独立政策 |

## 4. 工作批次

### Batch 0 — 冻结审计基线（Cross-platform Owner）

建立全量文档清单：职责、读者、Owner、文档类别、权威事实源、上下游引用、处置结论（保留/合并/拆分/归档/删除）、机器消费者、来源与许可证状态。

建立重复与矛盾问题表，附文件行号与证据。按 ID、revision、scope 对账 Windows 队列，重新计算条目数；**不继承此前“29/25 项”的统计**。

### Batch 1 — 先统一治理规则（Cross-platform Owner）

收敛根 `AGENTS.md` 为不变量与路由；完整责任定义保留在 ownership 文档。新增 capability-first 原则（OS、执行环境、shell/sandbox/权限、GUI 与自动化、MCP、网络、外部服务、目标平台与 artifact）。

明确 **能力不改变 Ownership**：Linux 通过获准渠道取得 Windows 证据不等于接管 Windows 实现；Windows 具备共享代码编辑能力不等于获得共享契约修改权。

统一工具失败分类：工具、权限、环境、账号、网络、产品缺陷分开；不得仅为适配工具故障修改生产逻辑；替代验证路径必须证明相同验收目标。

### Batch 2 — 验证模型、队列与 handoff 收口（Cross-platform Owner，Windows Owner 审核平台配方）

将三个维度分离：**实现状态**（`PLANNED`/`IN_PROGRESS`/`IMPLEMENTED`）、**验证结果**（`PASS`/`FAIL`/`BLOCKED`/`NOT_RUN`/`NOT_APPLICABLE`）、**交接与待办状态**（`READY_FOR_WINDOWS` 等）。

每项验证至少记录：ID、目标、优先级、Owner、实现状态、执行结果、延期或阻塞原因、source SHA、构建来源、artifact SHA-256、平台、环境、工具版本、执行方法、前置条件、步骤、预期结果、命令与退出码、证据、是否阻塞开发、是否阻塞发布、恢复条件、后续动作、证据复用条件。

`Method` 与 `Environment` 分开；委派表示执行者关系，不替代验证方法。不新增顶层 `INTERMITTENT PASS` 状态，改用 `status` + `stability`（stable/flaky/unknown）+ `attempts`，失败后重跑成功须同时记录两次事实。

队列按类别整理：当前 handoff、已发布版本遗留验收、已实现但延期、未实现功能、已关闭或被替代。handoff 只保留当前批次，历史通过 ID 与链接引用。区分实现 revision、文档 revision、候选 source、实际验证 revision。

### Batch 3 — 实现状态、架构与开发计划对账（Cross-platform Owner）

对每项当前能力追踪代码入口、配置/Schema、测试、平台实现、实际验证与已发布范围。优先核对 Sidecar v2、extraction-only → aria2-only、取消与 shutdown、WebSocket 与 Native Messaging、Core/Full 打包、配置持久化、代理 Batch A/B、日志渠道与图标、Telegram 与 Windows 安全边界。

`status.md` 改为当前能力矩阵；`roadmap.md` 只保留未完成方向。每个活跃计划统一结构：目标与不做事项、基线与确认原因、Owner 与跨平台依赖、编号工作项与顺序、输入输出与受影响文件、执行步骤与风险、验收用例与证据、当前进度与下一动作、延期项及恢复条件。

### Batch 4 — 去重、历史归档与发布文档对账（Cross-platform Owner）

`cross-platform-validation.md` 缩为流程导航；`testing-strategy.md`、`non-windows-completion.md` 与旧 Windows 文档的唯一证据迁入历史后，更新全部引用再移除或改为短索引。ADR 保留历史决定并标注 `superseded by`。
## 5. 关于已跳过的代理验证项

尊重 2026-10-01 的 Owner 决定：`WQ-PROXY-020-01` 至 `WQ-PROXY-020-04` 本轮不执行实机测试。该决定保留为历史记录。

Batch 2 只规范记录语义，不撤销该决定：实现状态记 `PLANNED`，执行状态记 `NOT_RUN`，原因 `IMPLEMENTATION_NOT_READY`，适用范围为当前 Batch A 不包含 Batch B，恢复条件为 Batch B 交付后重新排入。原 `NOT_APPLICABLE` 记法作为历史语义归一的说明保留。

依据：`desktop/src-tauri/src/commands.rs` 的 `system_proxy_supported()` 当前无条件返回 `false`；`desktop/src-tauri/src/proxy.rs` 的 `platform_resolver()` 在所有平台返回 `EnvironmentProxyResolver`，其文档注释说明只读取进程环境变量，不提供 registry、PAC 或 WPAD 的按 URL 解析。

## 6. 外部工具更新对本计划的影响

截至 2026-10-01 的核查结论记录在 `docs/references/external-sources.md`。要点：Cline 与 Codex 各自有独立的扩展/CLI/桌面发布线，版本不可混用；Codex CLI 近期集中修复 Windows 子进程、沙箱回退与中断/审批生命周期；两者仍在演进。

因此本计划采纳“结构性小升级”而非按小版本重写规则：`AGENTS.md` 固化谁负责什么、什么算完成、失败如何分类；validation policy 固化如何证明完成；handoff 固化上一 Owner 交给下一 Owner 什么；日常 Prompt 只表达本轮目标。

## 7. 执行顺序与提交边界

```text
Plan 落库
  → Batch 0 清单与问题表
  → Batch 1 治理规则
  → Batch 2 状态/队列/handoff
  → Batch 3 架构与开发计划对账
  → Batch 4 去重与历史归档
  → Batch 5 来源/许可证
  → Batch 6 Agent 接入
  → Batch 7 静态验收与交付
```

每批独立提交。禁止把文档治理、依赖整改、产品修复和发布混入同一变更。

## 8. 验收标准

完成时必须能回答：

1. 每份文档由谁维护、回答什么问题；
2. 每个当前状态的唯一入口在哪里；
3. 哪些功能已实现、哪些只通过局部测试、哪些没有平台验收；
4. 下一任务的输入、步骤、Owner 与验收条件；
5. 每项 PASS 对应哪个 revision 与 artifact；
6. 每段外部借鉴来自哪里、以何种方式使用、许可证是否核实；
7. Cline 与 Codex 实际加载了哪些规则、哪些能力仍未验证；
8. 文档整理是否完整保留历史失败与延期事实。

最终交付报告必须包含：基线与最终 revision、文档处置清单、权威职责表、已修复矛盾、当前真实开发状态、重算后的待办、外部来源与许可证未决项、实际执行的检查、未运行的工具与 Windows 验证、下一 Owner。

## 9. Batch 0 审计基线

生成工具：`scripts/docs-audit.mjs`（只读；`--json` 输出机器可读结果）。基线 commit `5da4674`，已跟踪 Markdown 87 份。

分类分布：release-note 33、development 21、architecture 6、design-snapshot 5、validation 5、instruction 4、root 4、release-governance 3、docs-index 2、audit 2、release-migration 1、handoff-status 1。

### 审计发现（全部为结构信号，非产品结论）

| 检查 | 结果 | 说明 |
|---|---|---|
| 失效相对链接 | 0 | Markdown 相对链接与裸反引号路径均可解析 |
| 无入口文档 | 7 | 见下方清单 |
| 过期分支链接 | 40 处，涉及 10 个文件 | 全部为 `feature/u7-desktop-production-integration` |
| skills 缺 frontmatter | 3 | `.agents/skills/` 下全部 SKILL.md |
| notes 未被索引引用 | 1 | `docs/release/notes/v0.2.0-pre.5.md` |

无入口文档：

- `.agents/skills/{cross-platform-handoff,project-code-audit,windows-validation}/SKILL.md`
- `aidlc-docs/inception/README.md`
- `docs/architecture/job-state-machine.md`
- `docs/development/mvp-scope.md`
- `docs/review/code-audit-guidelines.md`

过期分支链接所在文件：`docs/development/` 下 6 份（含 `windows-validation.md`）、`docs/release/notes/` 下 7 份、`docs/status/platform-handoff.md`、`docs/validation/windows-queue.md`、`docs/validation/windows-validation-history.md`、`docs/validation/windows-wdio-handoff.md`、`docs/review/engineering-audit-2026-09-26.md`。

### 审计工具自身缺陷记录

首版脚本因正则锚定行首、捕获组不含斜杠、插入位置错位等原因，全部检查误报 OK。经独立命令交叉核验（`v0.2.0-pre.5.md` 确实不在索引、三份 SKILL.md 确实无 frontmatter）后逐项修正，并改用括号深度分析定位缺失闭合。修正后各项结果与手工核验一致。

教训：审计工具的“全绿”必须与独立手段交叉验证后才能采信；本次若直接采信首版结果，会漏掉全部 51 个真实发现。

## 10. Progress

| 批次 | 状态 | 提交 |
|---|---|---|
| Plan 落库 | COMPLETE | `5da4674` |
| Batch 0 | COMPLETE | `8bee3da`, `4296b3e` |
| Batch 1 | COMPLETE | `d738ca4` |
| Batch 2 | COMPLETE | `c1a9d70`, `f0752c5` |
| Batch 3 | COMPLETE | `0e3ac4d` |
| Batch 4 | COMPLETE | `4296b3e`, `19819e8`, `348510d`, `a7f4012` |
| Batch 5 | COMPLETE | `2e7d490` |
| Batch 6 | COMPLETE | `38d022e` |
| Batch 7 | COMPLETE | 审计 12/12、`docs audit: PASS`、`git diff --check` |

### Batch 8 — 2026-10-02 文档语义与标题一致性 follow-up

**Owner：Cross-platform Owner。**不改产品代码，不重写 Release Notes，不删除历史验证证据，不执行 Windows 实机验收。

执行顺序：

1. 更新唯一当前状态矩阵中的自动配对实现/验收边界；检查当前 handoff 的 source、validation-record、documentation revision、Owner、下一动作和工作区状态字段。
2. 修复 runtime-flow 的 Telegram 现状与目标流程分界；修正 Job 状态机主图/幂等说明；整理 status 的当前矩阵与历史流水账边界。
3. 校正测试指南的 Owner 表述；将 Telegram Plan 与 handoff、queue、manual steps 的分工写清，核实 idempotency key 与数据库唯一约束，不推断未被代码证明的 bot 隔离行为。
4. 抽出 `cross-platform-validation.md` 唯一现行流程入口，移除正文污染与旧写入指令；历史资料如保留必须带不可操作的归档标记。
5. 修正 `windows-validation-history.md` 连续重复 H1；为 handoff 历史快照设置唯一标题；审查 queue 两个相同章节标题的上下文和当前/历史职责。不同 ADR、Plan 子章节和逐轮报告中的通用模板标题保留。
6. 在 docs audit 入口说明覆盖边界；评估并实现轻量标题/污染检查，必须区分硬错误与历史重复 warning，测试正常重复与异常重复样例。
7. 运行脚本针对性测试、全量 Markdown 链接/锚点检查、审计、状态与队列引用核对、whitespace 检查并人工复核最终 diff。任何范围超出本批次的代码契约问题记录为 follow-up，不在文档修改中静默解决。

**验收：**当前状态没有与代码/当前 handoff 冲突；历史验证按原 source/artifact 保留；Plan/handoff/queue/manual steps 各有唯一职责；明确重复 H1 已修复；合法历史重复无证据丢失；审计输出不会将历史模板重复误报为文档错误；所有文档检查结果和未执行项如实记录。

## 11. Follow-up 进度（2026-10-03）

| ID | 状态 | 说明 |
|---|---|---|
| Plan 更新 | COMPLETE | 本节及 §2.1–2.2 已先登记 follow-up，再开始实施 |
| DOC-FU-01（流程正文清理）、02、03、05、06、08（部分） | COMPLETE | 已更新导航/状态/架构/测试/Telegram Plan；发现 outbox 跨 bot 幂等契约缺口，登记为待设计的跨平台契约事项 |
| DOC-FU-04、07、08（部分）、09 | IN_PROGRESS | 当前 handoff 的 source/documentation revision、Owner 与未提交状态已区分；仍需人工核对所有历史快照边界、queue 多轮章节及受影响锚点 |
| DOC-FU-01 Base64 与旧操作入口 | COMPLETE | 导航页只保留当前流程入口，确认的 Base64 历史正文与旧操作入口已移除；README、Windows 规范和 queue 不再指向旧章节 |
| DOC-FU-10 审计边界与标题正反例检查 | COMPLETE | docs audit 已补 warning 级连续重复 H1、多 H1 与超长非列表正文检查（16/16 测试通过）；历史模板合法重复保留，不计入失败 |
| 验证与最终 diff | PENDING | 本轮工作仍有未解决的标题/归档语义与审计边界项；记录为进度检查点，不作为完成 handoff |

### 收口记录

- **Batch 2 handoff 拆分：** `platform-handoff.md` 由 871 行缩为 56 行，历史原文迁至 `platform-handoff-history.md`（846 行）。拆分前核对了 56 个唯一 run-ID/SHA，拆分后 56 个全部保留、零丢失。
- **Batch 4 旧文件归档：** `testing-strategy.md`、`non-windows-completion.md` 加归档声明后按原文保留；`cross-platform-validation.md` 收敛为导航页并作废其独立状态模型。
- **`windows-validation.md` 未迁移（有意）：** 该文件含 66 个唯一 run-ID/SHA，仅 7 个存在于 `windows-validation-history.md`。迁移会丢失 59 份证据且破坏 3 份 release notes 的引用，因此原地归档而非迁移，测量结果已写入文件本身。
- **Windows 验证项汇总：** [`../validation/windows-manual-steps.md`](../validation/windows-manual-steps.md)，含 BLOCKED/NOT RUN 项的手工步骤。

### 本轮验证范围

原 Batch 7 的 Targeted 验证记录：审计脚本 12 项单元测试、`docs audit`、`git diff --check`。本 follow-up 的最终验证状态以 §11 表格和本轮记录为准。未运行 Rust/Python/桌面功能测试（本轮未改动产品代码），未执行任何 Windows 实机测试。

33 份 notes 逐一核对文件、tag、source、Release object、迁移映射与 artifact，补齐 `v0.2.0-pre.5` 索引（发布状态以正式对账为准）。搜索 `docs/releases/` 的 Markdown 链接、裸反引号路径、workflow、测试、脚本与 skills，区分合法历史提及与失效路径，不做盲目全局替换。

### Batch 5 — 外部来源与许可证治理（Cross-platform Owner，发布由 Release/Windows Owner 复核）

新建 `docs/references/external-sources.md`，登记上游名称与维护者、URL、tag/commit/文档版本、核实日期、本地使用位置、使用性质（参考/短引用/复制/改编/补丁/捆绑）、许可证文件与状态、本地修改与归属、待处理义务。

重点审核 Cline 与 Codex 文档和技能、`hureyqi/x-spider-mod-2026` 参考、WDIO patch/adaptor、gallery-dl、aria2、PyInstaller worker 与 `_internal` 依赖、图标与截图。`THIRD_PARTY_NOTICES.md` 区分开发使用、计划使用、实际分发清单。未找到明确授权的内容不默认开源授权，标记 `LICENSE_UNVERIFIED`，必要时改为独立说明或链接引用。

### Batch 6 — Cline/Codex 接入与日常 Prompt 升级（Linux 负责共享文档，Windows Owner 验证 Windows 入口）

新增 `docs/development/agent-tooling.md`，只记录实际产品入口与版本记录方法、指令发现差异、skills 接入、能力检测、配置位置与安全限制、更新核查周期。根规则不绑定模型、CLI 版本和不稳定命令。

为三份 skills 补 YAML 元数据并核实发现路径；若 Cline 确需 `.cline/skills/`，使用薄入口或确定性生成，不复制两份人工维护正文，Windows 兼容性未确认前不依赖 symlink。

日常 Prompt 缩短为：本轮目标、当前 Plan、当前 Owner、读取权威规则、reconcile、完成当前范围、报告 completed/validation/unresolved/next owner。

恢复或中断后强制重新核查 Git、未完成进程、命令退出码、artifact、当前模式与 Owner。

### Batch 7 — 静态检查、差异审查与正式交付（Cross-platform Owner）

本任务以文档与指令变更为主，**不默认重跑 Rust/Python/桌面全部功能测试，也不安排未完成项目的 Windows 实机测试**。

执行 `git diff --check`、Markdown 相对链接与锚点检查、旧路径与裸路径检查、文档清单覆盖率、状态/Owner/revision 一致性、队列 ID 与历史引用完整性、33 份 notes 索引对账、来源与许可证登记检查、skills 元数据与适配检查、final diff 人工审查。

若修改机器消费路径、脚本或 workflow，增加对应 targeted 测试；若新增检查脚本，需覆盖其正反例。CI 接入作为独立提交，Windows workflow 变化由 Windows Owner 审核。
| 当前 handoff 混入多批次历史 | `docs/status/platform-handoff.md`（871 行） | Batch 2 |
| 当前队列混入历史快照，条目数不可直接采信 | `docs/validation/windows-queue.md`（2638 行） | Batch 0 / 2 |
| 同一状态前后矛盾 | 队列开头称 CodeQL #2 已关闭，后文旧条目仍称 open；G3 发布后说明与旧表格不一致 | Batch 2 |
| 验证规则多处重复定义 | `docs/development/testing.md`、`docs/development/cross-platform-validation.md`、`docs/validation/windows.md`、根 `AGENTS.md`、`.agents/skills/` | Batch 1 / 4 |
| Agent 路由指向旧历史入口 | `docs/AGENTS.md` 仍允许写入 `development/windows-validation.md` | Batch 2 / 4 |
| 计划文档状态与完成事实不符 | `docs/development/release-documentation-refactor-plan.md` 仍为 `APPROVED` 且章节顺序错位 | Batch 4 |
| 当前状态文档含逐轮流水账 | `docs/development/status.md` 大量测试计数与历史整改 | Batch 3 |
| 发布索引可能漏项 | 33 份 notes 中 `v0.2.0-pre.5.md` 未见对应 history 行 | Batch 4 |
| 未实现功能的记录语义不统一 | `docs/AGENTS.md` 对未实现项目要求 `PLANNED`/`NOT RUN`，而跳过项记为 `NOT_APPLICABLE` | Batch 2 |
| skills 缺少 YAML 元数据 | `.agents/skills/*/SKILL.md` 为纯 Markdown | Batch 6 |

## 13. 本轮专项收口批次（APPROVED，先更新治理文档再执行）

### 阶段 A：治理计划自检

- 整理本文件章节与状态表：历史 Batch 0–7 的已完成记录保留原样并注明日期；Follow-up 待办单独成表；不再把新增专项误记为原批次的一部分。
- 明确语义重复的判定边界：当前规则只保留一个权威定义；历史事实、必要引用、模板标题和合法重复保留并标注用途。
- 输出删除/归档候选的对账条件：独有内容、入站引用、迁移目标、保留或删除理由齐备后才可执行。

### 阶段 B：去重与语义澄清

1. 精简当前 `platform-handoff.md`：同一字段只出现一次；同一 revision 只在一个语义字段下声明；测试源 revision 与文档提交 revision 分开记录。
2. 收敛 queue：当前待办保留在前；确认属于历史批次的整段命令结果与修复过程，按 revision 归入历史文档；不合并不同 artifact 的 PASS/FAIL。
3. WDIO 当前可复用配方集中到 `docs/validation/windows.md`；`testing.md` 只保留范围、入口与权威引用；旧 WDIO handoff 按原样归档，迁移后再评估是否删除。
4. Telegram 当前能力与目标运行流分别陈述；outbox 唯一键与 bot 身份的逻辑幂等关系在 Plan 中明确。
5. 修改模糊表述时保留原证据含义，不提前关闭任何 FAIL/BLOCKED/NOT_RUN。

### 阶段 C：过时文件处置

- `windows-wdio-handoff.md`：配方与证据迁移完成后评估删除。
- `testing-strategy.md`、`non-windows-completion.md`：已归档，按原文保留；迁移必要边界与证据后再评估删除。
- `windows-validation.md`：暂不删除，独有历史证据保留。
- 已完成 Plan：标注完成范围与后继入口，不默认删除。
- Release Notes、验证历史、迁移记录：保留，不以“过时”为由删除。

### 阶段 D：链接与标题审计补强

- 现有结构审计保留离线稳定性；新增检查先输出 warning，不自动改写文档。
- 补充：本地锚点、引用式链接、非 Markdown 资源、连续重复标题、多 H1 与异常编码长行；历史模板合法重复提供正反例。
- 外部 URL 另行联网核查，输出“有效／确认失效／无法确认”；网络波动不计为普通文档审计失败。

### 阶段 E：验证与提交

- 审计脚本针对性测试；
- 全量本地链接、锚点和资源检查；
- 外部链接检查结果清单；
- 历史证据迁移完整性核对；
- `git diff --check`；
- 最终 diff 人工复核。

| 内部链接钉在过期分支 | 多个文档引用 `.../blob/feature/u7-desktop-production-integration/...` | Batch 4 |