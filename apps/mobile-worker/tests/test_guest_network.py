"""Offline policy construction; never installs nft rules or runs a privileged helper."""

import importlib.util
import ipaddress
from pathlib import Path
from types import SimpleNamespace

import pytest

ROOT = Path(__file__).resolve().parents[3]
NETWORK = ROOT / "infra/device-host/network"
SPEC = importlib.util.spec_from_file_location("guest_network_policy", NETWORK / "policy.py")
assert SPEC is not None and SPEC.loader is not None
policy = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(policy)


def test_guest_denials_include_metadata_private_loopback_and_translation():
    protected = [
        "127.0.0.1",
        "10.0.2.2",
        "10.42.0.1",
        "172.31.0.1",
        "192.168.1.1",
        "169.254.169.254",
        "100.100.100.100",
        "0.0.0.0",
        "::1",
        "fd00:ec2::254",
        "fe80::1",
        "::ffff:169.254.169.254",
        "64:ff9b::a9fe:a9fe",
    ]
    blocked = [ipaddress.ip_network(n) for n in (*policy.IPV4_DENY, *policy.IPV6_DENY)]
    for literal in protected:
        address = ipaddress.ip_address(literal)
        assert any(address in network for network in blocked if network.version == address.version)
    for literal in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"]:
        address = ipaddress.ip_address(literal)
        assert not any(
            address in network for network in blocked if network.version == address.version
        )


def test_policy_limits_reply_exception_and_is_atomic_without_global_flush():
    rendered = policy.rules("54.1.2.3", existing=True)
    assert rendered.startswith("delete table inet mobile_qa_emulators\n")
    assert "flush ruleset" not in rendered
    assert "54.1.2.3/32" in rendered
    assert 'socket cgroupv2 level 1 "mobile-qa-emulators"' in rendered
    assert "ct state established ct direction reply accept" in rendered
    assert "ct state established,related accept" not in rendered
    assert "fib daddr type { local, broadcast, multicast } counter drop" in rendered
    assert rendered.index("ip daddr @blocked4") < rendered.index("meta l4proto { tcp, udp } accept")
    assert rendered.index("ip6 daddr @blocked6") < rendered.index(
        "meta l4proto { tcp, udp } accept"
    )


def test_metadata_address_never_interpolates_untrusted_commands():
    with pytest.raises(ipaddress.AddressValueError):
        policy.rules("1.2.3.4; flush ruleset", existing=False)
    assert not policy.rules(None, existing=False).startswith("delete")


def test_failed_kernel_policy_check_does_not_publish_ready_marker(tmp_path, monkeypatch):
    cgroup, runtime = tmp_path / "group", tmp_path / "runtime"
    cgroup.mkdir()
    (cgroup / "cgroup.procs").write_text("")
    monkeypatch.setattr(policy, "CGROUP", cgroup)
    monkeypatch.setattr(policy, "RUNTIME", runtime)
    monkeypatch.setattr(policy.os, "geteuid", lambda: 0)
    original_is_file = Path.is_file
    monkeypatch.setattr(
        Path,
        "is_file",
        lambda path: True
        if str(path) == "/sys/fs/cgroup/cgroup.controllers"
        else original_is_file(path),
    )
    monkeypatch.setattr(policy, "public_ipv4", lambda: "54.1.2.3")
    calls = []

    def run(*args, **kwargs):
        calls.append((args, kwargs))
        return SimpleNamespace(returncode=1)

    monkeypatch.setattr(policy, "_run", run)
    with pytest.raises(ValueError, match="unsupported"):
        policy.install()
    assert not (runtime / "policy.ready").exists()
    assert len(calls) == 2
    assert calls[-1][0] == ("-c", "-f", "-")


def test_network_stop_preserves_policy_and_proves_cgroup_empty(tmp_path, monkeypatch):
    cgroup, runtime = tmp_path / "group", tmp_path / "runtime"
    cgroup.mkdir()
    runtime.mkdir()
    (cgroup / "cgroup.procs").write_text("")
    (runtime / "policy.ready").write_text("boot")
    monkeypatch.setattr(policy, "CGROUP", cgroup)
    monkeypatch.setattr(policy, "RUNTIME", runtime)
    monkeypatch.setattr(policy, "_run", lambda *_a, **_k: pytest.fail("removed network rules"))
    policy.stop()
    assert (cgroup / "cgroup.kill").read_text() == "1"
    assert not (runtime / "policy.ready").exists()


def test_launcher_source_permits_only_self_migration_before_irreversible_drop():
    source = (NETWORK / "launch-emulator.c").read_text()
    assert "strcmp(argv[1], EMULATOR) != 0" in source
    assert "(long)getpid()" in source
    assert "setgroups(1, &device_group)" in source
    assert "setresgid(group, group, group)" in source
    assert "setresuid(owner, owner, owner)" in source
    assert "PR_SET_NO_NEW_PRIVS" in source
    assert source.index("current_policy();") < source.index("write(destination, process")
    assert source.index("setresuid(owner") < source.index("execv(EMULATOR")
    assert "system(" not in source
    assert "execvp(" not in source
    assert "clearenv()" in source


def test_successful_policy_install_publishes_marker_only_after_atomic_apply(tmp_path, monkeypatch):
    import hashlib

    cgroup, runtime = tmp_path / "group", tmp_path / "runtime"
    cgroup.mkdir()
    (cgroup / "cgroup.procs").write_text("")
    monkeypatch.setattr(policy, "CGROUP", cgroup)
    monkeypatch.setattr(policy, "RUNTIME", runtime)
    monkeypatch.setattr(policy.os, "geteuid", lambda: 0)
    original_is_file, original_read = Path.is_file, Path.read_text
    monkeypatch.setattr(
        Path,
        "is_file",
        lambda path: True
        if str(path) == "/sys/fs/cgroup/cgroup.controllers"
        else original_is_file(path),
    )
    boot = "ad1c0b8d-7269-4868-9ea1-1c0f104db0ad"
    monkeypatch.setattr(
        Path,
        "read_text",
        lambda path, *a, **k: boot + "\n"
        if str(path) == "/proc/sys/kernel/random/boot_id"
        else original_read(path, *a, **k),
    )
    monkeypatch.setattr(policy, "public_ipv4", lambda: "54.1.2.3")
    calls = []

    def run(*args, **kwargs):
        assert not (runtime / "policy.ready").exists()
        calls.append((args, kwargs))
        return SimpleNamespace(returncode=0)

    monkeypatch.setattr(policy, "_run", run)
    policy.install()
    assert len(calls) == 3
    assert calls[-1][0] == ("-f", "-")
    rendered = calls[-1][1]["source"]
    assert (runtime / "policy.ready").read_text() == (
        boot + "\n" + hashlib.sha256(rendered.encode()).hexdigest() + "\n"
    )
    assert (runtime / "policy.ready").stat().st_mode & 0o777 == 0o644


def test_network_cleanup_timeout_does_not_assert_success(tmp_path, monkeypatch):
    cgroup, runtime = tmp_path / "group", tmp_path / "runtime"
    cgroup.mkdir()
    runtime.mkdir()
    (cgroup / "cgroup.procs").write_text("123\n")
    monkeypatch.setattr(policy, "CGROUP", cgroup)
    monkeypatch.setattr(policy, "RUNTIME", runtime)
    ticks = iter([1.0, 12.0])
    monkeypatch.setattr(policy.time, "monotonic", lambda: next(ticks))
    with pytest.raises(ValueError, match="cleanup_unconfirmed"):
        policy.stop()
