"""Errors exposed by the gallery-dl adapter."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass
class GalleryDlError(Exception):
    code: str
    message: str
    returncode: int | None = None

    def __str__(self) -> str:
        return f"{self.code}: {self.message}"


def classify_returncode(returncode: int, stderr: str) -> GalleryDlError:
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
    return GalleryDlError(code=code, message=text, returncode=returncode)


def cancelled_error() -> GalleryDlError:
    """Return the stable error used when a user cancel stopped the download."""
    return GalleryDlError(code="CANCELLED", message="download cancelled by the user")


def interrupted_error() -> GalleryDlError:
    """Return the stable error used when shutdown interrupts a download."""
    return GalleryDlError(code="INTERRUPTED", message="download interrupted by shutdown")


def timeout_error() -> GalleryDlError:
    """Return the stable error used when gallery-dl exceeded its time budget."""
    return GalleryDlError(code="DOWNLOAD_TIMEOUT", message="gallery-dl timed out")