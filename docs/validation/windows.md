# Windows 平台验证规范

> 本文是 Windows 验证的执行规范和报告模板。当前项目的具体验证结果继续记录在 [`../development/windows-validation.md`](../development/windows-validation.md)。

## 1. 目标与职责边界

主要开发工作以当前 Linux 项目目录为准。Windows 环境仅用于平台相关的构建、运行、测试和兼容性验证。

每次验证必须完成：

1. 检查当前 Linux 项目及项目文档；
2. 将需要验证的项目内容单向同步到 Windows `E:` 盘对应项目目录；
3. 根据项目自身文档、配置和脚本确定 Windows 验证范围；
4. 在 Windows 工作副本中执行验证；
5. 收集并分析错误；
6. 将验证结果、错误和未执行原因写回 Linux 验证文档。

## 1.1 执行节奏：批量开发、集中验证

Windows 验证默认延后到 Linux development phase 结束后集中执行。项目不采用“Feature A → Windows test → Feature B → Windows test”的逐功能节奏；默认采用：

```text
完成当前范围内所有 Linux 可开发功能
→ Linux verification
→ 累计 Windows Validation Queue
→ Windows Validation Preparation
→ 集中执行合并后的 Windows validation plan
```

除非 Windows 验证是后续开发的硬性前置条件，或用户明确要求立即验证，否则不得在开发过程中提前切换到 Windows。Linux 验证不能延后：每个开发阶段仍应执行适用的 unit tests、Linux integration/regression tests、lint、typecheck、formatter、build 和 static checks。

## 1.2 Windows Validation Queue

开发过程中维护累计队列，不因单个功能最终需要 Windows 验证而暂停整个 Plan。队列项使用以下状态：

- `WINDOWS_VERIFICATION_PENDING`：默认状态；Linux 开发可以继续，待集中验证；
- `WINDOWS_VERIFICATION_BLOCKING`：只有缺少 Windows 结果会使后续 Linux 设计/实现无法可靠继续时使用。

队列项模板：

```markdown
| ID | Validation item | Related feature/change | Files/modules | Why Windows is required | Exact behavior | Prerequisite | Expected result | Priority | Blocks Linux development | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| WQ-... | ... | ... | ... | ... | ... | ... | ... | P0/P1/P2 | yes/no | WINDOWS_VERIFICATION_PENDING |
```

不得仅因为某项最终需要 Windows 测试、可能存在兼容性问题或属于跨平台代码，就将其标记为 `WINDOWS_VERIFICATION_BLOCKING`。

## 1.3 Windows Validation Preparation

结束 Linux development phase 后，统一分析 final git diff、current Plan、changed modules、Windows-related code paths、项目文档、build/CI 配置、previous Windows validation history 和 Windows Validation Queue。合并重复场景，例如一次完整应用启动可以覆盖多个功能时，不得拆成多个重复启动测试。

集中式验证计划按以下类别组织：

1. Build / Toolchain；
2. Runtime；
3. Filesystem；
4. Integration；
5. Packaging；
6. Regression。

每项 handoff 至少包含：

```markdown
- ID:
- Test name:
- Purpose:
- Related changes:
- Prerequisites:
- Steps / command:
- Expected result:
- Priority: P0/P1/P2
- Manual interaction required: yes/no
```

只有 `WINDOWS_VERIFICATION_BLOCKING` 项目才可以在 Linux development phase 结束前要求 Windows 结果；其他 pending 项目统一在集中验证阶段处理。

## 2. Source of Truth

Linux 项目目录是以下内容的主要事实来源：

- 源代码和项目状态；
- 项目文档；
- 最终验证记录。

Windows `E:` 盘项目目录只是 Windows 验证工作副本。

除验证结果文档外，不得将 Windows 工作副本代码反向同步到 Linux 项目。

## 3. Phase 1：Repository Investigation

在任何同步或验证之前，先检查 Linux 源项目并记录：

- Git branch、commit、working tree 状态；
- 项目目录结构；
- 语言、框架、构建系统；
- Windows 相关文档、脚本和配置；
- 测试、lint、typecheck、build、package 等命令；
- 平台兼容性要求；
- 已有验证文档及格式；
- `AGENTS.md` 和其他 agent/project instructions；
- Windows 所需软件、环境变量、外部服务和工具。

优先相信以下来源：

1. `AGENTS.md` / 项目 agent instructions；
2. 项目文档；
3. CI / build / test 配置；
4. package/build scripts；
5. 当前代码实现。

不得凭空创造项目不存在的 Windows 验证要求。

## 4. Phase 2：Pre-sync Safety Check

同步方向固定为：

```text
Linux source → Windows E: validation workspace
```

同步前必须检查：

- Windows 目标目录是否存在；
- 目标目录是否有未提交、人工创建或需要保留的文件；
- 未知文件不得被无条件删除；
- Windows 本地配置、凭据、缓存和机器特定设置不得被覆盖；
- `.gitignore`、项目文档、构建配置和已有同步脚本规定的同步范围。

除非项目文档明确要求，通常不复制：

```text
.git
node_modules
.venv
target
build/dist cache
IDE cache
temporary files
secrets
machine-specific configuration
```

如果项目已有正式同步、checkout、worktree 或部署方式，优先使用该方式。

## 5. Phase 3：Sync Verification Workspace

同步后验证：

- 关键源代码已更新；
- Windows 工作副本对应本次 Linux 源状态；
- 没有因路径分隔符、大小写、符号链接或权限造成明显缺失；
- Windows 本地配置未被意外覆盖。

