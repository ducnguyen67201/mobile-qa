//! Durable demand and short power transactions. AWS calls happen only in the serialized controller.
use super::{
    device_hosts,
    execution_store::{conflict, decode, hash, word},
};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        capacity_hint_receipts, capacity_wake_outbox, device_control_receipts,
        device_hosts as host_rows, device_pools, device_power_operations, device_slot_bindings,
        device_slots, execution_attempts, execution_reservations, execution_runs, phone_sessions,
    },
};
use chrono::{DateTime, Datelike, NaiveTime, Timelike, Utc};
use loco_rs::app::AppContext;
use mobile_qa_contracts::device_hosts::*;
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait,
    EntityTrait, IntoActiveModel, QueryFilter, TransactionTrait,
};
use uuid::Uuid;

pub fn validate_policy(p: &PoolPolicy) -> ApiResult<()> {
    if !(60..=86400).contains(&p.idle_seconds)
        || p.warm_target > 16
        || p.timezone.parse::<chrono_tz::Tz>().is_err()
        || p.warm_windows.len() > 28
    {
        return Err(ApiFailure::invalid("Invalid pool policy"));
    }
    for w in &p.warm_windows {
        if w.weekdays.is_empty()
            || w.weekdays.len() > 7
            || w.weekdays.iter().any(|d| *d > 6)
            || clock_minutes(&w.start).is_none()
            || clock_minutes(&w.end).is_none()
            || w.start == w.end
        {
            return Err(ApiFailure::invalid(
                "Warm windows need distinct HH:MM times and weekdays 0–6",
            ));
        }
    }
    Ok(())
}
fn clock_minutes(s: &str) -> Option<u32> {
    if s.len() != 5 {
        return None;
    }
    let t = NaiveTime::parse_from_str(s, "%H:%M").ok()?;
    Some(t.hour() * 60 + t.minute())
}
/// Local wall-clock windows naturally handle DST: both repeated instants have the same policy.
pub fn scheduled(p: &PoolPolicy, now: DateTime<Utc>) -> bool {
    if !p.enabled || p.warm_target == 0 {
        return false;
    }
    let Ok(zone) = p.timezone.parse::<chrono_tz::Tz>() else {
        return false;
    };
    let local = now.with_timezone(&zone);
    let today = local.weekday().num_days_from_monday() as u8;
    let time = local.hour() * 60 + local.minute();
    p.warm_windows.iter().any(|w| {
        let (Some(start), Some(end)) = (clock_minutes(&w.start), clock_minutes(&w.end)) else {
            return false;
        };
        if start < end {
            w.weekdays.contains(&today) && time >= start && time < end
        } else {
            (w.weekdays.contains(&today) && time >= start)
                || (w.weekdays.contains(&((today + 6) % 7)) && time < end)
        }
    })
}

pub async fn demand(db: &impl ConnectionTrait, host: Uuid) -> ApiResult<i64> {
    let slot_ids = device_slots::Entity::find()
        .filter(device_slots::Column::HostId.eq(host))
        .all(db)
        .await?
        .into_iter()
        .map(|slot| slot.id)
        .collect::<Vec<_>>();
    if slot_ids.is_empty() {
        return Ok(0);
    }
    let bindings = device_slot_bindings::Entity::find()
        .filter(device_slot_bindings::Column::SlotId.is_in(slot_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|binding| (binding.app_id, binding.profile_id))
        .collect::<std::collections::HashSet<_>>();
    let reserved_apps = execution_reservations::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|reservation| reservation.resource)
        .collect::<std::collections::HashSet<_>>();
    let attempts = execution_attempts::Entity::find()
        .filter(execution_attempts::Column::State.eq("queued"))
        .all(db)
        .await?;
    let run_ids = attempts
        .iter()
        .map(|attempt| attempt.run_id)
        .collect::<Vec<_>>();
    let runs = if run_ids.is_empty() {
        Vec::new()
    } else {
        execution_runs::Entity::find()
            .filter(execution_runs::Column::Id.is_in(run_ids))
            .all(db)
            .await?
    };
    let runs = runs
        .into_iter()
        .map(|run| (run.id, run))
        .collect::<std::collections::HashMap<_, _>>();
    let mut count = 0i64;
    for attempt in attempts {
        let Some(run) = runs.get(&attempt.run_id) else {
            continue;
        };
        let manifest: mobile_qa_contracts::execution::RunManifest = decode(run.manifest.clone())?;
        if !run.cancel_requested
            && !reserved_apps.contains(&format!("app:{}", run.app_id))
            && bindings.contains(&(run.app_id, manifest.profile.id))
        {
            count += 1;
        }
    }
    for session in phone_sessions::Entity::find()
        .filter(phone_sessions::Column::Deadline.gt(Utc::now()))
        .all(db)
        .await?
    {
        if session
            .payload
            .get("state")
            .and_then(|value| value.as_str())
            == Some("queued")
            && !reserved_apps.contains(&format!("app:{}", session.app_id))
            && bindings.contains(&(session.app_id, session.profile_id))
        {
            count += 1;
        }
    }
    Ok(count)
}

