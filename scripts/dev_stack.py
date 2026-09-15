"""Explicit full local stack, with isolated Doppler children and private per-run logs."""

import argparse
import fcntl
import json
import os
import signal
import subprocess
import time
import tomllib
from pathlib import Path
from uuid import UUID

from runtime import ROOT, database, ensure_ports, wait_http

PRIVATE = ROOT / ".private/dev"
CONFIG = PRIVATE / "config.json"
STATE = PRIVATE / "supervisor.json"


def settings(path):
    data = json.loads(path.read_text())
    required = {"profile", "profile_id", "java_home", "api_project", "api_config"}
    if set(data) != required:
        raise ValueError(f"Dev config must contain only: {', '.join(sorted(required))}")
    UUID(data["profile_id"])
    profile = Path(data["profile"]).expanduser()
    profile = (
        (ROOT / profile).resolve() if not profile.is_absolute() else profile.resolve()
    )
    host = tomllib.loads(profile.read_text())
    java = Path(data["java_home"]).expanduser().resolve()
    if not (java / "bin/java").is_file():
        raise ValueError("Dev config java_home must point to an installed JDK 17")
    return data | {"profile": str(profile), "java_home": str(java)}, host


def identity(pid):
    result = subprocess.run(
        ["ps", "-p", str(pid), "-o", "lstart="],
        capture_output=True,
        text=True,
        check=False,
    )
    return result.stdout.strip() if result.returncode == 0 else None


def stop():
    if not STATE.exists():
        print("No managed dev stack is running.", flush=True)
        return
    state = json.loads(STATE.read_text())
    if identity(state["pid"]) != state["started"]:
        raise RuntimeError(
            "Stale supervisor record; refusing to signal a different process."
        )
    os.kill(state["pid"], signal.SIGTERM)
    deadline = time.monotonic() + 300
    while STATE.exists() and time.monotonic() < deadline:
        time.sleep(0.25)
    if STATE.exists():
        raise RuntimeError(
            "Dev cleanup is still pending. Inspect the current logs before restarting."
        )
    print("Dev stack stopped. Database contents preserved.", flush=True)


def doppler(project, config, command):
    return [
        "doppler",
        "run",
        "--project",
        project,
        "--config",
        config,
        "--no-fallback",
        "--forward-signals",
        "--",
        *command,
    ]


def clean_env():
    # Do not inherit a parent Doppler export into Vite, build tools or the supervisor's children.
    allowed = {
        "PATH",
        "HOME",
        "USER",
        "LOGNAME",
        "LANG",
        "LC_ALL",
        "TMPDIR",
        "TERM",
        "DOCKER_HOST",
        "DOCKER_CONTEXT",
        "DOCKER_CONFIG",
        "SSH_AUTH_SOCK",
    }
    return {k: v for k, v in os.environ.items() if k in allowed}


def wait_worker(name, child, log, children, stopping):
    deadline = time.monotonic() + 50
    while time.monotonic() < deadline and not stopping():
        if any(p.poll() is not None for p in children):
            raise RuntimeError(f"{name} startup failed; inspect {log}")
        if "Worker connected" in log.read_text(errors="replace"):
            return
        time.sleep(0.2)
    if not stopping():
        raise RuntimeError(f"{name} did not authenticate with the API; inspect {log}")


