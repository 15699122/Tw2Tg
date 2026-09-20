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
    assert "--skip-download" in command
    assert "--write-info-json" in command
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
        "media": [
            {
                "index": 1,
                "media_id": "abc",
                "media_type": "photo",
                "url": "https://pbs.twimg.com/media/abc.jpg",
                "filename": "_evil___sh\u00e6lle name.jpg",
                "mime_type": None,
            },
            {
                "index": 2,
                "media_id": "def.jpg",
                "media_type": "video",
                "url": "https://pbs.twimg.com/media/def.jpg?format=jpg&name=orig",
                "filename": "02.bin",
                "mime_type": None,
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
