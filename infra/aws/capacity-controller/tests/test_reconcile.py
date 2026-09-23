from datetime import UTC, datetime, timedelta
from uuid import uuid4

import pytest
from mobile_qa_worker.generated.models import (
    CapacityAction,
    CapacityOperation,
    CapacitySnapshot,
    HostState,
    ObservedPower,
    PoolPolicy,
)

from capacity_controller.control import Conflict, ControlUnavailable
from capacity_controller.reconcile import UnsafeIdentity, reconcile

NOW = datetime(2026, 9, 22, tzinfo=UTC)
INSTANCE = "i-0123456789abcdef0"


def snapshot(**changes):
    value = CapacitySnapshot(
        pool_id=uuid4(),
        host_id=uuid4(),
        instance_id=INSTANCE,
        policy=PoolPolicy(
            enabled=True, idle_seconds=900, warm_target=0, timezone="UTC", warm_windows=[]
        ),
        state=HostState.stopped,
        observed_power=ObservedPower.stopped,
        boot_id=None,
        generation=0,
        control_version=0,
        queued_jobs=1,
        active_work=0,
        last_demand_at=NOW,
        heartbeat_at=None,
        clean_at=None,
        startup_started_at=None,
        current_operation=None,
        reason=None,
        cache_bytes=0,
        desired_online=True,
    )
    return value.model_copy(update=changes)


class API:
    def __init__(self, value, conflict=None):
        self.value, self.calls, self.conflict = value, [], conflict

    def snapshot(self):
        return self.value

    def action(self, value, action, observed=None, reason=None):
        self.calls.append(action)
        if action == self.conflict:
            raise Conflict("racing_demand")
        if action in (CapacityAction.start, CapacityAction.commit_stop):
            self.value = value.model_copy(
                update={
                    "current_operation": CapacityOperation(
                        id=uuid4(), action=action, created_at=NOW
                    ),
                    "state": HostState.starting
                    if action == CapacityAction.start
                    else HostState.stop_committed,
                }
            )
        return self.value


class Power:
    def __init__(self, observed=ObservedPower.stopped, uncertain=False):
        self.observed, self.calls, self.uncertain = observed, [], uncertain

    def describe(self, instance_id):
        assert instance_id == INSTANCE
        return self.observed

    def start(self, instance_id):
        self.calls.append(("start", instance_id))
        if self.uncertain:
            raise TimeoutError

    def stop(self, instance_id):
        self.calls.append(("stop", instance_id))
        if self.uncertain:
            raise TimeoutError


def test_start_intent_precedes_single_aws_start():
    api, power = API(snapshot()), Power()
    assert reconcile(api, power, INSTANCE, NOW) == "start_requested"
    assert api.calls == [CapacityAction.observe, CapacityAction.start]
    assert power.calls == [("start", INSTANCE)]


def test_duplicate_invoke_does_not_repeat_unobserved_write():
    api, power = API(snapshot()), Power(uncertain=True)
    with pytest.raises(TimeoutError):
        reconcile(api, power, INSTANCE, NOW)
    assert reconcile(api, power, INSTANCE, NOW) == "operation_pending"
    assert len(power.calls) == 1


def test_stop_requires_committed_clean_drain():
    api = API(snapshot(state=HostState.draining, desired_online=False))
    power = Power(ObservedPower.running)
    assert reconcile(api, power, INSTANCE, NOW) == "stop_requested"
    assert api.calls[-1] == CapacityAction.commit_stop
    assert power.calls == [("stop", INSTANCE)]


def test_new_demand_before_stop_commit_blocks_aws():
    api = API(snapshot(state=HostState.draining), conflict=CapacityAction.commit_stop)
    power = Power(ObservedPower.running)
    with pytest.raises(Conflict):
        reconcile(api, power, INSTANCE, NOW)
    assert power.calls == []


def test_uncertain_stop_with_later_demand_is_never_replayed():
    operation = CapacityOperation(id=uuid4(), action=CapacityAction.commit_stop, created_at=NOW)
    api = API(
        snapshot(state=HostState.stop_committed, current_operation=operation, desired_online=True)
    )
    power = Power(ObservedPower.running)
    assert reconcile(api, power, INSTANCE, NOW) == "operation_pending"
    assert power.calls == []
    power.observed = ObservedPower.stopped
    assert reconcile(api, power, INSTANCE, NOW) == "operation_observed"
    assert power.calls == []


def test_demand_while_stopping_waits_for_future_pass():
    power = Power(ObservedPower.stopping)
    assert reconcile(API(snapshot()), power, INSTANCE, NOW) == "transition_pending"
    assert power.calls == []


def test_unknown_operation_quarantines_after_deadline():
    operation = CapacityOperation(
        id=uuid4(), action=CapacityAction.start, created_at=NOW - timedelta(seconds=901)
    )
    api, power = API(snapshot(current_operation=operation)), Power()
    assert reconcile(api, power, INSTANCE, NOW) == "quarantined"
    assert api.calls[-1] == CapacityAction.quarantine
    assert power.calls == []


def test_api_outage_never_stops_running_host():
    class Offline(API):
        def snapshot(self):
            raise ControlUnavailable("offline")

    power = Power(ObservedPower.running)
    with pytest.raises(ControlUnavailable):
        reconcile(Offline(snapshot()), power, INSTANCE, NOW)
    assert power.calls == []


def test_disabled_pool_stays_stopped():
    value = snapshot()
    value.policy.enabled = False
    power = Power()
    assert reconcile(API(value), power, INSTANCE, NOW) == "idle_stopped"
    assert power.calls == []


def test_mismatched_instance_fails_before_aws():
    with pytest.raises(UnsafeIdentity):
        reconcile(API(snapshot(instance_id="i-wrong")), Power(), INSTANCE, NOW)


def test_idle_host_requests_drain_without_stopping_yet():
    api, power = (
        API(snapshot(state=HostState.ready, desired_online=False)),
        Power(ObservedPower.running),
    )
    assert reconcile(api, power, INSTANCE, NOW) == "drain_requested"
    assert power.calls == []


@pytest.mark.parametrize(
    "state,power_state",
    [
        (HostState.stopped, ObservedPower.running),
        (HostState.ready, ObservedPower.stopped),
    ],
)
def test_unjournaled_power_transition_is_quarantined(state, power_state):
    api, power = API(snapshot(state=state)), Power(power_state)
    assert reconcile(api, power, INSTANCE, NOW) == "quarantined"
    assert api.calls[-1] == CapacityAction.quarantine
    assert power.calls == []
