import io
import json
import os
import subprocess
import sys
from pathlib import Path
from unittest.mock import patch

from xarchive_downloader import DownloadControl, handle_command, run_worker
from xarchive_downloader.errors import GalleryDlError
from xarchive_downloader.models import DownloadedFile, ExtractedTweet


def events_from(text: str) -> list[dict]:
    return [json.loads(line) for line in text.splitlines()]


def test_fake_download_emits_lifecycle_events() -> None:
    output = io.StringIO()

    class FakeRunner:
        def __init__(self, config):
            del config

        def run(self, url, staging_dir, emit):
            del staging_dir
            emit({"event": "metadata", "data": {"tweet_id": "1"}})
            emit(
                {
                    "event": "file",
                    "path": "01.jpg",
                    "size_bytes": 5,
                    "media_type": "photo",
                    "mime_type": "image/jpeg",
                }
            )
            return ExtractedTweet(
                tweet_id="1",
                url=url,
                files=(
                    DownloadedFile(
                        relative_path="01.jpg",
                        size_bytes=5,
                        media_type="photo",
                        mime_type="image/jpeg",
                    ),
                ),
            )

    with patch("xarchive_downloader.GalleryDlRunner", FakeRunner):
        handle_command(
            {
                "protocol_version": 1,
                "request_id": "request-1",
                "cmd": "download",
                "job_id": "job-1",
                "url": "https://x.com/example/status/1",
                "staging_dir": "/tmp/job-1",
            },
            output,
        )

    events = events_from(output.getvalue())
    assert [event["event"] for event in events] == [
        "started",
        "metadata",
        "file",
        "progress",
        "complete",
    ]
    assert events[-2]["current"] == 1
    assert events[-1]["files"][0]["relative_path"] == "01.jpg"


def test_worker_stops_after_shutdown() -> None:
    output = io.StringIO()
    run_worker(
        io.StringIO(
            '{"protocol_version":1,"request_id":"r1","cmd":"hello","job_id":"system"}\n'
            '{"protocol_version":1,"request_id":"r2","cmd":"shutdown","job_id":"system"}\n'
        ),
        output,
    )

    assert events_from(output.getvalue())[0]["event"] == "ready"


def test_worker_rejects_unknown_command_fields() -> None:
    output = io.StringIO()
    keep_running = handle_command(
        {
            "protocol_version": 1,
            "request_id": "request-1",
            "cmd": "hello",
            "job_id": "system",
            "executable": "custom-gallery-dl",
        },
        output,
    )

    assert keep_running is True
    assert events_from(output.getvalue()) == [
        {
            "protocol_version": 1,
            "event": "failed",
            "job_id": "system",
            "request_id": "request-1",
            "error_code": "INVALID_COMMAND",
            "error_message": "unknown command field(s): executable",
        }
    ]


def test_worker_reports_missing_gallery_dependency() -> None:
    output = io.StringIO()
    handle_command(
        {
            "protocol_version": 1,
            "request_id": "request-1",
            "cmd": "download",
            "job_id": "job-1",
            "url": "https://x.com/example/status/1",
            "staging_dir": "/tmp/job-1",
        },
        output,
    )
    events = events_from(output.getvalue())
    assert events[0]["event"] == "started"
    assert events[-1]["event"] == "failed"
    assert events[-1]["error_code"] == "SIDECAR_DEPENDENCY_MISSING"


def test_worker_processes_jsonl_over_real_subprocess() -> None:
    package_root = Path(__file__).parents[1]
    environment = os.environ.copy()
    environment["PYTHONPATH"] = str(package_root / "src")
    result = subprocess.run(
        [sys.executable, "-m", "xarchive_downloader"],
        cwd=package_root,
        env=environment,
        input=(
            '{"protocol_version":1,"request_id":"r1","cmd":"hello","job_id":"system"}\n'
            '{"protocol_version":1,"request_id":"r2","cmd":"shutdown","job_id":"system"}\n'
        ),
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout.splitlines()[0])["event"] == "ready"


