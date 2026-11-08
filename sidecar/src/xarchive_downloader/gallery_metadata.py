"""gallery-dl metadata discovery and normalization.

This module reads what a gallery-dl invocation wrote to a work directory and
normalizes it into protocol-v2 results. It never starts a process, never
downloads media bytes and holds no cancellation or timeout state; those stay in
:mod:`xarchive_downloader.extraction`.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from .errors import GalleryDlError
from .protocol_v2 import (
    DiscoveryCandidate,
    ExtractionMediaItem,
    ExtractionQuotedTweet,
    ExtractionRequestHeader,
    ExtractionResult,
    is_safe_filename,
    looks_like_secret,
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


def is_metadata_file(path: Path) -> bool:
    """Return whether a staging file is metadata rather than downloaded media."""
    name = path.name
    return (
        name == "info.json"
        or name.endswith(".info.json")
        or name == GALLERY_DL_JSONL
        or name.endswith(".aria2")
        or name.endswith(".part")
    )


def media_type_for_filename(filename: str, mime_type: str | None) -> str:
    """Classify one downloaded file from its guessed MIME type."""
    if mime_type and mime_type.startswith("video/"):
        return "video"
    if mime_type and mime_type.startswith("image/"):
        return "photo"


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

    return "unknown"
