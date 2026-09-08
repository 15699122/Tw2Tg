import io
import json
import os
import subprocess
import sys
from pathlib import Path
from unittest.mock import patch

from xarchive_downloader import handle_command, run_worker
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
            "executable": "/definitely/missing/gallery-dl",
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