# Cross-platform Handoff

Read AGENTS.md, docs/development/platform-ownership.md, docs/status/platform-handoff.md, and the current Git diff. Confirm the source revision, working-tree scope, completed cross-platform work, changed shared modules, Windows work and validation required, risks, expected behavior, relevant tests, priority, and deferred GUI/manual items.

Use READY_FOR_WINDOWS when the Linux batch is ready. Do not mirror over unintegrated Windows work. When Windows finds a shared contract, architecture, schema, data-model, or platform-neutral behavior issue, record CROSS_PLATFORM_CHANGE_REQUIRED and return the shared change to Linux. Use CROSS_PLATFORM_REVIEW_REQUIRED for a small shared adjustment that preserves an existing abstraction. Integrate effective changes into the canonical Git repository before closing the handoff.
