"""Worker HTTP supervision. A durable dirty journal forbids replay after a process crash."""

import os
import signal
import subprocess
import sys
import time
from pathlib import Path
from uuid import UUID, uuid4

from mobile_qa_worker.execution.client import Client, TransportError
from mobile_qa_worker.execution.journal import read, write
from mobile_qa_worker.generated.models import (
    ArtifactReceipt,
    AttemptReceipt,
    ClaimResponse,
    EventReceipt,
    EventRequest,
    ExecutionJob,
    ExecutionLease,
    LeaseStatusResponse,
    LocalExecutionResult,
)
from mobile_qa_worker.qualification.evidence import sha256
from mobile_qa_worker.qualification.process import host_lock, stop_group


def dispatch_child(
    job_path: Path, directory: Path, driver: str, scenario: str, profile_path: Path | None
) -> None:
    job = ExecutionJob.model_validate_json(job_path.read_bytes(), strict=True)
    if driver == "fake":
        from mobile_qa_worker.execution.fake import execute

        evidence = directory / "evidence"
        evidence.mkdir(mode=0o700)
        execute(job, evidence, scenario)
        write(
            directory / "events.json",
            {
                "events": [
                    {
                        "id": str(uuid4()),
                        "sequence": i + 1,
                        "action_id": a.id,
                        "phase": "checkpoint",
                        "message": "Synthetic checkpoint captured",
                    }
                    for i, a in enumerate(job.manifest.cases[job.case_index].case.actions)
                ]
            },
        )
        result = LocalExecutionResult.model_validate(
            {
                "outcome": "blocked" if scenario == "blocked" else "passed",
                "reason": "synthetic_execution",
                "usage": [],
                "reset": "verified_clean",
                "stopped": True,
                "boot_id": "synthetic",
                "evidence_reference": str(evidence),
            }
        )
    else:
        from mobile_qa_worker.execution.actions import execute

        if profile_path is None:
            raise ValueError("real_worker_requires_host_profile")
        result = execute(job, profile_path, directory, scenario)
    write(directory / "result.json", result.model_dump(mode="json"))


def transfer(client: Client, lease: ExecutionLease, directory: Path) -> None:
    prefix = f"/api/worker/attempts/{lease.attempt_id}"
    case = lease.manifest.cases[lease.case_index].case
    for action in case.actions:
        for suffix, mime in [(".xml", "application/xml"), (".png", "image/png")]:
            path = directory / "evidence" / (action.checkpoint_id + suffix)
            if not path.is_file() or path.is_symlink():
                continue
            client.send(
                prefix + "/heartbeat",
                {"generation": lease.generation},
                LeaseStatusResponse,
                lease.lease_token,
            )
            receipt = client.send(
                prefix + "/artifacts",
                {
                    "generation": lease.generation,
                    "checkpoint_id": action.checkpoint_id,
                    "name": path.name,
                    "mime": mime,
                    "byte_size": path.stat().st_size,
                    "sha256": sha256(path),
                },
                ArtifactReceipt,
                lease.lease_token,
            )
            data = client.raw(
                "PUT",
                prefix + f"/artifacts/{receipt.artifact.id}/content",
                path.read_bytes(),
                lease.lease_token,
                lease.generation,
                mime,
            )
            ArtifactReceipt.model_validate_json(data, strict=True)


