"""Keep lease authority alive while large transfers and cache checks block the caller."""

import threading
import time
from collections.abc import Callable
from types import TracebackType

from mobile_qa_worker.execution.client import TransportError


class PreparationStopped(ValueError):
    """The preparation ended before any device process was launched."""


class PreparationWatch:
    def __init__(self, heartbeat: Callable[[], bool], interval: float = 10) -> None:
        self.heartbeat = heartbeat
        self.interval = interval
        self.done = threading.Event()
        self.stopped = threading.Event()
        self.reason = "build_preparation_canceled"
        self.last_ack = time.monotonic()
        self.thread = threading.Thread(target=self._run, daemon=True)

    def _beat(self) -> None:
        try:
            if self.heartbeat():
                self.stopped.set()
            self.last_ack = time.monotonic()
        except TransportError as exc:
            if exc.status in (401, 403, 404, 409) or time.monotonic() - self.last_ack >= 40:
                self.reason = "lease_lost_during_preparation"
                self.stopped.set()
        except Exception:
            self.reason = "lease_lost_during_preparation"
            self.stopped.set()

    def _run(self) -> None:
        while not self.done.wait(self.interval):
            self._beat()
            if self.stopped.is_set():
                return

    def check(self) -> None:
        if self.stopped.is_set():
            raise PreparationStopped(self.reason)
        if time.monotonic() - self.last_ack >= 40:
            raise PreparationStopped("lease_lost_during_preparation")

    def __enter__(self) -> "PreparationWatch":
        self._beat()
        self.check()
        self.thread.start()
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc: BaseException | None,
        traceback: TracebackType | None,
    ) -> None:
        self.done.set()
        self.thread.join(timeout=6)
        if self.thread.is_alive():
            raise PreparationStopped("preparation_heartbeat_not_stopped")
        if exc_type is None:
            # A final in-flight heartbeat can revoke authority while __exit__ joins.
            # Recheck after it finishes, before the caller is allowed to boot Android.
            self.check()
