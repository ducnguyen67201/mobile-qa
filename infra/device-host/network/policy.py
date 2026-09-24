#!/usr/bin/python3
"""Root boot service for the fixed EC2 emulator network boundary.

The table is replaced atomically, never flushed globally. No daemon credentials or
APK inputs enter this program. Live Linux/KVM packet qualification remains required.
"""

import hashlib
import ipaddress
import os
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

CGROUP = Path("/sys/fs/cgroup/mobile-qa-emulators")
RUNTIME = Path("/run/mobile-qa-network")
NFT = "/usr/sbin/nft"
IPV4_DENY = (
    "0.0.0.0/8",
    "10.0.0.0/8",
    "100.64.0.0/10",
    "127.0.0.0/8",
    "169.254.0.0/16",
    "172.16.0.0/12",
    "192.0.0.0/24",
    "192.0.2.0/24",
    "192.168.0.0/16",
    "198.18.0.0/15",
    "198.51.100.0/24",
    "203.0.113.0/24",
    "224.0.0.0/4",
    "240.0.0.0/4",
)
# Translation/tunnel prefixes could encode an otherwise forbidden IPv4 endpoint.
IPV6_DENY = (
    "::/128",
    "::1/128",
    "::ffff:0:0/96",
    "64:ff9b::/96",
    "64:ff9b:1::/48",
    "100::/64",
    "2001::/32",
    "2001:db8::/32",
    "2002::/16",
    "fc00::/7",
    "fe80::/10",
    "ff00::/8",
)


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *_args, **_kwargs):
        raise ValueError("metadata_redirect_rejected")


def public_ipv4() -> str | None:
    """The fixed host public IPv4 is NATed by EC2 and is absent from local FIB."""
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}), NoRedirect())
    root = "http://169.254.169.254/latest/"
    request = urllib.request.Request(
        root + "api/token",
        method="PUT",
        headers={"X-aws-ec2-metadata-token-ttl-seconds": "60"},
    )
    with opener.open(request, timeout=3) as response:
        token = response.read(4097).decode("ascii")
    if not token or len(token) > 4096 or "\n" in token or "\r" in token:
        raise ValueError("metadata_token_invalid")
    request = urllib.request.Request(
        root + "meta-data/public-ipv4", headers={"X-aws-ec2-metadata-token": token}
    )
    try:
        with opener.open(request, timeout=3) as response:
            address = response.read(65).decode("ascii").strip()
    except urllib.error.HTTPError as error:
        if error.code == 404:
            return None
        raise ValueError("metadata_address_unavailable") from None
    return str(ipaddress.IPv4Address(address))


def rules(public_address: str | None, existing: bool) -> str:
    blocked4 = list(IPV4_DENY)
    if public_address:
        blocked4.append(str(ipaddress.IPv4Address(public_address)) + "/32")
    start = "delete table inet mobile_qa_emulators\n" if existing else ""
    return (
        start
        + """table inet mobile_qa_emulators {
    set blocked4 { type ipv4_addr; flags interval; auto-merge; elements = { %s }; }
    set blocked6 { type ipv6_addr; flags interval; auto-merge; elements = { %s }; }
    chain emulator_egress {
        ct state established ct direction reply accept
        fib daddr type { local, broadcast, multicast } counter drop
        ip daddr @blocked4 counter drop
        ip6 daddr @blocked6 counter drop
        meta l4proto { tcp, udp } accept
        counter drop
    }
    chain output {
        type filter hook output priority filter; policy accept;
        socket cgroupv2 level 1 "mobile-qa-emulators" jump emulator_egress
    }
}
"""
        % (", ".join(blocked4), ", ".join(IPV6_DENY))
    )


def _run(*args: str, source: str | None = None) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [NFT, *args],
        input=source,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
        timeout=10,
    )


def install() -> None:
    if os.geteuid() != 0 or not Path("/sys/fs/cgroup/cgroup.controllers").is_file():
        raise ValueError("root_and_cgroup_v2_required")
    RUNTIME.mkdir(mode=0o755, exist_ok=True)
    if RUNTIME.is_symlink() or CGROUP.is_symlink():
        raise ValueError("unsafe_network_policy_path")
    ready = RUNTIME / "policy.ready"
    ready.unlink(missing_ok=True)
    CGROUP.mkdir(mode=0o755, exist_ok=True)
    CGROUP.chmod(0o755)
    if (CGROUP / "cgroup.procs").read_text().strip():
        raise ValueError("emulator_cgroup_not_empty")
    existing = _run("list", "table", "inet", "mobile_qa_emulators").returncode == 0
    rendered = rules(public_ipv4(), existing)
    # -c validates kernel expression support; -f applies the entire batch atomically.
    if _run("-c", "-f", "-", source=rendered).returncode:
        raise ValueError("emulator_network_policy_unsupported")
    if _run("-f", "-", source=rendered).returncode:
        raise ValueError("emulator_network_policy_failed")
    digest = hashlib.sha256(rendered.encode()).hexdigest()
    boot = Path("/proc/sys/kernel/random/boot_id").read_text().strip()
    pending = RUNTIME / "policy.pending"
    pending.write_text(boot + "\n" + digest + "\n")
    pending.chmod(0o644)
    pending.replace(ready)


def stop() -> None:
    (RUNTIME / "policy.ready").unlink(missing_ok=True)
    if not CGROUP.exists():
        return
    # Emulators moved out of the supervisor cgroup require independent crash cleanup.
    # Keep nft rules installed until reboot; never create an unfiltered teardown gap.
    (CGROUP / "cgroup.kill").write_text("1")
    deadline = time.monotonic() + 10
    while (CGROUP / "cgroup.procs").read_text().strip():
        if time.monotonic() >= deadline:
            raise ValueError("emulator_cgroup_cleanup_unconfirmed")
        time.sleep(0.05)


def main() -> int:
    try:
        if len(sys.argv) != 2 or sys.argv[1] not in ("start", "stop"):
            raise ValueError("invalid_network_policy_command")
        (install if sys.argv[1] == "start" else stop)()
        return 0
    except Exception:
        # Metadata, command output and operator paths are never included in logs.
        print("mobile_qa_network_isolation_unavailable", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
