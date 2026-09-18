"""XArchive Python Sidecar JSONL worker."""

from __future__ import annotations

import json
import queue
import sys
import threading
from pathlib import Path
from typing import Any, Callable, TextIO

from .errors import GalleryDlError
from .gallery import GalleryDlConfig, GalleryDlRunner

PROTOCOL_VERSION = 1
GALLERY_DL_EXECUTABLE: str | None = None
ALLOWED_COMMAND_FIELDS = frozenset(
    {
        "protocol_version",
        "request_id",
        "cmd",
        "job_id",
        "url",
        "staging_dir",
        "browser",
        "profile",
    }
)


class DownloadControl:
    """Cancellation state shared by the command loop and the running download.

    The Desktop can only kill the Sidecar process itself, so a ``cancel`` has to
    reach the in-flight gallery-dl child while the worker is blocked waiting for
    it. This object is written by the control path and read by the download poll
    loop, so a cancel takes effect without waiting for the transfer to finish.
    """

    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._active_job: str | None = None
        self._cancelled = threading.Event()
        self._shutdown = threading.Event()

    @property
    def active_job(self) -> str | None:
        with self._lock:
            return self._active_job

    def begin(self, job_id: str) -> None:
        """Mark ``job_id`` as the single running download."""
        with self._lock:
            self._active_job = job_id
        self._cancelled.clear()

    def release(self, job_id: str) -> None:
        """Clear the running download, but only if it is still the same job.

        The cancel flag is cleared only for the job that owned it, so a stale
        release can never drop a cancel that a newer download already received.
        """
        with self._lock:
            if self._active_job != job_id:
                return
            self._active_job = None
            self._cancelled.clear()

    def stop_reason(self) -> str | None:
        """Return why an in-flight download must stop, or ``None`` to continue."""
        if self._shutdown.is_set():
            return "shutdown"
        if self._cancelled.is_set():
            return "cancel"
        return None

    def is_stopped(self) -> bool:
        return self.stop_reason() is not None

    def request_cancel(self, job_id: str | None = None) -> bool:
        """Cancel the active download; returns whether a download was affected."""
        active_job = self.active_job
        if active_job is None:
            return False
        if job_id is not None and job_id != active_job:
            return False
        self._cancelled.set()
        return True

    def request_shutdown(self) -> None:
        """Stop the active download and refuse any further work."""
        self._shutdown.set()

    @property
    def is_shutdown_requested(self) -> bool:
        return self._shutdown.is_set()



def emit(event: dict[str, Any], output: TextIO = sys.stdout) -> None:
    """Write exactly one protocol event to stdout and flush it immediately."""
    json.dump(event, output, separators=(",", ":"))
    output.write("\n")
    output.flush()


def handle_command(
    command: dict[str, Any],
    output: TextIO = sys.stdout,
    control: DownloadControl | None = None,
    drain: Callable[[], None] | None = None,
) -> bool:
    """Handle one Sidecar command and emit only JSONL protocol events.

    ``control`` carries cancellation state between the command loop and the
    running download. ``drain`` is invoked on every poll tick of a running
    download so queued ``cancel``/``shutdown`` commands take effect immediately
    instead of after gallery-dl finishes.
    """
    command_name = command.get("cmd")
    job_id = command.get("job_id", "system")
    request_id = command.get("request_id")

    unknown_fields = sorted(set(command) - ALLOWED_COMMAND_FIELDS)
    if unknown_fields:
        emit(
            {
                "protocol_version": PROTOCOL_VERSION,
                "event": "failed",
                "job_id": job_id,
                "request_id": request_id,
                "error_code": "INVALID_COMMAND",
                "error_message": "unknown command field(s): " + ", ".join(unknown_fields),
            },
            output,
        )
        return True

    if command.get("protocol_version") != PROTOCOL_VERSION:
        emit(
            {
                "protocol_version": PROTOCOL_VERSION,
                "event": "failed",
                "job_id": job_id,
                "request_id": request_id,
                "error_code": "UNSUPPORTED_PROTOCOL_VERSION",
                "error_message": "unsupported protocol version",
            },
            output,
        )
        return True

    if command_name == "hello":
        emit(
            {
                "protocol_version": PROTOCOL_VERSION,
                "event": "ready",
                "job_id": job_id,
                "request_id": request_id,
            },
            output,
        )
        return True

    if command_name == "download":
        if control is not None and control.active_job is not None:
            emit(
                {
                    "protocol_version": PROTOCOL_VERSION,
                    "event": "failed",
                    "job_id": job_id,
                    "request_id": request_id,
                    "error_code": "SIDECAR_BUSY",
                    "error_message": "another download is already running",
                },
                output,
            )
            return True

        if not command.get("url") or not command.get("staging_dir"):
            emit(
                {
                    "protocol_version": PROTOCOL_VERSION,
                    "event": "failed",
                    "job_id": job_id,
                    "request_id": request_id,
                    "error_code": "INVALID_DOWNLOAD_COMMAND",
                    "error_message": "url and staging_dir are required",
                },
                output,
            )
            return True

        emit(
            {
                "protocol_version": PROTOCOL_VERSION,
                "event": "started",
                "job_id": job_id,
                "request_id": request_id,
            },
            output,
        )
        try:
            runner = GalleryDlRunner(
                GalleryDlConfig(
                    executable=GALLERY_DL_EXECUTABLE or "gallery-dl",
                    browser=command.get("browser"),
                    profile=command.get("profile"),
                )
            )
            if control is not None:
                control.begin(str(job_id))
            runner_kwargs = {
                "emit": lambda event: emit(
                    {
                        "protocol_version": PROTOCOL_VERSION,
                        "job_id": job_id,
                        "request_id": request_id,
                        **event,
                    },
                    output,
                )
            }
            if control is not None:
                runner_kwargs.update(
                    {
                        "is_cancelled": control.stop_reason,
                        "on_tick": drain,
                    }
                )
            try:
                tweet = runner.run(
                    str(command["url"]),
                    Path(str(command["staging_dir"])),
                    **runner_kwargs,
                )
            finally:
                if control is not None:
                    control.release(str(job_id))
        except GalleryDlError as error:
            emit(
                {
                    "protocol_version": PROTOCOL_VERSION,
                    "event": "failed",
                    "job_id": job_id,
                    "request_id": request_id,
                    "error_code": error.code,
                    "error_message": error.message,
                },
                output,
            )
            return not (error.code == "INTERRUPTED" and control is not None and control.is_shutdown_requested)
        except (OSError, ValueError) as error:
            emit(
                {
                    "protocol_version": PROTOCOL_VERSION,
                    "event": "failed",
                    "job_id": job_id,
                    "request_id": request_id,
                    "error_code": "SIDECAR_INTERNAL_ERROR",
                    "error_message": str(error),
                },
                output,
            )
            return True
        files = [downloaded_file.__dict__ for downloaded_file in tweet.files]
        emit(
            {
                "protocol_version": PROTOCOL_VERSION,
                "event": "progress",
                "job_id": job_id,
                "request_id": request_id,
                "current": len(files),
                "total": len(files),
            },
            output,
        )
        emit(
            {
                "protocol_version": PROTOCOL_VERSION,
                "event": "complete",
                "job_id": job_id,
                "request_id": request_id,
                "files": files,
            },
            output,
        )
        return True

    if command_name == "cancel":
        affected = control.request_cancel(str(job_id)) if control is not None else False
        if affected:
            # The in-flight runner will emit the authoritative CANCELLED
            # failure after its child process has actually been reaped. Do not
            # emit a second terminal event from the control command itself.
            return True
        emit(
            {
                "protocol_version": PROTOCOL_VERSION,
                "event": "failed",
                "job_id": job_id,
                "request_id": request_id,
                "error_code": "CANCELLED",
                "error_message": "download cancelled" if affected else "no matching download is running",
            },
            output,
        )
        return True

    if command_name == "shutdown":
        if control is not None and control.active_job is not None:
            control.request_shutdown()
            return True
        if control is not None:
            control.request_shutdown()
        return False

    emit({"protocol_version": PROTOCOL_VERSION, "event": "failed", "job_id": job_id, "request_id": request_id, "error_code": "UNKNOWN_COMMAND", "error_message": "unknown sidecar command"}, output)
    return True


