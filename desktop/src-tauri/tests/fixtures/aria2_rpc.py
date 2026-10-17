#!/usr/bin/python3
"""Local RPC protocol fixture, not an aria2 compatibility implementation."""
import http.server
import json
import pathlib
import sys
import subprocess

options = dict(arg[2:].split("=", 1) for arg in sys.argv[1:] if arg.startswith("--") and "=" in arg)
secret = pathlib.Path(options["conf-path"]).read_text().strip().split("=", 1)[1]


class Handler(http.server.BaseHTTPRequestHandler):
    def do_POST(self):
        request = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        assert request["params"][0] == "token:" + secret
        method = request["method"]
        if method == "aria2.getVersion":
            result = {"version": "fixture"}
        elif method == "aria2.addUri":
            self.server.expired = request["params"][1][0].endswith("/expired")
            output = request["params"][2]
            self.server.payload = pathlib.Path(output["dir"]) / output["out"]
            self.server.payload.write_bytes(b"fixture-transfer")
            subprocess.Popen([sys.executable, "-c", "import time,pathlib,sys; time.sleep(2); pathlib.Path(sys.argv[1]).write_bytes(b'late')",
                              str(self.server.payload) + ".late"], start_new_session=True)
            result = "fixture-gid"
        elif method == "aria2.tellStatus":
            result = {"gid": "fixture-gid", "status": "complete", "completedLength": "16",
                      "totalLength": "16", "downloadSpeed": "0", "files": [
                          {"path": str(self.server.payload), "length": "16", "completedLength": "16"}]}
            if self.server.expired:
                result.update(status="error", errorCode="22", errorMessage="HTTP 403 Forbidden")
        elif method == "aria2.forceRemove":
            result = "fixture-gid"
        else:
            raise AssertionError(method)
        response = json.dumps({"jsonrpc": "2.0", "id": request["id"], "result": result}).encode()
        self.send_response(200)
        self.send_header("Content-Length", str(len(response)))
        self.end_headers()
        self.wfile.write(response)

    def log_message(self, *_args):
        pass


http.server.HTTPServer(("127.0.0.1", int(options["rpc-listen-port"])), Handler).serve_forever()