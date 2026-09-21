import asyncio
import json
import subprocess
import sys
from types import SimpleNamespace
from typing import cast

import pytest
from test_qualification import request_data

from mobile_qa_worker.generated.models import ModelCapability
from mobile_qa_worker.qualification import sdk_adapter
from mobile_qa_worker.qualification.config import Profile, QualificationError
from mobile_qa_worker.qualification.sdk_adapter import UsageRecorder


def model():
    from mobile_qa_worker.generated.models import ResolvedModel

    return ResolvedModel.model_validate(
        {
            "reference": {"key": "demo", "revision": 1},
            "display_name": "Demo",
            "provider": "open_ai",
            "provider_model": "demo",
            "capabilities": ["minitap_navigation"],
        }
    )


def test_usage_deduplicates_and_keeps_unknown():
    recorder = UsageRecorder(model())
    recorder.start("one")
    recorder.end("one", {"input_tokens": 10, "output_tokens": 2})
    recorder.end("one", {"input_tokens": 10, "output_tokens": 2})
    recorder.start("two")
    recorder.end("two", {"input_tokens": True, "output_tokens": 2})
    assert recorder.result() == {
        "model": "demo",
        "model_reference": {"key": "demo", "revision": 1},
        "calls": 2,
        "unknown_calls": 1,
        "input_tokens": 10,
        "output_tokens": 2,
    }
    empty = UsageRecorder(model())
    empty.start("x")
    assert empty.result()["input_tokens"] is None


def test_sdk_environment_in_subprocess(tmp_path):
    code = """
import os,sys
from pathlib import Path
from mobile_qa_worker.generated.models import ResolvedModel
from mobile_qa_worker.qualification.sdk_adapter import prepare_environment
os.environ['OPENAI_API_KEY']='dummy'
os.environ['DATABASE_URL']='must-not-propagate'
os.environ['DOPPLER_TOKEN']='must-not-propagate'
model=ResolvedModel.model_validate({'reference':{'key':'demo','revision':1},'display_name':'Demo','provider':'open_ai','provider_model':'demo','capabilities':['minitap_navigation']})
prepare_environment(model,Path(sys.argv[1]))
assert os.environ['OPENAI_API_KEY']=='dummy'
assert 'DATABASE_URL' not in os.environ and 'DOPPLER_TOKEN' not in os.environ
assert os.environ['PYTHON_DOTENV_DISABLED']=='1'
assert os.environ['MOBILE_USE_TELEMETRY_ENABLED']=='false'
assert not any(name.startswith('minitap') for name in sys.modules)
"""
    subprocess.run([sys.executable, "-c", code, str(tmp_path)], check=True)


def test_qualification_rejects_non_navigation_model_before_provider_setup(tmp_path, monkeypatch):
    restricted = model().model_copy(update={"capabilities": [ModelCapability.structured_authoring]})
    monkeypatch.setattr(
        sdk_adapter,
        "prepare_provider_environment",
        lambda *_: pytest.fail("provider setup must not run"),
    )
    with pytest.raises(QualificationError, match="model_capability_unavailable"):
        asyncio.run(
            sdk_adapter.navigate(
                cast(Profile, SimpleNamespace()),
                restricted,
                "emulator-5554",
                "ai.mobileqa.demo",
                "synthetic-attempt",
                tmp_path / "result.json",
                "Navigate",
            )
        )


