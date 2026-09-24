"""Persistent host reconciliation. Power-off remains the external controller's job."""

import logging
import os
import re
import signal
import threading
import time
from dataclasses import dataclass, replace
from datetime import UTC, datetime
from pathlib import Path
from uuid import UUID, uuid4
from zoneinfo import ZoneInfo

from mobile_qa_worker.artifacts.cache import Cache
from mobile_qa_worker.device.android import doctor
from mobile_qa_worker.execution.client import TransportError
from mobile_qa_worker.generated.models import (
    HostCleanupRequest,
    HostHeartbeatRequest,
    HostRegisterRequest,
    HostState,
    HostStatus,
    SlotDeviceRequest,
    SlotDeviceResponse,
    SlotGrantRequest,
)
from mobile_qa_worker.host.client import HostClient
from mobile_qa_worker.host.config import HostConfig, SlotConfig
from mobile_qa_worker.host.device import SlotAndroidDevice
from mobile_qa_worker.host.dispatch import dispatch
from mobile_qa_worker.host.ipc import DeviceServer
from mobile_qa_worker.host.isolation import (
    assert_emulators_stopped,
    require_installed_policy,
    require_isolation,
)
from mobile_qa_worker.host.policy import WarmPolicy
from mobile_qa_worker.host.runtime import SlotWorker
from mobile_qa_worker.host.slot import Slot
from mobile_qa_worker.qualification.config import Profile, QualificationError
from mobile_qa_worker.qualification.evidence import Evidence, sha256
from mobile_qa_worker.qualification.host import boot_id
from mobile_qa_worker.qualification.process import host_lock


@dataclass
class Runtime:
    slot: Slot
    worker: SlotWorker
    server: DeviceServer
    config: SlotConfig
    warm_qualified: bool
    next_claim_at: float = 0
    warm_thread: threading.Thread | None = None
    binding_index: int = 0


def slot_profile(source: Path, root: Path) -> tuple[Path, Profile]:
    """Copy only the operator's nonsecret profile and change its isolated state path."""
    destination = root / "profile.toml"
    content, count = re.subn(
        r"(?m)^state_root\s*=.*$", f'state_root = "{root / "device"}"', source.read_text(), count=1
    )
    if count != 1 or destination.is_symlink():
        raise QualificationError("invalid_slot_profile")
    destination.write_text(content)
    destination.chmod(0o600)
    profile = Profile.load(destination)
    if profile.state_root != root / "device":
        raise QualificationError("invalid_slot_profile")
    profile.state_root.mkdir(mode=0o700, parents=True, exist_ok=True)
    return destination, profile


def make_runtime(config: HostConfig, status: HostStatus, local: SlotConfig) -> Runtime:
    approved = next((s.definition for s in status.slots if s.definition.index == local.index), None)
    if (
        approved is None
        or not approved.qualified
        or approved.console_port != local.console_port
        or approved.adb_server_port != local.adb_port
        or approved.memory_mb != local.memory_mib
        or approved.cpu_cores != local.cores
        or not any(
            b.slot_id == approved.id
            and b.app_id == local.app_id
            and b.profile_id == local.profile_id
            for b in status.bindings
        )
    ):
        raise QualificationError("slot_not_operator_qualified")
    root = config.state_root / f"slot-{local.index}"
    if root.is_symlink() or root != root.resolve():
        raise QualificationError("unsafe_slot_root")
    root.mkdir(parents=True, mode=0o700, exist_ok=True)
    path, profile = slot_profile(config.profile, root)
    if profile.system_image != approved.system_image:
        raise QualificationError("slot_image_mismatch")
    evidence = Evidence(root / "supervisor" / str(uuid4()))
    device = SlotAndroidDevice(profile, evidence, local)
    slot = Slot(approved.id, root, device)
    worker = SlotWorker(slot, local, path, root / "device.sock", config.origin)
    socket_path = root / "device.sock"

    # A prior socket is not silently replaced: surviving commands may still own Android.
    def operation(request: SlotDeviceRequest) -> SlotDeviceResponse:
        if worker.state_root is None or (
            request.operation.value not in ("stop", "discard") and not worker.accepted_work()
        ):
            raise QualificationError("slot_assignment_missing")
        return dispatch(slot, device, worker.state_root, request)

    server = DeviceServer(socket_path, operation)
    server.start()
    return Runtime(
        slot, worker, server, local, approved.warm_qualified and bool(local.warm_qualification)
    )


def wants_warm(config: HostConfig, status: HostStatus, runtime: Runtime, now: datetime) -> bool:
    idle = (
        None
        if runtime.worker.last_work_at is None
        else time.monotonic() - runtime.worker.last_work_at
    )
    # Operator API policy is authoritative; local host configuration can only narrow it.
    for window in status.policy.warm_windows:
        if now.astimezone(ZoneInfo(status.policy.timezone)).weekday() not in {
            day.root for day in window.weekdays
        }:
            continue
        policy = WarmPolicy(
            status.policy.timezone,
            window.start,
            window.end,
            config.warm_schedule,
            status.policy.idle_seconds,
        )
        if policy.wanted(
            now,
            idle,
            target=status.policy.warm_target,
            qualified=runtime.warm_qualified,
            draining=status.state != HostState.ready,
        ):
            return True
    return WarmPolicy(grace_seconds=status.policy.idle_seconds).wanted(
        now,
        idle,
        target=status.policy.warm_target,
        qualified=runtime.warm_qualified,
        draining=status.state != HostState.ready,
    )


def prepare_warm(slot: Slot) -> None:
    try:
        slot.prepare()
    except (QualificationError, OSError, ValueError):
        slot.quarantine()
        logging.error("Warm slot requires operator recovery: slot=%s", slot.id)


