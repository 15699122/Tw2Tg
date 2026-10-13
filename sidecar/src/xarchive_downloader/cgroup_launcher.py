"""Linux-only trusted bootstrap: join an existing cgroup before exec.

Not a sandbox: same-user tools can migrate out if delegation permits it.
The supervisor must own kill/drain and keep this path gated until integration.
"""

import os
from pathlib import Path
import sys
import math
import time


def kill_and_drain(directory: int, timeout: float = 5.0) -> None:
    """Kill a caller-owned pinned cgroup and confirm drain, or raise.

    The caller retains the directory descriptor and prevents new launches until
    cleanup completes. This function neither closes it nor grants publication.
    """
    if not math.isfinite(timeout) or timeout < 0:
        raise ValueError("drain timeout must be finite and nonnegative")
    descriptor = os.open(
        "cgroup.kill", os.O_WRONLY | os.O_CLOEXEC | os.O_NOFOLLOW, dir_fd=directory,
    )
    try:
        if os.write(descriptor, b"1") != 1:
            raise OSError("short cgroup kill write")
    finally:
        os.close(descriptor)
    deadline = time.monotonic() + timeout
    while True:
        descriptor = os.open(
            "cgroup.events", os.O_RDONLY | os.O_CLOEXEC | os.O_NOFOLLOW,
            dir_fd=directory,
        )
        with os.fdopen(descriptor, "r", encoding="ascii") as stream:
            lines = stream.read(4097)
        if len(lines) > 4096:
            raise OSError("oversized cgroup events")
        populated = [line for line in lines.splitlines() if line.startswith("populated ")]
        if populated == ["populated 0"]:
            return
        if populated != ["populated 1"]:
            raise OSError("invalid cgroup populated state")
        if time.monotonic() >= deadline:
            raise TimeoutError("owned cgroup did not drain")
        time.sleep(min(0.01, max(0, deadline - time.monotonic())))


def launch(group: str, argv: list[str]) -> None:
    if sys.platform != "linux":
        raise ValueError("cgroup launcher requires Linux")
    if "\0" in group or any("\0" in argument for argument in argv):
        raise ValueError("launcher inputs must not contain NUL")
    path = Path(group)
    root = Path("/sys/fs/cgroup")
    if not path.is_absolute() or ".." in path.parts or not path.is_relative_to(root):
        raise ValueError("cgroup must be below /sys/fs/cgroup")
    if path == root or not argv or not Path(argv[0]).is_absolute():
        raise ValueError("dedicated cgroup and absolute executable required")
    # Never create or truncate a regular file masquerading as cgroup.procs.
    flags = os.O_WRONLY | os.O_CLOEXEC | os.O_NOFOLLOW
    directory = os.open(root, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC)
    try:
        for component in path.relative_to(root).parts:
            child = os.open(
                component, os.O_RDONLY | os.O_DIRECTORY | os.O_CLOEXEC | os.O_NOFOLLOW,
                dir_fd=directory,
            )
            os.close(directory)
            directory = child
        descriptor = os.open("cgroup.procs", flags, dir_fd=directory)
        try:
            payload = str(os.getpid()).encode("ascii")
            if os.write(descriptor, payload) != len(payload):
                raise OSError("short cgroup membership write")
        finally:
            os.close(descriptor)
    finally:
        os.close(directory)
    expected = "/" + str(path.relative_to(root))
    membership = Path("/proc/self/cgroup").read_text().splitlines()
    if f"0::{expected}" not in membership:
        raise OSError("cgroup membership verification failed")
    os.execv(argv[0], argv)


def main() -> None:
    if len(sys.argv) < 4 or sys.argv[2] != "--":
        raise SystemExit("usage: cgroup_launcher CGROUP -- ABSOLUTE_EXECUTABLE [ARGS]")
    try:
        launch(sys.argv[1], sys.argv[3:])
    except (OSError, ValueError):
        # Do not echo tool arguments, paths or secrets.
        print("cgroup launcher failed before target startup", file=sys.stderr)
        raise SystemExit(125) from None


if __name__ == "__main__":
    main()