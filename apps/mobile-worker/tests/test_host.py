"""Offline ownership and isolation tests. No Android, model, Doppler or cloud calls."""

import socket
from datetime import UTC, datetime
from uuid import uuid4

import pytest

from mobile_qa_worker.generated.models import SlotDeviceRequest, SlotDeviceResponse
from mobile_qa_worker.host.config import SlotConfig
from mobile_qa_worker.host.ipc import DeviceServer, read_frame, request
from mobile_qa_worker.host.policy import WarmPolicy
from mobile_qa_worker.host.qualification import MEASUREMENTS, REQUIRED_SCENARIOS, summarize
from mobile_qa_worker.host.slot import Slot
from mobile_qa_worker.qualification.config import QualificationError


class FakeDevice:
    def __init__(self):
        self.running = False
        self.dirty = False
        self.boots = 0
        self.stops = 0
        self.fail_boot = False
        self.fail_stop = False
        self.package = None

    def boot(self, timeout=None):
        self.boots += 1
        self.running = self.dirty = True
        if self.fail_boot:
            raise QualificationError("fake_boot_failed")

    def stop(self):
        self.stops += 1
        if self.fail_stop:
            raise QualificationError("fake_stop_failed")
        self.running = False

    def discard(self, *, recovery=False):
        assert not self.running
        self.dirty = False

    def assert_clean(self):
        assert not self.running and not self.dirty

    def bind(self, package, component):
        self.package = package


def owned(tmp_path):
    root = tmp_path / str(uuid4())
    root.mkdir()
    device = FakeDevice()
    return Slot(uuid4(), root, device), device


def test_cold_slot_never_reuses_prior_capability_or_guest(tmp_path):
    slot, device = owned(tmp_path)
    token = slot.authorize()
    slot.assign(token, "com.example.a", "com.example.a/.Main", 10)
    assert slot.state == "leased" and device.running
    first_boot = slot.boot_id
    slot.cleanup()
    slot.revoke()
    assert slot.drained()
    with pytest.raises(QualificationError, match="capability_expired"):
        slot.verify(token)
    next_token = slot.authorize()
    slot.assign(next_token, "com.example.b", "com.example.b/.Main", 10)
    assert slot.boot_id != first_boot and device.boots == 2


def test_warm_owner_handoff_avoids_second_boot_but_always_discards(tmp_path):
    slot, device = owned(tmp_path)
    slot.prepare()
    boot = slot.boot_id
    assert slot.pristine and device.boots == 1
    token = slot.authorize()
    slot.assign(token, "com.example.a", "com.example.a/.Main", None)
    assert not slot.pristine and slot.boot_id == boot and device.boots == 1
    slot.cleanup()
    slot.revoke()
    slot.prepare()
    assert device.boots == 2 and slot.boot_id != boot


def test_failed_boot_cleans_owned_partial_state(tmp_path):
    slot, device = owned(tmp_path)
    device.fail_boot = True
    with pytest.raises(QualificationError, match="fake_boot_failed"):
        slot.prepare()
    assert slot.drained() and device.stops == 1


def test_uncertain_stop_keeps_recovery_journal_and_never_reports_drained(tmp_path):
    slot, device = owned(tmp_path)
    slot.prepare()
    device.fail_stop = True
    with pytest.raises(QualificationError, match="fake_stop_failed"):
        slot.cleanup()
    assert slot.state == "quarantined" and slot.journal.exists() and not slot.drained()
    restarted = Slot(slot.id, slot.root, FakeDevice())
    assert restarted.state == "quarantined"
    with pytest.raises(QualificationError, match="not_available"):
        restarted.authorize()


def test_cleanup_is_slot_scoped(tmp_path):
    one, first = owned(tmp_path)
    two, second = owned(tmp_path)
    one.prepare()
    two.prepare()
    one.cleanup()
    assert not first.running and second.running and two.pristine


def test_duplicate_boot_is_not_a_second_lease(tmp_path):
    slot, device = owned(tmp_path)
    token = slot.authorize()
    slot.assign(token, "com.example.a", "com.example.a/.Main", 10)
    with pytest.raises(QualificationError, match="already_assigned"):
        slot.assign(token, "com.example.b", "com.example.b/.Main", 10)
    assert device.package == "com.example.a"


