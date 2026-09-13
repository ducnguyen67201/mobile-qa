"""Build a fresh, code-only navigation index without changing assistant settings.

Only portable graph/report files are copied into Git's managed output directory.
Temporary caches, interpreter paths and visualization assets stay outside the repo.
"""

import json
import os
from pathlib import Path
import subprocess
import tempfile


ROOT = Path(__file__).resolve().parents[1]
OUTPUTS = ("graph.json", "GRAPH_REPORT.md")


def main() -> None:
    env = {**os.environ, "GRAPHIFY_QUERY_LOG_DISABLE": "1", "PYTHONHASHSEED": "0"}
    # Do not inherit an output override that could write outside the staged directory.
    env.pop("GRAPHIFY_OUT", None)
    command = [
        "uv",
        "run",
        "--project",
        str(ROOT / "tools/graphify"),
        "--frozen",
        "graphify",
    ]
    with tempfile.TemporaryDirectory(prefix="mobile-qa-graphify-") as temporary:
        staged = Path(temporary) / "graphify-out"
        subprocess.run(
            [
                *command,
                "extract",
                ".",
                "--code-only",
                "--max-workers",
                "2",
                "--out",
                temporary,
            ],
            cwd=ROOT,
            env=env,
            check=True,
        )
        subprocess.run(
            [
                *command,
                "cluster-only",
                ".",
                "--graph",
                str(staged / "graph.json"),
                "--no-label",
                "--no-viz",
            ],
            cwd=ROOT,
            env=env,
            check=True,
        )
        graph = json.loads((staged / "graph.json").read_text())
        if not graph.get("nodes") or not graph.get("built_at_commit"):
            raise ValueError(
                "Graphify produced an empty graph or omitted its source commit"
            )
        for node in graph["nodes"]:
            source = node.get("source_file")
            if source and (Path(source).is_absolute() or ".." in Path(source).parts):
                raise ValueError(
                    f"Graphify produced a nonportable source path: {source}"
                )
        destination = ROOT / "graphify-out"
        destination.mkdir(exist_ok=True)
        for name in OUTPUTS:
            contents = (staged / name).read_bytes()
            target = destination / name
            if not target.exists() or target.read_bytes() != contents:
                target.write_bytes(contents)


if __name__ == "__main__":
    main()
