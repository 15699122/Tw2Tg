"""Extraction-only adapter: gallery-dl discovers media, never downloads it."""

from __future__ import annotations

import json
import subprocess
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable

from .errors import (
    GalleryDlError,
    cancelled_error,
    classify_returncode,
    interrupted_error,
    timeout_error,
)
from .models import normalize_metadata
from .process import POLL_INTERVAL_SECONDS, detached_spawn_options, terminate_tree
from .protocol_v2 import (
    ExtractionMediaItem,
    ExtractionRequestHeader,
    ExtractionResult,
    is_safe_filename,
    looks_like_secret,
    validate_extraction_result,
)


@dataclass(frozen=True)
class ExtractionConfig:
    executable: str = "gallery-dl"
    browser: str | None = None
    profile: str | None = None
    timeout_seconds: float = 300.0


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
        command = build_extraction_command(self.config, url)
        result = self._run_process(command, work_dir, is_cancelled, on_tick)

        if result.stderr and emit:
            emit({"event": "log", "level": "debug", "message": result.stderr[-4000:]})
        if result.returncode != 0:
            raise classify_returncode(result.returncode, result.stderr)

        tweet = normalize_metadata(self._read_info_json(work_dir), url)
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
        try:
            process = subprocess.Popen(
                command,
                cwd=work_dir,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                **detached_spawn_options(),
            )
        except FileNotFoundError as error:
            raise GalleryDlError("SIDECAR_DEPENDENCY_MISSING", str(error)) from error

        deadline = time.monotonic() + self.config.timeout_seconds
        while True:
            if on_tick:
                on_tick()
            stop_reason = is_cancelled() if is_cancelled is not None else None
            if stop_reason:
                terminate_tree(process)
                process.communicate()
                raise interrupted_error() if stop_reason == "shutdown" else cancelled_error()
            if time.monotonic() >= deadline:
                terminate_tree(process)
                process.communicate()
                raise timeout_error()
            try:
                stdout, stderr = process.communicate(timeout=POLL_INTERVAL_SECONDS)
            except subprocess.TimeoutExpired:
                continue
            return subprocess.CompletedProcess(
                args=command,
                returncode=process.returncode,
                stdout=stdout,
                stderr=stderr,
            )

    @staticmethod
    def _read_info_json(work_dir: Path) -> dict:
        candidates = sorted(
            {path for pattern in ("info.json", "*.info.json") for path in work_dir.rglob(pattern)}
        )
        if not candidates:
            raise GalleryDlError("METADATA_MISSING", "gallery-dl did not produce info.json")
        try:
            data = json.loads(candidates[0].read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise GalleryDlError("METADATA_INVALID", str(error)) from error
        if not isinstance(data, dict):
            raise GalleryDlError("METADATA_INVALID", "info.json must contain an object")
        return data


def sanitize_filename(raw: str | None, position: int) -> str:
    """Return a safe archive filename derived from metadata or a fallback."""
    if raw:
        cleaned: list[str] = []
        for char in raw:
            if char in {"/", "\\", "\0"} or ord(char) < 0x20:
                cleaned.append("_")
            else:
                cleaned.append(char)
        filename = "".join(cleaned).strip(". ")
        filename = filename.replace("..", "_")
        if filename and len(filename) <= 128 and is_safe_filename(filename):
            return filename
    return f"{position:02d}.bin"


def stable_media_id(item: Any, position: int) -> str:
    """Return a stable per-media identity, deriving one from the URL when needed."""
    if item.media_id:
        identity = str(item.media_id).strip()
        if identity:
            return identity
    url = (item.url or "").split("#", 1)[0].split("?", 1)[0]
    segment = url.rstrip("/").rsplit("/", 1)[-1]
    if segment and "." in segment:
        return segment
    return f"media-{position:02d}"


def to_extraction_result(tweet: Any, fallback_url: str) -> ExtractionResult:
    """Convert normalized metadata into durable extraction-only data."""
    media: list[ExtractionMediaItem] = []
    for position, item in enumerate(tweet.media, 1):
        media.append(
            ExtractionMediaItem(
                index=position,
                media_id=stable_media_id(item, position),
                media_type=item.media_type
                if item.media_type in {"photo", "video"}
                else "unknown",
                url=item.url,
                filename=sanitize_filename(item.filename, position),
                mime_type=item.mime_type,
            )
        )

    headers = [
        ExtractionRequestHeader(name="Referer", value="https://x.com/"),
        ExtractionRequestHeader(name="Accept", value="*/*"),
    ]
    for header in headers:
        if looks_like_secret(header.value):
            raise GalleryDlError("EXTRACTION_RESULT_INVALID", "header redaction failed")

    return ExtractionResult(
        tweet_id=tweet.tweet_id,
        url=tweet.url or fallback_url,
        tweet_type=tweet.tweet_type or "post",
        text=tweet.text or None,
        username=tweet.username,
        display_name=tweet.display_name,
        created_at=tweet.created_at,
        media=tuple(media),
        request_headers=tuple(headers),
    )


def build_extraction_command(config: ExtractionConfig, url: str) -> list[str]:
    """Build a deterministic extraction-only gallery-dl command.

    The command must never contain media download flags: extraction-only
    means gallery-dl discovers media and writes metadata, never media bytes.
    """
    command = [
        config.executable,
        "--config-ignore",
        "--no-input",
        "--quiet",
        "--skip-download",
        "--write-info-json",
    ]
    if config.browser:
        browser = config.browser
        if config.profile:
            browser = f"{browser}:{config.profile}"
        command.extend(["--cookies-from-browser", browser])
    command.append(url)
    media_write_flags = {"--directory", "--filename", "--download"}
    leaked = media_write_flags.intersection(command)
    if leaked:
        raise GalleryDlError(
            "EXTRACTION_COMMAND_INVALID",
            "extraction command must not contain media download flags: "
            + ", ".join(sorted(leaked)),
        )
    return command
