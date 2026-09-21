"""Local demo entry point: real device smoke by default, explicit paid agent mode.

The ADB driver is deliberately limited to our demo's two controls. It is not a
replacement agent, customer test executor, or evidence of Minitap qualification.
"""

import argparse
import json
import re
import time
from dataclasses import asdict, replace
from pathlib import Path
from uuid import UUID, uuid4
from xml.etree import ElementTree

from mobile_qa_worker.generated.models import ResolvedModel
from mobile_qa_worker.qualification.campaign import accepted, backend
from mobile_qa_worker.qualification.config import (
    ACTIVITY,
    CASE,
    PACKAGE,
    SERIAL,
    Profile,
    QualificationError,
    parse_request,
)
from mobile_qa_worker.qualification.device import Device, doctor
from mobile_qa_worker.qualification.evidence import atomic_json, sha256
from mobile_qa_worker.qualification.runner import ROOT, run_attempt


def navigate_demo(device: Device, task: str) -> None:
    # Only generated UUID task names may reach Android's input shell command.
    if task != "qa-" + str(UUID(task.removeprefix("qa-"))):
        raise QualificationError("invalid_demo_task")
    for resource in ("task_input", "save_task"):
        _, xml, _ = device.snapshot(task)
        root = ElementTree.fromstring(xml)  # snapshot has already validated this hierarchy.
        nodes = [
            n
            for n in root.iter("node")
            if n.get("package") == PACKAGE
            and n.get("resource-id") == PACKAGE + ":id/" + resource
            and n.get("enabled") == "true"
        ]
        if len(nodes) != 1:
            raise QualificationError("demo_control_unavailable")
        bounds = re.fullmatch(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", nodes[0].get("bounds", ""))
        if bounds is None:
            raise QualificationError("demo_control_bounds_invalid")
        left, top, right, bottom = map(int, bounds.groups())
        if not (0 <= left < right <= 1080 and 0 <= top < bottom <= 1920):
            raise QualificationError("demo_control_bounds_invalid")
        device.adb("shell", "input", "tap", str((left + right) // 2), str((top + bottom) // 2))
        if resource == "task_input":
            # input tap returning does not mean Android has focused the editor.
            # Observe focus before injecting text; cold-start IMEs can drop keys.
            deadline = time.monotonic() + device.profile.assertion_seconds
            while True:
                _, focus_xml, _ = device.snapshot(task)
                fields = [
                    n
                    for n in ElementTree.fromstring(focus_xml).iter("node")
                    if n.get("package") == PACKAGE
                    and n.get("resource-id") == PACKAGE + ":id/task_input"
                ]
                if len(fields) == 1 and fields[0].get("focused") == "true":
                    break
                if time.monotonic() >= deadline:
                    raise QualificationError("demo_input_not_focused")
                time.sleep(0.1)
            device.adb("shell", "input", "text", task)
            _, typed_xml, _ = device.snapshot(task)
            if not any(
                n.get("package") == PACKAGE
                and n.get("resource-id") == PACKAGE + ":id/task_input"
                and n.get("text") == task
                for n in ElementTree.fromstring(typed_xml).iter("node")
            ):
                raise QualificationError("demo_input_not_proven")
            device.adb("shell", "input", "keyevent", "KEYCODE_BACK")


def write_profile(path: Path, profile: Profile) -> None:
    """Persist nonsecret settings only; refuse replacement of an existing run profile."""
    with path.open("x") as stream:
        path.chmod(0o600)
        values = asdict(profile)
        values.pop("model_ref")
        model_ref = profile.model_ref.model_dump(mode="json") if profile.model_ref else None
        for key, value in values.items():
            stream.write(
                f"{key} = {json.dumps(str(value) if isinstance(value, Path) else value)}\n"
            )
        if model_ref is not None:
            stream.write("\n[model_ref]\n")
            stream.write(f"key = {json.dumps(model_ref['key'])}\n")
            stream.write(f"revision = {model_ref['revision']}\n")


def run_local(args: argparse.Namespace) -> int:
    if bool(args.agent) != bool(args.resolved_model):
        raise QualificationError("agent_requires_resolved_model_document")
    resolved_model = None
    if args.resolved_model:
        path = Path(args.resolved_model).resolve()
        if not path.is_file() or path.stat().st_size > 1048576:
            raise QualificationError("invalid_resolved_model_document")
        resolved_model = ResolvedModel.model_validate_json(path.read_bytes(), strict=True)
    profile = Profile.load(Path(args.profile).resolve())
    profile = replace(
        profile,
        model_ref=resolved_model.reference if resolved_model else None,
        headless=args.headless or profile.headless,
    )
    # Fail before booting or fetching any secret when setup/build is missing.
    doctor(profile)
    flavor = "broken" if args.scenario == "broken" else "good"
    apk = ROOT / f"apps/qa-demo-android/app/build/outputs/apk/{flavor}/debug/app-{flavor}-debug.apk"
    if not apk.is_file():
        raise QualificationError("demo_apk_missing_run_just_device_local_build")
    attempt_id = uuid4()
    output = ROOT / ".private/artifacts/local-device"
    output.mkdir(parents=True, mode=0o700, exist_ok=True)
    profile_path = output / f"{attempt_id}.profile.toml"
    write_profile(profile_path, profile)
    request = parse_request(
        json.dumps(
            {
                "version": 1,
                "attempt_id": str(attempt_id),
                "case_id": CASE,
                "apk_path": str(apk),
                "expected_apk_sha256": sha256(apk),
                "package": PACKAGE,
                "activity": ACTIVITY,
                "serial": SERIAL,
                "profile_path": str(profile_path),
                "output_root": str(output),
                "resolved_model": (
                    resolved_model.model_dump(mode="json") if resolved_model else None
                ),
            }
        )
    )
    # Own the fixture lifecycle too; never silently reuse an unrelated listener.
    with backend("unavailable" if args.scenario == "unavailable" else "ready"):
        result = run_attempt(request, driver="minitap" if args.agent else "adb-demo")
    expected = {"good": "passed", "broken": "failed", "unavailable": "blocked"}[args.scenario]
    success = accepted(result, expected)
    summary = {
        "scenario": args.scenario,
        "expected_outcome": expected,
        "outcome": result.outcome.value,
        "reset": result.reset.value,
        "matched_expectation": success,
        "driver": "minitap" if args.agent else "adb-demo",
        "report": str(output / str(attempt_id) / "report.md"),
    }
    atomic_json(output / "latest.json", summary, replace=True)
    print(json.dumps(summary, indent=2))
    return 0 if success else 1
