"""Protocol v2 extraction worker: hello/extract/discover/cancel/shutdown."""

from __future__ import annotations

import json
import queue
import sys
import threading
from pathlib import Path
from typing import Any, Callable, TextIO

from .errors import GalleryDlError
from .extraction import DiscoveryRunner, ExtractionConfig, ExtractionRunner
from .protocol_v2 import (
    REQUIRED_V2_CAPABILITIES,
    SIDECAR_V2_PROTOCOL_VERSION,
    SidecarV2Error,
    candidate_to_json,
    extraction_result_to_json,
    validate_command,
    validate_candidate,
    validate_extraction_result,
)


class ExtractionControl:
    """Cancellation state shared by the command loop and one extraction."""

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
        with self._lock:
            self._active_job = job_id
        self._cancelled.clear()

    def release(self, job_id: str) -> None:
        with self._lock:
            if self._active_job != job_id:
                return
            self._active_job = None
            self._cancelled.clear()

    def stop_reason(self) -> str | None:
        if self._shutdown.is_set():
            return "shutdown"
        if self._cancelled.is_set():
            return "cancel"
        return None

    def request_cancel(self, job_id: str | None = None) -> bool:
        active_job = self.active_job
        if active_job is None:
            return False
        if job_id is not None and job_id != active_job:
            return False
        self._cancelled.set()
        return True

    def request_shutdown(self) -> None:
        self._shutdown.set()

    @property
    def is_shutdown_requested(self) -> bool:
        return self._shutdown.is_set()


def emit_v2(event: dict[str, Any], output: TextIO = sys.stdout) -> None:
    json.dump(event, output, separators=(",", ":"))
    output.write("\n")
    output.flush()

def handle_v2_command(
    command: dict[str, Any],
    output: TextIO = sys.stdout,
    control: ExtractionControl | None = None,
    drain: Callable[[], None] | None = None,
    runner_factory: Callable[[ExtractionConfig], ExtractionRunner] | None = None,
    gallery_dl_executable: str = "gallery-dl",
    proxy: str | None = None,
    timeout_seconds: float | None = None,
    discovery_timeout_seconds: float | None = None,
    discovery_runner_factory: Callable[[ExtractionConfig], DiscoveryRunner] | None = None,
) -> bool:
    rejection = validate_command(command)
    job_id = command.get("job_id", "system")
    request_id = command.get("request_id")
    if rejection:
        emit_v2(
            {
                "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
                "event": "failed",
                "job_id": job_id,
                "request_id": request_id,
                "error_code": "INVALID_COMMAND",
                "error_message": rejection,
            },
            output,
        )
        return True

    command_name = command.get("cmd")
    if command_name == "hello":
        emit_v2(
            {
                "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
                "event": "ready",
                "job_id": job_id,
                "request_id": request_id,
                "capabilities": list(REQUIRED_V2_CAPABILITIES),
            },
            output,
        )
        return True

    if command_name == "extract":
        return handle_extract(
            command,
            output,
            control,
            drain,
            runner_factory,
            gallery_dl_executable,
            proxy,
            timeout_seconds,
        )

    if command_name == "discover":
        return handle_discover(
            command,
            output,
            control,
            drain,
            discovery_runner_factory,
            gallery_dl_executable,
            proxy,
            discovery_timeout_seconds,
        )

    if command_name == "cancel":
        affected = control.request_cancel(str(job_id)) if control is not None else False
        if affected:
            return True
        emit_v2(
            {
                "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
                "event": "failed",
                "job_id": job_id,
                "request_id": request_id,
                "error_code": "CANCELLED",
                "error_message": "no matching extraction is running",
            },
            output,
        )
        return True

    if command_name == "shutdown":
        if control is not None:
            control.request_shutdown()
            if control.active_job is not None:
                return True
        return False

    emit_v2(
        {
            "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
            "event": "failed",
            "job_id": job_id,
            "request_id": request_id,
            "error_code": "UNKNOWN_COMMAND",
            "error_message": "unknown sidecar command",
        },
        output,
    )
    return True



