"""One-shot app workers share a slot only through its private supervisor socket."""

import os
import subprocess
import sys
import time
from pathlib import Path
from uuid import uuid4

from mobile_qa_worker.execution.journal import read, write
from mobile_qa_worker.generated.models import SlotGrantResponse
from mobile_qa_worker.host.config import SlotConfig
from mobile_qa_worker.host.slot import Slot
from mobile_qa_worker.qualification.config import QualificationError
from mobile_qa_worker.qualification.process import stop_group


class SlotWorker:
    def __init__(self, slot: Slot, config: SlotConfig, profile: Path, socket: Path, origin: str):
        self.slot, self.config, self.profile, self.socket, self.origin = (
            slot,
            config,
            profile,
            socket,
            origin,
        )
        self.child: subprocess.Popen[bytes] | None = None
        self.mode = "execution-worker"
        self.state_root: Path | None = None
        self.last_work_at: float | None = None
        self.child_started: float | None = None
        self.journal = slot.root / "worker.json"
        if self.journal.exists():
            slot.state = "quarantined"

    def start(self, grant: SlotGrantResponse, cache: Path) -> None:
        if (
            self.child is not None
            or grant.profile_id != self.config.profile_id
            or grant.app_id != self.config.app_id
        ):
            raise QualificationError("slot_worker_scope_mismatch")
        state = self.slot.root / "workers" / str(uuid4())
        state.mkdir(parents=True, mode=0o700)
        token = self.slot.authorize()
        write(self.journal, {"state": "active", "worker_state": str(state), "mode": self.mode})
        env = {
            k: v
            for k, v in os.environ.items()
            if k in ("PATH", "HOME", "LANG", "LC_ALL", "JAVA_HOME", "VIRTUAL_ENV")
        }
        env.update(
            MOBILE_QA_WORKER_TOKEN=grant.worker_token,
            MOBILE_QA_SLOT_SOCKET=str(self.socket),
            MOBILE_QA_SLOT_TOKEN=token,
            MOBILE_QA_SLOT_LEASE_ROOT=str(state),
            MOBILE_QA_APK_CACHE_ROOT=str(cache),
        )
        args = [
            sys.executable,
            "-m",
            "mobile_qa_worker.cli",
            self.mode,
            "--origin",
            self.origin,
            "--state",
            str(state),
            "--profile",
            str(self.profile),
            "--once",
        ]
        if self.mode == "execution-worker":
            args += ["--profile-id", str(self.config.profile_id)]
        try:
            with (state / "worker.log").open("wb") as log:
                self.child = subprocess.Popen(
                    args,
                    env=env,
                    stdin=subprocess.DEVNULL,
                    stdout=log,
                    stderr=subprocess.STDOUT,
                    start_new_session=True,
                )
        except BaseException:
            self.slot.revoke()
            self.journal.unlink(missing_ok=True)
            raise
        self.state_root, self.child_started = state, time.monotonic()

    def reap(self) -> bool:
        if self.child is None or self.child.poll() is None:
            return False
        stop_group(self.child)
        success = self.child.returncode == 0
        self.child = None
        self.slot.revoke()
        if not success or self.unresolved_journals():
            try:
                self.slot.cleanup()
            finally:
                self.slot.quarantine()
            raise QualificationError("slot_worker_recovery_required")
        if self.slot.state not in ("offline", "idle"):
            self.slot.cleanup()
        self.mode = "task-worker" if self.mode == "execution-worker" else "execution-worker"
        self.child_started = None
        self.journal.unlink()
        return True

    def accepted_work(self) -> bool:
        """The parent worker records API acceptance before its device child can call IPC."""
        if self.state_root is None:
            return False
        execution = self.state_root / "execution.json"
        dirty = self.slot.root / "device" / "dirty.json"
        if not dirty.exists():
            return False
        marker = read(dirty)
        if self.mode == "execution-worker":
            return (
                execution.exists()
                and read(execution).get("state") == "active"
                and marker.get("attempt_id") == read(execution).get("attempt_id")
            )
        return isinstance(marker.get("phone_session"), str)

    def unresolved_journals(self) -> bool:
        if (self.slot.root / "device" / "dirty.json").exists():
            return True
        if self.state_root is None:
            return False
        execution = self.state_root / "execution.json"
        return (execution.exists() and read(execution).get("state") in ("active", "claiming")) or (
            self.state_root / "dirty.json"
        ).exists()

    def shutdown(self) -> None:
        # Process termination never manufactures a clean API receipt. Existing
        # execution/phone journals remain the operator recovery authority.
        if self.child is not None:
            stop_group(self.child, grace=5)
            self.child = None
        self.slot.revoke()
        self.slot.cleanup()
        if self.unresolved_journals():
            self.slot.quarantine()
            raise QualificationError("slot_worker_recovery_required")
