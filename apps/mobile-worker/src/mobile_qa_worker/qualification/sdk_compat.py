"""Version-bounded Minitap adaptation; loaded only after SDK environment isolation."""

from importlib.metadata import version

from mobile_qa_worker.qualification.config import QualificationError


def install_tool_runtime_compat() -> None:
    """Preserve Minitap's sequential executor while supplying LangGraph's new fields."""
    if version("minitap-mobile-use") != "4.0.0" or version("langgraph-prebuilt") != "1.1.0":
        raise QualificationError("sdk_compatibility_version_mismatch")

    from langchain_core.messages import AnyMessage, ToolCall
    from langchain_core.runnables import RunnableConfig
    from langgraph.prebuilt.tool_node import ToolRuntime
    from langgraph.runtime import Runtime
    from langgraph.typing import ContextT
    from minitap.mobile_use.agents.executor.tool_node import ExecutorToolNode
    from minitap.mobile_use.graph import graph
    from pydantic import BaseModel

    class CompatibleExecutorToolNode(ExecutorToolNode):
        def _build_tool_runtime(
            self,
            call: ToolCall,
            input: list[AnyMessage] | dict[str, object] | BaseModel,
            config: RunnableConfig,
            runtime: Runtime[ContextT],
        ) -> ToolRuntime[object, object]:
            return ToolRuntime(
                state=self._extract_state(input, config),
                tool_call_id=call["id"],
                config=config,
                context=runtime.context,
                store=runtime.store,
                stream_writer=runtime.stream_writer,
                tools=list(self.tools_by_name.values()),
                execution_info=runtime.execution_info,
                server_info=runtime.server_info,
            )

    # get_graph looks up this class when constructing each execution graph. Only
    # this SDK child sees the replacement; the installed package stays untouched.
    graph.ExecutorToolNode = CompatibleExecutorToolNode