def run_worker(input_stream: TextIO = sys.stdin, output: TextIO = sys.stdout) -> None:
    """Process JSONL commands until EOF or shutdown.

    Input is read on a separate thread while a download is running. The
    download polling callback drains that queue, allowing cancel/shutdown to
    reach the child process without waiting for gallery-dl to exit.
    """
    commands: queue.Queue[dict[str, Any] | None] = queue.Queue()
    control = DownloadControl()

    def read_commands() -> None:
        for line in input_stream:
            if not line.strip():
                continue
            try:
                command = json.loads(line)
            except json.JSONDecodeError as error:
                commands.put(
                    {
                        "protocol_version": PROTOCOL_VERSION,
                        "event": "failed",
                        "job_id": "unknown",
                        "error_code": "INVALID_JSON",
                        "error_message": str(error),
                    }
                )
                continue
            if isinstance(command, dict):
                commands.put(command)
            else:
                commands.put(
                    {
                        "protocol_version": PROTOCOL_VERSION,
                        "event": "failed",
                        "job_id": "unknown",
                        "error_code": "INVALID_COMMAND",
                        "error_message": "command must be a JSON object",
                    }
                )
        commands.put(None)

    reader = threading.Thread(target=read_commands, name="sidecar-command-reader", daemon=True)
    reader.start()

    def process_queued(command: dict[str, Any]) -> bool:
        if "cmd" not in command:
            emit(command, output)
            return True
        return handle_command(command, output, control=control, drain=drain)

    def drain() -> None:
        while True:
            try:
                queued = commands.get_nowait()
            except queue.Empty:
                return
            if queued is None:
                # The active download may consume EOF while draining the
                # command queue. Preserve the sentinel for the outer loop so
                # the worker exits after the in-flight command returns instead
                # of waiting forever for another command.
                commands.put(None)
                return
            if queued.get("cmd") in {"cancel", "shutdown"}:
                handle_command(queued, output, control=control, drain=drain)
            else:
                emit(
                    {
                        "protocol_version": PROTOCOL_VERSION,
                        "event": "failed",
                        "job_id": queued.get("job_id", "system"),
                        "request_id": queued.get("request_id"),
                        "error_code": "SIDECAR_BUSY",
                        "error_message": "download is already running",
                    },
                    output,
                )

    while True:
        command = commands.get()
        if command is None:
            break
        if not process_queued(command):
            break


def main() -> None:
    """Run the JSONL worker."""
    global GALLERY_DL_EXECUTABLE
    if "--gallery-dl" in sys.argv:
        index = sys.argv.index("--gallery-dl")
        if index + 1 >= len(sys.argv):
            raise SystemExit("--gallery-dl requires an executable path")
        GALLERY_DL_EXECUTABLE = sys.argv[index + 1]
    run_worker()


if __name__ == "__main__":
    main()