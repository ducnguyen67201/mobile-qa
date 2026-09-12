"""Install the locked tool dependencies for an explicitly requested setup.

No checks, generators, servers or Doppler configuration run here. Ordinary development
uses existing dependencies; Python launchers pass --no-sync to avoid hidden installs.
"""
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
private = ROOT / ".private"
private.mkdir(mode=0o700, exist_ok=True)
private.chmod(0o700)
artifacts = private / "artifacts"
artifacts.mkdir(mode=0o700, exist_ok=True)
artifacts.chmod(0o700)
# Frozen/locked flags preserve existing resolutions. Ignore pnpm lifecycle scripts
# so installing dependencies cannot silently run checks during the authoring phase.
for command, directory in [
    (["cargo", "fetch"] + (["--locked"] if (ROOT / "Cargo.lock").exists() else []), ROOT),
    (["pnpm", "install", "--ignore-scripts"] + (["--frozen-lockfile"] if (ROOT / "apps/web/pnpm-lock.yaml").exists() else []), ROOT / "apps/web"),
    (["uv", "sync", "--extra", "sdk"] + (["--frozen"] if (ROOT / "apps/mobile-worker/uv.lock").exists() else []), ROOT / "apps/mobile-worker"),
]:
    subprocess.run(command, cwd=directory, check=True)
