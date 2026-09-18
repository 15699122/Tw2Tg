"""Protocol v2 contract tests: typed extraction, handshake and rejection."""

import io
import json

from xarchive_downloader.errors import GalleryDlError
from xarchive_downloader.extraction import (
    ExtractionConfig,
    ExtractionRunner,
    build_extraction_command,
)
from xarchive_downloader.protocol_v2 import (
    ExtractionMediaItem,
    ExtractionRequestHeader,
    ExtractionResult,
    extraction_result_to_json,
    is_safe_filename,
    looks_like_secret,
    validate_command,
    validate_extraction_result,
)
from xarchive_downloader.worker_v2 import ExtractionControl, handle_v2_command, run_v2_worker


def sample_result() -> ExtractionResult:
    return ExtractionResult(
        tweet_id="123",
        url="https://x.com/alice/status/123",
        tweet_type="post",
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
        "cancel_active_extraction",
        "structured_media_plan",
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
            "staging_dir": "/tmp/job-1",
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
    import subprocess

    work_dir = tmp_path / "work"
    script = tmp_path / "fake-gallery-dl"
    script.write_text(
        "#!/bin/sh\n"
        'mkdir -p media_dir\n'
        "printf 'MEDIABYTES' > media_dir/01.jpg\n"
        "cat > media_dir/info.json <<'JSON'\n"
        '{"tweet_id":"123","url":"https://x.com/alice/status/123",'
        '"username":"alice","text":"hello",'
        '"media":[{"url":"https://cdn.example/1.jpg","type":"photo",'
        '"filename":"01.jpg","mime_type":"image/jpeg"}]}\n'
        "JSON\n"
        "exit 0\n",
        encoding="utf-8",
    )
    script.chmod(script.stat().st_mode | stat.S_IXUSR)

    runner = ExtractionRunner(
        ExtractionConfig(executable=str(script), timeout_seconds=60.0)
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
    script.write_text("#!/bin/sh\necho 'login required' >&2\nexit 401\n", encoding="utf-8")
    script.chmod(script.stat().st_mode | stat.S_IXUSR)

    runner = ExtractionRunner(
        ExtractionConfig(executable=str(script), timeout_seconds=60.0)
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
        "media",
        "request_headers",
    }
    assert "raw" not in json.dumps(payload)

