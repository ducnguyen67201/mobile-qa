"""One sequence, shared device executor, durable per-step receipts."""

import time
from pathlib import Path
from typing import TYPE_CHECKING

from mobile_qa_worker.automation.direct import check, execute
from mobile_qa_worker.device.android import AndroidDevice as Device
from mobile_qa_worker.generated.models import PhoneState, PhoneTask, PhoneTaskState, StepReceipt
from mobile_qa_worker.qualification.config import Profile, QualificationError

if TYPE_CHECKING:
    from mobile_qa_worker.task_sessions import SessionConnection


def run(
    connection: "SessionConnection",
    device: Device,
    task: PhoneTask,
    profile: Profile,
    profile_path: Path,
    directory: Path,
) -> None:
    from mobile_qa_worker.task_sessions import act, capture

    sequence = task.sequence
    if sequence is None:
        raise QualificationError("sequence_missing")
    deadline = time.monotonic() + 300

    def stopped() -> bool:
        return connection.stopped.is_set() or time.monotonic() >= deadline

    task.state = PhoneTaskState.acting
    for action in sequence.actions:
        if stopped():
            raise QualificationError("task_stopped")
        receipt = StepReceipt.model_validate(
            {"action_id": action.id, "state": "started", "message": "Running step"}
        )
        task.steps.append(receipt)
        connection.update(task=task, message="Running your steps")
        try:
            if action.kind.value == "direct" and action.command:
                execute(device, connection.session.profile.package, action.command, stopped=stopped)
            elif action.kind.value == "restart_app":
                device.adb("shell", "am", "force-stop", connection.session.profile.package)
                device.launch()
            elif action.kind.value == "navigate":
                # Minitap remains an explicit step; it cannot publish a whole-sequence completion.
                if connection.session.resolved_model is None:
                    raise QualificationError("model_profile_required")
                ai_task = task.model_copy(deep=True)
                ai_task.goal = action.instruction
                ai_task.control = None
                act(
                    connection,
                    device,
                    ai_task,
                    profile,
                    profile_path,
                    directory / action.id,
                    publish=False,
                )
            for expected in sequence.checks:
                if expected.checkpoint_id == action.checkpoint_id:
                    check(device, connection.session.profile.package, expected, stopped=stopped)
            receipt.state = receipt.state.__class__.completed
            receipt.message = "Step completed"
        except (QualificationError, OSError, ValueError) as exc:
            reason = str(exc) if isinstance(exc, QualificationError) else "device_action_uncertain"
            receipt.state = (
                receipt.state.__class__.inconclusive
                if reason == "device_action_uncertain"
                else receipt.state.__class__.failed
                if reason == "assertion_failed"
                else receipt.state.__class__.blocked
            )
            receipt.message = reason.replace("_", " ")
            task.state = PhoneTaskState.failed
            task.message = "Step stopped: " + receipt.message
            connection.update(task=task, frame=capture(device), message=task.message)
            if reason == "device_action_uncertain":
                connection.stopped.set()
            return
        connection.update(task=task, frame=capture(device), message=receipt.message)
    task.state = PhoneTaskState.completed
    task.message = (
        "Steps and checks completed"
        if sequence.checks
        else "Actions completed. Add an expected result to verify this test."
    )
    connection.update(
        state=PhoneState.ready, task=task, frame=capture(device), message=task.message
    )
