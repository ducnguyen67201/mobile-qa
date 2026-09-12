"""Only this subprocess imports Minitap. Its output never decides the QA verdict."""

import asyncio
import importlib
import importlib.metadata
import os
from collections.abc import Callable
from pathlib import Path
from typing import Any, cast

from mobile_qa_worker.qualification.config import Profile, QualificationError, parse_request
from mobile_qa_worker.qualification.evidence import atomic_json


class UsageRecorder:
    def __init__(self, model: str):
        self.model = model
        self.calls: dict[str, tuple[int, int] | None] = {}

    def start(self, identity: str) -> None:
        self.calls.setdefault(identity, None)

    def end(self, identity: str, metadata: object) -> None:
        self.start(identity)
        if not isinstance(metadata, dict):
            return
        values = cast(dict[str, object], metadata)
        incoming = values.get("input_tokens", values.get("prompt_tokens"))
        outgoing = values.get("output_tokens", values.get("completion_tokens"))
        if type(incoming) is int and type(outgoing) is int:
            if 0 <= incoming <= 10000000 and 0 <= outgoing <= 10000000:
                self.calls[identity] = incoming, outgoing

    def result(self) -> dict[str, object]:
        known = [value for value in self.calls.values() if value is not None]
        unknown = len(self.calls) - len(known)
        return {
            "model": self.model,
            "calls": len(self.calls),
            "unknown_calls": unknown,
            "input_tokens": sum(v[0] for v in known) if known else None,
            "output_tokens": sum(v[1] for v in known) if known else None,
        }


def prepare_environment(directory: Path) -> None:
    if directory != directory.resolve() or (directory / ".env").exists():
        raise QualificationError("unsafe_sdk_directory")
    key = os.environ.get("OPENAI_API_KEY")
    if not key:
        raise QualificationError("model_credentials_unavailable")
    allowed = {
        k: v
        for k, v in os.environ.items()
        if k in ("PATH", "LANG", "LC_ALL", "VIRTUAL_ENV", "HOME")
    }
    temp = directory / "tmp"
    temp.mkdir(mode=0o700, exist_ok=True)
    allowed.update(
        OPENAI_API_KEY=key,
        MOBILE_USE_TELEMETRY_ENABLED="false",
        PYTHON_DOTENV_DISABLED="1",
        TMPDIR=str(temp),
        TZ="UTC",
    )
    os.environ.clear()
    os.environ.update(allowed)
    os.chdir(directory)
    os.umask(0o077)
    # Bound individual files produced by upstream logging/trace code as well.
    import resource

    resource.setrlimit(resource.RLIMIT_FSIZE, (16777216, 16777216))


async def execute(request_path: Path, result_path: Path) -> None:
    request = parse_request(request_path.read_text())
    profile = Profile.load(Path(request.profile_path))
    prepare_environment(result_path.parent)
    # Keep SDK ADB helpers on the same pinned tools/user state as the supervisor.
    os.environ["PATH"] = (
        str(profile.sdk_root / "platform-tools") + os.pathsep + os.environ.get("PATH", "")
    )
    os.environ["ANDROID_HOME"] = str(profile.sdk_root)
    os.environ["ANDROID_USER_HOME"] = str(profile.state_root / "android")
    if importlib.metadata.version("minitap-mobile-use") != "4.0.0":
        raise QualificationError("sdk_version_mismatch")
    # Explicit Any is limited to the inspected, partly untyped third-party SDK seam.
    # Every file crossing back to our parent process is validated separately.
    from uuid import UUID

    from langchain_core.callbacks import AsyncCallbackHandler
    from langchain_core.outputs import LLMResult

    recorder = UsageRecorder(profile.model)

    class Callbacks(AsyncCallbackHandler):
        async def on_chat_model_start(
            self,
            serialized: dict[str, Any],
            messages: list[list[Any]],
            *,
            run_id: UUID,
            **kwargs: Any,
        ) -> None:
            recorder.start(str(run_id))

        async def on_llm_end(self, response: LLMResult, *, run_id: UUID, **kwargs: Any) -> None:
            try:
                payload: Any = response.llm_output or {}
                usage: object = payload.get("token_usage")
                if response.generations:
                    generation: Any = response.generations[0][0]
                    usage = (
                        getattr(getattr(generation, "message", None), "usage_metadata", None)
                        or usage
                    )
                recorder.end(str(run_id), usage)
            except (AttributeError, IndexError, TypeError, ValueError):
                recorder.start(str(run_id))

    sdk: Any = importlib.import_module("minitap.mobile_use.sdk")
    types: Any = importlib.import_module("minitap.mobile_use.sdk.types")
    builders: Any = importlib.import_module("minitap.mobile_use.sdk.builders.agent_config_builder")
    config: Any = importlib.import_module("minitap.mobile_use.config")
    node = {
        "provider": "openai",
        "model": profile.model,
        "fallback": {"provider": "openai", "model": profile.model},
    }
    llm = config.LLMConfig.model_validate(
        {
            **{n: node for n in ("planner", "orchestrator", "contextor", "cortex", "executor")},
            "utils": {"outputter": node, "hopper": node},
        }
    )
    agent_profile = types.AgentProfile(name="qualification", llm_config=llm)
    builder = builders.AgentConfigBuilder().for_device(
        platform=types.DevicePlatform.ANDROID, device_id=request.serial
    )
    agent = sdk.Agent(
        config=builder.add_profile(agent_profile)
        .with_default_profile("qualification")
        .with_graph_config_callbacks([Callbacks()])
        .build()
    )
    status = "error"
    try:
        await asyncio.wait_for(agent.init(), timeout=profile.init_seconds)
        goal = (
            f"Create exactly one task with the exact text qa-{request.attempt_id}. "
            "Press Save, verify the task is displayed in the list, then stop. "
            "Do not delete tasks, close the app, or change any settings."
        )
        atomic_json(result_path.parent / "navigation.started", {"phase": "navigation"})
        await asyncio.wait_for(
            agent.run_task(
                request=types.TaskRequest(
                    goal=goal,
                    task_name=str(request.attempt_id),
                    max_steps=profile.max_steps,
                    record_trace=True,
                    trace_path=result_path.parent / "traces",
                    locked_app_package=request.package,
                )
            ),
            timeout=profile.navigation_seconds,
        )
        status = "completed"
    finally:
        stop = cast(Callable[[], None], agent.stop_current_task)
        stop()
        atomic_json(result_path, {"status": status, "usage": recorder.result()})
