# R6 remaining capability responsibility review

Owner: Cross-platform Owner. Reviewed on 2026-11-08 against `f183e0f5`.
This records decisions under the approved R6 scope, not a new architecture.

## Sidecar runner lifecycle: keep together

`sidecar/src/xarchive_downloader/extraction.py` is 430 lines after extracting
`gallery_metadata.py` (322 lines) and `gallery_commands.py` (175 lines).
The latter modules own metadata normalization and command/environment construction;
the former retains execution and result assembly.

`ExtractionRunner._run_process` owns the same subprocess lifecycle for extraction,
discovery and download: staging output handles, bounded stderr capture, cancellation
and timeout polling, detached spawn, process-tree termination, wait and cleanup.
`DiscoveryRunner` and `DownloadRunner` delegate to an `ExtractionRunner` rather than
owning competing process loops. Discovery's candidate scan is coupled to that loop
through `on_tick`; download result construction consumes the same staging output.
Keep these cohesive callers together. Splitting them now would create extra
cross-module lifecycle coupling without establishing an independent responsibility.
No generic helper module or compatibility-only shell is introduced.

Protocol-v2 worker entry points and re-exported extraction names stay unchanged.
Targeted extraction/discovery/protocol/proxy/downloader-argument tests: **56 passed**,
actual pytest exit **0**. Evidence:
`.tw2tg-tmp/agent-scratch/build-logs/r6-sidecar-review.log`.

## Desktop configuration: keep existing boundary

`desktop/src-tauri/src/config.rs` is 1,814 lines, with tests beginning at line 1,067.
Its production responsibilities form the configuration boundary: serializable
settings and defaults, validation, loading, and the Linux configuration-exchange
transaction impl. Exchange's model and preview are already extracted to
`config_exchange.rs`; its transaction stays beside `AppConfig` and `PortablePaths`
and uses the same preview, validation and stale-revision check.
Network and Telegram settings are identifiable subdomains, but no new consumer or
independent lifecycle requires their extraction in this refactoring batch.
Keep existing import paths and serialization together; line count alone is not a
reason to move definitions or distribute inherent impls across modules.

## Frontend bootstrap: keep existing boundary

`desktop/src/main.jsx` is 446 lines and already delegates page rendering to page
and component modules. It owns application composition and shared state/event
wiring. The top-level awaited theme load and DOM theme application precede
`createRoot`; preserve that bootstrap order. No additional feature-state extraction
is justified by this review. This read-only decision is not GUI acceptance.

## Validation scope and follow-up

No source changes were made for these three decisions. Sidecar's focused tests
exercise its already-landed split. Desktop configuration regressions are included
in the next R5 Desktop lib run; frontend behavior is unchanged. Full regression is
reserved for R7 integration. Windows implementation and verification remain with
the Windows Platform Owner after formal handoff.

See the [approved plan](../../development/internal-v1-completion-and-refactor-plan.md)
and [active execution cursor](../../status/platform-handoff.md).