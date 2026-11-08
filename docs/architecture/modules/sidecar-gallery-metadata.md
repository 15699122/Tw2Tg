## sidecar-gallery-metadata

- **Path / entry:** `sidecar/src/xarchive_downloader/gallery_metadata.py`; reached
  through `xarchive_downloader.extraction`, which re-exports every public name
  here.
- **Responsibility:** Read what a gallery-dl invocation wrote under a work
  directory and normalize it into protocol-v2 extraction and discovery results.
- **Owner:** Cross-platform
- **Depends on:** `json`, `pathlib`; `.errors.GalleryDlError`;
  `.protocol_v2` (`ExtractionResult`, `ExtractionMediaItem`,
  `ExtractionQuotedTweet`, `ExtractionRequestHeader`, `DiscoveryCandidate`,
  `is_safe_filename`, `looks_like_secret`).
- **Depended on by:** `xarchive_downloader.extraction` (`ExtractionRunner`,
  `DiscoveryRunner`, `DownloadRunner`).
- **Public contract:** `GALLERY_DL_JSONL`, `GalleryDlJsonlParser`,
  `tweet_id_from_url`, `metadata_paths`, `read_gallery_jsonl`,
  `read_gallery_jsonl_matching`, `read_metadata_candidates`,
  `read_metadata_matching`, `purge_metadata_files`, `is_metadata_file`,
  `media_type_for_filename`, `sanitize_filename`, `stable_media_id`,
  `to_extraction_result`, `to_discovery_candidate`. Error codes
  (`METADATA_MISSING`, `METADATA_MISMATCH`, `METADATA_AMBIGUOUS`,
  `EXTRACTION_RESULT_INVALID`) and every produced payload are unchanged.
- **Compatibility entry:** `xarchive_downloader.extraction` re-exports these
  names, so `worker_v2` and the existing tests import exactly as before.
- **State and ownership:** none. No subprocess, cancellation, timeout, staging
  or download state is created or held here; the module only reads and
  transforms data that already exists on disk.
- **Failure behaviour:** fail closed. Ambiguous or mismatched metadata raises
  `GalleryDlError` instead of returning a best-effort record, and metadata
  selection is identity-based rather than file-order-based so a stale file from
  an earlier attempt cannot satisfy a different Tweet request.
- **Tests:** `sidecar/tests/test_protocol_v2.py`,
  `sidecar/tests/test_extraction_only.py`, `sidecar/tests/test_discovery.py`,
  `sidecar/tests/test_downloader_args.py` (imported through `extraction`).
- **Gates to re-run:** `python3 -m pytest sidecar/tests`, plus any Desktop or
  supervisor test that consumes sidecar extraction output.
- **Exceptions:** none.
- **Open questions:** none.
