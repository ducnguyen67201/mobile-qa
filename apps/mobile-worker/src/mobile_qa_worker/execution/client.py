"""Bounded, same-origin HTTP; generated Pydantic validates every JSON response."""

import json
import os
import time
import urllib.error
import urllib.request
from collections.abc import Callable
from pathlib import Path
from typing import TypeVar
from urllib.parse import urlsplit

from pydantic import BaseModel

T = TypeVar("T", bound=BaseModel)


class TransportError(Exception):
    def __init__(self, status: int):
        self.status = status
        super().__init__(f"execution_http_{status}")


class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):  # type: ignore[no-untyped-def]
        raise TransportError(code)


class Client:
    def __init__(self, origin: str, token: str):
        parsed = urlsplit(origin)
        if (
            parsed.username
            or parsed.password
            or parsed.query
            or parsed.fragment
            or parsed.path not in ("", "/")
        ):
            raise ValueError("invalid_api_origin")
        if parsed.scheme != "https" and not (
            parsed.scheme == "http" and parsed.hostname in ("127.0.0.1", "localhost")
        ):
            raise ValueError("https_required")
        if not 32 <= len(token) <= 256:
            raise ValueError("invalid_worker_token")
        self.origin = origin.rstrip("/")
        self.token = token
        self.opener = urllib.request.build_opener(NoRedirect())

    def get(self, path: str, model: type[T], lease_token: str = "") -> T:
        return model.model_validate_json(
            self.raw("GET", path, lease_token=lease_token), strict=True
        )

    def download(
        self,
        address: str,
        destination: Path,
        *,
        expected_bytes: int,
        lease_token: str,
        signed: bool,
        check: Callable[[], None],
        deadline: float,
    ) -> None:
        """Bounded binary transfer, with no credential forwarding to signed storage URLs.

        The caller maintains the lease on a separate heartbeat thread. Socket inactivity
        is limited to 60 seconds; the complete preparation has a separate deadline.
        Partial files are never returned on failure and all redirects are rejected.
        """
        if not 0 < expected_bytes <= 2 * 1024**3:
            raise ValueError("invalid_artifact_size")
        headers: dict[str, str] = {"Accept-Encoding": "identity"}
        if signed:
            parsed = urlsplit(address)
            if (
                len(address) > 16384
                or parsed.scheme != "https"
                or not parsed.hostname
                or parsed.username
                or parsed.password
                or parsed.fragment
            ):
                raise ValueError("invalid_artifact_url")
            url = address
        else:
            if not address.startswith("/api/") or address.startswith("//"):
                raise ValueError("invalid_api_path")
            url = self.origin + address
            headers.update(Authorization="Bearer " + self.token)
            headers["X-Lease-Token"] = lease_token
        request = urllib.request.Request(url, headers=headers, method="GET")
        check()
        if time.monotonic() >= deadline:
            raise ValueError("artifact_transfer_deadline")
        descriptor = os.open(
            destination, os.O_CREAT | os.O_EXCL | os.O_WRONLY | os.O_NOFOLLOW, 0o600
        )
        try:
            with os.fdopen(descriptor, "wb") as output:
                with self.opener.open(
                    request, timeout=min(60, deadline - time.monotonic())
                ) as response:
                    if response.headers.get("Content-Encoding", "identity") != "identity":
                        raise ValueError("unexpected_artifact_encoding")
                    declared = response.headers.get("Content-Length")
                    if declared is not None and int(declared) != expected_bytes:
                        raise ValueError("build_size_mismatch")
                    received = 0
                    while True:
                        check()
                        if time.monotonic() >= deadline:
                            raise ValueError("artifact_transfer_deadline")
                        chunk = response.read1(65536)
                        if not chunk:
                            break
                        received += len(chunk)
                        if received > expected_bytes:
                            raise ValueError("build_size_mismatch")
                        output.write(chunk)
                    check()
                    if received != expected_bytes:
                        raise ValueError("build_size_mismatch")
                    output.flush()
                    os.fsync(output.fileno())
        except urllib.error.HTTPError as exc:
            destination.unlink(missing_ok=True)
            raise TransportError(exc.code) from None
        except (urllib.error.URLError, TimeoutError, OSError):
            destination.unlink(missing_ok=True)
            raise TransportError(0) from None
        except BaseException:
            destination.unlink(missing_ok=True)
            raise

    def raw(
        self,
        method: str,
        path: str,
        payload: bytes | None = None,
        lease_token: str = "",
        generation: int | None = None,
        content_type: str = "application/json",
        limit: int = 1048576,
    ) -> bytes:
        if not path.startswith("/api/") or path.startswith("//"):
            raise ValueError("invalid_api_path")
        headers = {"Authorization": "Bearer " + self.token, "Content-Type": content_type}
        if lease_token:
            headers["X-Lease-Token"] = lease_token
        if generation is not None:
            headers["X-Lease-Generation"] = str(generation)
        request = urllib.request.Request(self.origin + path, payload, headers, method=method)
        try:
            # Claims may wait 30 seconds server-side; other calls keep their short
            # timeout so the heartbeat watchdog remains responsive.
            timeout = (
                35
                if method == "POST" and path in ("/api/worker/claims", "/api/worker/phone-claims")
                else 5
            )
            with self.opener.open(request, timeout=timeout) as response:
                data = response.read(limit + 1)
                if len(data) > limit:
                    raise ValueError("response_too_large")
                return data
        except urllib.error.HTTPError as exc:
            # Never retain a remote error body that may contain private state.
            raise TransportError(exc.code) from None
        except (urllib.error.URLError, TimeoutError, OSError):
            raise TransportError(0) from None

    def send(
        self,
        path: str,
        payload: BaseModel | dict[str, object],
        model: type[T],
        lease_token: str = "",
        limit: int = 1048576,
    ) -> T:
        raw = (
            payload.model_dump_json().encode()
            if isinstance(payload, BaseModel)
            else json.dumps(payload).encode()
        )
        return model.model_validate_json(
            self.raw("POST", path, raw, lease_token, limit=limit), strict=True
        )
