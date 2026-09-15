"""Parent-owned discovery effects. The SDK receives observations, never device credentials."""

import hashlib
import json
import os
import selectors
import subprocess
import sys
import time
from pathlib import Path
from typing import TYPE_CHECKING
from uuid import UUID, uuid4

from mobile_qa_worker.automation.direct import execute, hierarchy, nodes
from mobile_qa_worker.execution.client import TransportError
from mobile_qa_worker.generated.models import (
    DiscoveryCall,
    DiscoveryOutcome,
    DiscoveryReceipt,
    DiscoveryReply,
    DiscoverySnapshot,
    PhoneTask,
)
from mobile_qa_worker.qualification.config import Profile, QualificationError
from mobile_qa_worker.qualification.device import Device
from mobile_qa_worker.qualification.process import stop_group

if TYPE_CHECKING:
    from mobile_qa_worker.task_sessions import SessionConnection

MAX_MESSAGE = 65536
MAX_REPLY = 3 * 1048576


class DiscoveryBroker:
    def __init__(
        self, connection: "SessionConnection", device: Device, task: PhoneTask, deadline: float
    ):
        if task.progress is None or task.generation is None:
            raise QualificationError("discovery_request_missing")
        self.connection, self.device, self.task = connection, device, task
        self.progress, self.request = task.progress, task.generation
        self.journal = self.progress.journal or []
        self.progress.journal = self.journal
        self.deadline = deadline
        self.calls: set[UUID] = set()
        self.measured: dict[UUID, tuple[int | None, int | None]] = {}
        self.replies: dict[UUID, tuple[str, DiscoveryReply]] = {}
        self.finished = False
        self.halted = False

    def publish(self, message: str) -> None:
        self.task.message = message
        frame = self.progress.snapshots[-1].frame if self.progress.snapshots else None
        try:
            self.connection.update(task=self.task, frame=frame, message=message)
        except TransportError as exc:
            self.connection.failed.set()
            self.connection.stopped.set()
            raise QualificationError("discovery_authority_lost") from exc
        self.guard()

    def guard(self) -> None:
        if self.connection.stopped.is_set() or self.connection.failed.is_set():
            raise QualificationError("discovery_canceled")
        if time.monotonic() >= self.deadline:
            raise QualificationError("discovery_time_limit")

    def observe(self) -> DiscoverySnapshot:
        from mobile_qa_worker.authoring.generation import redact
        from mobile_qa_worker.task_sessions import capture, controls

        self.guard()
        package = self.connection.session.profile.package
        foreground = self.device.adb("shell", "dumpsys", "activity", "activities").decode()
        if not any(
            ("mResumedActivity" in line or "topResumedActivity" in line) and f"{package}/" in line
            for line in foreground.splitlines()
        ):
            raise QualificationError("wrong_foreground_package")
        xml = hierarchy(self.device)
        nodes(xml, package)
        frame = capture(self.device, selectable=False)
        frame.controls = controls(xml)
        frame = redact(frame, xml)
        # Package-local targets only; system keyboard text is not discovery context.
        frame.controls = [c for c in frame.controls if c.resource_id.startswith(package + ":id/")]
        fingerprint = hashlib.sha256(
            json.dumps(
                {
                    "controls": [c.model_dump(mode="json") for c in frame.controls],
                    "image": frame.png_base64,
                },
                sort_keys=True,
            ).encode()
        ).hexdigest()
        # Share only identical redacted observations; a fingerprint is not proof of screen coverage.
        if self.progress.snapshots and self.progress.snapshots[-1].fingerprint == fingerprint:
            return self.progress.snapshots[-1]
        if len(self.progress.snapshots) >= 8:
            raise QualificationError("discovery_observation_limit")
        snapshot = DiscoverySnapshot(id=uuid4(), frame=frame, fingerprint=fingerprint)
        self.progress.snapshots.append(snapshot)
        self.publish("AI is exploring your app")
        return snapshot

    def handle(self, call: DiscoveryCall) -> DiscoveryReply:
        r = call.root
        content = call.model_dump_json()
        if r.kind not in ("reserve", "usage") and r.id in self.replies:
            old, reply = self.replies[r.id]
            if old != content:
                raise QualificationError("discovery_request_conflict")
            return reply
        self.guard()
        snapshot: DiscoverySnapshot | None = None
        if r.kind == "finish":
            self.finished = True
        elif r.kind == "usage":
            values = (r.input_tokens, r.output_tokens)
            if r.id not in self.calls or (r.id in self.measured and self.measured[r.id] != values):
                raise QualificationError("discovery_usage_conflict")
            if r.id not in self.measured:
                self.measured[r.id] = values
                if None in values:
                    self.progress.usage.unknown_calls += 1
                    self.halted = True
                else:
                    incoming, outgoing = values
                    if incoming is None or outgoing is None or max(incoming, outgoing) > 10000000:
                        raise QualificationError("discovery_usage_invalid")
                    self.progress.usage.input_tokens += incoming
                    self.progress.usage.output_tokens += outgoing
                self.publish("AI is exploring your app")
        elif r.kind == "reserve":
            if r.id in self.calls:
                raise QualificationError("discovery_duplicate_model_call")
            if self.halted or self.progress.usage.calls >= 15 or self.progress.usage.unknown_calls:
                raise QualificationError("discovery_model_budget")
            self.calls.add(r.id)
            self.progress.usage.calls += 1
            self.publish("AI is exploring your app")
        elif self.halted:
            raise QualificationError("discovery_stopped")
        elif r.kind == "observe":
            snapshot = self.observe()
        elif r.kind == "execute":
            if not self.request.allow_writes and r.command.root.operation != "wait_for":
                raise QualificationError("discovery_interaction_scope_required")
            if len(self.journal) >= 12 or len(self.progress.snapshots) >= 7:
                raise QualificationError("discovery_evidence_limit")
            before = self.observe()
            if self.journal and self.journal[-1].outcome != DiscoveryOutcome.completed:
                raise QualificationError("discovery_previous_action_incomplete")
            receipt = DiscoveryReceipt(
                id=r.id,
                before_id=before.id,
                command=r.command,
                outcome=DiscoveryOutcome.pending,
                after_id=None,
            )
            self.journal.append(receipt)
            self.publish("AI is performing an observed action")
            try:
                execute(
                    self.device,
                    self.connection.session.profile.package,
                    r.command,
                    stopped=self.connection.stopped.is_set,
                )
                snapshot = self.observe()
                receipt.after_id = snapshot.id
                receipt.outcome = DiscoveryOutcome.completed
                self.progress.trace.append(r.command)
            except Exception:
                # Capture can fail after a successful effect: preserve uncertainty, never retry.
                receipt.outcome = DiscoveryOutcome.uncertain
                self.halted = True
                self.connection.stopped.set()
                raise
            finally:
                if not self.connection.failed.is_set():
                    self.task.message = "Discovery action recorded"
                    self.connection.update(task=self.task, message=self.task.message)
        reply = DiscoveryReply(id=r.id, error=None, snapshot=snapshot)
        if r.kind not in ("reserve", "usage"):
            self.replies[r.id] = content, reply
        return reply

    def close_usage(self) -> None:
        self.progress.usage.unknown_calls += len(self.calls - self.measured.keys())
        self.calls = set(self.measured)


