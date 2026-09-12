"""Explicit local process ownership; no watchers and no volume deletion."""
import argparse
import json
import os
import signal
import socket
import subprocess
import time
import urllib.error
import urllib.request
from contextlib import contextmanager
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
COMPOSE = ["docker", "compose", "-f", str(ROOT / "infra/compose.yaml")]


def run(command, **kwargs):
    return subprocess.run(command, cwd=ROOT, check=True, **kwargs)


@contextmanager
def database():
    running = run(COMPOSE + ["ps", "--status", "running", "-q", "postgres"], capture_output=True, text=True).stdout.strip()
    owned = not running
    try:
        if owned:
            run(COMPOSE + ["up", "-d", "--wait", "postgres"])
        yield
    finally:
        if owned:
            run(COMPOSE + ["stop", "postgres"])


def ensure_ports():
    for port in (5150, 5173):
        with socket.socket() as sock:
            try:
                sock.bind(("127.0.0.1", port))
            except OSError as exc:
                raise RuntimeError(f"Port {port} is occupied; stop its owner or use that running service.") from exc


def wait_http(url, children, timeout=60):
    start = time.monotonic()
    while time.monotonic() - start < timeout:
        if any(child.poll() is not None for child in children):
            raise RuntimeError("A development service exited. Inspect .private logs.")
        try:
            with urllib.request.urlopen(url, timeout=2) as response:
                if response.status == 200:
                    return time.monotonic() - start
        except (urllib.error.URLError, TimeoutError, OSError):
            pass
        time.sleep(0.2)
    raise RuntimeError(f"Service did not become ready: {url}")


@contextmanager
def services(built=False):
    ensure_ports()
    private = ROOT / ".private"
    private.mkdir(mode=0o700, exist_ok=True)
    private.chmod(0o700)
    commands = [([str(ROOT / "target/debug/mobile-qa-cli"), "start", "--environment", "development"] if built else ["cargo", "run", "--locked", "--bin", "mobile-qa-cli", "--", "start", "--environment", "development"]), ["pnpm", "--dir", "apps/web", "dev"]]
    children = []
    handles = []
    try:
        for name, command in zip(("api", "web"), commands, strict=True):
            handle = (private / f"{name}.log").open("w")
            handles.append(handle)
            children.append(subprocess.Popen(command, cwd=(ROOT / "apps/api" if name == "api" else ROOT), stdout=handle, stderr=subprocess.STDOUT, start_new_session=True))
        yield children
    finally:
        for child in reversed(children):
            if child.poll() is None:
                os.killpg(child.pid, signal.SIGTERM)
                try:
                    child.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    os.killpg(child.pid, signal.SIGKILL)
                    child.wait()
        for handle in handles:
            handle.close()


def smoke():
    # Hide dist while starting development to prove no static build dependency.
    dist, hidden = ROOT / "apps/web/dist", ROOT / ".private/dist-smoke"
    hidden.parent.mkdir(mode=0o700, exist_ok=True)
    if hidden.exists():
        raise RuntimeError("An earlier hidden dist exists; restore it before smoke.")
    moved = dist.exists()
    if moved:
        dist.rename(hidden)
    try:
        with database(), services(built=True) as children:
            started = wait_http("http://127.0.0.1:5150/api/health", children)
            wait_http("http://127.0.0.1:5173/api/health", children)
            for port in (5150, 5173):
                with urllib.request.urlopen(f"http://127.0.0.1:{port}/api/health") as response:
                    assert json.load(response) == {"status": "ok", "service": "mobile-qa", "version": "0.1.0"}
            try:
                urllib.request.urlopen("http://127.0.0.1:5150/api/missing")
            except urllib.error.HTTPError as error:
                assert error.code == 404
            else:
                raise AssertionError("Unknown API route must return 404")
            output = run(COMPOSE + ["exec", "-T", "postgres", "psql", "-U", "mobile_qa", "-d", "mobile_qa_development", "-Atc", "SELECT current_database(), to_regclass('seaql_migrations');"], capture_output=True, text=True).stdout.strip()
            assert output == "mobile_qa_development|seaql_migrations", output
            for kind in ("pass", "fail", "blocked"):
                run(["uv", "run", "--no-sync", "--project", "apps/mobile-worker", "--frozen", "mobile-qa-worker", "fake", f"contracts/fixtures/{kind}.json"])
            # Block network socket connections during the import-only SDK smoke.
            run(["uv", "run", "--no-sync", "--project", "apps/mobile-worker", "--frozen", "--extra", "sdk", "python", "scripts/sdk_smoke.py"])
            print(f"Smoke passed; warm built API ready in {started:.3f}s")
    finally:
        if moved:
            hidden.rename(dist)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("mode", choices=["dev", "smoke", "check-api"])
    parser.add_argument("--api-only", action="store_true", help="Check API/migration packages, not contract tests (CI scope)")
    args = parser.parse_args()
    if args.api_only and args.mode != "check-api":
        parser.error("--api-only requires check-api")
    if args.mode == "check-api":
        packages = ["--package", "mobile-qa", "--package", "migration"] if args.api_only else ["--workspace"]
        format_packages = packages if args.api_only else ["--all"]
        with database():
            run(["cargo", "fmt", *format_packages, "--", "--check"])
            run(["cargo", "clippy", *packages, "--all-targets", "--locked", "--", "-D", "warnings"])
            run(["cargo", "test", *packages, "--locked"])
    elif args.mode == "smoke":
        smoke()
    else:
        with database(), services() as children:
            start = wait_http("http://127.0.0.1:5150/api/health", children, timeout=900)
            wait_http("http://127.0.0.1:5173", children)
            print(f"Ready at http://127.0.0.1:5173 (API startup {start:.3f}s). Ctrl-C stops owned services.", flush=True)
            while all(child.poll() is None for child in children):
                time.sleep(1)
            raise RuntimeError("A development service exited. Inspect .private logs.")


if __name__ == "__main__":
    signal.signal(signal.SIGTERM, lambda *_: (_ for _ in ()).throw(KeyboardInterrupt()))
    try:
        main()
    except KeyboardInterrupt:
        pass
