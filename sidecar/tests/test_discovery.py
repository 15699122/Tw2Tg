"""P1-B metadata identity selection and P3-B account discovery tests."""

from __future__ import annotations

import io
import json
import stat
import sys
from pathlib import Path

from xarchive_downloader.errors import GalleryDlError
from xarchive_downloader.extraction import (
    DiscoveryRunner,
    ExtractionConfig,
    ExtractionRunner,
    purge_metadata_files,
    read_metadata_matching,
)
from xarchive_downloader.protocol_v2 import DiscoveryCandidate, candidate_to_json
from xarchive_downloader.worker_v2 import ExtractionControl, handle_v2_command

FAKE_GALLERY_DL = """#!/usr/bin/env python3
import json
from pathlib import Path

target = Path.cwd()
meta_dir = target / "twitter" / "alice"
meta_dir.mkdir(parents=True, exist_ok=True)
(meta_dir / "123.info.json").write_text(json.dumps({
    "tweet_id": "123",
    "url": "https://x.com/alice/status/123",
    "username": "alice",
    "user_id": "9001",
    "created_at": "2026-09-01T00:00:00Z",
    "items": [
        {"url": "https://pbs.twimg.com/media/abc.jpg", "type": "animated_gif"},
        {"url": "https://pbs.twimg.com/media/def.jpg", "type": "photo"},
    ],
}), encoding="utf-8")
"""

FAKE_DISCOVERY_GALLERY_DL = """#!/usr/bin/env python3
import json
from pathlib import Path

target = Path.cwd()
target.mkdir(parents=True, exist_ok=True)
(target / "100.info.json").write_text(json.dumps({
    "tweet_id": "100",
    "url": "https://x.com/alice/status/100",
    "username": "alice",
    "user_id": "9001",
    "created_at": "2026-09-02T00:00:00Z",
    "media": [{"url": "https://pbs.twimg.com/media/a.jpg", "type": "photo"}],
}), encoding="utf-8")
(target / "101.info.json").write_text(json.dumps({
    "tweet_id": "101",
    "url": "https://x.com/alice/status/101",
    "username": "alice",
    "user_id": "9001",
    "retweet_id": "555",
    "created_at": "2026-09-03T00:00:00Z",
    "media": [{"url": "https://pbs.twimg.com/media/b.jpg", "type": "photo"}],
}), encoding="utf-8")
(target / "102.info.json").write_text(json.dumps({
    "tweet_id": "102",
    "url": "https://x.com/alice/status/102",
    "username": "alice",
    "created_at": "2026-09-04T00:00:00Z",
}), encoding="utf-8")
(target / "broken.info.json").write_text("{not json", encoding="utf-8")
"""


def _write_fake(tmp_path: Path, name: str, script: str) -> Path:
    path = tmp_path / name
    path.write_text(script, encoding="utf-8")
    path.chmod(path.stat().st_mode | stat.S_IXUSR)
    return path


def test_purge_removes_stale_metadata_before_extraction(tmp_path: Path) -> None:
    work = tmp_path / "work"
    work.mkdir()
    (work / "info.json").write_text(
        json.dumps({"tweet_id": "999", "url": "https://x.com/a/status/999"}),
        encoding="utf-8",
    )
    fake = _write_fake(tmp_path, "fake-gallery-dl", FAKE_GALLERY_DL)
    runner = ExtractionRunner(
        ExtractionConfig(
            executable=sys.executable,
            executable_args=(str(fake),),
            timeout_seconds=30.0,
        )
    )
    extraction = runner.run("https://x.com/alice/status/123", work)
    assert extraction.tweet_id == "123"
    remaining = sorted(path.name for path in work.rglob("*.info.json"))
    assert remaining == ["123.info.json"]


def test_metadata_selection_is_identity_based_not_order_based(tmp_path: Path) -> None:
    work = tmp_path / "work"
    work.mkdir()
    (work / "000-stale.info.json").write_text(
        json.dumps({"tweet_id": "999", "url": "https://x.com/a/status/999"}),
        encoding="utf-8",
    )
    (work / "123.info.json").write_text(
        json.dumps({"tweet_id": "123", "url": "https://x.com/a/status/123"}),
        encoding="utf-8",
    )
    assert purge_metadata_files(work) == 2
    (work / "123.info.json").write_text(
        json.dumps({"tweet_id": "123", "url": "https://x.com/a/status/123"}),
        encoding="utf-8",
    )
    data = read_metadata_matching(work, "https://x.com/a/status/123")
    assert data["tweet_id"] == "123"
    (work / "456.info.json").write_text(
        json.dumps({"tweet_id": "456"}), encoding="utf-8"
    )
    try:
        read_metadata_matching(work, "https://x.com/alice")
    except GalleryDlError as error:
        assert error.code == "METADATA_AMBIGUOUS"
    else:
        raise AssertionError("expected METADATA_AMBIGUOUS")


