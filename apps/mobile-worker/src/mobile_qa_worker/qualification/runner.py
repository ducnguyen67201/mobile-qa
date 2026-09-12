"""Explicit one-attempt supervisor. Dirty state survives crashes; no blind replay."""

import os
import signal
import subprocess
import sys
import time
import urllib.error
import urllib.request
from collections.abc import Iterator
from contextlib import contextmanager
from datetime import UTC, datetime
from pathlib import Path
from typing import cast

from mobile_qa_worker.generated.models import QualificationRequest, QualificationResult
from mobile_qa_worker.qualification.config import (
    Profile,
    QualificationError,
    json_object,
    parse_request,
)
from mobile_qa_worker.qualification.device import Device, doctor
from mobile_qa_worker.qualification.evidence import Evidence, atomic_json, sha256
from mobile_qa_worker.qualification.process import host_lock, stop_group
from mobile_qa_worker.qualification.verifier import verdict

ROOT = Path(__file__).resolve().parents[5]


def boot_id() -> str:
    path = Path("/proc/sys/kernel/random/boot_id")
    return path.read_text().strip() if path.exists() else "non-linux-test"


@contextmanager
def cancellation(seconds: int) -> Iterator[None]:
    def stop(_sig: int, _frame: object) -> None:
        raise QualificationError("attempt_timeout" if _sig == signal.SIGALRM else "cancelled")

    old = {s: signal.signal(s, stop) for s in (signal.SIGINT, signal.SIGTERM, signal.SIGALRM)}
    signal.setitimer(signal.ITIMER_REAL, seconds)
    try:
        yield
    finally:
        signal.setitimer(signal.ITIMER_REAL, 0)
        for sig, handler in old.items():
            signal.signal(sig, handler)


def run_sdk(
    request: QualificationRequest,
    profile: Profile,
    evidence: Evidence,
    interrupt_after: float | None = None,
) -> list[dict[str, object]]:
    child_dir = evidence.directory / "sdk"
    child_dir.mkdir(mode=0o700)
    request_path = evidence.directory / "request.json"
    atomic_json(request_path, request.model_dump(mode="json"))
    output = child_dir / "child-result.json"
    # Doppler is scoped at the repo. SDK cwd changes only after injection, in the child.
    args = [
        "doppler",
        "run",
        "--project",
        profile.doppler_project,
        "--config",
        profile.doppler_config,
        "--no-fallback",
        "--forward-signals",
        "--",
        sys.executable,
        "-m",
        "mobile_qa_worker.cli",
        "_sdk-run",
        "--request",
        str(request_path),
        "--result",
        str(output),
    ]
    log = (child_dir / "console.log").open("wb")
    started = time.monotonic()
    navigation_started: float | None = None
    try:
        child = subprocess.Popen(
            args,
            cwd=ROOT,
            stdin=subprocess.DEVNULL,
            stdout=log,
            stderr=subprocess.STDOUT,
            start_new_session=True,
        )
        try:
            while child.poll() is None:
                elapsed = time.monotonic() - started
                if navigation_started is None and (child_dir / "navigation.started").exists():
                    navigation_started = time.monotonic()
                if (
                    interrupt_after is not None
                    and navigation_started is not None
                    and time.monotonic() - navigation_started >= interrupt_after
                ):
                    raise QualificationError("cancelled")
                if elapsed > profile.init_seconds + profile.navigation_seconds + 10:
                    raise QualificationError("sdk_timeout")
                if evidence.size() > profile.evidence_bytes:
                    raise QualificationError("evidence_budget_exceeded")
                time.sleep(0.2)
            if child.returncode != 0:
                raise QualificationError("sdk_failed")
        finally:
            try:
                stop_group(child)
            except BaseException as exc:
                # Never reset/reuse a phone while an SDK descendant may still touch it.
                raise QualificationError("sdk_cleanup_unconfirmed") from exc
    finally:
        log.close()
    result = json_object(output)
    if result.get("status") != "completed" or not isinstance(result.get("usage"), dict):
        raise QualificationError("invalid_sdk_result")
    return [cast(dict[str, object], result["usage"])]


