import pytest
from test_auth import NOW, POOL, SECRET, event

from capacity_controller import handler
from capacity_controller.control import Conflict


class Context:
    def get_remaining_time_in_millis(self):
        return 45000


@pytest.fixture(autouse=True)
def configuration(monkeypatch):
    for key, value in {
        "MOBILE_QA_POOL_ID": POOL,
        "MOBILE_QA_INSTANCE_ID": "i-test",
        "MOBILE_QA_CAPACITY_WAKE_SECRET": SECRET,
        "MOBILE_QA_API_ORIGIN": "https://api.example",
        "MOBILE_QA_CAPACITY_CONTROL_TOKEN": "t" * 40,
        "AWS_REGION": "us-east-1",
    }.items():
        monkeypatch.setenv(key, value)
    monkeypatch.setattr(handler.time, "time", lambda: NOW)


def test_invalid_hint_never_constructs_clients(monkeypatch):
    monkeypatch.setattr(handler, "Control", lambda *args: pytest.fail("authenticated first"))
    invalid = event()
    invalid["headers"]["x-mobile-qa-signature"] = "bad"
    assert handler.handler(invalid, Context())["statusCode"] == 401


def test_replayed_hint_never_touches_aws(monkeypatch):
    class Replay:
        def __init__(self, *args):
            pass

        def consume_hint(self, *args):
            raise Conflict("replay")

    monkeypatch.setattr(handler, "Control", Replay)
    monkeypatch.setattr(handler, "Power", lambda *args: pytest.fail("replayed hint"))
    assert handler.handler(event(), Context())["statusCode"] == 409


def test_untrusted_schedule_shape_rejected(monkeypatch):
    monkeypatch.setattr(handler, "Control", lambda *args: pytest.fail("invalid scheduler"))
    with pytest.raises(ValueError, match="invalid_invocation"):
        handler.handler({"source": "somewhere", "pool_id": POOL}, Context())


def test_quarantine_emits_lambda_error_for_alarm(monkeypatch):
    monkeypatch.setattr(handler, "Control", lambda *args: object())
    monkeypatch.setattr(handler, "Power", lambda *args: object())
    monkeypatch.setattr(handler, "reconcile", lambda *args: "quarantined")
    with pytest.raises(RuntimeError, match="capacity_quarantined"):
        handler.handler({"source": "mobile-qa-scheduler", "pool_id": POOL}, Context())
