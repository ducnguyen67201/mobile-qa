"""Discovery guards with synthetic device effects; the separate SDK fixture uses its real graph."""

import base64
import io
import subprocess
import sys
import threading
import time
from pathlib import Path
from types import SimpleNamespace
from uuid import uuid4

import pytest

from mobile_qa_worker.authoring import discovery_broker as broker
from mobile_qa_worker.generated.models import DiscoveryCall, PhoneFrame, PhoneTask, ResolvedModel
from mobile_qa_worker.qualification.config import QualificationError

PACKAGE = "ai.mobileqa.demo"


def task():
    return PhoneTask.model_validate(
        {
            "id": str(uuid4()),
            "goal": "Explore",
            "state": "acting",
            "message": "",
            "control": None,
            "generation": {
                "id": str(uuid4()),
                "session_id": str(uuid4()),
                "expected_revision": 0,
                "category": "smoke",
                "journey": "Save Hello",
                "allow_writes": True,
                "reuse_job_id": None,
                "engine": "minitap_v1",
            },
            "progress": {
                "engine": "minitap_v1",
                "state": "discovering",
                "snapshots": [],
                "trace": [],
                "journal": [],
                "proposals": [],
                "gaps": [],
                "usage": {"calls": 0, "unknown_calls": 0, "input_tokens": 0, "output_tokens": 0},
            },
        }
    )


@pytest.fixture
def seam(monkeypatch):
    from PIL import Image

    from mobile_qa_worker import task_sessions

    image = io.BytesIO()
    Image.new("RGB", (1080, 1920), "white").save(image, "PNG")
    frame = PhoneFrame(
        id=uuid4(),
        width=1080,
        height=1920,
        controls=[],
        png_base64=base64.b64encode(image.getvalue()).decode(),
    )
    xml = (
        b'<hierarchy><node package="ai.mobileqa.demo" '
        b'resource-id="ai.mobileqa.demo:id/input" enabled="true" text="Hello" '
        b'bounds="[10,10][500,100]"/></hierarchy>'
    )
    monkeypatch.setattr(broker, "hierarchy", lambda *_: xml)
    monkeypatch.setattr(task_sessions, "capture", lambda *_, **__: frame.model_copy(deep=True))
    updates = []
    connection = SimpleNamespace(
        stopped=threading.Event(),
        failed=threading.Event(),
        session=SimpleNamespace(
            profile=SimpleNamespace(package=PACKAGE),
            resolved_model=ResolvedModel.model_validate(
                {
                    "reference": {"key": "synthetic.discovery", "revision": 1},
                    "display_name": "Synthetic discovery",
                    "provider": "open_ai",
                    "provider_model": "synthetic-model",
                    "capabilities": ["minitap_navigation", "structured_authoring"],
                }
            ),
        ),
        update=lambda **kw: updates.append(kw["task"].model_copy(deep=True)),
    )
    device = SimpleNamespace(adb=lambda *_: b"mResumedActivity: ai.mobileqa.demo/.MainActivity")
    effects = []
    monkeypatch.setattr(broker, "execute", lambda *_, **__: effects.append("effect"))
    return (
        broker.DiscoveryBroker(connection, device, task(), time.monotonic() + 10),
        updates,
        effects,
    )


def call(kind, **extra):
    return DiscoveryCall.model_validate({"kind": kind, "id": str(uuid4()), **extra})


def test_intent_ack_before_effect_duplicate_is_once_and_history_immutable(seam):
    b, updates, effects = seam
    request = call("execute", command={"operation": "back"})
    reply = b.handle(request)
    assert effects == ["effect"]
    assert updates[1].progress.journal[0].outcome.value == "pending"
    assert updates[-1].progress.journal[0].outcome.value == "completed"
    assert b.handle(request) == reply
    assert effects == ["effect"]
    changed = request.model_dump(mode="json")
    changed["command"] = {"operation": "restart"}
    with pytest.raises(QualificationError, match="request_conflict"):
        b.handle(DiscoveryCall.model_validate(changed))


