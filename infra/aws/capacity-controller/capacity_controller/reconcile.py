"""One bounded pass. Ambiguous power requests remain journaled until observed.

Reserved Lambda concurrency is one. A durable API operation is committed before
an AWS write, and an old operation is never blindly reissued. Thus an invocation
that died after StopInstances cannot later stop a newly started generation.
"""

from datetime import datetime
from typing import Protocol

from mobile_qa_worker.generated.models import (
    CapacityAction,
    CapacitySnapshot,
    HostState,
    ObservedPower,
)


class ControlPort(Protocol):
    def snapshot(self) -> CapacitySnapshot: ...
    def action(
        self,
        snapshot: CapacitySnapshot,
        action: CapacityAction,
        observed: ObservedPower | None = None,
        reason: str | None = None,
    ) -> CapacitySnapshot: ...


class PowerPort(Protocol):
    def describe(self, instance_id: str) -> ObservedPower: ...
    def start(self, instance_id: str) -> None: ...
    def stop(self, instance_id: str) -> None: ...


class UnsafeIdentity(RuntimeError):
    """API registration cannot expand the instance allowed by the deployment."""


def reconcile(api: ControlPort, power: PowerPort, instance_id: str, now: datetime) -> str:
    snapshot = api.snapshot()
    if snapshot.instance_id != instance_id:
        raise UnsafeIdentity("instance_mismatch")
    observed = power.describe(instance_id)
    if observed == ObservedPower.unknown:
        return "unknown_power"
    # Recording observation is required even while a pool is paused: a previously
    # committed operation still needs resolution. Pausing never invents cleanup.
    snapshot = api.action(snapshot, CapacityAction.observe, observed)
    operation = snapshot.current_operation
    if operation is not None:
        reached = (
            operation.action == CapacityAction.start and observed == ObservedPower.running
        ) or (operation.action == CapacityAction.commit_stop and observed == ObservedPower.stopped)
        if reached:
            api.action(snapshot, CapacityAction.record_power, observed)
            return "operation_observed"
        if (now - operation.created_at).total_seconds() >= 900:
            api.action(snapshot, CapacityAction.quarantine, observed, "power_operation_timeout")
            return "quarantined"
        # Covers timeout after AWS acceptance and a crash before the AWS call.
        # Safety wins: reconcile observations, alert after deadline, never retry a
        # stale Stop. Operator recovery explicitly resolves an unissued operation.
        return "operation_pending"
    if snapshot.state == HostState.quarantined:
        return "quarantined"
    if observed in (ObservedPower.pending, ObservedPower.stopping):
        return "transition_pending"
    if (observed == ObservedPower.stopped and snapshot.state != HostState.stopped) or (
        observed == ObservedPower.running and snapshot.state == HostState.stopped
    ):
        api.action(snapshot, CapacityAction.quarantine, observed, "unexpected_power_transition")
        return "quarantined"
    if observed == ObservedPower.stopped:
        if not snapshot.policy.enabled or not snapshot.desired_online:
            return "idle_stopped"
        snapshot = api.action(snapshot, CapacityAction.start, observed)
        if (
            not snapshot.current_operation
            or snapshot.current_operation.action != CapacityAction.start
        ):
            raise UnsafeIdentity("missing_start_intent")
        power.start(instance_id)
        # Do not mark completion from AWS's response; observe state next pass.
        return "start_requested"
    if snapshot.state == HostState.draining:
        # Only the API may decide whether fresh cleanup is sufficient. Its CAS
        # rejects work submitted before commit; later demand waits for restart.
        snapshot = api.action(snapshot, CapacityAction.commit_stop, observed)
        if (
            snapshot.state != HostState.stop_committed
            or not snapshot.current_operation
            or snapshot.current_operation.action != CapacityAction.commit_stop
        ):
            raise UnsafeIdentity("missing_stop_intent")
        power.stop(instance_id)
        return "stop_requested"
    if snapshot.state == HostState.ready and not snapshot.desired_online:
        api.action(snapshot, CapacityAction.drain, observed)
        return "drain_requested"
    return "host_active"