def test_slot_ports_have_no_shared_adb_server():
    configurations = [SlotConfig(index, uuid4(), uuid4()) for index in range(8)]
    assert len({c.adb_port for c in configurations}) == 8
    assert len({p for c in configurations for p in (c.console_port, c.console_port + 1)}) == 16
    assert all(c.adb_port != 5037 for c in configurations)


def test_warm_policy_requires_qualification_and_drain_wins():
    policy = WarmPolicy(scheduled=True)
    now = datetime(2026, 9, 22, 12, tzinfo=UTC)
    assert policy.wanted(now, None, target=1, qualified=True, draining=False)
    for kwargs in (
        {"target": 0, "qualified": True, "draining": False},
        {"target": 1, "qualified": False, "draining": False},
        {"target": 1, "qualified": True, "draining": True},
    ):
        assert not policy.wanted(now, 0, **kwargs)
    assert not WarmPolicy().wanted(now, None, target=1, qualified=True, draining=False)
    assert WarmPolicy().wanted(now, 899, target=1, qualified=True, draining=False)
    assert not WarmPolicy().wanted(now, 900, target=1, qualified=True, draining=False)


def test_warm_policy_dst_repeated_hour_and_overnight():
    policy = WarmPolicy("America/Toronto", "01:00", "02:00", True)
    for hour in (5, 6):
        assert policy.wanted(
            datetime(2026, 11, 1, hour, 30, tzinfo=UTC),
            None,
            target=1,
            qualified=True,
            draining=False,
        )
    overnight = WarmPolicy("UTC", "22:00", "06:00", True)
    assert overnight.wanted(
        datetime(2026, 9, 22, 23, tzinfo=UTC), None, target=1, qualified=True, draining=False
    )
    with pytest.raises(QualificationError, match="timezone"):
        policy.wanted(datetime(2026, 1, 1), None, target=1, qualified=True, draining=False)


def test_ipc_rejects_oversize_before_reading_payload():
    left, right = socket.socketpair()
    try:
        left.sendall((65537).to_bytes(4, "big"))
        with pytest.raises(QualificationError, match="too_large"):
            read_frame(right, 65536)
    finally:
        left.close()
        right.close()


def test_private_socket_validates_capability_and_has_no_error_payload():
    import tempfile
    from pathlib import Path

    # macOS pytest roots exceed sockaddr_un's path limit; use a private short root.
    with tempfile.TemporaryDirectory(prefix="mqh-", dir="/tmp") as directory:
        path = Path(directory).resolve() / "slot.sock"
        token = "a" * 40

        def dispatch(payload):
            if payload.token != token:
                raise QualificationError("private information must not escape")
            return SlotDeviceResponse.model_validate({"ok": True})

        server = DeviceServer(path, dispatch)
        server.start()
        try:
            good = SlotDeviceRequest.model_validate({"token": token, "operation": "stop"})
            assert request(path, good).ok
            bad = SlotDeviceRequest.model_validate({"token": "expired", "operation": "stop"})
            with pytest.raises(QualificationError, match="^slot_operation_failed$"):
                request(path, bad)
            assert path.stat().st_mode & 0o777 == 0o600
        finally:
            server.close()
        assert not path.exists()


def campaign():
    return {
        "instance_type": "fixture-only",
        "region": "fixture",
        "image_id": "fixture",
        "toolchain_sha256": "a" * 64,
        "profile_sha256": "b" * 64,
        "apk_sha256": "c" * 64,
        "rendering": "software",
        "network_isolation_evidence": "fixture-only",
        "slots": 2,
        "samples": [
            dict(
                scenario=name,
                passed=True,
                oom=False,
                process_leak=False,
                reset_contamination=False,
                **{m: 1.0 for m in MEASUREMENTS},
            )
            for name in sorted(REQUIRED_SCENARIOS)
        ],
    }


def test_capacity_report_records_evidence_without_granting_qualification():
    result = summarize(campaign())
    assert result["eligible_for_operator_review"] is True
    assert result["approved_slots"] == 0 and result["candidate_slots"] == 2
    data = campaign()
    data["samples"][0]["oom"] = True
    result = summarize(data)
    assert result["failures"] == 1 and result["eligible_for_operator_review"] is False