如果可可靠获得 Git hash，记录 branch、commit，以及是否包含未提交修改。Linux 源项目有未提交改动时，必须明确记录 Windows 验证包含 working tree changes，不能伪装成纯 commit 验证。

## 6. Phase 4：Determine Windows Validation Scope

根据当前项目文档和配置确定实际范围。适用项目可包括：

- dependency/environment setup；
- code generation；
- formatter check；
- lint；
- typecheck；
- unit/integration tests；
- Windows-specific tests；
- build/package；
- application startup；
- CLI 行为；
- 文件系统和路径；
- subprocess/sidecar；
- network/service integration；
- installer/package validation。

验证清单中的每一项必须标记为：

| 类别 | 含义 |
|---|---|
| Required | 项目文档明确要求 |
| Applicable | 根据当前配置应执行 |
| Not applicable | 当前项目或 Windows 环境不适用 |

只执行与项目实际相关的验证。

## 7. Phase 5：Execute Windows Validation

执行原则：

- 使用项目已有命令和脚本；
- 优先使用项目规定的软件包管理器和工具链；
- 不因失败随意更换工具或改变项目配置；
- 不修改功能代码来制造通过结果；
- 不跳过失败并标记为成功；
- 一个非致命项目失败时，继续执行不依赖它的其他项目。

每项记录：

- 项目名称；
- 实际命令；
- 工作目录；
- 结果；
- 相关版本；
- 关键输出或错误摘要。

允许的结果状态：

```text
PASS
FAIL
BLOCKED
NOT RUN
NOT APPLICABLE
```

失败项目必须记录：

- 失败步骤；
- 关键错误信息；
- 最可能原因；
- 分类：Windows 平台兼容性、项目代码、环境配置、缺失依赖、外部服务、测试自身或暂时无法确定；
- 是否阻塞其他验证；
- 建议后续处理。

依赖失败项目的后续项目必须标记为 `BLOCKED` 并说明依赖关系。

## 8. Modification Policy

本流程的主要目的为验证，不是开发。除非项目文档明确要求生成或修改 Windows 本地配置：

- 不修改业务代码；
- 不修改 Linux 功能实现；
- 不为了通过测试修复代码；
- 不进行无关重构；
- 不升级依赖；
- 不改变项目架构。

Windows 工作副本可产生正常 build artifacts、dependency caches、test artifacts、logs、temporary files 和 machine-local configuration。

如果发现必须修改代码才能解决 Windows 问题，应记录问题、可能根因和建议修复位置，不要在本次验证任务中直接实施。

## 9. Phase 6：Update Linux Validation Documentation

验证结束后更新 Linux 项目已有验证文档，优先沿用原文件位置、结构、表格、状态标记和命名规则。已有合适文档时，不创建重复报告。

主验证文档至少包含以下信息：

### Validation Environment

- Windows version（如果可获得）；
- architecture；
- runtime/toolchain versions；
- Linux source branch；
- Linux source commit；
- 是否包含未提交修改；
- Windows 工作副本位置；
- 验证日期。

### Validation Results

逐项记录项目、状态、命令和简要结果。

### Errors

记录失败步骤、关键错误、最可能原因、是否 Windows-specific、是否阻塞其他验证和建议处理方式。不要把巨大完整日志复制到主文档；记录关键错误、相关 stack trace、日志文件路径和必要上下文即可。

### Not Executed / Blocked

每个未执行项目都要说明原因，例如：

- prerequisite failed；
- required external service unavailable；
- missing credential；
- hardware unavailable；
- test not applicable on Windows；
- project documentation does not currently define this validation；
- insufficient environment capability。

不能只写 `not tested`。

## 10. Final Review Checklist

- [ ] Windows 工作副本对应本次 Linux 源状态；
- [ ] 所有适用 Windows 验证项目都有明确状态；
- [ ] `FAIL` / `BLOCKED` / `NOT RUN` 均有原因；
- [ ] 没有将失败项目误记为成功；
- [ ] 没有遗漏明显的 Windows-specific 验证；
- [ ] Linux 项目没有产生验证范围之外的代码修改；
- [ ] Linux 验证文档已经更新；
- [ ] 最终 Linux Git diff 只包含预期修改。

## 11. Validation Report Template

后续验证可以使用以下结构写入已有结果文档：

```markdown
## Validation Environment

- Windows version:
- Architecture:
- Runtime/toolchain versions:
- Linux source branch:
- Linux source commit:
- Linux working tree changes included: yes/no
- Windows workspace:
- Validation date:

## Validation Results

| Scope | Category | Command | Working directory | Status | Summary |
|---|---|---|---|---|---|
| ... | Required/Applicable/Not applicable | `...` | `...` | PASS/FAIL/BLOCKED/NOT RUN/NOT APPLICABLE | ... |

## Errors

| Step | Error summary | Classification | Blocks other validation | Follow-up |
|---|---|---|---|---|
| ... | ... | ... | yes/no | ... |

## Not Executed / Blocked

- Item: reason and dependency.
```

## 12. Final Report Requirements

最终报告必须说明：

1. Linux branch、commit 和 working tree 状态；
2. Windows 同步结果；
3. PASS 项目；
4. FAIL 项目；
5. BLOCKED / NOT RUN 项目及原因；
6. Windows 平台问题；
7. 更新的 Linux 文档；
8. 需要后续开发任务处理的问题。

不能只报告“验证完成”。