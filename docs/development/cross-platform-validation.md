# 跨平台开发与验证流程（导航）

Status: `ACTIVE` — 本文件是流程导航与端到端顺序说明，**不定义状态语义、验证政策或所有权规则**。

## 本文件不再承担的内容

本文件此前独立定义了一套验证状态模型（含 `IMPLEMENTED`、`LINUX_VERIFIED`、`WINDOWS_VERIFICATION_PENDING` 等），并重复了所有权边界、验证范围、失败分类与文档规则。这些内容现已在各自的权威文档中定义，**本文件的定义已作废**，以权威文档为准。

## 权威来源

| 主题 | 唯一权威 |
|---|---|
| 所有权边界、升级标记、交接状态 | [`platform-ownership.md`](platform-ownership.md) |
| Git 交接流程、revision、scratch 边界 | [`git-platform-handoff.md`](git-platform-handoff.md) |
| **状态语义、证据字段、能力检测、工具失败分类** | [`../validation/validation-policy.md`](../validation/validation-policy.md) |
| 验证范围选择（Targeted → Module → Subsystem → Full） | [`testing.md`](testing.md) |
| 当前待验证项 | [`../validation/windows-queue.md`](../validation/windows-queue.md) |
| 当前可执行项与手工步骤 | [`../validation/windows-manual-steps.md`](../validation/windows-manual-steps.md) |
| Windows 执行规范 | [`../validation/windows.md`](../validation/windows.md) |
| 当前批次与 Owner | [`../status/platform-handoff.md`](../status/platform-handoff.md) |
| 日常 Prompt 模板 | [`platform-handoff-prompts.md`](platform-handoff-prompts.md) |

## 端到端顺序（唯一保留的实质内容）

以下是流程顺序，不重复各步骤的判定规则：

1. **批次开始**：读取 `AGENTS.md`、当前 Plan、`platform-handoff.md`；确认 branch、revision、工作区状态与当前 Owner。
2. **能力检测**：确认当前环境实际可做什么（见 validation-policy 的能力检测章节）。Linux 会话不得声称完成 Windows 验证。
3. **实现**：在当前 Owner 边界内完成工作。共享契约变更标记 `CROSS_PLATFORM_CHANGE_REQUIRED`。
4. **最小必要验证**：按 `testing.md` 选择范围。跳过 Full regression 时必须记录原因。
5. **记录**：实现结果、验证结果、证据、延期项分别记录，四者不得合并（见 validation-policy 的状态维度）。
6. **交接**：Git commit + push，更新 `platform-handoff.md` 中的当前 Owner 与下一 Owner（状态名称与字段见 `platform-ownership.md`）。禁止直接覆盖对方正式工作树。
7. **Windows 执行**：见 `../validation/windows.md`。GUI 自动化不可用时按 validation-policy 的能力策略处理，不得记为 PASS。
8. **Reconcile**：当前 Owner 的接收方核对对方结果，区分产品缺陷与环境/工具缺陷（见 validation-policy 的工具失败分类）。
9. **收口**：更新 handoff；无跨平台遗留时本批次结束。

## 历史正文处置

本文件早期附有一整份流程正文，后由上方权威链接取代。旧正文含作废的 Owner、写入路由和重复政策，并存在批量 Base64 污染；旧正文已移除，避免被误作当前操作指令。流程演进及逐批次证据见对应 Plan、handoff 与验证历史；本页只保留当前流程顺序与权威入口。

## Related Documentation

- Agent 执行规则：[`../../AGENTS.md`](../../AGENTS.md)
- 所有权、Git handoff 与验证政策：见上方权威来源表。
