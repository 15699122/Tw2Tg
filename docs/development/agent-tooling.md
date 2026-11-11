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

## Terminal execution

The repository-wide non-interactive shell and Git output rules are authoritative in the root [`AGENTS.md`](../../AGENTS.md#terminal-execution-rules). In particular, disable Git pagers explicitly, bound output when a summary is sufficient, and do not rerun a command merely because terminal integration failed to report completion when its output indicates it finished.

## Daily prompt

### Observed Linux continuation capabilities — 2026-11-08

The current session is WSL2 Linux x86_64, kernel
`6.18.58.1-microsoft-standard-WSL2`, with Rust/Cargo `1.98.0`, Node `26.7.0`
and Python `3.14.4`. The user systemd manager reports `running` and
`systemd-run` is installed. Repository `.venv/bin/python` has pytest `9.1.1`;
the focused Sidecar suite executed successfully. `rg` is absent, so use bounded
`grep`/Python source inspection rather than installing tooling.

The shell integration sometimes reports completion unavailable even when the
captured output has returned to a prompt. Do not repeat a state-changing operation
on that basis. Validation logs capture the real child return code directly and
require a nonzero selected test count. Background-command completion and subagent
execution are available, but a tool follow-up can abort an active subagent; check
its terminal status and working changes before resuming. No `/goal` auto-resume,
Windows desktop, GUI automation, or Windows execution capability is inferred from
the presence of these tools.

For multi-batch goals use [continuous goal execution](continuous-goal-execution.md).
The installed `/goal` dispatch/auto-resume behavior has not been verified here;
do not infer it from the command name. Context compaction, checkpoints and approval
settings must likewise be detected in the installed runtime before being relied
on. This documentation change does not configure any of those features.

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

## Project memory and session resume

This repository does not use a separate `memory-bank/` directory. The root `AGENTS.md` session-resume section is the entry point. The table below is a **navigation index** — it says where a fact is recorded, so a session does not have to search. It does not create a second set of rules and does not override the authority each document already has.

| Fact | Where it is recorded |
|---|---|
| Goal, product scope, stated limitations | `README.md` |
| Stable architecture, invariants, ADRs | `docs/architecture/` |
| Implementation state and known limits | `docs/development/status.md` |
| Scope, dependencies and acceptance for active work | the active `docs/development/*-plan.md` |
| Current batch: branch, revision, owner, next action | `docs/status/platform-handoff.md` |
| Validation rules, states and required evidence | `docs/validation/validation-policy.md` |
| Shared/Linux validation results and evidence pages | `docs/validation/` (e.g. `large-file-capability-refactor-evidence.md`, `shared-state-reconciliation-evidence.md`) |
| Windows queue and Windows validation history | `docs/validation/windows-queue.md`, `docs/validation/windows-validation-history.md` |
| Released scope per version | `docs/release/` |
| Durable rules | `AGENTS.md`, `docs/AGENTS.md` |

Reading order for a new session is defined in the root `AGENTS.md`. Resume from the cursor, then open the Plan it names, then only the documents the task needs. A full load of the handoff history, the whole Windows queue and all module documents is not a session-start requirement.

### What to persist, and when

Persist a fact when a later session cannot cheaply re-derive it: a decision and its reason, a scope change, a blocker plus its prerequisite, an authorization boundary, the location of a validation result, or the exact next action and its call site.

Do not persist transient working state: raw command output, intermediate or superseded counts, superseded plans, or anything already readable from `git diff` or the current code. Never write credentials, tokens, cookies, secrets or personal agent configuration into repository documents.

Validation evidence is not derivable from `git diff`: keep the result, its source revision and its location, and link to the evidence page rather than restating the measurements.

Update documents only when their own fact changes, at milestone boundaries or when drift is found:

| Event | Update |
|---|---|
| A milestone closes | the active Plan status, the evidence page slot, the handoff cursor |
| Revising a shared decision | the ADR in `docs/architecture/decisions.md`, then the routing documents that cite it |
| Capability or limitation changes | `docs/development/status.md` |
| A Linux/shared validation round completes | the relevant evidence page; the cursor only summarises and links |
| A Windows result arrives | `docs/validation/windows-queue.md` plus history; the cursor may summarise it and must link the evidence |
| Batch closed and committed | the handoff cursor, with the new source revision |

Rewriting every memory document each round is not required and is not expected. A document whose fact did not change stays untouched.

### Session handoff versus platform handoff

`/newtask`, `/smol`, context compaction, a model switch and a subagent transfer move context between sessions of the same owner. They must not be recorded as a formal platform handoff, must not overwrite `docs/status/platform-handoff.md` with an untracked summary, and must not imply that uncommitted work is integrated.

Before compacting or opening a new task, record in the durable set: the cursor in effect, completed scope, remaining scope, blockers with prerequisites, and the next concrete action. After resuming, re-verify branch, HEAD, working-tree state, applicable `AGENTS.md`, current approval scope and runtime capability before acting. A previous session's Git authorization does not carry over.

Formal Linux/Windows handoff still requires a Git commit, a push and the revision recorded in the handoff, per [git-platform-handoff](git-platform-handoff.md).

### Detecting stale memory

A stale statement is a documentation defect, not a validation result. Resolve a conflict by asking which document owns that kind of fact, then trusting that document and correcting the one that drifted — do not invent a global ranking between documents that own different things:

- a claim about current source behaviour is settled by reading the code, not by any document;
- a claim about scope or acceptance is settled by the active Plan;
- a claim about what was executed is settled by the evidence page for that revision;
- a claim about the current batch is settled by the handoff cursor.

Correct the document that is wrong; do not add a second authoritative copy, and do not silently delete an older revision-bound record. When the correct value cannot be established from repository evidence, mark it `NEEDS_REVIEW` and name the missing evidence instead of guessing.

## Maintaining this file

- Update it when an agent entry point, a skills path or a discovery rule changes.
- Record what was verified and when. Move an assumption to verified only with a probe result.
- Do not add model names, patch versions or CLI subcommands as project requirements.
- External material read to write this file is registered in [`../references/external-sources.md`](../references/external-sources.md).