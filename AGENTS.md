# Repository Governance

This repository uses a dual-owner development model:
- Linux = Cross-platform Owner
- Windows = Windows Platform Owner

Ownership covers design, implementation, validation, issue triage, and follow-up within the relevant platform boundary. Neither platform should duplicate the other owner's work unless required to unblock development.

## Canonical project state

The canonical project state is the current Git repository state, committed project documentation, current Plan/task state, and recorded platform validation results. No machine-local workspace is the sole source of truth. Linux and Windows workspaces are execution environments for their ownership scopes. Changes from either owner must be integrated into the canonical repository before they are considered project state.

## Ownership

Linux owns shared architecture, cross-platform business logic, shared libraries and APIs, protocols, data models and persistence formats, platform-neutral behavior, cross-platform refactoring, shared tests, and primary architecture documentation.

Windows owns Windows-specific implementation, native APIs, filesystem/process behavior, GUI behavior, services, registry and PowerShell integration, permissions, packaging/installers/signing, Windows configuration, compatibility fixes, and Windows runtime/GUI validation. Windows is a platform owner, not merely a validation environment, and may modify production code within this boundary.

If a Windows issue requires changing shared architecture, APIs, protocols, data models, or platform-neutral behavior, record CROSS_PLATFORM_CHANGE_REQUIRED and hand the shared change to Linux. A small shared-code adjustment that preserves an existing abstraction must be marked CROSS_PLATFORM_REVIEW_REQUIRED.

## Handoff and validation

Prefer batched work:

Linux cross-platform batch -> targeted Linux verification -> READY_FOR_WINDOWS handoff -> Windows implementation and targeted verification -> canonical integration -> Linux follow-up only when required.

Use WINDOWS_WORK_PENDING, WINDOWS_VERIFICATION_PENDING, and WINDOWS_BLOCKING with their documented meanings. Validation is risk-based and incremental: Targeted -> Module -> Subsystem -> Full. Use PASS, FAIL, BLOCKED, NOT_RUN, and NOT_APPLICABLE; never claim unperformed platform validation.

Computer Use or GUI automation failure is not product failure. After limited retry, mark the affected check BLOCKED with blocker COMPUTER_USE_UNAVAILABLE, continue independent checks, and add a manual validation item.

## Documentation routing

- Ownership and handoff: docs/development/platform-ownership.md
- Unified validation policy: docs/validation/validation-policy.md
- Current handoff: docs/status/platform-handoff.md
- Windows history: docs/validation/windows-validation-history.md
- Architecture: docs/architecture/overview.md
- Audit routing: docs/review/code-audit-guidelines.md
- Repeatable procedures: .agents/skills/

Keep this file limited to stable repository rules and routing. Inspect the current repository before acting, prefer the smallest correct change, avoid unrelated refactoring and unjustified dependencies, review the final Git diff, and keep status claims traceable to evidence.
