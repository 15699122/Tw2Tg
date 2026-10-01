# 文档索引

本目录保存 XArchive 的当前架构、开发方法、协议说明和平台验证文档。

## 阅读顺序

1. 项目使用者先阅读根目录 [`README.md`](../README.md)。
2. 新开发者阅读 [`development/setup.md`](development/setup.md) 和 [`architecture/repository-map.md`](architecture/repository-map.md)。
3. 理解系统边界时阅读 [`architecture/overview.md`](architecture/overview.md)、[`architecture/runtime-flow.md`](architecture/runtime-flow.md) 和 [`architecture/data-model.md`](architecture/data-model.md)。
4. 开始功能开发前阅读 [`development/status.md`](development/status.md)、[`development/roadmap.md`](development/roadmap.md) 和 [`development/testing.md`](development/testing.md)。
5. 涉及跨进程消息时阅读 [`protocols/overview.md`](protocols/overview.md)。
6. 正式跨平台交接前阅读 [`development/git-platform-handoff.md`](development/git-platform-handoff.md) 和 [`development/platform-handoff-prompts.md`](development/platform-handoff-prompts.md)。
7. 涉及 Windows 时阅读 [`development/cross-platform-validation.md`](development/cross-platform-validation.md)（第 16 节含批次 Owner 与 Git handoff 的固定流程）、[`validation/windows.md`](validation/windows.md) 和当前 [`validation/windows-queue.md`](validation/windows-queue.md)。
8. 负责发布时按顺序阅读 [`release/release-policy.md`](release/release-policy.md)、[`release/release-checklist.md`](release/release-checklist.md) 和 [`release/release-history.md`](release/release-history.md)，再按需查 [`release/notes/`](release/notes/) 中的逐版本说明。

## 文档职责

| 目录/文档 | 权威内容 |
|---|---|
| `architecture/` | 稳定的组件边界、运行流、数据模型、ADR 和文件职责 |
| `development/status.md` | 当前实现状态、限制和外部依赖 |
| `development/roadmap.md` | 未来开发方向、依赖和完成标准 |
| `development/testing.md` | 测试层级、命令、fixture 和验证门槛 |
| `development/risk-register.md` | 当前仍有效的风险和缓解措施 |
| `development/security-remediation-plan.md` | 依赖与供应链安全告警的评估基线、分批整改计划和验收门槛（计划文档，非验证证据） |
| `development/branch-integration-release-plan.md` | 分支收敛清单、`dev`/`main` 同步顺序与 `v0.2.0` 发布门槛（历史批次计划，非当前通用发布政策） |
| `development/release-documentation-refactor-plan.md` | 发布文档重构计划：统一为 `docs/release/` 的目标结构、迁移清理规则与验证方案（计划文档，非验证证据） |
| `release/release-policy.md` | 通用发布政策：RC 准入、分层门槛、延期的受限发布授权、Owner 职责与编号/tag 规则 |
| `release/release-checklist.md` | 每次发布复用的执行模板：按阶段记录状态、证据、责任人、阻塞原因与后续动作 |
| `release/release-history.md` | 各版本存在性与发布状态的索引；区分发布、资产、WDIO 与产品验收，未核实项显式标注 |
| `release/notes/` | 逐版本发布说明；每份必须区分已验证事实、未验证范围和授权情况 |
| `release/migration/` | 已退役 `pre.N` 编号的迁移记录与机器可读冻结台账（历史证据，只读） |
| `development/desktop-ui-known-folder-fix-plan.md` | v0.2.0 修复批次计划：Dashboard 卡片等高、侧栏服务状态外观、版本行间距、Windows Downloads known folder 解析（计划文档，非验证证据） |
| `review/` | 周期性工程审查报告：发现、证据、严重性/置信度与验证状态；不作为实现事实来源 |
| `review/engineering-audit-2026-09-26.md` | 2026-09-26 只读工程审查报告；整改计划见 `development/roadmap.md` R7，风险见 RISK-014 至 RISK-022 |
| `development/cross-platform-validation.md` | Linux ↔ Windows 开发与验证流程 |
| `validation/windows.md` | Windows 验证执行规范和报告模板 |
| `validation/windows-queue.md` | 当前 Windows Validation Queue，唯一当前队列事实源 |
| `development/windows-validation.md` | 历史 Windows 验证记录和兼容入口 |
| `protocols/` | 跨进程协议、消息顺序和 Schema 关系 |

## 事实来源优先级

1. 当前代码、配置和 Schema。
2. `architecture/` 中描述稳定行为的文档。
3. `development/status.md` 和 `roadmap.md`。
4. 当前验证队列和最新验证记录。
5. `aidlc-docs/` 中的 inception 文档；这些文档是初始设计快照，不覆盖当前实现。

## 文档维护规则

- 当前状态、未来计划和历史验证必须分开记录。
- README 不保存逐轮测试结果、内部工作流或 Windows 验证流水账。
- 新增或移动人工维护文件时，必须同步更新 `architecture/repository-map.md`。
- 文档中的 PASS、完成和阻塞状态必须有对应代码或验证证据。
- 旧路径在兼容期内保留索引或迁移说明，不复制整份内容。

## Ownership and handoff

- Platform ownership and handoff: development/platform-ownership.md
- Git-based cross-platform handoff: development/git-platform-handoff.md
- Daily owner prompts: development/platform-handoff-prompts.md
- Unified validation policy: validation/validation-policy.md
- Current platform handoff: status/platform-handoff.md
- Windows validation history: validation/windows-validation-history.md
- Code audit guidelines: review/code-audit-guidelines.md
- Repeatable workflows: ../.agents/skills/
