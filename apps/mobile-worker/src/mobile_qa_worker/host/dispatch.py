"""Authorize one bounded device operation after the private capability check."""

from pathlib import Path

from mobile_qa_worker.automation.direct import execute, hierarchy
from mobile_qa_worker.generated.models import SlotDeviceRequest, SlotDeviceResponse
from mobile_qa_worker.host.device import SlotAndroidDevice
from mobile_qa_worker.host.ipc import encoded
from mobile_qa_worker.host.slot import Slot
from mobile_qa_worker.qualification.config import QualificationError


def dispatch(
    slot: Slot, device: SlotAndroidDevice, root: Path, request: SlotDeviceRequest
) -> SlotDeviceResponse:
    slot.verify(request.token)
    operation = request.operation.value
    timeout = request.timeout_seconds
    if timeout is not None and not 1 <= timeout <= 600:
        raise QualificationError("invalid_slot_deadline")
    data = b""
    if operation == "boot":
        slot.assign(request.token, request.package or "", request.launch_component or "", timeout)
    elif operation in ("stop", "discard"):
        if not slot.drained() and slot.state != "offline":
            slot.cleanup()
    else:
        if slot.state != "leased":
            raise QualificationError("slot_lease_not_ready")
        if operation == "install":
            apk = device.allowed_apk(Path(request.path or ""), root)
            if not request.sha256 or len(request.sha256) != 64:
                raise QualificationError("invalid_slot_digest")
            device.install(apk, request.sha256)
        elif operation == "launch":
            device.launch()
        elif operation == "adb":
            # Host filesystem transfer, server controls, arbitrary device selection,
            # reverse tunnels and package installation have dedicated boundaries.
            if (
                not request.arguments
                or request.arguments[0] not in ("shell", "exec-out", "logcat")
                or len(request.arguments) > 32
                or sum(len(arg) for arg in request.arguments) > 16384
            ):
                raise QualificationError("slot_adb_operation_denied")
            data = device.adb(*request.arguments, timeout=timeout)
        elif operation == "direct":
            if request.command is None:
                raise QualificationError("slot_command_missing")
            execute(device, device.package, request.command)
        elif operation == "hierarchy":
            data = hierarchy(device)
        elif operation == "snapshot":
            xml, data = device.raw_snapshot()
            return SlotDeviceResponse.model_validate(
                {"ok": True, "data_base64": encoded(data), "xml_base64": encoded(xml)}
            )
        else:
            raise QualificationError("slot_operation_denied")
    return SlotDeviceResponse.model_validate({"ok": True, "data_base64": encoded(data)})
