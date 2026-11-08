"""InternalV1 Sidecar JSONL handshake fixture for non-cgroup lifecycle tests.

This fixture speaks protocol v2 over stdin/stdout JSONL and provides explicit,
timeout-bounded startup/pause/release/exit barriers so tests never rely on
sleep-based synchronization. It is intentionally Sidecar-only: no aria2, no
gallery-dl, no real network, and no cgroup delegation.

Events emitted (one JSON object per line), matching the production sidecar
(`sidecar/src/xarchive_downloader/worker_v2.py`) event vocabulary:
  ready              - in response to `hello`
  extraction_started - in response to `extract`
  extracted          - after a bounded extraction simulation
  download_started   - in response to `download`
  download_completed - after a bounded download simulation
  failed             - on protocol error, cancellation or shutdown

`download` cancellation reports `INTERRUPTED`, which is the code the
production download path raises through `GalleryDlError`. `shutdown` exits
without emitting an acknowledgement because the protocol event enum in
`crates/xarchive-protocol/src/sidecar_v2.rs` defines no such event.

Commands consumed:
  hello, extract, download, cancel, shutdown
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
        self._active_kind: Optional[str] = None
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
            return self._begin(job_id, request_id, "extract", "extraction_started")
        if cmd == "download":
            return self._begin(job_id, request_id, "download", "download_started")
        if cmd == "cancel":
            with self._lock:
                active_job = self._active_job
            if active_job == job_id:
                # Matches production: a cancel for the running job only signals
                # the active operation and emits no event of its own.
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
            # Production exits on shutdown without emitting an event: the
            # protocol event enum defines no `shutdown_ack`.
            return False
        emit({"protocol_version": PROTOCOL_VERSION, "event": "failed", "job_id": job_id, "request_id": request_id, "error_code": "UNKNOWN_COMMAND", "error_message": "unknown sidecar command"})
        return True

    def _begin(self, job_id: str, request_id: Optional[str], kind: str, started_event: str) -> bool:
        with self._lock:
            busy = self._active_job is not None
        if busy:
            emit({"protocol_version": PROTOCOL_VERSION, "event": "failed", "job_id": job_id, "request_id": request_id, "error_code": "SIDECAR_BUSY", "error_message": "another extraction is already running"})
            return True
        with self._lock:
            self._active_job = job_id
            self._active_kind = kind
        self._cancel_requested.clear()
        self._paused.clear()
        self._released.clear()
        self._extraction_done.clear()
        emit({"protocol_version": PROTOCOL_VERSION, "event": started_event, "job_id": job_id, "request_id": request_id})
        self._extraction_thread = threading.Thread(
            target=self._run_work, args=(job_id, request_id, kind), daemon=True
        )
        self._extraction_thread.start()
        return True

    def _run_work(self, job_id: str, request_id: Optional[str], kind: str) -> None:
        deadline = time.monotonic() + 15.0
        while time.monotonic() < deadline and not self._cancel_requested.is_set() and not self._released.wait(0.05):
            pass
        cancelled = self._cancel_requested.is_set()
        if cancelled:
            if kind == "download":
                # Production download cancellation surfaces as GalleryDlError
                # with code INTERRUPTED (see worker_v2.handle_download).
                emit({"protocol_version": PROTOCOL_VERSION, "event": "failed", "job_id": job_id, "request_id": request_id, "error_code": "INTERRUPTED", "error_message": "download interrupted by cancel"})
            else:
                emit({"protocol_version": PROTOCOL_VERSION, "event": "failed", "job_id": job_id, "request_id": request_id, "error_code": "CANCELLED", "error_message": "extraction cancelled by fixture"})
        elif self._released.is_set():
            if kind == "download":
                emit({"protocol_version": PROTOCOL_VERSION, "event": "download_completed", "job_id": job_id, "request_id": request_id, "download": self._download_payload(job_id, request_id), "result": {"tweet_id": "fixture-tweet", "url": "https://x.com/fixture/status/fixture-tweet", "tweet_type": "post", "text": "fixture text", "media": []}})
            else:
                emit({"protocol_version": PROTOCOL_VERSION, "event": "extracted", "job_id": job_id, "request_id": request_id, "result": {"tweet_id": "fixture-tweet", "url": "https://x.com/fixture/status/fixture-tweet", "tweet_type": "post", "text": "fixture text", "media": []}})
        else:
            emit({"protocol_version": PROTOCOL_VERSION, "event": "failed", "job_id": job_id, "request_id": request_id, "error_code": "EXTRACTION_TIMEOUT", "error_message": "fixture work did not complete"})
        with self._lock:
            self._active_job = None
            self._active_kind = None
        self._paused.set()
        self._extraction_done.set()
        thread = threading.current_thread()
        if thread is not self._extraction_thread:
            raise RuntimeError("work completion ran on an unexpected thread")

    def _download_payload(self, job_id: str, request_id: Optional[str]) -> Dict[str, Any]:
        return {
            "job_id": job_id,
            "request_id": request_id,
            "tweet_id": "fixture-tweet",
            "url": "https://x.com/fixture/status/fixture-tweet",
            "media": [],
            "files": [],
        }

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
