"""Lease-scoped capabilities, bounded transfers, and cancellation without external I/O."""

import hashlib
import threading
import time
import tracemalloc
import urllib.error
from datetime import UTC, datetime, timedelta
from types import SimpleNamespace
from uuid import uuid4

import pytest

from mobile_qa_worker.artifacts.cache import Cache
from mobile_qa_worker.artifacts.delivery import prepared_build
from mobile_qa_worker.artifacts.preparation import PreparationStopped, PreparationWatch
from mobile_qa_worker.execution.client import Client, TransportError
from mobile_qa_worker.generated.models import BuildDelivery


class Stream:
    def __init__(self, size, *, declared=None):
        self.remaining = size
        self.headers = {"Content-Length": str(size if declared is None else declared)}
        self.largest_read = 0

    def __enter__(self):
        return self

    def __exit__(self, *_):
        pass

    def read1(self, limit):
        self.largest_read = max(self.largest_read, limit)
        size = min(limit, self.remaining)
        self.remaining -= size
        return b"a" * size


def download(client, target, size, *, signed=False, check=lambda: None, deadline=None):
    client.download(
        "https://bucket.example/apk?signature=private" if signed else "/api/build",
        target,
        expected_bytes=size,
        lease_token="lease-private",
        signed=signed,
        check=check,
        deadline=deadline or time.monotonic() + 10,
    )


def test_large_stream_has_constant_memory_and_no_signed_credential_forwarding(tmp_path):
    client, seen = Client("https://api.example", "x" * 64), []
    size = 16 * 1024**2
    stream = Stream(size)

    def open_request(request, timeout):
        seen.append(request)
        assert timeout <= 60
        return stream

    client.opener = SimpleNamespace(open=open_request)
    tracemalloc.start()
    try:
        download(client, tmp_path / "build", size, signed=True)
        _, peak = tracemalloc.get_traced_memory()
    finally:
        tracemalloc.stop()
    assert peak < 1024**2
    assert stream.largest_read == 65536
    assert (tmp_path / "build").stat().st_size == size
    assert "Authorization" not in seen[0].headers
    assert "X-lease-token" not in seen[0].headers
    assert seen[0].headers["Accept-encoding"] == "identity"


def test_local_stream_keeps_only_same_origin_lease_authentication(tmp_path):
    client, seen = Client("http://127.0.0.1:1234", "x" * 64), []

    def open_request(request, **_):
        seen.append(request)
        return Stream(3)

    client.opener = SimpleNamespace(open=open_request)
    download(client, tmp_path / "build", 3)
    assert seen[0].full_url == "http://127.0.0.1:1234/api/build"
    assert seen[0].headers["Authorization"] == "Bearer " + "x" * 64
    assert seen[0].headers["X-lease-token"] == "lease-private"


@pytest.mark.parametrize("size,declared", [(2, 3), (4, 3), (3, 4)])
def test_truncated_oversized_and_misdeclared_streams_never_publish(tmp_path, size, declared):
    client = Client("https://api.example", "x" * 64)
    client.opener = SimpleNamespace(open=lambda *_a, **_k: Stream(size, declared=declared))
    target = tmp_path / "build"
    with pytest.raises(ValueError, match="build_size_mismatch"):
        download(client, target, 3)
    assert not target.exists()


def test_transfer_cancellation_and_timeout_remove_partial(tmp_path):
    client = Client("https://api.example", "x" * 64)
    client.opener = SimpleNamespace(open=lambda *_a, **_k: Stream(100000))
    checks = []

    def check():
        checks.append(True)
        if len(checks) == 3:
            raise PreparationStopped("build_preparation_canceled")

    with pytest.raises(PreparationStopped):
        download(client, tmp_path / "build", 100000, check=check)
    assert not (tmp_path / "build").exists()
    with pytest.raises(ValueError, match="artifact_transfer_deadline"):
        download(client, tmp_path / "late", 1, deadline=time.monotonic() - 1)
    assert not (tmp_path / "late").exists()


def test_http_failure_hides_sensitive_url_and_body(tmp_path):
    client = Client("https://api.example", "x" * 64)

    def fail(*_a, **_k):
        raise urllib.error.HTTPError("https://bucket?secret", 403, "private-body", {}, None)

    client.opener = SimpleNamespace(open=fail)
    with pytest.raises(TransportError) as error:
        download(client, tmp_path / "build", 3, signed=True)
    assert str(error.value) == "execution_http_403"
    assert not (tmp_path / "build").exists()


@pytest.mark.parametrize(
    "url", ["http://bucket/apk", "https://user:pass@bucket/apk", "file:///apk"]
)
def test_signed_url_validation_before_open(tmp_path, url):
    client = Client("https://api.example", "x" * 64)
    client.opener = SimpleNamespace(open=lambda *_a, **_k: pytest.fail("unsafe request sent"))
    with pytest.raises(ValueError, match="invalid_artifact_url"):
        client.download(
            url,
            tmp_path / "build",
            expected_bytes=3,
            lease_token="private",
            signed=True,
            check=lambda: None,
            deadline=time.monotonic() + 1,
        )


