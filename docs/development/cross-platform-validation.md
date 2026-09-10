# Cross-platform Development and Windows Validation Workflow

## 1. Purpose

本项目主要在 Linux 环境进行开发。

Windows 环境主要用于：

- Windows-specific build verification；
- Windows runtime verification；
- Windows packaging / installer verification；
- filesystem / path / process / sidecar 等平台相关验证；
- 项目文档定义的其他 Windows 验证。

Linux 项目目录是主要开发工作区和项目事实来源。

Windows 项目目录是验证工作副本。

## 2. Source of Truth

Linux 项目目录是以下内容的主要事实来源：

- source code；
- project configuration；
- project documentation；
- implementation state；
- Plan / task state；
- validation documentation。

Windows 工作副本主要用于验证，不作为主要开发源。

代码同步方向默认必须为：

```text
Linux → Windows
```

除验证文档结果外，不应将 Windows 工作副本中的代码修改自动反向同步到 Linux。

## 3. Development / Validation Cycle

标准工作流：

1. Linux implementation；
2. Linux verification；
3. 标记 Windows verification requirements；
4. 将 Linux 项目同步到 Windows；
5. Windows validation；
6. 将 Windows validation results 写回 Linux 文档；
7. Linux 重新读取并 reconcile Windows results；
8. 继续 implementation 或 fixes；
9. 必要时重复 Windows validation。

不得将“Linux 已修复”视为“Windows 已验证通过”。

### 3.1 Batch development and deferred Windows validation

本项目默认采用“批量开发、集中验证”，而不是每完成一个功能就立即切换 Windows：

```text
Feature A → Feature B → Feature C
→ 完成当前范围内所有 Linux 可开发工作
→ Linux verification
→ 汇总 Windows Validation Queue
→ Windows Validation Preparation
→ 一次性执行合并后的 Windows validation plan
```

除非 Windows 结果是继续开发的硬性前置条件，或用户明确要求立即验证，否则开发过程中不得提前进入 Windows validation phase。完成一个功能后，先完成实现和 Linux 验证，再将待确认项目加入累计队列，继续下一个不依赖 Windows 新结果的开发项。

不得仅因为功能最终需要 Windows 测试、可能存在兼容性问题或属于跨平台代码，就中断 Linux development phase。

只有以下情况可以提前中断：后续设计依赖 Windows-specific 行为；Windows API/filesystem/process/installer 行为无法可靠推断；关键兼容性假设错误会使大量后续开发失效；问题只能在 Windows 复现且阻塞继续开发；或用户明确要求立即验证。

## 4. Validation State Model

平台相关任务应尽可能使用以下状态：

- `IMPLEMENTED`
- `LINUX_VERIFIED`
- `WINDOWS_VERIFICATION_PENDING`
- `WINDOWS_VERIFICATION_BLOCKING`
- `WINDOWS_PASS`
- `WINDOWS_FAIL`
- `WINDOWS_BLOCKED`
- `NOT_RUN`
- `NOT_APPLICABLE`

典型状态流：

```text
IMPLEMENTED
→ LINUX_VERIFIED
→ WINDOWS_VERIFICATION_PENDING
→ WINDOWS_PASS
```

若 Windows 验证失败：

```text
WINDOWS_FAIL
→ Linux fix
→ LINUX_VERIFIED
→ WINDOWS_VERIFICATION_PENDING
→ Windows re-validation
```

Linux 端不得在 Windows 实际重新验证之前，将修复后的项目标记为 `WINDOWS_PASS`。

`WINDOWS_VERIFICATION_PENDING` 是默认状态，表示当前 Linux 开发可以继续，待 Linux development phase 结束后集中验证。`WINDOWS_VERIFICATION_BLOCKING` 只表示缺少 Windows 结果会导致后续 Linux 设计或实现无法可靠继续；它不是普通的“重要”或“最终需要测试”标记。

### Windows Validation Queue

开发阶段维护累计的 Windows Validation Queue。每发现一个需要 Windows 确认的修改，记录到队列，不自动切换环境。每项至少包含：

