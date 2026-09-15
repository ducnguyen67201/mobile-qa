"""Synthetic screen context only; no emulator, model or network access."""

from uuid import uuid4

import pytest

from mobile_qa_worker.generated.models import PhoneFrame, PhoneTask
from mobile_qa_worker.qualification.config import QualificationError
from mobile_qa_worker.task_sessions import controls, goal_for

XML = (
    b'<hierarchy><node text="Save" resource-id="app/save" '
    b'bounds="[10,20][100,80]" enabled="true"/>'
    b'<node text="secret" password="true" bounds="[10,90][100,120]"/></hierarchy>'
)


def test_controls_filter_passwords_and_preserve_bounds():
    items = controls(XML)
    assert len(items) == 1
    assert items[0].label == "Save"
    assert items[0].left == 10


@pytest.mark.parametrize(
    "xml",
    [
        b"<!DOCTYPE tree><tree/>",
        b'<hierarchy><node bounds="[-1,0][10,20]" text="Bad"/></hierarchy>',
    ],
)
def test_invalid_or_outside_controls(xml):
    if b"<!" in xml:
        with pytest.raises(QualificationError):
            controls(xml)
    else:
        assert controls(xml) == []


def test_goal_uses_current_control_and_rejects_missing_identity():
    items = controls(XML)
    frame = PhoneFrame.model_validate(
        {
            "id": str(uuid4()),
            "png_base64": "synthetic",
            "width": 1080,
            "height": 1920,
            "controls": items,
        }
    )
    task = PhoneTask.model_validate(
        {
            "id": str(uuid4()),
            "goal": "Save my task",
            "control": items[0],
            "state": "queued",
            "message": "",
        }
    )
    assert goal_for(task, frame).startswith("Save my task\nSelected control")
    frame.controls = []
    with pytest.raises(QualificationError, match="selected_control_changed"):
        goal_for(task, frame)
    task.control = None
    assert goal_for(task, frame) == "Save my task"


def test_shutdown_during_empty_claim_leaves_no_dirty_marker(tmp_path, monkeypatch):
    from types import SimpleNamespace

    from mobile_qa_worker import task_sessions

    handlers = {}
    monkeypatch.setattr(
        task_sessions.signal, "signal", lambda sig, callback: handlers.update({sig: callback})
    )

    class Client:
        def __init__(self, *_):
            pass

        def send(self, *_):
            handlers[task_sessions.signal.SIGTERM](0, None)
            return SimpleNamespace(lease=None)

    monkeypatch.setattr(task_sessions, "Client", Client)
    task_sessions.serve("http://127.0.0.1:5150", tmp_path, tmp_path / "unused.toml")
    assert not (tmp_path / "dirty.json").exists()
