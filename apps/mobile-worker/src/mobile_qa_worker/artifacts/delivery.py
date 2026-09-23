"""Lease-authorized immutable build delivery shared by execution and phone workers."""

import time
from collections.abc import Callable, Iterator
from contextlib import contextmanager
from pathlib import Path
from uuid import UUID

from mobile_qa_worker.artifacts.cache import Cache, installed_source
from mobile_qa_worker.execution.client import Client, TransportError
from mobile_qa_worker.generated.models import BuildDelivery


def _authorized(
    client: Client,
    endpoint: str,
    lease_token: str,
    app_id: UUID,
    digest: str,
    size: int,
) -> BuildDelivery:
    delivery = client.get(endpoint, BuildDelivery, lease_token)
    if (
        delivery.app_id != app_id
        or delivery.sha256 != digest
        or delivery.byte_size != size
        or delivery.headers
    ):
        raise ValueError("artifact_delivery_identity_mismatch")
    return delivery


@contextmanager
def prepared_build(
    client: Client,
    *,
    endpoint: str,
    lease_token: str,
    app_id: UUID,
    digest: str,
    size: int,
    state: Path,
    target: Path,
    check: Callable[[], None],
) -> Iterator[Path]:
    """A cache hit still requires current server authorization for this exact build.

    Interrupted transfers restart from zero with at most two retries. We do not
    trust a partial response as a range-resume identity or persist signed URLs.
    """
    check()
    delivery = _authorized(client, endpoint, lease_token, app_id, digest, size)
    deadline = time.monotonic() + 30 * 60

    def check_preparation() -> None:
        check()
        if time.monotonic() >= deadline:
            raise ValueError("artifact_transfer_deadline")

    def load(partial: Path) -> None:
        capability = delivery
        for attempt in range(3):
            check_preparation()
            try:
                client.download(
                    capability.url,
                    partial,
                    expected_bytes=size,
                    lease_token=lease_token,
                    signed=capability.authentication.value == "signed_url",
                    check=check_preparation,
                    deadline=deadline,
                )
                return
            except TransportError as exc:
                if attempt == 2 or exc.status not in (0, 403, 408, 429, 500, 502, 503, 504):
                    raise
                check_preparation()
                capability = _authorized(client, endpoint, lease_token, app_id, digest, size)
        raise AssertionError("unreachable_artifact_retry")

    with Cache.from_environment(state).acquire(
        app_id, digest, size, load, check_preparation
    ) as cached:
        with installed_source(cached, target) as source:
            yield source
