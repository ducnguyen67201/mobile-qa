"""Offline adapter boundaries; these fixtures never qualify a physical device."""

import io
import zipfile
from dataclasses import replace
from uuid import uuid4
from xml.etree import ElementTree

import pytest
from PIL import Image
from test_device import profile
from test_execution import job

from mobile_qa_worker.device import android
from mobile_qa_worker.device.capture import sanitize
from mobile_qa_worker.execution.adapters import device_for
from mobile_qa_worker.execution.fake import png
from mobile_qa_worker.generated.models import ExecutionProfile
from mobile_qa_worker.qualification.config import QualificationError
from mobile_qa_worker.qualification.evidence import Evidence, sha256


def assignment(package="com.example.notes"):
    p = job().manifest.profile.model_dump(mode="json")
    p.update(
        adapter="android_direct_v1",
        package=package,
        driver="direct",
        model="",
        image="system-images;android-35;google_apis;x86_64",
    )
    p["execution_context"] = dict(
        schema_version=1,
        adapter_revision="android_direct_v1",
        verifier_revision="ui_v1",
        worker_runtime_revision="direct_v1",
        reset_policy_hash="a" * 64,
        qualified_profile_id=p["id"],
        package=package,
        launch_component=package + "/.MainActivity",
        image=p["image"],
        abi="x86_64",
        width=1080,
        height=1920,
        density=420,
        locale="en-US",
        timezone="Etc/UTC",
        state_scope="local_only",
        qualification_reference="fixture only",
        starting_checks=[
            dict(
                id="start",
                checkpoint_id="preflight",
                description="Empty input",
                method="ui_property_equals_v1",
                resource_id=package + ":id/input",
                text_filter="",
                property="text",
                expected="",
                ready_resource_id=package + ":id/input",
                prerequisite_check_ids=[],
                required=True,
                observation_seconds=1,
            )
        ],
        stages=dict(boot_seconds=10, install_seconds=10, start_seconds=10, cleanup_seconds=10),
    )
    return ExecutionProfile.model_validate(p)


@pytest.mark.parametrize("package", ["com.example.notes", "org.example.shopping"])
def test_install_resolves_exact_package_and_launcher_without_fixture_tunnel(
    tmp_path, monkeypatch, package
):
    a = assignment(package)
    d = device_for(a, profile(tmp_path), Evidence(tmp_path / "evidence"))
    apk = tmp_path / "app.apk"
    with zipfile.ZipFile(apk, "w") as z:
        z.writestr("AndroidManifest.xml", b"fixture")
    monkeypatch.setattr(
        android, "command", lambda *args: f"package: name='{package}'\nsdkVersion:'26'".encode()
    )
    calls = []

    def adb(*args, **kwargs):
        calls.append(args)
        return (package + "/.MainActivity").encode() if "resolve-activity" in args else b"Success"

    monkeypatch.setattr(d, "adb", adb)
    d.install(apk, sha256(apk))
    d.launch()
    assert calls[-1][-1] == package + "/.MainActivity"
    assert not any("reverse" in c or "run-as" in c for c in calls)
    monkeypatch.setattr(
        d,
        "adb",
        lambda *args, **kw: b"android/.ResolverActivity"
        if "resolve-activity" in args
        else b"Success",
    )
    with pytest.raises(QualificationError, match="launcher_mismatch"):
        d.install(apk, sha256(apk))


def test_context_or_host_mismatch_never_opens_device(tmp_path):
    a = assignment()
    a.execution_context.state_scope = "remote"
    with pytest.raises(QualificationError):
        device_for(a, profile(tmp_path), Evidence(tmp_path / "one"))
    a = assignment()
    with pytest.raises(QualificationError):
        device_for(a, replace(profile(tmp_path), system_image="wrong"), Evidence(tmp_path / "two"))


def test_failed_boot_preserves_prior_avd_until_explicit_recovery(tmp_path, monkeypatch):
    d = device_for(assignment(), profile(tmp_path), Evidence(tmp_path / "evidence"))
    d.current.mkdir(parents=True)
    retained = d.current / "keep.txt"
    retained.write_text("prior instance")
    monkeypatch.setattr(android, "assert_ports_available", lambda: None)
    with pytest.raises(QualificationError, match="dirty_avd_requires_recovery"):
        d.boot()
    d.stop()
    with pytest.raises(QualificationError, match="dirty_avd_requires_recovery"):
        d.discard()
    assert retained.read_text() == "prior instance"
    d.discard(recovery=True)
    assert not d.current.exists()


def test_partial_owned_boot_can_be_discarded(tmp_path, monkeypatch):
    host = profile(tmp_path)
    host.state_root.mkdir()
    d = device_for(assignment(), host, Evidence(tmp_path / "evidence"))
    monkeypatch.setattr(android, "assert_ports_available", lambda: None)

    def fail_create(*args):
        raise QualificationError("avd_creation_failed")

    monkeypatch.setattr(android, "command", fail_create)
    with pytest.raises(QualificationError, match="avd_creation_failed"):
        d.boot()
    assert d.current.exists()
    d.stop()
    d.discard()
    assert not d.current.exists()


def test_password_pixels_and_xml_are_masked_before_retention():
    xml = (
        b'<hierarchy><node package="com.example.notes" text="secret" '
        b'content-desc="secret" password="true" bounds="[0,0][100,100]"/>'
        b'<node package="other" text="foreign"/></hierarchy>'
    )
    tree, image = sanitize(xml, png(), "com.example.notes")
    assert b"secret" not in tree and b"foreign" not in tree
    assert Image.open(io.BytesIO(image)).getpixel((50, 50)) in ((0, 0, 0), (0, 0, 0, 255), 0)
    assert len(list(ElementTree.fromstring(tree).iter("node"))) == 2


def test_preflight_ack_is_attempt_and_instance_bound(tmp_path):
    import time

    from mobile_qa_worker.execution.journal import write
    from mobile_qa_worker.execution.lifecycle import await_start_ack

    j = job()
    nonce = str(uuid4())
    write(
        tmp_path / "preflight-ack.json",
        dict(attempt_id=str(j.attempt_id), instance_nonce="foreign", accepted=True),
    )
    with pytest.raises(QualificationError, match="mismatch"):
        await_start_ack(tmp_path, j, nonce, time.monotonic() + 1)
    write(
        tmp_path / "preflight-ack.json",
        dict(attempt_id=str(j.attempt_id), instance_nonce=nonce, accepted=True),
    )
    await_start_ack(tmp_path, j, nonce, time.monotonic() + 1)
