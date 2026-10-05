"""Protocol v2 contract tests: typed extraction, handshake and rejection."""

import io
import json
import sys
import tempfile
from dataclasses import replace
from pathlib import Path, PurePath

import pytest

# `PurePath` follows the host flavour, so a POSIX path is not absolute on
# Windows. Fixtures derive a host-absolute staging directory instead of
# hard-coding one, which would assert the wrong rejection reason there.
STAGING_DIR = str(Path(tempfile.gettempdir()) / "job-1")
assert PurePath(STAGING_DIR).is_absolute()

from xarchive_downloader.errors import GalleryDlError
from xarchive_downloader.extraction import (
    DownloadRunner,
    ExtractionConfig,
    ExtractionRunner,
    build_download_command,
    build_extraction_command,
)
from xarchive_downloader.protocol_v2 import (
    DiscoveryCandidate,
    DownloadedMediaFile,
    DownloadResult,
    ExtractionMediaItem,
    ExtractionQuotedTweet,
    ExtractionRequestHeader,
    ExtractionResult,
    candidate_to_json,
    download_result_to_json,
    extraction_result_to_json,
    is_profile_url,
    is_safe_filename,
    looks_like_secret,
    validate_command,
    validate_candidate,
    validate_download_result,
    validate_extraction_result,
)
from xarchive_downloader.worker_v2 import (
    ExtractionControl,
    handle_v2_command,
    run_v2_worker,
)


def sample_result() -> ExtractionResult:
    return ExtractionResult(
        tweet_id="123",
        url="https://x.com/alice/status/123",
        tweet_type="post",
        user_id="9000",
        reply_to="111",
        quoted_tweet=ExtractionQuotedTweet(
            tweet_id="987",
            url="https://x.com/bob/status/987",
            username="bob",
            display_name="Bob",
            user_id="9002",
            text="original",
            tweet_type="post",
        ),
        media=(
            ExtractionMediaItem(
                index=1,
                media_id="m1",
                media_type="photo",
                url="https://cdn.example/1.jpg",
                filename="01.jpg",
                mime_type="image/jpeg",
            ),
        ),
        request_headers=(
            ExtractionRequestHeader(name="Referer", value="https://x.com/"),
        ),
    )


def events_from(text: str) -> list[dict]:
    return [json.loads(line) for line in text.splitlines()]


def test_extraction_command_is_extraction_only() -> None:
    command = build_extraction_command(
        ExtractionConfig(executable="gallery-dl"), "https://x.com/a/status/123"
    )
    assert "--skip-download" in command
    assert "--directory" not in command
    assert command[-1] == "https://x.com/a/status/123"


def test_validate_command_accepts_and_bounds_discover() -> None:
    assert (
        validate_command(
            {
                "protocol_version": 2,
                "request_id": "r1",
                "cmd": "discover",
                "job_id": "batch-1",
                "url": "https://x.com/alice",
            }
        )
        is None
    )
    assert (
        validate_command(
            {
                "protocol_version": 2,
                "request_id": "r1",
                "cmd": "discover",
                "job_id": "batch-1",
                "url": "https://example.com/alice",
            }
        )
        == "invalid account profile url"
    )
    assert is_profile_url("https://x.com/alice/")
    assert not is_profile_url("https://x.com/alice/status/1")
    assert not is_profile_url("https://x.com/a b")


def test_validate_candidate_rejects_identity_mismatch() -> None:
    candidate = DiscoveryCandidate(
        tweet_id="123",
        url="https://x.com/alice/status/123",
        tweet_type="post",
        has_media=True,
        media_count=1,
        user_id="9001",
        username="alice",
    )
    assert validate_candidate(candidate) is None
    payload = candidate_to_json(candidate)
    assert payload["tweet_id"] == "123"
    assert payload["is_repost"] is False
    broken = DiscoveryCandidate(
        tweet_id="123",
        url="https://x.com/alice/status/456",
        tweet_type="retweet",
        media_count=70,
        user_id="not-numeric",
    )
    assert validate_candidate(broken) == "candidate identity mismatch"


def test_v2_handshake_reports_required_capabilities() -> None:
    output = io.StringIO()
    handle_v2_command(
        {
            "protocol_version": 2,
            "request_id": "r1",
            "cmd": "hello",
            "job_id": "system",
        },
        output,
    )
    event = events_from(output.getvalue())[0]
    assert event["event"] == "ready"
    assert event["capabilities"] == [
        "extract_media",
        "download_media",
        "cancel_active_extraction",
        "structured_media_plan",
        "account_discovery",
    ]


