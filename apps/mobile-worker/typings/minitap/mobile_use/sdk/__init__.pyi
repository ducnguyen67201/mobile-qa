from pathlib import Path

from minitap.mobile_use.sdk.types import AgentConfig, TaskRequest

class Agent:
    def __init__(self, *, config: AgentConfig | None = None) -> None: ...
    async def init(
        self, server_restart_attempts: int = 3, retry_count: int = 5, retry_wait_seconds: int = 5
    ) -> bool: ...
    # The adapter ignores SDK output; object requires narrowing before anyone uses it.
    async def run_task(
        self,
        *,
        request: TaskRequest[None],
        locked_app_package: str | None = None,
        app_path: str | Path | None = None,
    ) -> object: ...
    def stop_current_task(self) -> None: ...
