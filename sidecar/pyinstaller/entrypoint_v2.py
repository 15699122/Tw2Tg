"""PyInstaller entry point for the Sidecar protocol v2 worker.

PyInstaller imports this module as `__main__`, so the explicit guard keeps the
worker reachable when the frozen executable is started directly.
"""

from __future__ import annotations

from xarchive_downloader import main


if __name__ == "__main__":
    main()