"""Explicit local fixture and SDK-import commands."""

import argparse
import importlib
import importlib.metadata
import json
import logging
import os
from pathlib import Path

from mobile_qa_worker.fake import execute, parse_request


def sdk_import() -> dict[str, str]:
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
    args = parser.parse_args()
    try:
        if args.command == "sdk-import":
            print(json.dumps(sdk_import(), sort_keys=True))
        else:
            fixture = Path(str(args.fixture))
            request = parse_request(fixture.read_text())
            print(execute(request).model_dump_json())
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
