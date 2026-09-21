"""Explicit developer commands, not a long-running production worker.

`fake` emits one JSON result and exits. `sdk-import` checks only package compatibility;
only scripts/sdk_smoke.py additionally blocks socket connections during that import.
Neither command constructs an agent or executes a customer test.
Explicit device commands qualify the controlled demo; they are never run by ordinary checks.
"""

import argparse
import asyncio
import importlib
import importlib.metadata
import json
import logging
import os
from pathlib import Path
from uuid import UUID

from mobile_qa_worker.fake import execute, parse_request


def sdk_import() -> dict[str, str]:
    """Verify the pinned SDK import seam without constructing its stateful Agent.

    Telemetry must be disabled before import. This proves neither device compatibility
    nor model availability; real execution qualification is a separate milestone.
    """
    os.environ["MOBILE_USE_TELEMETRY_ENABLED"] = "false"
    version = importlib.metadata.version("minitap-mobile-use")
    if version != "4.0.0":
        raise ValueError("expected minitap-mobile-use 4.0.0")
    # Import only. Never construct Agent (which initializes telemetry).
    module = importlib.import_module("minitap.mobile_use.sdk")
    if not hasattr(module, "Agent"):
        raise ValueError("SDK does not export Agent")
    return {"distribution": "minitap-mobile-use", "version": version, "status": "imported"}


def main() -> int:
    logging.basicConfig(level=logging.INFO, format="%(levelname)s %(message)s")
    parser = argparse.ArgumentParser(description="Mobile QA local fixture adapter")
    commands = parser.add_subparsers(dest="command", required=True)
    discovery = commands.add_parser("_minitap-discovery", help=argparse.SUPPRESS)
    discovery.add_argument("--request", type=Path, required=True)
    discovery.add_argument("--profile", type=Path, required=True)
    authoring = commands.add_parser("_authoring-model", help=argparse.SUPPRESS)
    authoring.add_argument("--request", type=Path, required=True)
    authoring.add_argument("--result", type=Path, required=True)
    authoring.add_argument("--profile", type=Path, required=True)
    fake = commands.add_parser("fake", help="Run a local fixture without SDK/device/model access")
    fake.add_argument("fixture", type=Path)
    commands.add_parser("sdk-import", help="Import installed SDK only; never create an Agent")
    doctor = commands.add_parser(
        "device-doctor", help="Inspect native host acceleration and pinned tools; no boot/model"
    )
    doctor.add_argument("--profile", type=Path, required=True)
    device = commands.add_parser("device-run", help="Execute one explicit controlled attempt")
    device.add_argument("--request", type=Path, required=True)
    campaign = commands.add_parser("device-qualify", help="Run controlled qualification campaign")
    campaign.add_argument("--config", type=Path, required=True)
    recovery = commands.add_parser(
        "device-recover", help="Recover quarantined state after host reboot"
    )
    recovery.add_argument("--profile", type=Path, required=True)
    recovery.add_argument(
        "--local-result",
        type=Path,
        help="Completed ADB demo result permits explicit same-boot recovery; never agent runs",
    )
    local = commands.add_parser("device-local", help="Run the real demo phone locally")
    local.add_argument("--profile", type=Path, required=True)
    local.add_argument("--scenario", choices=["good", "broken", "unavailable"], default="good")
    local.add_argument(
        "--agent", action="store_true", help="Use Minitap; requires --resolved-model and Doppler"
    )
    local.add_argument(
        "--resolved-model",
        type=Path,
        help="Safe ResolvedModel JSON exported by the API execution task",
    )
    local.add_argument("--headless", action="store_true")
    child = commands.add_parser("_sdk-run", help=argparse.SUPPRESS)
    child.add_argument("--request", type=Path, required=True)
    child.add_argument("--result", type=Path, required=True)
    worker = commands.add_parser(
        "execution-worker", help="Poll durable jobs; real profiles explicitly launch devices"
    )
    worker.add_argument("--origin", required=True)
    worker.add_argument("--profile-id", type=UUID, required=True)
    worker.add_argument("--state", type=Path, required=True)
    worker.add_argument("--profile", type=Path)
    worker.add_argument("--scenario", choices=["pass", "fail", "blocked"], default="pass")
    worker.add_argument("--once", action="store_true")
    run = commands.add_parser("_execution-run", help=argparse.SUPPRESS)
    run.add_argument("--job", type=Path, required=True)
    run.add_argument("--directory", type=Path, required=True)
    run.add_argument("--driver", choices=["fake", "minitap", "direct"], required=True)
    run.add_argument("--profile", type=Path)
    run.add_argument("--scenario", choices=["pass", "fail", "blocked"], default="pass")
    nav = commands.add_parser("_execution-sdk", help=argparse.SUPPRESS)
    nav.add_argument("--request", type=Path, required=True)
    nav.add_argument("--result", type=Path, required=True)
    phone = commands.add_parser(
        "task-worker", help="Explicit real phone sessions and Minitap tasks"
    )
    phone.add_argument("--origin", required=True)
    phone.add_argument("--state", type=Path, required=True)
    phone.add_argument("--profile", type=Path, required=True)
    phone.add_argument("--once", action="store_true")
    args = parser.parse_args()
    if args.command == "_minitap-discovery":
        from mobile_qa_worker.authoring.minitap_discovery import run as discover

        discover(args.request, args.profile)
        return 0
    if args.command == "_authoring-model":
        from mobile_qa_worker.authoring.model_adapter import run as authoring_model

        authoring_model(args.request, args.result, args.profile)
        return 0

    try:
        if args.command == "task-worker":
            from mobile_qa_worker.task_sessions import serve as serve_tasks

            serve_tasks(args.origin, args.state, args.profile, args.once)
            return 0
        if args.command == "execution-worker":
            from mobile_qa_worker.execution.runner import serve

            serve(args.origin, args.profile_id, args.state, args.profile, args.scenario, args.once)
            return 0
        if args.command == "_execution-run":
            from mobile_qa_worker.execution.runner import dispatch_child

            dispatch_child(args.job, args.directory, args.driver, args.scenario, args.profile)
            return 0
        if args.command == "_execution-sdk":
            from mobile_qa_worker.execution.sdk_adapter import execute as navigate

            asyncio.run(navigate(args.request.resolve(), args.result.resolve()))
            return 0
        if args.command.startswith("device-") or args.command == "_sdk-run":
            from mobile_qa_worker.qualification.commands import dispatch

            return dispatch(args)
        if args.command == "sdk-import":
            print(json.dumps(sdk_import(), sort_keys=True))
        else:
            fixture = Path(str(args.fixture))
            request = parse_request(fixture.read_text())
            print(execute(request).model_dump_json())
        # Exit status reports CLI success, not whether the simulated test passed.
        # Consumers read outcome from JSON; setup/invalid input exits 2 below.
        return 0
    except (ValueError, OSError, ImportError, importlib.metadata.PackageNotFoundError) as exc:
        # Avoid dumping fixture payloads, paths or credentials into logs.
        logging.error(
            "Input or setup failed (%s). Check the fixture or installed dependency.",
            type(exc).__name__,
        )
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
