"""Standalone PyInstaller entry point for the XArchive worker.

PyInstaller imports this module as `__main__`, so we must not rely on
`python -m` semantics. Importing `main` explicitly keeps the function
reachable for the hook scanner and for `--help` invocations.
"""

from xarchive_downloader import main

if __name__ == "__main__":
    main()