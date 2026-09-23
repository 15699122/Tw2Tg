## Dual-owner governance

This workflow is subordinate to the repository-level dual-owner model. Git repository state plus committed project documentation, Plan/task state, and recorded validation results are canonical. Linux is the Cross-platform Owner; Windows is the Windows Platform Owner. Windows-specific implementation and production-code changes are allowed within the Windows boundary.

Use READY_FOR_WINDOWS for a handoff, WINDOWS_WORK_PENDING or WINDOWS_VERIFICATION_PENDING for deferred work, WINDOWS_BLOCKING only for a hard prerequisite, CROSS_PLATFORM_CHANGE_REQUIRED for shared contract/architecture changes, and CROSS_PLATFORM_REVIEW_REQUIRED for a small shared adjustment that preserves an existing abstraction. Do not use unconditional Linux-to-Windows mirroring to overwrite unintegrated Windows work.

For detailed ownership rules see platform-ownership.md; for the Git-based handoff workflow, revision recording, and direct-sync boundary see git-platform-handoff.md; for test selection and Computer Use fallback see ../validation/validation-policy.md.

IyMgRHVhbC1vd25lciBnb3Zlcm5hbmNlCgpUaGlzIHdvcmtmbG93IGlzIHN1Ym9yZGluYXRlIHRvIHRoZSByZXBvc2l0b3J5LWxldmVsIGR1YWwtb3duZXIgbW9kZWwuIEdpdCByZXBvc2l0b3J5IHN0YXRlIHBsdXMgY29tbWl0dGVkIHByb2plY3QgZG9jdW1lbnRhdGlvbiwgUGxhbi90YXNrIHN0YXRlLCBhbmQgcmVjb3JkZWQgdmFsaWRhdGlvbiByZXN1bHRzIGFyZSBjYW5vbmljYWwuIExpbnV4IGlzIHRoZSBDcm9zcy1wbGF0Zm9ybSBPd25lcjsgV2luZG93cyBpcyB0aGUgV2luZG93cyBQbGF0Zm9ybSBPd25lci4gV2luZG93cy1zcGVjaWZpYyBpbXBsZW1lbnRhdGlvbiBhbmQgcHJvZHVjdGlvbi1jb2RlIGNoYW5nZXMgYXJlIGFsbG93ZWQgd2l0aGluIHRoZSBXaW5kb3dzIGJvdW5kYXJ5LgoKVXNlIFJFQURZX0ZPUl9XSU5ET1dTIGZvciBhIGhhbmRvZmYsIFdJTkRPV1NfV09SS19QRU5ESU5HIG9yIFdJTkRPV1NfVkVSSUZJQ0FUSU9OX1BFTkRJTkcgZm9yIGRlZmVycmVkIHdvcmssIFdJTkRPV1NfQkxPQ0tJTkcgb25seSBmb3IgYSBoYXJkIHByZXJlcXVpc2l0ZSwgQ1JPU1NfUExBVEZPUk1fQ0hBTkdFX1JFUVVJUkVEIGZvciBzaGFyZWQgY29udHJhY3QvYXJjaGl0ZWN0dXJlIGNoYW5nZXMsIGFuZCBDUk9TU19QTEFURk9STV9SRVZJRVdfUkVRVUlSRUQgZm9yIGEgc21hbGwgc2hhcmVkIGFkanVzdG1lbnQgdGhhdCBwcmVzZXJ2ZXMgYW4gZXhpc3RpbmcgYWJzdHJhY3Rpb24uIERvIG5vdCB1c2UgdW5jb25kaXRpb25hbCBMaW51eC10by1XaW5kb3dzIG1pcnJvcmluZyB0byBvdmVyd3JpdGUgdW5pbnRlZ3JhdGVkIFdpbmRvd3Mgd29yay4KCkZvciBkZXRhaWxlZCBvd25lcnNoaXAgcnVsZXMgc2VlIHBsYXRmb3JtLW93bmVyc2hpcC5tZDsgZm9yIHRlc3Qgc2VsZWN0aW9uIGFuZCBDb21wdXRlciBVc2UgZmFsbGJhY2sgc2VlIC4uL3ZhbGlkYXRpb24vdmFsaWRhdGlvbi1wb2xpY3kubWQuCgo=# Cross-platform Development and Windows Validation Workflow

## 1. Purpose

This repository uses a dual-owner model. Linux is the Cross-platform Owner and Windows is the Windows Platform Owner. Both owners are responsible for design, implementation, validation, issue triage, and follow-up within their boundary.

Linux owns shared architecture, cross-platform core behavior, shared APIs and protocols, data models, platform-neutral behavior, shared tests, and primary architecture documentation. Windows owns Windows-specific implementation, native integration, filesystem/process behavior, GUI, services, registry, PowerShell, packaging, configuration, compatibility fixes, and Windows validation.

