"""Controlled demo qualification policy layered on owned Android operations."""

import time
from collections.abc import Callable
from pathlib import Path

from mobile_qa_worker.device.android import (  # noqa: F401
    AndroidDevice,
    assert_ports_available,
    doctor,
    host_environment,
)
from mobile_qa_worker.qualification.config import ACTIVITY, PACKAGE, Profile, QualificationError
from mobile_qa_worker.qualification.evidence import Evidence
from mobile_qa_worker.qualification.verifier import Observation, observe, validate_png

__all__ = ["Device", "doctor", "assert_ports_available", "host_environment"]


class Device(AndroidDevice):
    def __init__(self, profile: Profile, evidence: Evidence):
        super().__init__(profile, evidence, PACKAGE, ACTIVITY)

    def install(self, apk: Path, expected: str) -> str:
        digest = super().install(apk, expected)
        self.adb("reverse", "tcp:8765", "tcp:8765")
        return digest

    def snapshot(self, task: str) -> tuple[Observation, bytes, bytes]:
        foreground = self.adb("shell", "dumpsys", "activity", "activities").decode()
        if not any(
            PACKAGE in line and ("mResumedActivity" in line or "topResumedActivity" in line)
            for line in foreground.splitlines()
        ):
            raise QualificationError("wrong_foreground_package")
        from mobile_qa_worker.automation.direct import hierarchy

        xml = hierarchy(self)
        png = self.adb("exec-out", "screencap", "-p")
        validate_png(png)
        return observe(xml, task), xml, png

    def capture(
        self,
        name: str,
        task: str,
        predicate: Callable[[Observation], bool] | None = None,
        *,
        timeout: int | None = None,
    ) -> Observation:
        deadline = time.monotonic() + min(
            self.profile.assertion_seconds,
            timeout if timeout is not None else self.profile.assertion_seconds,
        )
        last: tuple[Observation, bytes, bytes] | None = None
        stable = 0
        while time.monotonic() < deadline:
            try:
                sample = self.snapshot(task)
                if sample[0].unavailable or (
                    sample[0].ready and (predicate is None or predicate(sample[0]))
                ):
                    stable = stable + 1 if last and last[0] == sample[0] else 1
                    last = sample
                    if stable >= 2:
                        self.evidence.write(name + ".xml", sample[1], "application/xml")
                        self.evidence.write(name + ".png", sample[2], "image/png")
                        return sample[0]
                else:
                    stable = 0
            except QualificationError:
                stable = 0
            time.sleep(0.3)
        raise QualificationError("observation_timeout")
