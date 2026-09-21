"""The single provider boundary for nonsecret resolved model assignments."""

import os
from collections.abc import Callable
from pathlib import Path
from typing import Any

from mobile_qa_worker.generated.models import (
    ModelCapability,
    ModelProvider,
    ModelReference,
    ResolvedModel,
    WorkerModelCapabilities,
)
from mobile_qa_worker.qualification.config import Profile, QualificationError


def require_capability(resolved: ResolvedModel, capability: ModelCapability) -> None:
    if capability not in resolved.capabilities:
        raise QualificationError("model_capability_unavailable")


def require_host_assignment(resolved: ResolvedModel, host: ModelReference | None) -> None:
    if host is None or host != resolved.reference:
        raise QualificationError("worker_model_reference_mismatch")


def worker_capabilities(profile: Profile | None) -> WorkerModelCapabilities:
    return WorkerModelCapabilities(
        model=profile.model_ref if profile else None,
        providers=[ModelProvider.open_ai],
    )


def prepare_provider_environment(resolved: ResolvedModel, directory: Path) -> None:
    if resolved.provider != ModelProvider.open_ai:
        raise QualificationError("model_provider_unsupported")
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
    import resource

    resource.setrlimit(resource.RLIMIT_FSIZE, (16777216, 16777216))


def structured_model(
    resolved: ResolvedModel,
    *,
    timeout: int,
    max_completion_tokens: int,
    callbacks: list[Any] | None = None,
    factory: Callable[..., Any] | None = None,
) -> Any:
    if resolved.provider != ModelProvider.open_ai:
        raise QualificationError("model_provider_unsupported")
    if factory is None:
        from langchain_openai import ChatOpenAI

        factory = ChatOpenAI
    return factory(
        model=resolved.provider_model,
        max_retries=0,
        timeout=timeout,
        max_completion_tokens=max_completion_tokens,
        callbacks=callbacks,
    )


def minitap_model(resolved: ResolvedModel) -> Any:
    if resolved.provider != ModelProvider.open_ai:
        raise QualificationError("model_provider_unsupported")
    from minitap.mobile_use.config import LLM, LLMWithFallback

    return LLMWithFallback(
        provider="openai",
        model=resolved.provider_model,
        fallback=LLM(provider="openai", model=resolved.provider_model),
    )


def usage_identity(resolved: ResolvedModel) -> dict[str, object]:
    return {
        "model": resolved.provider_model,
        "model_reference": resolved.reference.model_dump(mode="json"),
    }
