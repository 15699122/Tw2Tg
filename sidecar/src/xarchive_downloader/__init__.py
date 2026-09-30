"""XArchive Python Sidecar package.

The package exposes the protocol v2 worker used by the Desktop supervisor. The
legacy protocol v1 worker, its `download` command, and the gallery-dl media
download adapter were removed in U8; gallery-dl now only performs extraction.
"""

from __future__ import annotations

import argparse

from .worker_v2 import run_v2_worker

__all__ = ["main", "run_v2_worker"]


def build_argument_parser() -> argparse.ArgumentParser:
    """Return the CLI parser shared by the package and PyInstaller entries."""
    parser = argparse.ArgumentParser(description="XArchive Sidecar protocol v2 worker")
    parser.add_argument(
        "--gallery-dl",
        default="gallery-dl",
        help="path to the gallery-dl executable used for extraction",
    )
    parser.add_argument(
        "--proxy",
        default=None,
        help="optional http(s) proxy applied to gallery-dl child processes",
    )
    parser.add_argument(
        "--timeout-seconds",
        type=float,
        default=300.0,
        help="per-extraction timeout in seconds",
    )
    parser.add_argument(
        "--discovery-timeout-seconds",
        type=float,
        default=3600.0,
        help="per-account discovery timeout in seconds",
    )
    return parser


def main(argv: list[str] | None = None) -> None:
    """Run the protocol v2 JSONL worker."""
    args = build_argument_parser().parse_args(argv)
    run_v2_worker(
        gallery_dl_executable=args.gallery_dl,
        proxy=args.proxy,
        timeout_seconds=args.timeout_seconds,
        discovery_timeout_seconds=args.discovery_timeout_seconds,
    )


if __name__ == "__main__":
    main()
