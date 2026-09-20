"""U8 packaged entrypoint contract: the Sidecar only starts the v2 worker.

The protocol v1 worker, its `download` command, and the gallery-dl media
download adapter were removed in U8. These tests pin the launcher and the
frozen-artifact entry contract so a legacy worker cannot come back unnoticed.
"""

from __future__ import annotations

import importlib.util
import json
import subprocess
import sys

import xarchive_downloader
from xarchive_downloader import build_argument_parser, main
from xarchive_downloader.protocol_v2 import REQUIRED_V2_CAPABILITIES


def test_package_exposes_the_v2_worker_entrypoint() -> None:
    assert callable(main)
    assert callable(xarchive_downloader.run_v2_worker)


def test_legacy_worker_surface_is_gone() -> None:
    for removed in ("DownloadControl", "handle_command", "run_worker", "GalleryDlRunner"):
        assert not hasattr(xarchive_downloader, removed), f"{removed} must be removed in U8"
    assert importlib.util.find_spec("xarchive_downloader.gallery") is None


def test_argument_parser_defaults_to_path_gallery_dl() -> None:
    parser = build_argument_parser()
    assert parser.parse_args([]).gallery_dl == "gallery-dl"
    assert parser.parse_args(["--gallery-dl", "custom-gallery-dl"]).gallery_dl == (
        "custom-gallery-dl"
    )


def test_main_starts_the_v2_worker_with_the_configured_executable(monkeypatch) -> None:
    captured: dict[str, str] = {}

    def fake_run_v2_worker(*, gallery_dl_executable: str = "gallery-dl") -> None:
        captured["executable"] = gallery_dl_executable

    monkeypatch.setattr(xarchive_downloader, "run_v2_worker", fake_run_v2_worker)
    main(["--gallery-dl", "custom-gallery-dl"])

    assert captured == {"executable": "custom-gallery-dl"}


def test_module_entrypoint_completes_v2_handshake_and_rejects_legacy_command() -> None:
    commands = "\n".join(
        json.dumps(command)
        for command in (
            {
                "protocol_version": 2,
                "request_id": "entrypoint-hello",
                "cmd": "hello",
                "job_id": "system",
            },
            {
                "protocol_version": 1,
                "request_id": "entrypoint-v1",
                "cmd": "download",
                "job_id": "job-1",
                "url": "https://x.com/a/status/1",
                "staging_dir": "/tmp/job-1",
            },
            {
                "protocol_version": 2,
                "request_id": "entrypoint-shutdown",
                "cmd": "shutdown",
                "job_id": "system",
            },
        )
    )
    completed = subprocess.run(
        [sys.executable, "-m", "xarchive_downloader", "--gallery-dl", "fake-gallery-dl"],
        input=f"{commands}\n",
        capture_output=True,
        text=True,
        timeout=60,
        check=False,
    )

    assert completed.returncode == 0, completed.stderr
    events = [json.loads(line) for line in completed.stdout.splitlines() if line.strip()]
    assert events[0]["event"] == "ready"
    assert events[0]["protocol_version"] == 2
    assert set(events[0]["capabilities"]) == set(REQUIRED_V2_CAPABILITIES)
    assert events[1]["event"] == "failed"
    assert events[1]["error_code"] == "INVALID_COMMAND"