Windows is not merely a validation environment. Windows may modify production code within its ownership boundary.

## 2. Canonical project state

The canonical state is the combination of current Git repository state, committed project documentation, current Plan/task state, and recorded platform validation results. No machine-local workspace is the sole source of truth. Linux and Windows workspaces are execution environments for their ownership scopes.

Changes from either owner must be integrated into the canonical Git repository before they are considered part of project state. Do not use unconditional Linux-to-Windows mirroring to overwrite unintegrated Windows work. Synchronization follows the active handoff and preserves local platform configuration, credentials, caches, and artifacts.

## 3. Development / Validation Cycle

标准工作流：

1. Linux implementation；
2. Linux verification；
3. 标记 Windows verification requirements；
4. 通过 Git handoff 将 Linux 批次交付 Windows（直接文件同步仅限诊断实验，见 [`git-platform-handoff.md`](git-platform-handoff.md)）；
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

### 3.2 Incremental validation and minimal scope

Linux 与 Windows 验证均采用最小必要范围，规则细节见 [`testing.md`](testing.md)「增量验证策略：最小必要范围」。对跨平台工作流的补充约束：

- Linux 阶段按当前 diff 执行最小相关验证；第 4 节与「End of Linux development phase」中提到的 unit/integration/regression/lint/typecheck/formatter/build/static checks 指**与改动相关的适用项**，不解释为每轮全仓库执行。全量组合仅在 release、major refactor、schema/migration change、large cross-module diff 等触发条件满足时执行。
- 进入 Windows Validation Preparation 时，基于 final diff、current Plan、changed modules、Windows Validation Queue 和 previous Windows validation results 做影响面分析：合并重复场景，剔除当前改动不会影响的项目，不把整份队列视为下一轮的默认执行清单。
- Windows 阶段只执行当前改动相关的平台验证；GUI / Computer Use 测试成本高、最后执行，只有改动涉及 layout、visual behavior、window lifecycle、interaction、native dialogs 或 GUI-driven workflow 时才默认执行。Computer Use 不可用时相关项目标记 `BLOCKED` 并加入 Manual Windows Validation Queue。

### 3.3 Windows revalidation rules

每个 Windows 验证项可维护重验元数据：last validated revision、related files/modules、dependencies、status。判定规则：

- 若 `current diff ∩ test impact area = empty` 且相关依赖未变化，则该项保持上一轮的有效结论（含 `WINDOWS_PASS`），无需重复执行；
- 若存在交集、依赖变化或行为可能使原结论失效，则标记 `REVALIDATION_REQUIRED` 并按状态流回到 `WINDOWS_VERIFICATION_PENDING`；
- 不因当前改动与某项无关而重复运行该项；也不得仅凭「上一轮通过」跳过当前 diff 明确命中的项目。

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

### 5.1 Linux Cline 测试工作流

Linux 侧遵循：

```text
开发
→ Unit Test
→ Integration Test
→ WDIO Browser Mode
→ Linux Native WDIO / Native E2E（适用时）
→ 修复
→ 相关回归测试
→ 继续剩余 Linux 开发
→ 当前阶段 Linux 工作全部完成
→ 整理 Windows Validation Queue
```

不得采用“开发一个小功能 → 停止所有开发 → 等待 Windows 验证 → 再继续”的节奏，除非 Windows 结果是后续设计或实现的硬性依赖，或用户明确要求立即验证。Linux 端应优先完成静态、快速和不依赖 Windows 的测试；Windows-only 用例在 Linux 上记录为 `SKIPPED_PLATFORM`，不视为失败。

Linux Codex 负责：

- 阅读当前仓库和项目文档；
- 执行主要开发工作；
- 执行 Linux 平台适用的验证；
- 分析 Windows 验证结果；
- 修复真正属于项目代码的问题；
- 更新 Plan 和项目文档；
- 标记需要下一轮 Windows 验证的内容。

Linux 开发环境中的 Tauri MCP 约束：

- MCP Bridge 是项目 Debug-only Rust 依赖配置，Linux/Windows 使用同一份源码；
- `@hypothesi/tauri-mcp-server` 属于 Agent 环境工具，不提交到项目 `package.json`；
- Linux 没有可用 GUI 时，不启动 Tauri MCP，不因缺少 GUI 中断 Linux development phase；
- Linux 继续执行 Rust、Node、Python、静态检查和非 GUI integration tests；
- GUI、WebView2、真实 Tauri runtime、进程、文件锁和打包验证统一进入 Windows handoff。

Linux Codex 不应：

- 假装执行了 Windows-only 验证；
- 将 Linux 测试成功等同于 Windows 验证成功；
- 为纯 Windows 环境配置问题修改项目代码；
- 无依据扩大当前任务范围。

## 6. Windows Owner Responsibilities