def test_v2_rejects_v1_and_unknown_fields() -> None:
    output = io.StringIO()
    handle_v2_command(
        {
            "protocol_version": 1,
            "request_id": "r1",
            "cmd": "download",
            "job_id": "job-1",
            "url": "https://x.com/a/status/1",
            "staging_dir": STAGING_DIR,
        },
        output,
    )
    assert events_from(output.getvalue())[0]["error_code"] == "INVALID_COMMAND"

    output = io.StringIO()
    handle_v2_command(
        {
            "protocol_version": 2,
            "request_id": "r1",
            "cmd": "extract",
            "job_id": "job-1",
            "url": "https://x.com/a/status/1",
            "executable": "custom",
        },
        output,
    )
    assert "unknown command field" in events_from(output.getvalue())[0]["error_message"]


def test_v2_extract_emits_typed_result_without_downloaded_files() -> None:
    output = io.StringIO()

    class FakeRunner:
        def __init__(self, config):
            del config

        def run(self, url, work_dir, emit, is_cancelled=None, on_tick=None):
            del url, work_dir, emit, is_cancelled, on_tick
            return sample_result()

    handle_v2_command(
        {
            "protocol_version": 2,
            "request_id": "r1",
            "cmd": "extract",
            "job_id": "job-1",
            "url": "https://x.com/alice/status/123",
        },
        output,
        control=ExtractionControl(),
        drain=lambda: None,
        runner_factory=FakeRunner,
    )
    events = events_from(output.getvalue())
    assert [event["event"] for event in events] == ["extraction_started", "extracted"]
    assert events[1]["result"]["tweet_id"] == "123"
    assert "downloaded_files" not in json.dumps(events[1])
    assert validate_extraction_result(sample_result()) is None


def test_v2_rejects_cookie_headers_and_unsafe_filenames() -> None:
    assert validate_command({"protocol_version": 2, "cmd": "extract"}) is not None
    assert not is_safe_filename("../escape.jpg")
    assert looks_like_secret("Cookie: session=secret")


def test_v2_worker_consumes_cancel_during_extraction() -> None:
    output = io.StringIO()

    class FakeRunner:
        def __init__(self, config):
            del config

        def run(self, url, work_dir, emit, is_cancelled=None, on_tick=None):
            del url, work_dir, emit
            assert on_tick is not None
            on_tick()
            assert is_cancelled() == "cancel"
            from xarchive_downloader.errors import GalleryDlError

            raise GalleryDlError("CANCELLED", "cancelled")

    commands = (
        '{"protocol_version":2,"request_id":"e1","cmd":"extract",'
        '"job_id":"job-1","url":"https://x.com/a/status/1"}\n'
        '{"protocol_version":2,"request_id":"c1","cmd":"cancel","job_id":"job-1"}\n'
    )
    import unittest.mock as mock

    with mock.patch("xarchive_downloader.worker_v2.ExtractionRunner", FakeRunner):
        run_v2_worker(io.StringIO(commands), output)
    events = events_from(output.getvalue())
    assert events[0]["event"] == "extraction_started"
    assert events[-1]["error_code"] == "CANCELLED"


def test_v2_worker_drains_metadata_events_and_exits_on_eof() -> None:
    import threading

    from xarchive_downloader.worker_v2 import MAX_QUEUED_COMMANDS

    class BurstThenEof(io.StringIO):
        def __init__(self) -> None:
            super().__init__(
                "".join(
                    json.dumps(
                        {
                            "protocol_version": 2,
                            "request_id": f"meta-{index}",
                            "event": "log",
                            "job_id": "job-1",
                            "message": "x" * 64,
                        }
                    )
                    + "\n"
                    for index in range(MAX_QUEUED_COMMANDS * 4)
                )
            )

        def __iter__(self):
            return self

        def __next__(self):
            line = self.readline()
            if line:
                return line
            raise StopIteration

    # A bounded reader must drain the complete burst and terminate on EOF.
    commands = BurstThenEof()
    output = io.StringIO()
    worker = threading.Thread(
        target=run_v2_worker,
        kwargs={"input_stream": commands, "output": output},
    )
    worker.start()
    worker.join(timeout=2)
    assert not worker.is_alive()
    events = events_from(output.getvalue())
    assert sum(event.get("event") == "log" for event in events) == MAX_QUEUED_COMMANDS * 4
    assert not any(event.get("event") == "shutdown" for event in events)


