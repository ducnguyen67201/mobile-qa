"""Loopback-only prerequisite fixture. No customer data, credentials or mutating endpoint."""

import argparse
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path


def server(mode: str, port: int = 8765) -> HTTPServer:
    if mode not in ("ready", "unavailable"):
        raise ValueError("invalid fixture mode")

    class Handler(BaseHTTPRequestHandler):
        def do_GET(self):
            code = (200 if mode == "ready" else 503) if self.path == "/session" else 404
            body = b'{"status":"ready"}' if code == 200 else b'{"status":"unavailable"}'
            self.send_response(code)
            self.send_header("Content-Type", "application/json")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)

        def log_message(self, *_args):
            pass

    return HTTPServer(("127.0.0.1", port), Handler)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=["ready", "unavailable"], required=True)
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--ready-file", type=Path)
    args = parser.parse_args()
    with server(args.mode, args.port) as http:
        # Publish only after our own bind succeeds; an existing server is not readiness.
        if args.ready_file is not None:
            with args.ready_file.open("x") as ready:
                ready.write("ready")
        http.serve_forever()
