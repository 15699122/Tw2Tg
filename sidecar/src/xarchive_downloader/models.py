"""Stable internal models for extractor results.

These models deliberately do not expose gallery-dl's internal extractor
objects. The Sidecar can therefore upgrade its adapter without changing the
Rust protocol or archive database.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any


@dataclass(frozen=True)
class MediaItem:
    index: int
    url: str
    media_type: str
    media_id: str | None = None
    filename: str | None = None
    mime_type: str | None = None
    metadata: dict[str, Any] = field(default_factory=dict)


@dataclass(frozen=True)
class DownloadedFile:
    relative_path: str
    size_bytes: int
    media_type: str
    mime_type: str | None = None


@dataclass(frozen=True)
class ExtractedTweet:
    tweet_id: str
    url: str
    tweet_type: str = "post"
    text: str = ""
    username: str | None = None
    display_name: str | None = None
    user_id: str | None = None
    created_at: str | None = None
    media: tuple[MediaItem, ...] = ()
    files: tuple[DownloadedFile, ...] = ()
    raw: dict[str, Any] = field(default_factory=dict)


def _first(data: dict[str, Any], *keys: str) -> Any:
    for key in keys:
        if data.get(key) is not None:
            return data[key]
    return None


def normalize_metadata(data: dict[str, Any], fallback_url: str) -> ExtractedTweet:
    """Convert one gallery-dl info object to the Sidecar's stable model."""
    tweet_id = str(_first(data, "tweet_id", "status_id", "id") or "")
    if not tweet_id.isdigit():
        raise ValueError("gallery metadata does not contain a numeric tweet id")

    raw_media = data.get("media") or data.get("items") or []
    media_items: list[MediaItem] = []
    if isinstance(raw_media, list):
        for index, item in enumerate(raw_media, 1):
            if not isinstance(item, dict):
                continue
            media_url = _first(item, "url", "src", "media_url")
            if not media_url:
                continue
            media_items.append(
                MediaItem(
                    index=int(item.get("index") or index),
                    url=str(media_url),
                    media_type=str(_first(item, "type", "media_type") or "unknown"),
                    media_id=str(_first(item, "media_id", "id")) if _first(item, "media_id", "id") is not None else None,
                    filename=str(item["filename"]) if item.get("filename") else None,
                    mime_type=str(item["mime_type"]) if item.get("mime_type") else None,
                    metadata=item,
                )
            )

    return ExtractedTweet(
        tweet_id=tweet_id,
        url=str(_first(data, "url", "tweet_url") or fallback_url),
        tweet_type=str(data.get("tweet_type") or "post"),
        text=str(_first(data, "text", "description") or ""),
        username=str(_first(data, "username", "user", "author_username")) if _first(data, "username", "user", "author_username") is not None else None,
        display_name=str(_first(data, "display_name", "author_name")) if _first(data, "display_name", "author_name") is not None else None,
        user_id=str(_first(data, "user_id", "author_id")) if _first(data, "user_id", "author_id") is not None else None,
        created_at=str(_first(data, "created_at", "date", "timestamp")) if _first(data, "created_at", "date", "timestamp") is not None else None,
        media=tuple(media_items),
        raw=data,
    )