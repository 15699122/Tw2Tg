"""InternalV1 Sidecar JSONL handshake fixture for non-cgroup lifecycle tests.

This fixture speaks protocol v2 over stdin/stdout JSONL and provides explicit,
timeout-bounded startup/pause/release/exit barriers so tests never rely on
sleep-based synchronization. It is intentionally Sidecar-only: no aria2, no
gallery-dl, no real network, and no cgroup delegation.

Events emitted (one JSON object per line):
  ready            - in response to `hello`
  extraction_started - in response to `extract`
  extracted          - after a bounded extraction simulation
  failed             - on protocol error or cancellation
  shutdown_ack       - after `shutdown` completes

Commands consumed:
  hello, extract, cancel, shutdown
"""

from __future__ import annotations

import json
import sys
import threading
import time
from typing import Any, Dict, Optional

PROTOCOL_VERSION = 2
READY_CAPABILITIES = [
    "extract_media",
    "download_media",
    "cancel_active_extraction",
    "structured_media_plan",
    "account_discovery",
]


def emit(event: Dict[str, Any], output=sys.stdout) -> None:
    json.dump(event, output, separators=(",", ":"))
    output.write("\n")
    output.flush()


class InternalV1Worker:
    """A minimal Sidecar-compatible worker with explicit lifecycle barriers."""

    def __init__(self) -> None:
        self._started = threading.Event()
        self._paused = threading.Event()
        self._released = threading.Event()
        self._exited = threading.Event()
        self._active_job: Optional[str] = None
        self._cancel_requested = threading.Event()
        self._lock = threading.Lock()
        self._extraction_thread: Optional[threading.Thread] = None
        self._extraction_done = threading.Event()
        self._extraction_done.set()

    # --- barrier accessors -------------------------------------------------
    def wait_started(self, timeout: float = 10.0) -> bool:
        return self._started.wait(timeout)

    def wait_paused(self, timeout: float = 10.0) -> bool:
        return self._paused.wait(timeout)

    def wait_released(self, timeout: float = 10.0) -> bool:
        return self._released.wait(timeout)

    def wait_exited(self, timeout: float = 10.0) -> bool:
        return self._exited.wait(timeout)

    def release(self) -> None:
        self._released.set()

    # --- command handling --------------------------------------------------
    def handle(self, command: Dict[str, Any]) -> bool:
        cmd = command.get("cmd")
        job_id = str(command.get("job_id", "system"))
        request_id = command.get("request_id")
        if cmd == "hello":
            self._started.set()
            emit({"protocol_version": PROTOCOL_VERSION, "event": "ready", "job_id": job_id, "request_id": request_id, "capabilities": READY_CAPABILITIES})
            return True
        if cmd == "extract":
            with self._lock:
                self._active_job = job_id
            self._cancel_requested.clear()
            self._paused.clear()
            self._released.clear()
            self._extraction_done.clear()
            emit({"protocol_version": PROTOCOL_VERSION, "event": "extraction_started", "job_id": job_id, "request_id": request_id})
            self._extraction_thread = threading.Thread(
                target=self._run_extraction, args=(job_id, request_id), daemon=True
            )
            self._extraction_thread.start()
            return True
        if cmd == "cancel":
            with self._lock:
                active_job = self._active_job
            if active_job == job_id:
                self._cancel_requested.set()
                return True
            emit({"protocol_version": PROTOCOL_VERSION, "event": "failed", "job_id": job_id, "request_id": request_id, "error_code": "CANCELLED", "error_message": "no matching extraction is running"})
            return True
        if cmd == "shutdown":
            with self._lock:
                active_job = self._active_job
            if active_job is not None:
                self._cancel_requested.set()
                if not self._extraction_done.wait(timeout=5.0):
                    emit({"protocol_version": PROTOCOL_VERSION, "event": "failed", "job_id": job_id, "request_id": request_id, "error_code": "SHUTDOWN_TIMEOUT", "error_message": "extraction thread did not stop"})
                    return False
                thread = self._extraction_thread
                if thread is not None:
                    thread.join(timeout=0)
                self._extraction_thread = None
            self._exited.set()
            emit({"protocol_version": PROTOCOL_VERSION, "event": "shutdown_ack", "job_id": job_id, "request_id": request_id})
            return False
        emit({"protocol_version": PROTOCOL_VERSION, "event": "failed", "job_id": job_id, "request_id": request_id, "error_code": "UNKNOWN_COMMAND", "error_message": "unknown sidecar command"})
        return True

    def _run_extraction(self, job_id: str, request_id: Optional[str]) -> None:
        deadline = time.monotonic() + 15.0
        while time.monotonic() < deadline and not self._cancel_requested.is_set() and not self._released.wait(0.05):
            pass
        cancelled = self._cancel_requested.is_set()
        if cancelled:
            emit({"protocol_version": PROTOCOL_VERSION, "event": "failed", "job_id": job_id, "request_id": request_id, "error_code": "CANCELLED", "error_message": "extraction cancelled by fixture"})
        elif self._released.is_set():
            emit({"protocol_version": PROTOCOL_VERSION, "event": "extracted", "job_id": job_id, "request_id": request_id, "result": {"tweet_id": "fixture-tweet", "url": "https://x.com/fixture/status/fixture-tweet", "tweet_type": "post", "text": "fixture text", "media": []}})
        else:
            emit({"protocol_version": PROTOCOL_VERSION, "event": "failed", "job_id": job_id, "request_id": request_id, "error_code": "EXTRACTION_TIMEOUT", "error_message": "fixture extraction did not complete"})
        cancelled = self._cancel_requested.is_set()
        with self._lock:
            self._active_job = None
        self._paused.set()
        self._extraction_done.set()
        if cancelled:
            thread = threading.current_thread()
            if thread is not self._extraction_thread:
                raise RuntimeError("extraction completion ran on an unexpected thread")

    def run(self) -> None:
        while True:
            line = sys.stdin.readline()
            if not line:
                return
            try:
                command = json.loads(line)
            except json.JSONDecodeError:
                continue
            if not self.handle(command):
                return


def main() -> None:
    worker = InternalV1Worker()
    worker.run()


if __name__ == "__main__":
    main()
