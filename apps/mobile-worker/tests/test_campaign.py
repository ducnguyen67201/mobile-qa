import importlib.util
import json
import threading
import urllib.request
from pathlib import Path

import pytest
from test_qualification import request_data, result_data
from test_verifier import png, xml

from mobile_qa_worker.qualification import campaign, runner
from mobile_qa_worker.qualification.config import QualificationError, parse_request
from mobile_qa_worker.qualification.evidence import validate_result
from mobile_qa_worker.qualification.verifier import Observation


def test_campaign_schedule_and_accounting():
    assert len(campaign.schedule()) == 9
    assert [x[2] for x in campaign.schedule()].count("failed") == 3
    result = validate_result(json.dumps(result_data() | {"reset": "verified_clean"}))
    assert campaign.accepted(result, "blocked")
    assert not campaign.accepted(result, "passed")
    estimate = campaign.cost_estimate(3600, "1.5", [result], None, None)
    assert estimate["host_usd"] == "1.5"
    assert campaign.cost_estimate(10, None, [], None, None)["host_usd"] is None
    with pytest.raises(QualificationError):
        campaign.cost_estimate(10, "-1", [], None, None)


@pytest.mark.parametrize(
    "scenario,expected",
    [
        ("good", "passed"),
        ("broken", "failed"),
        ("blocked", "blocked"),
        ("cancel", "inconclusive"),
        ("cleanup", "passed"),
        ("sdk_cleanup", "inconclusive"),
    ],
)
def test_full_supervisor_with_injected_device(tmp_path, monkeypatch, scenario, expected):
    data = request_data(tmp_path)
    Path(data["apk_path"]).write_bytes(b"fixture")
    Path(data["profile_path"]).write_text(
        f'sdk_root="{tmp_path}/sdk"\nstate_root="{tmp_path}/state"\ntoolchain="{tmp_path}/lock.json"\n[model_ref]\nkey="demo"\nrevision=1\n'
    )
    monkeypatch.setattr(runner, "doctor", lambda p: {"image": "test"})
    monkeypatch.setattr(runner, "fixture_ready", lambda: None)

    class FakeDevice:
        def __init__(self, p, e):
            self.e = e
            self.inventory = {"serial": "emulator-5554"}
            self.boots = 0

        def boot(self, timeout=None):
            self.boots += 1
            if scenario == "sdk_cleanup":
                assert self.boots == 1, "Do not reset while SDK termination is uncertain"
            if scenario == "cleanup" and self.boots > 1:
                raise QualificationError("reset_failed")

        def install(self, *args):
            return "a" * 64

        def launch(self):
            pass

        def adb(self, *args, **kwargs):
            return b""

        def stop(self):
            pass

        def discard(self):
            pass

        def start_recording(self):
            pass

        def collect_diagnostics(self):
            pass

        def capture(self, name, task):
            present = name == "created" or (name == "reopened" and scenario != "broken")
            unavailable = scenario == "blocked"
            self.e.write(
                name + ".xml",
                xml(task if present else "", unavailable=unavailable),
                "application/xml",
            )
            self.e.write(name + ".png", png(), "image/png")
            return Observation(not unavailable, unavailable, present)

    monkeypatch.setattr(runner, "Device", FakeDevice)

    def sdk(*args):
        if scenario == "sdk_cleanup":
            raise QualificationError("sdk_cleanup_unconfirmed")
        if scenario == "cancel":
            raise QualificationError("cancelled")
        return []

    monkeypatch.setattr(runner, "run_sdk", sdk)
    result = runner.run_attempt(parse_request(json.dumps(data)))
    assert result.model_dump(mode="json")["outcome"] == expected
    assert result.model_dump(mode="json")["reset"] == (
        "quarantined" if scenario in ("cleanup", "sdk_cleanup") else "verified_clean"
    )
    assert (Path(data["output_root"]) / data["attempt_id"] / "result.json").exists()
    if scenario in ("cleanup", "sdk_cleanup"):
        assert (tmp_path / "state/dirty.json").exists()
    else:
        assert not (tmp_path / "state/dirty.json").exists()


def test_recovery_refuses_same_boot(tmp_path, monkeypatch):
    from test_device import profile

    p = profile(tmp_path)
    p.state_root.mkdir()
    (p.state_root / "dirty.json").write_text(json.dumps({"boot_id": "same"}))
    monkeypatch.setattr(runner, "boot_id", lambda: "same")
    with pytest.raises(QualificationError, match="reboot_required"):
        runner.recover(p)


def test_fixture_http_actual_server():
    path = runner.ROOT / "infra/device-host/fixture_backend.py"
    spec = importlib.util.spec_from_file_location("qualification_backend", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    with module.server("ready", port=0) as server:
        thread = threading.Thread(target=server.serve_forever)
        thread.start()
        try:
            with urllib.request.urlopen(
                f"http://127.0.0.1:{server.server_port}/session"
            ) as response:
                assert response.status == 200
        finally:
            server.shutdown()
            thread.join()


def test_campaign_oracle_never_reaches_runner(tmp_path, monkeypatch):
    from contextlib import nullcontext

    monkeypatch.setattr(campaign, "backend", lambda mode: nullcontext())
    (tmp_path / "good.apk").write_bytes(b"good")
    (tmp_path / "broken.apk").write_bytes(b"bad")
    config = tmp_path / "campaign.toml"
    config.write_text(
        f'good_apk="{tmp_path}/good.apk"\nbroken_apk="{tmp_path}/broken.apk"\nprofile="{tmp_path}/profile.toml"\noutput_root="{tmp_path}/out"\nmax_seconds=100\nbudget_usd="10"\n'
    )
    calls = []

    def run(request, interrupt_after=None):
        assert "expected_outcome" not in request.model_dump()
        calls.append(request)
        # One quarantine must stop, retain the first result, and reject campaign.
        return validate_result(json.dumps(result_data() | {"reset": "quarantined"}))

    assert campaign.qualify(config, run) == 1
    assert len(calls) == 1
    assert (
        json.loads(next((tmp_path / "out").glob("*/campaign.json")).read_text())["status"]
        == "rejected"
    )
