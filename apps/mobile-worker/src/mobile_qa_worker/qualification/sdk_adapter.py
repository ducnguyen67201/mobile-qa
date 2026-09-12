"""Only this subprocess imports Minitap. Its output never decides the QA verdict."""

import asyncio
import importlib.metadata
import os
from pathlib import Path
from typing import cast

from mobile_qa_worker.qualification.config import Profile, QualificationError, parse_request
from mobile_qa_worker.qualification.evidence import atomic_json
from mobile_qa_worker.qualification.sdk_compat import install_tool_runtime_compat


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
    # Import only after credentials and telemetry settings are isolated. These are
    # the pinned SDK's real types, so our calls remain visible to the type checker.
    from uuid import UUID

    from langchain_core.callbacks import AsyncCallbackHandler
    from langchain_core.messages import AIMessage
    from langchain_core.outputs import ChatGeneration, LLMResult
    from minitap.mobile_use.config import LLM, LLMConfig, LLMConfigUtils, LLMWithFallback
    from minitap.mobile_use.sdk import Agent
    from minitap.mobile_use.sdk.builders.agent_config_builder import AgentConfigBuilder
    from minitap.mobile_use.sdk.types import AgentProfile, DevicePlatform, TaskRequest

    install_tool_runtime_compat()
    recorder = UsageRecorder(profile.model)

    class Callbacks(AsyncCallbackHandler):
        async def on_chat_model_start(
            self,
            serialized: object,
            messages: object,
            *,
            run_id: UUID,
            **kwargs: object,
        ) -> None:
            recorder.start(str(run_id))

        async def on_llm_end(self, response: LLMResult, *, run_id: UUID, **kwargs: object) -> None:
            try:
                usage: object = (response.llm_output or {}).get("token_usage")
                if response.generations:
                    generation = response.generations[0][0]
                    if isinstance(generation, ChatGeneration) and isinstance(
                        generation.message, AIMessage
                    ):
                        usage = generation.message.usage_metadata or usage
                recorder.end(str(run_id), usage)
            except (AttributeError, IndexError, TypeError, ValueError):
                recorder.start(str(run_id))

    node = LLMWithFallback(
        provider="openai",
        model=profile.model,
        fallback=LLM(provider="openai", model=profile.model),
    )
    llm = LLMConfig(
        planner=node,
        orchestrator=node,
        contextor=node,
        cortex=node,
        executor=node,
        utils=LLMConfigUtils(outputter=node, hopper=node),
    )
    agent_profile = AgentProfile(name="qualification", llm_config=llm)
    builder = AgentConfigBuilder().for_device(
        platform=DevicePlatform.ANDROID, device_id=request.serial
    )
    agent = Agent(
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
                request=TaskRequest[None](
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
        agent.stop_current_task()
        atomic_json(result_path, {"status": status, "usage": recorder.result()})