def capability(app, data=b"apk", *, authentication="signed_url"):
    return BuildDelivery.model_validate(
        {
            "app_id": str(app),
            "url": "https://bucket.example/apk?signature=private",
            "sha256": hashlib.sha256(data).hexdigest(),
            "byte_size": len(data),
            "headers": {},
            "authentication": authentication,
            "expires_at": (datetime.now(UTC) + timedelta(minutes=10)).isoformat(),
        }
    )


def test_cache_hit_is_authorized_again_and_both_consumers_share_bytes(tmp_path, monkeypatch):
    store, app = Cache(tmp_path / "cache", max_bytes=100, safety_bytes=0), uuid4()
    monkeypatch.setattr(Cache, "from_environment", lambda _: store)
    calls, downloads = [], []
    reply = capability(app)

    class FakeClient:
        def get(self, endpoint, model, token):
            calls.append((endpoint, token))
            assert model is BuildDelivery
            return reply

        def download(self, _, target, **kwargs):
            downloads.append(kwargs)
            target.write_bytes(b"apk")

    for endpoint in [
        "/api/worker/attempts/one/build/delivery",
        "/api/worker/phones/two/build/delivery",
    ]:
        with prepared_build(
            FakeClient(),
            endpoint=endpoint,
            lease_token="private",
            app_id=app,
            digest=reply.sha256,
            size=3,
            state=tmp_path,
            target=tmp_path / "build.apk",
            check=lambda: None,
        ) as path:
            assert path.read_bytes() == b"apk"
    assert len(calls) == 2
    assert len(downloads) == 1
    assert downloads[0]["signed"] is True
    assert not (tmp_path / "build.apk").exists()


def test_expired_capability_refreshes_authorization_and_bounded_retry(tmp_path, monkeypatch):
    store, app = Cache(tmp_path / "cache", max_bytes=100, safety_bytes=0), uuid4()
    monkeypatch.setattr(Cache, "from_environment", lambda _: store)
    calls, downloads = [], []
    reply = capability(app)

    class FakeClient:
        def get(self, *_):
            calls.append(True)
            return reply

        def download(self, _, target, **kwargs):
            downloads.append(True)
            if len(downloads) == 1:
                raise TransportError(403)
            target.write_bytes(b"apk")

    with prepared_build(
        FakeClient(),
        endpoint="/api/delivery",
        lease_token="private",
        app_id=app,
        digest=reply.sha256,
        size=3,
        state=tmp_path,
        target=tmp_path / "build.apk",
        check=lambda: None,
    ):
        pass
    assert len(calls) == len(downloads) == 2


def test_delivery_identity_mismatch_rejected_before_cache_access(tmp_path):
    app, reply = uuid4(), capability(uuid4())
    client = SimpleNamespace(get=lambda *_: reply)
    with pytest.raises(ValueError, match="identity_mismatch"):
        with prepared_build(
            client,
            endpoint="/api/delivery",
            lease_token="private",
            app_id=app,
            digest=reply.sha256,
            size=3,
            state=tmp_path,
            target=tmp_path / "build.apk",
            check=lambda: None,
        ):
            pytest.fail("downloaded another app's bytes")


def test_background_heartbeats_continue_while_preparation_blocks_and_cancel():
    called = threading.Event()
    count = 0

    def heartbeat():
        nonlocal count
        count += 1
        if count == 2:
            called.set()
            return True
        return False

    with pytest.raises(PreparationStopped, match="build_preparation_canceled"):
        with PreparationWatch(heartbeat, interval=0.01) as watch:
            assert called.wait(1)
            with pytest.raises(PreparationStopped, match="build_preparation_canceled"):
                watch.check()
    assert not watch.thread.is_alive()


def test_preparation_rejects_lost_lease_before_download():
    def heartbeat():
        raise TransportError(409)

    with pytest.raises(PreparationStopped, match="lease_lost"):
        with PreparationWatch(heartbeat):
            pytest.fail("prepared under revoked lease")


def test_cancellation_in_last_inflight_heartbeat_blocks_device_handoff():
    entered, release = threading.Event(), threading.Event()
    count = 0

    def heartbeat():
        nonlocal count
        count += 1
        if count == 2:
            entered.set()
            assert release.wait(1)
            return True
        return False

    with pytest.raises(PreparationStopped, match="canceled"):
        with PreparationWatch(heartbeat, interval=0.01) as watch:
            assert entered.wait(1)
            watch.check()
            release.set()
            # __exit__ joins the response that cancels this lease after the last check.
