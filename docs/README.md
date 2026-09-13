# 文档索引

本目录保存 XArchive 的当前架构、开发方法、协议说明和平台验证文档。

## 阅读顺序

1. 项目使用者先阅读根目录 [`README.md`](../README.md)。
2. 新开发者阅读 [`development/setup.md`](development/setup.md) 和 [`architecture/repository-map.md`](architecture/repository-map.md)。
3. 理解系统边界时阅读 [`architecture/overview.md`](architecture/overview.md)、[`architecture/runtime-flow.md`](architecture/runtime-flow.md) 和 [`architecture/data-model.md`](architecture/data-model.md)。
4. 开始功能开发前阅读 [`development/status.md`](development/status.md)、[`development/roadmap.md`](development/roadmap.md) 和 [`development/testing.md`](development/testing.md)。
5. 涉及跨进程消息时阅读 [`protocols/overview.md`](protocols/overview.md)。
6. 涉及 Windows 时阅读 [`development/cross-platform-validation.md`](development/cross-platform-validation.md)、[`validation/windows.md`](validation/windows.md) 和当前 [`validation/windows-queue.md`](validation/windows-queue.md)。

## 文档职责

| 目录/文档 | 权威内容 |
|---|---|
| `architecture/` | 稳定的组件边界、运行流、数据模型、ADR 和文件职责 |
| `development/status.md` | 当前实现状态、限制和外部依赖 |
| `development/roadmap.md` | 未来开发方向、依赖和完成标准 |
| `development/testing.md` | 测试层级、命令、fixture 和验证门槛 |
| `development/risk-register.md` | 当前仍有效的风险和缓解措施 |
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