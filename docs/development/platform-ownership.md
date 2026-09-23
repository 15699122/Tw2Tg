# Platform Ownership and Handoff

## Model

Linux is the Cross-platform Owner. Windows is the Windows Platform Owner. Ownership includes design, implementation, validation, issue triage, and follow-up. The canonical project state is Git repository state plus committed documentation, Plan/task state, and recorded validation results. A local Linux or Windows directory is never the sole source of truth.

## Linux responsibility

Linux owns shared architecture, cross-platform core and business logic, shared APIs and protocols, persistence/data models, platform abstractions, shared dependency decisions, cross-platform tests, and shared documentation. A change that should have the same meaning on both platforms normally belongs to Linux.

## Windows responsibility

Windows owns native APIs, process and filesystem semantics, registry, services, permissions, PowerShell/shell integration, Windows GUI and dialogs, application lifecycle, packaging/signing/installers, Windows configuration, compatibility fixes, Windows-specific sidecars, and Windows validation.

Windows may modify production code within this boundary. The old rule "Windows only validates and never changes production code" is retired.

## Boundary routing

Ask in order:
1. Is this shared behavior, contract, schema, data model, or architecture? Route to Linux.
2. Is the cause a Windows API, runtime, GUI, filesystem, installer, process, or packaging behavior? Route to Windows.
3. Does a Windows fix require a shared change? Linux owns the shared change; Windows owns the adapter/integration.

A small shared adjustment that preserves an existing abstraction is allowed only with CROSS_PLATFORM_REVIEW_REQUIRED. A change to shared API, protocol, schema, data model, architecture, or cross-platform semantics is CROSS_PLATFORM_CHANGE_REQUIRED and must return to Linux.

## Handoff states

Use CROSS_PLATFORM_IN_PROGRESS, READY_FOR_WINDOWS, WINDOWS_IN_PROGRESS, WINDOWS_WORK_PENDING, WINDOWS_VERIFICATION_PENDING, WINDOWS_PASS, WINDOWS_FAIL, WINDOWS_BLOCKED, CROSS_PLATFORM_CHANGE_REQUIRED, and CROSS_PLATFORM_REVIEW_REQUIRED.

A feature is not complete merely because Linux implementation or Windows validation is complete. For applicable work the lifecycle is: cross-platform implementation -> cross-platform verification -> Windows implementation -> Windows verification -> canonical integration.

## Handoff record

Every handoff records source revision, completed cross-platform work, changed shared modules, Windows work required, validation required, known risks, expected behavior, relevant tests, priority, and deferred/manual GUI checks. Windows completion records implementation, automated PASS/FAIL/BLOCKED results, manual work, shared-code changes, unresolved issues, and final status.

Do not use unconditional mirroring to overwrite another owner's unintegrated work. Integrate effective changes into Git before the next synchronization.
