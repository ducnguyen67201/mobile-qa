"""Own one disposable AVD. Refuse existing emulator-5554 rather than borrowing it."""

import os
import platform
import shutil
import socket
import subprocess
import time
import zipfile
from pathlib import Path
from typing import TYPE_CHECKING, cast

if TYPE_CHECKING:
    from mobile_qa_worker.generated.models import DirectCommand

from mobile_qa_worker.qualification.config import (
    Profile,
    QualificationError,
    json_object,
)
from mobile_qa_worker.qualification.evidence import Evidence, sha256
from mobile_qa_worker.qualification.host import native_abi
from mobile_qa_worker.qualification.process import command, stop_group
from mobile_qa_worker.qualification.verifier import validate_png


def assert_ports_available(ports: tuple[int, ...] = (5554, 5555)) -> None:
    """Prove we can listen, without treating a closed TCP connection as a live owner."""
    for port in ports:
        with socket.socket() as sock:
            sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
            sock.bind(("127.0.0.1", port))
            # listen also rejects a competing owner on platforms that permit bind
            # with SO_REUSEADDR. We never set SO_REUSEPORT or contact that owner.
            sock.listen(1)


def host_environment(profile: Profile) -> dict[str, str]:
    env = {
        key: value
        for key, value in os.environ.items()
        if key in ("PATH", "JAVA_HOME", "LANG", "LC_ALL", "HOME")
    }
    env.update(
        ANDROID_HOME=str(profile.sdk_root),
        ANDROID_AVD_HOME=str(profile.state_root / "current"),
        ANDROID_USER_HOME=str(profile.state_root / "android"),
        TZ="UTC",
    )
    return env


def doctor(profile: Profile) -> dict[str, str]:
    if profile.abi != native_abi():
        raise QualificationError("host_image_architecture_mismatch")
    linux = platform.system() == "Linux"
    if linux and not os.access("/dev/kvm", os.R_OK | os.W_OK):
        raise QualificationError("kvm_unavailable")
    inventory = {"host_os": platform.system(), "host_arch": platform.machine()}
    manifest = json_object(profile.toolchain)
    packages = manifest.get("packages")
    if not isinstance(packages, list):
        raise QualificationError("invalid_toolchain")
    for package in cast(list[object], packages):
        if not isinstance(package, dict):
            raise QualificationError("invalid_toolchain")
        values_map = cast(dict[str, object], package)
        name, revision = values_map.get("path"), values_map.get("revision")
        if not isinstance(name, str) or not isinstance(revision, str):
            raise QualificationError("invalid_toolchain")
        if name.startswith("system-images;") and name != profile.system_image:
            continue
        properties = profile.sdk_root / name.replace(";", "/") / "source.properties"
        values = dict(
            line.split("=", 1) for line in properties.read_text().splitlines() if "=" in line
        )
        actual = values.get("Pkg.Revision", "").strip()
        if actual != revision:
            raise QualificationError("toolchain_revision_mismatch")
        inventory[name] = actual
    if profile.system_image not in inventory:
        raise QualificationError("system_image_not_pinned")
    accel = command([str(profile.sdk_root / "emulator/emulator"), "-accel-check"], 20).decode()
    expected = "KVM" if linux else "Hypervisor.Framework"
    # The Mac tool reports a numeric success status and OS version, not Linux's
    # "installed and usable" sentence. Check the status rather than prose.
    lines = [line.strip() for line in accel.splitlines() if line.strip()]
    if len(lines) < 3 or lines[:2] != ["accel:", "0"] or expected not in accel:
        raise QualificationError("hardware_acceleration_unavailable")
    inventory["accelerator"] = expected
    if shutil.disk_usage(profile.sdk_root).free < profile.disk_min_bytes:
        raise QualificationError("insufficient_disk")
    return inventory


