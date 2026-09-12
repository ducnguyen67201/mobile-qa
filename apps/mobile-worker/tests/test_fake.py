"""Python half of the shared fixture contract and local CLI checks.

No device SDK is needed. These tests prove deterministic simulation, strict wire
validation and useful exit behavior; they do not qualify the real mobile runner.
"""

import json
import subprocess
import sys
from pathlib import Path

import pytest
from pydantic import ValidationError

from mobile_qa_worker.fake import execute, parse_request
from mobile_qa_worker.generated.models import ContractProbe

ROOT = Path(__file__).resolve().parents[3]
FIXTURES = ROOT / "contracts/fixtures"


@pytest.mark.parametrize(
    ("kind", "outcome"), [("pass", "passed"), ("fail", "failed"), ("blocked", "blocked")]
)
def test_fake_is_deterministic(kind, outcome):
    request = parse_request((FIXTURES / f"{kind}.json").read_text())
    first = execute(request).model_dump(mode="json")
    assert first == execute(request).model_dump(mode="json")
    assert first["outcome"] == outcome
    assert first["run_id"] == str(request.run_id)


@pytest.mark.parametrize(
    ("key", "bad"),
    [
        ("version", 2),
        ("version", "1"),
        ("version", True),
        ("version", None),
        ("run_id", "bad"),
        ("scenario", {"kind": "unknown"}),
        ("scenario", {"kind": "pass", "extra": 1}),
    ],
)
def test_invalid_request_rejected(key, bad):
    raw = json.loads((FIXTURES / "pass.json").read_text())
    raw[key] = bad
    with pytest.raises((ValidationError, ValueError)):
        parse_request(json.dumps(raw))


def test_missing_version_rejected():
    raw = json.loads((FIXTURES / "pass.json").read_text())
    del raw["version"]
    with pytest.raises(ValidationError):
        parse_request(json.dumps(raw))


def test_probe_preserves_wire_contract():
    raw = (FIXTURES / "probe.json").read_text()
    probe = ContractProbe.model_validate_json(raw, strict=True)
    assert probe.counter == "9007199254740993"
    assert json.loads(probe.model_dump_json(exclude_unset=True)) == json.loads(raw)
    optional = json.loads(raw) | {"optional_note": None}
    assert (
        ContractProbe.model_validate_json(json.dumps(optional), strict=True).optional_note is None
    )


@pytest.mark.parametrize(
    ("key", "bad"),
    [
        ("observed_at", "yesterday"),
        ("run_id", "bad"),
        ("counter", 9007199254740993),
        ("counter", "01"),
        ("counter", "-1"),
        ("counter", ""),
    ],
)
def test_probe_rejects_invalid_boundary(key, bad):
    raw = json.loads((FIXTURES / "probe.json").read_text())
    raw[key] = bad
    with pytest.raises(ValidationError):
        ContractProbe.model_validate_json(json.dumps(raw), strict=True)


def test_nullable_is_required():
    raw = json.loads((FIXTURES / "probe.json").read_text())
    del raw["nullable_note"]
    with pytest.raises(ValidationError):
        ContractProbe.model_validate_json(json.dumps(raw), strict=True)


@pytest.mark.parametrize("kind", ["pass", "fail", "blocked"])
def test_cli_json(kind):
    result = subprocess.run(
        [sys.executable, "-m", "mobile_qa_worker.cli", "fake", str(FIXTURES / f"{kind}.json")],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 0, result.stderr
    assert json.loads(result.stdout)["version"] == 1


@pytest.mark.parametrize("contents", ["not json", "{}", '{"version":2}'])
def test_cli_invalid_nonzero(tmp_path, contents):
    fixture = tmp_path / "invalid.json"
    fixture.write_text(contents)
    result = subprocess.run(
        [sys.executable, "-m", "mobile_qa_worker.cli", "fake", str(fixture)],
        capture_output=True,
        text=True,
        check=False,
    )
    assert result.returncode == 2
    assert not result.stdout
    assert "Input or setup failed" in result.stderr


def test_fake_does_not_import_sdk():
    assert not any(name.startswith("minitap") for name in sys.modules)