def test_sdk_exact_public_seam(tmp_path, monkeypatch):
    data = request_data(tmp_path)
    data["resolved_model"] = model().model_dump(mode="json")
    request = tmp_path / "request.json"
    request.write_text(json.dumps(data))
    profile = tmp_path / "profile.toml"
    profile.write_text(
        f'sdk_root="{tmp_path}/sdk"\nstate_root="{tmp_path}/state"\ntoolchain="{tmp_path}/lock.json"\n[model_ref]\nkey="demo"\nrevision=1\n'
    )
    seen = []
    usage = UsageRecorder(model())
    monkeypatch.setattr(sdk_adapter, "UsageRecorder", lambda model: usage)

    class Builder:
        def for_device(self, **kwargs):
            seen.append(kwargs)
            return self

        def add_profile(self, p):
            return self

        def with_default_profile(self, p):
            return self

        def with_graph_config_callbacks(self, c):
            seen.append(c[0])
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

    class TaskRequest:
        def __class_getitem__(cls, item):
            return lambda **kwargs: kwargs

    modules = {
        "minitap.mobile_use.sdk": SimpleNamespace(Agent=Agent),
        "minitap.mobile_use.sdk.types": SimpleNamespace(
            DevicePlatform=SimpleNamespace(ANDROID="android"),
            AgentProfile=lambda **k: k,
            TaskRequest=TaskRequest,
        ),
        "minitap.mobile_use.sdk.builders.agent_config_builder": SimpleNamespace(
            AgentConfigBuilder=Builder
        ),
        "minitap.mobile_use.config": SimpleNamespace(
            LLM=lambda **k: k,
            LLMWithFallback=lambda **k: k,
            LLMConfigUtils=lambda **k: k,
            LLMConfig=lambda **k: k,
        ),
    }
    for name, module in modules.items():
        monkeypatch.setitem(sys.modules, name, module)
    monkeypatch.setattr(sdk_adapter.importlib.metadata, "version", lambda name: "4.0.0")
    monkeypatch.setattr(sdk_adapter, "prepare_provider_environment", lambda m, p: None)
    monkeypatch.setattr(sdk_adapter, "minitap_model", lambda m: {})
    monkeypatch.setattr(sdk_adapter, "install_tool_runtime_compat", lambda: None)
    asyncio.run(sdk_adapter.execute(request, tmp_path / "child-result.json"))
    assert seen[0] == {"platform": "android", "device_id": "emulator-5554"}
    assert seen[-1] == "stop"
    assert "expected_outcome" not in str(seen)
    assert json.loads((tmp_path / "child-result.json").read_text())["status"] == "completed"

    # Exercise the real callback against LangChain's typed responses. Missing usage
    # stays unknown; valid message metadata takes precedence over provider fallback.
    from uuid import uuid4

    from langchain_core.messages import AIMessage
    from langchain_core.outputs import ChatGeneration, Generation, LLMResult

    callback = seen[1]
    identity = uuid4()
    asyncio.run(callback.on_chat_model_start({}, [], run_id=identity))
    asyncio.run(callback.on_llm_end(LLMResult(generations=[]), run_id=identity))
    asyncio.run(
        callback.on_llm_end(
            LLMResult(
                generations=[
                    [
                        ChatGeneration(
                            message=AIMessage(
                                content="done",
                                usage_metadata={
                                    "input_tokens": 10,
                                    "output_tokens": 2,
                                    "total_tokens": 12,
                                },
                            )
                        )
                    ]
                ],
                llm_output={"token_usage": {"prompt_tokens": 99, "completion_tokens": 99}},
            ),
            run_id=identity,
        )
    )
    asyncio.run(
        callback.on_llm_end(
            LLMResult(
                generations=[[Generation(text="done")]],
                llm_output={"token_usage": {"prompt_tokens": 3, "completion_tokens": 1}},
            ),
            run_id=uuid4(),
        )
    )
    asyncio.run(callback.on_llm_end(LLMResult(generations=[[]]), run_id=uuid4()))
    assert usage.result() == {
        "model": "demo",
        "model_reference": {"key": "demo", "revision": 1},
        "calls": 3,
        "unknown_calls": 1,
        "input_tokens": 13,
        "output_tokens": 3,
    }


