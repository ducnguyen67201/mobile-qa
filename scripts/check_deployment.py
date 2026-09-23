"""Explicit configuration validation; never apply, plan against AWS, build AMIs or deploy.

Provider/plugin installation contacts their public registries. Validation runs with
AWS credentials removed and backend initialization disabled. Run only after coding.
"""

import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
POOL = ROOT / "infra/aws/device-pool"
AMI = ROOT / "infra/device-host/ami"
TOFU_VERSION = "1.12.6"
PACKER_VERSION = "1.16.1"


def offline_environment() -> dict[str, str]:
    environment = {
        key: value
        for key, value in os.environ.items()
        if not key.startswith(
            ("AWS_", "DOPPLER_", "TF_VAR_", "TF_CLI_ARGS", "PKR_VAR_")
        )
    }
    environment["AWS_EC2_METADATA_DISABLED"] = "true"
    # Even a locally configured default AWS profile must never be consulted.
    environment["AWS_SHARED_CREDENTIALS_FILE"] = os.devnull
    environment["AWS_CONFIG_FILE"] = os.devnull
    return environment


def assert_nested_virtualization(schema: dict) -> None:
    providers = schema["provider_schemas"]
    keys = [
        key
        for key in (
            "registry.terraform.io/hashicorp/aws",
            "registry.opentofu.org/hashicorp/aws",
        )
        if key in providers
    ]
    if len(keys) != 1:
        raise ValueError("Expected exactly one approved AWS provider namespace")
    provider = providers[keys[0]]
    cpu = provider["resource_schemas"]["aws_instance"]["block"]["block_types"][
        "cpu_options"
    ]
    if "nested_virtualization" not in cpu["block"]["attributes"]:
        raise ValueError("AWS provider does not expose required nested virtualization")


def provider_schema_configuration(source: str) -> str:
    """Keep the exact version/provider requirements, excluding operational backend.

    The module deliberately has one empty backend declaration. Refuse a changed
    layout instead of attempting to initialize or migrate any remote state.
    """
    requirements, separator, _ = source.partition('\nprovider "aws"')
    if not separator:
        raise ValueError("Cannot isolate provider requirements")
    result, removed = re.subn(r'\bbackend\s+"s3"\s*\{\s*\}', "", requirements)
    if removed != 1:
        raise ValueError("Expected one empty operational S3 backend declaration")
    return result


def check() -> None:
    missing = [name for name in ("tofu", "packer") if shutil.which(name) is None]
    if missing:
        raise RuntimeError(
            f"Install OpenTofu {TOFU_VERSION} and Packer {PACKER_VERSION}; missing: {', '.join(missing)}"
        )
    environment = offline_environment()
    plugin_root = ROOT / ".private/deployment-tools/packer-plugins"
    plugin_root.mkdir(parents=True, exist_ok=True)
    environment["PACKER_PLUGIN_PATH"] = str(plugin_root)

    def run(arguments: list[str], *, capture: bool = False) -> str:
        result = subprocess.run(
            arguments,
            cwd=ROOT,
            env=environment,
            check=True,
            text=True,
            stdout=subprocess.PIPE if capture else None,
        )
        return result.stdout or ""

    tofu = ["tofu", f"-chdir={POOL}"]
    run([*tofu, "fmt", "-check"])
    run([*tofu, "init", "-backend=false", "-lockfile=readonly", "-input=false"])
    run([*tofu, "validate"])
    with tempfile.TemporaryDirectory(prefix="mobile-qa-provider-schema-") as directory:
        schema_root = Path(directory)
        (schema_root / "versions.tf").write_text(
            provider_schema_configuration((POOL / "versions.tf").read_text())
        )
        shutil.copy2(POOL / ".terraform.lock.hcl", schema_root / ".terraform.lock.hcl")
        (schema_root / ".terraform").mkdir()
        (schema_root / ".terraform/providers").symlink_to(POOL / ".terraform/providers")
        isolated = ["tofu", f"-chdir={schema_root}"]
        run([*isolated, "init", "-backend=false", "-lockfile=readonly", "-input=false"])
        assert_nested_virtualization(
            json.loads(run([*isolated, "providers", "schema", "-json"], capture=True))
        )
    run(["packer", "fmt", "-check", str(AMI)])
    # Packer validates its plugin schema with synthetic nonsecret identifiers.
    # No builder or provisioner executes, and no IAM credential is loaded.
    run(["packer", "init", str(AMI)])
    with tempfile.TemporaryDirectory(prefix="mobile-qa-deployment-check-") as directory:
        release = Path(directory) / "release.tar"
        release.write_bytes(b"not-a-release; validation-does-not-build")
        values = {
            "region": "us-east-1",
            "source_ami": "ami-0123456789abcdef0",
            "builder_subnet_id": "subnet-0123456789abcdef0",
            "builder_security_group_id": "sg-0123456789abcdef0",
            "release_archive": str(release),
            "release_sha256": "0" * 64,
            "git_revision": "0" * 40,
        }
        run(
            [
                "packer",
                "validate",
                *[f"-var={key}={value}" for key, value in values.items()],
                str(AMI),
            ]
        )
    if platform.system() == "Linux":
        with tempfile.TemporaryDirectory(
            prefix="mobile-qa-launcher-check-"
        ) as directory:
            run(
                [
                    "cc",
                    "-std=c11",
                    "-Wall",
                    "-Wextra",
                    "-Werror",
                    "-O2",
                    str(ROOT / "infra/device-host/network/launch-emulator.c"),
                    "-o",
                    str(Path(directory) / "launcher"),
                ]
            )
    else:
        print(
            "Linux launcher compilation skipped on this platform; Linux CI and hosted gate are required."
        )
    for script in (
        "infra/deployment/install_runtime.sh",
        "infra/device-host/ami/provision.sh",
        "infra/aws/device-pool/user-data.sh.tftpl",
        "infra/device-host/network/install.sh",
    ):
        run(["bash", "-n", str(ROOT / script)])
    print(
        "Deployment definitions validated. Images, AWS integration and device isolation remain release gates."
    )


if __name__ == "__main__":
    try:
        check()
    except (RuntimeError, ValueError, subprocess.CalledProcessError) as error:
        print(str(error), file=sys.stderr)
        raise SystemExit(1) from None
