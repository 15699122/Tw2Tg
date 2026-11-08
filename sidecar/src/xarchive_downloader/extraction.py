"""Extraction-only adapter: gallery-dl discovers media, never downloads it.

This module owns the subprocess lifecycle for one gallery-dl invocation:
staging, timeouts, cancellation, output capture and cleanup. Metadata
normalization lives in :mod:`xarchive_downloader.gallery_metadata`; invocation
configuration, argv and child environment live in
:mod:`xarchive_downloader.gallery_commands`.

Every previously public name is re-exported below, so the protocol-v2 worker
entry and existing tests keep importing from this module unchanged.
"""

from __future__ import annotations

import json
import mimetypes
import subprocess
import tempfile
import time
from pathlib import Path
from typing import Callable

from .errors import (
    GalleryDlError,
    cancelled_error,
    classify_returncode,
    interrupted_error,
    sanitize_error_text,
    timeout_error,
)
from .models import normalize_metadata
from .gallery_commands import (
    PROXY_ENVIRONMENT_KEYS as PROXY_ENVIRONMENT_KEYS,
    ExtractionConfig,
    build_download_command,
    build_extraction_command,
    spawn_env,
)
from .gallery_metadata import (
    GALLERY_DL_JSONL,
    GalleryDlJsonlParser as GalleryDlJsonlParser,
    is_metadata_file,
    media_type_for_filename,
    metadata_paths,
    purge_metadata_files,
    read_gallery_jsonl,
    read_gallery_jsonl_matching,
    read_metadata_candidates as read_metadata_candidates,
    read_metadata_matching,
    sanitize_filename,
    stable_media_id as stable_media_id,
    to_discovery_candidate,
    to_extraction_result,
    tweet_id_from_url as tweet_id_from_url,
)
from .process import POLL_INTERVAL_SECONDS, detached_spawn_options, terminate_tree
from .protocol_v2 import (
    DiscoveryCandidate,
    DownloadedMediaFile,
    DownloadResult,
    ExtractionResult,
    candidate_to_json,
    validate_candidate,
    validate_download_result,
    validate_extraction_result,
)


# gallery-dl can emit a large amount of diagnostics. Reading stderr fully into
# memory would let a chatty run grow the worker without bound (ENG-05), so only
# the trailing characters are retained for classification and logging.
MAX_CAPTURED_OUTPUT_CHARS = 64 * 1024


def _drain_bounded(stream) -> str:
    """Read a stream to its end, retaining at most the trailing characters."""
    chunks: list[str] = []
    kept = 0
    for raw in iter(stream.readline, ""):
        text = raw if isinstance(raw, str) else raw.decode("utf-8", "replace")
        chunks.append(text)
        kept += len(text)
        if kept > MAX_CAPTURED_OUTPUT_CHARS * 2:
            # Keep memory bounded; only the tail is used downstream.
            chunks = chunks[-2:]
            kept = sum(len(chunk) for chunk in chunks)
    captured = "".join(chunks)
    if len(captured) > MAX_CAPTURED_OUTPUT_CHARS:
        captured = captured[-MAX_CAPTURED_OUTPUT_CHARS:]
    return captured


class ExtractionRunner:
    def __init__(self, config: ExtractionConfig | None = None) -> None:
        self.config = config or ExtractionConfig()

    def run(
        self,
        url: str,
        work_dir: Path,
        emit: Callable[[dict], None] | None = None,
        is_cancelled: Callable[[], str | None] | None = None,
        on_tick: Callable[[], None] | None = None,
    ) -> ExtractionResult:
        """Run one extraction-only gallery-dl invocation."""
        work_dir.mkdir(parents=True, exist_ok=True)
        # P1-B: stale metadata from a previous failed attempt must never be
        # able to satisfy this request; the on-disk workspace is purged first
        # and the surviving file is then identity-matched to the request URL.
        purge_metadata_files(work_dir)
        command = build_extraction_command(self.config, url)
        result = self._run_process(command, work_dir, is_cancelled, on_tick)

        if result.stderr and emit:
            # Diagnostics cross the protocol boundary, so redact before emit.
            emit({"event": "log", "level": "debug", "message": sanitize_error_text(result.stderr)[-4000:]})
        if result.returncode != 0:
            raise classify_returncode(result.returncode, result.stderr)

        try:
            data = read_gallery_jsonl_matching(work_dir, url)
        except GalleryDlError as jsonl_error:
            try:
                data = read_metadata_matching(work_dir, url)
            except GalleryDlError:
                raise jsonl_error
        tweet = normalize_metadata(data, url)
        extraction = to_extraction_result(tweet, url)
        rejection = validate_extraction_result(extraction)
        if rejection:
            raise GalleryDlError("EXTRACTION_RESULT_INVALID", rejection)
        return extraction

    def _run_process(
        self,
        command: list[str],
        work_dir: Path,
        is_cancelled: Callable[[], str | None] | None,
        on_tick: Callable[[], None] | None,
    ) -> subprocess.CompletedProcess:
        deadline = time.monotonic() + self.config.timeout_seconds
        try:
            # stderr goes to a temporary file instead of `subprocess.PIPE` so a
            # chatty gallery-dl run cannot grow the worker's memory without
            # bound (ENG-05); only the bounded tail is read back afterwards.
            with (work_dir / GALLERY_DL_JSONL).open("w", encoding="utf-8") as output_file, tempfile.TemporaryFile(
                mode="w+", encoding="utf-8", errors="replace"
            ) as error_file:
                process = subprocess.Popen(
                    command,
                    cwd=work_dir,
                    stdout=output_file,
                    stderr=error_file,
                    text=True,
                    env=spawn_env(self.config.proxy, self.config.proxy_mode),
                    **detached_spawn_options(),
                )
                try:
                    while True:
                        if on_tick:
                            on_tick()
                        stop_reason = is_cancelled() if is_cancelled is not None else None
                        if stop_reason:
                            terminate_tree(process)
                            process.wait()
                            raise interrupted_error() if stop_reason == "shutdown" else cancelled_error()
                        if time.monotonic() >= deadline:
                            terminate_tree(process)
                            process.wait()
                            raise timeout_error()
                        returncode = process.poll()
                        if returncode is not None:
                            error_file.seek(0)
                            return subprocess.CompletedProcess(
                                args=command,
                                returncode=returncode,
                                stdout="",
                                stderr=_drain_bounded(error_file),
                            )
                        time.sleep(POLL_INTERVAL_SECONDS)
                finally:
                    if process.poll() is None:
                        terminate_tree(process)
        except FileNotFoundError as error:
            raise GalleryDlError("SIDECAR_DEPENDENCY_MISSING", sanitize_error_text(str(error))) from error

    @staticmethod
    def _read_info_json(work_dir: Path) -> dict:
        # Compatibility shim; production reads go through
        # ``read_metadata_matching`` so identity is verified.
        return read_metadata_matching(work_dir, "")


