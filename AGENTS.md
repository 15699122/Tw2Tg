# Agent Instructions

## Project Context

Tw2Tg / XArchive 的主要开发环境是当前 Linux 项目目录。Windows 环境只用于 Windows 平台相关的构建、运行、测试和兼容性验证。

## Mandatory Cross-platform Workflow

For Linux ↔ Windows development and validation workflow, follow:

[`docs/development/cross-platform-validation.md`](docs/development/cross-platform-validation.md)

This workflow is mandatory for Windows-specific implementation and validation tasks.

## Development and Deferred Windows Validation Policy

本项目采用“批量开发、集中验证”的工作方式。Linux 是主要开发环境；Windows 仅用于无法在 Linux 环境充分完成的平台相关验证。

不得机械采用以下节奏：

```text
Feature A → Windows test → Feature B → Windows test → Feature C → Windows test
```

默认应采用：

```text
Feature A → Feature B → Feature C
→ 完成当前范围内所有可在 Linux 执行的开发
→ Linux verification
→ 汇总 Windows Validation Queue
→ Windows validation phase
```

除非 Windows 验证结果是继续开发的硬性前置条件，或用户明确要求立即验证，否则不得在 Linux 开发过程中提前切换到 Windows。

开发阶段完成一个功能后，应：

1. 完成功能实现；
2. 执行 Linux 适用的测试、静态检查、lint、formatter、build 或 regression verification；
3. 将需要 Windows 后续确认的内容加入累计的 Windows Validation Queue；
4. 继续下一个不依赖新 Windows 结果的开发项。

只有以下情况可以中断 Linux 开发并提前要求 Windows 验证：

- 后续设计依赖某个 Windows-specific 行为是否成立；
- Windows API、filesystem、process 或 installer 行为无法从代码/文档可靠判断；
- 关键兼容性假设若错误会使后续大量开发失效；
- 当前失败只能在 Windows 复现且阻塞继续开发；
- 用户明确要求立即执行 Windows 验证。

不要仅因为某功能最终需要 Windows 测试、可能存在跨平台问题或属于跨平台代码，就中断当前 Linux 开发阶段。

Windows 队列状态默认使用 `WINDOWS_VERIFICATION_PENDING`。仅当缺少 Windows 结果会阻止后续 Linux 实现可靠继续时，才使用 `WINDOWS_VERIFICATION_BLOCKING`。队列项至少记录：验证项、关联功能/修改、相关文件或模块、Windows 验证原因、精确验证行为、前置条件、预期结果、优先级及是否阻塞后续 Linux 开发。

进入 Windows 验证前，必须先结束当前 Linux development phase：当前 Plan 中所有不依赖 Windows 的开发项已完成，Linux 适用验证已完成，已知 Linux 错误已处理，Windows 队列已累计完整且没有遗漏明显平台要求。Windows 验证准备阶段必须基于最终 diff、当前 Plan、变更模块、Windows 代码路径、项目文档、构建/CI 配置和历史验证记录合并重复场景后统一规划。

最终 Windows handoff 应按 Build/Toolchain、Runtime、Filesystem、Integration、Packaging、Regression 分类，并为每项提供 ID、名称、目的、关联修改、前置条件、步骤/命令、预期结果、优先级和是否需要人工交互。

## Windows Validation Goal

对当前 Linux 项目的最新开发状态执行 Windows 平台验证：

1. 检查当前 Linux 项目及项目文档。
2. 将需要验证的项目内容单向同步到 Windows `E:` 盘对应项目目录。
3. 根据项目自身文档、配置和脚本确定 Windows 平台需要执行的验证项目。
4. 在 Windows 环境执行这些验证。
5. 收集并分析验证过程中出现的错误。
6. 将验证结果、错误信息以及未执行项目的原因更新到 Linux 项目中的相应验证文档。

## Source of Truth

当前 Linux 项目目录是以下内容的唯一主要事实来源：

- 源代码和项目状态；
- 项目文档；
- 最终验证记录。

Windows `E:` 盘项目目录只是 Windows 验证工作副本。

除验证结果文档外，不得将 Windows 工作副本中的代码反向同步到 Linux 项目。

## Phase 1 — Repository Investigation

在任何同步或验证之前，先检查 Linux 源项目。自动确定并记录：

- 当前 Git branch、commit 和 working tree 状态；
- 项目目录结构；
- 使用的语言、框架和构建系统；
- Windows 相关文档、脚本和配置；
- 测试、lint、typecheck、build、package 等命令；
- 已有的平台兼容性要求；
- 已有验证文档及其格式；
- `AGENTS.md` 或其他 agent/project instructions；
- Windows 验证依赖的软件、环境变量、外部服务和工具。

信息判断优先级：

1. `AGENTS.md` / 项目 agent instructions；
2. 项目文档；
3. CI、build、test 配置；
4. package/build scripts；
5. 当前代码实现。

不要凭空创造项目不存在的 Windows 验证要求。

## Phase 2 — Pre-sync Safety Check

同步方向必须是：

```text
Linux source → Windows E: validation workspace
```

同步前必须：

- 检查 Windows 目标目录是否存在；
- 检查目标目录中的未提交、人工创建或可能需要保留的文件；
- 不无条件删除未知文件；
- 不覆盖明显属于 Windows 本地配置、凭据、缓存或机器特定设置的内容；
- 根据 `.gitignore`、项目文档、构建配置和已有同步脚本决定同步范围。

