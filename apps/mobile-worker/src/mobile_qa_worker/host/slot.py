"""One ownership state machine per slot; uncertain cleanup remains quarantined on disk."""

import hmac
import secrets
from pathlib import Path
from typing import Protocol
from uuid import UUID, uuid4

from mobile_qa_worker.execution.journal import write
from mobile_qa_worker.qualification.config import QualificationError


class OwnedDevice(Protocol):
    def boot(self, timeout: int | None = None) -> None: ...
    def stop(self) -> None: ...
    def discard(self, *, recovery: bool = False) -> None: ...
    def assert_clean(self) -> None: ...
    def bind(self, package: str, launch_component: str) -> None: ...


class Slot:
    def __init__(self, slot_id: UUID, root: Path, device: OwnedDevice):
        self.id = slot_id
        self.root = root
        self.device = device
        self.journal = root / "owner.json"
        self.state = "quarantined" if self.journal.exists() else "offline"
        self.boot_id: UUID | None = None
        self.token: str | None = None
        self.pristine = False

    def prepare(self) -> None:
        if self.state not in ("offline", "idle") or self.token is not None:
            raise QualificationError("slot_not_available")
        if self.pristine:
            return
        self.boot_id = uuid4()
        self.state = "preparing"
        write(self.journal, {"state": "preparing", "emulator_boot_id": str(self.boot_id)})
        try:
            self.device.boot()
            self.pristine = True
            self.state = "idle"
            write(self.journal, {"state": "idle", "emulator_boot_id": str(self.boot_id)})
        except BaseException:
            self.cleanup()
            raise

    def authorize(self) -> str:
        if self.state not in ("offline", "idle") or self.token is not None:
            raise QualificationError("slot_not_available")
        self.token = secrets.token_urlsafe(32)
        return self.token

    def verify(self, token: str) -> None:
        if self.token is None or not hmac.compare_digest(self.token, token):
            raise QualificationError("slot_capability_expired")
        if self.state == "quarantined":
            raise QualificationError("slot_quarantined")

    def assign(self, token: str, package: str, component: str, timeout: int | None) -> None:
        self.verify(token)
        if self.state not in ("offline", "idle"):
            raise QualificationError("slot_already_assigned")
        self.device.bind(package, component)
        self.state = "preparing"
        if self.boot_id is None:
            self.boot_id = uuid4()
        write(self.journal, {"state": "leased", "emulator_boot_id": str(self.boot_id)})
        try:
            if not self.pristine:
                self.device.boot(timeout)
            self.pristine = False
            self.state = "leased"
        except BaseException:
            self.cleanup()
            raise

    def cleanup(self) -> None:
        self.state = "cleaning"
        write(self.journal, {"state": "cleaning", "emulator_boot_id": str(self.boot_id)})
        try:
            self.device.stop()
            self.device.discard()
            self.device.assert_clean()
        except BaseException:
            self.state = "quarantined"
            write(self.journal, {"state": "quarantined", "emulator_boot_id": str(self.boot_id)})
            raise
        self.state, self.pristine, self.boot_id = "offline", False, None
        self.journal.unlink()

    def quarantine(self) -> None:
        self.state = "quarantined"
        write(self.journal, {"state": "quarantined", "emulator_boot_id": str(self.boot_id)})

    def revoke(self) -> None:
        self.token = None

    def drained(self) -> bool:
        return self.token is None and self.state == "offline" and not self.journal.exists()
