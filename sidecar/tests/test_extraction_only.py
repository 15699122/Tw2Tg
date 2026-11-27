"""U4 extraction-only regression tests: no media download facts survive."""

import io
import json
import os
import stat
import sys
from pathlib import Path

import pytest

from xarchive_downloader.errors import GalleryDlError
from xarchive_downloader.extraction import (
    ExtractionConfig,
    ExtractionRunner,
    build_extraction_command,
    sanitize_filename,
    stable_media_id,
)
from xarchive_downloader.models import MediaItem
from xarchive_downloader.protocol_v2 import extraction_result_to_json


FAKE_GALLERY_DL = """#!/usr/bin/env python3
import json
import sys
from pathlib import Path

target = Path.cwd()
# Simulate a misbehaving extractor that still writes media bytes next to the
# metadata: extraction-only semantics must ignore these files entirely.
(target / "media").mkdir(parents=True, exist_ok=True)
(target / "media" / "01.jpg").write_bytes(b" pretend-media-bytes ")
meta_dir = target / "twitter" / "alice"
meta_dir.mkdir(parents=True, exist_ok=True)
(meta_dir / "123.info.json").write_text(
    json.dumps(
        {
            "tweet_id": "123",
            "url": "https://x.com/alice/status/123",
            "text": "hello",
            "username": "alice",
            "user_id": "9001",
            "in_reply_to_status_id_str": "111",
            "quoted_status": {
                "tweet_id": "987",
                "url": "https://x.com/bob/status/987",
                "username": "bob",
                "user_id": "9002",
                "text": "original",
                "tweet_type": "post",
            },
            "media": [
                {
                    "url": "https://pbs.twimg.com/media/abc.jpg",
                    "type": "photo",
                    "filename": "../evil/../sh\\u00e6lle name.jpg",
                    "media_id": "abc",
                },
                {"url": "https://pbs.twimg.com/media/def.jpg?format=jpg&name=orig", "type": "video"},
            ],
        }
    ),
    encoding="utf-8",
)
"""


@pytest.fixture()
def fake_gallery_dl(tmp_path: Path) -> Path:
    script = tmp_path / "fake-gallery-dl"
    script.write_text(FAKE_GALLERY_DL, encoding="utf-8")
    script.chmod(script.stat().st_mode | stat.S_IXUSR)
    return script


def test_sanitize_filename_neutralizes_traversal_and_separators() -> None:
    assert sanitize_filename("../evil/shell.jpg", 1) != "../evil/shell.jpg"
    assert "/" not in sanitize_filename("a/b\\c.jpg", 1)
    assert ".." not in sanitize_filename("..hidden..jpg", 1)
    assert sanitize_filename("\x00\x01ctl.jpg", 2).startswith("_")
    assert sanitize_filename(None, 3) == "03.bin"
    assert sanitize_filename("", 4) == "04.bin"
    assert sanitize_filename("x" * 200, 5) == "05.bin"
    assert sanitize_filename("..", 6) == "06.bin"


def test_stable_media_id_prefers_metadata_then_url_segment() -> None:
    from_url = MediaItem(index=1, url="https://cdn/x/def.jpg?format=jpg&name=orig", media_type="photo")
    assert stable_media_id(from_url, 1) == "def.jpg"
    from_id = MediaItem(index=2, url="https://cdn/x/def.jpg", media_type="photo", media_id="abc123")
    assert stable_media_id(from_id, 2) == "abc123"
    fallback = MediaItem(index=3, url="https://cdn/x/", media_type="photo")
    assert stable_media_id(fallback, 3) == "media-03"


def test_extraction_command_never_contains_media_download_flags() -> None:
    command = build_extraction_command(
        ExtractionConfig(executable="gallery-dl", browser="firefox", profile="default"),
        "https://x.com/a/status/123",
    )
    assert "--no-download" in command
    assert "--skip-download" not in command
    assert "--write-info-json" in command
    assert "--dump-json" in command
    assert "output.jsonl=true" in command
    assert "extractor.twitter.text-tweets=true" in command
    for forbidden in ("--directory", "--filename", "--download"):
        assert forbidden not in command
    assert command[-1] == "https://x.com/a/status/123"
    assert "firefox:default" in command


