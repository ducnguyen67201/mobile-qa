"""Bounded, same-origin HTTP; generated Pydantic validates every JSON response."""

import json
import urllib.error
import urllib.request
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
            with self.opener.open(request, timeout=5) as response:
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
    ) -> T:
        raw = (
            payload.model_dump_json().encode()
            if isinstance(payload, BaseModel)
            else json.dumps(payload).encode()
        )
        return model.model_validate_json(self.raw("POST", path, raw, lease_token), strict=True)
