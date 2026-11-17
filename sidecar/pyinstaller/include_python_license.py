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


OPENSSL_LICENSE_SHA256 = "7d5450cb2d142651b8afa315b5f238efc805dad827d91ba367d8516bc9d49e7a"


def include_openssl_license(worker_root: Path, license_path: Path) -> bool:
    """Include the upstream OpenSSL 3 licence when a matching DLL name is present."""
    internal = worker_root / "_internal"
    libraries = [p for p in internal.iterdir() if p.is_file() and
                 p.name.lower().startswith("libcrypto-3") and p.suffix.lower() == ".dll"]
    if not libraries:
        return False
    if not license_path.is_file() or digest(license_path) != OPENSSL_LICENSE_SHA256:
        raise ValueError("Missing or altered OpenSSL 3 licence")
    output = worker_root / "LICENSE.OpenSSL.txt"
    shutil.copyfile(license_path, output)
    if digest(output) != OPENSSL_LICENSE_SHA256:
        raise ValueError("Copied OpenSSL licence differs from upstream text")
    return True


def main() -> None:
    if sys.platform != "win32" or len(sys.argv) != 2:
        raise SystemExit("Usage on Windows: python include_python_license.py <worker-directory>")
    runtime_name = f"python{sys.version_info.major}{sys.version_info.minor}.dll"
    value = include_python_license(Path(sys.base_prefix), Path(sys.argv[1]), runtime_name)
    print(f"Python runtime licence SHA-256: {value}")
    license_path = Path(__file__).resolve().parents[1] / "third-party" / "openssl" / "LICENSE"
    if include_openssl_license(Path(sys.argv[1]), license_path):
        print(f"OpenSSL 3 licence SHA-256: {OPENSSL_LICENSE_SHA256}")


if __name__ == "__main__":
    main()