def test_sanitize_filename_neutralizes_paths_control_and_dotdot() -> None:
    from xarchive_downloader.extraction import sanitize_filename

    assert sanitize_filename("/etc/passwd", 1) == "_etc_passwd"
    assert sanitize_filename("..\\escape.jpg", 2) == "_escape.jpg"
    assert sanitize_filename("a..b.jpg", 3) == "a_b.jpg"
    assert sanitize_filename("a\x01b.jpg", 4) == "a_b.jpg"
    assert sanitize_filename("..", 5) == "05.bin"
    assert sanitize_filename(None, 6) == "06.bin"
    assert sanitize_filename("x" * 200, 7) == "07.bin"
    assert is_safe_filename(sanitize_filename("pícture two.jpg", 8))


def test_stable_media_id_prefers_metadata_then_url_then_position() -> None:
    from xarchive_downloader.extraction import stable_media_id
    from xarchive_downloader.models import MediaItem

    with_id = MediaItem(index=1, url="https://cdn.example/a.jpg", media_type="photo", media_id="m1")
    from_url = MediaItem(
        index=2,
        url="https://pbs.twimg.com/media/abc123.jpg?format=jpg&name=large",
        media_type="photo",
    )
    fallback = MediaItem(index=3, url="", media_type="photo")

    assert stable_media_id(with_id, 1) == "m1"
    assert stable_media_id(from_url, 2) == "abc123.jpg"
    assert stable_media_id(fallback, 3) == "media-03"
    # Stable across repeated calls for the same item.
    assert stable_media_id(from_url, 2) == stable_media_id(from_url, 2)


def test_extraction_runner_returns_plan_only_when_media_bytes_were_written(tmp_path) -> None:
    """Extraction-only regression: a gallery-dl that still writes media bytes
    must not turn them into downloaded-file facts in the extraction result."""
    import stat

    work_dir = tmp_path / "work"
    script = tmp_path / "fake-gallery-dl"
    script.write_text(
        "import json\n"
        "from pathlib import Path\n"
        "target = Path.cwd() / 'media_dir'\n"
        "target.mkdir(parents=True, exist_ok=True)\n"
        "(target / '01.jpg').write_bytes(b'MEDIABYTES')\n"
        "(target / 'info.json').write_text(json.dumps({\n"
        "    'tweet_id': '123',\n"
        "    'url': 'https://x.com/alice/status/123',\n"
        "    'username': 'alice',\n"
        "    'text': 'hello',\n"
        "    'media': [{'url': 'https://cdn.example/1.jpg', 'type': 'photo',\n"
        "               'filename': '01.jpg', 'mime_type': 'image/jpeg'}]\n"
        "}), encoding='utf-8')\n",
        encoding="utf-8",
    )
    script.chmod(script.stat().st_mode | stat.S_IXUSR)

    runner = ExtractionRunner(
        ExtractionConfig(
            executable=sys.executable,
            executable_args=(str(script),),
            timeout_seconds=60.0,
        )
    )
    extraction = runner.run("https://x.com/alice/status/123", work_dir)

    # The fake gallery-dl really wrote media bytes.
    assert (work_dir / "media_dir" / "01.jpg").read_bytes() == b"MEDIABYTES"

    # The result is a transfer plan, not downloaded-file facts.
    payload = json.loads(json.dumps(extraction_result_to_json(extraction)))
    assert payload["tweet_id"] == "123"
    assert set(payload) == {
        "tweet_id",
        "url",
        "tweet_type",
        "text",
        "username",
        "display_name",
        "created_at",
        "user_id",
        "reply_to",
        "quoted_tweet",
        "media",
        "request_headers",
    }
    assert payload["media"] == [
        {
            "index": 1,
            "media_id": "1.jpg",
            "media_type": "photo",
            "url": "https://cdn.example/1.jpg",
            "filename": "01.jpg",
            "mime_type": "image/jpeg",
        }
    ]
    assert "files" not in json.dumps(payload)
    assert "MEDIABYTES" not in json.dumps(payload)
    assert not hasattr(extraction, "files")
    assert [item.index for item in extraction.media] == [1]


def test_extraction_runner_classifies_auth_failure_without_result(tmp_path) -> None:
    import stat

    script = tmp_path / "fake-gallery-dl-auth"
    script.write_text(
        "import sys\n"
        "print('login required', file=sys.stderr)\n"
        "raise SystemExit(401)\n",
        encoding="utf-8",
    )
    script.chmod(script.stat().st_mode | stat.S_IXUSR)

    runner = ExtractionRunner(
        ExtractionConfig(
            executable=sys.executable,
            executable_args=(str(script),),
            timeout_seconds=60.0,
        )
    )
    try:
        runner.run("https://x.com/alice/status/123", tmp_path / "work")
    except GalleryDlError as error:
        assert error.code == "AUTH_REQUIRED"
    else:
        raise AssertionError("expected AUTH_REQUIRED failure")