| 字段 | 要求 |
|---|---|
| Validation item | 验证项目名称或 ID |
| Related feature/change | 关联功能和本轮修改 |
| Files/modules | 相关文件、crate、命令或平台路径 |
| Why Windows is required | Linux 无法充分判断的具体原因 |
| Exact behavior | 需要实际确认的行为 |
| Prerequisite | 账号、工具、artifact、automation target 等 |
| Expected result | 可观察的通过标准 |
| Priority | `P0` / `P1` / `P2` |
| Blocks Linux development | `yes/no` |
| Status | 默认 `WINDOWS_VERIFICATION_PENDING`，仅硬性前置使用 `WINDOWS_VERIFICATION_BLOCKING` |

### End of Linux development phase

只有在以下条件基本满足后，才能结束当前 Linux development phase：

- 当前 Plan 中所有不依赖 Windows 的开发项已完成；
- Linux 适用的 unit/integration/regression/lint/typecheck/formatter/build/static checks 已完成；
- 已知 Linux 错误已经处理；
- Windows Validation Queue 已累计完整；
- 没有遗漏明显的平台相关验证要求。

此后进入 `Windows Validation Preparation`，不要继续零散地追加与当前范围无关的功能开发。

### Windows Validation Preparation and handoff

准备阶段必须统一分析最终 `git diff`、current Plan、changed modules、Windows code paths、项目文档、build/CI 配置、历史 Windows validation 和 Windows Validation Queue，合并重复或高度相关的验证场景。

集中式 handoff 按以下类别组织：

1. Build / Toolchain；
2. Runtime；
3. Filesystem；
4. Integration；
5. Packaging；
6. Regression。

每个 handoff 项必须包含：ID、test name、purpose、related changes、prerequisites、steps/command、expected result、priority 和是否需要人工交互。优先级含义为：`P0` 必须验证且失败可能导致任务不能完成；`P1` 重要平台兼容性；`P2` 建议验证但不阻塞主要功能。

## 5. Linux Development Responsibilities

Linux Codex 负责：

- 阅读当前仓库和项目文档；
- 执行主要开发工作；
- 执行 Linux 平台适用的验证；
- 分析 Windows 验证结果；
- 修复真正属于项目代码的问题；
- 更新 Plan 和项目文档；
- 标记需要下一轮 Windows 验证的内容。

Linux Codex 不应：

- 假装执行了 Windows-only 验证；
- 将 Linux 测试成功等同于 Windows 验证成功；
- 为纯 Windows 环境配置问题修改项目代码；
- 无依据扩大当前任务范围。

## 6. Windows Validation Responsibilities

Windows Codex 负责：

- 读取 Linux 项目及项目文档；
- 将需要验证的内容从 Linux 单向同步至 Windows 工作副本；
- 根据仓库、文档、构建配置和脚本确定验证范围；
- 执行适用的 Windows 验证；
- 记录 `PASS` / `FAIL` / `BLOCKED` / `NOT RUN` / `NOT APPLICABLE`；
- 分析失败原因；
- 将实际验证结果写回 Linux 项目的验证文档。

Windows Codex 默认执行验证，而不是功能开发。

除机器本地配置、构建产物或验证所需临时变化外，不应为了让验证通过而自行修改业务代码。

若发现代码问题，应记录：

- failure；
- reproduction；
- likely root cause；
- relevant code location；
- suggested follow-up。

该问题应交由 Linux 开发阶段处理。

## 7. Windows Synchronization Rules

Windows 验证工作区应以当前 Linux 项目状态为准。

同步前应检查：

- Linux branch；
- Linux commit；
- Linux working tree；
- Windows target directory；
- 是否存在 Windows 本地需要保留的配置。

默认不要跨平台同步：

- `.git`，除非验证流程需要；
- `node_modules`；
- Python virtual environments；
- Rust `target`；
- build caches；
- IDE caches；
- temporary files；
- secrets；
- machine-specific configuration。

优先使用项目已有的：

- sync scripts；
- checkout procedures；
- worktree workflow；
- build preparation scripts。

## 8. Windows Validation Result Classification

每个验证项目至少应使用以下结果之一：

### PASS

