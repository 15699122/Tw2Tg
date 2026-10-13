import os
import sys

import pytest

from xarchive_downloader.cgroup_launcher import kill_and_drain

pytestmark = pytest.mark.skipif(sys.platform != "linux", reason="Linux cgroup adapter")


@pytest.mark.parametrize("events,error", [
    ("populated 0\nfrozen 0\n", None),
    ("populated 1\n", TimeoutError),
    ("", OSError),
    ("populated 2\n", OSError),
    ("populated 0\npopulated 1\n", OSError),
])
def test_drain_state_and_pinned_directory(tmp_path, events, error):
    (tmp_path / "cgroup.kill").write_text("0")
    (tmp_path / "cgroup.events").write_text(events)
    descriptor = os.open(tmp_path, os.O_RDONLY | os.O_DIRECTORY)
    try:
        if error:
            with pytest.raises(error):
                kill_and_drain(descriptor, timeout=0)
        else:
            kill_and_drain(descriptor, timeout=0)
        assert (tmp_path / "cgroup.kill").read_text() == "1"
        os.fstat(descriptor)  # Borrowed descriptor remains caller-owned.
    finally:
        os.close(descriptor)


@pytest.mark.parametrize("timeout", [-1, float("nan"), float("inf")])
def test_invalid_timeout_before_io(timeout):
    with pytest.raises(ValueError):
        kill_and_drain(-1, timeout)


def test_short_kill_write_fails_before_reading_events(tmp_path, monkeypatch):
    (tmp_path / "cgroup.kill").write_text("0")
    directory = os.open(tmp_path, os.O_RDONLY | os.O_DIRECTORY)
    closed = []
    close = os.close

    def record_close(descriptor):
        closed.append(descriptor)
        close(descriptor)

    monkeypatch.setattr(os, "write", lambda descriptor, payload: 0)
    monkeypatch.setattr(os, "close", record_close)
    try:
        with pytest.raises(OSError, match="short cgroup kill write"):
            kill_and_drain(directory)
        assert len(closed) == 1
        os.fstat(directory)
    finally:
        close(directory)


@pytest.mark.parametrize("name", ["cgroup.kill", "cgroup.events"])
def test_control_file_link_is_rejected(tmp_path, name):
    outside = tmp_path / "outside"
    outside.write_text("populated 0\n")
    group = tmp_path / "group"
    group.mkdir()
    (group / "cgroup.kill").write_text("0")
    (group / "cgroup.events").write_text("populated 0\n")
    (group / name).unlink()
    (group / name).symlink_to(outside)
    directory = os.open(group, os.O_RDONLY | os.O_DIRECTORY)
    try:
        with pytest.raises(OSError):
            kill_and_drain(directory)
        assert outside.read_text() == "populated 0\n"
    finally:
        os.close(directory)


def test_drain_uses_original_directory_after_namespace_replacement(tmp_path):
    group = tmp_path / "group"
    group.mkdir()
    (group / "cgroup.kill").write_text("0")
    (group / "cgroup.events").write_text("populated 0\n")
    descriptor = os.open(group, os.O_RDONLY | os.O_DIRECTORY)
    original = tmp_path / "original"
    try:
        group.rename(original)
        group.mkdir()
        (group / "cgroup.kill").write_text("0")
        (group / "cgroup.events").write_text("populated 1\n")
        kill_and_drain(descriptor, timeout=0)
        assert (original / "cgroup.kill").read_text() == "1"
        assert (group / "cgroup.kill").read_text() == "0"
    finally:
        os.close(descriptor)