from collections.abc import Sequence

from langchain_core.messages import AnyMessage, ToolCall
from langchain_core.runnables import RunnableConfig
from langchain_core.tools import BaseTool
from langgraph.prebuilt import ToolNode
from langgraph.prebuilt.tool_node import ToolRuntime
from langgraph.runtime import Runtime
from langgraph.typing import ContextT
from pydantic import BaseModel

type GraphInput = list[AnyMessage] | dict[str, object] | BaseModel

class ExecutorToolNode(ToolNode):
    def __init__(
        self, tools: Sequence[BaseTool], messages_key: str, trace_id: str | None = None
    ) -> None: ...
    def _build_tool_runtime(
        self,
        call: ToolCall,
        input: GraphInput,
        config: RunnableConfig,
        runtime: Runtime[ContextT],
    ) -> ToolRuntime[object, object]: ...
