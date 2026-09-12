"""Sequential approved actions on the qualified demo adapter, with supervised SDK children."""

import json
import os
import signal
import subprocess
import sys
from pathlib import Path
from typing import cast
from uuid import uuid4

from mobile_qa_worker.execution.journal import write
from mobile_qa_worker.generated.models import (
    ExecutionJob,
    LocalExecutionResult,
    ModelUsage,
    NavigationRequest,
)
from mobile_qa_worker.qualification.campaign import backend
from mobile_qa_worker.qualification.config import Profile, QualificationError
from mobile_qa_worker.qualification.device import Device, doctor
from mobile_qa_worker.qualification.evidence import Evidence, atomic_json
from mobile_qa_worker.qualification.host import boot_id
from mobile_qa_worker.qualification.process import host_lock, stop_group
from mobile_qa_worker.qualification.runner import cancellation


def sdk_action(
    job: ExecutionJob,
    profile: Profile,
    profile_path: Path,
    directory: Path,
    instruction: str,
    step_budget: int,
    usage: list[ModelUsage],
) -> None:
    directory.mkdir(mode=0o700)
    request = NavigationRequest.model_validate(
        {
            "attempt_id": str(job.attempt_id),
            "profile_path": str(profile_path),
            "serial": "emulator-5554",
            "package": job.manifest.profile.package,
            "instruction": instruction,
            "max_steps": step_budget,
        }
    )
    request_path = directory / "request.json"
    atomic_json(request_path, request.model_dump(mode="json"))
    output = directory / "result.json"
    # No API/lease tokens are written or passed to the model child.
    env = {k: v for k, v in os.environ.items() if k in ("PATH", "HOME", "LANG", "VIRTUAL_ENV")}
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
        "_execution-sdk",
        "--request",
        str(request_path),
        "--result",
        str(output),
    ]
    with (directory / "console.log").open("wb") as log:
        child = subprocess.Popen(
            args,
            stdin=subprocess.DEVNULL,
            stdout=log,
            stderr=subprocess.STDOUT,
            start_new_session=True,
            env=env,
        )
        try:
            child.wait(timeout=profile.init_seconds + profile.navigation_seconds + 10)
            if child.returncode:
                raise QualificationError("sdk_failed")
        finally:
            try:
                stop_group(child)
            except (QualificationError, OSError, subprocess.TimeoutExpired) as exc:
                raise QualificationError("sdk_cleanup_unconfirmed") from exc
            finally:
                # Preserve usage even when the SDK exits nonzero; absence stays unknown.
                if output.is_file() and output.stat().st_size <= 1048576:
                    value = json.loads(output.read_text())
                    if isinstance(value, dict):
                        metadata = cast(dict[str, object], value)
                        if isinstance(metadata.get("usage"), dict):
                            usage.append(ModelUsage.model_validate(metadata["usage"]))


def execute(
    job: ExecutionJob, profile_path: Path, directory: Path, scenario: str
) -> LocalExecutionResult:
    profile = Profile.load(profile_path)
    definition = job.manifest.cases[job.case_index].case
    if (
        job.manifest.profile.driver.value != "minitap"
        or definition.adapter != "demo_persistence_v1"
    ):
        raise ValueError("unsupported_device_adapter")
    if (
        profile.model != job.manifest.profile.model
        or profile.system_image != job.manifest.profile.image
    ):
        raise ValueError("host_profile_does_not_match_manifest")
    evidence = Evidence(directory / "evidence", definition.budget.artifact_bytes)
    device = Device(profile, evidence)
    outcome, reason, reset, stopped = (
        "passed",
        "actions_completed_verification_pending",
        "quarantined",
        False,
    )
    usage: list[ModelUsage] = []
    events: list[dict[str, object]] = []
    task = "qa-" + str(job.attempt_id)
    nav_count = max(1, sum(a.kind.value == "navigate" for a in definition.actions))
    step_budget = max(1, min(profile.max_steps, definition.budget.max_steps // nav_count))
    with host_lock(profile.state_root) as dirty:
        if dirty.exists():
            raise QualificationError("dirty_host_requires_recovery")
        doctor(profile)
        atomic_json(
            dirty, {"attempt_id": str(job.attempt_id), "pid": os.getpid(), "boot_id": boot_id()}
        )
        try:
            with (
                cancellation(definition.budget.duration_seconds),
                backend("unavailable" if scenario == "blocked" else "ready"),
            ):
                device.boot()
                device.install(directory / "build.apk", job.manifest.build_sha256)
                device.launch()
                if device.capture("preflight", task).unavailable:
                    raise QualificationError("prerequisite_unavailable")
                for action in definition.actions:
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
                    if action.kind.value == "navigate":
                        sdk_action(
                            job,
                            profile,
                            profile_path,
                            directory / ("sdk-" + action.id),
                            action.instruction.replace("${task_title}", task),
                            step_budget,
                            usage,
                        )
                    elif action.kind.value == "restart_app":
                        device.adb("shell", "am", "force-stop", definition.package)
                        device.launch()
                    observation_seconds = min(
                        (
                            c.observation_seconds
                            for c in definition.checks
                            if c.checkpoint_id == action.checkpoint_id
                        ),
                        default=profile.assertion_seconds,
                    )
                    observation = device.capture(
                        action.checkpoint_id, task, timeout=observation_seconds
                    )
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
                    if observation.unavailable:
                        outcome, reason = "blocked", "prerequisite_unavailable"
                        break
        except (QualificationError, OSError, subprocess.TimeoutExpired) as exc:
            outcome = (
                "canceled"
                if str(exc) == "cancelled"
                else "blocked"
                if str(exc) == "prerequisite_unavailable"
                else "inconclusive"
            )
            reason = str(exc) if isinstance(exc, QualificationError) else "device_execution_error"
        finally:
            # Additional signals cannot interrupt bounded cleanup and falsely release the phone.
            old = {s: signal.signal(s, signal.SIG_IGN) for s in (signal.SIGINT, signal.SIGTERM)}
            try:
                with cancellation(profile.cleanup_seconds):
                    if reason in ("sdk_cleanup_unconfirmed", "process_group_not_stopped"):
                        raise QualificationError("execution_stop_unconfirmed")
                    device.stop()
                    device.discard()
                    with backend("ready"):
                        device.boot(timeout=profile.cleanup_seconds)
                        device.install(directory / "build.apk", job.manifest.build_sha256)
                        device.launch()
                        device.adb(
                            "shell",
                            "run-as",
                            definition.package,
                            "sh",
                            "-c",
                            "'test ! -e shared_prefs/tasks.xml'",
                        )
                        observation = device.capture("reset", task)
                        if observation.task_present:
                            raise QualificationError("reset_contaminated")
                        device.stop()
                        device.discard()
                    reset, stopped = "verified_clean", True
                    dirty.unlink()
            except (QualificationError, OSError):
                reset = "quarantined"
                try:
                    device.stop()
                except (QualificationError, OSError):
                    pass
            finally:
                for sig, handler in old.items():
                    signal.signal(sig, handler)
    return LocalExecutionResult.model_validate(
        {
            "outcome": outcome,
            "reason": reason,
            "usage": usage,
            "reset": reset,
            "stopped": stopped,
            "boot_id": boot_id(),
            "evidence_reference": str(directory),
        }
    )
