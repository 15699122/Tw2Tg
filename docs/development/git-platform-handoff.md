# Git-based Cross-platform Handoff Workflow

## 1. Purpose

本项目的正式跨平台协作使用 Git-based handoff。

长期正式流程：

`Linux → Git remote → Windows`

以及：

`Windows → Git remote → Linux`

直接：

`Linux → Windows filesystem`

仅作为快速实验和诊断通道。

本规范用于明确：

- canonical state；
- platform handoff；
- workspace ownership；
- direct-sync boundary；
- branch/revision tracking；
- platform reconciliation。

## 2. Workspace Model

项目维护两个独立 working tree。

### Linux Working Tree

用途：

- Cross-platform development；
- shared architecture；
- shared tests；
- Linux/cross-platform validation。

应位于 Linux-native filesystem，例如：

`~/projects/<project>`

### Windows Working Tree

用途：

- Windows-specific development；
- Windows runtime validation；
- Windows GUI；
- packaging / installer；
- Windows-native integration。

应位于 Windows-native filesystem，例如：

`E:\Projects\<project>`

两个 working tree：

- 各自拥有 `.git`；
- 各自维护自己的 build artifacts 和 dependency cache；
- 不通过文件镜像保持实时一致；
- 通过 Git revision 交换正式项目状态。

## 3. Canonical State

正式项目状态由 Git history 定义。

每次 platform handoff 必须能回答：

- 当前 branch 是什么？
- handoff source commit 是什么？
- 是否包含未提交修改？
- 谁是当前 Owner？
- 下一 Owner 应从哪个 revision 开始？

正式 handoff 默认不得依赖未提交修改。

如果确实存在必须保留的 uncommitted state，应先：

- commit；
- 或建立明确的 temporary/WIP commit；

再进行正式 platform handoff。

## 4. Formal Linux -> Windows Handoff

Linux Cross-platform Owner 完成当前 batch 后：

1. 完成当前所有不依赖 Windows blocking result 的 Cross-platform 工作；
2. 执行最小必要 Linux/cross-platform validation；
3. 检查 `git diff`；
4. 更新当前 Plan 和 `platform-handoff.md`；
5. commit 当前 batch；
6. push 到 configured Git remote；
7. 在 handoff 文档中记录 source revision；
8. 将 ownership 转交给 Windows Platform Owner。

Windows Owner：

1. fetch remote；
2. 确认当前 Windows working tree 无未处理本地修改；
3. checkout/update 到 handoff revision；
4. 对照 handoff 文档确认 revision；
5. 开始 Windows batch。

## 5. Formal Windows -> Linux Handoff

Windows Owner 完成 Windows batch 后：

1. 完成 Windows-owned implementation；
2. 完成最小必要 Windows validation；
3. 处理 Windows-owned failures；
4. 标记 `CROSS_PLATFORM_CHANGE_REQUIRED` / `CROSS_PLATFORM_REVIEW_REQUIRED`（如适用）；
5. 更新 handoff / validation documentation；
6. 检查 final diff；
7. commit Windows batch；
8. push 到 Git remote。

如果没有 Cross-platform follow-up：Windows batch 可以直接结束。

如果存在 Cross-platform follow-up，Linux Owner：

1. fetch remote；
2. 更新 Linux working tree；
3. 阅读 Windows handoff；
4. review Windows shared changes；
5. 处理 Cross-platform follow-up。

## 6. Branch Strategy

默认优先保持简单。

同一逻辑 batch 可以使用同一个 feature/task branch：

`feature/<task>`

典型流程：

Linux:
`A → B`

handoff to Windows

Windows:
`B → C`

handoff to Linux when required

Linux:
`C → D`

不要仅因为切换平台自动创建大量平台分支。

如果 Windows 实验具有较高风险或可能被丢弃，可以使用临时分支，例如：

`windows/experiment-<topic>`

实验稳定后再整合。

## 7. Handoff Revision

每次 handoff 都必须记录明确 revision。

例如：

```text
Branch:
feature/foo

Source revision:
abc1234

Owner:
Cross-platform -> Windows
```

Windows 验证结果必须对应一个明确 revision。

不能只记录“最新代码”。

如果 Windows 在验证前进行了 Windows-owned 修改，应记录：

```text
Input revision:
abc1234

Windows implementation revision:
def5678

Validation revision:
def5678
```

这样可以明确测试真正覆盖了哪个状态。

## 8. Direct Sync Fast Path

直接：

`Linux → E:`

同步只用于：

- diagnostic experiment；
- exploratory Windows testing；
- blocking compatibility investigation；
- quick GUI/runtime check；
- 尚不值得形成正式 Git handoff 的临时状态。

Direct-sync workspace 必须：

- 是 disposable workspace；
- 或明确标记为 scratch；
- 与正式 Windows Owner repo 分离。

推荐：

`E:\Projects\<project>` = 正式 Windows Owner repository

`E:\Scratch\<project>` = direct-sync experimental workspace

## 9. Direct Sync Restrictions

禁止 Linux direct sync 覆盖正式 Windows Owner working tree。

特别是在 Windows working tree 存在：

- uncommitted changes；
- Windows-owned implementation；
- local platform configuration；
- pending handoff；

时。

Direct sync 不得成为：

