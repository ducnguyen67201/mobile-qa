"""Bounded discovery drives the same direct commands testers record and rerun."""

import base64
import io
import os
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import TYPE_CHECKING
from xml.etree import ElementTree

from mobile_qa_worker.execution.journal import write
from mobile_qa_worker.generated.models import (
    AuthoringModelRequest,
    AuthoringModelResponse,
    AuthoringUsage,
    DiscoveryEngine,
    DiscoveryOutcome,
    GenerationProgress,
    GenerationState,
    PhoneFrame,
    PhoneTask,
    PhoneTaskState,
)
from mobile_qa_worker.qualification.config import Profile, QualificationError
from mobile_qa_worker.qualification.device import Device
from mobile_qa_worker.qualification.process import stop_group

if TYPE_CHECKING:
    from mobile_qa_worker.task_sessions import SessionConnection


def redact(frame: PhoneFrame, xml: bytes) -> PhoneFrame:
    """Mask password regions before any provider request, including screen pixels."""
    from PIL import Image, ImageDraw

    if len(xml) > 1048576 or b"<!" in xml:
        raise QualificationError("redaction_failed")
    try:
        tree = ElementTree.fromstring(xml)
        image = Image.open(io.BytesIO(base64.b64decode(frame.png_base64, validate=True)))
        if image.size != (1080, 1920):
            raise QualificationError("redaction_failed")
        draw = ImageDraw.Draw(image)
        for node in tree.iter("node"):
            if node.get("password") != "true":
                continue
            bounds = re.fullmatch(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.get("bounds", ""))
            if not bounds:
                raise QualificationError("redaction_failed")
            left, top, right, bottom = map(int, bounds.groups())
            draw.rectangle((left, top, right, bottom), fill="black")
            frame = frame.model_copy(deep=True)
            frame.controls = [
                c
                for c in frame.controls
                if c.right <= left or c.left >= right or c.bottom <= top or c.top >= bottom
            ]
        output = io.BytesIO()
        image.save(output, format="PNG")
        clean = frame.model_copy(deep=True)
        clean.png_base64 = base64.b64encode(output.getvalue()).decode()
        return clean
    except Exception as exc:
        raise QualificationError("redaction_failed") from exc


def invoke(
    request: AuthoringModelRequest,
    profile: Profile,
    profile_path: Path,
    root: Path,
    connection: "SessionConnection",
    deadline: float,
) -> AuthoringModelResponse:
    root.mkdir(parents=True, mode=0o700, exist_ok=True)
    source, result = root / "request.json", root / "result.json"
    write(source, request.model_dump(mode="json"))
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
        "_authoring-model",
        "--request",
        str(source),
        "--result",
        str(result),
        "--profile",
        str(profile_path),
    ]
    with (root / "model.log").open("wb") as log:
        child = subprocess.Popen(
            args,
            stdin=subprocess.DEVNULL,
            stdout=log,
            stderr=subprocess.STDOUT,
            env=env,
            start_new_session=True,
        )
        try:
            while child.poll() is None:
                if connection.stopped.wait(0.2) or time.monotonic() >= deadline:
                    raise QualificationError("generation_stopped_or_timed_out")
            if child.returncode or not result.is_file() or result.stat().st_size > 1048576:
                raise QualificationError("model_response_unavailable")
            return AuthoringModelResponse.model_validate_json(result.read_bytes())
        finally:
            stop_group(child)