def test_observe_only_and_cancel_make_no_effect(seam):
    b, _, effects = seam
    b.request.allow_writes = False
    with pytest.raises(QualificationError, match="scope_required"):
        b.handle(call("execute", command={"operation": "back"}))
    assert not effects
    b.connection.stopped.set()
    with pytest.raises(QualificationError, match="canceled"):
        b.handle(call("observe"))


def test_passive_redraw_between_actions_preserves_recorded_path(seam, monkeypatch):
    from PIL import Image

    from mobile_qa_worker import task_sessions

    b, _, effects = seam
    b.handle(call("execute", command={"operation": "back"}))
    first = b.progress.journal[0]
    frame = b.progress.snapshots[-1].frame.model_copy(deep=True)
    image = io.BytesIO()
    Image.new("RGB", (1080, 1920), "gray").save(image, "PNG")
    frame.png_base64 = base64.b64encode(image.getvalue()).decode()
    monkeypatch.setattr(task_sessions, "capture", lambda *_, **__: frame.model_copy(deep=True))
    b.handle(call("execute", command={"operation": "back"}))
    second = b.progress.journal[1]
    assert first.after_id != second.before_id
    assert all(r.outcome.value == "completed" for r in b.progress.journal)
    assert effects == ["effect", "effect"]


def test_lost_effect_response_halts_next_action(seam, monkeypatch):
    b, _, effects = seam

    def uncertain(*_, **__):
        effects.append("effect")
        raise QualificationError("device_action_uncertain")

    monkeypatch.setattr(broker, "execute", uncertain)
    with pytest.raises(QualificationError, match="uncertain"):
        b.handle(call("execute", command={"operation": "back"}))
    assert b.progress.journal[0].outcome.value == "uncertain"
    with pytest.raises(QualificationError):
        b.handle(call("execute", command={"operation": "back"}))
    assert effects == ["effect"]


def test_reserve_counts_every_role_and_unknown_usage_prevents_more_calls(seam):
    b, _, _ = seam
    reserved = call("reserve")
    b.handle(reserved)
    with pytest.raises(QualificationError, match="duplicate_model_call"):
        b.handle(reserved)
    b.handle(
        DiscoveryCall.model_validate(
            {"kind": "usage", "id": reserved.root.id, "input_tokens": None, "output_tokens": None}
        )
    )
    with pytest.raises(QualificationError, match="budget"):
        b.handle(call("reserve"))
    b.close_usage()
    assert b.progress.usage.calls == b.progress.usage.unknown_calls == 1


def test_budget_reserves_one_drafting_call_and_counts_unfinished_calls(seam):
    b, _, _ = seam
    for _ in range(15):
        b.handle(call("reserve"))
    with pytest.raises(QualificationError, match="budget"):
        b.handle(call("reserve"))
    b.close_usage()
    assert b.progress.usage.unknown_calls == 15


def test_changed_foreground_and_oversized_context_fail_closed(seam):
    b, _, _ = seam
    b.device.adb = lambda *_: b"mResumedActivity other.app/.Main"
    with pytest.raises(QualificationError, match="foreground"):
        b.handle(call("observe"))
    from langchain_core.messages import HumanMessage

    from mobile_qa_worker.authoring.minitap_discovery import check_context

    with pytest.raises(QualificationError, match="context_too_large"):
        check_context([[HumanMessage(content="x" * 65537)]])


@pytest.mark.parametrize("mode", ["direct", "native"])
def test_real_pinned_sdk_graph_offline(tmp_path, mode):
    # Isolate SDK monkeypatches and globals from the worker/test process. No provider/device calls.
    fixture = Path(__file__).with_name("sdk_discovery_fixture.py")
    result = subprocess.run(
        [sys.executable, str(fixture), str(tmp_path), mode],
        capture_output=True,
        text=True,
        timeout=60,
    )
    assert result.returncode == 0, result.stdout[-5000:] + result.stderr[-5000:]
    assert "SDK discovery conformance passed" in result.stdout