def fixture_ready() -> None:
    # Only the controlled local backend, never a caller-supplied URL.
    try:
        with urllib.request.urlopen("http://127.0.0.1:8765/session", timeout=2) as response:
            if response.status != 200:
                raise QualificationError("prerequisite_unavailable")
    except urllib.error.HTTPError as exc:
        if exc.code != 503:
            raise QualificationError("fixture_unavailable") from exc
    except OSError as exc:
        raise QualificationError("fixture_unavailable") from exc


def run_attempt(
    request: QualificationRequest, *, interrupt_after: float | None = None
) -> QualificationResult:
    request = parse_request(request.model_dump_json())
    profile = Profile.load(Path(request.profile_path))
    started_at = datetime.now(UTC)
    output = Path(request.output_root)
    output.mkdir(parents=True, mode=0o700, exist_ok=True)
    evidence = Evidence(output / str(request.attempt_id), profile.evidence_bytes)
    phases: dict[str, int] = {}
    inventory: dict[str, str] = {}
    observed_hash: str | None = None
    usage: list[dict[str, object]] = []
    outcome, reason, reset = "blocked", "preflight_not_started", "not_started"
    stage = "preflight"
    execution_stopped = True
    with host_lock(profile.state_root) as dirty:
        if dirty.exists():
            raise QualificationError("dirty_host_requires_recovery")
        device = Device(profile, evidence)
        dirty_written = False
        try:
            with cancellation(profile.attempt_seconds):
                inventory = doctor(profile)
                fixture_ready()
                atomic_json(
                    dirty,
                    {
                        "attempt_id": str(request.attempt_id),
                        "state": "dirty",
                        "pid": os.getpid(),
                        "boot_id": boot_id(),
                    },
                )
                dirty_written = True
                begin = time.monotonic()
                device.boot()
                phases["boot"] = int((time.monotonic() - begin) * 1000)
                begin = time.monotonic()
                observed_hash = device.install(Path(request.apk_path), request.expected_apk_sha256)
                phases["install"] = int((time.monotonic() - begin) * 1000)
                inventory.update(device.inventory)
                device.launch()
                task = "qa-" + str(request.attempt_id)
                initial = device.capture("initial", task)
                if initial.unavailable:
                    outcome, reason = "blocked", "prerequisite_unavailable"
                else:
                    device.start_recording()
                    stage = "execution"
                    begin = time.monotonic()
                    usage = run_sdk(request, profile, evidence, interrupt_after)
                    phases["navigation"] = int((time.monotonic() - begin) * 1000)
                    created = device.capture("created", task)
                    if not created.task_present:
                        outcome, reason = "inconclusive", "creation_not_proven"
                    else:
                        begin = time.monotonic()
                        device.adb("shell", "am", "force-stop", request.package)
                        device.launch()
                        reopened = device.capture("reopened", task)
                        phases["reopen_assertion"] = int((time.monotonic() - begin) * 1000)
                        outcome, reason = verdict(created, reopened)
        except (QualificationError, OSError) as exc:
            outcome = "inconclusive" if stage == "execution" else "blocked"
            reason = str(exc) if isinstance(exc, QualificationError) else "host_io_error"
            execution_stopped = reason != "sdk_cleanup_unconfirmed"
        finally:
            # Ignore additional termination requests only during bounded cleanup.
            old = {s: signal.signal(s, signal.SIG_IGN) for s in (signal.SIGINT, signal.SIGTERM)}
            begin = time.monotonic()

            def cleanup_timeout(_sig: int, _frame: object) -> None:
                raise QualificationError("cleanup_timeout")

            alarm = signal.signal(signal.SIGALRM, cleanup_timeout)
            signal.setitimer(signal.ITIMER_REAL, profile.cleanup_seconds)
            try:
                if dirty_written:
                    device.collect_diagnostics()
                    device.stop()
                    if not execution_stopped:
                        raise QualificationError("sdk_cleanup_unconfirmed")
                    device.discard()
                    # Prove fresh app state independently, then leave all owned devices stopped.
                    device.boot(timeout=profile.cleanup_seconds)
                    device.install(Path(request.apk_path), request.expected_apk_sha256)
                    device.launch()
                    device.adb(
                        "shell",
                        "run-as",
                        request.package,
                        "sh",
                        "-c",
                        "'test ! -e shared_prefs/tasks.xml'",
                    )
                    evidence.write(
                        "reset-storage.txt", b"Fresh demo preferences are absent.\n", "text/plain"
                    )
                    clean = device.capture("reset", "qa-" + str(request.attempt_id))
                    if clean.task_present:
                        raise QualificationError("reset_contaminated")
                    device.stop()
                    device.discard()
                    dirty.unlink()
                    reset = "verified_clean"
            except (QualificationError, OSError):
                reset = "quarantined"
                try:
                    device.stop()
                except (QualificationError, OSError):
                    pass
            finally:
                signal.setitimer(signal.ITIMER_REAL, 0)
                signal.signal(signal.SIGALRM, alarm)
                phases["cleanup"] = int((time.monotonic() - begin) * 1000)
                for sig, handler in old.items():
                    signal.signal(sig, handler)
    if not any(a.get("name") == "video.mp4" for a in evidence.artifacts):
        evidence.missing("video.mp4", "navigation_not_started")
    evidence.event("final", reason)
    for name in (
        "initial.png",
        "initial.xml",
        "created.png",
        "created.xml",
        "reopened.png",
        "reopened.xml",
    ):
        if not any(a.get("name") == name for a in evidence.artifacts):
            evidence.missing(name, reason)
    # SDK files remain private diagnostics; enumerate with hashes, not inferred verdicts.
    for path in sorted(evidence.directory.rglob("*")):
        if path.is_symlink():
            raise QualificationError("unsafe_artifact_path")
        if (
            path.is_file()
            and path.stat().st_size
            and path.name not in {a.get("path") for a in evidence.artifacts}
        ):
            name = str(path.relative_to(evidence.directory))
            evidence.artifacts.append(
                {
                    "status": "available",
                    "name": name,
                    "path": name,
                    "mime": "application/octet-stream",
                    "bytes": path.stat().st_size,
                    "sha256": sha256(path),
                }
            )
    return evidence.finalize(
        {
            "version": 1,
            "attempt_id": str(request.attempt_id),
            "case_id": request.case_id,
            "outcome": outcome,
            "reason_code": reason,
            "expected_behavior": "Created task persists after process restart.",
            "observed_behavior": reason,
            "started_at": started_at.isoformat(),
            "ended_at": datetime.now(UTC).isoformat(),
            "requested_build_sha256": request.expected_apk_sha256,
            "observed_build_sha256": observed_hash,
            "device_inventory": inventory,
            "model_profile_sha256": sha256(Path(request.profile_path)),
            "reset": reset,
            "phase_ms": phases,
            "artifacts": evidence.artifacts,
            "usage": usage,
        }
    )


def recover(profile: Profile) -> None:
    """Recovery requires host service stop first; never kill a PID from stale JSON."""
    with host_lock(profile.state_root) as dirty:
        if not dirty.exists():
            return
        if json_object(dirty).get("boot_id") == boot_id():
            raise QualificationError("reboot_required_for_crash_recovery")
        # Refuse recovery while any target port is occupied. Operator/systemd owns
        # killing stale cgroups; this command never guesses process identity from a PID.
        recovery_evidence = Evidence(profile.state_root / ("recovery-" + str(time.time_ns())))
        dummy = Device(profile, recovery_evidence)
        dummy.stop()
        dummy.discard()
        # Retain quarantine until a fresh attempt proves full boot/install/reset.
        dirty.rename(profile.state_root / ("recovered-" + str(time.time_ns()) + ".json"))
