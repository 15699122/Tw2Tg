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
class QuotedTweet:
    tweet_id: str
    url: str
    username: str | None = None
    display_name: str | None = None
    user_id: str | None = None
    text: str | None = None
    created_at: str | None = None
    tweet_type: str | None = None


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
    reply_to_tweet_id: str | None = None
    quoted_tweet: QuotedTweet | None = None
    media: tuple[MediaItem, ...] = ()
    files: tuple[DownloadedFile, ...] = ()
    raw: dict[str, Any] = field(default_factory=dict)


def _first(data: dict[str, Any], *keys: str) -> Any:
    for key in keys:
        if data.get(key) is not None:
            return data[key]
    return None


def _optional_str(value: Any) -> str | None:
    if value is None:
        return None
    text = str(value).strip()
    return text or None


def _optional_numeric_id(value: Any) -> str | None:
    if value is None:
        return None
    text = str(value).strip()
    return text if text.isdigit() else None


def _normalize_quoted_tweet(data: dict[str, Any]) -> QuotedTweet | None:
    if not isinstance(data, dict):
        return None
    tweet_id = _optional_numeric_id(_first(data, "tweet_id", "status_id", "id"))
    if tweet_id is None:
        return None
    url = _optional_str(_first(data, "url", "tweet_url"))
    if url is None:
        # A quoted reference is only useful when both id and URL survive,
        # so an absent URL means the reference is dropped rather than guessed.
        return None
    return QuotedTweet(
        tweet_id=tweet_id,
        url=url,
        username=_optional_str(_first(data, "username", "user", "author_username")),
        display_name=_optional_str(_first(data, "display_name", "author_name")),
        user_id=_optional_str(_first(data, "user_id", "author_id")),
        text=_optional_str(_first(data, "text", "description")),
        created_at=_optional_str(_first(data, "created_at", "date", "timestamp")),
        tweet_type=_optional_str(data.get("tweet_type")),
    )


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

    quoted_raw = data.get("quoted_tweet") or data.get("quoted_status")
    username_value = _first(data, "username", "user", "author_username")
    display_value = _first(data, "display_name", "author_name")
    user_value = _first(data, "user_id", "author_id")
    created_value = _first(data, "created_at", "date", "timestamp")
    reply_value = _first(
        data,
        "in_reply_to_status_id_str",
        "in_reply_to_status_id",
        "in_reply_to",
        "reply_to_tweet_id",
        "reply_to",
    )

    return ExtractedTweet(
        tweet_id=tweet_id,
        url=str(_first(data, "url", "tweet_url") or fallback_url),
        tweet_type=str(data.get("tweet_type") or "post"),
        text=str(_first(data, "text", "description") or ""),
        username=str(username_value) if username_value is not None else None,
        display_name=str(display_value) if display_value is not None else None,
        user_id=str(user_value) if user_value is not None else None,
        created_at=str(created_value) if created_value is not None else None,
        reply_to_tweet_id=_optional_numeric_id(reply_value),
        quoted_tweet=(
            _normalize_quoted_tweet(quoted_raw)
            if isinstance(quoted_raw, dict)
            else None
        ),
        media=tuple(media_items),
        raw=data,
    )