class DiscoveryRunner:
    """Account discovery: enumerate durable Tweet candidates for one profile.

    gallery-dl still runs extraction-only (``--skip-download``); the workspace
    is purged first so only files produced by this invocation are scanned, and
    every candidate is identity/shape validated before it is emitted.
    """

    def __init__(self, config: ExtractionConfig | None = None) -> None:
        self.config = config or ExtractionConfig()
        self._runner = ExtractionRunner(self.config)

    def run(
        self,
        url: str,
        work_dir: Path,
        emit: Callable[[dict], None] | None = None,
        is_cancelled: Callable[[], str | None] | None = None,
        on_tick: Callable[[], None] | None = None,
    ) -> list[DiscoveryCandidate]:
        work_dir.mkdir(parents=True, exist_ok=True)
        purge_metadata_files(work_dir)
        command = build_extraction_command(self.config, url)
        candidates: list[DiscoveryCandidate] = []
        seen_ids: set[str] = set()
        seen_paths: set[Path] = set()

        def scan_metadata() -> None:
            records = read_gallery_jsonl(work_dir)
            if records:
                for data in records:
                    try:
                        tweet = normalize_metadata(data, url)
                    except ValueError as error:
                        if emit:
                            emit({"event": "log", "level": "debug", "message": str(error)})
                        continue
                    candidate = to_discovery_candidate(tweet, url)
                    if candidate is None:
                        continue
                    rejection = validate_candidate(candidate)
                    if rejection:
                        if emit:
                            emit({"event": "log", "level": "debug", "message": rejection})
                        continue
                    if candidate.tweet_id in seen_ids:
                        continue
                    seen_ids.add(candidate.tweet_id)
                    candidates.append(candidate)
                    if emit:
                        emit({"event": "candidate", "candidate": candidate_to_json(candidate)})
                return
            for path in metadata_paths(work_dir):
                if path in seen_paths:
                    continue
                try:
                    data = json.loads(path.read_text(encoding="utf-8"))
                except (OSError, json.JSONDecodeError):
                    # A file may be between write and fsync while gallery-dl is
                    # still running. Leave it unseen so the next tick retries it.
                    continue
                if not isinstance(data, dict):
                    seen_paths.add(path)
                    continue
                seen_paths.add(path)
                try:
                    tweet = normalize_metadata(data, url)
                except ValueError as error:
                    if emit:
                        emit({"event": "log", "level": "debug", "message": str(error)})
                    continue
                candidate = to_discovery_candidate(tweet, url)
                if candidate is None:
                    continue
                rejection = validate_candidate(candidate)
                if rejection:
                    if emit:
                        emit({"event": "log", "level": "debug", "message": rejection})
                    continue
                if candidate.tweet_id in seen_ids:
                    continue
                seen_ids.add(candidate.tweet_id)
                candidates.append(candidate)
                if emit:
                    emit({"event": "candidate", "candidate": candidate_to_json(candidate)})

        def tick() -> None:
            scan_metadata()
            if on_tick:
                on_tick()

        result = self._runner._run_process(command, work_dir, is_cancelled, tick)
        scan_metadata()
        if result.stderr and emit:
            # Diagnostics cross the protocol boundary, so redact before emit.
            emit({"event": "log", "level": "debug", "message": sanitize_error_text(result.stderr)[-4000:]})
        if result.returncode != 0:
            raise classify_returncode(result.returncode, result.stderr)
        return candidates


