"""Authenticate a wake hint before any API or AWS action. Replay lives in the API DB."""

import base64
import hashlib
import hmac
import re
from collections.abc import Mapping
from typing import cast
from uuid import UUID

from mobile_qa_worker.generated.models import CapacityHint


class InvalidHint(ValueError):
    """Public error carries no request body, secret or provider output."""


def verify_hint(event: Mapping[str, object], secret: str, pool_id: str, now: int) -> str:
    if len(secret) < 32 or event.get("rawPath") != "/wake":
        raise InvalidHint("invalid_hint")
    context = event.get("requestContext")
    if not isinstance(context, dict):
        raise InvalidHint("invalid_hint")
    http = cast(dict[str, object], context).get("http")
    if not isinstance(http, dict):
        raise InvalidHint("invalid_hint")
    if cast(dict[str, object], http).get("method") != "POST":
        raise InvalidHint("invalid_hint")
    headers_raw = event.get("headers")
    if not isinstance(headers_raw, dict):
        raise InvalidHint("invalid_hint")
    headers = {k.lower(): str(v) for k, v in cast(dict[str, object], headers_raw).items()}
    stamp = headers.get("x-mobile-qa-timestamp", "")
    request_id = headers.get("x-mobile-qa-request-id", "")
    signature = headers.get("x-mobile-qa-signature", "")
    if not re.fullmatch(r"[0-9]{10}", stamp) or abs(now - int(stamp)) > 60:
        raise InvalidHint("expired_hint")
    try:
        if str(UUID(request_id)) != request_id:
            raise ValueError
    except ValueError as error:
        raise InvalidHint("invalid_hint") from error
    raw = event.get("body")
    if not isinstance(raw, str) or len(raw) > 4096:
        raise InvalidHint("invalid_hint")
    try:
        body = (
            base64.b64decode(raw, validate=True) if event.get("isBase64Encoded") else raw.encode()
        )
        if len(body) > 1024:
            raise ValueError
        canonical = f"POST\n/wake\n{hashlib.sha256(body).hexdigest()}\n{stamp}\n{request_id}"
        expected = hmac.new(secret.encode(), canonical.encode(), hashlib.sha256).hexdigest()
        if not hmac.compare_digest(expected, signature):
            raise InvalidHint("invalid_hint")
        value = CapacityHint.model_validate_json(body, strict=True)
        if str(value.pool_id) != pool_id or str(value.request_id) != request_id:
            raise InvalidHint("invalid_hint")
    except (ValueError, UnicodeError) as error:
        raise InvalidHint("invalid_hint") from error
    return request_id
