import io
import json

from xarchive_downloader import handle_command, run_worker


def events_from(text: str) -> list[dict]:
    return [json.loads(line) for line in text.splitlines()]


def test_fake_download_emits_lifecycle_events() -> None:
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

    assert [event["event"] for event in events_from(output.getvalue())] == [
        "started",
        "metadata",
        "progress",
        "complete",
    ]


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