Windows Codex owns Windows-specific implementation, compatibility fixes, platform-specific tests, GUI validation, packaging validation, and Windows runtime diagnosis. It may modify production code within that boundary.

If a finding requires shared architecture, API, protocol, schema, data-model, or platform-neutral behavior changes, record CROSS_PLATFORM_CHANGE_REQUIRED and hand it back to the Linux Cross-platform Owner. A small shared adjustment that preserves an existing abstraction must be marked CROSS_PLATFORM_REVIEW_REQUIRED.

Windows records PASS, FAIL, BLOCKED, NOT_RUN, and NOT_APPLICABLE with commands, evidence, reasons, and follow-up. It does not claim validation that was not executed.

## 7. Handoff and synchronization rules

正式 platform handoff 使用 Git-based workflow，规范见 [`git-platform-handoff.md`](git-platform-handoff.md)：

`Linux working tree → Git commit → Git remote → Windows working tree`，反向同理。正式 Windows Owner repository（`E:\Projects\<project>`）只通过 Git 更新；直接 Linux → Windows 文件同步仅作为临时诊断通道，必须指向 disposable scratch workspace（`E:\Scratch\<project>`），不得覆盖 Windows Owner 正式 working tree，其结果在通过正式 Git workflow 复现或集成前一律视为 experimental。

同步/checkout 前应检查：

- Linux branch；
- Linux commit；
- Linux working tree；
- Windows target directory；
- 是否存在 Windows 本地需要保留的配置。

诊断性 direct-sync 默认排除：

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

### 单测试结果状态

本节的 `PASS` / `FAIL` / `BLOCKED` / `NOT RUN` / `NOT APPLICABLE` 用于平台验证项目和队列，不应替代单个测试的诊断分类。单测试统一使用：

```text
PASS
PASS_FLAKY
PASS_AFTER_FIX
PASS_AFTER_TEST_FIX
FAIL_PRODUCT
FAIL_PRODUCT_NEEDS_DEVELOPMENT
FAIL_TEST
BLOCKED_ENV
BLOCKED_AUTOMATION
SKIPPED_PLATFORM
NEEDS_REVIEW
```

其中：

- `PASS_FLAKY` 仅用于初次失败、按安全条件局部重试后成功的用例；
- `FAIL_PRODUCT` 表示产品或平台实现确定性错误；需要架构、需求判断或大范围修改时使用 `FAIL_PRODUCT_NEEDS_DEVELOPMENT`；
- `FAIL_TEST` 表示 selector、fixture、mock、expectation、隔离或 WDIO 配置问题；
- `BLOCKED_ENV` 表示依赖、工具链、WebView2、驱动、外部服务或凭据前置缺失；
- `BLOCKED_AUTOMATION` 表示自动化工具或 GUI 控制不可用，且没有产品缺陷证据；
- `SKIPPED_PLATFORM` 表示当前平台不适用；`NEEDS_REVIEW` 表示需求或规范冲突需要人工裁决。

测试失败后的诊断顺序固定为：测试步骤/断言、WDIO 输出、frontend console、Tauri IPC/invoke、Rust/backend 日志、平台系统错误、测试代码、产品代码、环境/自动化基础设施。仅对可能 transient/flaky 的问题局部重试，最多 2 次；不得对确定性失败、compiler error、panic、schema mismatch 或权限错误进行无意义重试。

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

### Automation, independence and security

- 快速测试优先于昂贵测试；失败测试优先局部重跑；环境问题只阻塞依赖该环境的测试；阶段结束再执行适当范围的综合回归。
- 每个 E2E 尽量独立创建和清理自己的数据，不依赖执行顺序或共享可变全局状态；发生状态污染时只恢复受影响的测试环境，不无条件重置用户环境。
- E2E 可以启用 `tauri-plugin-wdio`、`tauri-plugin-wdio-webdriver` 和测试专用 capability，但这些能力必须仅存在于 E2E/Test build。不得将 WebDriver 暴露到 production build、永久扩大生产 capability、禁用生产安全机制或提交真实凭据。
- 自动修复仅限明确的测试脚本、局部实现、类型/编译和配置问题；架构、数据模型、安全模型、权限扩大、用户数据格式和 API breaking change 必须记录为 `NEEDS_DEVELOPMENT_REVIEW`。
- 单测试结果至少保留 `case_id`、test name、platform、layer、command、timestamp、expected、actual、相关 WDIO 输出、frontend console、backend/Rust log 和 stack trace（如有）；GUI 问题按需附带 screenshot、窗口信息和 URL/route。最终汇总应区分产品、测试、环境、自动化阻塞、平台跳过和待评审项目。
- `BLOCKED_AUTOMATION` 必须附带前置条件、启动方式、交互步骤、测试数据、预期结果、日志收集要求以及 PASS/FAIL 判定标准。

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
