"""Offline actual Agent.run_task graph with scripted LangChain model responses, never a device."""

# Imports intentionally follow environment isolation and socket denial.
# ruff: noqa: E402
import asyncio
import base64
import io
import json
import os
import socket
import sys
from pathlib import Path
from types import SimpleNamespace
from uuid import uuid4

from mobile_qa_worker.generated.models import ResolvedModel
from mobile_qa_worker.qualification.sdk_adapter import prepare_environment

os.environ["OPENAI_API_KEY"] = "synthetic-offline-no-network"
resolved = ResolvedModel.model_validate(
    {
        "reference": {"key": "offline", "revision": 1},
        "display_name": "Offline",
        "provider": "open_ai",
        "provider_model": "offline",
        "capabilities": ["minitap_navigation"],
    }
)
prepare_environment(resolved, Path(sys.argv[1]))


def deny_network(*args, **kwargs):
    raise AssertionError("SDK must not access a network/device in conformance")


socket.socket.connect = deny_network
socket.create_connection = deny_network

from langchain_core.language_models.chat_models import BaseChatModel
from langchain_core.messages import AIMessage
from langchain_core.outputs import ChatGeneration, ChatResult
from langchain_core.runnables import RunnableLambda
from minitap.mobile_use.agents.planner import planner
from minitap.mobile_use.sdk.types import TaskRequest
from PIL import Image

from mobile_qa_worker.authoring.minitap_discovery import configure_agent
from mobile_qa_worker.generated.models import DiscoveryCall, DiscoveryReply, DiscoverySnapshot

planner.generate_id = lambda: "goal-one"
blocked_tool = len(sys.argv) > 2 and sys.argv[2] == "native"
roles = []
executed = []
reserved = set()
measured = set()


class ScriptedModel(BaseChatModel):
    role: str = "executor"

    @property
    def _llm_type(self):
        return "offline-discovery"

    def with_structured_output(self, schema, **kwargs):
        return self.model_copy(update={"role": schema.__name__}) | RunnableLambda(
            lambda msg: schema.model_validate_json(msg.content)
        )

    def bind_tools(self, tools, **kwargs):
        assert [t.name for t in tools] == ["perform_direct_action"]
        return self

    def _generate(self, messages, stop=None, run_manager=None, **kwargs):
        roles.append(self.role)
        if self.role == "PlannerOutput":
            data = {"subgoals": [{"description": "Enter Hello"}]}
        elif self.role == "CortexOutput":
            data = (
                {
                    "decisions": "Enter Hello with perform_direct_action",
                    "decisions_reason": "Observed input",
                }
                if roles.count("CortexOutput") == 1
                else {
                    "complete_subgoals_by_ids": ["goal-one"],
                    "goals_completion_reason": "Observed Hello",
                }
            )
        elif self.role == "OrchestratorOutput":
            data = {
                "completed_subgoal_ids": ["goal-one"],
                "needs_replaning": False,
                "reason": "Observed",
            }
        else:
            if blocked_tool:
                return ChatResult(
                    generations=[
                        ChatGeneration(
                            message=AIMessage(
                                content="",
                                tool_calls=[
                                    {"id": "native", "name": "tap", "args": {"x": 1, "y": 1}},
                                    {
                                        "id": "later",
                                        "name": "perform_direct_action",
                                        "args": {"command": {"operation": "back"}},
                                    },
                                ],
                                usage_metadata={
                                    "input_tokens": 10,
                                    "output_tokens": 5,
                                    "total_tokens": 15,
                                },
                            )
                        )
                    ]
                )
            return ChatResult(
                generations=[
                    ChatGeneration(
                        message=AIMessage(
                            content="",
                            tool_calls=[
                                {
                                    "id": "enter-one",
                                    "name": "perform_direct_action",
                                    "args": {
                                        "command": {
                                            "operation": "set_text",
                                            "target": {
                                                "by": "resource_id",
                                                "value": "ai.mobileqa.demo:id/task_input",
                                            },
                                            "text": "Hello",
                                        }
                                    },
                                }
                            ],
                            usage_metadata={
                                "input_tokens": 10,
                                "output_tokens": 5,
                                "total_tokens": 15,
                            },
                        )
                    )
                ]
            )
        return ChatResult(
            generations=[
                ChatGeneration(
                    message=AIMessage(
                        content=json.dumps(data),
                        usage_metadata={"input_tokens": 10, "output_tokens": 5, "total_tokens": 15},
                    )
                )
            ]
        )


image = io.BytesIO()
Image.new("RGB", (1080, 1920), "white").save(image, "PNG")
snapshot = DiscoverySnapshot.model_validate(
    {
        "id": str(uuid4()),
        "frame": {
            "id": str(uuid4()),
            "width": 1080,
            "height": 1920,
            "png_base64": base64.b64encode(image.getvalue()).decode(),
            "controls": [],
        },
    }
)


class Broker:
    def call(self, payload):
        request = DiscoveryCall.model_validate(payload).root
        if request.kind == "reserve":
            assert request.id not in reserved
            reserved.add(request.id)
        if request.kind == "usage":
            assert request.id in reserved
            assert request.input_tokens == 10 and request.output_tokens == 5
            measured.add(request.id)
        if request.kind == "execute":
            executed.append(request.command.model_dump(mode="json"))
        return DiscoveryReply(
            id=request.id,
            error=None,
            snapshot=snapshot if request.kind in ("observe", "execute") else None,
        )


async def main():
    agent = configure_agent(
        Broker(),
        SimpleNamespace(max_steps=30),
        resolved,
        "ai.mobileqa.demo",
        model_factory=lambda **kw: ScriptedModel(callbacks=kw["callbacks"]),
    )
    await asyncio.wait_for(
        agent.run_task(request=TaskRequest(goal="Enter Hello", max_steps=30, record_trace=False)),
        30,
    )
    if blocked_tool:
        assert not executed, "Native tool failure must abort later calls in the batch"
    else:
        assert executed == [
            {
                "operation": "set_text",
                "target": {"by": "resource_id", "value": "ai.mobileqa.demo:id/task_input"},
                "text": "Hello",
            }
        ]
    assert roles == [
        "PlannerOutput",
        "CortexOutput",
        "executor",
        "CortexOutput",
        "OrchestratorOutput",
    ], roles
    assert len(reserved) == 5 and reserved == measured
    print("SDK discovery conformance passed")


asyncio.run(main())
