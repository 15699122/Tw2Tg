"""Errors exposed by the gallery-dl adapter."""

from __future__ import annotations

import re
from dataclasses import dataclass

# Maximum length of an error message that crosses the protocol boundary.
MAX_ERROR_MESSAGE_CHARS = 2000

# Patterns whose values must never reach SQLite, the UI, or exported diagnostics.
# gallery-dl failures can echo the invoked command, cookie paths, or a request
# URL, so credentials and absolute local paths are removed before the error is
# classified or persisted.
_REDACTIONS: tuple[tuple[re.Pattern[str], str], ...] = (
    # Authorization / Proxy-Authorization headers. These consume the rest of
    # the line, because a header value can itself contain further tokens.
    (
        re.compile(r"(?i)\b(authorization|proxy-authorization)\s*[:=][^\r\n]*"),
        r"\1=[REDACTED]",
    ),
    # Common token shapes: bot tokens, bearer tokens, long opaque secrets.
    (re.compile(r"\b\d{6,12}:[A-Za-z0-9_-]{30,}"), "[REDACTED_BOT_TOKEN]"),
    (re.compile(r"(?i)\bbearer\s+[A-Za-z0-9._~+/=-]{8,}"), "Bearer [REDACTED]"),
    (re.compile(r"\bgh[pousr]_[A-Za-z0-9]{16,}\b"), "[REDACTED_TOKEN]"),
    (re.compile(r"(?i)\b(xoxb|xoxp|sk)-[A-Za-z0-9-]{10,}"), "[REDACTED_TOKEN]"),
    # Cookie material and cookie file paths.
    (re.compile(r"(?i)\b(cookie|cookies|cookies-from-browser)\s*[:=]\s*\S+"), r"\1=[REDACTED]"),
    (re.compile(r"(?i)(--cookies[\w-]*\s+)\S+"), r"\1[REDACTED]"),
    # Credentials embedded in a URL: scheme://user:pass@host
    (re.compile(r"(?i)\b([a-z][a-z0-9+.-]*://)[^/\s:@]+:[^/\s@]+@"), r"\1[REDACTED]@"),
    # Query-string secrets.
    (re.compile(r"(?i)([?&](?:token|auth|key|secret|password|passwd|sig|signature)=)[^&\s]+"), r"\1[REDACTED]"),
    # Absolute local paths that reveal the operator's directory layout.
    (re.compile(r"(?i)([A-Z]:\\[^\s\"']*|/(?:home|Users|root|tmp|var|etc)/[^\s\"']*)"), "[REDACTED_PATH]"),
)


def sanitize_error_text(text: str) -> str:
    """Redact credentials and local paths, then bound the message length.

    This runs before an external tool message is classified, logged, or sent
    over the JSONL protocol, so a failing download cannot leak secrets into
    SQLite rows, the desktop log, or an exported diagnostics bundle.
    """
    cleaned = text
    for pattern, replacement in _REDACTIONS:
        cleaned = pattern.sub(replacement, cleaned)
    cleaned = cleaned.strip()
    if len(cleaned) > MAX_ERROR_MESSAGE_CHARS:
        cleaned = cleaned[-MAX_ERROR_MESSAGE_CHARS:]
        cleaned = "[TRUNCATED]" + cleaned
    return cleaned


@dataclass(frozen=True)
class GalleryDlError(Exception):
    code: str
    message: str
    returncode: int | None = None

    def __str__(self) -> str:
        return f"{self.code}: {self.message}"


def classify_returncode(returncode: int, stderr: str) -> GalleryDlError:
    # Classify on the raw text so redaction cannot change the error code, but
    # only ever expose the sanitized, length-bounded message.
    text = stderr.strip() or f"gallery-dl exited with status {returncode}"
    lowered = text.lower()
    if returncode in (401, 403) or "authentication" in lowered or "login" in lowered or "cookie" in lowered:
        code = "AUTH_REQUIRED"
    elif "429" in lowered or "too many requests" in lowered:
        code = "RATE_LIMITED"
    elif "not found" in lowered or "does not exist" in lowered:
        code = "TWEET_NOT_FOUND"
    else:
        code = "EXTRACT_OR_DOWNLOAD_FAILED"
    return GalleryDlError(code=code, message=sanitize_error_text(text), returncode=returncode)
