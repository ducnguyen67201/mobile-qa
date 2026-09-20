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
    with pytest.raises(QualificationError, match="sdk_cleanup_unconfirmed"):
        actions.sdk_action(job(), profile, tmp_path / "profile", tmp_path / "sdk", "Open", 1, [])


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
