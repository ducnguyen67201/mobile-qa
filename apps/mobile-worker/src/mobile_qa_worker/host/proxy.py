"""Executor adapter: capability-bound device access, with no emulator process ownership."""

import base64
import os
from pathlib import Path

from mobile_qa_worker.device.android import AndroidDevice
from mobile_qa_worker.generated.models import DirectCommand, SlotDeviceRequest, SlotDeviceResponse
from mobile_qa_worker.host.ipc import request
from mobile_qa_worker.qualification.config import Profile, QualificationError
from mobile_qa_worker.qualification.evidence import Evidence


class SupervisedDevice(AndroidDevice):
    def __init__(self, profile: Profile, evidence: Evidence, package: str, component: str):
        super().__init__(profile, evidence, package, component)
        self.remote_commands = True
        self.socket_path = Path(os.environ["MOBILE_QA_SLOT_SOCKET"])
        self.capability = os.environ["MOBILE_QA_SLOT_TOKEN"]
        self.lease_root = Path(os.environ["MOBILE_QA_SLOT_LEASE_ROOT"])
        if (
            not self.socket_path.is_absolute()
            or not self.lease_root.is_absolute()
            or not evidence.directory.is_relative_to(self.lease_root)
        ):
            raise QualificationError("slot_scope_mismatch")

    def call(self, operation: str, **values: object) -> SlotDeviceResponse:
        return request(
            self.socket_path,
            SlotDeviceRequest.model_validate(
                {"token": self.capability, "operation": operation, **values}
            ),
        )

    def boot(self, timeout: int | None = None) -> None:
        self.call(
            "boot",
            package=self.package,
            launch_component=self.launch_component,
            timeout_seconds=timeout,
        )
        self.ever_launched = True

    def install(self, apk: Path, expected: str) -> str:
        self.call("install", path=str(apk), sha256=expected)
        self.installed = True
        return expected

    def launch(self) -> None:
        self.call("launch")

    def adb(self, *args: str, timeout: float | None = None) -> bytes:
        result = self.call(
            "adb", arguments=list(args), timeout_seconds=int(timeout) if timeout else None
        )
        return base64.b64decode(result.data_base64 or "", validate=True)

    def hierarchy(self) -> bytes:
        result = self.call("hierarchy")
        return base64.b64decode(result.data_base64 or "", validate=True)

    def execute_direct(self, command: DirectCommand) -> None:
        self.call("direct", command=command.model_dump(mode="json"))

    def stop(self) -> None:
        self.call("stop")

    def discard(self, *, recovery: bool = False) -> None:
        if recovery:
            raise QualificationError("slot_recovery_requires_operator")
        self.call("discard")
