import hashlib
import hmac
import json
from uuid import uuid4

import pytest

from capacity_controller.auth import InvalidHint, verify_hint

SECRET = "s" * 40
POOL = str(uuid4())
NOW = 1800000000


def event():
    request = str(uuid4())
    body = json.dumps({"pool_id": POOL, "request_id": request})
    canonical = f"POST\n/wake\n{hashlib.sha256(body.encode()).hexdigest()}\n{NOW}\n{request}"
    return {
        "rawPath": "/wake",
        "requestContext": {"http": {"method": "POST"}},
        "headers": {
            "x-mobile-qa-timestamp": str(NOW),
            "x-mobile-qa-request-id": request,
            "x-mobile-qa-signature": hmac.new(
                SECRET.encode(), canonical.encode(), hashlib.sha256
            ).hexdigest(),
        },
        "body": body,
    }


def test_valid_signature_returns_request_id():
    hint = event()
    assert verify_hint(hint, SECRET, POOL, NOW) == hint["headers"]["x-mobile-qa-request-id"]


@pytest.mark.parametrize(
    "mutation", ["path", "method", "expired", "body", "pool", "signature", "huge"]
)
def test_invalid_hint_fails_before_replay_consumption(mutation):
    hint = event()
    now, pool = NOW, POOL
    if mutation == "path":
        hint["rawPath"] = "/other"
    elif mutation == "method":
        hint["requestContext"]["http"]["method"] = "GET"
    elif mutation == "expired":
        now += 61
    elif mutation == "body":
        hint["body"] += " "
    elif mutation == "pool":
        pool = str(uuid4())
    elif mutation == "signature":
        hint["headers"]["x-mobile-qa-signature"] = "0" * 64
    else:
        hint["body"] = "x" * 4097
    with pytest.raises(InvalidHint):
        verify_hint(hint, SECRET, pool, now)
