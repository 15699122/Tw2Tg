# Documentation Agent Instructions

本目录的文档维护必须遵循仓库根目录 [`../AGENTS.md`](../AGENTS.md) 的完整规则；本文件只补充文档目录的局部约束。

- `docs/architecture/` 只记录稳定边界、运行流、ADR、数据模型和文件职责，不写逐轮验证流水账。
- `docs/development/status.md` 记录当前实现事实；`roadmap.md` 记录未来方向；历史 Windows 结果写入 [`validation/windows-validation-history.md`](validation/windows-validation-history.md)（`development/windows-validation.md` 仅为兼容索引，不得作为新的写入目标）。
- `docs/validation/windows-queue.md` 是当前 Windows 队列的唯一事实源；未实现功能的实现状态必须为 `PLANNED`，验证结果用 `NOT_RUN` 并写明原因，不得写成 `WINDOWS_PASS`。
- 外部引用、改编与再分发内容必须在 [`references/external-sources.md`](references/external-sources.md) 登记来源、固定版本、使用性质与许可证状态；未核实写 `LICENSE_UNVERIFIED`。
- 状态取值、证据字段、能力检测与工具失败分类只在 [`validation/validation-policy.md`](validation/validation-policy.md) 定义一次，其他文档引用而不重复定义。
- 修改协议、入口、配置、migration 或模块职责时，必须同步检查 repository map、setup、testing 和相关验证范围。
- 新增人工维护文档时，必须在 `docs/architecture/repository-map.md` 登记职责、入口、运行关系、维护约束和测试位置。