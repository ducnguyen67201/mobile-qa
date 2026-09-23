"""Pure warm policy: ambiguous DST hours follow local wall time; idle grace is monotonic."""

from dataclasses import dataclass
from datetime import datetime, time
from zoneinfo import ZoneInfo

from mobile_qa_worker.qualification.config import QualificationError


@dataclass(frozen=True)
class WarmPolicy:
    timezone: str = "UTC"
    start: str = "09:00"
    end: str = "17:00"
    scheduled: bool = False
    grace_seconds: int = 900

    def __post_init__(self) -> None:
        ZoneInfo(self.timezone)
        if self.grace_seconds < 0 or self._time(self.start) == self._time(self.end):
            raise QualificationError("invalid_warm_schedule")

    @staticmethod
    def _time(value: str) -> time:
        try:
            result = time.fromisoformat(value)
        except ValueError as exc:
            raise QualificationError("invalid_warm_schedule") from exc
        if len(value) != 5 or result.tzinfo is not None:
            raise QualificationError("invalid_warm_schedule")
        return result

    def wanted(
        self,
        now: datetime,
        idle_seconds: float | None,
        *,
        target: int,
        qualified: bool,
        draining: bool,
    ) -> bool:
        if now.tzinfo is None:
            raise QualificationError("warm_clock_requires_timezone")
        if draining or target < 1 or not qualified:
            return False
        if idle_seconds is not None and 0 <= idle_seconds < self.grace_seconds:
            return True
        if not self.scheduled:
            return False
        local = now.astimezone(ZoneInfo(self.timezone)).time().replace(tzinfo=None)
        start, end = self._time(self.start), self._time(self.end)
        return start <= local < end if start < end else local >= start or local < end
