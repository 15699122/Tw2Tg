"""Diagnostic escaped-writer check; run inside a disposable delegated service.

systemd-run --user --wait --pipe -p Delegate=yes python3 <absolute-script-path>
This proves kernel kill/drain capability only, not production integration.
"""

import os
from pathlib import Path
import subprocess
import sys
import tempfile
import time


def exercise(parent, kill_writer):
    """Use an exclusively created group; never signal a cached PID on cleanup."""
    group = parent / f"tw2tg-probe-{os.getpid()}-{int(kill_writer)}"
    group.mkdir()
    process = None
    try:
        with tempfile.TemporaryDirectory(prefix="tw2tg-cgroup-") as temporary:
            ready = Path(temporary) / "ready"
            marker = Path(temporary) / "late-write"
            child = (
                "import os,time; from pathlib import Path; os.setsid(); "
                f"Path({str(ready)!r}).write_text(str(os.getpid())); "
                f"time.sleep(2); Path({str(marker)!r}).write_text('escaped')"
            )
            bootstrap = (
                "import subprocess,sys; from pathlib import Path; "
                f"assert '0::/{group.relative_to('/sys/fs/cgroup')!s}' "
                "in Path('/proc/self/cgroup').read_text().splitlines(); "
                "subprocess.Popen([sys.executable,'-c',sys.argv[1]])"
            )
            launcher = Path(__file__).resolve().parents[1] / (
                "sidecar/src/xarchive_downloader/cgroup_launcher.py"
            )
            process = subprocess.Popen([
                sys.executable, str(launcher), str(group), "--",
                sys.executable, "-c", bootstrap, child,
            ])
            assert process.wait(timeout=5) == 0, "bootstrap failed"
            deadline = time.monotonic() + 5
            while not ready.exists() and time.monotonic() < deadline:
                time.sleep(0.01)
            assert ready.exists(), "escaped writer did not start"
            escaped_pid = int(ready.read_text())
            assert os.getsid(escaped_pid) == escaped_pid, "writer must escape session"
            assert "populated 1" in (group / "cgroup.events").read_text()
            assert not marker.exists(), "writer already wrote before kill boundary"
            if kill_writer:
                (group / "cgroup.kill").write_text("1")
            deadline = time.monotonic() + 5
            while "populated 1" in (group / "cgroup.events").read_text():
                assert time.monotonic() < deadline, "group did not drain"
                time.sleep(0.01)
            time.sleep(2.1 if kill_writer else 0)
            assert marker.exists() != kill_writer, "unexpected delayed write result"
    finally:
        # Kill through the owned group, not a PID that could have been reused.
        (group / "cgroup.kill").write_text("1")
        if process is not None:
            process.wait(timeout=5)
        deadline = time.monotonic() + 5
        while "populated 1" in (group / "cgroup.events").read_text():
            if time.monotonic() >= deadline:
                raise TimeoutError("cleanup did not drain owned group")
            time.sleep(0.01)
        group.rmdir()


def main():
    relative = next(
        line.split("::", 1)[1]
        for line in Path("/proc/self/cgroup").read_text().splitlines()
        if line.startswith("0::")
    )
    parent = Path("/sys/fs/cgroup") / relative.lstrip("/")
    exercise(parent, kill_writer=False)
    print("PASS: positive control setsid writer performs delayed write")
    exercise(parent, kill_writer=True)
    print("PASS: setsid writer killed, cgroup drained, late write absent")


if __name__ == "__main__":
    main()