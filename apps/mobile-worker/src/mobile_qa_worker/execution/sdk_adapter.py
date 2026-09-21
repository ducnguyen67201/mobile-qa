"""Explicit bounded navigation; imports the SDK after process environment isolation."""

from dataclasses import replace
from pathlib import Path

from mobile_qa_worker.generated.models import NavigationRequest
from mobile_qa_worker.qualification.config import Profile
from mobile_qa_worker.qualification.sdk_adapter import navigate


async def execute(request_path: Path, result_path: Path) -> None:
    request = NavigationRequest.model_validate_json(request_path.read_bytes(), strict=True)
    if request.package != "ai.mobileqa.demo" or request.serial != "emulator-5554":
        raise ValueError("unsupported_navigation_target")
    # The child may load host paths and call policy, but provider/model identity is
    # always taken from the API-frozen resolved assignment below.
    profile = Profile.load(Path(request.profile_path))
    profile = replace(profile, max_steps=min(profile.max_steps, request.max_steps))
    await navigate(
        profile,
        request.resolved_model,
        request.serial,
        request.package,
        str(request.attempt_id),
        result_path,
        request.instruction,
    )
