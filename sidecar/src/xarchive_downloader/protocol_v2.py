"""Typed protocol v2 models shared by the extraction worker and tests."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

SIDECAR_V2_PROTOCOL_VERSION = 2
REQUIRED_V2_CAPABILITIES = (
    "extract_media",
    "cancel_active_extraction",
    "structured_media_plan",
)
ALLOWED_V2_COMMAND_FIELDS = frozenset(
    {"protocol_version", "request_id", "cmd", "job_id", "url", "browser", "profile"}
)
ALLOWED_V2_HEADER_NAMES = frozenset(
    {"referer", "origin", "user-agent", "accept", "accept-language"}
)
_SECRET_MARKERS = (
    "cookie",
    "authorization",
    "bearer ",
    "secret",
    "token=",
)


class SidecarV2Error(Exception):
    """Structured protocol error whose message is bounded and redacted."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = redact_error_code(code)
        self.message = bound_error_message(message)


@dataclass(frozen=True)
class ExtractionMediaItem:
    index: int
    media_id: str | None
    media_type: str
    url: str
    filename: str
    mime_type: str | None = None


@dataclass(frozen=True)
class ExtractionRequestHeader:
    name: str
    value: str


@dataclass(frozen=True)
class ExtractionResult:
    tweet_id: str
    url: str
    tweet_type: str = "post"
    text: str | None = None
    username: str | None = None
    display_name: str | None = None
    created_at: str | None = None
    media: tuple[ExtractionMediaItem, ...] = ()
    request_headers: tuple[ExtractionRequestHeader, ...] = ()


def validate_command(command: dict[str, Any]) -> str | None:
    """Return a stable rejection reason, or ``None`` when accepted."""
    unknown = sorted(set(command) - ALLOWED_V2_COMMAND_FIELDS)
    if unknown:
        return "unknown command field(s): " + ", ".join(unknown)
    if command.get("protocol_version") != SIDECAR_V2_PROTOCOL_VERSION:
        return "unsupported protocol version"
    command_name = command.get("cmd")
    if command_name not in {"hello", "extract", "cancel", "shutdown"}:
        return "unknown sidecar command"
    if not command.get("request_id") or not command.get("job_id"):
        return "request_id and job_id are required"
    if command_name in {"hello", "shutdown"} and command.get("url"):
        return "url is not allowed for this command"
    if command_name == "extract":
        if not command.get("url"):
            return "url is required"
        if command.get("job_id") == "system":
            return "extract requires a job identity"
    return None


def validate_extraction_result(result: ExtractionResult) -> str | None:
    """Validate durable extraction data before it is emitted as JSONL."""
    if not result.tweet_id.isdigit() or len(result.tweet_id) > 32:
        return "invalid tweet identity"
    if result.tweet_id not in result.url:
        return "request/job identity mismatch"
    if result.tweet_type not in {"post", "reply", "quote"}:
        return "invalid tweet type"
    if len(result.media) > 32 or len(result.request_headers) > 8:
        return "extraction result is too large"

    expected_index = 1
    for item in result.media:
        if item.index != expected_index:
            return "media order is unstable"
        expected_index += 1
        if not item.url or len(item.url) > 2048:
            return "invalid media url"
        if not is_safe_filename(item.filename):
            return "unsafe media filename"
        if looks_like_secret(item.url):
            return "media url contains credential material"

    for header in result.request_headers:
        if header.name.lower() not in ALLOWED_V2_HEADER_NAMES:
            return "header is not allowlisted"
        if not header.value or len(header.value) > 2048:
            return "invalid header value"
        if looks_like_secret(header.value):
            return "header contains credential material"
    return None


def is_safe_filename(filename: str) -> bool:
    if not filename or len(filename) > 128:
        return False
    if filename in {".", ".."} or filename.startswith("."):
        return False
    if any(separator in filename for separator in ("/", "\\", "\0")):
        return False
    if ".." in filename:
        return False
    return True


def looks_like_secret(value: str) -> bool:
    lowered = value.lower()
    return any(marker in lowered for marker in _SECRET_MARKERS)


def redact_error_code(code: str) -> str:
    trimmed = (code or "").strip()
    if not trimmed or len(trimmed) > 64:
        return "SIDECAR_INTERNAL_ERROR"
    if all(char.isalnum() or char == "_" for char in trimmed):
        return trimmed
    return "SIDECAR_INTERNAL_ERROR"


def bound_error_message(message: str) -> str:
    trimmed = (message or "").strip()
    if not trimmed:
        return "sidecar extraction failed"
    bounded = trimmed[:500]
    if looks_like_secret(bounded):
        return "sidecar extraction failed"
    return bounded


def extraction_result_to_json(result: ExtractionResult) -> dict[str, Any]:
    """Serialize durable extraction data without downloaded-file facts."""
    return {
        "tweet_id": result.tweet_id,
        "url": result.url,
        "tweet_type": result.tweet_type,
        "text": result.text,
        "username": result.username,
        "display_name": result.display_name,
        "created_at": result.created_at,
        "media": [
            {
                "index": item.index,
                "media_id": item.media_id,
                "media_type": item.media_type,
                "url": item.url,
                "filename": item.filename,
                "mime_type": item.mime_type,
            }
            for item in result.media
        ],
        "request_headers": [
            {"name": header.name, "value": header.value}
            for header in result.request_headers
        ],
    }
