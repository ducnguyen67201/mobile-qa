"""Deterministic device seam; no emulator, network or model credentials."""

import base64
import io
from types import SimpleNamespace
from uuid import uuid4

import pytest

from mobile_qa_worker.authoring.generation import redact
from mobile_qa_worker.automation.direct import execute, resolve
from mobile_qa_worker.generated.models import DirectCommand, DirectTarget, PhoneFrame
from mobile_qa_worker.qualification.config import QualificationError

PACKAGE = "ai.mobileqa.demo"
XML = (
    b'<hierarchy><node package="ai.mobileqa.demo" '
    b'resource-id="ai.mobileqa.demo:id/input" class="android.widget.EditText" '
    b'enabled="true" bounds="[10,10][500,100]" /></hierarchy>'
)
TARGET = DirectTarget.model_validate({"by": "resource_id", "value": PACKAGE + ":id/input"})


class Device:
    direct_automation = False
    remote_commands = False
    serial = "emulator-5554"
    adb_server_port = 5037

    def adb(self, *args):
        return XML

    def launch(self):
        pass


class Rpc:
    def __init__(self, uncertain=False):
        self.texts = []
        self.uncertain = uncertain

    def __call__(self, **kwargs):
        assert kwargs["resourceId"] == PACKAGE + ":id/input"
        assert kwargs["packageName"] == PACKAGE
        return self

    def set_text(self, text, timeout):
        self.texts.append(text)
        if self.uncertain:
            raise TimeoutError("response lost after write")

    def click(self, timeout):
        pass


@pytest.mark.parametrize("text", ["", "Xin chào 👋", '"; $(echo nope)\nnext line'])
def test_literal_set_text_uses_rpc_without_model(text, monkeypatch):
    monkeypatch.delenv("OPENAI_API_KEY", raising=False)
    rpc = Rpc()
    command = DirectCommand.model_validate(
        {"operation": "set_text", "target": TARGET.model_dump(mode="json"), "text": text}
    )
    execute(Device(), PACKAGE, command, rpc)
    assert rpc.texts == [text]


@pytest.mark.parametrize(
    "xml",
    [
        XML.replace(b"</hierarchy>", XML[11:]),
        XML.replace(b'enabled="true"', b'enabled="false"'),
        XML.replace(b'enabled="true"', b'password="true" enabled="true"'),
        XML.replace(b"[500,100]", b"[2000,100]"),
        XML.replace(b"ai.mobileqa.demo", b"other.app"),
    ],
)
def test_rejects_ambiguous_disabled_secret_outside_targets(xml):
    with pytest.raises(QualificationError):
        resolve(xml, PACKAGE, TARGET, True)


def test_timeout_after_effect_never_retries():
    rpc = Rpc(uncertain=True)
    command = DirectCommand.model_validate(
        {"operation": "set_text", "target": TARGET.model_dump(mode="json"), "text": "only once"}
    )
    with pytest.raises(QualificationError, match="device_action_uncertain"):
        execute(Device(), PACKAGE, command, rpc)
    assert rpc.texts == ["only once"]


def test_cancel_before_action_has_no_effect():
    rpc = Rpc()
    command = DirectCommand.model_validate(
        {"operation": "set_text", "target": TARGET.model_dump(mode="json"), "text": "no"}
    )
    with pytest.raises(QualificationError, match="task_stopped"):
        execute(Device(), PACKAGE, command, rpc, stopped=lambda: True)
    assert not rpc.texts


def test_password_pixels_are_redacted_and_invalid_redaction_fails_closed():
    Image = pytest.importorskip("PIL.Image")
    output = io.BytesIO()
    Image.new("RGB", (1080, 1920), "white").save(output, format="PNG")
    frame = PhoneFrame(
        id=uuid4(),
        width=1080,
        height=1920,
        png_base64=base64.b64encode(output.getvalue()).decode(),
        controls=[],
    )
    clean = redact(
        frame, b'<hierarchy><node password="true" bounds="[10,10][500,100]" /></hierarchy>'
    )
    image = Image.open(io.BytesIO(base64.b64decode(clean.png_base64)))
    assert image.getpixel((20, 20)) == (0, 0, 0)
    assert image.getpixel((600, 600)) == (255, 255, 255)
    with pytest.raises(QualificationError, match="redaction_failed"):
        redact(frame, b"<!DOCTYPE bad><hierarchy/>")


