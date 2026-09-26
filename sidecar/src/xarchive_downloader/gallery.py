"""Subprocess adapter for the gallery-dl command line."""

from __future__ import annotations

import json
import mimetypes
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Callable

from .errors import GalleryDlError, classify_returncode, sanitize_error_text
from .models import DownloadedFile, ExtractedTweet, normalize_metadata

# gallery-dl can emit a large amount of diagnostics. `capture_output=True`
# buffers everything in memory, so the stream is drained incrementally and only
# the tail is kept for classification and logging.
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


@dataclass(frozen=True)
class GalleryDlConfig:
    executable: str = "gallery-dl"
    browser: str | None = None
    profile: str | None = None
    timeout_seconds: float = 900.0


def build_command(config: GalleryDlConfig, url: str, staging_dir: Path) -> list[str]:
    """Build a deterministic gallery-dl command without shell interpolation."""
    command = [
        config.executable,
        "--config-ignore",
        "--no-input",
        "--quiet",
        "--directory",
        str(staging_dir),
        "--filename",
        "{num:>02}.{extension}",
        "--write-info-json",
    ]
    if config.browser:
        browser = config.browser
        if config.profile:
            browser = f"{browser}:{config.profile}"
        command.extend(["--cookies-from-browser", browser])
    command.append(url)
    return command


class GalleryDlRunner:
    def __init__(self, config: GalleryDlConfig | None = None) -> None:
        self.config = config or GalleryDlConfig()

    def run(
        self,
        url: str,
        staging_dir: Path,
        emit: Callable[[dict], None] | None = None,
    ) -> ExtractedTweet:
        staging_dir.mkdir(parents=True, exist_ok=True)
        command = build_command(self.config, url, staging_dir)
        # Use temporary files instead of `capture_output=True` so a chatty
        # gallery-dl run cannot grow the worker's memory without bound.
        with tempfile.TemporaryFile(mode="w+", encoding="utf-8", errors="replace") as out, tempfile.TemporaryFile(
            mode="w+", encoding="utf-8", errors="replace"
        ) as err:
            try:
                result = subprocess.run(
                    command,
                    cwd=staging_dir,
                    stdout=out,
                    stderr=err,
                    timeout=self.config.timeout_seconds,
                    check=False,
                )
            except FileNotFoundError as error:
                raise GalleryDlError("SIDECAR_DEPENDENCY_MISSING", sanitize_error_text(str(error))) from error
            except subprocess.TimeoutExpired as error:
                raise GalleryDlError("DOWNLOAD_TIMEOUT", "gallery-dl timed out") from error

            err.seek(0)
            stderr_text = _drain_bounded(err)

        if stderr_text and emit:
            # Diagnostics cross the protocol boundary, so redact before emit.
            emit(
                {
                    "event": "log",
                    "level": "debug",
                    "message": sanitize_error_text(stderr_text)[-4000:],
                }
            )
        if result.returncode != 0:
            raise classify_returncode(result.returncode, stderr_text)

        metadata = self._read_info_json(staging_dir)
        tweet = normalize_metadata(metadata, url)
        files = self._scan_downloaded_files(staging_dir)
        tweet = ExtractedTweet(**{**tweet.__dict__, "files": tuple(files)})
        if emit:
            emit({"event": "metadata", "data": tweet.raw})
            for downloaded_file in files:
                emit(
                    {
                        "event": "file",
                        "path": downloaded_file.relative_path,
                        "size_bytes": downloaded_file.size_bytes,
                        "media_type": downloaded_file.media_type,
                        "mime_type": downloaded_file.mime_type,
                    }
                )
        return tweet

    @staticmethod
    def _scan_downloaded_files(staging_dir: Path) -> list[DownloadedFile]:
        files: list[DownloadedFile] = []
        for path in sorted(staging_dir.rglob("*")):
            if not path.is_file() or path.name == "info.json" or path.name.endswith(".aria2"):
                continue
            relative_path = path.relative_to(staging_dir).as_posix()
            mime_type, _ = mimetypes.guess_type(path.name)
            media_type = "video" if mime_type and mime_type.startswith("video/") else "photo"
            files.append(
                DownloadedFile(
                    relative_path=relative_path,
                    size_bytes=path.stat().st_size,
                    media_type=media_type,
                    mime_type=mime_type,
                )
            )
        return files

    @staticmethod
    def _read_info_json(staging_dir: Path) -> dict:
        candidates = sorted(staging_dir.rglob("info.json"))
        if not candidates:
            raise GalleryDlError("METADATA_MISSING", "gallery-dl did not produce info.json")
        try:
            data = json.loads(candidates[0].read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as error:
            raise GalleryDlError("METADATA_INVALID", str(error)) from error
        if not isinstance(data, dict):
            raise GalleryDlError("METADATA_INVALID", "info.json must contain an object")
        return data