class AndroidDevice:
    def __init__(
        self,
        profile: Profile,
        evidence: Evidence,
        package: str,
        launch_component: str,
        *,
        console_port: int = 5554,
        adb_server_port: int = 5037,
        memory_mib: int = 4096,
        cores: int = 2,
        rendering: str | None = None,
        emulator_prefix: tuple[str, ...] = (),
        dns_servers: tuple[str, ...] = (),
    ):
        if console_port % 2 or not 5554 <= console_port <= 5680:
            raise QualificationError("invalid_emulator_port")
        if not 1024 <= adb_server_port <= 65535 or not 1024 <= memory_mib <= 16384:
            raise QualificationError("invalid_slot_resources")
        if not 1 <= cores <= 8:
            raise QualificationError("invalid_slot_resources")
        self.emulator_prefix = emulator_prefix
        self.dns_servers = dns_servers
        self.console_port = console_port
        self.adb_server_port = adb_server_port
        self.serial = f"emulator-{console_port}"
        self.memory_mib = memory_mib
        self.cores = cores
        self.rendering = rendering or (
            "auto" if platform.system() == "Darwin" else "swiftshader_indirect"
        )
        self.remote_commands = False
        self.package = package
        self.launch_component = launch_component
        self.installed = False
        self.profile = profile
        self.evidence = evidence
        self.child: subprocess.Popen[bytes] | None = None
        self.env = host_environment(profile)
        self.env["ANDROID_ADB_SERVER_PORT"] = str(adb_server_port)
        self.current = profile.state_root / "current"
        self.owns_current = False
        self.ever_launched = False
        self.inventory: dict[str, str] = {}
        self.video_pid: str | None = None
        self.direct_automation = False

    def adb(self, *args: str, timeout: float | None = None) -> bytes:
        return command(
            [
                str(self.profile.sdk_root / "platform-tools/adb"),
                "-P",
                str(self.adb_server_port),
                "-s",
                self.serial,
                *args,
            ],
            timeout or self.profile.command_seconds,
            self.env,
        )

    def hierarchy(self) -> bytes:
        raise QualificationError("remote_device_operation_unavailable")

    def execute_direct(self, command: "DirectCommand") -> None:
        raise QualificationError("remote_device_operation_unavailable")

    def assert_ports_released(self) -> None:
        if self.console_port == 5554:
            assert_ports_available()
        else:
            assert_ports_available((self.console_port, self.console_port + 1))

    def boot(self, timeout: int | None = None) -> None:
        try:
            self.assert_ports_released()
        except OSError as exc:
            raise QualificationError("emulator_port_owned_elsewhere") from exc
        if self.current.exists():
            raise QualificationError("dirty_avd_requires_recovery")
        self.current.mkdir(mode=0o700)
        self.owns_current = True
        command(
            [
                str(self.profile.sdk_root / "cmdline-tools/19.0/bin/avdmanager"),
                "create",
                "avd",
                "--name",
                "mobile-qa-owned",
                "--package",
                self.profile.system_image,
                "--path",
                str(self.current / "phone.avd"),
                "--device",
                "pixel_2",
            ],
            60,
            self.env,
        )
        config = self.current / "phone.avd/config.ini"
        settings = dict(
            line.split("=", 1) for line in config.read_text().splitlines() if "=" in line
        )
        settings.update(
            {
                "hw.lcd.width": "1080",
                "hw.lcd.height": "1920",
                "hw.lcd.density": "420",
                "hw.ramSize": str(self.memory_mib),
                "hw.cpu.ncore": str(self.cores),
                "hw.sdCard": "no",
                "showDeviceFrame": "no",
            }
        )
        config.write_text("".join(f"{key}={value}\n" for key, value in settings.items()))
        log = (self.evidence.directory / "emulator.log").open("ab")
        try:
            self.child = subprocess.Popen(
                [
                    *self.emulator_prefix,
                    str(self.profile.sdk_root / "emulator/emulator"),
                    "-avd",
                    "mobile-qa-owned",
                    "-port",
                    str(self.console_port),
                    *(["-no-window"] if self.profile.headless else []),
                    "-accel",
                    "on",
                    "-no-audio",
                    "-no-metrics",
                    *(["-dns-server", ",".join(self.dns_servers)] if self.dns_servers else []),
                    "-crash-report-mode",
                    "disabled",
                    "-no-snapshot",
                    "-wipe-data",
                    "-gpu",
                    self.rendering,
                    "-timezone",
                    "Etc/UTC",
                ],
                env=self.env,
                stdin=subprocess.DEVNULL,
                stdout=log,
                stderr=subprocess.STDOUT,
                start_new_session=True,
            )
            self.ever_launched = True
        finally:
            log.close()
        deadline = time.monotonic() + (timeout or self.profile.boot_seconds)
        while time.monotonic() < deadline:
            if self.child.poll() is not None:
                raise QualificationError("emulator_exited")
            try:
                if self.adb("shell", "getprop", "sys.boot_completed").strip() == b"1":
                    self.adb("shell", "pm", "path", "android")
                    validate_png(self.adb("exec-out", "screencap", "-p"))
                    self.adb("shell", "settings", "put", "system", "accelerometer_rotation", "0")
                    self.adb("shell", "settings", "put", "system", "user_rotation", "0")
                    self.inventory.update(
                        {
                            "serial": self.serial,
                            "api": self.adb("shell", "getprop", "ro.build.version.sdk")
                            .decode()
                            .strip(),
                            "abi": self.adb("shell", "getprop", "ro.product.cpu.abi")
                            .decode()
                            .strip(),
                            "locale": self.adb("shell", "getprop", "persist.sys.locale")
                            .decode()
                            .strip()
                            or self.adb("shell", "getprop", "ro.product.locale").decode().strip(),
                            "timezone": self.adb("shell", "getprop", "persist.sys.timezone")
                            .decode()
                            .strip(),
                            "fingerprint": self.adb("shell", "getprop", "ro.build.fingerprint")
                            .decode()
                            .strip(),
                            "density": self.adb("shell", "wm", "density").decode().strip(),
                        }
                    )
                    if (
                        self.inventory["api"] != "35"
                        or self.inventory["abi"] != self.profile.abi
                        or self.inventory["locale"] != "en-US"
                        or self.inventory["timezone"] not in ("UTC", "Etc/UTC")
                        or "420" not in self.inventory["density"]
                    ):
                        raise QualificationError("device_profile_mismatch")
                    return
            except QualificationError as exc:
                if str(exc) == "device_profile_mismatch":
                    raise
            time.sleep(0.5)
        raise QualificationError("boot_timeout")

    def install(self, apk: Path, expected: str) -> str:
        if not apk.is_file() or not 1 <= apk.stat().st_size <= 2147483648:
            raise QualificationError("apk_size_invalid")
        digest = sha256(apk)
        if digest != expected:
            raise QualificationError("apk_checksum_mismatch")
        try:
            with zipfile.ZipFile(apk) as archive:
                names = archive.namelist()
                if "AndroidManifest.xml" not in names:
                    raise QualificationError("invalid_apk")
                abis = {
                    name.split("/")[1]
                    for name in names
                    if name.startswith("lib/") and name.endswith(".so")
                }
                if abis and self.profile.abi not in abis:
                    raise QualificationError("unsupported_abi")
        except zipfile.BadZipFile as exc:
            raise QualificationError("invalid_apk") from exc
        metadata = command(
            [str(self.profile.sdk_root / "build-tools/35.0.0/aapt"), "dump", "badging", str(apk)]
        ).decode()
        import re

        identity = re.search(r"^package: name='([^']+)'", metadata, re.MULTILINE)
        if identity is None or identity[1] != self.package:
            raise QualificationError("package_mismatch")
        sdk = re.search(r"sdkVersion:'(\d+)'", metadata)
        if sdk is None or int(sdk[1]) > 35:
            raise QualificationError("unsupported_sdk")
        if b"Success" not in self.adb("install", str(apk), timeout=self.profile.install_seconds):
            raise QualificationError("install_failed")
        resolved = (
            self.adb(
                "shell",
                "cmd",
                "package",
                "resolve-activity",
                "--brief",
                "-a",
                "android.intent.action.MAIN",
                "-c",
                "android.intent.category.LAUNCHER",
                self.package,
            )
            .decode()
            .strip()
            .splitlines()
        )
        components = [line.strip() for line in resolved if "/" in line]

        def canonical(component: str) -> str:
            package, _, activity = component.partition("/")
            return package + "/" + (package + activity if activity.startswith(".") else activity)

        if len(components) != 1 or canonical(components[0]) != canonical(self.launch_component):
            raise QualificationError("launcher_mismatch")
        self.installed = True
        return digest

    def launch(self) -> None:
        if not self.installed:
            raise QualificationError("app_not_installed")
        self.adb("shell", "am", "start", "-W", "-n", self.launch_component)

    def raw_snapshot(self) -> tuple[bytes, bytes]:
        from mobile_qa_worker.automation.direct import hierarchy, nodes
        from mobile_qa_worker.device.capture import sanitize

        foreground = self.adb("shell", "dumpsys", "activity", "activities").decode()
        if not any(
            self.package + "/" in line
            and ("mResumedActivity" in line or "topResumedActivity" in line)
            for line in foreground.splitlines()
        ):
            raise QualificationError("wrong_foreground_package")
        xml = hierarchy(self)
        nodes(xml, self.package)
        png = self.adb("exec-out", "screencap", "-p")
        validate_png(png)
        return sanitize(xml, png, self.package)

    def capture_checkpoint(self, name: str) -> None:
        xml, png = self.raw_snapshot()
        self.evidence.write(name + ".xml", xml, "application/xml")
        self.evidence.write(name + ".png", png, "image/png")

    def start_recording(self) -> None:
        try:
            pid = (
                self.adb(
                    "shell",
                    "screenrecord --time-limit 180 /data/local/tmp/qualification.mp4 "
                    ">/dev/null 2>&1 & echo $!",
                )
                .decode()
                .strip()
            )
            if not pid.isdigit():
                raise QualificationError("recording_unavailable")
            self.video_pid = pid
        except (QualificationError, OSError):
            self.evidence.missing("video.mp4", "recording_unavailable")

    def collect_diagnostics(self) -> None:
        if self.video_pid is not None:
            try:
                self.adb("shell", "kill", "-2", self.video_pid)
                time.sleep(1)
                video = self.adb("exec-out", "cat", "/data/local/tmp/qualification.mp4")
                if len(video) < 16 or video[4:8] != b"ftyp":
                    raise QualificationError("recording_incomplete")
                self.evidence.write("video.mp4", video, "video/mp4")
            except (QualificationError, OSError):
                self.evidence.missing("video.mp4", "recording_incomplete_or_over_budget")
            self.video_pid = None
        try:
            self.evidence.write("logcat.txt", self.adb("logcat", "-d", "-t", "500"), "text/plain")
        except (QualificationError, OSError):
            self.evidence.missing("logcat.txt", "logcat_unavailable")

    def stop(self) -> None:
        if self.child is not None:
            stop_group(self.child)
            self.child = None
        # Reuse requires the emulator ports to be free, not just a reaped Popen.
        # macOS can release its listeners shortly after the process group exits.
        # A live listener must still block reuse; closed TCP sessions must not.
        deadline = time.monotonic() + 5
        while True:
            try:
                self.assert_ports_released()
                return
            except OSError as exc:
                if time.monotonic() >= deadline:
                    raise QualificationError("emulator_still_running") from exc
                time.sleep(0.1)

    def discard(self, *, recovery: bool = False) -> None:
        if self.child is not None:
            raise QualificationError("cannot_delete_running_avd")
        if self.current.exists():
            if (
                self.current.is_symlink()
                or self.current.resolve().parent != self.profile.state_root
            ):
                raise QualificationError("unsafe_avd_path")
            # Only the explicit operator recovery path may discard a prior instance.
            if not self.owns_current and not recovery:
                raise QualificationError("dirty_avd_requires_recovery")
            shutil.rmtree(self.current)
            self.owns_current = False
