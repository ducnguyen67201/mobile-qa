"""Offline transport and action evidence tests; never boot or call a model."""

import json
import os
import subprocess
import sys
from pathlib import Path
from uuid import uuid4

import pytest

from mobile_qa_worker.execution.client import Client, TransportError
from mobile_qa_worker.execution.fake import execute, png
from mobile_qa_worker.execution.journal import read, write
from mobile_qa_worker.generated.models import ClaimResponse, ExecutionJob, ExecutionLease
from mobile_qa_worker.qualification.verifier import validate_png

ROOT = Path(__file__).resolve().parents[3]


def job():
    definition = json.loads(
        (ROOT / "contracts/fixtures/execution/persistence-case.json").read_text()
    )["content"]
    return ExecutionJob.model_validate(
        {
            "attempt_id": str(uuid4()),
            "case_index": 0,
            "manifest": {
                "app_id": str(uuid4()),
                "build_id": str(uuid4()),
                "build_sha256": "a" * 64,
                "build_bytes": 1,
                "plan_version_id": str(uuid4()),
                "plan_hash": "b" * 64,
                "environment_revision": 1,
                "profile": {
                    "id": str(uuid4()),
                    "name": "Synthetic",
                    "driver": "fake",
                    "package": "ai.mobileqa.demo",
                    "adapter": "demo_persistence_v1",
                    "device_identity": "synthetic",
                    "image": "synthetic",
                    "model": "none",
                    "qualified": True,
                    "qualification_reference": "test",
                    "max_apk_bytes": 100,
                },
                "cases": [
                    {
                        "definition_id": str(uuid4()),
                        "content_hash": "c" * 64,
                        "data_variant": "default",
                        "required": True,
                        "case": definition,
                    }
                ],
                "budget": {"duration_seconds": 600, "max_steps": 30, "artifact_bytes": 16777216},
                "diagnostic_retries": 0,
                "exclusions": [],
            },
        }
    )


def test_fake_retains_created_and_contradictory_reopened_state(tmp_path):
    j = job()
    execute(j, tmp_path, "fail")
    task = "qa-" + str(j.attempt_id)
    assert task in (tmp_path / "created.xml").read_text()
    assert task not in (tmp_path / "reopened.xml").read_text()
    validate_png(png())


def test_fake_blocked_uses_prerequisite_marker(tmp_path):
    execute(job(), tmp_path, "blocked")
    assert "prerequisite_unavailable" in (tmp_path / "created.xml").read_text()


def test_journal_is_private_and_replaces_atomically(tmp_path):
    path = tmp_path / "journal.json"
    write(path, {"state": "claiming", "claim_id": "one"})
    write(path, {"state": "active", "attempt_id": "two"})
    assert read(path) == {"state": "active", "attempt_id": "two"}
    assert path.stat().st_mode & 0o777 == 0o600
    assert not path.with_suffix(".pending").exists()


def test_journal_rejects_symlink(tmp_path):
    target = tmp_path / "target"
    target.write_text("{}")
    link = tmp_path / "journal"
    link.symlink_to(target)
    with pytest.raises(ValueError):
        write(link, {"state": "active"})


@pytest.mark.parametrize(
    "origin",
    [
        "http://example.com",
        "https://user:pass@example.com",
        "https://example.com/path",
        "https://example.com?token=x",
    ],
)
def test_client_refuses_unsafe_origin(origin):
    with pytest.raises(ValueError):
        Client(origin, "x" * 64)


def test_client_does_not_follow_redirect_or_expose_body():
    from mobile_qa_worker.execution.client import NoRedirect

    with pytest.raises(TransportError) as e:
        NoRedirect().redirect_request(None, None, 302, "secret", {}, "https://elsewhere.invalid")
    assert str(e.value) == "execution_http_302"


def test_generated_boundary_rejects_extra_and_incomplete_claim():
    with pytest.raises(ValueError):
        ClaimResponse.model_validate_json(
            '{"lease":null,"poll_after_seconds":5,"token":"secret"}', strict=True
        )
    with pytest.raises(ValueError):
        ExecutionLease.model_validate_json("{}", strict=True)


def test_worker_refuses_restart_of_active_attempt(tmp_path):
    write(tmp_path / "execution.json", {"state": "active", "claim_id": "one"})
    code = """
import sys
from pathlib import Path
from uuid import uuid4
from mobile_qa_worker.execution.runner import serve
try:
    serve('http://127.0.0.1:9', uuid4(), Path(sys.argv[1]), None, once=True)
except ValueError as e:
    assert str(e)=='active_execution_requires_operator_recovery'
else:
    raise AssertionError('replayed active execution')
"""
    subprocess.run(
        [sys.executable, "-c", code, str(tmp_path)],
        env={**os.environ, "MOBILE_QA_WORKER_TOKEN": "x" * 64},
        check=True,
    )


