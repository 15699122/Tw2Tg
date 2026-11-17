"""Include the licence for the exact Python runtime bundled in a Windows worker.

This is one component notice; it does not establish the worker's complete
third-party inventory or redistribution obligations.
"""

from __future__ import annotations

import hashlib
from pathlib import Path
import shutil
import sys


def digest(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def include_python_license(runtime_root: Path, worker_root: Path, runtime_name: str) -> str:
    source_runtime = runtime_root / runtime_name
    bundled_runtime = worker_root / "_internal" / runtime_name
    license_path = runtime_root / "LICENSE.txt"
    for path in (source_runtime, bundled_runtime, license_path):
        if not path.is_file() or path.stat().st_size == 0:
            raise ValueError(f"Missing or empty Python runtime input: {path}")
    if digest(source_runtime) != digest(bundled_runtime):
        raise ValueError("Bundled Python runtime does not match the licence source installation")
    expected = digest(license_path)
    output = worker_root / "LICENSE.Python.txt"
    shutil.copyfile(license_path, output)
    if digest(output) != expected:
        raise ValueError("Copied Python licence differs from source")
    return expected


def main() -> None:
    if sys.platform != "win32" or len(sys.argv) != 2:
        raise SystemExit("Usage on Windows: python include_python_license.py <worker-directory>")
    runtime_name = f"python{sys.version_info.major}{sys.version_info.minor}.dll"
    value = include_python_license(Path(sys.base_prefix), Path(sys.argv[1]), runtime_name)
    print(f"Python runtime licence SHA-256: {value}")


if __name__ == "__main__":
    main()
