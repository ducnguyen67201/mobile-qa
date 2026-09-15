"""Host portability and real-device entry boundaries, without booting in ordinary CI."""

import argparse
import json
import socket
import zipfile
from dataclasses import replace
from uuid import uuid4

import pytest
from test_device import profile
from test_qualification import result_data

from mobile_qa_worker.device import android as device
from mobile_qa_worker.qualification import host, local
from mobile_qa_worker.qualification.config import Profile, QualificationError
from mobile_qa_worker.qualification.device import Device
from mobile_qa_worker.qualification.evidence import Evidence, sha256, validate_result


@pytest.mark.parametrize(
    "system,machine,abi",
    [
        ("Darwin", "arm64", "arm64-v8a"),
        ("Darwin", "x86_64", "x86_64"),
        ("Linux", "x86_64", "x86_64"),
        ("Linux", "aarch64", None),
        ("Windows", "AMD64", None),
    ],
)
def test_native_host_support(monkeypatch, system, machine, abi):
    monkeypatch.setattr(host.platform, "system", lambda: system)
    monkeypatch.setattr(host.platform, "machine", lambda: machine)
    if abi is None:
        with pytest.raises(QualificationError, match="unsupported_device_host"):
            host.native_abi()
    else:
        assert host.native_abi() == abi


def test_macos_boot_identity_changes_and_fails_closed(monkeypatch):
    monkeypatch.setattr(host.platform, "system", lambda: "Darwin")
    first, second = str(uuid4()), str(uuid4())
    for value in (first, second):
        monkeypatch.setattr(
            host, "command", lambda args, value=value: (value.upper() + "\n").encode()
        )
        assert host.boot_id() == value
    monkeypatch.setattr(host, "command", lambda args: b"non-linux-test")
    with pytest.raises(QualificationError, match="host_boot_identity"):
        host.boot_id()


def test_mac_doctor_requires_matching_pinned_image_and_acceleration(tmp_path, monkeypatch):
    monkeypatch.setattr(host.platform, "system", lambda: "Darwin")
    monkeypatch.setattr(host.platform, "machine", lambda: "arm64")
    p = replace(profile(tmp_path), system_image="system-images;android-35;google_apis;arm64-v8a")
    image = p.sdk_root / p.system_image.replace(";", "/")
    image.mkdir(parents=True)
    (image / "source.properties").write_text("Pkg.Revision=9\n")
    p.toolchain.write_text(
        json.dumps(
            {
                "packages": [
                    {"path": p.system_image, "revision": "9"},
                    {"path": "system-images;android-35;google_apis;x86_64", "revision": "9"},
                ]
            }
        )
    )
    monkeypatch.setattr(
        device.os, "access", lambda *args: pytest.fail("Mac must not check /dev/kvm")
    )
    monkeypatch.setattr(
        device,
        "command",
        lambda *args: b"accel:\n0\nHypervisor.Framework OS X Version 15.7\naccel\n",
    )
    monkeypatch.setattr(device.shutil, "disk_usage", lambda path: argparse.Namespace(free=10**10))
    assert device.doctor(p)["accelerator"] == "Hypervisor.Framework"
    with pytest.raises(QualificationError, match="architecture_mismatch"):
        device.doctor(replace(p, system_image="system-images;android-35;google_apis;x86_64"))
    monkeypatch.setattr(
        device, "command", lambda *args: b"accel:\n0\nKVM is installed and usable.\naccel\n"
    )
    with pytest.raises(QualificationError, match="hardware_acceleration"):
        device.doctor(p)
    monkeypatch.setattr(
        device,
        "command",
        lambda *args: b"accel:\n0\nHypervisor.Framework OS X Version 15.7\naccel\n",
    )
    (image / "source.properties").write_text("Pkg.Revision=10\n")
    with pytest.raises(QualificationError, match="revision_mismatch"):
        device.doctor(p)


