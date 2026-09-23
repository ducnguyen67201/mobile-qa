"""Lambda entry point: signed hint or IAM-authorized scheduled reconciliation."""

import json
import logging
import os
import time
from collections.abc import Mapping
from datetime import UTC, datetime
from typing import Protocol, cast

from capacity_controller.auth import InvalidHint, verify_hint
from capacity_controller.aws import Power
from capacity_controller.control import Conflict, Control
from capacity_controller.reconcile import reconcile

LOG = logging.getLogger(__name__)
LOG.setLevel(logging.INFO)


class Context(Protocol):
    def get_remaining_time_in_millis(self) -> int: ...


def handler(event: Mapping[str, object], context: Context) -> dict[str, object]:
    pool_id, instance_id = os.environ["MOBILE_QA_POOL_ID"], os.environ["MOBILE_QA_INSTANCE_ID"]
    hint_id: str | None = None
    if "requestContext" in event:
        try:
            hint_id = verify_hint(
                event, os.environ["MOBILE_QA_CAPACITY_WAKE_SECRET"], pool_id, int(time.time())
            )
        except InvalidHint:
            return {"statusCode": 401, "body": '{"code":"invalid_hint"}'}
    elif event != {"source": "mobile-qa-scheduler", "pool_id": pool_id}:
        raise ValueError("invalid_invocation")
    # Reserve enough budget for bounded API calls and a single AWS action. Returning
    # early is safe: durable demand and the minute schedule remain authoritative.
    if context.get_remaining_time_in_millis() < 30000:
        raise RuntimeError("insufficient_controller_budget")
    api = Control(
        os.environ["MOBILE_QA_API_ORIGIN"], os.environ["MOBILE_QA_CAPACITY_CONTROL_TOKEN"], pool_id
    )
    try:
        if hint_id:
            headers = event["headers"]
            assert isinstance(headers, dict)
            timestamp = next(
                str(v)
                for k, v in cast(dict[str, object], headers).items()
                if k.lower() == "x-mobile-qa-timestamp"
            )
            api.consume_hint(hint_id, int(timestamp))
        outcome = reconcile(api, Power(os.environ["AWS_REGION"]), instance_id, datetime.now(UTC))
    except Conflict:
        # CAS conflicts are expected when a host heartbeat or new demand wins.
        return {"statusCode": 409, "body": '{"code":"control_conflict"}'}
    except Exception:
        LOG.error("capacity_reconcile_failed", extra={"pool_id": pool_id})
        # Do not expose exception strings; AWS and urllib errors can carry private
        # request details. Raising a fixed reason records a Lambda Errors metric.
        raise RuntimeError("capacity_reconcile_failed") from None
    if outcome == "quarantined":
        LOG.error(json.dumps({"event": "capacity_quarantined", "pool_id": pool_id}))
        raise RuntimeError("capacity_quarantined")
    LOG.info(json.dumps({"event": "capacity_reconciled", "pool_id": pool_id, "outcome": outcome}))
    return {"statusCode": 202, "body": '{"accepted":true}'}