def test_extraction_result_json_has_fixed_schema_keys() -> None:
    payload = extraction_result_to_json(sample_result())
    assert set(payload) == {
        "tweet_id",
        "url",
        "tweet_type",
        "text",
        "username",
        "display_name",
        "created_at",
        "user_id",
        "reply_to",
        "quoted_tweet",
        "media",
        "request_headers",
    }
    assert payload["user_id"] == "9000"
    assert payload["reply_to"] == "111"
    assert payload["quoted_tweet"]["tweet_id"] == "987"
    assert payload["quoted_tweet"]["user_id"] == "9002"
    assert "raw" not in json.dumps(payload)


def test_validate_extraction_result_rejects_invalid_relationship_fields() -> None:
    assert (
        validate_extraction_result(replace(sample_result(), user_id="not-numeric"))
        == "invalid author identity"
    )
    assert (
        validate_extraction_result(replace(sample_result(), reply_to="12x"))
        == "invalid reply identity"
    )
    assert validate_extraction_result(sample_result()) is None
    broken_quoted = replace(
        sample_result(),
        quoted_tweet=ExtractionQuotedTweet(
            tweet_id="987",
            url="https://x.com/bob/status/555",
        ),
    )
    assert validate_extraction_result(broken_quoted) == "quoted tweet identity mismatch"


def sample_download() -> DownloadResult:
    return DownloadResult(
        tweet_id="123",
        url="https://x.com/alice/status/123",
        media=(
            DownloadedMediaFile(
                index=1,
                media_id="m1",
                media_type="photo",
                filename="01.jpg",
                relative_path="01.jpg",
                size_bytes=2048,
                mime_type="image/jpeg",
            ),
        ),
        result=sample_result(),
    )


def test_validate_command_requires_absolute_staging_dir_for_download() -> None:
    base = {
        "protocol_version": 2,
        "request_id": "r1",
        "cmd": "download",
        "job_id": "job-1",
        "url": "https://x.com/alice/status/123",
    }
    # The positive case guards against a validator that rejects everything:
    # every rejection assertion below would still pass while downloads broke.
    assert validate_command({**base, "staging_dir": STAGING_DIR}) is None
    assert validate_command(base) == "staging_dir is required"
    assert (
        validate_command({**base, "staging_dir": "relative/job-1"})
        == "staging_dir must be an absolute path"
    )
    assert validate_command({**base, "staging_dir": ""}) == "staging_dir is required"
    assert (
        validate_command({**base, "job_id": "system", "staging_dir": STAGING_DIR})
        == "download requires a job identity"
    )
    # Only download may carry a staging directory.
    assert (
        validate_command(
            {
                "protocol_version": 2,
                "request_id": "r1",
                "cmd": "extract",
                "job_id": "job-1",
                "url": "https://x.com/alice/status/123",
                "staging_dir": STAGING_DIR,
            }
        )
        == "staging_dir is only allowed for download"
    )


def _download_with_media(**changes: object) -> DownloadResult:
    sample = sample_download()
    return replace(sample, media=(replace(sample.media[0], **changes),))


def test_validate_download_result_rejects_unsafe_paths_and_sizes() -> None:
    assert validate_download_result(sample_download()) is None
    payload = download_result_to_json(sample_download())
    assert payload["tweet_id"] == "123"
    assert payload["media"][0]["relative_path"] == "01.jpg"

    assert (
        validate_download_result(_download_with_media(relative_path="../escape.jpg"))
        == "unsafe downloaded media path"
    )
    assert (
        validate_download_result(_download_with_media(relative_path="/abs.jpg"))
        == "unsafe downloaded media path"
    )
    assert (
        validate_download_result(_download_with_media(relative_path="sub/../x.jpg"))
        == "unsafe downloaded media path"
    )
    assert (
        validate_download_result(_download_with_media(filename=".hidden.jpg"))
        == "unsafe downloaded media filename"
    )
    assert (
        validate_download_result(_download_with_media(size_bytes=0))
        == "invalid downloaded media size"
    )
    assert (
        validate_download_result(_download_with_media(media_type="audio"))
        == "invalid downloaded media type"
    )
    assert (
        validate_download_result(
            replace(sample_download(), result=replace(sample_result(), tweet_id="999"))
        )
        == "request/job identity mismatch"
    )


