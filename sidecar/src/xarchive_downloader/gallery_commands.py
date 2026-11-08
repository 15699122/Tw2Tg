"""gallery-dl invocation description: configuration, argv and child environment.

This module decides *what* command to run; it never runs it. Subprocess
lifecycle, timeouts, cancellation and cleanup stay in
:mod:`xarchive_downloader.extraction`, so a command can be reviewed and tested
without starting a process.
"""

from __future__ import annotations

import os
from dataclasses import dataclass
from pathlib import Path

from .errors import GalleryDlError

#: Environment variables a child process would read as a proxy source. This
#: mirrors the Rust ``PROXY_ENVIRONMENT_KEYS`` so a `direct` mode removes the same
#: set on both sides of the process boundary.
PROXY_ENVIRONMENT_KEYS: tuple[str, ...] = (
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "http_proxy",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
    "NO_PROXY",
    "no_proxy",
    "XARCHIVE_PROXY",
    "XARCHIVE_PROXY_MODE",
)

#: Uppercase view of the same set, used to match inherited names regardless of
#: case. Windows treats environment variable names case-insensitively.
_PROXY_ENVIRONMENT_KEYS_UPPER = frozenset(key.upper() for key in PROXY_ENVIRONMENT_KEYS)


@dataclass(frozen=True)
class ExtractionConfig:
    executable: str = "gallery-dl"
    executable_args: tuple[str, ...] = ()
    browser: str | None = None
    profile: str | None = None
    proxy: str | None = None
    timeout_seconds: float = 300.0
    #: ``system``, ``direct`` or ``manual``, supplied by the Desktop through
    #: ``XARCHIVE_PROXY_MODE``. It travels with the worker so the mode applies to
    #: every gallery-dl child, not only to the worker's own requests.
    proxy_mode: str = "system"


def spawn_env(proxy: str | None, mode: str | None = None) -> dict[str, str] | None:
    """Return the child environment that honors the configured proxy mode.

    The Desktop sends ``XARCHIVE_PROXY_MODE`` so this side does not have to
    guess. A ``direct`` mode actively removes the inherited variables: setting
    them to an empty string is not equivalent, because gallery-dl and the
    requests library treat an empty value as "configured but unusable" in some
    code paths. ``system`` leaves the environment untouched so gallery-dl can
    discover the platform proxy itself.
    """
    normalized = (mode or "system").strip().lower()
    if normalized == "direct":
        env = os.environ.copy()
        # Windows environment variable names are case-insensitive, so a mixed
        # spelling such as `Http_Proxy` would survive a case-sensitive removal
        # and re-enable the proxy in the child.
        for key in list(env):
            if key.upper() in _PROXY_ENVIRONMENT_KEYS_UPPER:
                env.pop(key)
        return env
    if normalized != "manual" or not proxy:
        # `system` leaves the inherited environment alone so gallery-dl can
        # discover the platform proxy itself, and it must not pin a value even
        # if one is present in the configuration.
        return None
    env = os.environ.copy()
    for key in PROXY_ENVIRONMENT_KEYS:
        env[key] = proxy
    return env



def build_extraction_command(config: ExtractionConfig, url: str) -> list[str]:
    """Build a deterministic extraction-only gallery-dl command.

    The command must never contain media download flags: extraction-only
    means gallery-dl discovers media and writes metadata, never media bytes.
    """
    command = [
        config.executable,
        *config.executable_args,
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
    ]
    if config.browser:
        browser = config.browser
        if config.profile:
            browser = f"{browser}:{config.profile}"
        command.extend(["--cookies-from-browser", browser])
    command.append(url)
    media_write_flags = {"--directory", "--filename", "--download"}
    leaked = media_write_flags.intersection(command)
    if leaked:
        raise GalleryDlError(
            "EXTRACTION_COMMAND_INVALID",
            "extraction command must not contain media download flags: "
            + ", ".join(sorted(leaked)),
        )
    return command


def build_download_command(
    config: ExtractionConfig,
    url: str,
    staging_dir: Path,
    user_args: tuple[str, ...] = (),
) -> list[str]:
    """Build a deterministic gallery-dl command that downloads media bytes.

    This is the optional direct-download path (`use_aria2 = false`). It mirrors
    the extraction command except that it drops the metadata-only flags and
    adds the staging destination, the deterministic media filename, and the
    info.json writer used to recover the extraction result afterwards.
    """
    if not staging_dir.is_absolute():
        raise GalleryDlError(
            "DOWNLOAD_COMMAND_INVALID",
            "staging directory must be an absolute path",
        )
    command = [
        config.executable,
        *config.executable_args,
        # Already-validated task argv entries come before every application-owned
        # flag so a repeated destination or filename option cannot displace the
        # staging contract.
        *user_args,
        "--config-ignore",
        "--no-input",
        "--quiet",
        # Metadata is written next to the media so the extraction result can be
        # recovered without a second gallery-dl invocation.
        "--write-info-json",
        "--directory",
        str(staging_dir),
        "--filename",
        "{num:>02}.{extension}",
        "-o",
        "extractor.twitter.text-tweets=true",
    ]
    if config.browser:
        browser = config.browser
        if config.profile:
            browser = f"{browser}:{config.profile}"
        command.extend(["--cookies-from-browser", browser])
    command.append(url)
    # A download command must never carry the metadata-only or simulation
    # flags: they would keep gallery-dl from writing media bytes into staging.
    forbidden = {"--skip-download", "--dump-json", "--resolve-json", "--simulate"}
    leaked = forbidden.intersection(command)
    if leaked:
        raise GalleryDlError(
            "DOWNLOAD_COMMAND_INVALID",
            "download command must not contain metadata-only flags: "
            + ", ".join(sorted(leaked)),
        )
    return command
