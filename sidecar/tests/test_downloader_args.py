"""Batch E: downloader arguments are validated and never handed to a shell."""

from pathlib import Path

import pytest

from xarchive_downloader.extraction import (
    ExtractionConfig,
    build_download_command,
    build_extraction_command,
)


def test_executable_args_are_passed_as_argv_not_shell():
    config = ExtractionConfig(
        executable="gallery-dl",
        executable_args=("--timeout", "30", "--proxy-all", "127.0.0.1:9"),
    )
    command = build_extraction_command(config, "https://x.com/tweet/1")
    # executable_args travel as discrete argv entries; nothing is joined with a shell.
    assert command == [
        "gallery-dl",
        "--timeout",
        "30",
        "--proxy-all",
        "127.0.0.1:9",
        "--config-ignore",
        "--no-input",
        "--quiet",
        "--skip-download",
        "--write-info-json",
        "--dump-json",
        "-o",
        "output.jsonl=true",
        "-o",
        "extractor.twitter.text-tweets=true",
        "https://x.com/tweet/1",
    ]


def test_download_command_does_not_use_shell_metacharacters():
    config = ExtractionConfig(
        executable="gallery-dl",
        executable_args=("--directory", "/tmp/staging"),
    )
    command = build_download_command(config, "https://x.com/tweet/1", Path("/tmp/staging"))
    assert "|" not in command
    assert ">" not in command
    assert "&&" not in command
