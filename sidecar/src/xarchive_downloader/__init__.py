"""XArchive Python Sidecar JSONL worker."""

from __future__ import annotations

import json
import sys
from typing import Any, TextIO

PROTOCOL_VERSION = 1


def emit(event: dict[str, Any], output: TextIO = sys.stdout) -> None:
    """Write exactly one protocol event to stdout and flush it immediately."""
    json.dump(event, output, separators=(",", ":"))
    output.write("\n")
    output.flush()


def handle_command(command: dict[str, Any], output: TextIO = sys.stdout) -> bool:
    """Handle one deterministic Fake Sidecar command.

    The fake worker deliberately creates no files. It validates the JSONL
    lifecycle so the Rust supervisor can be tested before gallery-dl is added.
    Returns False when the worker should stop.
    """
    command_name = command.get("cmd")
    job_id = command.get("job_id", "system")
    request_id = command.get("request_id")

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

        emit({"protocol_version": PROTOCOL_VERSION, "event": "started", "job_id": job_id, "request_id": request_id}, output)
        emit({"protocol_version": PROTOCOL_VERSION, "event": "metadata", "job_id": job_id, "request_id": request_id}, output)
        emit({"protocol_version": PROTOCOL_VERSION, "event": "progress", "job_id": job_id, "request_id": request_id, "current": 1, "total": 1}, output)
        emit({"protocol_version": PROTOCOL_VERSION, "event": "complete", "job_id": job_id, "request_id": request_id}, output)
        return True

    if command_name == "cancel":
        emit({"protocol_version": PROTOCOL_VERSION, "event": "failed", "job_id": job_id, "request_id": request_id, "error_code": "CANCELLED", "error_message": "download cancelled"}, output)
        return True

    if command_name == "shutdown":
        return False

    emit({"protocol_version": PROTOCOL_VERSION, "event": "failed", "job_id": job_id, "request_id": request_id, "error_code": "UNKNOWN_COMMAND", "error_message": "unknown sidecar command"}, output)
    return True


def run_worker(input_stream: TextIO = sys.stdin, output: TextIO = sys.stdout) -> None:
    """Process JSONL commands until EOF or shutdown."""
    for line in input_stream:
        if not line.strip():
            continue
        try:
            command = json.loads(line)
        except json.JSONDecodeError as error:
            emit(
                {
                    "protocol_version": PROTOCOL_VERSION,
                    "event": "failed",
                    "job_id": "unknown",
                    "error_code": "INVALID_JSON",
                    "error_message": str(error),
                },
                output,
            )
            continue
        if not isinstance(command, dict):
            emit(
                {
                    "protocol_version": PROTOCOL_VERSION,
                    "event": "failed",
                    "job_id": "unknown",
                    "error_code": "INVALID_COMMAND",
                    "error_message": "command must be a JSON object",
                },
                output,
            )
            continue
        if not handle_command(command, output):
            break


def main() -> None:
    """Run the JSONL worker."""
    run_worker()