def test_arm_apk_allowed_only_with_arm_profile(tmp_path, monkeypatch):
    p = replace(profile(tmp_path), system_image="system-images;android-35;google_apis;arm64-v8a")
    d = Device(p, Evidence(tmp_path / "attempt"))
    apk = tmp_path / "app.apk"
    with zipfile.ZipFile(apk, "w") as archive:
        archive.writestr("AndroidManifest.xml", b"manifest")
        archive.writestr("lib/arm64-v8a/libdemo.so", b"native")
    monkeypatch.setattr(
        device, "command", lambda *args: b"package: name='ai.mobileqa.demo'\nsdkVersion:'26'"
    )
    monkeypatch.setattr(
        d,
        "adb",
        lambda *args, **kwargs: b"ai.mobileqa.demo/.MainActivity"
        if "resolve-activity" in args
        else b"Success",
    )
    assert d.install(apk, sha256(apk)) == sha256(apk)


def test_profile_roundtrip_and_strict_host_fields(tmp_path):
    path = tmp_path / "profile.toml"
    p = replace(
        profile(tmp_path),
        system_image="system-images;android-35;google_apis;arm64-v8a",
        headless=False,
    )
    local.write_profile(path, p)
    assert Profile.load(path) == p
    with pytest.raises(FileExistsError):
        local.write_profile(path, p)
    text = path.read_text()
    path.write_text(text.replace("headless = false", 'headless = "false"'))
    with pytest.raises(QualificationError, match="headless"):
        Profile.load(path)


@pytest.mark.parametrize("agent,model", [(True, None), (False, "some-model")])
def test_agent_requires_explicit_choice_before_any_io(agent, model):
    with pytest.raises(QualificationError, match="agent_requires"):
        local.run_local(argparse.Namespace(agent=agent, model=model))


def test_demo_navigation_uses_current_control_bounds():
    task = "qa-" + str(uuid4())
    calls = []

    class Demo:
        profile = argparse.Namespace(assertion_seconds=15)

        def snapshot(self, _task):
            xml = (
                b'<hierarchy><node package="ai.mobileqa.demo" '
                b'resource-id="ai.mobileqa.demo:id/task_input" enabled="true" '
                b'focused="true" text="TASK" bounds="[0,100][200,200]"/>'
                b'<node package="ai.mobileqa.demo" '
                b'resource-id="ai.mobileqa.demo:id/save_task" enabled="true" '
                b'bounds="[0,200][200,300]"/></hierarchy>'
            )
            return None, xml.replace(b"TASK", task.encode()), b""

        def adb(self, *args):
            calls.append(args)

    local.navigate_demo(Demo(), task)
    assert calls == [
        ("shell", "input", "tap", "100", "150"),
        ("shell", "input", "text", task),
        ("shell", "input", "keyevent", "KEYCODE_BACK"),
        ("shell", "input", "tap", "100", "250"),
    ]


def test_local_broken_success_means_expected_failure_and_clean_reset(tmp_path, monkeypatch):
    from contextlib import nullcontext

    p = profile(tmp_path)
    pp = tmp_path / "profile.toml"
    local.write_profile(pp, p)
    apk = tmp_path / "apps/qa-demo-android/app/build/outputs/apk/broken/debug/app-broken-debug.apk"
    apk.parent.mkdir(parents=True)
    apk.write_bytes(b"demo")
    monkeypatch.setattr(local, "ROOT", tmp_path)
    monkeypatch.setattr(local, "doctor", lambda profile: {})
    monkeypatch.setattr(local, "backend", lambda mode: nullcontext())
    calls = []

    def attempt(request, *, driver):
        calls.append(driver)
        # A blocked result is never accepted as successful broken-app detection.
        return validate_result(json.dumps(result_data() | {"reset": "verified_clean"}))

    monkeypatch.setattr(local, "run_attempt", attempt)
    args = argparse.Namespace(
        profile=pp, agent=False, model=None, headless=False, scenario="broken"
    )
    assert local.run_local(args) == 1
    assert calls == ["adb-demo"]
    assert (
        json.loads((tmp_path / ".private/artifacts/local-device/latest.json").read_text())[
            "matched_expectation"
        ]
        is False
    )


