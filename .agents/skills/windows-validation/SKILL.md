# Windows Validation

Read AGENTS.md, docs/development/platform-ownership.md, docs/development/git-platform-handoff.md, docs/validation/validation-policy.md, docs/validation/windows.md, docs/validation/windows-queue.md, and docs/status/platform-handoff.md before acting.

Select the smallest risk-appropriate scope: Targeted, Module, Subsystem, or Full. Record the validation revision so results bind to the commit they tested, per docs/development/git-platform-handoff.md. Reuse prior PASS only when related code, dependencies, contracts, and environment remain unchanged; otherwise mark REVALIDATION_REQUIRED. Record PASS, FAIL, BLOCKED, NOT_RUN, or NOT_APPLICABLE with commands, evidence, reasons, and follow-up.

Windows-specific implementation and fixes are allowed within the Windows ownership boundary. Shared contract or architecture changes are CROSS_PLATFORM_CHANGE_REQUIRED. GUI automation failure is not product failure: retry finitely, then use BLOCKED with COMPUTER_USE_UNAVAILABLE, continue independent checks, and create a concrete manual validation item.