def handle_extract(
    command: dict[str, Any],
    output: TextIO,
    control: ExtractionControl | None,
    drain: Callable[[], None] | None,
    runner_factory: Callable[[ExtractionConfig], ExtractionRunner] | None,
    gallery_dl_executable: str,
    proxy: str | None = None,
    timeout_seconds: float | None = None,
) -> bool:
    job_id = str(command["job_id"])
    request_id = command.get("request_id")
    if control is not None and control.active_job is not None:
        emit_v2(
            {
                "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
                "event": "failed",
                "job_id": job_id,
                "request_id": request_id,
                "error_code": "SIDECAR_BUSY",
                "error_message": "another extraction is already running",
            },
            output,
        )
        return True

    emit_v2(
        {
            "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
            "event": "extraction_started",
            "job_id": job_id,
            "request_id": request_id,
        },
        output,
    )
    try:
        factory = runner_factory or ExtractionRunner
        config_kwargs: dict[str, Any] = {
            "executable": gallery_dl_executable,
            "browser": command.get("browser"),
            "profile": command.get("profile"),
        }
        if proxy:
            config_kwargs["proxy"] = proxy
        if timeout_seconds is not None:
            config_kwargs["timeout_seconds"] = timeout_seconds
        runner = factory(ExtractionConfig(**config_kwargs))
        if control is not None:
            control.begin(job_id)
        emit = lambda event: emit_v2(
            {
                "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
                "job_id": job_id,
                "request_id": request_id,
                **event,
            },
            output,
        )
        kwargs: dict[str, Any] = {"emit": emit}
        if control is not None:
            kwargs.update({"is_cancelled": control.stop_reason, "on_tick": drain})
        extraction = runner.run(str(command["url"]), Path("/tmp/xarchive-v2"), **kwargs)
    except GalleryDlError as error:
        emit_v2(
            {
                "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
                "event": "failed",
                "job_id": job_id,
                "request_id": request_id,
                "error_code": error.code,
                "error_message": error.message,
            },
            output,
        )
        return not (
            error.code == "INTERRUPTED"
            and control is not None
            and control.is_shutdown_requested
        )
    finally:
        if control is not None:
            control.release(job_id)

    rejection = validate_extraction_result(extraction)
    if rejection:
        error = SidecarV2Error("EXTRACTION_RESULT_INVALID", rejection)
        emit_v2(
            {
                "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
                "event": "failed",
                "job_id": job_id,
                "request_id": request_id,
                "error_code": error.code,
                "error_message": error.message,
            },
            output,
        )
        return True

    emit_v2(
        {
            "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
            "event": "extracted",
            "job_id": job_id,
            "request_id": request_id,
            "result": extraction_result_to_json(extraction),
        },
        output,
    )
    return True