@pytest.mark.parametrize("change", ["none", "invented_source", "invented_path", "different_action"])
def test_only_recorded_prefix_can_be_proposed_and_reuse_never_explores(
    seam, monkeypatch, tmp_path, change
):
    from mobile_qa_worker.authoring import generation
    from mobile_qa_worker.generated.models import AuthoringModelResponse

    b, updates, _ = seam
    b.handle(call("execute", command={"operation": "back"}))
    receipt = b.progress.journal[0]
    proposal = {
        "id": str(uuid4()),
        "title": "Navigation smoke",
        "category": "smoke",
        "requirement": "Confirm expected behavior",
        "questions": ["Confirm navigation"],
        "path_ids": [str(receipt.id)],
        "source_ids": [str(receipt.before_id)],
        "sequence": {
            "actions": [
                {
                    "id": "back",
                    "checkpoint_id": "back",
                    "kind": "direct",
                    "instruction": "",
                    "command": {"operation": "back"},
                }
            ],
            "checks": [],
        },
    }
    if change == "invented_source":
        proposal["source_ids"] = [str(uuid4())]
    if change == "invented_path":
        proposal["path_ids"] = [str(uuid4())]
    if change == "different_action":
        proposal["sequence"]["actions"][0]["command"] = {"operation": "restart"}
    monkeypatch.setattr(broker, "explore", lambda *_: pytest.fail("reuse must not explore"))

    def invoke(*args):
        assert args[0].request.root.kind == "propose"
        assert args[0].model.reference == b.connection.session.resolved_model.reference
        return AuthoringModelResponse.model_validate(
            {
                "decision": None,
                "batch": {"proposals": [proposal]},
                "usage": {"calls": 1, "input_tokens": 20, "output_tokens": 10, "unknown_calls": 0},
            }
        )

    monkeypatch.setattr(generation, "invoke", invoke)
    generation.run(
        b.connection, b.device, b.task, SimpleNamespace(), tmp_path / "profile", tmp_path
    )
    assert b.task.state.value == ("completed" if change == "none" else "failed")
    assert b.progress.usage.calls == 1 and b.progress.usage.input_tokens == 20
    if change != "none":
        assert b.progress.proposals == []


def test_drafting_builds_provenance_from_journal_and_rejects_unobserved_suffix(seam):
    from mobile_qa_worker.authoring.proposals import materialize
    from mobile_qa_worker.generated.models import AuthoringModelRequest, DiscoveryDraftBatch

    b, _, _ = seam
    b.handle(call("execute", command={"operation": "back"}))
    request = AuthoringModelRequest.model_validate(
        {
            "kind": "propose",
            "package": PACKAGE,
            "journey": "Navigate",
            "category": "smoke",
            "snapshots": b.progress.snapshots,
            "trace": b.progress.trace,
            "journal": b.journal,
        }
    ).root
    drafts = DiscoveryDraftBatch.model_validate(
        {
            "proposals": [
                {
                    "through_action": 1,
                    "title": "Navigation smoke",
                    "requirement": "Review navigation",
                    "questions": [],
                    "checks": [],
                }
            ]
        }
    )
    proposal = materialize(request, drafts).proposals[0]
    assert proposal.path_ids == [b.journal[0].id]
    assert proposal.source_ids == [b.journal[0].before_id]
    assert proposal.sequence.actions[0].command == b.journal[0].command
    from mobile_qa_worker.generated.models import DiscoveryDraftCheck

    check = DiscoveryDraftCheck.model_validate(
        {
            "after_action": 1,
            "description": "Hello is visible",
            "method": "ui_property_equals_v1",
            "resource_id": PACKAGE + ":id/input",
            "ready_resource_id": PACKAGE + ":id/input",
            "text_filter": "",
            "property": "text",
            "expected": "Hello",
            "required": True,
            "observation_seconds": 2,
        }
    )
    drafts.proposals[0].checks = [check]
    proposal = materialize(request, drafts).proposals[0]
    assert proposal.sequence.checks[0].checkpoint_id == str(b.journal[0].id)
    assert proposal.sequence.checks[0].id == "check-1"
    check.after_action = 2
    with pytest.raises(QualificationError, match="check_not_observed"):
        materialize(request, drafts)
    check.after_action = 1
    drafts.proposals[0].through_action = 2
    with pytest.raises(QualificationError, match="path_not_observed"):
        materialize(request, drafts)