def run_lease(
    client: Client, lease: ExecutionLease, state: Path, profile: Path | None, scenario: str
) -> None:
    directory = state / str(lease.attempt_id)
    directory.mkdir(mode=0o700)
    job = ExecutionJob.model_validate(
        {"attempt_id": lease.attempt_id, "manifest": lease.manifest, "case_index": lease.case_index}
    )
    write(directory / "job.json", job.model_dump(mode="json"))
    prefix = f"/api/worker/attempts/{lease.attempt_id}"
    build = client.raw(
        "GET",
        prefix + f"/build?generation={lease.generation}",
        lease_token=lease.lease_token,
        limit=lease.manifest.build_bytes,
    )
    apk = directory / "build.apk"
    apk.write_bytes(build)
    apk.chmod(0o600)
    if len(build) != lease.manifest.build_bytes or sha256(apk) != lease.manifest.build_sha256:
        raise ValueError("build_checksum_mismatch")
    args = [
        sys.executable,
        "-m",
        "mobile_qa_worker.cli",
        "_execution-run",
        "--job",
        str(directory / "job.json"),
        "--directory",
        str(directory),
        "--driver",
        lease.manifest.profile.driver.value,
        "--scenario",
        scenario,
    ]
    if profile:
        args += ["--profile", str(profile)]
    env = {k: v for k, v in os.environ.items() if k in ("PATH", "HOME", "LANG", "VIRTUAL_ENV")}
    with (directory / "supervisor.log").open("wb") as log:
        child = subprocess.Popen(
            args,
            stdin=subprocess.DEVNULL,
            stdout=log,
            stderr=subprocess.STDOUT,
            start_new_session=True,
            env=env,
        )
        last_ack = time.monotonic()
        next_heartbeat = last_ack
        stop_at: float | None = None
        try:
            while child.poll() is None:
                now = time.monotonic()
                if now >= next_heartbeat:
                    try:
                        heartbeat = client.send(
                            prefix + "/heartbeat",
                            {"generation": lease.generation},
                            LeaseStatusResponse,
                            lease.lease_token,
                        )
                        last_ack = time.monotonic()
                        event_path = directory / "events.json"
                        if event_path.is_file():
                            pending = read(event_path)
                            pending["generation"] = lease.generation
                            client.send(
                                prefix + "/events",
                                EventRequest.model_validate(pending),
                                EventReceipt,
                                lease.lease_token,
                            )
                        if heartbeat.cancel_requested and stop_at is None:
                            child.send_signal(signal.SIGTERM)
                            stop_at = now
                    except TransportError as exc:
                        if exc.status in (401, 404, 409) and stop_at is None:
                            child.send_signal(signal.SIGTERM)
                            stop_at = now
                    next_heartbeat = now + 10
                if now - last_ack >= 40 and stop_at is None:
                    child.send_signal(signal.SIGTERM)
                    stop_at = now
                if stop_at is not None and now - stop_at > 610:
                    raise ValueError("cleanup_not_acknowledged")
                time.sleep(0.2)
        finally:
            stop_group(child)
    path = directory / "result.json"
    if child.returncode or not path.is_file():
        result = LocalExecutionResult.model_validate(
            {
                "outcome": "inconclusive",
                "reason": "worker_child_failed_requires_recovery",
                "usage": [],
                "reset": "quarantined",
                "stopped": False,
                "boot_id": "unconfirmed",
                "evidence_reference": str(directory),
            }
        )
    else:
        result = LocalExecutionResult.model_validate_json(path.read_bytes(), strict=True)
    # Keep a bounded transport retry window; API loss never re-executes the case.
    deadline = time.monotonic() + 35
    while True:
        try:
            client.send(
                prefix + "/heartbeat",
                {"generation": lease.generation},
                LeaseStatusResponse,
                lease.lease_token,
            )
            events_path = directory / "events.json"
            if events_path.is_file():
                pending = read(events_path)
                pending["generation"] = lease.generation
                events = EventRequest.model_validate(pending)
                client.send(prefix + "/events", events, EventReceipt, lease.lease_token)
            transfer(client, lease, directory)
            client.send(
                prefix + "/complete",
                {
                    "generation": lease.generation,
                    "execution_outcome": result.outcome.value,
                    "reason": result.reason,
                    "usage": [u.model_dump(mode="json") for u in result.usage],
                },
                AttemptReceipt,
                lease.lease_token,
            )
            receipt = client.send(
                prefix + "/cleanup",
                {
                    "generation": lease.generation,
                    "stopped": result.stopped,
                    "reset": result.reset.value,
                    "evidence_reference": result.evidence_reference,
                    "boot_id": result.boot_id,
                },
                AttemptReceipt,
                lease.lease_token,
            )
            if receipt.attempt.cleanup.value != "verified_clean":
                raise ValueError("resource_quarantined")
            return
        except TransportError as exc:
            if exc.status not in (0, 408, 429, 500, 503) or time.monotonic() >= deadline:
                raise
            time.sleep(1)


def serve(
    origin: str,
    profile_id: UUID,
    state: Path,
    profile: Path | None,
    scenario: str = "pass",
    once: bool = False,
) -> None:
    state = state.resolve()
    state.mkdir(mode=0o700, parents=True, exist_ok=True)
    client = Client(origin, os.environ.get("MOBILE_QA_WORKER_TOKEN", ""))
    journal = state / "execution.json"
    with host_lock(state):
        if journal.exists() and read(journal).get("state") == "active":
            raise ValueError("active_execution_requires_operator_recovery")
        connected = False
        while True:
            claim_id = str(read(journal)["claim_id"]) if journal.exists() else str(uuid4())
            write(journal, {"state": "claiming", "claim_id": claim_id})
            try:
                response = client.send(
                    "/api/worker/claims",
                    {"version": 2, "claim_id": claim_id, "profile_id": str(profile_id)},
                    ClaimResponse,
                )
            except TransportError as exc:
                if once or exc.status not in (0, 408, 429, 500, 503):
                    raise
                time.sleep(5)
                continue
            if not connected:
                print("Worker connected: execution jobs", flush=True)
                connected = True
            if response.lease is None:
                journal.unlink(missing_ok=True)
                if once:
                    return
                time.sleep(response.poll_after_seconds)
                continue
            lease = response.lease
            write(
                journal,
                {"state": "active", "claim_id": claim_id, "attempt_id": str(lease.attempt_id)},
            )
            run_lease(client, lease, state, profile, scenario)
            journal.unlink()
            if once:
                return
