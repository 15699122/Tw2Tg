# Continuous goal execution

Owner: Cross-platform Owner. Status: ACTIVE.

## Scope and sources

This procedure governs long-running agent goals, not product acceptance rules.
The applicable Plan owns scope and dependencies; [platform handoff](../status/platform-handoff.md)
owns the current execution cursor. [Validation policy](../validation/validation-policy.md)
and [platform ownership](platform-ownership.md) remain authoritative.

## Execution loop

1. Inspect Git, the Plan and current cursor before trusting a previous summary.
2. Classify remaining work as executable, dependency-waiting, decision-required,
   or Windows-owned. Record precise decisions; do not invent contracts.
3. Select a production milestone and implement it through small reviewable batches.
4. Validate each batch at the minimum justified scope, then continue the next
   executable batch. A passing local test is not a reason to end the goal.
5. Record milestone evidence and the next concrete call site or acceptance check
   in the handoff. Keep history out of the current cursor.
6. When one item is blocked, continue independent permitted work.

Primitive hardening is a subtask, not a substitute for production integration.
After two consecutive diagnostic/test-only batches, reassess the main call chain
and explain why integration cannot yet proceed. Do not use increasing test counts
as evidence that a production gate has closed.

## Stop conditions

End only when the goal is verified, every remaining executable path is concretely
blocked, a user decision blocks all remaining work, or the runtime actually
requires interruption. Preserve approval and safety boundaries. Never invent a
budget limit or claim an unobserved runtime interruption.

On interruption, report unfinished scope, actual stop reason, branch/source HEAD,
dirty state, evidence and the exact next action. A completed micro-batch alone
is not a stop reason. Do not commit/push without the applicable authorization.

## Stable evidence capture and local tooling

Use non-interactive checks with separate logs and completion/exit-code records.
Bind evidence to source HEAD and dirty state. Missing completion evidence is not
PASS. Reconcile completed output before rerunning a check after terminal capture
failure; do not mask an earlier failure with a later successful command.

Prefer serial Cargo builds sharing one target directory, and parallel independent
reads or non-conflicting checks. This procedure does not install a validation
runner, configure delegation, change approvals or enable background execution.

Verify the installed agent's `/goal`, context compaction, checkpoints and approval
behavior before relying on them. No automatic continuation is assumed. Keep
personal settings outside Git; do not disable sandboxing or broaden permissions.
Any future ignore file must preserve source, dependency manifests, lockfiles,
fixtures and required evidence. Record detected capabilities in
[agent tooling](agent-tooling.md), not unsupported version-specific claims.

## Goal prompt

```text
Continue the current non-Windows goal using continuous-goal-execution.md.
Inspect Git, the Plan and platform handoff; build the remaining dependency matrix.
Use approved decisions and advance production milestones through multiple batches.
After targeted validation, continue the next executable task instead of ending
because one primitive or test passed. Record blockers and continue independent work.
Preserve ownership, approvals, user changes and fail-closed acceptance gates.
Stop only under the procedure's stop conditions. If interrupted, save the exact
execution cursor and report the actual reason; do not claim overall completion.
```