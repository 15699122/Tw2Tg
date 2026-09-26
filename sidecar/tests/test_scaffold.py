from xarchive_downloader import main
from xarchive_downloader.errors import (
    MAX_ERROR_MESSAGE_CHARS,
    classify_returncode,
    sanitize_error_text,
)


def test_package_exposes_worker_entrypoint() -> None:
    assert callable(main)


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
