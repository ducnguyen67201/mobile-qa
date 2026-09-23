"""Slot Android adapter owns its private foreground ADB server and emulator group."""

import subprocess
import time
from pathlib import Path

from mobile_qa_worker.device.android import AndroidDevice, assert_ports_available
from mobile_qa_worker.host.config import SlotConfig
from mobile_qa_worker.qualification.config import Profile, QualificationError
from mobile_qa_worker.qualification.evidence import Evidence
from mobile_qa_worker.qualification.process import command, stop_group


class SlotAndroidDevice(AndroidDevice):
    def __init__(self, profile: Profile, evidence: Evidence, config: SlotConfig):
        super().__init__(
            profile,
            evidence,
            "",
            "",
            console_port=config.console_port,
            adb_server_port=config.adb_port,
            memory_mib=config.memory_mib,
            cores=config.cores,
            rendering=config.rendering,
            emulator_prefix=("/usr/local/libexec/mobile-qa-emulator",),
            dns_servers=("1.1.1.1", "8.8.8.8"),
        )
        self.server: subprocess.Popen[bytes] | None = None
        self.serial = f"127.0.0.1:{config.console_port + 1}"
        self.transport_connected = False

    def start_server(self) -> None:
        assert_ports_available((self.adb_server_port,))
        log_path = self.evidence.directory / "adb.log"
        with log_path.open("ab") as log:
            self.server = subprocess.Popen(
                [
                    str(self.profile.sdk_root / "platform-tools/adb"),
                    "-L",
                    f"tcp:127.0.0.1:{self.adb_server_port}",
                    "server",
                    "nodaemon",
                ],
                stdin=subprocess.DEVNULL,
                stdout=log,
                stderr=subprocess.STDOUT,
                env=self.env,
                start_new_session=True,
            )
        # Poll the owned process only. Connecting to an existing server is forbidden.
        deadline = time.monotonic() + 5
        while time.monotonic() < deadline:
            if self.server.poll() is not None:
                raise QualificationError("slot_adb_server_failed")
            try:
                assert_ports_available((self.adb_server_port,))
            except OSError:
                return
            time.sleep(0.05)
        raise QualificationError("slot_adb_server_timeout")

    def adb(self, *args: str, timeout: float | None = None) -> bytes:
        if not self.transport_connected:
            # The guest cannot initiate traffic toward the host ADB server. Start
            # the transport from the trusted side of the egress firewall instead.
            response = command(
                [
                    str(self.profile.sdk_root / "platform-tools/adb"),
                    "-P",
                    str(self.adb_server_port),
                    "connect",
                    self.serial,
                ],
                3,
                self.env,
            )
            if b"connected to " + self.serial.encode() not in response:
                raise QualificationError("slot_adb_transport_not_ready")
            self.transport_connected = True
        return super().adb(*args, timeout=timeout)

    def boot(self, timeout: int | None = None) -> None:
        if self.server is None:
            self.start_server()
        super().boot(timeout)

    def stop(self) -> None:
        # Stop only groups we spawned. Never send adb kill-server to a shared endpoint.
        super().stop()
        if self.server is not None:
            stop_group(self.server)
            self.server = None
        assert_ports_available((self.adb_server_port,))
        self.transport_connected = False

    def discard(self, *, recovery: bool = False) -> None:
        super().discard(recovery=recovery)
        self.installed = False
        self.direct_automation = False
        self.video_pid = None
        self.inventory.clear()

    def bind(self, package: str, launch_component: str) -> None:
        import re

        if (
            not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*(?:\.[A-Za-z0-9_]+)+", package)
            or not launch_component.startswith(package + "/")
            or any(c.isspace() for c in launch_component)
        ):
            raise QualificationError("invalid_slot_package")
        if self.installed:
            raise QualificationError("slot_reset_required")
        self.package, self.launch_component = package, launch_component

    def assert_clean(self) -> None:
        if self.child is not None or self.server is not None or self.current.exists():
            raise QualificationError("slot_cleanup_unconfirmed")
        assert_ports_available((self.console_port, self.console_port + 1, self.adb_server_port))

    @staticmethod
    def allowed_apk(path: Path, root: Path) -> Path:
        if (
            path.name != "build.apk"
            or not path.is_absolute()
            or path.is_symlink()
            or path != path.resolve()
            or not path.is_relative_to(root)
        ):
            raise QualificationError("slot_artifact_scope_mismatch")
        return path
