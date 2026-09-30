"""Typed protocol v2 models shared by the extraction worker and tests."""

from __future__ import annotations

from dataclasses import dataclass
from typing import Any

SIDECAR_V2_PROTOCOL_VERSION = 2
REQUIRED_V2_CAPABILITIES = (
    "extract_media",
    "cancel_active_extraction",
    "structured_media_plan",
    "account_discovery",
)
ALLOWED_V2_COMMAND_FIELDS = frozenset(
    {"protocol_version", "request_id", "cmd", "job_id", "url", "browser", "profile"}
)
ALLOWED_V2_HEADER_NAMES = frozenset(
    {"referer", "origin", "user-agent", "accept", "accept-language"}
)
ALLOWED_V2_TWEET_TYPES = frozenset({"post", "reply", "quote"})
ALLOWED_CANDIDATE_TWEET_TYPES = frozenset({"post", "reply", "quote", "retweet"})
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
class ExtractionQuotedTweet:
    tweet_id: str
    url: str
    username: str | None = None
    display_name: str | None = None
    user_id: str | None = None
    text: str | None = None
    created_at: str | None = None
    tweet_type: str | None = None


@dataclass(frozen=True)
class ExtractionResult:
    tweet_id: str
    url: str
    tweet_type: str = "post"
    text: str | None = None
    username: str | None = None
    display_name: str | None = None
    created_at: str | None = None
    user_id: str | None = None
    reply_to: str | None = None
    quoted_tweet: ExtractionQuotedTweet | None = None
    media: tuple[ExtractionMediaItem, ...] = ()
    request_headers: tuple[ExtractionRequestHeader, ...] = ()


@dataclass(frozen=True)
class DiscoveryCandidate:
    tweet_id: str
    url: str
    tweet_type: str = "post"
    is_repost: bool = False
    has_media: bool = False
    media_count: int = 0
    created_at: str | None = None
    user_id: str | None = None
    username: str | None = None


def validate_command(command: dict[str, Any]) -> str | None:
    """Return a stable rejection reason, or ``None`` when accepted."""
    unknown = sorted(set(command) - ALLOWED_V2_COMMAND_FIELDS)
    if unknown:
        return "unknown command field(s): " + ", ".join(unknown)
    if command.get("protocol_version") != SIDECAR_V2_PROTOCOL_VERSION:
        return "unsupported protocol version"
    command_name = command.get("cmd")
    if command_name not in {"hello", "extract", "discover", "cancel", "shutdown"}:
        return "unknown sidecar command"
    if not command.get("request_id") or not command.get("job_id"):
        return "request_id and job_id are required"
    if command_name in {"hello", "shutdown"} and command.get("url"):
        return "url is not allowed for this command"
    if command_name in {"extract", "discover"}:
        if not command.get("url"):
            return "url is required"
        if command.get("job_id") == "system":
            return "extract requires a job identity"
    if command_name == "discover" and not is_profile_url(str(command.get("url") or "")):
        return "invalid account profile url"
    return None


def is_profile_url(url: str) -> bool:
    """Return whether ``url`` is a canonical X/Twitter account profile URL."""
    for prefix in ("https://x.com/", "https://twitter.com/"):
        if url.startswith(prefix):
            rest = url[len(prefix) :].rstrip("/")
            return (
                bool(rest)
                and len(rest) <= 32
                and "?" not in rest
                and "#" not in rest
                and "/" not in rest
                and all(character.isascii() and (character.isalnum() or character in "_-") for character in rest)
            )
    return False


def validate_extraction_result(result: ExtractionResult) -> str | None:
    """Validate durable extraction data before it is emitted as JSONL."""
    if not result.tweet_id.isdigit() or len(result.tweet_id) > 32:
        return "invalid tweet identity"
    if result.tweet_id not in result.url:
        return "request/job identity mismatch"
    if result.tweet_type not in {"post", "reply", "quote"}:
        return "invalid tweet type"
    if not _is_valid_numeric_id(result.user_id):
        return "invalid author identity"
    if not _is_valid_numeric_id(result.reply_to):
        return "invalid reply identity"
    rejection = _validate_quoted_tweet(result.quoted_tweet)
    if rejection:
        return rejection
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


def _is_valid_numeric_id(value: str | None) -> bool:
    if value is None:
        return True
    return value.isdigit() and len(value) <= 32


def _validate_quoted_tweet(quoted: ExtractionQuotedTweet | None) -> str | None:
    if quoted is None:
        return None
    if not quoted.tweet_id.isdigit() or len(quoted.tweet_id) > 32:
        return "invalid quoted tweet identity"
    if (
        not quoted.url
        or len(quoted.url) > 2048
        or not quoted.url.startswith(("https://", "http://"))
    ):
        return "invalid quoted tweet url"
    if quoted.tweet_id not in quoted.url:
        return "quoted tweet identity mismatch"
    if not _is_valid_numeric_id(quoted.user_id):
        return "invalid quoted author identity"
    if quoted.tweet_type is not None and quoted.tweet_type not in {"post", "reply", "quote"}:
        return "invalid quoted tweet type"
    return None


def validate_candidate(candidate: DiscoveryCandidate) -> str | None:
    """Return a stable rejection reason for one discovery candidate."""
    if not candidate.tweet_id.isdigit() or len(candidate.tweet_id) > 32:
        return "invalid candidate tweet identity"
    if (
        not candidate.url
        or len(candidate.url) > 2048
        or not candidate.url.startswith(("https://", "http://"))
    ):
        return "invalid candidate url"
    if candidate.tweet_id not in candidate.url:
        return "candidate identity mismatch"
    if candidate.tweet_type not in ALLOWED_CANDIDATE_TWEET_TYPES:
        return "invalid candidate tweet type"
    if candidate.media_count < 0 or candidate.media_count > 64:
        return "invalid candidate media count"
    if not _is_valid_numeric_id(candidate.user_id):
        return "invalid candidate author identity"
    if candidate.username is not None and not (1 <= len(candidate.username) <= 64):
        return "invalid candidate username"
    return None


def candidate_to_json(candidate: DiscoveryCandidate) -> dict[str, Any]:
    """Serialize one durable discovery candidate."""
    return {
        "tweet_id": candidate.tweet_id,
        "url": candidate.url,
        "created_at": candidate.created_at,
        "tweet_type": candidate.tweet_type,
        "is_repost": candidate.is_repost,
        "has_media": candidate.has_media,
        "media_count": candidate.media_count,
        "user_id": candidate.user_id,
        "username": candidate.username,
    }


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
    quoted = result.quoted_tweet
    return {
        "tweet_id": result.tweet_id,
        "url": result.url,
        "tweet_type": result.tweet_type,
        "text": result.text,
        "username": result.username,
        "display_name": result.display_name,
        "created_at": result.created_at,
        "user_id": result.user_id,
        "reply_to": result.reply_to,
        "quoted_tweet": (
            {
                "tweet_id": quoted.tweet_id,
                "url": quoted.url,
                "username": quoted.username,
                "display_name": quoted.display_name,
                "user_id": quoted.user_id,
                "text": quoted.text,
                "created_at": quoted.created_at,
                "tweet_type": quoted.tweet_type,
            }
            if quoted is not None
            else None
        ),
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
