"""Retrieve only a Doppler bootstrap token into memory, then replace this process.

Used by the Lambda image and the EC2 service. No token or application secret is
written to a file, CLI argument, fallback cache or OpenTofu state.
"""

import json
import os
import sys
from pathlib import Path
from typing import TYPE_CHECKING, cast

import boto3
from botocore.config import Config

if TYPE_CHECKING:
    from mypy_boto3_secretsmanager import SecretsManagerClient


def command(mode: str) -> list[str]:
    if mode == "lambda":
        return ["/var/lang/bin/python", "-m", "awslambdaric", "capacity_controller.handler.handler"]
    if mode == "host":
        return [
            "/opt/mobile-qa/repo/apps/mobile-worker/.venv/bin/mobile-qa-worker",
            "host-supervisor",
            "--config",
            "/etc/mobile-qa/host.toml",
        ]
    raise ValueError("invalid_bootstrap_mode")


def main() -> None:
    mode = sys.argv[1]
    invocation = command(mode)
    if mode == "host":
        config = cast(dict[str, str], json.loads(Path("/etc/mobile-qa/bootstrap.json").read_text()))
        secret_arn, region = config["secret_arn"], config["region"]
        project, scope = config["project"], config["config"]
    else:
        secret_arn = os.environ["MOBILE_QA_BOOTSTRAP_SECRET_ARN"]
        region = os.environ["AWS_REGION"]
        project, scope = os.environ["DOPPLER_PROJECT"], os.environ["DOPPLER_CONFIG"]
    client = cast(
        "SecretsManagerClient",
        boto3.client(
            "secretsmanager",
            region_name=region,
            config=Config(
                connect_timeout=3,
                read_timeout=5,
                retries={"total_max_attempts": 2},
            ),
        ),
    )
    token = client.get_secret_value(SecretId=secret_arn).get("SecretString", "")
    if not token.startswith("dp.st.") or len(token) > 1024 or any(c.isspace() for c in token):
        raise ValueError("invalid_bootstrap_token")
    environment = {**os.environ, "DOPPLER_TOKEN": token, "DOPPLER_CONFIG_DIR": "/tmp/doppler"}
    os.execve(
        "/usr/local/bin/doppler",
        [
            "doppler",
            "run",
            "--no-fallback",
            "--forward-signals",
            "--project",
            project,
            "--config",
            scope,
            "--",
            *invocation,
        ],
        environment,
    )


if __name__ == "__main__":
    try:
        main()
    except Exception:
        # SDK exceptions can embed request metadata. The service supervisor logs
        # only this fixed failure reason and performs bounded restarts.
        print("capacity_bootstrap_failed", file=sys.stderr)
        raise SystemExit(1) from None