pub async fn snapshot(db: &impl ConnectionTrait, pool: Uuid) -> ApiResult<CapacitySnapshot> {
    let row = host_rows::Entity::find()
        .filter(host_rows::Column::PoolId.eq(pool))
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let id = row.id;
    let policy: PoolPolicy = decode(
        device_pools::Entity::find_by_id(pool)
            .one(db)
            .await?
            .ok_or_else(ApiFailure::missing)?
            .policy,
    )?;
    let queued = demand(db, id).await?;
    let active = device_hosts::active_work(db, id).await?;
    let operation = device_power_operations::Entity::find()
        .filter(device_power_operations::Column::HostId.eq(id))
        .filter(device_power_operations::Column::CompletedAt.is_null())
        .one(db)
        .await?
        .map(|operation| {
            Ok::<CapacityOperation, ApiFailure>(CapacityOperation {
                id: operation.id,
                action: decode(serde_json::Value::String(operation.action))?,
                created_at: operation.created_at,
            })
        })
        .transpose()?;
    Ok(CapacitySnapshot {
        pool_id: pool,
        host_id: id,
        instance_id: row.instance_id,
        desired_online: policy.enabled
            && (queued > 0 || active > 0 || scheduled(&policy, Utc::now())),
        policy,
        state: decode(serde_json::Value::String(row.state))?,
        observed_power: decode(serde_json::Value::String(row.observed_power))?,
        boot_id: row.boot_id,
        generation: row.generation,
        control_version: row.control_version,
        queued_jobs: queued,
        active_work: active,
        last_demand_at: row.last_demand_at,
        heartbeat_at: row.heartbeat_at,
        clean_at: row.clean_at,
        startup_started_at: row.startup_started_at,
        current_operation: operation,
        reason: row.reason,
        cache_bytes: row.cache_bytes,
    })
}

