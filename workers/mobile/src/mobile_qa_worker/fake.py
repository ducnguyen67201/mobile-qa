"""Pure deterministic fixture execution; never imports the device SDK."""

import json

from mobile_qa_worker.generated.models import FakeExecutionRequest, FakeExecutionResult


def parse_request(raw: str) -> FakeExecutionRequest:
    request = FakeExecutionRequest.model_validate_json(raw, strict=True)
    if request.version != 1:
        raise ValueError("unsupported fixture protocol version")
    return request


def execute(request: FakeExecutionRequest) -> FakeExecutionResult:
    scenario = request.model_dump(mode="json")["scenario"]
    kind = scenario["kind"]
    outcomes = {
        "pass": ("passed", "Fixture expectation observed."),
        "fail": ("failed", "Fixture expectation was not observed."),
        "blocked": ("blocked", "Fixture prerequisite is unavailable."),
    }
    if request.version != 1 or kind not in outcomes:
        raise ValueError("invalid fixture request")
    outcome, message = outcomes[kind]
    return FakeExecutionResult.model_validate_json(
        json.dumps(
            {
                "version": 1,
                "run_id": str(request.run_id),
                "outcome": outcome,
                "message": message,
            }
        ),
        strict=True,
    )
