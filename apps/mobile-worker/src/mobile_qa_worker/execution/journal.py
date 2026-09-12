"""No token is persisted. An active local attempt requires recovery, never blind replay."""

import json
import os
from pathlib import Path
from typing import cast


def write(path: Path, data: dict[str, object]) -> None:
    if path.parent != path.parent.resolve() or path.is_symlink():
        raise ValueError("unsafe_journal")
    path.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
    temp = path.with_suffix(".pending")
    fd = os.open(temp, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(fd, "w") as out:
        json.dump(data, out, sort_keys=True)
        out.flush()
        os.fsync(out.fileno())
    os.replace(temp, path)


def read(path: Path) -> dict[str, object]:
    if path.is_symlink() or path.stat().st_size > 1048576:
        raise ValueError("unsafe_journal")
    value = json.loads(path.read_text())
    if not isinstance(value, dict):
        raise ValueError("invalid_journal")
    return cast(dict[str, object], value)
