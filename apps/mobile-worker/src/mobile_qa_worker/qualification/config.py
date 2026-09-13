"""Nonsecret host profile, validated before a command can mutate a device."""

import json
import tomllib
from dataclasses import dataclass
from pathlib import Path
from typing import cast

from mobile_qa_worker.generated.models import QualificationRequest

PACKAGE = "ai.mobileqa.demo"
ACTIVITY = PACKAGE + "/.MainActivity"
SERIAL = "emulator-5554"
CASE = "persist-task-v1"


class QualificationError(Exception):
    """Stable, safe reason code; never contains a provider response or credential."""


@dataclass(frozen=True)
class Profile:
    sdk_root: Path
    state_root: Path
    model: str
    toolchain: Path
    system_image: str = "system-images;android-35;google_apis;x86_64"
    headless: bool = True
    doppler_project: str = "mobile-qa"
    doppler_config: str = "dev"
    boot_seconds: int = 180
    command_seconds: int = 20
    install_seconds: int = 90
    init_seconds: int = 60
    navigation_seconds: int = 180
    cleanup_seconds: int = 240
    attempt_seconds: int = 600
    assertion_seconds: int = 15
    max_steps: int = 30
    evidence_bytes: int = 262144000
    disk_min_bytes: int = 5368709120

    @classmethod
    def load(cls, path: Path) -> "Profile":
        raw: dict[str, object] = tomllib.loads(path.read_text())
        required = {"sdk_root", "state_root", "toolchain"}
        if not required <= raw.keys() or raw.keys() - cls.__dataclass_fields__.keys():
            raise QualificationError("invalid_profile_fields")
        strings = required | {"model", "doppler_project", "doppler_config", "system_image"}
        for name in strings:
            if name in raw and (not isinstance(raw[name], str) or not raw[name]):
                raise QualificationError("invalid_profile_string")
        if "headless" in raw and type(raw["headless"]) is not bool:
            raise QualificationError("invalid_profile_headless")
        for name in raw.keys() - strings - {"headless"}:
            value = raw[name]
            if type(value) is not int or not 1 <= value <= 53687091200:
                raise QualificationError("invalid_profile_limit")
        # Only local operator paths; no command strings or environment interpolation.
        for name in ("sdk_root", "state_root", "toolchain"):
            value = Path(cast(str, raw[name]))
            if not value.is_absolute() or value != value.resolve():
                raise QualificationError("profile_paths_must_be_absolute_without_symlinks")
            raw[name] = value
        raw.setdefault("model", "")
        profile = cls(**cast(dict[str, object], raw))  # type: ignore[arg-type]
        if profile.system_image not in (
            "system-images;android-35;google_apis;x86_64",
            "system-images;android-35;google_apis;arm64-v8a",
        ):
            raise QualificationError("unsupported_system_image")
        if profile.state_root == Path("/") or len(profile.state_root.parts) < 4:
            raise QualificationError("unsafe_state_root")
        if profile.model == "REQUIRED_APPROVED_MODEL_ID":
            raise QualificationError("model_profile_required")
        if not 1 <= profile.max_steps <= 100 or profile.navigation_seconds > 180:
            raise QualificationError("invalid_execution_budget")
        if (
            max(profile.boot_seconds, profile.install_seconds, profile.init_seconds) > 600
            or max(profile.command_seconds, profile.assertion_seconds) > 60
            or profile.cleanup_seconds > 600
            or profile.attempt_seconds > 3600
            or profile.evidence_bytes > 262144000
        ):
            raise QualificationError("invalid_execution_budget")
        return profile

    @property
    def abi(self) -> str:
        return self.system_image.rsplit(";", 1)[1]


def parse_request(raw: str) -> QualificationRequest:
    request = QualificationRequest.model_validate_json(raw, strict=True)
    if request.version != 1 or request.case_id != CASE or request.serial != SERIAL:
        raise QualificationError("invalid_request")
    for name in (request.apk_path, request.profile_path, request.output_root):
        path = Path(name)
        if not path.is_absolute() or path != path.resolve():
            raise QualificationError("request_paths_must_be_absolute_without_symlinks")
    return request


def json_object(path: Path) -> dict[str, object]:
    if path.stat().st_size > 1048576:
        raise QualificationError("metadata_too_large")
    value: object = json.loads(path.read_text())
    if not isinstance(value, dict):
        raise QualificationError("invalid_json_object")
    return cast(dict[str, object], value)
