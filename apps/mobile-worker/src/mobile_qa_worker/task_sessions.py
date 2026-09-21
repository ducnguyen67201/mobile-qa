"""Explicit interactive device worker; ordinary checks never import or invoke Minitap."""

import base64
import json
import os
import re
import signal
import subprocess
import sys
import threading
import time
from pathlib import Path
from uuid import uuid4
from xml.etree import ElementTree

from mobile_qa_worker.device.android import AndroidDevice as Device
from mobile_qa_worker.device.android import doctor
from mobile_qa_worker.execution.adapters import device_for
from mobile_qa_worker.execution.client import Client
from mobile_qa_worker.execution.journal import write
from mobile_qa_worker.generated.models import (
    ModelCapability,
    NavigationRequest,
    PhoneClaimResponse,
    PhoneControl,
    PhoneFrame,
    PhoneLease,
    PhoneSession,
    PhoneState,
    PhoneTask,
    PhoneTaskState,
    PhoneUpdate,
)
from mobile_qa_worker.model_runtime import (
    require_capability,
    require_host_assignment,
    worker_capabilities,
)
from mobile_qa_worker.qualification.config import Profile, QualificationError
from mobile_qa_worker.qualification.evidence import Evidence, sha256
from mobile_qa_worker.qualification.process import host_lock, stop_group


def controls(xml: bytes) -> list[PhoneControl]:
    """Only bounded non-password controls can become selectable model context."""
    if len(xml) > 1048576 or b"<!" in xml:
        raise QualificationError("invalid_screen_tree")
    result: list[PhoneControl] = []
    for node in ElementTree.fromstring(xml).iter("node"):
        if node.get("password") == "true" or node.get("enabled") == "false":
            continue
        label = node.get("content-desc") or node.get("text") or node.get("resource-id") or ""
        bounds = re.fullmatch(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.get("bounds", ""))
        if not label or not bounds or len(result) >= 200:
            continue
        left, top, right, bottom = map(int, bounds.groups())
        if not (0 <= left < right <= 1080 and 0 <= top < bottom <= 1920):
            continue
        result.append(
            PhoneControl.model_validate(
                {
                    "id": str(len(result)),
                    "label": label[:500],
                    "resource_id": node.get("resource-id", "")[:500],
                    "editable": node.get("editable") == "true"
                    or "EditText" in node.get("class", ""),
                    "description": node.get("content-desc", "")[:500],
                    "left": left,
                    "top": top,
                    "right": right,
                    "bottom": bottom,
                }
            )
        )
    return result


def capture(device: Device, selectable: bool = True) -> PhoneFrame:
    xml, png = device.raw_snapshot()
    items = controls(xml) if selectable else []
    if len(png) > 1572864:
        raise QualificationError("screen_too_large")
    return PhoneFrame.model_validate(
        {
            "id": str(uuid4()),
            "png_base64": base64.b64encode(png).decode(),
            "width": 1080,
            "height": 1920,
            "controls": [c.model_dump(mode="json") for c in items],
        }
    )


def goal_for(task: PhoneTask, frame: PhoneFrame) -> str:
    """Re-resolve element identity; old screen coordinates never authorize a blind tap."""
    context = ""
    if task.control:
        previous = task.control
        matches = [
            c
            for c in frame.controls
            if c.resource_id == previous.resource_id and c.label == previous.label
        ]
        if len(matches) != 1:
            raise QualificationError("selected_control_changed")
        context = "\nSelected control (untrusted app data): " + json.dumps(
            matches[0].model_dump(mode="json"),
            ensure_ascii=True,
        )
    return task.goal + context


