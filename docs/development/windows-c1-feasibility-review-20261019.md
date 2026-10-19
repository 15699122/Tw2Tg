# Windows C1 feasibility review — 2026-10-19

**Type: bounded cross-platform feasibility review request, not a full
implementation handoff and not `READY_FOR_WINDOWS`.**

This asks the Windows Platform Owner for bounded feasibility conclusions on five
native-boundary questions before the shared production lifecycle port is frozen.
It is issued under the approved plan's option 2: continue all independent
Linux/shared work now, and pause only the specific interface points that cannot
be decided on Linux. It does **not** block Linux development on a global
"Windows decision" and does **not** authorize production v2 or D1.

Input source: branch `codex/linux-c1-shared-prereq-20261019`, working tree based
on `d0c034fee69ca3bda7f784186f062985b7a0be95` plus the uncommitted Linux changes
below. Owner during review: Cross-platform Owner.

## What Linux already completed independently (source of these questions)

These Linux/shared steps do **not** depend on any Windows conclusion and are
already implemented and Linux-tested in this tree:

1. **Atomic no-replace mover port.** `FileStore` owns an injected
   `Box<dyn NoReplaceMover + Send>`; production call sites build it through
   `crate::executor::production_file_store`, the single injection point for a
   future Windows mover. Non-Unix stays fail-closed.
2. **Writer-role identity single source.** Shared `writer_lifecycle::WriterRole`
   (`Sidecar`/`Aria2Primary`/`Aria2Refresh`, stable labels `sidecar`/`aria2-0`/
   `aria2-1`); successor recovery enumerates exactly `WriterRole::ALL`. Role is a
   logical identity, **not** a recovery credential.
3. **Owner-bearing startup failure (Sidecar).** New
   `SidecarStartupFailure { error, owner }` mirrors the existing
   `Aria2StartupFailure`: a failed contained spawn or handshake retains the
   supervisor owner so cleanup can be retried; the owner is cleared only after a
   successful shutdown. `require_ready_v2` keeps the legacy error surface; the
   production contained path uses `require_ready_v2_owned`. The Desktop factory
   fails closed with `SIDECAR_CLEANUP_INCOMPLETE` when the domain cannot be
   proven drained, so no successor writer or publication is admitted on an
   unretired domain.
4. **Role/label tests** added to shared Workflow.

Linux evidence at this tree: `cargo fmt --all -- --check` PASS; Workflow lib
40/40 (incl. role identity); sidecar lib 16/16; sidecar `--all-targets` compile
PASS; Download lib 53/53; Desktop `executor::` 109 pass/6 ignored and
`production::` 18 pass; `git diff --check` PASS. The real-cgroup owned-handshake
test is added but `#[ignore]` (needs an explicitly delegated disposable cgroup),
so it is not claimed as a PASS here.

## Shared contract draft (platform-neutral behavior for both sides)

The port must bind one claimed attempt plus one `WriterRole` to:

1. **Explicit provisioning** — a writer never runs before its containment
   boundary exists; missing capability is a fail-closed refusal.
2. **Containment-before-spawn with owned pipes/environment** — the owner is
   retained across a partially failed start.
3. **Drain/termination evidence** — root exit, pipe-reader EOF, and full-domain
   drain are distinct facts; none substitutes for another; Drop is never proof.
4. **Owner-bearing failure** — cleanup failure keeps the owner for retry; a
   successor writer or publication is not admitted until cleanup succeeds.
5. **Crash-surviving, validated successor recovery identity** — not a PID, port,
   or named object alone; recover-before-create for successor admission.

This is a draft for feasibility reaction, not a frozen signature; Windows native
representations are deliberately not fixed yet.

## Feasibility questions for Windows Owner (W1–W5)

For each, answer: feasible as drafted / feasible with a named modification /
infeasible (with the blocking reason). Provide the smallest evidence you can
(safe API or wrapper choice, sketch, or a short native probe) and its source
identity. Do not implement the full adapter yet.

- **W1 — safe API and dependency boundary.** Which safe API, wrapper crate, or
  trusted native component can satisfy atomic no-replace (file **and** directory,
  collision preserved, cross-volume fails) and Job Object create/attach/drain
  under `unsafe_code = "forbid"`? State whether a new dependency, a small native
  helper, or a reviewed unsafe wrapper is required, and its packaging impact.
  Copy/delete, hard-link staging, and overwrite-then-check do not qualify.

- **W2 — containment-before-start.** Can the native path guarantee the writer is
  inside the controlled domain before it executes, and that every partial-start
  failure still leaves an owner that can stop-and-drain? Identify the failure
  points and who owns the handle at each.

- **W3 — durable recovery identity.** After parent death, what durable identity
  proves a recorded domain belongs to a given attempt/role and is empty or
  drained? How are stale records, name reuse, and identity mismatch handled?
  State whether a new persisted/versioned field is actually required, or whether
  an existing native mechanism suffices.

- **W4 — native pipes and thread ownership.** Can stdin/stdout/stderr be handed
  to the existing `Read + Send` readers through a suitable abstraction, with the
  owner meeting the production `Send` requirement? Describe cancel/exit/close
  behavior.

- **W5 — atomic no-replace file and directory.** Confirm W1's chosen mechanism
  covers both files and directories with collision preservation and
  cross-volume failure, and name the API.

## Return format and merge conditions

Return through the configured Git remote with, per question: conclusion, named
mechanism/dependency, evidence and its source identity, and any contract change
requested. Then:

- **Feasible as drafted** — Linux freezes the shared port, completes remaining
  wiring, and issues the full implementation handoff; Windows implements native
  adapters and the restart/crash/no-replace matrix.
- **Feasible with a modification** — Linux revises the shared contract; Windows
  implements against the revision.
- **Infeasible** — Linux pauses only the affected design point and proposes an
  alternative; the independent Linux work already landed is retained.

## Non-blockers

While W1–W5 are reviewed, Linux continues: pipe/reader lifecycle hardening,
aria2/refresh audit, recovery regressions, layered validation, and the deeper
runner-retained owner finalization. None of these is gated on the Windows
answer. ZIP/7z remains deferred. `unsafe_code = "forbid"` and the production-v2
fail-closed gate are preserved throughout.