def test_android_toolchain_checked_before_claim(tmp_path, monkeypatch):
    from test_device import profile

    from mobile_qa_worker.execution import runner
    from mobile_qa_worker.qualification.config import QualificationError

    host = profile(tmp_path)
    monkeypatch.setenv("JAVA_HOME", str(tmp_path / "jdk17"))
    monkeypatch.setenv("OPENAI_API_KEY", "must-not-reach-toolchain")
    monkeypatch.setenv("MOBILE_QA_WORKER_TOKEN", "must-not-reach-child")
    child_env = runner.child_environment()
    assert child_env["JAVA_HOME"] == str(tmp_path / "jdk17")
    assert "OPENAI_API_KEY" not in child_env
    assert "MOBILE_QA_WORKER_TOKEN" not in child_env
    observed = []

    def command(args, timeout, env):
        observed.append((args, timeout, env))
        raise QualificationError("command_failed")

    monkeypatch.setattr("mobile_qa_worker.qualification.process.command", command)
    with pytest.raises(QualificationError, match="android_toolchain_unavailable"):
        runner.preflight_android_tools(host)
    assert observed[0][0] == [
        str(host.sdk_root / "cmdline-tools/19.0/bin/avdmanager"),
        "list",
        "device",
    ]
    assert observed[0][2]["JAVA_HOME"] == str(tmp_path / "jdk17")
    assert "OPENAI_API_KEY" not in observed[0][2]


@pytest.mark.parametrize("emulator_launched", [False, True])
def test_failed_setup_only_auto_cleans_before_emulator_launch(
    tmp_path, monkeypatch, emulator_launched
):
    from contextlib import nullcontext

    from test_device import profile

    from mobile_qa_worker.execution import actions
    from mobile_qa_worker.generated.models import Driver
    from mobile_qa_worker.qualification.config import QualificationError

    host = profile(tmp_path)
    host.state_root.mkdir()
    assigned = job()
    assigned.manifest.profile.image = host.system_image
    assigned.manifest.profile.driver = Driver.minitap
    calls = []

    class Device:
        ever_launched = emulator_launched

        def boot(self, **kwargs):
            calls.append("boot")
            raise QualificationError("avd_creation_failed")

        def stop(self):
            calls.append("stop")

        def discard(self):
            calls.append("discard")

    monkeypatch.setattr(actions.Profile, "load", lambda _: host)
    monkeypatch.setattr(actions, "Device", lambda *args: Device())
    monkeypatch.setattr(actions, "doctor", lambda _: None)
    monkeypatch.setattr(actions, "boot_id", lambda: "fixture")
    monkeypatch.setattr(actions, "cancellation", lambda _: nullcontext())
    monkeypatch.setattr(actions, "backend", lambda _: nullcontext())
    result = actions.execute(assigned, tmp_path / "profile", tmp_path, "pass")
    assert calls == (
        ["boot", "stop", "discard", "boot", "stop"]
        if emulator_launched
        else ["boot", "stop", "discard"]
    )
    assert result.reason == "avd_creation_failed"
    assert result.outcome.value == "inconclusive"
    assert result.reset.value == ("quarantined" if emulator_launched else "verified_clean")
    assert result.stopped is not emulator_launched
    assert (host.state_root / "dirty.json").exists() is emulator_launched


def test_client_actual_http_response_and_redirect_boundaries():
    from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
    from threading import Thread

    class Handler(BaseHTTPRequestHandler):
        def do_POST(self):
            if self.path.endswith("/redirect"):
                self.send_response(302)
                self.send_header("Location", "http://127.0.0.1:9/private")
                self.end_headers()
                return
            assert self.headers["Authorization"] == "Bearer " + "x" * 64
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'{"lease":null,"poll_after_seconds":5}')

        def log_message(self, *_args):
            pass

    with ThreadingHTTPServer(("127.0.0.1", 0), Handler) as server:
        thread = Thread(target=server.serve_forever, daemon=True)
        thread.start()
        try:
            c = Client(f"http://127.0.0.1:{server.server_port}", "x" * 64)
            assert c.send("/api/claims", {}, ClaimResponse).lease is None
            with pytest.raises(TransportError):
                c.send("/api/redirect", {}, ClaimResponse)
            with pytest.raises(ValueError):
                c.raw("POST", "/api/claims", b"{}", limit=2)
        finally:
            server.shutdown()
            thread.join()


def test_sdk_unconfirmed_process_stop_is_quarantined(tmp_path, monkeypatch):
    from types import SimpleNamespace

    from mobile_qa_worker.execution import actions
    from mobile_qa_worker.qualification.config import QualificationError

    class Child:
        returncode = 0

        def wait(self, **kwargs):
            return 0

    monkeypatch.setattr(actions.subprocess, "Popen", lambda *args, **kwargs: Child())

    def cannot_stop(child):
        raise QualificationError("process_group_not_stopped")

    monkeypatch.setattr(actions, "stop_group", cannot_stop)
    profile = SimpleNamespace(
        doppler_project="unused", doppler_config="unused", init_seconds=1, navigation_seconds=1
    )
    assigned = job()
    assigned.manifest.resolved_model = {
        "reference": {"key": "synthetic.openai", "revision": 1},
        "display_name": "Synthetic",
        "provider": "open_ai",
        "provider_model": "synthetic-model",
        "capabilities": ["minitap_navigation"],
    }
    with pytest.raises(QualificationError, match="sdk_cleanup_unconfirmed"):
        actions.sdk_action(assigned, profile, tmp_path / "profile", tmp_path / "sdk", "Open", 1, [])


