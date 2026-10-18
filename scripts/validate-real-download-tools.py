#!/usr/bin/env python3
"""Run opt-in Linux production transfer/output checks against real tools.

Requires an already-built current Cargo library test executable and an enabled
user systemd manager. Never installs tools or accesses external media/accounts.
Extraction is controlled; both real download engines fetch only loopback bytes.
"""

import argparse
import hashlib
import http.server
import json
import os
from pathlib import Path
import subprocess
import threading


class MediaHandler(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        body = b"real-tool-media"
        self.send_response(200)
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *_args):
        pass


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--aria2", type=Path, required=True)
    parser.add_argument("--gallery-dl", type=Path, required=True)
    parser.add_argument("--evidence", type=Path, required=True)
    args = parser.parse_args()
    for path in (args.binary, args.aria2, args.gallery_dl):
        if not path.is_absolute() or not path.is_file() or not os.access(path, os.X_OK):
            parser.error("each executable must be an existing executable absolute path")
    if not args.evidence.is_absolute():
        parser.error("evidence prefix must be absolute")
    args.evidence.parent.mkdir(parents=True, exist_ok=True)
    if args.evidence.with_suffix(".log").exists():
        parser.error("use a fresh evidence prefix; prior results must be preserved")
    identity = {
        "head": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "status": subprocess.check_output(["git", "status", "--short"], text=True),
        "binary_sha256": hashlib.sha256(args.binary.read_bytes()).hexdigest(),
        "versions": {},
        "scope": "controlled extraction, real tools, loopback media; not live X acceptance",
    }
    for name, program in (("aria2", args.aria2), ("gallery-dl", args.gallery_dl)):
        identity["versions"][name] = subprocess.check_output(
            [str(program), "--version"], text=True, timeout=10
        ).splitlines()[0]
    args.evidence.with_suffix(".json").write_text(json.dumps(identity, indent=2))
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), MediaHandler)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    environment = [
        f"--setenv=TW2TG_TEST_ARIA2={args.aria2}",
        f"--setenv=TW2TG_TEST_GALLERY={args.gallery_dl}",
        f"--setenv=TW2TG_TEST_MEDIA_URL=http://127.0.0.1:{server.server_port}/media.jpg",
    ]
    if os.environ.get("LD_LIBRARY_PATH"):
        environment.append(f"--setenv=LD_LIBRARY_PATH={os.environ['LD_LIBRARY_PATH']}")
    try:
        with args.evidence.with_suffix(".log").open("w") as log:
            result = subprocess.run([
                "systemd-run", "--user", "--wait", "--pipe", "--collect",
                "-p", "Delegate=yes", *environment, str(args.binary),
                "production_real_", "--ignored", "--test-threads=1",
            ], stdout=log, stderr=subprocess.STDOUT, timeout=120, check=False)
        args.evidence.with_suffix(".exit").write_text(f"{result.returncode}\n")
    finally:
        server.shutdown()
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())