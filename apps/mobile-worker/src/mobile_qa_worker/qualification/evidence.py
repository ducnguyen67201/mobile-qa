"""Private immutable attempt evidence. A trace filename is never a QA verdict."""

import hashlib
import json
import os
from collections.abc import Mapping
from pathlib import Path

from mobile_qa_worker.generated.models import QualificationResult
from mobile_qa_worker.qualification.config import QualificationError


def sha256(path: Path) -> str:
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def safe_path(root: Path, name: str) -> Path:
    path = root / name
    if not name or Path(name).is_absolute() or "\\" in name or ".." in Path(name).parts:
        raise QualificationError("unsafe_artifact_path")
    if (
        path.is_symlink()
        or path.resolve() != path.absolute()
        or not path.resolve().is_relative_to(root.resolve())
    ):
        raise QualificationError("unsafe_artifact_path")
    return path


def atomic_json(path: Path, value: object, *, replace: bool = False) -> None:
    """A completed result is write-once; mutable dirty/campaign checkpoints opt in."""
    safe_path(path.parent, path.name)
    temp = path.with_name(path.name + ".pending")
    descriptor = os.open(temp, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    try:
        with os.fdopen(descriptor, "w") as stream:
            json.dump(value, stream, indent=2, allow_nan=False)
            stream.write("\n")
            stream.flush()
            os.fsync(stream.fileno())
        if replace:
            os.replace(temp, path)
        else:
            # Hard-link publication cannot overwrite an already finalized attempt.
            os.link(temp, path)
            temp.unlink()
        parent = os.open(path.parent, os.O_RDONLY)
        try:
            os.fsync(parent)
        finally:
            os.close(parent)
    finally:
        temp.unlink(missing_ok=True)


class Evidence:
    def __init__(self, directory: Path, limit: int = 262144000):
        if directory.exists() or directory.is_symlink():
            raise QualificationError("attempt_already_exists")
        if directory != directory.resolve():
            raise QualificationError("unsafe_artifact_path")
        directory.mkdir(mode=0o700, parents=True)
        directory.chmod(0o700)
        self.directory = directory
        self.limit = limit
        self.artifacts: list[dict[str, object]] = []

    def write(self, name: str, data: bytes, mime: str) -> None:
        path = safe_path(self.directory, name)
        if not data or self.size() + len(data) > self.limit:
            raise QualificationError("evidence_budget_or_empty")
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(data)
        self.artifacts.append(
            {
                "status": "available",
                "name": name,
                "path": name,
                "mime": mime,
                "bytes": len(data),
                "sha256": sha256(path),
            }
        )

    def missing(self, name: str, reason: str) -> None:
        self.artifacts.append({"status": "unavailable", "name": name, "reason": reason})

    def size(self) -> int:
        total = 0
        for path in self.directory.rglob("*"):
            if path.is_symlink():
                raise QualificationError("unsafe_artifact_path")
            if path.is_file():
                total += path.stat().st_size
        return total

    def event(self, phase: str, reason: str) -> None:
        path = safe_path(self.directory, "events.jsonl")
        line = json.dumps({"phase": phase, "reason": reason}) + "\n"
        if self.size() + len(line) > self.limit:
            raise QualificationError("evidence_budget_exceeded")
        with path.open("a") as stream:
            path.chmod(0o600)
            stream.write(line)
            stream.flush()

    def finalize(self, payload: Mapping[str, object]) -> QualificationResult:
        result = validate_result(json.dumps(payload))
        atomic_json(self.directory / "result.json", result.model_dump(mode="json"))
        # Only controlled strings and relative filenames are rendered, no SDK prose.
        data = result.model_dump(mode="json")
        report = (
            f"# Qualification attempt {result.attempt_id}\n\n"
            f"Outcome: {data['outcome']}\n\nReason: {result.reason_code}\n\n"
            f"Reset: {data['reset']}\n\nExpected: {result.expected_behavior}\n\n"
            f"Observed: {result.observed_behavior}\n\n"
            "See result.json for evidence hashes and missing-artifact reasons.\n"
            "Raw SDK traces describe execution; their PASS labels are not this verdict.\n"
        )
        (self.directory / "report.md").write_text(report)
        (self.directory / "report.md").chmod(0o600)
        return result


def validate_result(raw: str) -> QualificationResult:
    result = QualificationResult.model_validate_json(raw, strict=True)
    data = result.model_dump(mode="json")
    if (
        result.version != 1
        or result.case_id != "persist-task-v1"
        or result.ended_at < result.started_at
    ):
        raise QualificationError("invalid_result_metadata")
    for key in ("requested_build_sha256", "model_profile_sha256", "observed_build_sha256"):
        value = data[key]
        if value is not None and (
            len(value) != 64 or any(c not in "0123456789abcdef" for c in value)
        ):
            raise QualificationError("invalid_result_hash")
    available: set[str] = set()
    for artifact in data["artifacts"]:
        if artifact["status"] == "available":
            safe_path(Path("/qualification"), artifact["path"])
            available.add(artifact["name"])
    if data["outcome"] in ("passed", "failed"):
        if not {"created.png", "created.xml", "reopened.png", "reopened.xml"} <= available:
            raise QualificationError("decisive_evidence_missing")
        if result.observed_build_sha256 != result.requested_build_sha256:
            raise QualificationError("build_not_verified")
    if any(u.unknown_calls > u.calls for u in result.usage):
        raise QualificationError("invalid_usage")
    return result