def explore(
    connection: "SessionConnection",
    device: Device,
    task: PhoneTask,
    profile: Profile,
    profile_path: Path,
    directory: Path,
    deadline: float,
) -> None:
    """Bounded pipe protocol; SDK logging goes to /dev/null, never into public evidence."""
    directory.mkdir(mode=0o700, parents=True, exist_ok=True)
    broker = DiscoveryBroker(connection, device, task, min(deadline - 30, time.monotonic() + 150))
    broker.observe()
    if task.generation is None:
        raise QualificationError("discovery_request_missing")
    request_path = directory / "discovery-request.json"
    from mobile_qa_worker.execution.journal import write

    write(request_path, task.generation.model_dump(mode="json"))
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
        "_minitap-discovery",
        "--request",
        str(request_path),
        "--profile",
        str(profile_path),
    ]
    env = {k: v for k, v in os.environ.items() if k in ("PATH", "HOME", "LANG", "VIRTUAL_ENV")}
    child = subprocess.Popen(
        args,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        env=env,
        start_new_session=True,
    )
    try:
        if child.stdout is None or child.stdin is None:
            raise QualificationError("discovery_pipe_unavailable")
        with selectors.DefaultSelector() as selector:
            selector.register(child.stdout, selectors.EVENT_READ)
            buffer = b""
            while not broker.finished:
                broker.guard()
                if not selector.select(0.2):
                    if child.poll() is not None:
                        raise QualificationError("discovery_agent_stopped")
                    continue
                chunk = os.read(child.stdout.fileno(), 8192)
                if not chunk:
                    raise QualificationError("discovery_agent_stopped")
                buffer += chunk
                if len(buffer) > MAX_MESSAGE:
                    raise QualificationError("discovery_message_too_large")
                while b"\n" in buffer:
                    line, buffer = buffer.split(b"\n", 1)
                    call = DiscoveryCall.model_validate_json(line)
                    try:
                        reply = broker.handle(call)
                    except QualificationError as exc:
                        broker.halted = True
                        if str(exc) not in broker.progress.gaps:
                            broker.progress.gaps.append(str(exc))
                        reply = DiscoveryReply(id=call.root.id, error=str(exc), snapshot=None)
                    encoded = reply.model_dump_json().encode() + b"\n"
                    if len(encoded) > MAX_REPLY:
                        raise QualificationError("discovery_reply_too_large")
                    # Never block the lease owner on a child that stopped reading.
                    os.set_blocking(child.stdin.fileno(), False)
                    with selectors.DefaultSelector() as output:
                        output.register(child.stdin, selectors.EVENT_WRITE)
                        remaining = memoryview(encoded)
                        while remaining:
                            broker.guard()
                            if output.select(0.2):
                                try:
                                    written = os.write(child.stdin.fileno(), remaining)
                                    remaining = remaining[written:]
                                except BlockingIOError:
                                    continue
    except (QualificationError, OSError, ValueError) as exc:
        reason = str(exc) if isinstance(exc, QualificationError) else "discovery_connection_lost"
        broker.progress.gaps.append(reason[:500])
    finally:
        stop_group(child)
        if child.stdin:
            child.stdin.close()
        if child.stdout:
            child.stdout.close()
        broker.close_usage()
        for receipt in broker.journal:
            if receipt.outcome == DiscoveryOutcome.pending:
                receipt.outcome = DiscoveryOutcome.uncertain
                connection.stopped.set()
