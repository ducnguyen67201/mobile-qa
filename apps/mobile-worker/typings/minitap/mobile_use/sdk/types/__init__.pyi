from enum import StrEnum
from pathlib import Path

from minitap.mobile_use.config import LLMConfig
from pydantic import BaseModel

class DevicePlatform(StrEnum):
    ANDROID = "android"
    IOS = "ios"

class AgentConfig(BaseModel): ...

class AgentProfile(BaseModel):
    name: str
    llm_config: LLMConfig = ...
    def __init__(self, *, name: str, llm_config: LLMConfig) -> None: ...

class TaskRequest[TOutput](BaseModel):
    goal: str
    profile: str | None = None
    task_name: str | None = None
    output_description: str | None = None
    output_format: type[TOutput] | None = None
    max_steps: int = ...
    record_trace: bool = False
    trace_path: Path = ...
    llm_output_path: Path | None = None
    thoughts_output_path: Path | None = None
    locked_app_package: str | None = None
    app_path: Path | None = None