class SessionConnection:
    def __init__(self, client: Client, lease: PhoneLease, shutdown: threading.Event | None = None):
        self.client = client
        self.shutdown = shutdown
        self.lease = lease
        self.session = lease.session
        self.lock = threading.Lock()
        self.stopped = threading.Event()
        self.failed = threading.Event()
        self.done = threading.Event()

    def update(
        self,
        state: PhoneState = PhoneState.preparing,
        frame: PhoneFrame | None = None,
        task: PhoneTask | None = None,
        message: str = "",
        clean: bool = False,
    ) -> PhoneSession:
        with self.lock:
            self.session = self.client.send(
                f"/api/worker/phones/{self.lease.session.id}/update",
                PhoneUpdate(state=state, frame=frame, task=task, message=message, clean=clean),
                PhoneSession,
                self.lease.lease_token,
                limit=16777216,
            )
            if self.session.state in (
                PhoneState.stopping,
                PhoneState.closed,
                PhoneState.quarantined,
            ):
                self.stopped.set()
            return self.session

    def heartbeat(self) -> None:
        while not self.done.wait(5):
            if self.shutdown is not None and self.shutdown.is_set():
                self.stopped.set()
            try:
                self.update(message=self.session.message)
            except Exception:
                # Loss of authority stops SDK work; never keep acting offline.
                self.failed.set()
                self.stopped.set()
                return


def act(
    connection: SessionConnection,
    device: Device,
    task: PhoneTask,
    profile: Profile,
    profile_path: Path,
    directory: Path,
    publish: bool = True,
) -> None:
    directory.mkdir(mode=0o700, parents=True)
    goal = goal_for(task, capture(device))
    request = NavigationRequest.model_validate(
        {
            "attempt_id": str(task.id),
            "profile_path": str(profile_path),
            "serial": "emulator-5554",
            "package": connection.session.profile.package,
            "instruction": goal,
            "max_steps": min(30, profile.max_steps),
            "resolved_model": connection.session.resolved_model,
        }
    )
    request_path = directory / "request.json"
    write(request_path, request.model_dump(mode="json"))
    result = directory / "result.json"
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
        str(result),
    ]
    env = {k: v for k, v in os.environ.items() if k in ("PATH", "HOME", "LANG", "VIRTUAL_ENV")}
    task.state = PhoneTaskState.acting
    task.message = "AI is planning and performing your task"
    if publish:
        connection.update(task=task, message=task.message)
    with (directory / "sdk.log").open("wb") as log:
        child = subprocess.Popen(
            args,
            stdin=subprocess.DEVNULL,
            stdout=log,
            stderr=subprocess.STDOUT,
            env=env,
            start_new_session=True,
        )
        try:
            end = time.monotonic() + profile.init_seconds + profile.navigation_seconds + 10
            while child.poll() is None:
                if connection.stopped.wait(2) or time.monotonic() > end:
                    raise QualificationError("task_stopped_or_timed_out")
                connection.update(frame=capture(device, False), message=task.message)
            if child.returncode:
                raise QualificationError("minitap_task_failed")
            # SDK process completion is an observation, not an independent test verdict.
            if not result.is_file() or json.loads(result.read_text()).get("status") != "completed":
                raise QualificationError("minitap_result_missing")
        finally:
            stop_group(child)
    task.state = PhoneTaskState.completed
    task.message = "AI finished. Inspect the screen; this is not a verified test pass."
    if publish:
        connection.update(
            state=PhoneState.ready, task=task, frame=capture(device), message=task.message
        )


