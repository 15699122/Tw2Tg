# Documentation Agent Instructions

本目录的文档维护必须遵循仓库根目录 [`../AGENTS.md`](../AGENTS.md) 的完整规则；本文件只补充文档目录的局部约束。

- `docs/architecture/` 只记录稳定边界、运行流、ADR、数据模型和文件职责，不写逐轮验证流水账。
- `docs/development/status.md` 记录当前实现事实；`roadmap.md` 记录未来方向；历史 Windows 结果写入 `development/windows-validation.md` 或相应验证报告。
- `docs/validation/windows-queue.md` 是当前 Windows 队列的事实源；未实现功能必须使用 `PLANNED`/`NOT RUN`，不能写成 `WINDOWS_PASS`。
- 修改协议、入口、配置、migration 或模块职责时，必须同步检查 repository map、setup、testing 和相关验证范围。
- 新增人工维护文档时，必须在 `docs/architecture/repository-map.md` 登记职责、入口、运行关系、维护约束和测试位置。