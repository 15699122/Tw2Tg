# Agent Tooling

Owner: Cross-platform Owner.
Status: `ACTIVE` — records how agents are wired into this repository and what has actually been verified. Repository rules live in the root `AGENTS.md`; this file records the tooling around them.

## Why this file is separate

`AGENTS.md` states what is true regardless of tool: who owns what, what counts as done, how failures are classified. Those rules must not be rewritten every time an agent CLI ships a patch release.

This file records the things that *do* change: where an agent looks for instructions, which entry points exist, and what has been verified versus assumed.

## Repository instruction files

| Path | Scope | Authority |
|---|---|---|
| `AGENTS.md` | Whole repository | Project invariants, ownership summary, capability policy, routing |
| `docs/AGENTS.md` | Everything under `docs/` | Documentation maintenance rules, source and state constraints |
| `.agents/skills/*/SKILL.md` | Task-triggered | Repeatable procedures; not a second copy of policy |

Both `AGENTS.md` files carry YAML frontmatter only where a tool requires it; neither contains a version pin, a model name, or a CLI subcommand.

## Entry points and discovery

Instructions are discovered differently per tool, and the differences matter when adding a new file.

- **Codex CLI** discovers `AGENTS.md` from the repository root toward the current working directory, and honours `AGENTS.override.md` in the same directories. Deeper files take precedence, subject to a combined size limit.
- **Cline** reads repository rules through its own rule discovery and also honours `AGENTS.md`.

Two consequences:

1. **A `windows/` subdirectory `AGENTS.md` would not automatically apply to Windows work.** Discovery follows the directory tree, not the operating system. Windows-specific constraints belong in [`../validation/windows.md`](../validation/windows.md) and [`windows-validation` skills](../../.agents/skills/windows-validation/SKILL.md), reached by explicit routing from the root file.
2. **`AGENTS.override.md` is not a way to relax root rules.** It is used here only to scope an experimental subtree, and it may not weaken the ownership boundary or the formal Git handoff rule.

## Skills

Three repeatable procedures live in `.agents/skills/`:

| Skill | Purpose |
|---|---|
| `cross-platform-handoff` | Prepare and close a formal Git handoff between platform owners |
| `project-code-audit` | Review a diff for correctness, ownership routing and validation gaps |
| `windows-validation` | Run or record Windows validation against an exact revision |

Each `SKILL.md` carries `name` and `description` frontmatter so a tool can discover and trigger it.

**Discovery is tool-specific and only partly verified here.** The frontmatter is present and this repository has no `.cline/` or `.codex/` directory. Whether a given agent build resolves `.agents/skills/` has **not** been verified in this session; see the table below. A thin generated entry point is the fallback if a tool cannot see `.agents/skills/`, rather than a second hand-maintained copy of the prose.

## Verified versus assumed

Nothing in this table is a claim that a tool will keep behaving this way.

| Fact | Status | How to confirm |
|---|---|---|
| Codex CLI present in this Linux environment at `3.0.65` | Verified 2026-10-01 | `codex --version` |
| Cline VS Code extension is the active Linux editor agent | Observed 2026-10-01 | Editor session |
| Codex discovers `AGENTS.md` from root toward CWD | Assumed from official documentation | Run a probe session in a subdirectory |
| Codex honours `AGENTS.override.md` precedence | Assumed from official documentation | Probe with a temporary override |
| Cline discovers `.agents/skills/` | **Not verified** | Trigger the skill in a Cline session |
| Windows-side Codex behaviour (sandbox, Computer Use, GUI) | **Not verified from Linux** | Windows Owner session |

Rows marked assumed or not verified must not be cited as evidence that an agent loaded a rule.

## Configuration boundary

Personal agent configuration is **not** part of the repository. `~/.codex/config.toml`, editor settings and MCP server definitions live outside Git and must not be copied into project documents.

Consequences for agents:

- Do not read a whole personal config into a document; it can contain credentials.
- Do not install, upgrade or downgrade an agent CLI to make a check pass.
- Do not disable a sandbox or an approval gate to obtain evidence. If a sandbox blocks a needed check, that is a `BLOCKED` result with the blocker named, not a reason to lower the guard.

Capability detection and the fallback rules are in the root `AGENTS.md`.

## Daily prompt

Stable rules belong in the repository, so a daily prompt only states the round's goal. The full templates remain in [`platform-handoff-prompts.md`](platform-handoff-prompts.md); the short form is:

```text
Take over the current cross-platform batch.

Read and follow the AGENTS.md in scope, plus the current plan,
validation policy and platform handoff.

Reconcile the previous owner's latest results, continue all executable
work inside the current ownership boundary, run the minimum necessary
validation, and update the handoff.

Do not take over work belonging to the other platform owner.

Report: completed, validation, unresolved, next owner.
```

## Maintaining this file

- Update it when an agent entry point, a skills path or a discovery rule changes.
- Record what was verified and when. Move an assumption to verified only with a probe result.
- Do not add model names, patch versions or CLI subcommands as project requirements.
- External material read to write this file is registered in [`../references/external-sources.md`](../references/external-sources.md).