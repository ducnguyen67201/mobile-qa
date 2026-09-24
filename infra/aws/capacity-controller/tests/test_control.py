import urllib.error
from uuid import uuid4

import pytest

from capacity_controller.control import Conflict, Control, ControlUnavailable, NoRedirect


@pytest.mark.parametrize(
    "origin",
    [
        "http://api.example",
        "https://u:p@api.example",
        "https://api.example/path",
        "https://api.example?token=a",
    ],
)
def test_control_rejects_credential_leaking_origins(origin):
    with pytest.raises(ValueError):
        Control(origin, "t" * 40, str(uuid4()))


def test_redirects_are_not_followed():
    assert (
        NoRedirect().redirect_request(None, None, 302, "Moved", {}, "https://evil.example") is None
    )


def test_hint_replay_is_a_conflict_and_response_is_not_exposed():
    control = Control("https://api.example", "t" * 40, str(uuid4()))

    class Replay:
        def open(self, request, timeout):
            assert timeout == 3
            assert request.get_header("Authorization") == "Bearer " + "t" * 40
            raise urllib.error.HTTPError(
                request.full_url, 409, "private-provider-details", {}, None
            )

    control.opener = Replay()
    with pytest.raises(Conflict, match="control_conflict"):
        control.consume_hint(str(uuid4()), 1800000000)


def test_failed_control_call_exposes_only_safe_reason():
    control = Control("https://api.example", "t" * 40, str(uuid4()))

    class Offline:
        def open(self, *args, **kwargs):
            raise OSError("secret request details")

    control.opener = Offline()
    with pytest.raises(ControlUnavailable, match="^control_unavailable$"):
        control.snapshot()
