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

from mobile_qa_worker.execution.client import Client
from mobile_qa_worker.execution.journal import write
from mobile_qa_worker.generated.models import (
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
from mobile_qa_worker.qualification.config import Profile, QualificationError
from mobile_qa_worker.qualification.device import Device, doctor
from mobile_qa_worker.qualification.evidence import Evidence, sha256
from mobile_qa_worker.qualification.process import host_lock, stop_group
from mobile_qa_worker.qualification.verifier import validate_png


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
                    "left": left,
                    "top": top,
                    "right": right,
                    "bottom": bottom,
                }
            )
        )
    return result


def capture(device: Device, selectable: bool = True) -> PhoneFrame:
    items: list[PhoneControl] = []
    if selectable:
        device.adb("shell", "uiautomator", "dump", "/data/local/tmp/mobile-qa.xml")
        items = controls(device.adb("exec-out", "cat", "/data/local/tmp/mobile-qa.xml"))
    png = device.adb("exec-out", "screencap", "-p")
    validate_png(png)
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
    def __init__(self, client: Client, lease: PhoneLease):
        self.client = client
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
                limit=3145728,
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
) -> None:
    directory.mkdir(mode=0o700)
    goal = goal_for(task, capture(device))
    request = NavigationRequest.model_validate(
        {
            "attempt_id": str(task.id),
            "profile_path": str(profile_path),
            "serial": "emulator-5554",
            "package": connection.session.profile.package,
            "instruction": goal,
            "max_steps": min(30, profile.max_steps),
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
    task.message = "Minitap is planning and performing your task"
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
    task.message = "Minitap finished. Inspect the screen; this is not a verified test pass."
    connection.update(
        state=PhoneState.ready, task=task, frame=capture(device), message=task.message
    )


def run_session(client: Client, lease: PhoneLease, state: Path, profile_path: Path) -> None:
    profile = Profile.load(profile_path)
    if (
        lease.session.profile.package != "ai.mobileqa.demo"
        or lease.session.profile.model != profile.model
    ):
        raise QualificationError("worker_profile_mismatch")
    # This first slice uses the already-qualified demo device adapter. No arbitrary
    # package is admitted through the qualification SDK seam.
    with host_lock(profile.state_root) as dirty:
        if dirty.exists():
            raise QualificationError("device_recovery_required")
        write(dirty, {"phone_session": str(lease.session.id)})
        root = state / str(lease.session.id)
        root.mkdir(mode=0o700)
        evidence = Evidence(root / "device")
        device = Device(profile, evidence)
        connection = SessionConnection(client, lease)
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
                    act(
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
                    task = pending.model_copy(deep=True)
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
    def stop_requested(signum: int, frame: object) -> None:
        raise KeyboardInterrupt

    signal.signal(signal.SIGTERM, stop_requested)
    state = state.resolve()
    state.mkdir(parents=True, mode=0o700, exist_ok=True)
    client = Client(origin, os.environ.get("MOBILE_QA_WORKER_TOKEN", ""))
    with host_lock(state) as pending:
        if pending.exists():
            raise QualificationError("worker_claim_recovery_required")
        while True:
            claim_id = uuid4()
            write(pending, {"claim_id": str(claim_id)})
            response = client.send(
                "/api/worker/phone-claims", {"claim_id": str(claim_id)}, PhoneClaimResponse
            )
            if response.lease:
                run_session(client, response.lease, state, profile_path.resolve())
            pending.unlink()
            if once:
                return
            time.sleep(3)