def test_checkpoint_deadline_respects_approved_window(monkeypatch):
    from types import SimpleNamespace

    from mobile_qa_worker.qualification import device
    from mobile_qa_worker.qualification.config import QualificationError

    ticks = iter([0.0, 2.0])
    monkeypatch.setattr(device.time, "monotonic", lambda: next(ticks))
    fake_device = SimpleNamespace(profile=SimpleNamespace(assertion_seconds=60))
    with pytest.raises(QualificationError, match="observation_timeout"):
        device.Device.capture(fake_device, "checkpoint", "task", timeout=1)


def test_long_poll_has_longer_timeout_than_heartbeat():
    from io import BytesIO

    class Opener:
        def __init__(self):
            self.timeouts = []

        def open(self, request, timeout):
            self.timeouts.append(timeout)
            return BytesIO(b'{"lease":null,"poll_after_seconds":0}')

    client = Client("http://127.0.0.1:5151", "x" * 64)
    opener = Opener()
    client.opener = opener
    result = client.send("/api/worker/claims", {}, ClaimResponse)
    assert result.lease is None
    assert result.poll_after_seconds == 0
    client.raw("POST", "/api/worker/attempts/example/heartbeat", b"{}")
    assert opener.timeouts == [35, 5]


def test_user_authored_published_case_uses_the_existing_worker_protocol():
    published = job().model_dump(mode="json")
    published["manifest"]["cases"][0]["case"]["provenance"] = "user_authored"
    parsed = ExecutionJob.model_validate(published)
    assert parsed.manifest.cases[0].case.provenance == "user_authored"
    assert ExecutionJob.model_validate_json(parsed.model_dump_json()) == parsed


def test_saved_case_manifest_keeps_legacy_plan_shape_optional():
    """Protocol four accepts saved-case sources without inventing a plan."""
    from mobile_qa_worker.generated.models import RunManifest

    raw = job().manifest.model_dump(mode="json", exclude_none=True)
    legacy = RunManifest.model_validate(raw)
    assert legacy.plan_version_id is not None
    raw.pop("plan_version_id")
    raw.pop("plan_hash")
    raw["source"] = {"kind": "saved_case_v1", "case_version_id": raw["cases"][0]["definition_id"]}
    current = RunManifest.model_validate(raw)
    assert current.plan_version_id is None
    assert current.model_dump(mode="json", exclude_none=True)["source"] == raw["source"]


def test_canceled_preparation_does_not_start_device_and_reports_clean(tmp_path, monkeypatch):
    from datetime import UTC, datetime, timedelta
    from types import SimpleNamespace

    from mobile_qa_worker.execution import runner

    assigned = job()
    lease = ExecutionLease.model_validate(
        {
            "attempt_id": assigned.attempt_id,
            "run_id": uuid4(),
            "case_index": 0,
            "generation": 1,
            "lease_token": "lease-private",
            "expires_at": datetime.now(UTC) + timedelta(minutes=1),
            "manifest": assigned.manifest,
        }
    )
    client = SimpleNamespace(send=lambda *_: SimpleNamespace(cancel_requested=True))
    monkeypatch.setattr(runner, "run_prepared", lambda *_: pytest.fail("device started"))
    reports = []
    monkeypatch.setattr(runner, "report_result", lambda *_args: reports.append(_args[-1]))
    runner.run_lease(client, lease, tmp_path, None, "pass")
    assert len(reports) == 1
    assert reports[0].outcome.value == "canceled"
    assert reports[0].reset.value == "verified_clean"
    assert reports[0].stopped is True
    assert not (tmp_path / str(lease.attempt_id) / "build.apk").exists()


def test_slot_child_inherits_only_private_device_capability(monkeypatch):
    from mobile_qa_worker.execution.runner import child_environment

    monkeypatch.setenv("MOBILE_QA_SLOT_SOCKET", "/private/slot.sock")
    monkeypatch.setenv("MOBILE_QA_SLOT_TOKEN", "local-capability")
    monkeypatch.setenv("MOBILE_QA_SLOT_LEASE_ROOT", "/private/lease")
    monkeypatch.setenv("MOBILE_QA_WORKER_TOKEN", "http-secret")
    child = child_environment()
    assert child["MOBILE_QA_SLOT_SOCKET"] == "/private/slot.sock"
    assert child["MOBILE_QA_SLOT_TOKEN"] == "local-capability"
    assert child["MOBILE_QA_SLOT_LEASE_ROOT"] == "/private/lease"
    assert "MOBILE_QA_WORKER_TOKEN" not in child
