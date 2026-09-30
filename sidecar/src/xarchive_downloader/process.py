"""Cross-platform termination of a download child process and its descendants.

The Rust Desktop can only kill the direct Sidecar process. Child processes that
the Sidecar spawned (gallery-dl and anything gallery-dl started) therefore need
their own cleanup path inside the worker: cancelling a job must reclaim the
whole subtree so a stopped Job cannot keep writing into its staging directory.

POSIX children are started as their own session so ``os.killpg`` reaches the
whole subtree without ever signalling the Sidecar itself. Windows has no safe
process-group primitive available to pure Python, so the documented ``taskkill
/T`` tree form is used instead.
"""

from __future__ import annotations

import os
import signal
import subprocess

TERMINATE_GRACE_SECONDS = 5.0
# How often the worker wakes up to consume control commands and re-check the
# download deadline while gallery-dl is still transferring. Small enough that a
# user cancel feels immediate, large enough to stay cheap on loopback I/O.
POLL_INTERVAL_SECONDS = 0.1


def detached_spawn_options() -> dict:
    """Return the ``subprocess.Popen`` options that isolate a download child."""
    if os.name == "nt":
        return {}
    return {"start_new_session": True}


def terminate_tree(process: subprocess.Popen, grace_seconds: float = TERMINATE_GRACE_SECONDS) -> None:
    """Terminate ``process`` and its descendants, escalating only when needed."""
    if process.poll() is not None:
        return
    _signal_tree(process, force=False)
    try:
        process.wait(timeout=grace_seconds)
        return
    except subprocess.TimeoutExpired:
        pass
    _signal_tree(process, force=True)
    try:
        process.wait(timeout=grace_seconds)
    except subprocess.TimeoutExpired:
        # The child is unresponsive even to a forced tree kill; fall back to the
        # direct handle so the Sidecar never blocks indefinitely on cleanup.
        process.kill()


def _signal_tree(process: subprocess.Popen, *, force: bool) -> None:
    if os.name == "nt":
        subprocess.run(
            ["taskkill", "/PID", str(process.pid), "/T", "/F"],
            capture_output=True,
            text=True,
            check=False,
        )
        return
    name = signal.SIGKILL if force else signal.SIGTERM
    try:
        os.killpg(process.pid, name)
    except (ProcessLookupError, PermissionError, OSError):
        # The subtree is already gone or the session leader changed ownership;
        # signalling the direct child is the last safe option.
        try:
            process.send_signal(name)
        except (ProcessLookupError, PermissionError, OSError):
            pass