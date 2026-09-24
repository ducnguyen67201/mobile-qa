"""Control API adapter: generated contracts, fixed origin and bounded response sizes."""

import urllib.error
import urllib.request
from typing import TypeVar
from urllib.parse import urlsplit
from uuid import UUID, uuid4

from mobile_qa_worker.generated.models import (
    CapacityAction,
    CapacityActionRequest,
    CapacitySnapshot,
    ConsumeHintRequest,
    HintReceipt,
    ObservedPower,
)
from pydantic import BaseModel

T = TypeVar("T", bound=BaseModel)


class ControlUnavailable(RuntimeError):
    """A safe reason code; never interpolate network response bodies or credentials."""


class Conflict(ControlUnavailable):
    """Another actor changed admission state, or a hint was already consumed."""


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, *args: object, **kwargs: object) -> None:
        return None


class Control:
    def __init__(self, origin: str, token: str, pool_id: str) -> None:
        parsed = urlsplit(origin)
        if (
            parsed.scheme != "https"
            or not parsed.hostname
            or parsed.username
            or parsed.password
            or parsed.path
            or parsed.query
            or parsed.fragment
        ):
            raise ValueError("invalid_control_origin")
        if len(token) < 32:
            raise ValueError("invalid_control_token")
        self.base = f"{origin}/api/internal/capacity/pools/{UUID(pool_id)}"
        self.token = token
        self.opener = urllib.request.build_opener(NoRedirect())

    def _request(self, suffix: str, result: type[T], body: BaseModel | None = None) -> T:
        request = urllib.request.Request(
            self.base + suffix,
            data=body.model_dump_json().encode() if body is not None else None,
            headers={"Authorization": f"Bearer {self.token}", "Content-Type": "application/json"},
            method="POST" if body is not None else "GET",
        )
        try:
            with self.opener.open(request, timeout=3) as response:
                data = response.read(262145)
            if len(data) > 262144:
                raise ControlUnavailable("control_response_too_large")
            return result.model_validate_json(data, strict=True)
        except urllib.error.HTTPError as error:
            error.close()
            if error.code == 409:
                raise Conflict("control_conflict") from None
            raise ControlUnavailable("control_http_failure") from None
        except (OSError, ValueError):
            raise ControlUnavailable("control_unavailable") from None

    def snapshot(self) -> CapacitySnapshot:
        return self._request("", CapacitySnapshot)

    def consume_hint(self, request_id: str, timestamp: int) -> None:
        receipt = self._request(
            "/hints",
            HintReceipt,
            ConsumeHintRequest(
                request_id=UUID(request_id),
                timestamp=timestamp,
            ),
        )
        if not receipt.accepted:
            raise Conflict("replayed_hint")

    def action(
        self,
        snapshot: CapacitySnapshot,
        action: CapacityAction,
        observed: ObservedPower | None = None,
        reason: str | None = None,
    ) -> CapacitySnapshot:
        return self._request(
            "/actions",
            CapacitySnapshot,
            CapacityActionRequest(
                request_id=uuid4(),
                action=action,
                host_id=snapshot.host_id,
                expected_control_version=snapshot.control_version,
                observed_power=observed,
                operation_id=snapshot.current_operation.id if snapshot.current_operation else None,
                reason=reason,
            ),
        )
