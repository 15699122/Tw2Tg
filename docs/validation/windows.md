# Windows 平台验证规范

> 本文是 Windows 验证的执行规范和报告模板。当前项目的具体验证结果继续记录在 [`../development/windows-validation.md`](../development/windows-validation.md)。

当前 WDIO 后续 Windows 执行清单见 [`windows-wdio-handoff.md`](windows-wdio-handoff.md)。

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

对于 `BLOCKED` 项目，不得只写 `not tested`。必须记录阻塞前置、跳过原因，并提供可在前置满足后执行的手工步骤；本轮统一手工步骤见 [`../development/windows-validation.md`](../development/windows-validation.md) 的“本轮最终收口：BLOCKED / NOT RUN 手工验证”章节。

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

### 7.1 Windows Codex 执行顺序

Windows 验证 Agent 按以下顺序执行：读取项目文档和 Linux 验证结果 → 构建 E2E artifact → 执行共享 `@wdio/tauri-service` → 执行 Windows-only WDIO → 自动诊断并在允许范围内低风险修复 → 重新执行相关测试 → 对 WDIO 无法稳定覆盖的系统级场景使用 Computer Use → 汇总并回写结果。确定性自动化测试优先于视觉 GUI 自动化，局部失败不得无条件终止无关测试。

优先级为：

1. 静态、lint、typecheck、Rust compile/check、unit、frontend unit 和 integration；
2. 不依赖原生 Windows 行为的 WDIO Browser Mode；
3. 共享 `@wdio/tauri-service` Native E2E，默认 `driverProvider: embedded`，仅在已有 fallback 配置且有驱动层证据时尝试 external provider；
4. Windows-only WDIO：路径、WebView2、文件系统、IPC、托盘、通知、Registry、安装/卸载、权限和平台快捷键；
5. Computer Use：原生文件选择器、系统通知、托盘菜单、安装器、原生窗口、DPI/多显示器、拖放、WDIO 无法稳定覆盖的系统组件和视觉验收。

Browser Mode 已充分覆盖的 UI 逻辑不重复使用 Computer Use；不得为了测试失败临时改写项目测试架构。

每项记录：

- 项目名称；
- 实际命令；
- 工作目录；
- 结果；
- 相关版本；
- 关键输出或错误摘要。

平台验证项目允许的结果状态：

```text
PASS
FAIL
BLOCKED
NOT RUN
NOT APPLICABLE
```

上述是 Windows 队列/验证项目状态。单个测试用例应使用 `docs/development/testing.md` 定义的细粒度状态，包括 `PASS_FLAKY`、`FAIL_PRODUCT`、`FAIL_TEST`、`BLOCKED_ENV`、`BLOCKED_AUTOMATION`、`SKIPPED_PLATFORM` 和 `NEEDS_REVIEW`；不得用单独的 `FAIL` 或 `BLOCKED` 隐藏具体分类。

### 7.2 失败分类、诊断与重试

失败后先分类再采取动作：

- `FAIL_PRODUCT`：相同输入稳定复现，业务、Rust/backend、frontend、IPC、状态机、数据读写或 Windows 实现不符合需求；
- `FAIL_TEST`：selector、fixture、mock、expectation、初始化、隔离或 WDIO 配置问题；
- `BLOCKED_ENV`：依赖、WebView2、Node/Rust 工具链、驱动、网络、外部服务或凭据缺失；
- `BLOCKED_AUTOMATION`：WDIO service/session、embedded/external driver 或 Computer Use 不可用，且没有产品缺陷证据；
- `NEEDS_REVIEW`：需求、文档、测试预期和实现冲突，无法在当前任务中裁决；
- Windows-only 用例在 Linux 上：`SKIPPED_PLATFORM`。

诊断顺序为：测试步骤和断言 → WDIO 输出 → frontend console → Tauri IPC/invoke → Rust/backend 日志 → Windows 系统错误 → 测试代码 → 产品代码 → 环境和自动化基础设施。仅对真正可能 transient/flaky 的问题进行局部重试，最多 2 次、总执行最多 3 次，建议退避约 1 秒和 3 秒；编译错误、确定性断言失败、panic、schema mismatch、权限错误和稳定 frontend exception 不自动重试。重试成功记录 `PASS_FLAKY`，修复后通过记录 `PASS_AFTER_FIX` 或 `PASS_AFTER_TEST_FIX`，连续稳定失败则停止重试并归类。

单个环境或自动化问题只暂停依赖该组件的项目，继续执行独立测试。若 Computer Use 暂时不可用，必须记录 `BLOCKED_AUTOMATION` 和详细人工验证步骤，不代表产品失败。

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

失败用例至少收集：`case_id`、test name、platform、layer、command、timestamp、expected、actual、相关 WDIO 输出、frontend console、backend/Rust log 和 stack trace（如有）。GUI 问题按需保存 failure screenshot、当前窗口信息和 URL/route。

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
- [ ] `BLOCKED_AUTOMATION` 用例都有完整人工验证步骤；
- [ ] 环境问题、自动化基础设施问题、测试缺陷和产品缺陷已分开；
- [ ] 确定性失败没有被无意义重复重试；
- [ ] 单测试结果与 Windows 队列状态没有混用。

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

同时提供结构化最终汇总，至少包括：

```json
{
  "summary": {
    "total": 0,
    "pass": 0,
    "pass_flaky": 0,
    "pass_after_fix": 0,
    "fail_product": 0,
    "fail_test": 0,
    "blocked_env": 0,
    "blocked_automation": 0,
    "skipped_platform": 0,
    "needs_review": 0
  },
  "regression_status": "PASS_WITH_ISSUES",
  "remaining_windows_validation": [],
  "manual_tests_required": [],
  "development_followups": [],
  "environment_issues": [],
  "flaky_tests": []
}
```

`regression_status` 必须反映实际证据；不能因部分项目未执行、被平台跳过或自动化阻塞而宣称全量 PASS。

不能只报告“验证完成”。