验证实际执行，并满足预期。

### FAIL

验证实际执行，但项目行为或输出不符合要求。

应记录：

- command；
- failure point；
- relevant error；
- likely cause；
- whether Windows-specific；
- follow-up recommendation。

### BLOCKED

由于前置条件缺失无法继续。例如：

- required dependency unavailable；
- certificate unavailable；
- external service unavailable；
- credential unavailable；
- prerequisite test/build failed；
- unsupported environment。

必须记录阻塞原因。

### NOT RUN

本轮未执行，但理论上应执行。必须说明原因。

### NOT APPLICABLE

根据当前项目或平台状态明确不适用。

## 9. Reconciliation of Windows Results on Linux

当 Windows 验证结果写回 Linux 项目后，Linux Codex 必须先重新读取：

- `git diff`；
- modified documentation；
- current Plan；
- validation result documents；
- relevant `AGENTS.md` instructions。

然后重新评估当前 Plan。

应区分：

- 已经 Windows 验证完成的事项；
- Windows 失败且属于代码问题的事项；
- 环境问题；
- `BLOCKED` 项目；
- 尚未执行项目；
- 已不再适用的项目；
- 需要下一轮 Windows re-validation 的项目。

不得机械继续旧 Plan。当前仓库和最新验证结果优先于旧任务假设。

## 10. Handling Windows Failures

Windows `FAIL` 后，Linux Codex 首先判断问题属于：

- project code；
- Windows compatibility；
- dependency；
- environment；
- credential；
- external service；
- test infrastructure；
- unknown。

只有真正属于项目代码或必要兼容性问题时才修改代码。

修复后：

1. 执行 Linux 适用的 regression tests；
2. 执行相关 Linux verification；
3. 更新文档；
4. 将 Windows 状态改为 `WINDOWS_VERIFICATION_PENDING`，而不是 `WINDOWS_PASS`。

同时记录下一轮 Windows 应重新执行哪些验证。

## 11. Documentation Rules

Windows 实际验证历史必须保留，不得为了反映最新代码状态而删除历史失败记录。

建议使用以下结构：

```text
Previous Windows validation:
FAIL

Issue:
...

Linux fix:
...

Linux verification:
PASS

Current Windows status:
WINDOWS_VERIFICATION_PENDING
```

验证文档应区分：

- Windows validation result；
- Linux implementation/fix；
- Linux verification；
- pending Windows re-validation。

## 12. Plan Management

Windows 验证可能影响原 Plan。Linux Codex 读取结果后，应对 Plan 中每个剩余步骤判断：

- completed；
- still required；
- modified；
- obsolete；
- blocked；
- requires Windows re-validation。

Plan 的总体目标不应因为验证结果自动扩大。只有当前仓库、明确需求或实际验证事实证明必要时，才能新增工作。

## 13. Verification Principles

Codex 应优先从仓库自动确定验证命令，包括：

- `AGENTS.md`；
- project documentation；
- CI configuration；
- package scripts；
- `Cargo.toml`；
- `pyproject.toml`；
- `package.json`；
- build scripts；
- test configuration。

不得凭空创造项目不存在的验证流程。

## 14. Final Diff Review

每轮 Linux 开发结束时检查最终 `git diff`，确保：

- 没有无关修改；
- Plan / validation documentation 已更新；
- Windows 状态没有被错误标记；
- 需要重新验证的项目已经明确列出。

每轮 Windows 验证结束时同样检查：

- Windows 工作副本没有意外成为新的代码事实来源；
- Linux 端只接收预期的验证文档更新；
- 没有未经确认的代码反向同步。

## 15. Core Principle

始终区分三个不同事实：

1. `Code implemented`；
2. `Linux verified`；
3. `Windows verified`。

只有在 Windows 平台实际执行并通过相应验证后，才能声明：

```text
Windows verified
```

## Related Documentation

- Agent 执行规则：[`../../AGENTS.md`](../../AGENTS.md)
- Windows 验证规范：[`../validation/windows.md`](../validation/windows.md)
- 当前 Windows 验证结果：[`windows-validation.md`](windows-validation.md)