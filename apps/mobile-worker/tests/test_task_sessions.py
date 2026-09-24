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
    from test_device import profile

    monkeypatch.setattr(task_sessions.Profile, "load", lambda _: profile(tmp_path))
    task_sessions.serve("http://127.0.0.1:5150", tmp_path, tmp_path / "unused.toml")
    assert not (tmp_path / "dirty.json").exists()


@pytest.mark.parametrize(
    "host_key,image_matches,accepted",
    [
        ("different", True, False),
        (None, True, False),
        ("approved", True, True),
        ("approved", False, False),
    ],
)
def test_session_binds_image_and_model_before_side_effects(
    tmp_path, monkeypatch, host_key, image_matches, accepted
):
    from dataclasses import replace
    from types import SimpleNamespace
    from unittest.mock import Mock

    from test_device import profile as host_profile
    from test_execution import job

    from mobile_qa_worker import task_sessions
    from mobile_qa_worker.generated.models import ExecutionProfile, ModelReference, ResolvedModel

    host_ref = ModelReference(key=host_key, revision=1) if host_key else None
    host = replace(host_profile(tmp_path), model_ref=host_ref)
    raw = job().manifest.profile.model_dump(mode="json")
    raw.update(
        driver="minitap",
        model={"key": "approved", "revision": 1},
        image=host.system_image if image_matches else "different-image",
    )
    resolved = ResolvedModel.model_validate(
        {
            "reference": {"key": "approved", "revision": 1},
            "display_name": "Approved",
            "provider": "open_ai",
            "provider_model": "provider-model",
            "capabilities": ["minitap_navigation"],
        }
    )
    lease = SimpleNamespace(
        session=SimpleNamespace(
            id=uuid4(),
            profile=ExecutionProfile.model_validate(raw),
            resolved_model=resolved,
            authoring_model=None,
        )
    )
    monkeypatch.setattr(task_sessions.Profile, "load", lambda _: host)

    class AdmissionReached(Exception):
        pass

    # Stop at the first side-effect boundary: accepted profiles must reach it, mismatches must not.
    lock = Mock(side_effect=AdmissionReached)
    client = Mock()
    monkeypatch.setattr(task_sessions, "host_lock", lock)
    if accepted:
        with pytest.raises(AdmissionReached):
            task_sessions.run_session(client, lease, tmp_path, tmp_path / "host.toml")
        lock.assert_called_once_with(host.state_root)
    else:
        with pytest.raises(
            QualificationError,
            match="^(worker_profile_mismatch|worker_model_reference_mismatch)$",
        ):
            task_sessions.run_session(client, lease, tmp_path, tmp_path / "host.toml")
        lock.assert_not_called()
    assert client.mock_calls == []


def test_phone_canceled_during_install_never_launches_and_cleans(tmp_path, monkeypatch):
    from contextlib import contextmanager
    from types import SimpleNamespace

    from test_device import profile as host_profile
    from test_execution import job

    from mobile_qa_worker import task_sessions
    from mobile_qa_worker.artifacts.preparation import PreparationStopped

    host = host_profile(tmp_path)
    assignment = job().manifest.profile
    assignment.image = host.system_image
    lease = SimpleNamespace(
        session=SimpleNamespace(
            id=uuid4(),
            app_id=uuid4(),
            profile=assignment,
            resolved_model=None,
            authoring_model=None,
        ),
        lease_token="private",
        build_sha256="a" * 64,
        build_bytes=3,
    )
    calls = []
    connections = []
    original_connection = task_sessions.SessionConnection

    def connection(*args):
        instance = original_connection(*args)
        connections.append(instance)
        return instance

    class FakeDevice:
        def boot(self):
            calls.append("boot")

        def install(self, *_):
            calls.append("install")
            connections[0].stopped.set()

        def launch(self):
            pytest.fail("launched after cancellation")

        def stop(self):
            calls.append("stop")

        def discard(self):
            calls.append("discard")

    @contextmanager
    def prepare(*_args, **kwargs):
        kwargs["check"]()
        path = kwargs["target"]
        path.write_bytes(b"apk")
        try:
            yield path
        finally:
            path.unlink()

    def send(*_args, **_kwargs):
        return SimpleNamespace(state=task_sessions.PhoneState.preparing)

    monkeypatch.setattr(task_sessions.Profile, "load", lambda _: host)
    monkeypatch.setattr(task_sessions, "device_for", lambda *_: FakeDevice())
    monkeypatch.setattr(task_sessions, "doctor", lambda _: None)
    monkeypatch.setattr(task_sessions, "prepared_build", prepare)
    monkeypatch.setattr(task_sessions, "SessionConnection", connection)
    with pytest.raises(PreparationStopped, match="canceled"):
        task_sessions.run_session(
            SimpleNamespace(send=send), lease, tmp_path, tmp_path / "host.toml"
        )
    assert calls == ["boot", "install", "stop", "discard"]
    assert not (host.state_root / "dirty.json").exists()
    assert not (tmp_path / str(lease.session.id) / "build.apk").exists()
