import subprocess
import sys
import zipfile
from pathlib import Path

import pytest

from mobile_qa_worker.device import android as device_module
from mobile_qa_worker.qualification.config import Profile, QualificationError
from mobile_qa_worker.qualification.device import Device, host_environment
from mobile_qa_worker.qualification.evidence import Evidence, sha256
from mobile_qa_worker.qualification.process import command, host_lock, stop_group


def profile(tmp_path):
    return Profile(
        tmp_path / "sdk",
        tmp_path / "state",
        None,
        tmp_path / "lock.json",
    )


def test_environment_excludes_server_secrets(tmp_path, monkeypatch):
    monkeypatch.setenv("DATABASE_URL", "not-for-emulator")
    monkeypatch.setenv("OPENAI_API_KEY", "not-for-emulator")
    env = host_environment(profile(tmp_path))
    assert "DATABASE_URL" not in env and "OPENAI_API_KEY" not in env
    assert env["ANDROID_AVD_HOME"] == str(tmp_path / "state/current")


def test_host_lock_and_symlinks(tmp_path):
    with host_lock(tmp_path / "state"):
        with pytest.raises(QualificationError, match="device_busy"):
            with host_lock(tmp_path / "state"):
                pass
    (tmp_path / "link").symlink_to(tmp_path / "state")
    with pytest.raises(QualificationError):
        with host_lock(tmp_path / "link"):
            pass


def test_command_and_timeout():
    assert command([sys.executable, "-c", 'print("ok")']).strip() == b"ok"
    with pytest.raises(QualificationError, match="command_failed"):
        command([sys.executable, "-c", "raise SystemExit(3)"])
    with pytest.raises(QualificationError, match="command_timeout"):
        command([sys.executable, "-c", "import time; time.sleep(60)"], 0.1)


def test_command_stops_noisy_process_before_it_finishes():
    with pytest.raises(QualificationError, match="command_output_too_large"):
        command(
            [sys.executable, "-c", "import os,time; os.write(1,b'x'*17000000); time.sleep(60)"], 5
        )


def test_stop_group_when_parent_exits(tmp_path):
    marker = tmp_path / "child"
    code = (
        "import subprocess,sys; "
        "p=subprocess.Popen([sys.executable,'-c','import time; time.sleep(60)']); "
        "open(sys.argv[1],'w').write(str(p.pid))"
    )
    parent = subprocess.Popen([sys.executable, "-c", code, str(marker)], start_new_session=True)
    parent.wait(timeout=5)
    stop_group(parent, grace=0.1)
    pid = int(marker.read_text())
    # Orphan may be a zombie until the OS reaps it, but must not execute.
    stat = Path(f"/proc/{pid}/stat")
    if stat.exists():
        assert stat.read_text().rsplit(")", 1)[1].split()[0] == "Z"
    elif sys.platform == "darwin":
        status = subprocess.run(
            ["ps", "-o", "stat=", "-p", str(pid)], capture_output=True, text=True
        ).stdout.strip()
        assert not status or status.startswith("Z")


def test_apk_metadata_and_checksum(tmp_path, monkeypatch):
    d = Device(profile(tmp_path), Evidence(tmp_path / "attempt"))
    apk = tmp_path / "app.apk"
    with zipfile.ZipFile(apk, "w") as z:
        z.writestr("AndroidManifest.xml", b"manifest")
    monkeypatch.setattr(
        device_module,
        "command",
        lambda *a, **k: b"package: name='ai.mobileqa.demo'\nsdkVersion:'26'",
    )
    monkeypatch.setattr(
        d,
        "adb",
        lambda *a, **k: b"ai.mobileqa.demo/.MainActivity"
        if "resolve-activity" in a
        else b"Success",
    )
    assert d.install(apk, sha256(apk)) == sha256(apk)
    with pytest.raises(QualificationError, match="checksum"):
        d.install(apk, "a" * 64)
    with zipfile.ZipFile(apk, "a") as z:
        z.writestr("lib/arm64-v8a/native.so", b"x")
    with pytest.raises(QualificationError, match="unsupported_abi"):
        d.install(apk, sha256(apk))


def test_discard_does_not_follow_link(tmp_path):
    p = profile(tmp_path)
    p.state_root.mkdir()
    d = Device(p, Evidence(tmp_path / "attempt"))
    other = tmp_path / "other"
    other.mkdir()
    (other / "keep").write_text("data")
    d.current.symlink_to(other)
    with pytest.raises(QualificationError):
        d.discard()
    assert (other / "keep").read_text() == "data"
