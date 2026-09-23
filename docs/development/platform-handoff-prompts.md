# Platform Handoff Daily Prompts

规则固化之后，日常 Prompt 无需重复说明 Git 细节。本文件长期保存 Linux 与 Windows 两个操作模板，按当前 Owner 平台选用。规范依据见 [`git-platform-handoff.md`](git-platform-handoff.md)。

## Linux — Cross-platform Owner

接手当前 Cross-platform batch。

读取：

- `AGENTS.md`
- `docs/development/platform-ownership.md`
- `docs/development/git-platform-handoff.md`
- `docs/validation/validation-policy.md`
- `docs/status/platform-handoff.md`
- 当前 Plan

首先 fetch/reconcile 最新 canonical Git state，并确认当前 branch、revision、working tree 与 handoff 状态。

然后：

- reconcile Windows 上一轮 implementation / validation；
- review `CROSS_PLATFORM_REVIEW_REQUIRED`；
- 处理 `CROSS_PLATFORM_CHANGE_REQUIRED`；
- 继续所有仍属于 Cross-platform Owner 的工作；
- 不接管仍属于 Windows Owner 的 Windows-specific implementation；
- 采用最小必要 validation；
- 将非阻塞 Windows work / verification 累积到下一 Windows batch；
- 完成所有非 Windows-dependent 工作后，再形成正式 Windows handoff。

正式 handoff 前：

1. 检查 final `git diff`；
2. 更新 Plan 和 `docs/status/platform-handoff.md`；
3. 确认 handoff revision；
4. commit 当前 Cross-platform batch；
5. push 到 configured Git remote；
6. 将状态更新为 `READY_FOR_WINDOWS`。

不要通过直接覆盖 `E:` 盘正式 Windows repo 来完成 handoff。

如果仅需要快速 Windows 实验且不值得形成正式 handoff，可以使用 direct-sync scratch path；该结果必须视为 experimental，不得作为 canonical project state。

最后报告：

- branch；
- input revision；
- Cross-platform handoff revision；
- 完成的 Cross-platform work；
- validation scope/results；
- Windows work queue；
- Windows validation queue；
- 是否存在 `WINDOWS_BLOCKING`；
- 下一 Owner。

## Windows — Windows Platform Owner

接手当前 Windows Platform batch。

读取：

- `AGENTS.md`
- `docs/development/platform-ownership.md`
- `docs/development/git-platform-handoff.md`
- `docs/validation/validation-policy.md`
- `docs/status/platform-handoff.md`
- 当前 Plan

首先：

1. fetch configured Git remote；
2. 检查 Windows working tree 是否存在未提交修改；
3. 将正式 Windows repo 更新到 handoff revision；
4. 确认实际 revision 与 `platform-handoff.md` 一致。

不要通过 Linux 目录直接覆盖正式 Windows working tree。

然后：

- 完成所有当前 Windows-owned implementation；
- 完成最小必要 Windows validation；
- Windows-owned failure 直接修复并重新验证；
- shared/contract 问题标记 `CROSS_PLATFORM_CHANGE_REQUIRED`；
- 小型 shared implementation change 标记 `CROSS_PLATFORM_REVIEW_REQUIRED`；
- Computer Use 不可用时使用 `BLOCKED + Manual Windows Validation Queue`；
- 尽量完成整个 Windows batch，避免无必要的平台往返。

Windows batch 完成后：

1. 检查 final `git diff`；
2. 更新 validation / handoff documentation；
3. 记录实际 Windows validation revision；
4. commit Windows-owned changes；
5. push 到 configured Git remote；
6. 仅在存在 Cross-platform follow-up 时将 ownership 返回 Linux。

如果不存在 Cross-platform follow-up，可以直接结束当前 batch。

如果需要快速测试尚未正式 handoff 的 Linux 中间状态，只能使用独立 scratch workspace。此类 direct-sync 测试：

- 不得覆盖正式 Windows repo；
- 不得成为正式 Windows implementation；
- 不得将 scratch state 标记为 canonical；
- 有价值的代码必须通过正常 Git workflow 重新集成。

最后报告：

- input branch/revision；
- Windows implementation revision；
- Windows validation revision；
- 完成的 Windows-owned work；
- PASS / FAIL / BLOCKED；
- Manual Validation Queue；
- `CROSS_PLATFORM_CHANGE_REQUIRED`；
- `CROSS_PLATFORM_REVIEW_REQUIRED`；
- 是否需要新的 Git handoff；
- 下一 Owner。

## Formal loop

规则固化后的正式循环：

```text
Linux Cross-platform Owner
        ↓
   batch development
        ↓
 targeted validation
        ↓
   commit + push
        ↓
      GitHub
        ↓
Windows Platform Owner
        ↓
 implementation + validation
        ↓
   commit + push
        ↓
      GitHub
        ↓
Linux only when cross-platform follow-up exists
```

## Independent fast path

完全独立的快速通道：

```text
Linux unfinished state
        ↓
direct sync
        ↓
E:\Scratch\<project>
        ↓
diagnostic experiment
        ↓
discard / record evidence
```

## Standing rule

**`E:\Projects\<project>` 永远只通过 Git 更新；`E:\Scratch\<project>` 才允许 Linux 直接同步。**

该规则用于最大程度避免在两个同步机制之间混淆正式状态与实验状态。