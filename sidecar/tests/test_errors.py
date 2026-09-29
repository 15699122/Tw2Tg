"""Error sanitisation and bounded output capture contracts (ENG-05 / ENG-12)."""

import sys
import tempfile
from pathlib import Path

import pytest

from xarchive_downloader.errors import (
    MAX_ERROR_MESSAGE_CHARS,
    GalleryDlError,
    classify_returncode,
    sanitize_error_text,
)
from xarchive_downloader.extraction import (
    MAX_CAPTURED_OUTPUT_CHARS,
    ExtractionConfig,
    ExtractionRunner,
    _drain_bounded,
)


def test_sanitize_removes_a_telegram_bot_token() -> None:
    raw = "failed to post 123456789:AAF-abcdefghijklmnopqrstuvwxyz01 to chat"
    cleaned = sanitize_error_text(raw)
    assert "AAF-abcdefghijklmnopqrstuvwxyz01" not in cleaned
    assert "REDACTED" in cleaned


def test_sanitize_removes_bearer_and_authorization_values() -> None:
    cleaned = sanitize_error_text("Authorization: Bearer abcdef0123456789 rejected")
    assert "abcdef0123456789" not in cleaned


def test_sanitize_removes_url_credentials_and_query_secrets() -> None:
    cleaned = sanitize_error_text("GET https://user:hunter2@x.com/i/status/1?token=abcdef123456 failed")
    assert "hunter2" not in cleaned
    assert "abcdef123456" not in cleaned


def test_sanitize_removes_absolute_local_paths() -> None:
    for raw in (
        "could not read C:\\Users\\operator\\AppData\\Local\\cookies.sqlite",
        "could not read /home/operator/.config/gallery-dl/cookies.txt",
    ):
        cleaned = sanitize_error_text(raw)
        assert "operator" not in cleaned, cleaned
        assert "REDACTED_PATH" in cleaned, cleaned


def test_sanitize_bounds_the_message_length() -> None:
    cleaned = sanitize_error_text("x" * (MAX_ERROR_MESSAGE_CHARS * 3))
    assert len(cleaned) <= MAX_ERROR_MESSAGE_CHARS + len("[TRUNCATED]")
    assert cleaned.startswith("[TRUNCATED]")


def test_classify_returncode_sanitizes_the_exposed_message() -> None:
    error = classify_returncode(1, "cookie login failed for 987654321:AAH-secretsecretsecretsecretsecretsecret")
    assert error.code == "AUTH_REQUIRED"
    assert "AAH-secretsecretsecretsecretsecretsecret" not in error.message


def test_classify_returncode_keeps_diagnosable_codes() -> None:
    assert classify_returncode(429, "429 too many requests").code == "RATE_LIMITED"
    assert classify_returncode(1, "tweet not found").code == "TWEET_NOT_FOUND"
    assert classify_returncode(1, "boom").code == "EXTRACT_OR_DOWNLOAD_FAILED"


def test_drain_bounded_retains_only_the_tail() -> None:
    payload = "".join(f"line {index}\n" for index in range(60000))  # ~500 KiB, above the cap
    with tempfile.TemporaryFile(mode="w+", encoding="utf-8", errors="replace") as handle:
        handle.write(payload)
        handle.seek(0)
        drained = _drain_bounded(handle)
    assert 0 < len(drained) <= MAX_CAPTURED_OUTPUT_CHARS
    assert payload.endswith(drained)
    assert not drained.startswith("line 0\n")


def test_run_process_keeps_stderr_bounded(tmp_path: Path) -> None:
    runner = ExtractionRunner(ExtractionConfig(executable=sys.executable))
    command = [sys.executable, "-c", "import sys; sys.stderr.write('x' * 300000)"]
    result = runner._run_process(command, tmp_path, None, None)
    assert result.returncode == 0
    assert len(result.stderr) <= MAX_CAPTURED_OUTPUT_CHARS
    assert result.stderr.endswith("x" * 100)


def test_missing_dependency_message_is_redacted() -> None:
    runner = ExtractionRunner(ExtractionConfig(executable="/home/operator/no-such-gallery-dl"))
    with pytest.raises(GalleryDlError) as raised:
        runner.run("https://x.com/alice/status/1", Path("/tmp/xarchive-test-missing"))
    assert raised.value.code == "SIDECAR_DEPENDENCY_MISSING"
    assert "operator" not in raised.value.message


def test_extract_stderr_emit_is_redacted(tmp_path: Path) -> None:
    script = tmp_path / "chatty-gallery-dl.py"
    script.write_text(
        "import sys\n"
        "sys.stderr.write('Authorization: Bearer topsecrettokenvalue rejected\\n')\n"
        "sys.exit(0)\n",
        encoding="utf-8",
    )
    runner = ExtractionRunner(ExtractionConfig(executable=sys.executable, executable_args=(str(script),)))
    events: list[dict] = []
    # No metadata is produced, so the run ends in a diagnostic error; the log
    # event emitted before that must already be redacted.
    with pytest.raises(GalleryDlError):
        runner.run("https://x.com/alice/status/1", tmp_path, emit=events.append)
    log_messages = [event["message"] for event in events if event.get("event") == "log"]
    assert log_messages, events
    for message in log_messages:
        assert "topsecrettokenvalue" not in message