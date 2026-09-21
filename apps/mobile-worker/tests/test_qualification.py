"""Offline contract, CLI and immutable-evidence checks; no real device or model."""

import json
import subprocess
import sys
from uuid import uuid4

import pytest
from pydantic import ValidationError

from mobile_qa_worker.qualification.config import Profile, QualificationError, parse_request
from mobile_qa_worker.qualification.evidence import (
    Evidence,
    atomic_json,
    safe_path,
    sha256,
    validate_result,
)


def request_data(tmp_path):
    return {
        "version": 1,
        "attempt_id": str(uuid4()),
        "case_id": "persist-task-v1",
        "apk_path": str(tmp_path / "app.apk"),
        "expected_apk_sha256": "a" * 64,
        "package": "ai.mobileqa.demo",
        "activity": "ai.mobileqa.demo/.MainActivity",
        "serial": "emulator-5554",
        "profile_path": str(tmp_path / "profile.toml"),
        "output_root": str(tmp_path / "out"),
    }


def result_data():
    return {
        "version": 1,
        "attempt_id": str(uuid4()),
        "case_id": "persist-task-v1",
        "outcome": "blocked",
        "reason_code": "prerequisite_unavailable",
        "expected_behavior": "Persists.",
        "observed_behavior": "Unavailable.",
        "started_at": "2026-09-12T01:00:00Z",
        "ended_at": "2026-09-12T01:01:00Z",
        "requested_build_sha256": "a" * 64,
        "observed_build_sha256": None,
        "device_inventory": {},
        "model_profile_sha256": "b" * 64,
        "reset": "not_started",
        "phase_ms": {},
        "artifacts": [],
        "usage": [],
    }


@pytest.mark.parametrize(
    "field,value",
    [
        ("version", True),
        ("version", "1"),
        ("version", 2),
        ("serial", "emulator-5556"),
        ("package", "customer.app"),
        ("expected_apk_sha256", "x" * 64),
        ("case_id", "other"),
        ("extra", "ignored?"),
    ],
)
def test_request_rejects_before_device(tmp_path, field, value):
    data = request_data(tmp_path) | {field: value}
    with pytest.raises((ValueError, QualificationError)):
        parse_request(json.dumps(data))


def test_request_and_result_roundtrip(tmp_path):
    data = request_data(tmp_path)
    assert parse_request(json.dumps(data)).model_dump(mode="json", exclude_none=True) == data
    assert validate_result(json.dumps(result_data())).outcome.value == "blocked"


@pytest.mark.parametrize(
    "change",
    [
        {"ended_at": "2025-01-01T00:00:00Z"},
        {"outcome": "passed"},
        {"observed_build_sha256": "bad"},
        {"phase_ms": {"boot": -1}},
        {
            "usage": [
                {
                    "model": "demo",
                    "calls": 0,
                    "unknown_calls": 1,
                    "input_tokens": None,
                    "output_tokens": None,
                }
            ]
        },
    ],
)
def test_result_semantics(change):
    with pytest.raises((QualificationError, ValidationError)):
        validate_result(json.dumps(result_data() | change))


def test_private_evidence_is_immutable_and_contained(tmp_path):
    evidence = Evidence(tmp_path / "attempt")
    evidence.write("sample.xml", b"<hierarchy/>", "application/xml")
    assert evidence.artifacts[0]["sha256"] == sha256(tmp_path / "attempt/sample.xml")
    evidence.missing("video.mp4", "unsupported")
    evidence.event("preflight", "ready")
    payload = result_data() | {"artifacts": evidence.artifacts}
    evidence.finalize(payload)
    assert (evidence.directory / "report.md").exists()
    with pytest.raises(FileExistsError):
        evidence.finalize(payload)
    for name in ("../escape", "/absolute", "a\\b"):
        with pytest.raises(QualificationError):
            safe_path(evidence.directory, name)
    (evidence.directory / "link").symlink_to(tmp_path / "outside")
    with pytest.raises(QualificationError):
        safe_path(evidence.directory, "link")
    with pytest.raises(QualificationError):
        evidence.size()


def test_budget_and_atomic_replacement(tmp_path):
    evidence = Evidence(tmp_path / "small", limit=1)
    with pytest.raises(QualificationError):
        evidence.write("too-big", b"xx", "text/plain")
    path = tmp_path / "checkpoint.json"
    atomic_json(path, {"state": 1})
    atomic_json(path, {"state": 2}, replace=True)
    assert json.loads(path.read_text()) == {"state": 2}
    assert not path.with_name("checkpoint.json.pending").exists()


def test_profile_validation(tmp_path):
    path = tmp_path / "profile.toml"
    path.write_text(
        f'sdk_root="{tmp_path}/sdk"\nstate_root="{tmp_path}/state"\ntoolchain="{tmp_path}/lock.json"\n[model_ref]\nkey="demo"\nrevision=1\n'
    )
    assert Profile.load(path).model_ref.key == "demo"
    path.write_text(path.read_text() + "max_steps=true\n")
    with pytest.raises(QualificationError):
        Profile.load(path)


def test_old_adb_only_profile_sentinel_is_model_free(tmp_path):
    path = tmp_path / "profile.toml"
    path.write_text(
        f'sdk_root="{tmp_path}/sdk"\nstate_root="{tmp_path}/state"\ntoolchain="{tmp_path}/lock.json"\nmodel="no-model-adb-demo"\n'
    )
    assert Profile.load(path).model_ref is None
    path.write_text(path.read_text().replace("no-model-adb-demo", "gpt-4.1"))
    with pytest.raises(QualificationError, match="legacy_model_profile_use_model_ref"):
        Profile.load(path)


def test_doctor_fails_without_boot_on_unprepared_host(tmp_path):
    path = tmp_path / "profile.toml"
    path.write_text(
        f'sdk_root="{tmp_path}/sdk"\nstate_root="{tmp_path}/state"\ntoolchain="{tmp_path}/lock.json"\n[model_ref]\nkey="demo"\nrevision=1\n'
    )
    proc = subprocess.run(
        [sys.executable, "-m", "mobile_qa_worker.cli", "device-doctor", "--profile", str(path)],
        capture_output=True,
    )
    assert proc.returncode == 2
    assert proc.stderr
    assert not (tmp_path / "state").exists()
