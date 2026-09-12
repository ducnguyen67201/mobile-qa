import struct
import zlib

import pytest

from mobile_qa_worker.qualification.config import QualificationError
from mobile_qa_worker.qualification.verifier import Observation, observe, validate_png, verdict


def xml(task="qa-unique", ready=True, unavailable=False):
    nodes = ""
    for identity, text in [
        ("ready_marker" if ready else "loading", ""),
        ("prerequisite_unavailable" if unavailable else "other", ""),
        ("task_row", task),
    ]:
        nodes += (
            '<node package="ai.mobileqa.demo" '
            f'resource-id="ai.mobileqa.demo:id/{identity}" text="{text}" />'
        )
    return ("<hierarchy>" + nodes + "</hierarchy>").encode()


def png():
    def chunk(kind, data):
        return (
            struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))
        )

    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", 1080, 1920, 8, 0, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(b"\0" * (1081 * 1920)))
        + chunk(b"IEND", b"")
    )


@pytest.mark.parametrize(
    "created,reopened,expected",
    [
        (Observation(True, False, True), Observation(True, False, True), "passed"),
        (Observation(True, False, True), Observation(True, False, False), "failed"),
        (Observation(True, False, False), Observation(True, False, False), "inconclusive"),
        (Observation(True, False, True), Observation(False, True, False), "blocked"),
        (Observation(True, False, True), Observation(False, False, False), "inconclusive"),
    ],
)
def test_business_oracle_ignores_sdk_claim(created, reopened, expected):
    assert verdict(created, reopened)[0] == expected


def test_package_scoped_exact_task():
    assert observe(xml(), "qa-unique").task_present
    assert not observe(xml(), "unique").task_present
    assert observe(xml(unavailable=True), "qa-unique").unavailable
    with pytest.raises(QualificationError):
        observe(xml().replace(b"ai.mobileqa.demo", b"other.app"), "qa-unique")


@pytest.mark.parametrize(
    "data", [b"", b"<hierarchy>", b"<foo/>", b"<!DOCTYPE x><hierarchy/>", b"x" * 2097153]
)
def test_bad_hierarchy_inconclusive(data):
    with pytest.raises(QualificationError):
        observe(data, "task")


def test_screenshot_integrity():
    validate_png(png())
    for bad in (png()[:-1], b"not png", png()[:50] + b"x" + png()[51:]):
        with pytest.raises(QualificationError):
            validate_png(bad)