def test_build_download_command_targets_staging_and_drops_metadata_only_flags(
    tmp_path,
) -> None:
    staging = tmp_path / "job-1"
    command = build_download_command(
        ExtractionConfig(executable="gallery-dl", browser="firefox", profile="main"),
        "https://x.com/alice/status/123",
        staging,
    )
    assert command[:2] == ["gallery-dl", "--config-ignore"]
    assert command[command.index("--directory") + 1] == str(staging)
    assert "--write-info-json" in command
    assert "--cookies-from-browser" in command
    assert command[-1] == "https://x.com/alice/status/123"
    for forbidden in ("--skip-download", "--dump-json", "--simulate"):
        assert forbidden not in command
    with pytest.raises(GalleryDlError) as failure:
        build_download_command(
            ExtractionConfig(), "https://x.com/alice/status/123", Path("relative/job-1")
        )
    assert failure.value.code == "DOWNLOAD_COMMAND_INVALID"


def test_download_runner_reports_staging_relative_media_and_purges_metadata(
    tmp_path, monkeypatch
) -> None:
    staging = tmp_path / "job-1"
    staging.mkdir()
    # A locked leftover from an earlier attempt must not be reported.
    (staging / "00.stale.jpg").write_bytes(b"stale")

    info = {
        "tweet_id": "123",
        "url": "https://x.com/alice/status/123",
        "tweet_type": "post",
        "user_id": "9000",
        "media": [
            {"url": "https://cdn.example/1.jpg", "type": "photo", "media_id": "m1"},
            {"url": "https://cdn.example/2.mp4", "type": "video", "media_id": "m2"},
        ],
    }

    class FakeProcess:
        returncode = 0
        stdout = ""
        stderr = ""

    def fake_run(self, command, work_dir, is_cancelled=None, on_tick=None):
        del self, is_cancelled, on_tick
        assert "--directory" in command
        (work_dir / "01.jpg").write_bytes(b"jpeg-bytes")
        (work_dir / "02.mp4").write_bytes(b"mp4-bytes")
        (work_dir / "info.json").write_text(json.dumps(info), encoding="utf-8")
        return FakeProcess()

    monkeypatch.setattr(ExtractionRunner, "_run_process", fake_run)
    download = DownloadRunner(ExtractionConfig(executable="gallery-dl")).run(
        "https://x.com/alice/status/123", staging
    )

    assert download.tweet_id == "123"
    assert [item.relative_path for item in download.media] == ["01.jpg", "02.mp4"]
    assert [item.media_id for item in download.media] == ["m1", "m2"]
    assert [item.media_type for item in download.media] == ["photo", "video"]
    assert download.media[0].size_bytes == len(b"jpeg-bytes")
    assert download.result is not None
    assert validate_download_result(download) is None
    # gallery-dl metadata is metadata, not media: it never reaches the commit path.
    assert not (staging / "info.json").exists()
    assert not (staging / "00.stale.jpg").exists()


def test_handle_v2_command_emits_typed_download_events(tmp_path) -> None:
    output = io.StringIO()
    staging = tmp_path / "job-1"

    class FakeDownloadRunner:
        def __init__(self, config):
            del config

        def run(self, url, staging_dir, emit=None, is_cancelled=None, on_tick=None):
            del url, emit, is_cancelled, on_tick
            assert staging_dir == staging
            return sample_download()

    handle_v2_command(
        {
            "protocol_version": 2,
            "request_id": "r1",
            "cmd": "download",
            "job_id": "job-1",
            "url": "https://x.com/alice/status/123",
            "staging_dir": str(staging),
        },
        output,
        control=ExtractionControl(),
        drain=lambda: None,
        download_runner_factory=FakeDownloadRunner,
    )
    events = events_from(output.getvalue())
    assert [event["event"] for event in events] == ["download_started", "download_completed"]
    assert events[1]["download"]["media"][0]["relative_path"] == "01.jpg"
    assert events[1]["result"]["tweet_id"] == "123"


def test_handle_v2_command_rejects_download_without_staging_dir() -> None:
    output = io.StringIO()
    handle_v2_command(
        {
            "protocol_version": 2,
            "request_id": "r1",
            "cmd": "download",
            "job_id": "job-1",
            "url": "https://x.com/alice/status/123",
        },
        output,
        control=ExtractionControl(),
        drain=lambda: None,
    )
    event = events_from(output.getvalue())[0]
    assert event["error_code"] == "INVALID_COMMAND"
    assert event["error_message"] == "staging_dir is required"