- formal commit history 的替代品；
- Windows-owned code 的长期存储位置；
- canonical validation revision；
- release source。

## 10. Experimental Result Promotion

如果 direct-sync experiment 得到有价值结果：

### Result only

如果只是得到平台事实，例如：

“Windows API X 在该条件下不可用”

将证据写入 handoff / issue / validation documentation。

然后由相应 Owner 根据 canonical Git state 正式实施。

### Experimental code worth keeping

不要直接把 scratch workspace 整体反向复制到正式项目。

应：

1. 识别真正需要保留的 change；
2. 在正式 Owner repo 中重新应用或有控制地迁移；
3. review diff；
4. test；
5. commit；
6. 通过正常 Git workflow 集成。

## 11. Dirty Workspace Protection

任何正式 handoff 前都必须检查：

`git status`

如果目标 workspace 存在未提交修改：

不要直接 pull/reset/覆盖。

先判断：

- 当前修改属于谁；
- 是否已完成；
- 是否应 commit；
- 是否应 stash；
- 是否可安全丢弃。

不要自动使用 destructive reset 来解决 handoff 冲突。

## 12. Conflict Handling

如果 Linux 和 Windows 修改了相同 shared files：

使用 Git merge/rebase conflict 作为显式 reconciliation point。

不要通过文件同步工具选择“较新的文件”来解决冲突。

解决冲突时应检查：

- ownership；
- intended behavior；
- shared contract；
- platform-specific requirements。

## 13. Cross-platform Review

如果 Windows commit 包含 `CROSS_PLATFORM_REVIEW_REQUIRED`，Linux Owner 必须检查：

- shared API 是否改变；
- cross-platform behavior 是否改变；
- Linux behavior 是否受影响；
- shared tests 是否足够；
- Windows fix 是否泄漏平台逻辑到 shared core。

审查完成后：

- 接受；
- 调整；
- 或重新设计。

## 14. Validation and Revision Reuse

验证结果必须绑定到 revision。

例如：

```text
WIN-BUILD-001
PASS
revision: def5678
```

后续 revision 如果没有影响该验证项相关的 files、dependencies、contracts、environment assumptions，可以根据 validation policy 复用结果。

如果相关影响发生变化，标记：`REVALIDATION_REQUIRED`

## 15. Platform Handoff Status

推荐状态：

- `CROSS_PLATFORM_IN_PROGRESS`
- `READY_FOR_WINDOWS`
- `WINDOWS_IN_PROGRESS`
- `WINDOWS_WORK_PENDING`
- `WINDOWS_VERIFICATION_PENDING`
- `WINDOWS_BLOCKING`
- `CROSS_PLATFORM_CHANGE_REQUIRED`
- `CROSS_PLATFORM_REVIEW_REQUIRED`
- `COMPLETE`

这些表示 workflow state，不代替测试结果：

- `PASS`
- `FAIL`
- `BLOCKED`
- `NOT_RUN`
- `NOT_APPLICABLE`

## 16. platform-handoff.md Template

`docs/status/platform-handoff.md` 建议保持为当前批次状态：

```text
# Current Platform Handoff

## Batch

Task:
Branch:
Current owner:
Current state:

## Revisions

Cross-platform input revision:
Cross-platform handoff revision:

Windows input revision:
Windows implementation revision:
Windows validation revision:

## Cross-platform Work Completed

-

## Windows Work Required

-

## Windows Validation Required

-

## Expected Behavior

-

## Known Risks

-

## Windows Results

Implementation:
-

PASS:
-

FAIL:
-

BLOCKED:
-

Manual validation required:
-

## Cross-platform Follow-up

CROSS_PLATFORM_CHANGE_REQUIRED:
-

CROSS_PLATFORM_REVIEW_REQUIRED:
-

## Next Owner

Owner:

Required actions:
-
```

只记录当前有效 handoff。

历史验证放入独立历史文档（`docs/validation/windows-validation-history.md`），不要无限追加到 handoff 文件。

## 17. Batch Principle

平台交接应该按 batch 发生。

推荐：

```text
Linux A + B + C
→ Git handoff
→ Windows D + E + validation
→ Git handoff if required
```

不要采用：

```text
Linux A
→ Git
→ Windows
→ Git
→ Linux B
→ Git
→ Windows
```

除非存在真正的 `WINDOWS_BLOCKING`。

## 18. Core Rules

1. Git history defines formal project state.
2. Git remote is the normal platform exchange point.
3. Linux and Windows maintain independent native working trees.
4. Direct filesystem sync is diagnostic-only.
5. Direct sync must not overwrite the formal Windows Owner repository.
6. Platform handoff must identify a revision.
7. Validation results should identify the revision they tested.
8. Windows-owned code may be developed and committed on Windows.
9. Shared-contract changes belong to the Cross-platform Owner.
10. Prefer batch handoff over platform ping-pong.

## Related Documentation

- Repository rules: [`../../AGENTS.md`](../../AGENTS.md)
- Ownership and routing: [`platform-ownership.md`](platform-ownership.md)
- Validation policy: [`../validation/validation-policy.md`](../validation/validation-policy.md)
- Current handoff state: [`../status/platform-handoff.md`](../status/platform-handoff.md)
- Daily owner prompts: [`platform-handoff-prompts.md`](platform-handoff-prompts.md)