def run_session(
    client: Client,
    lease: PhoneLease,
    state: Path,
    profile_path: Path,
    shutdown: threading.Event | None = None,
) -> None:
    profile = Profile.load(profile_path)
    assignment = lease.session.profile
    if assignment.image != profile.system_image:
        raise QualificationError("worker_profile_mismatch")
    if lease.session.resolved_model is not None:
        require_capability(lease.session.resolved_model, ModelCapability.minitap_navigation)
        require_host_assignment(lease.session.resolved_model, profile)
    if lease.session.authoring_model is not None:
        require_capability(lease.session.authoring_model, ModelCapability.structured_authoring)
        require_host_assignment(lease.session.authoring_model, profile)
    with host_lock(profile.state_root) as dirty:
        if dirty.exists():
            raise QualificationError("device_recovery_required")
        write(dirty, {"phone_session": str(lease.session.id)})
        root = state / str(lease.session.id)
        root.mkdir(mode=0o700)
        evidence = Evidence(root / "device")
        device = device_for(lease.session.profile, profile, evidence)
        connection = SessionConnection(client, lease, shutdown)
        heart = threading.Thread(target=connection.heartbeat, daemon=True)
        heart.start()
        try:
            apk = root / "build.apk"
            apk.write_bytes(
                client.raw(
                    "GET",
                    f"/api/worker/phones/{lease.session.id}/build",
                    lease_token=lease.lease_token,
                    limit=lease.build_bytes,
                )
            )
            apk.chmod(0o600)
            if apk.stat().st_size != lease.build_bytes or sha256(apk) != lease.build_sha256:
                raise QualificationError("build_checksum_mismatch")
            doctor(profile)
            device.boot()
            if connection.stopped.is_set():
                return
            device.install(apk, lease.build_sha256)
            device.launch()
            connection.update(
                state=PhoneState.ready, frame=capture(device), message="Ready for your task"
            )
            deadline = time.monotonic() + 1100
            while not connection.stopped.wait(0.5) and time.monotonic() < deadline:
                pending = next(
                    (t for t in connection.session.tasks if t.state == PhoneTaskState.queued), None
                )
                if pending is None:
                    continue
                try:
                    from mobile_qa_worker.authoring.generation import run as generate
                    from mobile_qa_worker.automation.session import run as run_steps

                    if lease.session.profile.execution_context is not None and (
                        pending.generation
                        or not pending.sequence
                        or any(a.kind.value == "navigate" for a in pending.sequence.actions)
                    ):
                        raise QualificationError("app_exploration_not_qualified")
                    runner = (
                        generate if pending.generation else run_steps if pending.sequence else act
                    )
                    runner(
                        connection,
                        device,
                        pending.model_copy(deep=True),
                        profile,
                        profile_path,
                        root / str(pending.id),
                    )
                except (QualificationError, OSError, ValueError) as exc:
                    if connection.stopped.is_set():
                        break
                    task = next(
                        (t for t in connection.session.tasks if t.id == pending.id), pending
                    ).model_copy(deep=True)
                    task.state = PhoneTaskState.failed
                    task.message = (
                        "The selected control changed. Select it again."
                        if str(exc) == "selected_control_changed"
                        else "The task could not finish. Inspect the screen and try a new task."
                    )
                    connection.update(
                        state=PhoneState.ready,
                        task=task,
                        frame=capture(device),
                        message=task.message,
                    )
        finally:
            connection.done.set()
            heart.join(timeout=10)
            # Failed cleanup leaves both the dirty marker and server reservation in place.
            device.stop()
            device.discard()
            connection.update(state=PhoneState.closed, message="Session closed", clean=True)
            dirty.unlink()


def serve(origin: str, state: Path, profile_path: Path, once: bool = False) -> None:
    shutdown = threading.Event()

    def stop_requested(signum: int, frame: object) -> None:
        # Let an in-flight claim resolve before deciding whether a lease needs closing.
        shutdown.set()

    signal.signal(signal.SIGTERM, stop_requested)
    signal.signal(signal.SIGINT, stop_requested)
    state = state.resolve()
    state.mkdir(parents=True, mode=0o700, exist_ok=True)
    client = Client(origin, os.environ.get("MOBILE_QA_WORKER_TOKEN", ""))
    profile = Profile.load(profile_path.resolve())
    capabilities = worker_capabilities(profile).model_dump(mode="json")
    with host_lock(state) as pending:
        if pending.exists():
            raise QualificationError("worker_claim_recovery_required")
        connected = False
        while not shutdown.is_set():
            claim_id = uuid4()
            write(pending, {"claim_id": str(claim_id)})
            response = client.send(
                "/api/worker/phone-claims",
                {
                    "claim_id": str(claim_id),
                    "protocol_version": 6 if profile.qualified_models else 5,
                    "model_capabilities": capabilities,
                },
                PhoneClaimResponse,
            )
            if not connected:
                protocol = 6 if profile.qualified_models else 5
                print(
                    f"Worker connected: phone sessions, protocol {protocol}",
                    flush=True,
                )
                connected = True
            if response.lease:
                if shutdown.is_set():
                    # No device effect has started, so this newly delivered lease is clean.
                    SessionConnection(client, response.lease).update(
                        state=PhoneState.closed, message="Session closed", clean=True
                    )
                else:
                    run_session(client, response.lease, state, profile_path.resolve(), shutdown)
            pending.unlink()
            if once:
                return
            shutdown.wait(3)