/// Called in the same app transaction that inserts work. Even a lost HTTP hint leaves durable demand.
pub async fn enqueue(db: &impl ConnectionTrait, app: Uuid, profile: Uuid) -> ApiResult<()> {
    let slot_ids = device_slot_bindings::Entity::find()
        .filter(device_slot_bindings::Column::AppId.eq(app))
        .filter(device_slot_bindings::Column::ProfileId.eq(profile))
        .all(db)
        .await?
        .into_iter()
        .map(|binding| binding.slot_id)
        .collect::<Vec<_>>();
    if slot_ids.is_empty() {
        return Ok(());
    }
    let mut host_ids = device_slots::Entity::find()
        .filter(device_slots::Column::Id.is_in(slot_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|slot| slot.host_id)
        .collect::<Vec<_>>();
    host_ids.sort();
    host_ids.dedup();
    for id in host_ids {
        let locked = device_hosts::host(db, id, true).await?;
        // Demand before stop commit cancels drain. A committed stop can only finish and reboot.
        let pool_id = locked.model.pool_id;
        let was_draining = locked.model.state == "draining";
        let mut model = locked.model.into_active_model();
        model.last_demand_at = Set(Utc::now());
        if was_draining {
            model.state = Set("ready".into());
        }
        model.clean_at = Set(None);
        model.update(db).await?;
        capacity_wake_outbox::ActiveModel {
            id: Set(Uuid::new_v4()),
            pool_id: Set(pool_id),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

pub async fn action(
    ctx: &AppContext,
    pool: Uuid,
    input: CapacityActionRequest,
) -> ApiResult<CapacitySnapshot> {
    if input.reason.as_ref().is_some_and(|r| r.len() > 500) {
        return Err(ApiFailure::invalid("Control reason is too long"));
    }
    let fingerprint = hash(serde_json::to_vec(&input).map_err(|_| ApiFailure::internal())?);
    let tx = ctx.db.begin().await?;
    let row = device_hosts::host(&tx, input.host_id, true).await?;
    if row.model.pool_id != pool {
        return Err(ApiFailure::missing());
    }
    let replay = device_control_receipts::Entity::find_by_id((pool, input.request_id))
        .one(&tx)
        .await?;
    if let Some(old) = replay {
        if old.fingerprint != fingerprint {
            return Err(conflict(
                "Control request ID was reused with different content",
            ));
        }
        let result = snapshot(&tx, pool).await?;
        tx.commit().await?;
        return Ok(result);
    }
    if row.model.control_version != input.expected_control_version {
        return Err(conflict("Control snapshot is stale"));
    }
    let snap = snapshot(&tx, pool).await?;
    let now = Utc::now();
    match input.action {
        CapacityAction::Observe | CapacityAction::RecordPower => {
            let power = input
                .observed_power
                .as_ref()
                .ok_or_else(|| ApiFailure::invalid("Observed power is required"))?;
            if input.action == CapacityAction::RecordPower {
                let operation = snap
                    .current_operation
                    .as_ref()
                    .ok_or_else(|| conflict("No pending power operation"))?;
                if Some(operation.id) != input.operation_id {
                    return Err(conflict("Power operation is stale"));
                }
                let reached = matches!(
                    (&operation.action, power),
                    (CapacityAction::Start, ObservedPower::Running)
                        | (CapacityAction::CommitStop, ObservedPower::Stopped)
                );
                if reached {
                    if let Some(operation) =
                        device_power_operations::Entity::find_by_id(operation.id)
                            .filter(device_power_operations::Column::CompletedAt.is_null())
                            .one(&tx)
                            .await?
                    {
                        let mut operation = operation.into_active_model();
                        operation.completed_at = Set(Some(Utc::now()));
                        operation.update(&tx).await?;
                    }
                }
            }
            let current = host_rows::Entity::find_by_id(input.host_id)
                .one(&tx)
                .await?
                .ok_or_else(ApiFailure::missing)?;
            let pending_operation = device_power_operations::Entity::find()
                .filter(device_power_operations::Column::HostId.eq(input.host_id))
                .filter(device_power_operations::Column::CompletedAt.is_null())
                .one(&tx)
                .await?
                .is_some();
            let power_word = word(power);
            let next_state = if matches!(current.state.as_str(), "stop_committed" | "stopping")
                && power_word == "stopping"
            {
                Some("stopping")
            } else if matches!(current.state.as_str(), "stop_committed" | "stopping")
                && power_word == "stopped"
                && !pending_operation
            {
                Some("stopped")
            } else {
                None
            };
            let mut current = current.into_active_model();
            current.observed_power = Set(power_word);
            if let Some(next_state) = next_state {
                current.state = Set(next_state.into());
            }
            current.update(&tx).await?;
            if snap.state == HostState::Stopped
                && matches!(power, ObservedPower::Running | ObservedPower::Pending)
            {
                quarantine(
                    &tx,
                    input.host_id,
                    "Unexpected running compute; operator commissioning or recovery required",
                )
                .await?;
            }
            if matches!(snap.state, HostState::Ready | HostState::Draining)
                && snap.heartbeat_at.is_none_or(|at| {
                    at < now - chrono::Duration::seconds(device_hosts::HEARTBEAT_SECONDS)
                })
            {
                quarantine(
                    &tx,
                    input.host_id,
                    "Host heartbeat expired; preserve ownership for recovery",
                )
                .await?;
            }
            if snap.state == HostState::Starting
                && snap
                    .startup_started_at
                    .is_some_and(|at| at < now - chrono::Duration::minutes(15))
            {
                quarantine(&tx, input.host_id, "Host startup deadline exceeded").await?;
            }
            if snap
                .current_operation
                .as_ref()
                .is_some_and(|op| op.created_at < now - chrono::Duration::minutes(15))
            {
                quarantine(
                    &tx,
                    input.host_id,
                    "Power operation outcome is unresolved; operator observation required",
                )
                .await?;
            }
        }
        CapacityAction::Start => {
            if !snap.desired_online
                || snap.observed_power != ObservedPower::Stopped
                || snap.state != HostState::Stopped
                || snap.current_operation.is_some()
            {
                return Err(conflict("Host cannot start from this state"));
            }
            let current = host_rows::Entity::find_by_id(input.host_id)
                .one(&tx)
                .await?
                .ok_or_else(ApiFailure::missing)?;
            let mut current = current.into_active_model();
            current.state = Set("starting".into());
            current.startup_started_at = Set(Some(Utc::now()));
            current.clean_at = Set(None);
            current.heartbeat_at = Set(None);
            current.update(&tx).await?;
            operation(&tx, &snap, CapacityAction::Start).await?;
        }
        CapacityAction::Drain => {
            if snap.desired_online
                || snap.active_work > 0
                || snap.state != HostState::Ready
                || snap.observed_power != ObservedPower::Running
                || snap.current_operation.is_some()
                || now.signed_duration_since(snap.last_demand_at).num_seconds()
                    < i64::from(snap.policy.idle_seconds)
            {
                return Err(conflict("Host is not eligible for idle drain"));
            }
            let current = host_rows::Entity::find_by_id(input.host_id)
                .one(&tx)
                .await?
                .ok_or_else(ApiFailure::missing)?;
            let mut current = current.into_active_model();
            current.state = Set("draining".into());
            current.clean_at = Set(None);
            current.update(&tx).await?;
        }
        CapacityAction::CommitStop => {
            if snap.desired_online
                || snap.active_work > 0
                || snap.state != HostState::Draining
                || snap.current_operation.is_some()
                || snap.observed_power != ObservedPower::Running
                || snap
                    .clean_at
                    .is_none_or(|at| at < now - chrono::Duration::seconds(60))
                || snap.heartbeat_at.is_none_or(|at| {
                    at < now - chrono::Duration::seconds(device_hosts::HEARTBEAT_SECONDS)
                })
            {
                return Err(conflict(
                    "Fresh drain receipt and absence of demand are required",
                ));
            }
            let current = host_rows::Entity::find_by_id(input.host_id)
                .one(&tx)
                .await?
                .ok_or_else(ApiFailure::missing)?;
            let mut current = current.into_active_model();
            current.state = Set("stop_committed".into());
            current.update(&tx).await?;
            operation(&tx, &snap, CapacityAction::CommitStop).await?;
        }
        CapacityAction::Quarantine => {
            quarantine(
                &tx,
                input.host_id,
                input
                    .reason
                    .as_deref()
                    .unwrap_or("Controller reported an uncertain host state"),
            )
            .await?;
        }
    }
    let current = host_rows::Entity::find_by_id(input.host_id)
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let mut current = current.into_active_model();
    current.control_version = Set(current.control_version.take().unwrap_or_default() + 1);
    current.update(&tx).await?;
    device_control_receipts::ActiveModel {
        pool_id: Set(pool),
        request_id: Set(input.request_id),
        fingerprint: Set(fingerprint),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    let result = snapshot(&tx, pool).await?;
    tx.commit().await?;
    Ok(result)
}
async fn quarantine(db: &impl ConnectionTrait, id: Uuid, reason: &str) -> ApiResult<()> {
    let current = host_rows::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let mut current = current.into_active_model();
    current.state = Set("quarantined".into());
    current.clean_at = Set(None);
    current.reason = Set(Some(reason.to_owned()));
    current.update(db).await?;
    Ok(())
}
async fn operation(
    db: &impl ConnectionTrait,
    s: &CapacitySnapshot,
    action: CapacityAction,
) -> ApiResult<()> {
    device_power_operations::ActiveModel {
        id: Set(Uuid::new_v4()),
        host_id: Set(s.host_id),
        action: Set(word(&action)),
        generation: Set(s.generation),
        control_version: Set(s.control_version + 1),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}

pub async fn consume_hint(
    ctx: &AppContext,
    pool: Uuid,
    input: ConsumeHintRequest,
) -> ApiResult<HintReceipt> {
    if Utc::now().timestamp().abs_diff(input.timestamp) > 300 {
        return Err(conflict("Wake hint expired"));
    }
    let accepted = capacity_hint_receipts::Entity::insert(capacity_hint_receipts::ActiveModel {
        pool_id: Set(pool),
        request_id: Set(input.request_id),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::columns([
            capacity_hint_receipts::Column::PoolId,
            capacity_hint_receipts::Column::RequestId,
        ])
        .do_nothing()
        .to_owned(),
    )
    .exec_without_returning(&ctx.db)
    .await?;
    if accepted != 1 {
        return Err(conflict("Wake hint was already consumed"));
    }
    Ok(HintReceipt { accepted: true })
}

/// Customer readiness deliberately excludes instance IDs and provider details.
pub async fn queue_reason(
    db: &impl ConnectionTrait,
    app: Uuid,
    profile: Uuid,
) -> ApiResult<Option<mobile_qa_contracts::execution::QueueReason>> {
    use mobile_qa_contracts::execution::QueueReason;
    let slot_ids = device_slot_bindings::Entity::find()
        .filter(device_slot_bindings::Column::AppId.eq(app))
        .filter(device_slot_bindings::Column::ProfileId.eq(profile))
        .all(db)
        .await?
        .into_iter()
        .map(|binding| binding.slot_id)
        .collect::<Vec<_>>();
    let slots = if slot_ids.is_empty() {
        Vec::new()
    } else {
        device_slots::Entity::find()
            .filter(device_slots::Column::Id.is_in(slot_ids))
            .all(db)
            .await?
    };
    let Some(host_id) = slots.first().map(|slot| slot.host_id) else {
        return Ok(None);
    };
    let h = host_rows::Entity::find_by_id(host_id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let p: PoolPolicy = decode(
        device_pools::Entity::find_by_id(h.pool_id)
            .one(db)
            .await?
            .ok_or_else(ApiFailure::missing)?
            .policy,
    )?;
    if !p.enabled {
        return Ok(Some(QueueReason::PoolPaused));
    }
    Ok(match h.state.as_str() {
        "quarantined" => Some(QueueReason::DeviceRecoveryRequired),
        "draining" | "stop_committed" | "stopping" => Some(QueueReason::HostDraining),
        "stopped" | "starting" => Some(QueueReason::HostStarting),
        "ready" => {
            if h.heartbeat_at.is_none_or(|at| {
                at < Utc::now() - chrono::Duration::seconds(device_hosts::HEARTBEAT_SECONDS)
            }) {
                Some(QueueReason::WorkerOffline)
            } else if !slots.iter().any(|slot| {
                slot.host_id == h.id && matches!(slot.state.as_str(), "idle" | "preparing")
            }) {
                Some(QueueReason::WaitingForSlot)
            } else {
                None
            }
        }
        _ => Some(QueueReason::ProfileIncompatible),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn policy() -> PoolPolicy {
        PoolPolicy {
            enabled: true,
            idle_seconds: 900,
            warm_target: 1,
            timezone: "America/Toronto".into(),
            warm_windows: vec![WarmWindow {
                weekdays: vec![6],
                start: "01:00".into(),
                end: "02:00".into(),
            }],
        }
    }
    #[test]
    fn warm_windows_handle_dst_and_overnight() {
        let p = policy();
        validate_policy(&p).unwrap();
        for at in ["2026-11-01T05:30:00Z", "2026-11-01T06:30:00Z"] {
            assert!(scheduled(&p, at.parse().unwrap()));
        }
        assert!(!scheduled(&p, "2026-11-01T07:00:00Z".parse().unwrap()));
        let mut p = p;
        p.warm_windows = vec![WarmWindow {
            weekdays: vec![0],
            start: "23:00".into(),
            end: "02:00".into(),
        }];
        assert!(scheduled(&p, "2026-09-22T05:00:00Z".parse().unwrap()));
        assert!(!scheduled(&p, "2026-09-22T06:00:00Z".parse().unwrap()));
    }
    #[test]
    fn malformed_policy_is_rejected() {
        let mut p = policy();
        p.timezone = "not-a-zone".into();
        assert!(validate_policy(&p).is_err());
        p = policy();
        p.warm_windows[0].start = "25:00".into();
        assert!(validate_policy(&p).is_err());
        p = policy();
        p.enabled = false;
        assert!(!scheduled(&p, Utc::now()));
    }
}