def test_sdk_stub_surface_matches_installed_package(tmp_path):
    # Import the actual SDK in an isolated process: no Agent construction, secrets,
    # network connections, or persistent upstream import side effects in pytest.
    from pathlib import Path

    stubs = Path(__file__).resolve().parents[1] / "typings/minitap/mobile_use"
    code = r"""
import ast
import importlib
import inspect
import os
import socket
import sys
from pathlib import Path
from unittest.mock import patch

os.environ['MOBILE_USE_TELEMETRY_ENABLED'] = 'false'
os.environ['PYTHON_DOTENV_DISABLED'] = '1'

def blocked(*args, **kwargs):
    raise AssertionError('SDK compatibility check attempted network access')

with (
    patch.object(socket.socket, 'connect', blocked),
    patch.object(socket.socket, 'connect_ex', blocked),
    patch.object(socket, 'create_connection', blocked),
):
    root = Path(sys.argv[1])
    for path in root.rglob('*.pyi'):
        suffix = path.relative_to(root).with_suffix('').parts
        if suffix[-1] == '__init__':
            suffix = suffix[:-1]
        module = importlib.import_module('.'.join(('minitap', 'mobile_use', *suffix)))
        tree = ast.parse(path.read_text())
        namespace = dict(vars(module))
        for declaration in tree.body:
            if isinstance(declaration, (ast.Import, ast.ImportFrom)):
                exec(ast.unparse(declaration), namespace)
        for cls in tree.body:
            if not isinstance(cls, ast.ClassDef):
                continue
            actual = getattr(module, cls.name)
            for member in cls.body:
                if isinstance(member, ast.AnnAssign) and isinstance(member.target, ast.Name):
                    field = actual.model_fields[member.target.id]
                    label = (cls.name, member.target.id)
                    assert field.is_required() == (member.value is None), label
                    if member.value is not None and not (
                        isinstance(member.value, ast.Constant) and member.value.value is Ellipsis
                    ):
                        assert field.default == ast.literal_eval(member.value)
                    # The generic output_format refers to the class type parameter;
                    # all concrete model field types must match the wheel exactly.
                    if member.target.id != 'output_format':
                        expected = eval(ast.unparse(member.annotation), namespace)
                        assert field.annotation == expected, (cls.name, member.target.id)
                if not isinstance(member, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    continue
                method = getattr(actual, member.name)
                asynchronous = isinstance(member, ast.AsyncFunctionDef)
                assert inspect.iscoroutinefunction(method) == asynchronous
                signature = inspect.signature(method)
                positional = [arg.arg for arg in member.args.args if arg.arg != 'self']
                keywords = [arg.arg for arg in member.args.kwonlyargs]
                # The stub's supported call must be accepted by the real method.
                signature.bind(
                    None, *[object() for _ in positional], **dict.fromkeys(keywords, object())
                )
                for name in positional + keywords:
                    assert name in signature.parameters, (cls.name, member.name, name)
    # An actual offline graph catches private-method compatibility failures that
    # importing the SDK and mocking Agent cannot expose. No model/device is used.
    import asyncio
    from langchain_core.messages import AIMessage, ToolMessage
    from langchain_core.tools import tool
    from langgraph.graph import StateGraph, MessagesState, START, END
    from mobile_qa_worker.qualification.sdk_compat import install_tool_runtime_compat
    from minitap.mobile_use.graph import graph as sdk_graph

    install_tool_runtime_compat()
    calls = []

    @tool
    def record_value(value: str) -> str:
        'Record a harmless value for the offline compatibility test.'
        calls.append(value)
        return 'recorded:' + value

    graph = StateGraph(MessagesState)
    graph.add_node('execute', sdk_graph.ExecutorToolNode([record_value], messages_key='messages'))
    graph.add_edge(START, 'execute')
    graph.add_edge('execute', END)
    result = asyncio.run(graph.compile().ainvoke({'messages': [AIMessage(
        content='', tool_calls=[{
            'name': 'record_value', 'args': {'value': 'demo'}, 'id': 'offline-call',
            'type': 'tool_call',
        }],
    )]}))
    assert calls == ['demo']
    message = result['messages'][-1]
    assert isinstance(message, ToolMessage)
    assert message.content == 'recorded:demo' and message.status == 'success'
    print('Pinned SDK signatures, model fields and offline tool execution passed')
"""
    subprocess.run([sys.executable, "-c", code, str(stubs)], cwd=tmp_path, check=True)
