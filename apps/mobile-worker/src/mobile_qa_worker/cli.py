"""Explicit developer commands, not a long-running production worker.

`fake` emits one JSON result and exits. `sdk-import` checks only package compatibility;
only scripts/sdk_smoke.py additionally blocks socket connections during that import.
Neither command constructs an agent or executes a customer test.
Explicit device commands qualify the controlled demo; they are never run by ordinary checks.
"""

import argparse
import importlib
import importlib.metadata
import json
import logging
import os
from pathlib import Path

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
        "--agent", action="store_true", help="Use Minitap; requires --model and Doppler"
    )
    local.add_argument("--model", help="Explicit OpenAI model for this agent attempt")
    local.add_argument("--headless", action="store_true")
    child = commands.add_parser("_sdk-run", help=argparse.SUPPRESS)
    child.add_argument("--request", type=Path, required=True)
    child.add_argument("--result", type=Path, required=True)
    args = parser.parse_args()
    try:
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