def test_metadata_selection_rejects_identity_mismatch(tmp_path: Path) -> None:
    work = tmp_path / "work"
    work.mkdir()
    (work / "999.info.json").write_text(
        json.dumps({"tweet_id": "999"}), encoding="utf-8"
    )
    try:
        read_metadata_matching(work, "https://x.com/a/status/123")
    except GalleryDlError as error:
        assert error.code == "METADATA_MISMATCH"
    else:
        raise AssertionError("expected METADATA_MISMATCH")


def test_extraction_supports_items_key_and_gif_media(tmp_path: Path) -> None:
    work = tmp_path / "work"
    fake = _write_fake(tmp_path, "fake-gallery-dl", FAKE_GALLERY_DL)
    runner = ExtractionRunner(
        ExtractionConfig(
            executable=sys.executable,
            executable_args=(str(fake),),
            timeout_seconds=30.0,
        )
    )
    extraction = runner.run("https://x.com/alice/status/123", work)
    assert extraction.user_id == "9001"
    assert [item.media_type for item in extraction.media] == ["unknown", "photo"]



def test_discovery_runner_emits_validated_candidates(tmp_path: Path) -> None:
    work = tmp_path / "discovery"
    work.mkdir(parents=True)
    (work / "old.info.json").write_text(
        json.dumps({"tweet_id": "42", "url": "https://x.com/alice/status/42"}),
        encoding="utf-8",
    )
    fake = _write_fake(tmp_path, "fake-discovery", FAKE_DISCOVERY_GALLERY_DL)
    runner = DiscoveryRunner(
        ExtractionConfig(
            executable=sys.executable,
            executable_args=(str(fake),),
            timeout_seconds=30.0,
        )
    )
    events: list[dict] = []
    candidates = runner.run("https://x.com/alice", work, emit=events.append)

    assert [candidate.tweet_id for candidate in candidates] == ["100", "101", "102"]
    assert candidates[0].user_id == "9001"
    assert candidates[0].has_media and candidates[0].media_count == 1
    assert candidates[1].is_repost and candidates[1].tweet_type == "retweet"
    assert not candidates[2].has_media
    candidate_events = [event for event in events if event["event"] == "candidate"]
    assert len(candidate_events) == 3
    assert candidate_events[0]["candidate"] == candidate_to_json(candidates[0])
    assert "42" not in {candidate.tweet_id for candidate in candidates}


def test_handle_discover_streams_started_candidates_and_completed() -> None:
    output = io.StringIO()

    class FakeDiscoveryRunner:
        def __init__(self, config: ExtractionConfig) -> None:
            self.config = config

        def run(self, url, work_dir, emit=None, is_cancelled=None, on_tick=None):  # noqa: ANN001
            del url, work_dir, is_cancelled, on_tick
            candidate = DiscoveryCandidate(
                tweet_id="100",
                url="https://x.com/alice/status/100",
                tweet_type="post",
                has_media=True,
                media_count=1,
                created_at="2026-09-02T00:00:00Z",
                user_id="9001",
                username="alice",
            )
            if emit:
                emit({"event": "candidate", "candidate": candidate_to_json(candidate)})
            return [candidate]

    control = ExtractionControl()
    continue_loop = handle_v2_command(
        {
            "protocol_version": 2,
            "request_id": "r1",
            "cmd": "discover",
            "job_id": "batch-1",
            "url": "https://x.com/alice",
        },
        output,
        control=control,
        discovery_runner_factory=FakeDiscoveryRunner,
    )
    assert continue_loop is True
    events = [json.loads(line) for line in output.getvalue().splitlines()]
    assert [event["event"] for event in events] == [
        "discovery_started",
        "candidate",
        "discovery_completed",
    ]
    assert events[-1]["candidates_found"] == 1
    assert control.active_job is None


def test_handle_discover_rejects_busy_worker() -> None:
    output = io.StringIO()
    control = ExtractionControl()
    control.begin("other-job")
    handle_v2_command(
        {
            "protocol_version": 2,
            "request_id": "r1",
            "cmd": "discover",
            "job_id": "batch-1",
            "url": "https://x.com/alice",
        },
        output,
        control=control,
    )
    event = json.loads(output.getvalue().splitlines()[0])
    assert event["error_code"] == "SIDECAR_BUSY"