@pytest.mark.parametrize("driver,allowed", [("adb-demo", True), ("minitap", False)])
def test_explicit_local_recovery_requires_matching_adb_receipt(
    tmp_path, monkeypatch, driver, allowed
):
    from mobile_qa_worker.qualification import runner

    p = profile(tmp_path)
    pp = tmp_path / "profile.toml"
    local.write_profile(pp, p)
    p.state_root.mkdir()
    payload = result_data() | {
        "reset": "quarantined",
        "device_inventory": {"navigation_driver": driver},
        "model_profile_sha256": sha256(pp),
    }
    receipt = tmp_path / "result.json"
    receipt.write_text(json.dumps(payload))
    marker = p.state_root / "dirty.json"
    marker.write_text(json.dumps({"boot_id": "same", "attempt_id": payload["attempt_id"]}))
    monkeypatch.setattr(runner, "boot_id", lambda: "same")

    class StoppedDevice:
        def __init__(self, *args):
            pass

        def stop(self):
            pass

        def discard(self):
            pass

    monkeypatch.setattr(runner, "Device", StoppedDevice)
    if allowed:
        runner.recover(p, local_result=receipt, profile_path=pp)
        assert not marker.exists()
    else:
        with pytest.raises(QualificationError, match="local_recovery_evidence_mismatch"):
            runner.recover(p, local_result=receipt, profile_path=pp)
        assert marker.exists()


def test_mac_boot_uses_effective_locale_and_unattended_flags(tmp_path, monkeypatch):
    from test_verifier import png

    p = replace(
        profile(tmp_path),
        system_image="system-images;android-35;google_apis;arm64-v8a",
        headless=False,
    )
    p.state_root.mkdir()
    d = Device(p, Evidence(tmp_path / "attempt"))
    monkeypatch.setattr(device.platform, "system", lambda: "Darwin")
    monkeypatch.setattr(device, "assert_ports_available", lambda: None)

    def create(args, *rest):
        assert p.system_image in args
        folder = d.current / "phone.avd"
        folder.mkdir()
        (folder / "config.ini").write_text("hw.ramSize=2048\n")

    monkeypatch.setattr(device, "command", create)
    launched = []

    def launch(args, **kwargs):
        launched.extend(args)
        return argparse.Namespace(poll=lambda: None)

    monkeypatch.setattr(device.subprocess, "Popen", launch)
    props = {
        "sys.boot_completed": "1",
        "ro.build.version.sdk": "35",
        "ro.product.cpu.abi": "arm64-v8a",
        "persist.sys.locale": "",
        "ro.product.locale": "en-US",
        "persist.sys.timezone": "Etc/UTC",
        "ro.build.fingerprint": "test-image",
    }

    def adb(*args):
        if args[:2] == ("exec-out", "screencap"):
            return png()
        if args[:2] == ("shell", "getprop"):
            return props[args[2]].encode()
        return b"420"

    monkeypatch.setattr(d, "adb", adb)
    d.boot()
    assert d.inventory["locale"] == "en-US"
    assert "-no-window" not in launched and "-no-metrics" in launched
    assert launched[launched.index("-crash-report-mode") + 1] == "disabled"
    assert launched[launched.index("-timezone") + 1] == "Etc/UTC"
    assert launched[launched.index("-gpu") + 1] == "auto"


def test_stop_waits_for_listener_release(tmp_path, monkeypatch):
    attempts = []

    def available():
        attempts.append(True)
        if len(attempts) < 3:
            raise OSError("listener closing")

    monkeypatch.setattr(device, "assert_ports_available", available)
    monkeypatch.setattr(device.time, "sleep", lambda seconds: None)
    d = Device(profile(tmp_path), Evidence(tmp_path / "attempt"))
    d.stop()
    assert len(attempts) == 3


def test_port_probe_cannot_borrow_an_active_listener():
    with socket.socket() as owner:
        owner.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        owner.bind(("127.0.0.1", 0))
        owner.listen(1)
        port = owner.getsockname()[1]
        with pytest.raises(OSError):
            device.assert_ports_available((port,))
    device.assert_ports_available((port,))
