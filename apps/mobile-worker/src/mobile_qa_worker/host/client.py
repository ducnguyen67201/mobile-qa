"""Typed host control transport, distinct from app-bound executor credentials."""

from uuid import UUID

from mobile_qa_worker.execution.client import Client
from mobile_qa_worker.generated.models import (
    HostCleanupRequest,
    HostHeartbeatRequest,
    HostRegisterRequest,
    HostStatus,
    SlotGrantRequest,
    SlotGrantResponse,
)


class HostClient:
    def __init__(self, origin: str, host_id: UUID, token: str):
        self.client = Client(origin, token)
        self.prefix = f"/api/internal/hosts/{host_id}"

    def register(self, payload: HostRegisterRequest) -> HostStatus:
        return self.client.send(self.prefix + "/register", payload, HostStatus)

    def heartbeat(self, payload: HostHeartbeatRequest) -> HostStatus:
        return self.client.send(self.prefix + "/heartbeat", payload, HostStatus)

    def grant(self, slot_id: UUID, payload: SlotGrantRequest) -> SlotGrantResponse:
        return self.client.send(self.prefix + f"/slots/{slot_id}/grant", payload, SlotGrantResponse)

    def cleanup(self, payload: HostCleanupRequest) -> HostStatus:
        return self.client.send(self.prefix + "/cleanup", payload, HostStatus)
