import os
from pathlib import Path
import subprocess
import sys

import pytest

from xarchive_downloader.cgroup_launcher import launch


@pytest.mark.skipif(sys.platform != "linux", reason="Linux cgroup bootstrap")
@pytest.mark.parametrize("argv", [[], ["python3"], ["./tool"],
                                 [sys.executable, "secret\0argument"]])
def test_invalid_target_is_rejected_before_filesystem_access(argv, monkeypatch):
    def unexpected(*args, **kwargs):
        pytest.fail("invalid target must not access cgroup")

    monkeypatch.setattr(os, "open", unexpected)
    with pytest.raises(ValueError):
        launch("/sys/fs/cgroup/dedicated", argv)


@pytest.mark.skipif(sys.platform != "linux", reason="Linux cgroup bootstrap")
@pytest.mark.parametrize("group", ["relative", "/tmp/group", "/sys/fs/cgroup",
                                        "/sys/fs/cgroup/../escape",
                                        "/sys/fs/cgroup/bad\0name"])
def test_invalid_group_never_executes(group, monkeypatch):
    def unexpected(*args):
        pytest.fail("target must not execute")
    monkeypatch.setattr(os, "execv", unexpected)
    with pytest.raises(ValueError):
        launch(group, [sys.executable, "-c", "pass"])


@pytest.mark.skipif(sys.platform != "linux", reason="Linux cgroup bootstrap")
def test_failed_join_never_starts_target(tmp_path):
    marker = tmp_path / "target-started"
    launcher = Path(__file__).parents[1] / "src/xarchive_downloader/cgroup_launcher.py"
    result = subprocess.run(
        [sys.executable, str(launcher), "/sys/fs/cgroup/tw2tg-nonexistent-bootstrap",
         "--", sys.executable, "-c", f"open({str(marker)!r},'w').write('started')"],
        capture_output=True, text=True, timeout=5,
    )
    assert result.returncode == 125
    assert not marker.exists()
    assert str(marker) not in result.stderr


@pytest.mark.skipif(sys.platform != "linux", reason="Linux cgroup bootstrap")
@pytest.mark.parametrize("failure", ["short-write", "membership", None])
def test_join_verification_precedes_exec(monkeypatch, failure):
    events = []
    descriptors = iter(range(10, 20))

    def opened(path, flags, **kwargs):
        events.append(("open", str(path), flags, kwargs))
        return next(descriptors)

    def written(descriptor, payload):
        events.append(("write", descriptor, payload))
        return len(payload) - 1 if failure == "short-write" else len(payload)

    def read_membership(path, *args, **kwargs):
        assert path == Path("/proc/self/cgroup")
        events.append(("verify",))
        return "0::/other\n" if failure == "membership" else "0::/dedicated\n"

    def executed(program, argv):
        events.append(("exec", program, argv))

    monkeypatch.setattr(os, "open", opened)
    monkeypatch.setattr(os, "write", written)
    monkeypatch.setattr(os, "close", lambda fd: events.append(("close", fd)))
    monkeypatch.setattr(Path, "read_text", read_membership)
    monkeypatch.setattr(os, "execv", executed)
    argv = [sys.executable, "argument with spaces", "--literal=value"]
    if failure:
        with pytest.raises(OSError):
            launch("/sys/fs/cgroup/dedicated", argv)
        assert not any(event[0] == "exec" for event in events)
    else:
        launch("/sys/fs/cgroup/dedicated", argv)
        assert events[-2:] == [("verify",), ("exec", sys.executable, argv)]
    opened_descriptors = [10, 11, 12]
    assert sorted(event[1] for event in events if event[0] == "close") == opened_descriptors
    for event in events:
        if event[0] == "open" and event[1] != "/sys/fs/cgroup":
            assert event[2] & os.O_NOFOLLOW