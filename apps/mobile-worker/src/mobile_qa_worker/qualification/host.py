"""Host differences only; Android test execution stays shared across Mac and Linux."""

import platform
from pathlib import Path
from uuid import UUID

from mobile_qa_worker.qualification.config import QualificationError
from mobile_qa_worker.qualification.process import command


def native_abi() -> str:
    system, machine = platform.system(), platform.machine().lower()
    if system == "Darwin" and machine in ("arm64", "aarch64"):
        return "arm64-v8a"
    if system in ("Darwin", "Linux") and machine in ("x86_64", "amd64"):
        return "x86_64"
    raise QualificationError("unsupported_device_host")


def boot_id() -> str:
    """A real boot identity prevents stale workers being mistaken for a clean host."""
    if platform.system() == "Darwin":
        value = command(["/usr/sbin/sysctl", "-n", "kern.bootsessionuuid"]).decode().strip()
    elif platform.system() == "Linux":
        value = Path("/proc/sys/kernel/random/boot_id").read_text().strip()
    else:
        raise QualificationError("unsupported_device_host")
    try:
        return str(UUID(value))
    except ValueError as exc:
        raise QualificationError("host_boot_identity_unavailable") from exc
