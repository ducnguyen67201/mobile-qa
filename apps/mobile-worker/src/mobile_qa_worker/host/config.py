"""Operator-owned local configuration; no credentials or customer state belong here."""

import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import cast
from uuid import UUID

from mobile_qa_worker.qualification.config import QualificationError


@dataclass(frozen=True)
class SlotConfig:
    index: int
    app_id: UUID
    profile_id: UUID
    memory_mib: int = 4096
    cores: int = 2
    rendering: str = "software"
    warm_qualification: str = ""

    @property
    def console_port(self) -> int:
        return 5554 + self.index * 2

    @property
    def adb_port(self) -> int:
        return 5038 + self.index


@dataclass(frozen=True)
class HostConfig:
    origin: str
    host_id: UUID
    state_root: Path
    profile: Path
    slots: tuple[SlotConfig, ...]
    warm_schedule: bool = False
    network_isolation_manifest: Path = Path("/etc/mobile-qa/network-isolation.json")

    @classmethod
    def load(cls, path: Path) -> "HostConfig":
        if path.is_symlink() or path.stat().st_size > 65536:
            raise QualificationError("invalid_host_configuration")
        values: dict[str, object] = tomllib.loads(path.read_text())
        required = {"origin", "host_id", "state_root", "profile", "slots"}
        optional = {"warm_schedule", "network_isolation_manifest"}
        if not required <= values.keys() or values.keys() - required - optional:
            raise QualificationError("invalid_host_configuration")
        slots = values.pop("slots")
        if not isinstance(slots, list) or not 1 <= len(cast(list[object], slots)) <= 8:
            raise QualificationError("invalid_host_slots")
        parsed: list[SlotConfig] = []
        for raw in cast(list[object], slots):
            if not isinstance(raw, dict):
                raise QualificationError("invalid_host_slots")
            item = cast(dict[str, object], raw)
            if not {"index", "app_id", "profile_id"} <= item.keys():
                raise QualificationError("invalid_host_slots")
            if item.keys() - SlotConfig.__dataclass_fields__.keys():
                raise QualificationError("invalid_host_slots")
            index, memory, cores = item["index"], item.get("memory_mib", 4096), item.get("cores", 2)
            if (
                type(index) is not int
                or not 0 <= index < 8
                or type(memory) is not int
                or not 1024 <= memory <= 16384
                or type(cores) is not int
                or not 1 <= cores <= 8
            ):
                raise QualificationError("invalid_slot_resources")
            rendering = item.get("rendering", "software")
            qualification = item.get("warm_qualification", "")
            if rendering not in ("software", "swiftshader") or not isinstance(qualification, str):
                raise QualificationError("invalid_slot_resources")
            parsed.append(
                SlotConfig(
                    index,
                    UUID(str(item["app_id"])),
                    UUID(str(item["profile_id"])),
                    memory,
                    cores,
                    str(rendering),
                    qualification,
                )
            )
        if len({slot.index for slot in parsed}) != len(parsed):
            raise QualificationError("duplicate_slot_index")
        root, profile = Path(str(values["state_root"])), Path(str(values["profile"]))
        if any(not p.is_absolute() or p != p.resolve() for p in (root, profile)):
            raise QualificationError("unsafe_host_path")
        if len(root.parts) < 4:
            raise QualificationError("unsafe_host_path")
        schedule = values.get("warm_schedule", False)
        if type(schedule) is not bool:
            raise QualificationError("invalid_warm_schedule")
        return cls(
            str(values["origin"]),
            UUID(str(values["host_id"])),
            root,
            profile,
            tuple(parsed),
            schedule,
            Path(
                str(
                    values.get(
                        "network_isolation_manifest", "/etc/mobile-qa/network-isolation.json"
                    )
                )
            ),
        )