def test_worker_accepts_gallery_dl_executable_argument() -> None:
    package_root = Path(__file__).parents[1]
    environment = os.environ.copy()
    environment["PYTHONPATH"] = str(package_root / "src")
    result = subprocess.run(
        [sys.executable, "-m", "xarchive_downloader", "--gallery-dl", "/tmp/tools/gallery dl.exe"],
        cwd=package_root,
        env=environment,
        input='{"protocol_version":1,"request_id":"r1","cmd":"hello","job_id":"system"}\n'
        '{"protocol_version":1,"request_id":"r2","cmd":"shutdown","job_id":"system"}\n',
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout.splitlines()[0])["event"] == "ready"


def test_download_control_is_job_scoped_and_distinguishes_shutdown() -> None:
    control = DownloadControl()
    control.begin("job-1")

    assert control.request_cancel("job-2") is False
    assert control.stop_reason() is None
    assert control.request_cancel("job-1") is True
    assert control.stop_reason() == "cancel"

    control.release("job-1")
    control.begin("job-2")
    control.request_shutdown()
    assert control.stop_reason() == "shutdown"


def test_cancel_command_reports_no_matching_job_without_running_download() -> None:
    output = io.StringIO()
    control = DownloadControl()

    assert handle_command(
        {
            "protocol_version": 1,
            "request_id": "request-cancel",
            "cmd": "cancel",
            "job_id": "job-missing",
        },
        output,
        control=control,
    )

    event = events_from(output.getvalue())[0]
    assert event["error_code"] == "CANCELLED"
    assert event["error_message"] == "no matching download is running"


def test_cancel_command_arms_active_download_without_duplicate_terminal_event() -> None:
    output = io.StringIO()
    control = DownloadControl()
    control.begin("job-1")

    assert handle_command(
        {
            "protocol_version": 1,
            "request_id": "request-cancel",
            "cmd": "cancel",
            "job_id": "job-1",
        },
        output,
        control=control,
    )

    assert output.getvalue() == ""
    assert control.stop_reason() == "cancel"


def test_worker_consumes_cancel_during_download_and_continues() -> None:
    output = io.StringIO()

    class FakeRunner:
        def __init__(self, config):
            del config

        def run(self, url, staging_dir, emit, is_cancelled, on_tick):
            del url, staging_dir, emit
            on_tick()
            assert is_cancelled() == "cancel"
            raise GalleryDlError("CANCELLED", "download cancelled by the user")

    commands = (
        '{"protocol_version":1,"request_id":"download","cmd":"download",'
        '"job_id":"job-1","url":"https://x.com/a/status/1","staging_dir":"/tmp/job-1"}\n'
        '{"protocol_version":1,"request_id":"cancel","cmd":"cancel","job_id":"job-1"}\n'
        '{"protocol_version":1,"request_id":"hello","cmd":"hello","job_id":"system"}\n'
    )
    with patch("xarchive_downloader.GalleryDlRunner", FakeRunner):
        run_worker(io.StringIO(commands), output)

    events = events_from(output.getvalue())
    assert [(event["event"], event.get("error_code")) for event in events] == [
        ("started", None),
        ("failed", "SIDECAR_BUSY"),
        ("failed", "CANCELLED"),
    ]


def test_worker_consumes_shutdown_during_download_and_exits() -> None:
    output = io.StringIO()

    class FakeRunner:
        def __init__(self, config):
            del config

        def run(self, url, staging_dir, emit, is_cancelled, on_tick):
            del url, staging_dir, emit
            on_tick()
            assert is_cancelled() == "shutdown"
            raise GalleryDlError("INTERRUPTED", "download interrupted by shutdown")

    commands = (
        '{"protocol_version":1,"request_id":"download","cmd":"download",'
        '"job_id":"job-1","url":"https://x.com/a/status/1","staging_dir":"/tmp/job-1"}\n'
        '{"protocol_version":1,"request_id":"shutdown","cmd":"shutdown","job_id":"system"}\n'
        '{"protocol_version":1,"request_id":"hello","cmd":"hello","job_id":"system"}\n'
    )
    with patch("xarchive_downloader.GalleryDlRunner", FakeRunner):
        run_worker(io.StringIO(commands), output)

    events = events_from(output.getvalue())
    assert [(event["event"], event.get("error_code")) for event in events] == [
        ("started", None),
        ("failed", "SIDECAR_BUSY"),
        ("failed", "INTERRUPTED"),
    ]