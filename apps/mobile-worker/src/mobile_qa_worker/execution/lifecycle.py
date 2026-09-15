"""Supervised direct replay: seal clean-start proof before accepting a mutation."""

import os
import signal
import subprocess
import time
from datetime import UTC, datetime
from pathlib import Path
from uuid import uuid4

from mobile_qa_worker.automation.direct import check
from mobile_qa_worker.automation.direct import execute as direct_execute
from mobile_qa_worker.execution.adapters import device_for
from mobile_qa_worker.execution.journal import read, write
from mobile_qa_worker.generated.models import ExecutionJob, LocalExecutionResult
from mobile_qa_worker.qualification.config import Profile, QualificationError
from mobile_qa_worker.qualification.device import doctor
from mobile_qa_worker.qualification.evidence import Evidence
from mobile_qa_worker.qualification.host import boot_id
from mobile_qa_worker.qualification.process import host_lock
from mobile_qa_worker.qualification.runner import cancellation


def await_start_ack(directory: Path, job: ExecutionJob, nonce: str, deadline: float) -> None:
    # Only the supervisor owns HTTP credentials. This private acknowledgement never contains them.
    while time.monotonic() < deadline:
        path = directory / "preflight-ack.json"
        if path.is_file() and not path.is_symlink():
            ack = read(path)
            if (
                ack.get("attempt_id") == str(job.attempt_id)
                and ack.get("instance_nonce") == nonce
                and ack.get("accepted") is True
            ):
                return
            raise QualificationError("start_acknowledgement_mismatch")
        time.sleep(0.2)
    raise QualificationError("start_acknowledgement_missing")


def execute(job: ExecutionJob, profile_path: Path, directory: Path) -> LocalExecutionResult:
    host = Profile.load(profile_path)
    assignment = job.manifest.profile
    context = assignment.execution_context
    case = job.manifest.cases[job.case_index].case
    if (
        context is None
        or case.adapter != assignment.adapter
        or case.package != assignment.package
        or any(a.kind.value == "navigate" for a in case.actions)
    ):
        raise QualificationError("unsupported_device_adapter")
    evidence = Evidence(directory / "evidence", case.budget.artifact_bytes)
    device = device_for(assignment, host, evidence)
    nonce = str(uuid4())
    outcome, reason, reset, stopped = (
        "inconclusive",
        "clean_start_not_verified",
        "quarantined",
        False,
    )
    events: list[dict[str, object]] = []
    with host_lock(host.state_root) as dirty:
        if dirty.exists():
            raise QualificationError("dirty_host_requires_recovery")
        doctor(host)
        write(
            dirty,
            {
                "attempt_id": str(job.attempt_id),
                "instance_nonce": nonce,
                "pid": os.getpid(),
                "boot_id": boot_id(),
            },
        )
        try:
            start = time.monotonic()
            started_at = datetime.now(UTC).isoformat()
            with cancellation(context.stages.boot_seconds):
                device.boot(timeout=context.stages.boot_seconds)
            with cancellation(context.stages.install_seconds):
                device.install(directory / "build.apk", job.manifest.build_sha256)
            with cancellation(context.stages.start_seconds):
                device.launch()
                for expected in context.starting_checks:
                    check(device, case.package, expected)
                device.capture_checkpoint("preflight")
            write(
                directory / "preflight.json",
                {
                    "attempt_id": str(job.attempt_id),
                    "instance_nonce": nonce,
                    "context": context.model_dump(mode="json"),
                    "build_sha256": job.manifest.build_sha256,
                    "started_at": started_at,
                    "ready_at": datetime.now(UTC).isoformat(),
                    "duration_ms": int((time.monotonic() - start) * 1000),
                    "artifact_ids": [],
                },
            )
            with cancellation(40):
                await_start_ack(directory, job, nonce, time.monotonic() + 35)
            with cancellation(case.budget.duration_seconds):
                for action in case.actions:
                    events.append(
                        {
                            "id": str(uuid4()),
                            "sequence": len(events) + 1,
                            "action_id": action.id,
                            "phase": "started",
                            "message": "Approved action started",
                        }
                    )
                    write(directory / "events.json", {"events": events})
                    if action.kind.value == "direct" and action.command:
                        direct_execute(device, case.package, action.command)
                    elif action.kind.value == "restart_app":
                        device.adb("shell", "am", "force-stop", case.package)
                        device.launch()
                    try:
                        for expected in case.checks:
                            if expected.checkpoint_id == action.checkpoint_id:
                                check(device, case.package, expected)
                    except QualificationError:
                        # Retain the contradictory/missing state for the server verifier.
                        pass
                    device.capture_checkpoint(action.checkpoint_id)
                    events.append(
                        {
                            "id": str(uuid4()),
                            "sequence": len(events) + 1,
                            "action_id": action.id,
                            "phase": "checkpoint",
                            "message": "Checkpoint captured",
                        }
                    )
                    write(directory / "events.json", {"events": events})
                outcome, reason = "passed", "actions_completed_verification_pending"
        except (QualificationError, OSError, subprocess.TimeoutExpired) as exc:
            outcome = "canceled" if str(exc) == "cancelled" else "inconclusive"
            reason = str(exc) if isinstance(exc, QualificationError) else "device_execution_error"
        finally:
            old = {s: signal.signal(s, signal.SIG_IGN) for s in (signal.SIGINT, signal.SIGTERM)}
            try:
                # Cleanup has its own deadline; the action timeout must not interrupt disposal.
                with cancellation(context.stages.cleanup_seconds):
                    device.stop()
                    device.discard()
                    reset, stopped = "verified_clean", True
                # Local disposal alone does not clear the dirty marker; wait for the API ack.
            except (QualificationError, OSError, subprocess.TimeoutExpired):
                reset, stopped = "quarantined", False
            finally:
                for sig, handler in old.items():
                    signal.signal(sig, handler)
    return LocalExecutionResult.model_validate(
        {
            "outcome": outcome,
            "reason": reason,
            "usage": [],
            "reset": reset,
            "stopped": stopped,
            "boot_id": boot_id(),
            "evidence_reference": nonce,
        }
    )
