"""Extraction-only adapter: gallery-dl discovers media, never downloads it."""

from __future__ import annotations

import json
import os
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
    DiscoveryCandidate,
    ExtractionMediaItem,
    ExtractionQuotedTweet,
    ExtractionRequestHeader,
    ExtractionResult,
    candidate_to_json,
    is_safe_filename,
    looks_like_secret,
    validate_candidate,
    validate_extraction_result,
)


GALLERY_DL_JSONL = "gallery-dl.jsonl"


class GalleryDlJsonlParser:
    """Aggregate gallery-dl Message.Directory/Url tuples by Tweet identity."""

    def __init__(self) -> None:
        self.records: dict[str, dict[str, Any]] = {}
        self.order: list[str] = []

    def feed_line(self, line: str) -> None:
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            return
        if not isinstance(message, list) or not message:
            return
        kind = message[0]
        if kind == 2 and len(message) == 2 and isinstance(message[1], dict):
            self._upsert(message[1], [])
        elif kind == 3 and len(message) == 3:
            url, metadata = message[1], message[2]
            if isinstance(url, str) and isinstance(metadata, dict):
                item = dict(metadata)
                item["url"] = url
                tweet_metadata = dict(metadata)
                tweet_metadata.pop("url", None)
                extension = str(item.get("extension") or "").lower()
                item.setdefault(
                    "type",
                    "video" if extension in {"mp4", "m4v", "mov", "webm"} else "photo",
                )
                self._upsert(tweet_metadata, [item])

    def _upsert(self, metadata: dict[str, Any], media: list[dict[str, Any]]) -> None:
        tweet_id = str(metadata.get("tweet_id") or metadata.get("status_id") or "")
        if not tweet_id.isdigit():
            return
        record = self.records.setdefault(tweet_id, {})
        for key, value in metadata.items():
            if value not in (None, "", {}, []):
                record[key] = value
        if media:
            record.setdefault("media", []).extend(media)
        if tweet_id not in self.order:
            self.order.append(tweet_id)

    def values(self) -> list[dict[str, Any]]:
        return [self.records[tweet_id] for tweet_id in self.order]


@dataclass(frozen=True)
class ExtractionConfig:
    executable: str = "gallery-dl"
    executable_args: tuple[str, ...] = ()
    browser: str | None = None
    profile: str | None = None
    proxy: str | None = None
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
        # P1-B: stale metadata from a previous failed attempt must never be
        # able to satisfy this request; the on-disk workspace is purged first
        # and the surviving file is then identity-matched to the request URL.
        purge_metadata_files(work_dir)
        command = build_extraction_command(self.config, url)
        result = self._run_process(command, work_dir, is_cancelled, on_tick)

        if result.stderr and emit:
            emit({"event": "log", "level": "debug", "message": result.stderr[-4000:]})
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
            with (work_dir / GALLERY_DL_JSONL).open("w", encoding="utf-8") as output_file:
                process = subprocess.Popen(
                    command,
                    cwd=work_dir,
                    stdout=output_file,
                    stderr=subprocess.PIPE,
                    text=True,
                    env=spawn_env(self.config.proxy),
                    **detached_spawn_options(),
                )
                try:
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
                            _stdout, stderr = process.communicate(timeout=POLL_INTERVAL_SECONDS)
                            return subprocess.CompletedProcess(
                                args=command,
                                returncode=process.returncode,
                                stdout="",
                                stderr=stderr,
                            )
                        except subprocess.TimeoutExpired:
                            continue
                finally:
                    if process.poll() is None:
                        terminate_tree(process)
        except FileNotFoundError as error:
            raise GalleryDlError("SIDECAR_DEPENDENCY_MISSING", str(error)) from error

    @staticmethod
    def _read_info_json(work_dir: Path) -> dict:
        # Compatibility shim; production reads go through
        # ``read_metadata_matching`` so identity is verified.
        return read_metadata_matching(work_dir, "")


def spawn_env(proxy: str | None) -> dict[str, str] | None:
    """Return the child environment, applying the configured proxy if any."""
    if not proxy:
        return None
    env = os.environ.copy()
    for key in ("HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy"):
        env[key] = proxy
    return env


def purge_metadata_files(work_dir: Path) -> int:
    """Best-effort removal of previously written info.json files."""
    removed = 0
    for pattern in ("info.json", "*.info.json"):
        for path in work_dir.rglob(pattern):
            try:
                if path.is_file():
                    path.unlink()
                    removed += 1
            except OSError:
                # Identity matching below remains the correctness guarantee.
                continue
    return removed


def tweet_id_from_url(url: str) -> str | None:
    """Extract the status id from a canonical X/Twitter status URL."""
    marker = "/status/" if "/status/" in url else "/statuses/"
    if marker not in url:
        return None
    tail = url.split(marker, 1)[1].split("?", 1)[0].split("#", 1)[0].split("/", 1)[0]
    return tail if tail.isdigit() else None


def metadata_paths(work_dir: Path) -> list[Path]:
    """Return stable, deterministic metadata-file paths under a work directory."""
    return sorted(
        {path for pattern in ("info.json", "*.info.json") for path in work_dir.rglob(pattern)}
    )