def test_sequence_dispatches_only_explicit_ai_steps(monkeypatch, tmp_path):
    import threading

    from mobile_qa_worker import task_sessions
    from mobile_qa_worker.automation import session as runner
    from mobile_qa_worker.generated.models import PhoneTask

    calls = []
    frame = PhoneFrame(id=uuid4(), width=1080, height=1920, png_base64="synthetic", controls=[])
    monkeypatch.setattr(runner, "execute", lambda *a, **kw: calls.append("direct"))
    monkeypatch.setattr(task_sessions, "act", lambda *a, **kw: calls.append("ai"))
    monkeypatch.setattr(task_sessions, "capture", lambda *_: frame)
    updates = []
    connection = SimpleNamespace(
        stopped=threading.Event(),
        session=SimpleNamespace(
            profile=SimpleNamespace(package=PACKAGE),
            resolved_model={
                "reference": {"key": "approved", "revision": 1},
                "display_name": "Approved",
                "provider": "open_ai",
                "provider_model": "provider-model",
                "capabilities": ["minitap_navigation"],
            },
        ),
        update=lambda **kw: updates.append(kw["task"].model_copy(deep=True)),
    )
    task = PhoneTask.model_validate(
        {
            "id": str(uuid4()),
            "goal": "Mixed",
            "control": None,
            "state": "queued",
            "message": "",
            "sequence": {
                "actions": [
                    {
                        "id": "one",
                        "checkpoint_id": "one",
                        "kind": "direct",
                        "instruction": "",
                        "command": {"operation": "back"},
                    },
                    {
                        "id": "two",
                        "checkpoint_id": "two",
                        "kind": "navigate",
                        "instruction": "Explore",
                    },
                ],
                "checks": [],
            },
        }
    )
    runner.run(
        connection,
        Device(),
        task,
        SimpleNamespace(model="test"),
        tmp_path / "profile",
        tmp_path / "task",
    )
    assert calls == ["direct", "ai"]
    assert updates[0].steps[0].state.value == "started"
    assert updates[-1].state.value == "completed"


def test_capture_shares_active_rpc_instead_of_starting_a_second_instrumentation(monkeypatch):
    from mobile_qa_worker.automation import direct

    device = Device()
    device.direct_automation = True
    monkeypatch.setattr(
        direct.importlib,
        "import_module",
        lambda _: SimpleNamespace(
            connect=lambda _: SimpleNamespace(dump_hierarchy=lambda **_: XML.decode())
        ),
    )
    monkeypatch.setattr(
        device,
        "adb",
        lambda *args: pytest.fail("shell dumper conflicts with active instrumentation"),
    )
    assert direct.hierarchy(device) == XML


@pytest.mark.parametrize(
    ("ready", "rows", "reason"),
    [
        (True, [], "assertion_failed"),
        (True, ["Buy eggs"], "assertion_failed"),
        (True, ["Buy milk", "Buy milk"], "target_not_unique"),
        (False, [], "screen_not_ready"),
        (True, ["Buy milk"], None),
    ],
)
def test_result_missing_on_ready_screen_is_failure(ready, rows, reason, monkeypatch):
    from mobile_qa_worker.automation import direct
    from mobile_qa_worker.generated.models import ExpectedCheck

    def node(name, text=""):
        return f'<node package="{PACKAGE}" resource-id="{PACKAGE}:id/{name}" text="{text}" />'

    xml = "<hierarchy>" + node("container") + (node("ready") if ready else "")
    xml += "".join(node("row", text) for text in rows) + "</hierarchy>"
    monkeypatch.setattr(direct, "hierarchy", lambda _: xml.encode())
    clock = iter([0, 2])
    monkeypatch.setattr(direct.time, "monotonic", lambda: next(clock))
    expected = ExpectedCheck.model_validate(
        {
            "id": "persisted",
            "checkpoint_id": "restarted",
            "description": "Task survives restart",
            "method": "ui_property_equals_v1",
            "resource_id": PACKAGE + ":id/row",
            "ready_resource_id": PACKAGE + ":id/ready",
            "property": "text",
            "expected": "Buy milk",
            "text_filter": "",
            "required": True,
            "observation_seconds": 1,
            "prerequisite_check_ids": [],
        }
    )
    if reason:
        with pytest.raises(QualificationError, match=reason):
            direct.check(Device(), PACKAGE, expected)
    else:
        direct.check(Device(), PACKAGE, expected)
