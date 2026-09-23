"""Bind runtime locks to the baked image; actual device qualification remains separate."""

import hashlib
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path("/opt/mobile-qa/repo")
MANIFEST = Path("/opt/mobile-qa/image-manifest.json")
LOCKS = (
    "apps/mobile-worker/uv.lock",
    "infra/device-host/toolchain.lock.json",
    "infra/aws/capacity-controller/uv.lock",
)


def digests(root: Path) -> dict[str, str]:
    return {
        name: hashlib.sha256((root / name).read_bytes()).hexdigest() for name in LOCKS
    }


def main() -> None:
    if len(sys.argv) == 4 and sys.argv[1] == "--write":
        inventory = subprocess.check_output(
            ["dpkg-query", "-W", "-f=${Package}=${Version}\n"],
            text=True,
        )
        MANIFEST.write_text(
            json.dumps(
                {
                    "git_revision": sys.argv[2],
                    "release_sha256": sys.argv[3],
                    "locks": digests(ROOT),
                    "os_packages": inventory.splitlines(),
                },
                indent=2,
            )
            + "\n"
        )
        return
    if len(sys.argv) != 1:
        raise ValueError("invalid_image_verifier_arguments")
    manifest = json.loads(MANIFEST.read_text())
    if manifest["locks"] != digests(ROOT):
        raise ValueError("image_lock_mismatch")
    if not os.access("/dev/kvm", os.R_OK | os.W_OK):
        raise ValueError("kvm_unavailable")


if __name__ == "__main__":
    main()
