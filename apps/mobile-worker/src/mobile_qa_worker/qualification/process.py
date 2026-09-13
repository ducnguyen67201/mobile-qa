"""Bounded owned process groups and a persistent dirty marker for crash recovery."""

import fcntl
import os
import selectors
import signal
import subprocess
import time
from collections.abc import Iterator
from contextlib import contextmanager
from pathlib import Path

from mobile_qa_worker.qualification.config import QualificationError


def group_running(group: int) -> bool:
    """Ignore reaped/orphan zombies; inspect process status without signal-zero probes."""
    proc = Path("/proc")
    if proc.exists():
        for entry in proc.iterdir():
            if entry.name.isdigit():
                try:
                    fields = (entry / "stat").read_text().rsplit(")", 1)[1].split()
                    if int(fields[2]) == group and fields[0] != "Z":
                        return True
                except (OSError, ValueError, IndexError):
                    continue
        return False
    snapshot = subprocess.run(
        ["ps", "-axo", "pgid=,stat="], capture_output=True, text=True, check=True, timeout=3
    ).stdout
    return any(
        len(parts := line.split()) == 2 and parts[0] == str(group) and not parts[1].startswith("Z")
        for line in snapshot.splitlines()
    )


def stop_group(child: subprocess.Popen[bytes], grace: float = 3) -> None:
    # The group can still contain a grandchild after the immediate child exits.
    group = child.pid
    for sig in (signal.SIGTERM, signal.SIGKILL):
        if not group_running(group):
            child.wait(timeout=grace)
            return
        try:
            os.killpg(group, sig)
        except ProcessLookupError:
            child.wait(timeout=grace)
            return
        except PermissionError as exc:
            raise QualificationError("process_group_not_stopped") from exc
        end = time.monotonic() + grace
        while time.monotonic() < end:
            child.poll()
            if not group_running(group):
                child.wait(timeout=grace)
                return
            time.sleep(0.05)
    if group_running(group):
        raise QualificationError("process_group_not_stopped")
    child.wait(timeout=grace)


@contextmanager
def host_lock(root: Path) -> Iterator[Path]:
    if root != root.resolve():
        raise QualificationError("unsafe_state_root")
    root.mkdir(parents=True, mode=0o700, exist_ok=True)
    root.chmod(0o700)
    descriptor = os.open(root / "device.lock", os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "w") as handle:
        try:
            fcntl.flock(handle, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except BlockingIOError as exc:
            raise QualificationError("device_busy") from exc
        yield root / "dirty.json"


def command(args: list[str], timeout: float = 20, env: dict[str, str] | None = None) -> bytes:
    """Cap output while reading, so a noisy command cannot exhaust supervisor memory."""
    with subprocess.Popen(
        args,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        stdin=subprocess.DEVNULL,
        start_new_session=True,
        env=env,
    ) as child:
        try:
            assert child.stdout is not None
            deadline = time.monotonic() + timeout
            output = bytearray()
            with selectors.DefaultSelector() as selector:
                selector.register(child.stdout, selectors.EVENT_READ)
                while selector.get_map():
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        raise subprocess.TimeoutExpired(args, timeout)
                    for key, _ in selector.select(remaining):
                        chunk = os.read(key.fd, 65536)
                        if not chunk:
                            selector.unregister(key.fd)
                            break
                        output.extend(chunk)
                        if len(output) > 16777216:
                            raise QualificationError("command_output_too_large")
            child.wait(timeout=max(0.001, deadline - time.monotonic()))
        except subprocess.TimeoutExpired as exc:
            stop_group(child)
            raise QualificationError("command_timeout") from exc
        except BaseException:
            stop_group(child)
            raise
        if child.returncode:
            raise QualificationError("command_failed")
        return bytes(output)