def serve(config):
    PRIVATE.mkdir(parents=True, mode=0o700, exist_ok=True)
    PRIVATE.chmod(0o700)
    with (PRIVATE / "supervisor.lock").open("a") as lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError:
            raise RuntimeError(
                "Dev is already running. Use just dev-restart."
            ) from None
        data, host = settings(config)
        ensure_ports()
        logs = PRIVATE / time.strftime("%Y%m%d-%H%M%S")
        logs.mkdir(mode=0o700)
        latest = PRIVATE / "latest"
        if latest.is_symlink():
            latest.unlink()
        latest.symlink_to(logs.name, target_is_directory=True)
        env = clean_env()
        env["JAVA_HOME"] = data["java_home"]
        env["PATH"] = (
            str(Path(data["java_home"]) / "bin") + os.pathsep + env.get("PATH", "")
        )
        stopping = False

        def shutdown(*_):
            nonlocal stopping
            stopping = True

        signal.signal(signal.SIGTERM, shutdown)
        signal.signal(signal.SIGINT, shutdown)
        STATE.write_text(
            json.dumps(
                {
                    "pid": os.getpid(),
                    "started": identity(os.getpid()),
                    "logs": str(logs),
                }
            )
        )
        STATE.chmod(0o600)
        children, handles = [], []

        def launch(name, command, cwd=ROOT, child_env=None):
            path = logs / f"{name}.log"
            handle = path.open("w")
            path.chmod(0o600)
            handles.append(handle)
            process = subprocess.Popen(
                command,
                cwd=cwd,
                env=child_env or env,
                stdout=handle,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            children.append(process)
            print(f"Started {name}; log: {path}", flush=True)
            return process, path

        try:
            # One build per explicit startup; no compiler/watch loop and no build credentials.
            build, _ = launch(
                "build", ["cargo", "build", "--locked", "--bin", "mobile-qa-cli"]
            )
            while build.poll() is None and not stopping:
                time.sleep(0.2)
            if stopping:
                return
            if build.returncode:
                raise RuntimeError(f"API build failed; inspect {logs / 'build.log'}")
            children.remove(build)
            with database():
                try:
                    api_env = env | {
                        "MOBILE_QA_DEV_ORIGIN": "http://localhost:5173",
                        "MOBILE_QA_ANDROID_SDK": host["sdk_root"],
                    }
                    launch(
                        "api",
                        doppler(
                            data["api_project"],
                            data["api_config"],
                            [
                                str(ROOT / "target/debug/mobile-qa-cli"),
                                "start",
                                "--environment",
                                "development",
                            ],
                        ),
                        ROOT / "apps/api",
                        api_env,
                    )
                    launch("web", ["pnpm", "--dir", "apps/web", "dev"])
                    wait_http("http://127.0.0.1:5150/api/health", children)
                    wait_http("http://localhost:5173", children)
                    if stopping:
                        return
                    for name, args in (
                        ("phone-worker", ["task-worker"]),
                        (
                            "execution-worker",
                            ["execution-worker", "--profile-id", data["profile_id"]],
                        ),
                    ):
                        command = [
                            "uv",
                            "run",
                            "--no-sync",
                            "--project",
                            "apps/mobile-worker",
                            "--frozen",
                            "mobile-qa-worker",
                            *args,
                            "--origin",
                            "http://127.0.0.1:5150",
                            "--state",
                            str(PRIVATE / name),
                            "--profile",
                            data["profile"],
                        ]
                        process, log = launch(
                            name,
                            doppler(
                                host["doppler_project"], host["doppler_config"], command
                            ),
                        )
                        wait_worker(name, process, log, children, lambda: stopping)
                        if stopping:
                            return
                    print(
                        "Ready: http://localhost:5173 — API, web, phone and execution workers connected.\n"
                        "The emulator opens when you connect a phone in the UI. AI runs only when requested.\n"
                        "Logs: just dev-logs | Stop: Ctrl-C or just dev-stop",
                        flush=True,
                    )
                    while not stopping:
                        if any(child.poll() is not None for child in children):
                            raise RuntimeError(
                                f"A service exited. Inspect {logs}; automatic replay is disabled."
                            )
                        time.sleep(0.5)
                finally:
                    # Workers clean their device leases while the API/DB are still available.
                    cleanup(children)
        finally:
            cleanup(children)
            for handle in handles:
                handle.close()
            STATE.unlink(missing_ok=True)


def cleanup(children):
    while children:
        child = children.pop()
        try:
            os.killpg(child.pid, signal.SIGTERM)
        except ProcessLookupError:
            continue
        try:
            child.wait(timeout=120)
        except subprocess.TimeoutExpired:
            print(
                "Cleanup timed out; preserving worker/device journals for recovery.",
                flush=True,
            )
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            child.wait()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "mode", choices=["start", "stop", "restart", "logs"], nargs="?", default="start"
    )
    parser.add_argument("--config", type=Path, default=CONFIG)
    args = parser.parse_args()
    if args.mode in ("stop", "restart"):
        stop()
    if args.mode == "logs":
        paths = sorted((PRIVATE / "latest").glob("*.log"))
        if not paths:
            raise RuntimeError("No dev logs yet. Start just dev first.")
        os.execvp("tail", ["tail", "-n", "40", "-F", *map(str, paths)])
    if args.mode in ("start", "restart"):
        serve(args.config)


if __name__ == "__main__":
    try:
        main()
    except (OSError, ValueError, RuntimeError) as exc:
        raise SystemExit(str(exc)) from None
