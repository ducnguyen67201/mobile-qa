import asyncio
import json
import subprocess
import sys
from types import SimpleNamespace

from test_qualification import request_data

from mobile_qa_worker.qualification import sdk_adapter
from mobile_qa_worker.qualification.sdk_adapter import UsageRecorder


def test_usage_deduplicates_and_keeps_unknown():
    recorder = UsageRecorder("demo")
    recorder.start("one")
    recorder.end("one", {"input_tokens": 10, "output_tokens": 2})
    recorder.end("one", {"input_tokens": 10, "output_tokens": 2})
    recorder.start("two")
    recorder.end("two", {"input_tokens": True, "output_tokens": 2})
    assert recorder.result() == {
        "model": "demo",
        "calls": 2,
        "unknown_calls": 1,
        "input_tokens": 10,
        "output_tokens": 2,
    }
    empty = UsageRecorder("demo")
    empty.start("x")
    assert empty.result()["input_tokens"] is None


def test_sdk_environment_in_subprocess(tmp_path):
    code = """
import os,sys
from pathlib import Path
from mobile_qa_worker.qualification.sdk_adapter import prepare_environment
os.environ['OPENAI_API_KEY']='dummy'
os.environ['DATABASE_URL']='must-not-propagate'
os.environ['DOPPLER_TOKEN']='must-not-propagate'
prepare_environment(Path(sys.argv[1]))
assert os.environ['OPENAI_API_KEY']=='dummy'
assert 'DATABASE_URL' not in os.environ and 'DOPPLER_TOKEN' not in os.environ
assert os.environ['PYTHON_DOTENV_DISABLED']=='1'
assert os.environ['MOBILE_USE_TELEMETRY_ENABLED']=='false'
assert not any(name.startswith('minitap') for name in sys.modules)
"""
    subprocess.run([sys.executable, "-c", code, str(tmp_path)], check=True)


def test_sdk_exact_public_seam(tmp_path, monkeypatch):
    data = request_data(tmp_path)
    request = tmp_path / "request.json"
    request.write_text(json.dumps(data))
    profile = tmp_path / "profile.toml"
    profile.write_text(
        f'sdk_root="{tmp_path}/sdk"\nstate_root="{tmp_path}/state"\ntoolchain="{tmp_path}/lock.json"\nmodel="demo"\n'
    )
    seen = []

    class Builder:
        def for_device(self, **kwargs):
            seen.append(kwargs)
            return self

        def add_profile(self, p):
            return self

        def with_default_profile(self, p):
            return self

        def with_graph_config_callbacks(self, c):
            return self

        def build(self):
            return "configured"

    class Agent:
        def __init__(self, config):
            assert config == "configured"

        async def init(self):
            seen.append("init")

        async def run_task(self, request):
            seen.append(request)

        def stop_current_task(self):
            seen.append("stop")

    modules = {
        "minitap.mobile_use.sdk": SimpleNamespace(Agent=Agent),
        "minitap.mobile_use.sdk.types": SimpleNamespace(
            DevicePlatform=SimpleNamespace(ANDROID="android"),
            AgentProfile=lambda **k: k,
            TaskRequest=lambda **k: k,
        ),
        "minitap.mobile_use.sdk.builders.agent_config_builder": SimpleNamespace(
            AgentConfigBuilder=Builder
        ),
        "minitap.mobile_use.config": SimpleNamespace(
            LLMConfig=SimpleNamespace(model_validate=lambda d: d)
        ),
    }
    original = sdk_adapter.importlib.import_module
    monkeypatch.setattr(
        sdk_adapter.importlib,
        "import_module",
        lambda name, *args, **kwargs: modules[name]
        if name in modules
        else original(name, *args, **kwargs),
    )
    monkeypatch.setattr(sdk_adapter.importlib.metadata, "version", lambda name: "4.0.0")
    monkeypatch.setattr(sdk_adapter, "prepare_environment", lambda p: None)
    asyncio.run(sdk_adapter.execute(request, tmp_path / "child-result.json"))
    assert seen[0] == {"platform": "android", "device_id": "emulator-5554"}
    assert seen[-1] == "stop"
    assert "expected_outcome" not in str(seen)
    assert json.loads((tmp_path / "child-result.json").read_text())["status"] == "completed"