def purge_downloaded_files(work_dir: Path) -> int:
    """Best-effort removal of files left by an earlier download attempt."""
    removed = 0
    for path in sorted(work_dir.rglob("*"), reverse=True):
        try:
            if path.is_symlink() or path.is_file():
                path.unlink()
                removed += 1
            elif path.is_dir():
                path.rmdir()
        except OSError:
            # Identity matching and the size verification below remain the
            # correctness guarantee, so a locked leftover fails later rather
            # than being reported as this run's download.
            continue
    return removed


class DownloadRunner:
    """Full media download into the job staging directory.

    gallery-dl writes the media bytes itself, so this runner replaces the
    legacy aria2 transfer for one tweet: it runs a single gallery-dl
    invocation into ``staging_dir``, recovers the extraction result from the
    ``info.json`` files gallery-dl wrote beside the media, and reports only
    staging-relative regular files.
    """

    def __init__(self, config: ExtractionConfig | None = None) -> None:
        self.config = config or ExtractionConfig()
        self._runner = ExtractionRunner(self.config)

    def run(
        self,
        url: str,
        staging_dir: Path,
        emit: Callable[[dict], None] | None = None,
        is_cancelled: Callable[[], str | None] | None = None,
        on_tick: Callable[[], None] | None = None,
        user_args: tuple[str, ...] = (),
    ) -> DownloadResult:
        staging_dir.mkdir(parents=True, exist_ok=True)
        # Stale files from a previous failed attempt must never satisfy this
        # request, so the job directory is emptied before the invocation.
        purge_downloaded_files(staging_dir)
        command = build_download_command(self.config, url, staging_dir, user_args)
        result = self._runner._run_process(command, staging_dir, is_cancelled, on_tick)

        if result.stderr and emit:
            # Diagnostics cross the protocol boundary, so redact before emit.
            diagnostic = sanitize_error_text(result.stderr)
            for argument in user_args:
                if argument and len(argument) >= 4:
                    diagnostic = diagnostic.replace(argument, "[REDACTED_ARGUMENT]")
            emit(
                {
                    "event": "log",
                    "level": "debug",
                    "message": diagnostic[-4000:],
                }
            )
        if result.returncode != 0:
            error = classify_returncode(result.returncode, result.stderr)
            for argument in user_args:
                if argument and len(argument) >= 4:
                    error.message = error.message.replace(argument, "[REDACTED_ARGUMENT]")
            raise error

        extraction = self._read_extraction(staging_dir, url)
        files = self._scan_downloaded_files(staging_dir, extraction)
        # gallery-dl wrote its metadata into the job staging directory; that is
        # metadata, not media, so it never reaches the archive commit path.
        purge_metadata_files(staging_dir)
        download = DownloadResult(
            tweet_id=extraction.tweet_id,
            url=extraction.url,
            media=tuple(files),
            result=extraction,
        )
        rejection = validate_download_result(download)
        if rejection:
            raise GalleryDlError("DOWNLOAD_RESULT_INVALID", rejection)
        return download

    def _read_extraction(self, staging_dir: Path, url: str) -> ExtractionResult:
        """Recover the extraction result written next to the downloaded media."""
        try:
            data = read_gallery_jsonl_matching(staging_dir, url)
        except GalleryDlError as jsonl_error:
            try:
                data = read_metadata_matching(staging_dir, url)
            except GalleryDlError:
                raise jsonl_error
        tweet = normalize_metadata(data, url)
        extraction = to_extraction_result(tweet, url)
        rejection = validate_extraction_result(extraction)
        if rejection:
            raise GalleryDlError("EXTRACTION_RESULT_INVALID", rejection)
        return extraction

    def _scan_downloaded_files(
        self,
        staging_dir: Path,
        extraction: ExtractionResult,
    ) -> list[DownloadedMediaFile]:
        """Report the staging media, pairing files with extracted media items.

        gallery-dl names media ``{num:>02}.{extension}`` in extraction order, so
        sorted scan order matches the extraction media indexes. A file that
        cannot be paired keeps its own index and reports a null ``media_id``
        instead of borrowing another media item's identity.
        """
        paths = sorted(
            path
            for path in staging_dir.rglob("*")
            if path.is_file()
            and not path.is_symlink()
            and not is_metadata_file(path)
        )
        media_by_index = {item.index: item for item in extraction.media}
        files: list[DownloadedMediaFile] = []
        for position, path in enumerate(paths, 1):
            item = media_by_index.get(position)
            mime_type, _ = mimetypes.guess_type(path.name)
            files.append(
                DownloadedMediaFile(
                    index=position,
                    media_id=item.media_id if item is not None else None,
                    media_type=media_type_for_filename(path.name, mime_type),
                    filename=sanitize_filename(path.name, position),
                    relative_path=path.relative_to(staging_dir).as_posix(),
                    size_bytes=path.stat().st_size,
                    mime_type=mime_type,
                )
            )
        return files
