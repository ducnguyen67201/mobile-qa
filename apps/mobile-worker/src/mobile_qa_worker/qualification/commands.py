"""Device CLI boundary, deliberately absent from the fake/import execution path."""

import argparse
import asyncio
import json
import logging
from pathlib import Path

from mobile_qa_worker.qualification.config import Profile, QualificationError, parse_request


def dispatch(args: argparse.Namespace) -> int:
    try:
        if args.command == "device-doctor":
            from mobile_qa_worker.qualification.device import doctor

            print(json.dumps(doctor(Profile.load(Path(args.profile)))))
        elif args.command == "device-run":
            from mobile_qa_worker.qualification.runner import run_attempt

            print(run_attempt(parse_request(Path(args.request).read_text())).model_dump_json())
        elif args.command == "device-local":
            from mobile_qa_worker.qualification.local import run_local

            return run_local(args)
        elif args.command == "device-qualify":
            from mobile_qa_worker.qualification.campaign import qualify

            return qualify(Path(args.config))
        elif args.command == "device-recover":
            from mobile_qa_worker.qualification.runner import recover

            recover(
                Profile.load(Path(args.profile)),
                local_result=args.local_result,
                profile_path=Path(args.profile),
            )
            print(json.dumps({"status": "recovered_requires_fresh_qualification"}))
        elif args.command == "_sdk-run":
            from mobile_qa_worker.qualification.sdk_adapter import execute

            asyncio.run(execute(Path(args.request).resolve(), Path(args.result).resolve()))
        else:
            raise QualificationError("unknown_command")
        return 0
    except QualificationError as exc:
        logging.error("Qualification stopped: %s", str(exc))
        return 2
