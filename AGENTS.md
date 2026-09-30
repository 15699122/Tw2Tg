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

## Formal Platform Handoff

The normal production workflow is:

Linux working tree -> Git commit -> Git remote -> Windows working tree

and:

Windows working tree -> Git commit -> Git remote -> Linux working tree

The configured Git remote, normally GitHub, is the exchange point between platform owners.

Formal handoff must use Git history.

See: `docs/development/git-platform-handoff.md`

## Direct Linux -> Windows Sync

Direct filesystem synchronization from Linux to Windows is allowed only as a temporary diagnostic or experimental path.

It must not be used as the normal ownership handoff mechanism.

Direct sync must target a disposable or explicitly designated scratch workspace. It must not overwrite the Windows Platform Owner's canonical working tree.

Direct-sync results are considered experimental until reproduced or integrated through the formal Git workflow.

Use direct sync only when:

- a fast Windows experiment is needed;
- committing an incomplete change would be undesirable;
- Windows behavior is required to unblock further design;
- the experiment can be safely discarded.

Never treat a direct-sync workspace as the source of truth.

## Workspace Model

Linux:

- uses a Linux-native filesystem working tree;
- owns cross-platform development;
- should not perform normal development from `/mnt/<drive>`.

Windows:

- uses an NTFS working tree such as `E:\...\project`;
- owns Windows platform development;
- should not use the WSL repository itself as its normal working tree.

The two owner workspaces are independent Git working trees.

`E:\Projects\<project>` is updated only through Git. Only `E:\Scratch\<project>` may receive direct Linux sync.

## Ownership Boundaries

### Cross-platform Owner

Linux owns:

- shared architecture;
- shared APIs and contracts;
- cross-platform business logic;
- shared persistence/data models;
- protocol definitions;
- platform abstractions;
- cross-platform tests.

### Windows Platform Owner

Windows owns:

- Windows-native integration;
- Windows filesystem/process behavior;
- Windows-specific GUI behavior;
- Windows services/registry integration;
- Windows configuration;
- Windows compatibility fixes;
- installer/packaging;
- Windows-specific tests and validation.

If Windows discovers that a fix requires changing a shared contract, architecture, protocol, schema, or cross-platform behavior, mark `CROSS_PLATFORM_CHANGE_REQUIRED`.

If Windows makes a small shared implementation change that does not alter the shared contract, mark `CROSS_PLATFORM_REVIEW_REQUIRED`.

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

Computer Use / GUI automation failure is an automation failure, not a product failure.

If Computer Use is unavailable:

- perform limited retry;
- mark affected tests `BLOCKED`;
- use blocker `COMPUTER_USE_UNAVAILABLE`;
- continue independent validation;
- create or update the Manual Windows Validation Queue.

Never mark an unexecuted GUI test as PASS.

## Handoff State

Current platform handoff state is maintained in: `docs/status/platform-handoff.md`

This file describes the current batch, not the complete historical development log. Validation history should remain in the validation/history documents.

## Documentation routing

- Ownership and handoff: docs/development/platform-ownership.md
- Git-based cross-platform handoff: docs/development/git-platform-handoff.md
- Daily owner prompts: docs/development/platform-handoff-prompts.md
- Unified validation policy: docs/validation/validation-policy.md
- Current handoff: docs/status/platform-handoff.md
- Windows history: docs/validation/windows-validation-history.md
- Architecture: docs/architecture/overview.md
- Audit routing: docs/review/code-audit-guidelines.md
- Repeatable procedures: .agents/skills/

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