def test_campaign_missing_scenario_and_nonfinite_metrics_fail_closed():
    data = campaign()
    data["samples"].pop()
    assert summarize(data)["eligible_for_operator_review"] is False
    data["samples"][0]["rss_mib"] = float("nan")
    with pytest.raises(QualificationError, match="measurement_invalid"):
        summarize(data)


def test_profile_copy_keeps_model_configuration_and_isolates_state(tmp_path):
    from mobile_qa_worker.host.supervisor import slot_profile

    source = tmp_path / "source.toml"
    root = tmp_path / "slot-1"
    root.mkdir()
    source.write_text(
        f'sdk_root = "{tmp_path / "sdk"}"\n'
        f'state_root = "{tmp_path / "original"}"\n'
        f'toolchain = "{tmp_path / "lock.json"}"\n'
        'doppler_project = "operator-project"\n'
    )
    path, profile = slot_profile(source, root)
    assert profile.state_root == root / "device"
    assert profile.doppler_project == "operator-project"
    assert path.stat().st_mode & 0o777 == 0o600
    assert "original" in source.read_text()


def test_supervisor_reports_cold_slot_ready_for_claim_and_fences_reply(tmp_path):
    from types import SimpleNamespace

    from mobile_qa_worker.generated.models import HostState, HostStatus
    from mobile_qa_worker.host.supervisor import heartbeat

    boot = uuid4()
    status = HostStatus.model_validate(
        {
            "host_id": str(uuid4()),
            "pool_id": str(uuid4()),
            "boot_id": str(boot),
            "generation": 1,
            "state": "ready",
            "heartbeat_seconds": 20,
            "policy": {
                "enabled": True,
                "idle_seconds": 900,
                "warm_target": 0,
                "timezone": "UTC",
                "warm_windows": [],
            },
            "slots": [],
            "bindings": [],
        }
    )
    slot, _ = owned(tmp_path)
    payloads = []

    def respond(payload):
        payloads.append(payload)
        return status

    client = SimpleNamespace(heartbeat=respond)
    heartbeat(client, status, boot, [SimpleNamespace(slot=slot)])
    assert payloads[-1].slots[0].state.value == "idle"
    status.state = HostState.draining
    heartbeat(client, status, boot, [SimpleNamespace(slot=slot)])
    assert payloads[-1].slots[0].state.value == "offline"
    client.heartbeat = lambda _: status.model_copy(update={"generation": 2})
    with pytest.raises(QualificationError, match="boot_fenced"):
        heartbeat(client, status, boot, [SimpleNamespace(slot=slot)])


def test_proxy_does_not_launch_android_and_sends_generated_commands(tmp_path, monkeypatch):
    from test_device import profile

    from mobile_qa_worker.host import proxy
    from mobile_qa_worker.qualification.evidence import Evidence

    root = tmp_path / "worker"
    root.mkdir()
    monkeypatch.setenv("MOBILE_QA_SLOT_SOCKET", str(tmp_path / "device.sock"))
    monkeypatch.setenv("MOBILE_QA_SLOT_TOKEN", "private")
    monkeypatch.setenv("MOBILE_QA_SLOT_LEASE_ROOT", str(root))
    calls = []

    def send(path, payload):
        calls.append(payload)
        return SlotDeviceResponse.model_validate({"ok": True})

    monkeypatch.setattr(proxy, "request", send)
    device = proxy.SupervisedDevice(
        profile(tmp_path), Evidence(root / "evidence"), "com.example.app", "com.example.app/.Main"
    )
    device.boot(10)
    device.stop()
    device.discard()
    assert device.child is None and device.remote_commands
    assert [c.operation.value for c in calls] == ["boot", "stop", "discard"]
    assert calls[0].package == "com.example.app"


def test_escaped_emulator_group_blocks_host_drain(tmp_path, monkeypatch):
    import os

    from mobile_qa_worker.host.isolation import assert_emulators_stopped

    path = tmp_path / "cgroup.procs"
    path.write_text("4242\n")
    original = type(path).stat

    def owned_stat(self, **kwargs):
        values = list(original(self, **kwargs))
        if self == path:
            values[4] = 0
        return os.stat_result(values)

    monkeypatch.setattr(type(path), "stat", owned_stat)
    with pytest.raises(QualificationError, match="not_empty"):
        assert_emulators_stopped(path)
    path.write_text("")
    assert_emulators_stopped(path)