def heartbeat(
    client: HostClient,
    status: HostStatus,
    boot: UUID,
    runtimes: list[Runtime],
    cache_bytes: int = 0,
) -> HostStatus:
    result = client.heartbeat(
        HostHeartbeatRequest.model_validate(
            {
                "generation": status.generation,
                "boot_id": str(boot),
                "cache_bytes": cache_bytes,
                "slots": [
                    {
                        "slot_id": str(r.slot.id),
                        "state": "idle"
                        if r.slot.state == "offline" and status.state == HostState.ready
                        else r.slot.state,
                        "emulator_boot_id": str(r.slot.boot_id) if r.slot.boot_id else None,
                    }
                    for r in runtimes
                ],
            }
        )
    )
    if result.boot_id != boot or result.generation != status.generation:
        raise QualificationError("host_boot_fenced")
    return result


def serve(config_path: Path, *, once: bool = False) -> None:
    config = HostConfig.load(config_path)
    base = Profile.load(config.profile)
    require_isolation(config.network_isolation_manifest, sha256(base.toolchain))
    require_installed_policy()
    doctor(base)
    boot = UUID(boot_id())
    client = HostClient(config.origin, config.host_id, os.environ.get("MOBILE_QA_HOST_TOKEN", ""))
    stop = threading.Event()

    def requested(_signum: int, _frame: object) -> None:
        stop.set()

    old = {sig: signal.signal(sig, requested) for sig in (signal.SIGINT, signal.SIGTERM)}
    runtimes: list[Runtime] = []
    with host_lock(config.state_root):
        cache_root = config.state_root / "apk-cache"
        cache = Cache(cache_root)
        status = client.register(
            HostRegisterRequest.model_validate(
                {"version": 1, "boot_id": str(boot), "toolchain_digest": sha256(base.toolchain)}
            )
        )
        if status.boot_id != boot:
            raise QualificationError("host_boot_fenced")
        try:
            for local in config.slots:
                runtimes.append(make_runtime(config, status, local))
            last_heartbeat = 0.0
            while not stop.is_set():
                now = time.monotonic()
                if now - last_heartbeat >= min(20, status.heartbeat_seconds):
                    try:
                        status = heartbeat(client, status, boot, runtimes, cache.resident_bytes())
                    except TransportError:
                        # Keep servicing accepted work. No new admission on stale control state.
                        stop.wait(2)
                        continue
                    last_heartbeat = now
                ready = status.state == HostState.ready and status.policy.enabled
                for position, runtime in enumerate(runtimes):
                    slot, worker = runtime.slot, runtime.worker
                    try:
                        if slot.state == "leased":
                            worker.last_work_at = now
                        if runtime.warm_thread is not None:
                            if runtime.warm_thread.is_alive():
                                continue
                            runtime.warm_thread = None
                        if worker.reap() and worker.mode == "execution-worker":
                            runtime.binding_index += 1
                        if worker.child is not None or slot.state == "quarantined":
                            continue
                        warm = position < status.policy.warm_target and wants_warm(
                            config, status, runtime, datetime.now(UTC)
                        )
                        if not ready:
                            if slot.state != "offline":
                                slot.cleanup()
                            continue
                        if not warm and slot.pristine:
                            slot.cleanup()
                        if warm and not slot.pristine:
                            runtime.warm_thread = threading.Thread(
                                target=prepare_warm, args=(slot,), daemon=True
                            )
                            runtime.warm_thread.start()
                            continue
                        if now < runtime.next_claim_at:
                            continue
                        cache.admit(
                            required_bytes=len(config.slots)
                            * (8 * 1024**3 + 2 * 2147483648 + 262144000)
                        )
                        bindings = [b for b in status.bindings if b.slot_id == slot.id]
                        if not bindings:
                            raise QualificationError("slot_binding_revoked")
                        binding = bindings[runtime.binding_index % len(bindings)]
                        worker.config = replace(
                            runtime.config, app_id=binding.app_id, profile_id=binding.profile_id
                        )
                        grant = client.grant(
                            slot.id,
                            SlotGrantRequest.model_validate(
                                {
                                    "generation": status.generation,
                                    "boot_id": str(boot),
                                    "app_id": str(binding.app_id),
                                    "profile_id": str(binding.profile_id),
                                }
                            ),
                        )
                        worker.start(grant, cache_root)
                        runtime.next_claim_at = now + 3
                    except TransportError:
                        runtime.next_claim_at = now + 5
                    except (QualificationError, OSError, ValueError):
                        slot.quarantine()
                        logging.error("Device slot requires operator recovery: slot=%s", slot.id)
                if status.state == HostState.draining and all(
                    r.worker.child is None
                    and not r.worker.unresolved_journals()
                    and r.slot.drained()
                    for r in runtimes
                ):
                    try:
                        assert_emulators_stopped()
                        status = client.cleanup(
                            HostCleanupRequest.model_validate(
                                {
                                    "generation": status.generation,
                                    "boot_id": str(boot),
                                    "processes_stopped": True,
                                    "ports_released": True,
                                    "journals_resolved": True,
                                }
                            )
                        )
                    except (QualificationError, OSError):
                        for runtime in runtimes:
                            runtime.slot.quarantine()
                        logging.error("Host drain requires operator process recovery")
                if once:
                    break
                stop.wait(0.5)
        finally:
            for runtime in runtimes:
                try:
                    runtime.server.close()
                    if runtime.warm_thread is not None and runtime.warm_thread.is_alive():
                        raise QualificationError("warm_boot_still_running")
                    runtime.worker.shutdown()
                except (QualificationError, OSError):
                    logging.error(
                        "Device slot cleanup remains unresolved: slot=%s", runtime.slot.id
                    )
            for sig, handler in old.items():
                signal.signal(sig, handler)
