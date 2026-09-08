import json
import tempfile
from pathlib import Path
from unittest.mock import patch

from xarchive_downloader.errors import classify_returncode
from xarchive_downloader.gallery import GalleryDlConfig, GalleryDlRunner, build_command
from xarchive_downloader.models import normalize_metadata


def test_build_command_is_shell_safe_and_uses_exact_staging_directory() -> None:
    with tempfile.TemporaryDirectory() as directory:
        tmp_path = Path(directory)
        command = build_command(
            GalleryDlConfig(executable="gallery-dl.exe", browser="edge", profile="Default"),
            "https://x.com/alice/status/123",
            tmp_path,
        )
    assert command[:2] == ["gallery-dl.exe", "--config-ignore"]
    assert "--directory" in command
    assert str(tmp_path) in command
    assert "--cookies-from-browser" in command
    assert "edge:Default" in command
    assert command[-1] == "https://x.com/alice/status/123"


def test_normalizes_gallery_metadata_without_exposing_internal_shape() -> None:
    tweet = normalize_metadata(
        {
            "tweet_id": 123,
            "tweet_url": "https://x.com/alice/status/123",
            "username": "alice",
            "display_name": "Alice",
            "text": "hello",
            "media": [{"id": "m1", "url": "https://pbs.example/1.jpg", "type": "photo"}],
        },
        "https://x.com/fallback/status/123",
    )
    assert tweet.tweet_id == "123"
    assert tweet.username == "alice"
    assert len(tweet.media) == 1
    assert tweet.media[0].media_id == "m1"


def test_runner_reads_info_json_and_emits_metadata() -> None:
    with tempfile.TemporaryDirectory() as directory:
        tmp_path = Path(directory)
        info = {
            "tweet_id": "123",
            "url": "https://x.com/alice/status/123",
            "username": "alice",
            "media": [{"url": "https://pbs.example/1.jpg", "type": "photo"}],
        }
        (tmp_path / "info.json").write_text(json.dumps(info), encoding="utf-8")

        class Result:
            returncode = 0
            stdout = ""
            stderr = ""

        with patch("xarchive_downloader.gallery.subprocess.run", return_value=Result()):
            tweet = GalleryDlRunner(GalleryDlConfig(executable="fake-gallery-dl")).run(
                "https://x.com/a/status/123", tmp_path
            )
    assert tweet.tweet_id == "123"


def test_runner_reports_downloaded_files_but_skips_metadata_and_control_files() -> None:
    with tempfile.TemporaryDirectory() as directory:
        staging_dir = Path(directory)
        (staging_dir / "info.json").write_text(
            json.dumps({"tweet_id": "123", "media": []}), encoding="utf-8"
        )
        (staging_dir / "01.jpg").write_bytes(b"image")
        (staging_dir / "02.mp4.aria2").write_bytes(b"control")
        (staging_dir / "02.mp4").write_bytes(b"video")

        class Result:
            returncode = 0
            stdout = ""
            stderr = ""

        events: list[dict] = []
        with patch("xarchive_downloader.gallery.subprocess.run", return_value=Result()):
            tweet = GalleryDlRunner(GalleryDlConfig(executable="fake-gallery-dl")).run(
                "https://x.com/a/status/123", staging_dir, events.append
            )

    assert [item.relative_path for item in tweet.files] == ["01.jpg", "02.mp4"]
    assert [event["event"] for event in events] == ["metadata", "file", "file"]


def test_classifies_authentication_failure() -> None:
    error = classify_returncode(1, "HTTP 403: login required or cookies are invalid")
    assert error.code == "AUTH_REQUIRED"