def test_runner_returns_metadata_without_downloaded_file_facts(
    tmp_path: Path, fake_gallery_dl: Path
) -> None:
    work_dir = tmp_path / "work"
    runner = ExtractionRunner(
        ExtractionConfig(
            executable=sys.executable,
            executable_args=(str(fake_gallery_dl),),
            timeout_seconds=30.0,
        )
    )
    extraction = runner.run("https://x.com/alice/status/123", work_dir)

    # Media was actually written by the misbehaving fake, but the durable
    # result must not carry downloaded-file facts.
    assert (work_dir / "media" / "01.jpg").exists()
    assert extraction_result_to_json(extraction) == {
        "tweet_id": "123",
        "url": "https://x.com/alice/status/123",
        "tweet_type": "post",
        "text": "hello",
        "username": "alice",
        "display_name": None,
        "created_at": None,
        "user_id": "9001",
        "reply_to": "111",
        "quoted_tweet": {
            "tweet_id": "987",
            "url": "https://x.com/bob/status/987",
            "username": "bob",
            "display_name": None,
            "user_id": "9002",
            "text": "original",
            "created_at": None,
            "tweet_type": "post",
        },
        "media": [
            {
                "index": 1,
                "media_id": "abc",
                "media_type": "photo",
                "url": "https://pbs.twimg.com/media/abc.jpg",
                "filename": "_evil___sh\u00e6lle name.jpg",
                "mime_type": "image/jpeg",
            },
            {
                "index": 2,
                "media_id": "def.jpg",
                "media_type": "video",
                "url": "https://pbs.twimg.com/media/def.jpg?format=jpg&name=orig",
                "filename": "02.jpg",
                "mime_type": "image/jpeg",
            },
        ],
        "request_headers": [
            {"name": "Referer", "value": "https://x.com/"},
            {"name": "Accept", "value": "*/*"},
        ],
    }


def test_runner_maps_auth_failure_without_download_fallback(
    tmp_path: Path,
) -> None:
    script = tmp_path / "auth-fail-gallery-dl"
    script.write_text(
        "#!/usr/bin/env python3\n"
        "import sys\n"
        "print('authentication required', file=sys.stderr)\n"
        "sys.exit(401)\n",
        encoding="utf-8",
    )
    script.chmod(script.stat().st_mode | stat.S_IXUSR)
    runner = ExtractionRunner(
        ExtractionConfig(executable=sys.executable, executable_args=(str(script),))
    )
    output = io.StringIO()
    with pytest.raises(GalleryDlError) as excinfo:
        runner.run("https://x.com/a/status/123", tmp_path / "w", emit=lambda event: None)
    assert excinfo.value.code == "AUTH_REQUIRED"


def test_runner_detaches_child_stdin_and_fails_fast_on_cli_error(
    tmp_path: Path,
) -> None:
    import time

    script = tmp_path / "stdin-reading-fail"
    script.write_text(
        "#!/usr/bin/env python3\n"
        "import sys\n"
        "_ = sys.stdin.read()\n"
        "print('gallery-dl: error: unrecognized arguments: --bogus', file=sys.stderr)\n"
        "sys.exit(2)\n",
        encoding="utf-8",
    )
    script.chmod(script.stat().st_mode | stat.S_IXUSR)
    runner = ExtractionRunner(
        ExtractionConfig(
            executable=sys.executable,
            executable_args=(str(script),),
            timeout_seconds=10.0,
        )
    )
    started = time.monotonic()
    with pytest.raises(GalleryDlError) as excinfo:
        runner.run("https://x.com/a/status/123", tmp_path / "w")
    elapsed = time.monotonic() - started
    assert elapsed < 3.0
    assert "unrecognized arguments" in excinfo.value.message


def test_to_extraction_result_completes_extension_and_mime_for_x_media() -> None:
    from xarchive_downloader.models import normalize_metadata
    from xarchive_downloader.gallery_metadata import to_extraction_result

    tweet = normalize_metadata(
        {
            "tweet_id": "2125606699356635170",
            "url": "https://x.com/alice/status/2125606699356635170",
            "media": [
                {
                    "media_id": "HXf25EvbUAAcGX2",
                    "type": "photo",
                    "filename": "HXf25EvbUAAcGX2",
                    "extension": "jpg",
                    "url": "https://pbs.twimg.com/media/HXf25EvbUAAcGX2?format=jpg&name=orig",
                },
                {
                    "media_id": "HXf25EvbUAAcGX3",
                    "type": "photo",
                    "filename": "HXf25EvbUAAcGX3",
                    "url": "https://pbs.twimg.com/media/HXf25EvbUAAcGX3?format=png&name=orig",
                },
            ],
        },
        "https://x.com/alice/status/2125606699356635170",
    )
    result = to_extraction_result(tweet, tweet.url)
    assert result.media[0].filename == "HXf25EvbUAAcGX2.jpg"
    assert result.media[0].mime_type == "image/jpeg"
    assert result.media[1].filename == "HXf25EvbUAAcGX3.png"
    assert result.media[1].mime_type == "image/png"