def run(
    connection: "SessionConnection",
    device: Device,
    task: PhoneTask,
    profile: Profile,
    profile_path: Path,
    directory: Path,
) -> None:
    from mobile_qa_worker.authoring.discovery_broker import explore

    request = task.generation
    if request is None:
        raise QualificationError("generation_request_missing")
    progress = task.progress or GenerationProgress(
        engine=DiscoveryEngine.minitap_v1,
        journal=[],
        source_job_id=None,
        state=GenerationState.discovering,
        proposals=[],
        snapshots=[],
        trace=[],
        gaps=[],
        usage=AuthoringUsage(calls=0, input_tokens=0, output_tokens=0, unknown_calls=0),
    )
    task.progress = progress
    task.state = PhoneTaskState.acting
    deadline = time.monotonic() + 180

    def publish(message: str) -> None:
        task.message = message
        connection.update(task=task, message=message)

    def ask(payload: object) -> AuthoringModelResponse:
        if (
            progress.usage.calls >= 16
            or progress.usage.unknown_calls
            or time.monotonic() >= deadline
            or connection.stopped.is_set()
        ):
            raise QualificationError("generation_budget_exhausted")
        progress.usage.calls += 1
        publish("AI is preparing test suggestions")
        try:
            response = invoke(
                AuthoringModelRequest.model_validate(payload),
                profile,
                profile_path,
                directory / str(progress.usage.calls),
                connection,
                min(deadline, time.monotonic() + 50),
            )
        except Exception:
            progress.usage.unknown_calls += 1
            raise
        progress.usage.input_tokens += response.usage.input_tokens
        progress.usage.output_tokens += response.usage.output_tokens
        progress.usage.unknown_calls += response.usage.unknown_calls
        if response.usage.calls != 1 or progress.usage.unknown_calls:
            raise QualificationError("model_usage_unavailable")
        return response

    try:
        progress.state = GenerationState.discovering
        if request.engine != DiscoveryEngine.minitap_v1:
            raise QualificationError("discovery_worker_upgrade_required")
        if not progress.snapshots:
            explore(connection, device, task, profile, profile_path, directory, deadline)
        if connection.stopped.is_set():
            raise QualificationError("discovery_canceled")
        if not progress.trace or progress.usage.unknown_calls:
            progress.state = GenerationState.drafting
            publish("Preparing discovery observations")
            progress.state = GenerationState.needs_input
            progress.gaps.append(
                "No replayable flow was recorded. Describe a test-data journey and try again."
            )
            task.state = PhoneTaskState.completed
            publish("Review the observations and missing prerequisites")
            return
        progress.state = GenerationState.drafting
        publish("Generating named test scenarios")
        response = ask(
            {
                "kind": "propose",
                "journal": [r.model_dump(mode="json") for r in (progress.journal or [])],
                "package": connection.session.profile.package,
                "journey": request.journey,
                "category": request.category.value,
                "snapshots": [s.model_dump(mode="json") for s in progress.snapshots],
                "trace": [c.model_dump(mode="json") for c in progress.trace],
            }
        )
        if response.batch is None or not 1 <= len(response.batch.proposals) <= 5:
            raise QualificationError("invalid_proposal_batch")
        sources = {str(s.id): s for s in progress.snapshots}
        for proposal in response.batch.proposals:
            completed = [
                r for r in (progress.journal or []) if r.outcome == DiscoveryOutcome.completed
            ]
            count = len(proposal.sequence.actions)
            if not 1 <= count <= len(completed) or proposal.path_ids != [
                r.id for r in completed[:count]
            ]:
                raise QualificationError("proposal_path_not_observed")
            for action, receipt in zip(proposal.sequence.actions, completed, strict=False):
                if (
                    action.kind.value != "direct"
                    or action.command != receipt.command
                    or receipt.before_id not in proposal.source_ids
                    or receipt.after_id not in proposal.source_ids
                ):
                    raise QualificationError("proposal_path_not_observed")
            if not proposal.source_ids or any(str(i) not in sources for i in proposal.source_ids):
                raise QualificationError("invalid_proposal_source")
            controls = [c for i in proposal.source_ids for c in sources[str(i)].frame.controls]
            if any(a.kind.value == "navigate" for a in proposal.sequence.actions):
                raise QualificationError("proposal_requires_direct_steps")
            for action in proposal.sequence.actions:
                op = action.command.root if action.command else None
                if op and (
                    op.operation == "tap"
                    or op.operation == "set_text"
                    or op.operation == "wait_for"
                ):
                    selector = op.target.root
                    if not any(
                        (c.resource_id if selector.by == "resource_id" else c.description)
                        == selector.value
                        for c in controls
                    ):
                        raise QualificationError("unobserved_proposal_target")
            for expected in proposal.sequence.checks:
                if any(
                    not any(c.resource_id == target for c in controls)
                    for target in (expected.resource_id, expected.ready_resource_id)
                ):
                    raise QualificationError("unobserved_proposal_check")
        progress.proposals = response.batch.proposals
        progress.state = (
            GenerationState.needs_input
            if any(p.questions or not p.sequence.checks for p in progress.proposals)
            else GenerationState.ready
        )
        task.state = PhoneTaskState.completed
        publish("Review the suggested tests and confirm expected behavior")
    except (QualificationError, OSError, ValueError) as exc:
        # Failed calls count even if provider telemetry is unavailable.
        if progress.usage.calls and not progress.usage.input_tokens:
            progress.usage.unknown_calls = max(1, progress.usage.unknown_calls)
        progress.state = (
            GenerationState.canceled if connection.stopped.is_set() else GenerationState.failed
        )
        progress.gaps.append(
            str(exc)[:500] if isinstance(exc, QualificationError) else "Generation could not finish"
        )
        task.state = PhoneTaskState.failed
        progress.proposals = []
        if not connection.failed.is_set():
            publish("Generation stopped. Existing tests were not changed.")
        if str(exc) == "device_action_uncertain":
            connection.stopped.set()
