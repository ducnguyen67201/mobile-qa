"""Fail closed until an operator has qualified guest networking for this exact image.

This is a release gate, not a firewall implementation. Private ADB ports and KVM
alone do not keep guest applications away from host services or cloud metadata.
"""

import stat
from pathlib import Path
from typing import cast

from mobile_qa_worker.qualification.config import QualificationError, json_object
from mobile_qa_worker.qualification.host import boot_id

REQUIRED_CONTROLS = {
    "guest_denies_cloud_metadata",
    "guest_denies_host_services",
    "guest_denies_private_networks",
    "guest_denies_other_slots",
}


def require_isolation(path: Path, toolchain_digest: str) -> None:
    if not path.is_absolute() or path != path.resolve() or path.is_symlink():
        raise QualificationError("network_isolation_not_qualified")
    details = path.stat(follow_symlinks=False)
    if (
        not stat.S_ISREG(details.st_mode)
        or details.st_uid != 0
        or details.st_mode & (stat.S_IWGRP | stat.S_IWOTH)
    ):
        raise QualificationError("network_isolation_manifest_not_trusted")
    manifest = json_object(path)
    controls = manifest.get("verified_controls")
    reference = manifest.get("evidence_reference")
    if (
        manifest.get("version") != 1
        or manifest.get("qualified") is not True
        or manifest.get("toolchain_sha256") != toolchain_digest
        or not isinstance(reference, str)
        or not reference.strip()
        or not isinstance(controls, list)
        or any(not isinstance(c, str) for c in cast(list[object], controls))
        or set(cast(list[str], controls)) != REQUIRED_CONTROLS
    ):
        raise QualificationError("network_isolation_not_qualified")


def require_installed_policy(path: Path = Path("/run/mobile-qa-network/policy.ready")) -> None:
    """The root boot service seals this only after installing the current nft policy."""
    if path.is_symlink() or path != path.resolve():
        raise QualificationError("network_policy_not_installed")
    details = path.stat(follow_symlinks=False)
    if (
        not stat.S_ISREG(details.st_mode)
        or details.st_uid != 0
        or details.st_size > 256
        or details.st_mode & (stat.S_IWGRP | stat.S_IWOTH)
    ):
        raise QualificationError("network_policy_not_trusted")
    values = path.read_text().splitlines()
    if (
        len(values) != 2
        or values[0] != boot_id()
        or len(values[1]) != 64
        or any(c not in "0123456789abcdef" for c in values[1])
    ):
        raise QualificationError("network_policy_stale")


def assert_emulators_stopped(
    path: Path = Path("/sys/fs/cgroup/mobile-qa-emulators/cgroup.procs"),
) -> None:
    """Escaped process groups still inherit the root-controlled emulator cgroup."""
    if path.is_symlink() or path != path.resolve() or path.stat().st_uid != 0:
        raise QualificationError("emulator_cgroup_unavailable")
    with path.open("rb") as source:
        if source.read(65536).strip():
            raise QualificationError("emulator_cgroup_not_empty")
