"""Return the result chosen by a fixture; do not discover whether an app works.

This fake has no phone, model, database or lease API. Rust owns its message shapes
in crates/contracts/src/worker.rs; generated Pydantic models enforce the JSON boundary.
"""

import json

from mobile_qa_worker.generated.models import FakeExecutionRequest, FakeExecutionResult


def parse_request(raw: str) -> FakeExecutionRequest:
    """Validate fixture JSON without coercing values such as string versions to integers."""
    request = FakeExecutionRequest.model_validate_json(raw, strict=True)
    if request.version != 1:
        raise ValueError("unsupported fixture protocol version")
    return request


def execute(request: FakeExecutionRequest) -> FakeExecutionResult:
    """Echo the run ID and map the requested scenario to its fixed simulated outcome.

    Messages describe the fixture story, not evidence collected from an actual app.
    A failed fixture is a valid result; invalid input raises instead of becoming a result.
    """
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
    # Validate our output too, using the same JSON representation consumers receive.
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
