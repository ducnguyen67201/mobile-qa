from typing import Literal

from pydantic import BaseModel

LLMProvider = Literal[
    "azure", "openai", "google", "vertexai", "openrouter", "xai", "minitap", "anthropic", "minimax"
]

class LLM(BaseModel):
    provider: LLMProvider
    model: str

class LLMWithFallback(LLM):
    fallback: LLM

class LLMConfigUtils(BaseModel):
    outputter: LLMWithFallback
    hopper: LLMWithFallback
    video_analyzer: LLMWithFallback | None = None

class LLMConfig(BaseModel):
    planner: LLMWithFallback
    orchestrator: LLMWithFallback
    contextor: LLMWithFallback
    cortex: LLMWithFallback
    executor: LLMWithFallback
    utils: LLMConfigUtils
