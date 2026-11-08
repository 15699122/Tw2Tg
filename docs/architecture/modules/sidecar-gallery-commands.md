## sidecar-gallery-commands

- **Path / entry:** `sidecar/src/xarchive_downloader/gallery_commands.py`; reached
  through `xarchive_downloader.extraction`, which re-exports every public name
  here.
- **Responsibility:** Describe one gallery-dl invocation — configuration, argv
  and child environment — without running it.
- **Owner:** Cross-platform
- **Depends on:** `os`, `dataclasses`, `pathlib`; `.errors.GalleryDlError`.
- **Depended on by:** `xarchive_downloader.extraction` (`ExtractionRunner`,
  `DiscoveryRunner`, `DownloadRunner` build their commands and environment
  here).
- **Public contract:** `ExtractionConfig`, `spawn_env`,
  `PROXY_ENVIRONMENT_KEYS`, `build_extraction_command`,
  `build_download_command`. Flag sets, argument order, error codes
  (`EXTRACTION_COMMAND_INVALID`, `DOWNLOAD_COMMAND_INVALID`) and proxy-mode
  semantics (`system` / `direct` / `manual`) are unchanged. `direct` mode still
  removes proxy variables case-insensitively for Windows.
- **Compatibility entry:** `xarchive_downloader.extraction` re-exports these
  names, so `worker_v2` and the existing tests import exactly as before.
- **State and ownership:** none. The module never spawns, waits, signals or
  cleans up a process, so cancellation, timeout and subprocess cleanup stay in
  `extraction`.
- **Failure behaviour:** fail closed before any process starts. A staging
  directory that is not absolute, an extraction command carrying media-write
  flags or a download command carrying metadata-only flags raises
  `GalleryDlError` instead of being launched.
- **Tests:** `sidecar/tests/test_downloader_args.py`,
  `sidecar/tests/test_proxy_mode.py`, `sidecar/tests/test_protocol_v2.py`
  (imported through `extraction`).
- **Gates to re-run:** `python3 -m pytest sidecar/tests`, plus the supervisor
  containment tests that launch real sidecar children.
- **Exceptions:** none.
- **Open questions:** none.
