"""Failure injection at lifecycle boundaries, without device or model access."""

from contextlib import nullcontext

import pytest
from test_android_adapter import assignment
from test_device import profile
from test_execution import job

from mobile_qa_worker.execution import lifecycle
from mobile_qa_worker.execution.journal import read
from mobile_qa_worker.generated.models import ExecutionJob
from mobile_qa_worker.qualification.config import QualificationError


@pytest.mark.parametrize("failure", ["boot", "start", "ack", "cleanup", None])
def test_actions_require_start_ack_and_cleanup_retains_dirty_until_server_ack(
    tmp_path, monkeypatch, failure
):
    host = profile(tmp_path)
    host.state_root.mkdir()
    j = job().model_dump(mode="json")
    a = assignment()
    j["manifest"]["profile"] = a.model_dump(mode="json")
    c = j["manifest"]["cases"][0]["case"]
    c.update(
        package=a.package,
        adapter=a.adapter,
        actions=[
            dict(
                id="type",
                kind="direct",
                instruction="",
                checkpoint_id="typed",
                command={
                    "operation": "set_text",
                    "target": {"by": "resource_id", "value": a.package + ":id/input"},
                    "text": "Hello",
                },
            )
        ],
        checks=[],
    )
    j = ExecutionJob.model_validate(j)
    calls = []

    def stage(name):
        calls.append(name)
        if name == failure:
            raise QualificationError(name + "_unconfirmed")

    class Device:
        def boot(self, **kw):
            stage("boot")

        def install(self, *args):
            stage("install")

        def launch(self):
            stage("launch")

        def capture_checkpoint(self, *args):
            stage("capture")

        def stop(self):
            stage("cleanup")

        def discard(self):
            stage("discard")

    monkeypatch.setattr(lifecycle.Profile, "load", lambda _: host)
    monkeypatch.setattr(lifecycle, "device_for", lambda *args: Device())
    monkeypatch.setattr(lifecycle, "doctor", lambda _: None)
    monkeypatch.setattr(lifecycle, "boot_id", lambda: "fixture")
    monkeypatch.setattr(lifecycle, "cancellation", lambda _: nullcontext())
    monkeypatch.setattr(lifecycle, "check", lambda *args: stage("start"))
    monkeypatch.setattr(lifecycle, "await_start_ack", lambda *args: stage("ack"))
    monkeypatch.setattr(lifecycle, "direct_execute", lambda *args: stage("action"))
    result = lifecycle.execute(j, tmp_path / "profile", tmp_path)
    assert ("action" in calls) == (failure in (None, "cleanup"))
    assert result.reset.value == ("quarantined" if failure == "cleanup" else "verified_clean")
    assert result.usage == []
    markers = list(host.state_root.glob("*"))
    dirty = [p for p in markers if p.name.endswith("json")]
    assert any(read(p).get("attempt_id") == str(j.attempt_id) for p in dirty)
    if failure is None:
        assert calls.index("ack") < calls.index("action") < calls.index("cleanup")
