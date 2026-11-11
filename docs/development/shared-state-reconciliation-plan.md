# 共享层状态对账与交付收口计划

Owner: Cross-platform Owner。Status: **IN_PROGRESS**（2026-11-11 立项）。

本计划接续已完成的
[`large-file-capability-refactor-plan.md`](large-file-capability-refactor-plan.md)（P0–P13，
`COMPLETE`），不重开该计划的任何批次，也不继承其 Git 授权。当前执行游标唯一来源为
[`platform-handoff.md`](../status/platform-handoff.md)。

Windows 原生实现与验证归 Windows Platform Owner，见
[`windows-queue.md`](../validation/windows-queue.md)。aria2 专属验收继续延期并保持
`NOT_RUN`。

## 1. 目标与非目标

### 目标

1. 核销工作区中先于本计划存在的 11 个源码/测试修改，确认它们只是可见性与测试构建面调整。
2. 让架构基线重新成为"已评审快照"，而不是对当前树的过期断言。
3. 把"当前实现""既定目标""测试结果""平台状态"四类事实在文档中显式分开。
4. 让新会话用最小读取量恢复游标，且不产生第二套权威来源。
5. 在明确授权下通过 Git 交付正式状态。

### 非目标

- 不重做 P0–P13 的模块拆分或测试外置。
- 不把下载链路迁移成 aria2-only。既定目标是 aria2-only，当前源码仍保留
  `use_aria2` 双路径；本计划只如实记录，不实施迁移。
- 不改 Telegram 行为、协议、schema、数据模型或任何 fail-closed 语义。
- 不新建 `memory-bank/`、`.clinerules/`、Hook、Plugin 或自动记忆写入程序。
- 不删除、压缩或改写历史验证记录；不覆盖他人 worktree。
- 不扩大 aria2 验收范围。

## 2. 授权与操作边界

用户于 2026-11-11 批准：

- 审查后更新架构基线（不扩大 exception、不修改阈值与检查器）；
- staging / commit / fetch 远端核对 / push；
- 在限定临时 scope 内执行四项 disposable delegated-cgroup 验收。

未获授权、仍需单独确认：

- 任何系统权限、cgroup delegation 配置或 `systemd` 单元持久化配置的修改；
- 真实 X / Telegram 账号副作用、真实 aria2c / gallery-dl 二进制获取；
- 安装或升级依赖、改变审批与 sandbox 设置；
- 删除备份、日志、恢复证据或用户文件；
- 历史重写、`reset`、`stash`、分支创建或切换；
- Windows 工作树改动。

发现某项需要更大权限时：停止该子项、写入游标、继续独立工作。

### Documentation Gate

业务代码修改前必须记录完成状态。本计划自身的 Gate 条目：

1. 本计划与本计划证据页存在，且状态口径一致；
2. `docs/status/platform-handoff.md` 只有一个有效游标，下一动作具体到文件与符号；
3. `docs/architecture/repository-map.md` 已登记本计划与证据页；
4. 旧 large-file Plan 顶部指向本计划，`COMPLETE` 状态不被改写。

Gate 未闭合时只允许只读调查、文档修正与只读检查脚本。

## 3. 批次与验收

依赖关系：

```text
R0（本计划 + 证据 + 游标 + repository map）
 → R1 既有源码核销（只读审查 + 结论入证据）
 → R2 基线收口（需 R1 结论）→ 架构门禁绿色
 → R3 产品事实对账（可与 R1 并行）
 → R4 记忆规则纠偏与历史隔离（可与 R1/R3 并行）
 → R5 最小必要验证（依赖 R2/R3/R4）
 → R6 Git 交付（依赖 R5）
 → R7 delegated 四项（独立，条件满足即可执行）
```

| 批次 | 工作范围 | 保持与验证重点 |
|---|---|---|
| R0 | 新计划、证据页、游标 résumé、repository map 登记、旧 Plan 后继引用 | 只改文档；docs audit、diff check |
| R1 | 审查 11 个既有源码/测试 diff：`production.rs` 及其子模块、`batch.rs`/`batch/discovery.rs`、`executor.rs`、`runtime.rs`、两个 Storage 测试 | 逐项判定可见性/测试构建面/行为；调用点与导出路径核对；不得把 `#[cfg(test)]` 变化记为业务行为变化 |
| R2 | 在 R1 结论基础上更新 `scripts/architecture-baseline.json` | 只接受已审文件；行数与 SHA-256 同时可追溯；不扩大 exception、不改阈值与检查器 |
| R3 | README、架构总览、开发状态、roadmap、用户指南的当前/目标分离 | 每项有代码依据；"存在""接线""验证通过"分开表述 |
| R4 | `AGENTS.md`、`agent-tooling.md`、`platform-handoff.md` 的记忆规则纠偏与受控历史隔离 | 单一游标；来源表只作导航；历史保真、入站链接不失效 |
| R5 | Targeted → Module 级验证与最终 diff 审查 | 真实退出码、选中数、ignored 归属；未跑项显式 `NOT_RUN` |
| R6 | 分组提交、远端核对与推送 | 显式文件清单；分叉先报告；远端确认前不宣布正式交接 |
| R7 | 四项 delegated-cgroup 验收 | 每项 exact + ignored 恰选一项；绑定二进制品身份；不获 scope 则记录 `NOT_RUN` 与前置条件 |

## 4. 验证命令与门槛

常规（每批结束时）：

```sh
node scripts/docs-audit.mjs --json
git --no-pager diff --check
```

R1/R5 源码相关：

```sh
cargo check -p xarchive-desktop --lib --tests
cargo test -p xarchive-desktop --lib -- --test-threads=1
cargo test -p xarchive-storage --lib -- --test-threads=1
```

选择器必须实际选中测试；`--list` 枚举不等于执行。风险升级顺序为
Targeted → Module → Subsystem → Full；Full 仅在恢复顺序、资源生命周期或
持久化行为被认为改变，或影响面不明时执行。

R2 门槛：

```sh
node scripts/check-architecture.mjs
node --test scripts/check-architecture.test.mjs
```

证据绑定 source revision 与 dirty identity；失败与重试全部保留。结果状态与必填字段
只以 [`validation-policy.md`](../validation/validation-policy.md) 为准，本文件不重复定义。

## 5. 完成标准

1. 11 个既有源码/测试修改都有书面核销结论，且未发现未经解释的行为变化。
2. 架构检查在收口后退出 0，基线差异可逐项追溯。
3. 产品文档中"当前实现 / 既定目标 / 验证结果 / 平台状态"可区分。
4. 当前 handoff 只有一个有效游标，恢复代价为常数级。
5. Windows 工作集中记录，未从 Linux 结果推导 Windows 验收。
6. Git 交付完成，或明确记录为未闭合项并给出具体下一步。
7. 未放宽任何断言、威胁模型或验收标准以取得 PASS。

## 6. 依据

- [统一验证政策](../validation/validation-policy.md)
- [平台所有权](platform-ownership.md)
- [Git-based 跨平台交接](git-platform-handoff.md)
- [模块化设计政策](../architecture/MODULAR_DESIGN_POLICY.md)
- [本计划证据](../validation/shared-state-reconciliation-evidence.md)
- [当前游标](../status/platform-handoff.md)
