"""Batch E: downloader arguments are validated and never handed to a shell."""

from pathlib import Path
import json
import os
import subprocess
import sys

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


def test_user_args_remain_task_scoped_argv_entries():
    user_args = ("--limit-rate", "2M", "value with spaces; $HOME")
    config = ExtractionConfig(executable="gallery-dl")
    command = build_download_command(
        config, "https://x.com/tweet/1", Path("/tmp/staging"), user_args
    )
    start = command.index("--limit-rate")
    assert command[start : start + len(user_args)] == list(user_args)
    assert command[-1] == "https://x.com/tweet/1"
    assert all(argument not in command for argument in ("|", ">", "&&"))


def test_fake_process_receives_gallery_dl_argv_as_ordered_unicode_entries(tmp_path):
    executable = tmp_path / "fake-gallery-dl.py"
    argv_file = tmp_path / "argv.json"
    executable.write_text(
        "import json, os, sys\n"
        "with open(os.environ['XARCHIVE_TEST_ARGV'], 'w', encoding='utf-8') as stream:\n"
        "    json.dump(sys.argv[1:], stream, ensure_ascii=False)\n",
        encoding="utf-8",
    )
    # Use the current Python executable as the fake child: argv after the script
    # is observed exactly as a process receives it, without shell tokenization.
    user_args = ("--max-tries", "7", "值 with spaces; $HOME")
    command = [
        sys.executable,
        str(executable),
        *build_download_command(
            ExtractionConfig(executable="gallery-dl"),
            "https://x.com/tweet/1",
            tmp_path / "staging",
            user_args,
        )[1:],
    ]
    environment = os.environ.copy()
    environment["XARCHIVE_TEST_ARGV"] = str(argv_file)
    subprocess.run(command, env=environment, check=True, shell=False)

    actual = json.loads(argv_file.read_text(encoding="utf-8"))
    start = actual.index("--max-tries")
    assert actual[start : start + len(user_args)] == list(user_args)
    assert actual.index("--directory") > start
    assert actual[-1] == "https://x.com/tweet/1"


def test_user_args_cannot_displace_application_owned_download_destination():
    # Rejecting application-owned options is the Rust policy boundary's job. This
    # is defence in depth: the application flag is emitted after the task
    # entries, so a task-supplied --directory stays an earlier duplicate value
    # rather than replacing the staging destination.
    config = ExtractionConfig(executable="gallery-dl")
    command = build_download_command(
        config,
        "https://x.com/tweet/1",
        Path("/tmp/staging"),
        ("--directory", "/attacker"),
    )
    first = command.index("--directory")
    second = command.index("--directory", first + 1)
    assert command[first + 1] == "/attacker"
    assert command[second + 1] == "/tmp/staging"


def test_user_args_do_not_change_extraction_command():
    config = ExtractionConfig(executable="gallery-dl")
    command = build_extraction_command(config, "https://x.com/tweet/1")
    assert "--limit-rate" not in command


def test_download_command_does_not_use_shell_metacharacters():
    config = ExtractionConfig(
        executable="gallery-dl",
        executable_args=("--directory", "/tmp/staging"),
    )
    command = build_download_command(config, "https://x.com/tweet/1", Path("/tmp/staging"))
    assert "|" not in command
    assert ">" not in command
    assert "&&" not in command
