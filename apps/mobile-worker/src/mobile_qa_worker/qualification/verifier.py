"""Deterministic oracle for the controlled persistence demo, not arbitrary customer apps."""

import struct
import zlib
from dataclasses import dataclass
from xml.etree import ElementTree

from mobile_qa_worker.qualification.config import PACKAGE, QualificationError


@dataclass(frozen=True)
class Observation:
    ready: bool
    unavailable: bool
    task_present: bool


def validate_png(data: bytes) -> None:
    # Validate framing/CRC and dimensions without importing a device/model/image SDK.
    if len(data) > 16777216 or not data.startswith(b"\x89PNG\r\n\x1a\n"):
        raise QualificationError("invalid_screenshot")
    offset = 8
    kinds: list[bytes] = []
    compressed = bytearray()
    row_bytes = 0
    while offset + 12 <= len(data):
        size = struct.unpack(">I", data[offset : offset + 4])[0]
        chunk = data[offset + 4 : offset + 8 + size]
        end = offset + 12 + size
        if end > len(data) or zlib.crc32(chunk) != struct.unpack(">I", data[end - 4 : end])[0]:
            raise QualificationError("invalid_screenshot")
        kind = chunk[:4]
        kinds.append(kind)
        if kind == b"IHDR":
            if size != 13:
                raise QualificationError("invalid_screenshot")
            width, height = struct.unpack(">II", chunk[4:12])
            if width != 1080 or height != 1920:
                raise QualificationError("wrong_screenshot_dimensions")
            depth, color, compression, filtering, interlace = chunk[12:17]
            if (
                depth != 8
                or color not in (0, 2, 6)
                or (compression, filtering, interlace) != (0, 0, 0)
            ):
                raise QualificationError("unsupported_screenshot_encoding")
            row_bytes = 1 + width * {0: 1, 2: 3, 6: 4}[color]
        if kind == b"IDAT":
            compressed.extend(chunk[4:])
        offset = end
        if kind == b"IEND":
            break
    if (
        offset != len(data)
        or not kinds
        or kinds[0] != b"IHDR"
        or kinds[-1] != b"IEND"
        or b"IDAT" not in kinds
    ):
        raise QualificationError("invalid_screenshot")

    try:
        decoder = zlib.decompressobj()
        pixels = decoder.decompress(compressed, 16777217)
        if (
            not decoder.eof
            or len(pixels) != row_bytes * 1920
            or any(pixels[i] > 4 for i in range(0, len(pixels), row_bytes))
        ):
            raise QualificationError("invalid_screenshot_pixels")
    except zlib.error as exc:
        raise QualificationError("invalid_screenshot_pixels") from exc


def observe(xml: bytes, task: str) -> Observation:
    if not xml or len(xml) > 2097152 or b"<!DOCTYPE" in xml.upper() or b"<!ENTITY" in xml.upper():
        raise QualificationError("invalid_hierarchy")
    try:
        root = ElementTree.fromstring(xml)
    except ElementTree.ParseError as exc:
        raise QualificationError("invalid_hierarchy") from exc
    if root.tag != "hierarchy":
        raise QualificationError("invalid_hierarchy")
    nodes = list(root.iter("node"))
    if len(nodes) > 10000:
        raise QualificationError("invalid_hierarchy")
    owned = [n for n in nodes if n.get("package") == PACKAGE]
    if not owned:
        raise QualificationError("wrong_foreground_package")
    ids = {n.get("resource-id") for n in owned}
    return Observation(
        PACKAGE + ":id/ready_marker" in ids,
        PACKAGE + ":id/prerequisite_unavailable" in ids,
        any(
            n.get("text") == task and n.get("resource-id") == PACKAGE + ":id/task_row"
            for n in owned
        ),
    )


def verdict(created: Observation, reopened: Observation) -> tuple[str, str]:
    if created.unavailable or reopened.unavailable:
        return "blocked", "prerequisite_unavailable"
    if not created.ready or not created.task_present:
        return "inconclusive", "creation_not_proven"
    if not reopened.ready:
        return "inconclusive", "reopen_not_proven"
    if reopened.task_present:
        return "passed", "persistence_observed"
    return "failed", "persistence_lost"
