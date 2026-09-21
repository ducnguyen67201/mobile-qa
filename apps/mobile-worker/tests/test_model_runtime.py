from pathlib import Path
from unittest.mock import Mock

import pytest

from mobile_qa_worker.generated.models import ModelCapability, ModelReference, ResolvedModel
from mobile_qa_worker.model_runtime import (
    prepare_provider_environment,
    require_capability,
    require_host_assignment,
    structured_model,
    usage_identity,
    worker_capabilities,
)
from mobile_qa_worker.qualification.config import Profile, QualificationError


def resolved() -> ResolvedModel:
    return ResolvedModel.model_validate(
        {
            "reference": {"key": "synthetic.openai", "revision": 2},
            "display_name": "Synthetic",
            "provider": "open_ai",
            "provider_model": "provider-model",
            "capabilities": ["minitap_navigation", "structured_authoring"],
        }
    )


def test_factory_uses_frozen_provider_model_and_usage_reference():
    factory = Mock(return_value=object())
    model = resolved()
    structured_model(model, timeout=12, max_completion_tokens=34, factory=factory)
    factory.assert_called_once_with(
        model="provider-model",
        max_retries=0,
        timeout=12,
        max_completion_tokens=34,
        callbacks=None,
    )
    assert usage_identity(model)["model_reference"] == {
        "key": "synthetic.openai",
        "revision": 2,
    }


def test_capability_and_host_reference_fail_closed():
    model = resolved()
    require_capability(model, ModelCapability.minitap_navigation)
    require_host_assignment(model, model.reference)
    with pytest.raises(QualificationError, match="model_capability_unavailable"):
        require_capability(
            model.model_copy(update={"capabilities": []}),
            ModelCapability.structured_authoring,
        )
    with pytest.raises(QualificationError, match="worker_model_reference_mismatch"):
        require_host_assignment(model, ModelReference(key="other", revision=1))


def test_missing_credentials_fail_before_provider_import(tmp_path: Path, monkeypatch):
    monkeypatch.delenv("OPENAI_API_KEY", raising=False)
    with pytest.raises(QualificationError, match="model_credentials_unavailable"):
        prepare_provider_environment(resolved(), tmp_path.resolve())


def test_qualified_model_set_requires_evidence_and_matches_exact_reference(tmp_path: Path):
    profile_path = tmp_path / "profile.toml"
    image = "system-images;android-35;google_apis;x86_64"
    base = (
        f'sdk_root="{tmp_path}/sdk"\nstate_root="{tmp_path}/state"\n'
        f'toolchain="{tmp_path}/lock.json"\nsystem_image="{image}"\n'
    )
    model = (
        '[[qualified_models]]\nkey="synthetic.openai"\nrevision=2\n'
        'evidence_reference="operator-campaign-1"\n'
        f'sdk_sha256="{"a" * 64}"\nruntime_sha256="{"b" * 64}"\n'
        f'image="{image}"\n'
    )
    profile_path.write_text(base + model)
    profile = Profile.load(profile_path)
    caps = worker_capabilities(profile)
    assert caps.model is None
    assert caps.models == [resolved().reference]
    require_host_assignment(resolved(), profile)
    with pytest.raises(QualificationError, match="worker_model_reference_mismatch"):
        require_host_assignment(
            resolved().model_copy(
                update={"reference": ModelReference(key="synthetic.openai", revision=3)}
            ),
            profile,
        )
    profile_path.write_text(base + model.replace("operator-campaign-1", ""))
    with pytest.raises(QualificationError, match="invalid_qualified_models"):
        Profile.load(profile_path)
