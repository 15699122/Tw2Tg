# 模块化设计政策

Owner: Cross-platform Owner。状态：ACTIVE（2026-11-08，Phase R0）。

本文件定义模块拆分的判定标准。它是 [`../development/internal-v1-completion-and-refactor-plan.md`](../development/internal-v1-completion-and-refactor-plan.md)
第 6 节 R0–R7 的执行依据，不替代平台所有权、验证政策或 ADR-015。

## 1. 什么算一次有效拆分

一次拆分被接受，必须同时满足：

- 业务职责和依赖方向更清楚，而不是文件行数更小；
- 公共契约显式，或明确记录为"故意不变"；
- 既有行为有回归测试覆盖；
- 不引入环、不产生通用垃圾桶模块、不产生空壳包装；
- 不通过把代码搬进测试或删注释来降低行数。

行数是**评审信号**，不是拆分算法。内聚的大文件可以保持原状，但必须有一条
`scripts/architecture-exceptions.json` 中的具名例外，写明理由和计划阶段。

## 2. 强制边界

- 保持现有 crate 边界，除非依赖证据支持一次明确的边界变更。
- 迁移一个职责时，保持 ADR-015 的顺序：admission → scopes → claim → execute
  → drain → authorize → finalize → terminal persistence → release。
- 改动资源所有权、恢复、发布或执行器生命周期时，必须重跑对应的 V1 回归矩阵。
- 不得以重构为名引入通用事件总线、依赖注入框架、投机抽象或 V2 注册。

## 3. 自动化检查

`node scripts/check-architecture.mjs` 报告四类信号：

| 信号 | 含义 |
|---|---|
| `over-limit-without-exception` | 文件超过其类别行数上限且没有具名例外 |
| `baseline-delta` | 文件行数与已评审基线不一致 |
| `dependency-boundary` | 相对导入跨越了 `architecture-policy.json` 声明的禁止边界 |
| `dependency-cycle` | JavaScript/TypeScript 源文件之间存在相对导入环 |
| `baseline-renamed` | 基线文件被同名内容移动，需显式更新基线 |
| `unresolved-relative-import` | 相对导入无法解析到受跟踪源文件 |

配套规则：

- 基线是**已评审快照**，只允许通过一次显式提交更新；禁止 CI 或脚本自动重写。
- 例外必须写明理由（≥20 字符）和计划阶段；空泛理由会被判为
  `exception-without-reason`。
- 策略文件缺失时检查器以退出码 2 失败关闭，不会静默通过。
- 策略、例外、基线本身的单元测试在 `scripts/check-architecture.test.mjs`。

## 4. 新模块必须说明什么

新增或拆分出的模块必须有一份说明，包含本文件第 5 节模板中的字段。说明不是
验证记录：实现状态、验证结果和所有权状态仍然分开记录。

## 5. 评审问题清单

1. 这个模块唯一的业务职责是什么？
2. 它依赖谁、谁依赖它？是否形成新环？
3. 它的公共契约是什么，调用方是否被迫感知内部结构？
4. 拆出去之后，原文件的入口和兼容导出是什么？
5. 哪些既有测试覆盖了它？哪些 gate 需要重跑？
6. 如果这部分无法在不改变语义的前提下拆出，记录理由而不是硬拆。

## 6. 模板

见 [`module-template.md`](module-template.md)。
