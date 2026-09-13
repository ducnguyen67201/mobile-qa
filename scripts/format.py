"""Explicit repository formatting; generators retain ownership of generated output."""

import argparse
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
PRETTIER_EXTENSIONS = {
    ".js",
    ".jsx",
    ".mjs",
    ".cjs",
    ".ts",
    ".tsx",
    ".css",
    ".scss",
    ".html",
    ".json",
    ".jsonc",
    ".yaml",
    ".yml",
    ".md",
    ".mdx",
}
GENERATED = (
    "graphify-out/",
    "apps/web/src/api/generated/",
    "apps/mobile-worker/src/mobile_qa_worker/generated/",
    "contracts/browser.openapi.json",
    "contracts/worker.schema.json",
)


def run(command):
    subprocess.run(command, cwd=ROOT, check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    output = subprocess.check_output(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard"],
        cwd=ROOT,
    )
    paths = sorted({p.decode() for p in output.split(b"\0") if p})
    paths = [p for p in paths if (ROOT / p).is_file() and not p.startswith(GENERATED)]
    web = [
        p
        for p in paths
        if Path(p).suffix in PRETTIER_EXTENSIONS and not p.endswith("pnpm-lock.yaml")
    ]
    python = [p for p in paths if p.endswith(".py")]
    run(["cargo", "fmt", "--all", *(["--", "--check"] if args.check else [])])
    # Use the locked local binary and root config; no transient package downloads.
    for offset in range(0, len(web), 100):
        run(
            [
                str(ROOT / "apps/web/node_modules/.bin/prettier"),
                "--config",
                ".prettierrc.json",
                "--ignore-path",
                ".prettierignore",
                "--check" if args.check else "--write",
                *web[offset : offset + 100],
            ]
        )
    run(
        [
            "uv",
            "run",
            "--no-sync",
            "--project",
            "apps/mobile-worker",
            "--frozen",
            "ruff",
            "format",
            *(["--check"] if args.check else []),
            *python,
        ]
    )


if __name__ == "__main__":
    main()