def read_gallery_jsonl(work_dir: Path) -> list[dict]:
    """Read gallery-dl JSONL messages into normalized metadata dictionaries."""
    path = work_dir / GALLERY_DL_JSONL
    try:
        lines = path.read_text(encoding="utf-8").splitlines()
    except OSError:
        return []
    parser = GalleryDlJsonlParser()
    for line in lines:
        parser.feed_line(line)
    return parser.values()


def read_gallery_jsonl_matching(work_dir: Path, url: str) -> dict:
    """Return the canonical JSONL record matching one requested status URL."""
    records = read_gallery_jsonl(work_dir)
    expected = tweet_id_from_url(url)
    if expected:
        for data in records:
            if _metadata_tweet_id(data) == expected:
                return data
        raise GalleryDlError(
            "METADATA_MISMATCH",
            f"gallery-dl JSONL does not match requested tweet {expected}",
        )
    if len(records) == 1:
        return records[0]
    if not records:
        raise GalleryDlError("METADATA_MISSING", "gallery-dl produced no JSONL records")
    raise GalleryDlError(
        "METADATA_AMBIGUOUS",
        "multiple gallery-dl JSONL records and the request URL has no status id",
    )


def read_metadata_candidates(work_dir: Path) -> list[dict]:
    """Return every parseable info.json object under ``work_dir``."""
    parsed: list[dict] = []
    for path in metadata_paths(work_dir):
        try:
            data = json.loads(path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError):
            continue
        if isinstance(data, dict):
            parsed.append(data)
    return parsed


def _metadata_tweet_id(data: dict) -> str | None:
    value = str(data.get("tweet_id") or data.get("status_id") or data.get("id") or "")
    return value if value.isdigit() else None


def read_metadata_matching(work_dir: Path, url: str) -> dict:
    """Read the info.json whose identity matches the requested status URL.

    Selection is identity-based, never file-order-based: a stale file left by
    a failed earlier attempt cannot satisfy a different Tweet request.
    """
    candidates = read_metadata_candidates(work_dir)
    if not candidates:
        raise GalleryDlError("METADATA_MISSING", "gallery-dl did not produce info.json")
    expected = tweet_id_from_url(url)
    if expected:
        for data in candidates:
            if _metadata_tweet_id(data) == expected:
                return data
        raise GalleryDlError(
            "METADATA_MISMATCH",
            f"gallery-dl metadata does not match requested tweet {expected}",
        )
    if len(candidates) == 1:
        return candidates[0]
    raise GalleryDlError(
        "METADATA_AMBIGUOUS",
        "multiple info.json files and the request URL has no status id",
    )


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

    quoted = tweet.quoted_tweet
    quoted_payload = (
        ExtractionQuotedTweet(
            tweet_id=quoted.tweet_id,
            url=quoted.url,
            username=quoted.username,
            display_name=quoted.display_name,
            user_id=quoted.user_id,
            text=quoted.text,
            created_at=quoted.created_at,
            tweet_type=quoted.tweet_type,
        )
        if quoted is not None
        else None
    )

    return ExtractionResult(
        tweet_id=tweet.tweet_id,
        url=tweet.url or fallback_url,
        tweet_type=tweet.tweet_type or "post",
        text=tweet.text or None,
        username=tweet.username,
        display_name=tweet.display_name,
        created_at=tweet.created_at,
        user_id=tweet.user_id,
        reply_to=tweet.reply_to_tweet_id,
        quoted_tweet=quoted_payload,
        media=tuple(media),
        request_headers=tuple(headers),
    )


def to_discovery_candidate(
    tweet: Any, fallback_url: str
) -> DiscoveryCandidate | None:
    """Convert one normalized tweet into a durable discovery candidate."""
    contains_id = tweet.tweet_id in (tweet.url or "")
    if contains_id:
        url = tweet.url
    elif tweet.username:
        url = f"https://x.com/{tweet.username}/status/{tweet.tweet_id}"
    else:
        return None
    tweet_type = tweet.tweet_type or "post"
    if tweet_type not in {"post", "reply", "quote"}:
        tweet_type = "retweet" if tweet.is_repost else "post"
    if tweet.is_repost:
        tweet_type = "retweet"
    return DiscoveryCandidate(
        tweet_id=tweet.tweet_id,
        url=url,
        tweet_type=tweet_type,
        is_repost=tweet.is_repost,
        has_media=bool(tweet.media),
        media_count=min(len(tweet.media), 64),
        created_at=tweet.created_at,
        user_id=tweet.user_id,
        username=tweet.username,
    )


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
            emit({"event": "log", "level": "debug", "message": result.stderr[-4000:]})
        if result.returncode != 0:
            raise classify_returncode(result.returncode, result.stderr)
        return candidates


def build_extraction_command(config: ExtractionConfig, url: str) -> list[str]:
    """Build a deterministic extraction-only gallery-dl command.

    The command must never contain media download flags: extraction-only
    means gallery-dl discovers media and writes metadata, never media bytes.
    """
    command = [
        config.executable,
        *config.executable_args,
        "--config-ignore",
        "--no-input",
        "--quiet",
        "--skip-download",
        "--write-info-json",
        "--dump-json",
        "-o",
        "output.jsonl=true",
        "-o",
        "extractor.twitter.text-tweets=true",
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