def handle_discover(
    command: dict[str, Any],
    output: TextIO,
    control: ExtractionControl | None,
    drain: Callable[[], None] | None,
    discovery_runner_factory: Callable[[ExtractionConfig], DiscoveryRunner] | None,
    gallery_dl_executable: str,
    proxy: str | None = None,
    discovery_timeout_seconds: float | None = None,
) -> bool:
    """Run one account discovery and stream validated candidates."""
    job_id = str(command["job_id"])
    request_id = command.get("request_id")
    if control is not None and control.active_job is not None:
        emit_v2(
            {
                "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
                "event": "failed",
                "job_id": job_id,
                "request_id": request_id,
                "error_code": "SIDECAR_BUSY",
                "error_message": "another extraction is already running",
            },
            output,
        )
        return True

    emit_v2(
        {
            "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
            "event": "discovery_started",
            "job_id": job_id,
            "request_id": request_id,
        },
        output,
    )
    try:
        factory = discovery_runner_factory or DiscoveryRunner
        config_kwargs: dict[str, Any] = {
            "executable": gallery_dl_executable,
            "browser": command.get("browser"),
            "profile": command.get("profile"),
        }
        if proxy:
            config_kwargs["proxy"] = proxy
        if discovery_timeout_seconds is not None:
            config_kwargs["timeout_seconds"] = discovery_timeout_seconds
        runner = factory(ExtractionConfig(**config_kwargs))
        if control is not None:
            control.begin(job_id)
        emit = lambda event: emit_v2(
            {
                "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
                "job_id": job_id,
                "request_id": request_id,
                **event,
            },
            output,
        )
        kwargs: dict[str, Any] = {"emit": emit}
        if control is not None:
            kwargs.update({"is_cancelled": control.stop_reason, "on_tick": drain})
        candidates = runner.run(str(command["url"]), Path("/tmp/xarchive-v2-discovery"), **kwargs)
    except GalleryDlError as error:
        emit_v2(
            {
                "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
                "event": "failed",
                "job_id": job_id,
                "request_id": request_id,
                "error_code": error.code,
                "error_message": error.message,
            },
            output,
        )
        return not (
            error.code == "INTERRUPTED"
            and control is not None
            and control.is_shutdown_requested
        )
    finally:
        if control is not None:
            control.release(job_id)

    for candidate in candidates:
        rejection = validate_candidate(candidate)
        if rejection:
            emit_v2(
                {
                    "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
                    "event": "failed",
                    "job_id": job_id,
                    "request_id": request_id,
                    "error_code": "CANDIDATE_INVALID",
                    "error_message": rejection,
                },
                output,
            )
            return True

    emit_v2(
        {
            "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
            "event": "discovery_completed",
            "job_id": job_id,
            "request_id": request_id,
            "candidates_found": len(candidates),
        },
        output,
    )
    return True


def run_v2_worker(
    input_stream: TextIO = sys.stdin,
    output: TextIO = sys.stdout,
    gallery_dl_executable: str = "gallery-dl",
    proxy: str | None = None,
    timeout_seconds: float | None = None,
    discovery_timeout_seconds: float | None = None,
) -> None:
    commands: queue.Queue[dict[str, Any] | None] = queue.Queue()
    control = ExtractionControl()

    def read_commands() -> None:
        for line in input_stream:
            if not line.strip():
                continue
            try:
                command = json.loads(line)
            except json.JSONDecodeError as error:
                commands.put(
                    {
                        "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
                        "event": "failed",
                        "job_id": "unknown",
                        "error_code": "INVALID_JSON",
                        "error_message": str(error),
                    }
                )
                continue
            commands.put(command)
        commands.put(None)

    reader = threading.Thread(target=read_commands, daemon=True)
    reader.start()

    def process_queued(command: dict[str, Any]) -> bool:
        if "cmd" not in command:
            emit_v2(command, output)
            return True
        return handle_v2_command(
            command,
            output,
            control=control,
            drain=drain,
            gallery_dl_executable=gallery_dl_executable,
            proxy=proxy,
            timeout_seconds=timeout_seconds,
            discovery_timeout_seconds=discovery_timeout_seconds,
        )

    def drain() -> None:
        while True:
            try:
                queued = commands.get_nowait()
            except queue.Empty:
                return
            if queued is None:
                commands.put(None)
                return
            if queued.get("cmd") in {"cancel", "shutdown"}:
                handle_v2_command(
                    queued,
                    output,
                    control=control,
                    drain=drain,
                    gallery_dl_executable=gallery_dl_executable,
                )
            else:
                emit_v2(
                    {
                        "protocol_version": SIDECAR_V2_PROTOCOL_VERSION,
                        "event": "failed",
                        "job_id": queued.get("job_id", "system"),
                        "request_id": queued.get("request_id"),
                        "error_code": "SIDECAR_BUSY",
                        "error_message": "extraction is already running",
                    },
                    output,
                )

    while True:
        command = commands.get()
        if command is None:
            break
        if not process_queued(command):
            break
