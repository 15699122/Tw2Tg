# Code Audit Guidelines

## Route findings by ownership

- Shared/cross-platform finding: Linux Cross-platform Owner.
- Windows-specific finding involving native API, filesystem/process semantics, GUI, registry, services, packaging, or installer: Windows Platform Owner.
- Unclear boundary: NEEDS_VERIFICATION; identify the behavior and evidence needed before routing.

## Audit rules

Review the final Git diff, changed contracts and schemas, dependency impact, platform branches, error handling, secrets/privacy boundaries, persistence compatibility, tests, and documentation. Separate implementation claims from validation evidence. Do not infer native GUI, real integration, installer, signing, or external-service acceptance from unit tests, mocks, startup, or smoke tests.

For shared changes, check Linux and Windows impact and require cross-platform review. For Windows changes, verify that the shared contract remains unchanged unless the finding is explicitly escalated as CROSS_PLATFORM_CHANGE_REQUIRED.

Every finding records severity, file/module, evidence, ownership route, recommended action, and validation needed.