除非项目文档明确要求，通常不得复制：

- `.git`；
- `node_modules`；
- Python virtualenv；
- Rust `target`；
- build/dist cache；
- IDE cache；
- 临时文件；
- secrets；
- machine-specific configuration。

如果项目已有正式的同步、checkout、worktree 或部署方式，优先使用已有方式，不自行实现另一套同步机制。

## Phase 3 — Sync Verification Workspace

同步完成后至少确认：

- 关键源代码已经更新；
- Windows 工作副本对应本次 Linux 源状态；
- 没有因路径分隔符、大小写、符号链接或权限造成明显缺失；
- Windows 本地专用配置未被意外覆盖。

如果可以可靠获得 Git commit/hash，记录：

- branch；
- commit；
- 是否包含未提交改动。

Linux 源项目包含未提交改动时，不得假装 Windows 验证对应一个纯 Git commit；必须明确记录该验证包含 working tree changes。

## Phase 4 — Determine Windows Validation Scope

读取项目文档和仓库配置，自行确定与当前项目实际相关的 Windows 验证范围。可能包括：

- dependency/environment setup；
- code generation；
- formatter check；
- lint；
- typecheck；
- unit tests；
- integration tests；
- Windows-specific tests；
- build；
- packaging；
- application startup；
- CLI behavior；
- filesystem/path behavior；
- subprocess/sidecar behavior；
- network/service integration；
- installer/package validation。

验证前形成实际清单，并标记：

- `Required`：项目文档明确要求；
- `Applicable`：根据当前项目配置应执行；
- `Not applicable`：当前 Windows 环境或项目不适用。

只执行当前项目实际相关的项目。

## Phase 5 — Execute Windows Validation

在 Windows 工作副本中：

- 使用项目已有命令和脚本；
- 优先使用项目规定的软件包管理器和工具链；
- 不因命令失败而随意更换工具或改变项目配置；
- 不为了让测试通过而修改功能代码；
- 不跳过失败项目并将其标记为成功。

每个验证项目记录：

- 验证项目名称；
- 实际执行命令；
- 工作目录；
- 结果；
- 必要的版本信息；
- 关键输出或错误摘要。

结果必须使用以下状态之一：

- `PASS`
- `FAIL`
- `BLOCKED`
- `NOT RUN`
- `NOT APPLICABLE`

验证失败时：

1. 保存关键错误信息；
2. 分析最可能原因；
3. 判断其属于 Windows 平台兼容性、项目代码、环境配置、缺失依赖、外部服务、测试自身或暂时无法确定；
4. 在合理情况下继续执行不依赖该失败项的其他验证。

后续验证依赖此前失败项目时，必须标记为 `BLOCKED` 并说明依赖关系。非致命项目失败不得自动停止全部验证。

## Modification Policy

本任务主要目的是验证，不是开发。除非项目文档明确要求生成或修改 Windows 本地配置：

- 不修改业务代码；
- 不修改 Linux 项目的功能实现；
- 不为了通过测试自行修复代码；
- 不进行无关重构；
- 不升级依赖；
- 不改变项目架构。

允许 Windows 工作副本产生正常的 build artifacts、dependency caches、test artifacts、logs、temporary files 和 machine-local configuration。

如果必须修改代码才能解决 Windows 问题，不要直接在本次验证任务中实施；应在验证文档记录问题、可能根因和建议修复位置。

## Phase 6 — Update Linux Validation Documentation

验证结束后，将结果写回 Linux 项目已有的相应验证文档，并优先沿用其：

- 文件位置；
- Markdown 结构；
- 表格格式；
- 状态标记；
- 命名规则。

已有合适文档时不要创建重复验证报告。验证记录至少包含：

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

逐项记录项目、状态、执行命令和简要结果。

### Errors

逐项记录失败步骤、关键错误、最可能原因、是否 Windows-specific、是否阻塞其他验证和建议后续处理方式。不要把巨大完整日志复制进主文档；优先记录关键错误、相关 stack trace、日志路径和必要上下文。

### Not Executed / Blocked

所有未执行项目必须说明原因，例如 prerequisite failed、外部服务不可用、缺失凭据、硬件不可用、不适用于 Windows、项目文档未定义该验证或环境能力不足。不能只写 `not tested`。

## Final Review

完成后检查：

1. Windows 工作副本对应本次 Linux 源状态；
2. 所有适用 Windows 验证项目都有明确状态；
3. `FAIL`、`BLOCKED`、`NOT RUN` 均有原因；
4. 没有把失败项目误记为成功；
5. 没有遗漏明显的 Windows-specific 验证；
6. 没有对 Linux 项目做验证范围之外的代码修改；
7. Linux 验证文档已更新；
8. 最终 Linux Git diff 只包含本任务预期的修改。

## Final Response

最终报告必须明确说明：

1. Linux branch、commit 和 working tree 状态；
2. Windows 同步结果；
3. `PASS` 项目；
4. `FAIL` 项目；
5. `BLOCKED` / `NOT RUN` 项目及原因；
6. 发现的 Windows 平台问题；
7. 更新的 Linux 文档；
8. 需要后续开发任务处理的问题。

不能只报告“验证完成”。任何失败、阻塞或未执行项目都必须明确指出。

## Related Documentation

- Windows 验证规范：`docs/validation/windows.md`
- 当前项目 Windows 验证结果：`docs/development/windows-validation.md`