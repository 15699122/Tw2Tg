# Repository Agent Instructions

## Development Model

This repository uses a dual-owner development model:

- Linux = `Cross-platform Owner`
- Windows = `Windows Platform Owner`

Linux owns shared and cross-platform implementation.

Windows owns Windows-specific implementation, integration, runtime behavior, GUI, packaging, and platform validation.

Detailed ownership rules: `docs/development/platform-ownership.md`

## Canonical Project State

The canonical project state is the Git repository state plus committed project documentation.

A machine-local working directory is not, by itself, the canonical project state.

Formal platform handoff must always identify:

- branch;
- source commit;
- handoff commit when applicable;
- uncommitted-state status;
- current owner.

Do not treat copied files as a formal handoff.

Capability never changes ownership. Obtaining Windows evidence through an approved channel does not transfer Windows implementation ownership to the Cross-platform Owner, and the Windows Owner's ability to edit shared code does not grant authority over a shared contract.

## Formal Platform Handoff

The normal production workflow is:

Linux working tree -> Git commit -> Git remote -> Windows working tree

and:

Windows working tree -> Git commit -> Git remote -> Linux working tree

The configured Git remote, normally GitHub, is the exchange point between platform owners.

Formal handoff must use Git history.

See: `docs/development/git-platform-handoff.md`

## Direct Linux -> Windows Sync

Direct filesystem synchronization is a temporary diagnostic or experimental path only. It must target a disposable or explicitly designated scratch workspace, must never overwrite the Windows Platform Owner's canonical working tree, and its results stay experimental until reproduced through the formal Git workflow.

The permitted paths, the conditions for using direct sync, and the reverse-sync prohibition are defined in [`docs/development/git-platform-handoff.md`](docs/development/git-platform-handoff.md).

## Workspace Model

Linux uses a Linux-native working tree; Windows uses an NTFS working tree such as `E:\Projects\<project>`. The two owner workspaces are independent Git working trees. The formal Windows repository is updated only through Git; only a designated scratch directory may receive direct Linux sync.

Concretely: `E:\Projects\<project>` via Git only, `E:\Scratch\<project>` may receive direct sync. See [`docs/development/git-platform-handoff.md`](docs/development/git-platform-handoff.md).

## Ownership Boundaries

The full responsibility split, the retired "Windows only validates" rule, and the routing questions are defined once in [`docs/development/platform-ownership.md`](docs/development/platform-ownership.md). Do not restate them here.

The two escalation markers that agents must apply are also defined there: `CROSS_PLATFORM_CHANGE_REQUIRED` for a shared contract, architecture, protocol, schema, or cross-platform behavior change, and `CROSS_PLATFORM_REVIEW_REQUIRED` for a small shared implementation change that preserves an existing abstraction.

## Environment Capability Detection

Agent capabilities are runtime properties, not repository properties. Detect before declaring work blocked:

- operating system and execution environment (native, VM, WSL, container);
- shell, sandbox, approval, and filesystem permissions;
- GUI, browser, and automation availability;
- MCP servers, network reachability, and required external services;
- the target platform and the artifact identity any evidence must bind to.

Do not infer capability from the agent role or from a previous session. Never assume a Linux session can perform Windows validation, and never assume a Windows session can render or automate a desktop application.

Record what was detected when a result depends on it.

## Execution Capability Policy

Before declaring a check `BLOCKED` or `NOT_RUN`:

1. Determine the capabilities actually available in the current environment.
2. Prefer the least invasive permitted execution path.
3. Use an alternative permitted path when it yields equivalent evidence for the same target.
4. Do not weaken product behavior, assertions, or acceptance criteria to accommodate an unavailable tool.
5. Mark `BLOCKED` only when no permitted path can produce the required evidence, and state which capability was missing.

An equivalent path may be recorded as `PASS` only when it proves the same acceptance target. A different check is a different check.

## Tooling Failure Policy

A tooling, sandbox, automation, MCP, network, or environment failure is not by itself a reason to change production code.

Before changing product code in response to a failure:

1. determine whether it reproduces independently of the tooling;
2. separate a product defect from an environment or tool defect;
3. record tooling failures separately from product results;
4. modify product code only when evidence indicates a product-level defect.

The detailed state model, evidence requirements, and reuse conditions live in [`docs/validation/validation-policy.md`](docs/validation/validation-policy.md).

## Batch Development

Prefer batch ownership:

Cross-platform batch -> handoff -> Windows batch -> handoff if required

Avoid unnecessary platform ping-pong.

A normal Windows validation requirement does not justify interrupting Linux development.

Only use `WINDOWS_BLOCKING` when Windows behavior must be known before cross-platform development can safely continue.

## Validation

Use risk-based incremental validation.

Default escalation: `Targeted -> Module -> Subsystem -> Full`

Do not run the full test suite after every change.

Detailed rules: `docs/validation/validation-policy.md`

## Computer Use Failure

GUI automation failure is an automation failure, not a product failure. Retry finitely, then record `BLOCKED` with blocker `COMPUTER_USE_UNAVAILABLE`, continue independent checks, and create or update the Manual Windows Validation Queue. Never record an unexecuted GUI check as PASS.

The full state model, evidence fields, and reuse conditions are defined once in [`docs/validation/validation-policy.md`](docs/validation/validation-policy.md).

## Handoff State

Current platform handoff state is maintained in: `docs/status/platform-handoff.md`

This file describes the current batch, not the complete historical development log. Validation history should remain in the validation/history documents.

## Documentation routing

Repository rules are defined once per topic. Follow the link rather than restating a rule locally.

- Ownership, boundary routing, escalation markers: docs/development/platform-ownership.md
- Git-based cross-platform handoff, revision, scratch boundary: docs/development/git-platform-handoff.md
- Daily owner prompts: docs/development/platform-handoff-prompts.md
- Validation scope, states, evidence, capability policy: docs/validation/validation-policy.md
- Current handoff (current batch only): docs/status/platform-handoff.md
- Current Windows queue (single source): docs/validation/windows-queue.md
- Windows execution recipes: docs/validation/windows.md
- Windows history and reconciliation: docs/validation/windows-validation-history.md
- Architecture entry: docs/architecture/overview.md
- Release policy / checklist / history: docs/release/release-policy.md, release-checklist.md, release-history.md
- Audit routing: docs/review/code-audit-guidelines.md
- External sources and license status: docs/references/external-sources.md
- Agent tooling entry points and versions: docs/development/agent-tooling.md
- Documentation governance plan: docs/development/documentation-governance-plan.md
- Repeatable procedures: .agents/skills/
- Document structure audit: scripts/docs-audit.mjs

## General Rules

All agents must:

- inspect the current repository before relying on previous assumptions;
- prefer the smallest correct change;
- avoid unrelated refactoring;
- respect platform ownership;
- use Git for formal handoff;
- distinguish implementation from verification;
- distinguish experimental direct-sync results from formal project state;
- use minimal necessary validation;
- review final `git diff` before finishing;
- never claim platform validation that was not actually performed.

Keep this file limited to stable repository rules and routing. Avoid unjustified dependencies and keep status claims traceable to evidence.

## Terminal Execution Rules

All shell commands must be non-interactive. Prefer commands that terminate deterministically and return stdout/stderr directly. Do not invoke interactive pagers (`less`, `more`), interactive Git commands, editors opened from Git, or commands requiring keyboard input.

For Git commands that may invoke a pager, disable it explicitly, for example:

- `git --no-pager show ...`
- `git --no-pager diff ...`
- `git --no-pager log ...`

Use bounded Git output when the full output is unnecessary, for example `git --no-pager log -20 --oneline`, `git --no-pager show --stat <commit>`, `git --no-pager diff --stat`, or `git --no-pager diff --name-status`. This reduces unnecessary output and terminal/model parsing pressure.

If a command appears complete in the terminal but the agent still reports it as running, treat that as a terminal integration issue rather than repeatedly rerunning the command.
