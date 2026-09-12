"""Synthetic HTTP-worker evidence. It is never advertised as device qualification."""

import struct
import zlib
from pathlib import Path
from xml.sax.saxutils import escape

from mobile_qa_worker.generated.models import ExecutionJob


def png() -> bytes:
    def chunk(kind: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
        )

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", 1080, 1920, 8, 0, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress((b"\x00" * 1081) * 1920))
        + chunk(b"IEND", b"")
    )


def execute(job: ExecutionJob, directory: Path, scenario: str) -> None:
    case = job.manifest.cases[job.case_index].case
    task = escape("qa-" + str(job.attempt_id), {'"': "&quot;"})
    restarted = False
    for action in case.actions:
        if action.kind.value == "restart_app":
            restarted = True
        marker = "prerequisite_unavailable" if scenario == "blocked" else "ready_marker"
        task_node = (
            ""
            if restarted and scenario == "fail"
            else (
                f'<node package="{case.package}" '
                f'resource-id="{case.package}:id/task_row" text="{task}"/>'
            )
        )
        xml = (
            f'<hierarchy><node package="{case.package}" '
            f'resource-id="{case.package}:id/{marker}" text=""/>{task_node}</hierarchy>'
        )
        (directory / (action.checkpoint_id + ".xml")).write_text(xml)
        (directory / (action.checkpoint_id + ".png")).write_bytes(png())
