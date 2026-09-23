//! Host identity is separate from app authority. Only operator bindings issue worker grants.
use super::{
    execution_store::{conflict, decode, hash, json, word},
    test_definitions,
    worker_auth::Worker,
};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        apps, device_hosts as host_rows, device_pools, device_power_operations,
        device_slot_bindings, device_slots, execution_attempts, execution_reservations,
        execution_workers, phone_sessions,
    },
};
use chrono::Utc;
use loco_rs::app::AppContext;
use mobile_qa_contracts::device_hosts::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel,
    QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use uuid::Uuid;

pub const HEARTBEAT_SECONDS: i64 = 90;

pub struct HostRecord {
    pub model: host_rows::Model,
    pub policy: PoolPolicy,
}

pub async fn authenticate_host(db: &impl ConnectionTrait, id: Uuid, token: &str) -> ApiResult<()> {
    if !(32..=256).contains(&token.len()) {
        return Err(ApiFailure::unauthorized());
    }
    host_rows::Entity::find_by_id(id)
        .filter(host_rows::Column::TokenHash.eq(hash(token)))
        .filter(host_rows::Column::Revoked.eq(false))
        .one(db)
        .await
        .map_err(|_| ApiFailure::unauthorized())?
        .ok_or_else(ApiFailure::unauthorized)?;
    Ok(())
}

pub async fn authenticate_control(
    db: &impl ConnectionTrait,
    pool: Uuid,
    token: &str,
) -> ApiResult<()> {
    if !(32..=256).contains(&token.len()) {
        return Err(ApiFailure::unauthorized());
    }
    device_pools::Entity::find_by_id(pool)
        .filter(device_pools::Column::ControlTokenHash.eq(hash(token)))
        .one(db)
        .await
        .map_err(|_| ApiFailure::unauthorized())?
        .ok_or_else(ApiFailure::unauthorized)?;
    Ok(())
}

pub async fn host(
    db: &impl sea_orm::ConnectionTrait,
    id: Uuid,
    lock: bool,
) -> ApiResult<HostRecord> {
    let query = host_rows::Entity::find_by_id(id);
    let model = if lock {
        query.lock_exclusive().one(db).await?
    } else {
        query.one(db).await?
    }
    .ok_or_else(ApiFailure::missing)?;
    let pool = device_pools::Entity::find_by_id(model.pool_id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    Ok(HostRecord {
        model,
        policy: decode(pool.policy)?,
    })
}

pub fn same_boot(row: &HostRecord, boot: Uuid, generation: i32) -> ApiResult<()> {
    if row.model.boot_id != Some(boot) || row.model.generation != generation {
        return Err(conflict("Host boot generation is stale"));
    }
    Ok(())
}

pub async fn status(db: &impl ConnectionTrait, id: Uuid) -> ApiResult<HostStatus> {
    let row = host(db, id, false).await?;
    let slot_models = device_slots::Entity::find()
        .filter(device_slots::Column::HostId.eq(id))
        .order_by_asc(device_slots::Column::SlotIndex)
        .all(db)
        .await?;
    let slots = slot_models
        .iter()
        .map(|slot| {
            Ok(HostSlotStatus {
                definition: decode(slot.definition.clone())?,
                state: decode(serde_json::Value::String(slot.state.clone()))?,
                emulator_boot_id: slot.emulator_boot_id,
            })
        })
        .collect::<ApiResult<_>>()?;
    let mut bindings = Vec::new();
    for slot in &slot_models {
        bindings.extend(
            device_slot_bindings::Entity::find()
                .filter(device_slot_bindings::Column::SlotId.eq(slot.id))
                .order_by_asc(device_slot_bindings::Column::AppId)
                .order_by_asc(device_slot_bindings::Column::ProfileId)
                .all(db)
                .await?
                .into_iter()
                .map(|binding| SlotBinding {
                    slot_id: binding.slot_id,
                    app_id: binding.app_id,
                    profile_id: binding.profile_id,
                }),
        );
    }
    Ok(HostStatus {
        host_id: id,
        pool_id: row.model.pool_id,
        boot_id: row.model.boot_id,
        generation: row.model.generation,
        state: decode(serde_json::Value::String(row.model.state))?,
        heartbeat_seconds: 20,
        policy: row.policy,
        slots,
        bindings,
    })
}

/// Registrations are explicit operator maintenance; instance and qualification are never host supplied.
pub async fn provision(
    ctx: &AppContext,
    actor: Uuid,
    input: PoolRegistration,
    host_token: &str,
    control_token: &str,
) -> ApiResult<()> {
    validate_registration(&input)?;
    if [host_token, control_token]
        .iter()
        .any(|s| !(32..=256).contains(&s.len()))
    {
        return Err(ApiFailure::invalid(
            "Inject distinct host and controller credentials of at least 32 bytes",
        ));
    }
    if host_token == control_token {
        return Err(ApiFailure::invalid(
            "Host and controller credentials must differ",
        ));
    }
    for binding in &input.bindings {
        test_definitions::operator(ctx, actor, binding.app_id).await?;
        let p = test_definitions::profile(&ctx.db, binding.app_id, binding.profile_id).await?;
        p.validate().map_err(ApiFailure::invalid)?;
        let slot = input
            .slots
            .iter()
            .find(|s| s.id == binding.slot_id)
            .ok_or_else(|| ApiFailure::invalid("Binding refers to an unknown slot"))?;
        if !p.qualified
            || p.device_identity != slot.device_identity
            || p.image != slot.system_image
            || p.adapter != "android_direct_v1"
            || p.execution_context.is_none()
            || p.driver != mobile_qa_contracts::execution::Driver::Direct
        {
            return Err(ApiFailure::invalid(
                "Binding requires a qualified profile for this physical slot",
            ));
        }
    }
    let tx = ctx.db.begin().await?;
    // Binding a previously standalone device must wait for every pre-binding claim transaction.
    // Otherwise a legacy claim could publish its reservation after the first hosted drain check.
    let mut apps: Vec<_> = input.bindings.iter().map(|b| b.app_id).collect();
    apps.sort();
    apps.dedup();
    for app in apps {
        apps::Entity::find_by_id(app)
            .lock_exclusive()
            .one(&tx)
            .await?
            .ok_or_else(ApiFailure::missing)?;
    }
    device_pools::ActiveModel {
        id: Set(input.pool_id),
        policy: Set(json(&input.policy)?),
        control_token_hash: Set(hash(control_token)),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    host_rows::ActiveModel {
        id: Set(input.host_id),
        pool_id: Set(input.pool_id),
        instance_id: Set(input.instance_id),
        token_hash: Set(hash(host_token)),
        toolchain_digest: Set(input.toolchain_digest),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    for slot in input.slots {
        device_slots::ActiveModel {
            id: Set(slot.id),
            host_id: Set(input.host_id),
            slot_index: Set(slot.index as i32),
            definition: Set(json(&slot)?),
            ..Default::default()
        }
        .insert(&tx)
        .await?;
    }
    for b in input.bindings {
        device_slot_bindings::ActiveModel {
            slot_id: Set(b.slot_id),
            app_id: Set(b.app_id),
            profile_id: Set(b.profile_id),
        }
        .insert(&tx)
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

fn validate_registration(input: &PoolRegistration) -> ApiResult<()> {
    super::capacity_control::validate_policy(&input.policy)?;
    if input.slots.is_empty()
        || input.slots.len() > 16
        || input.bindings.is_empty()
        || input.bindings.len() > 100
        || !input.instance_id.starts_with("i-")
        || input.instance_id.len() > 32
        || input.toolchain_digest.len() != 64
        || !input
            .toolchain_digest
            .bytes()
            .all(|b| b.is_ascii_hexdigit())
    {
        return Err(ApiFailure::invalid("Invalid fixed-host registration"));
    }
    let mut ids = std::collections::HashSet::new();
    let mut indices = std::collections::HashSet::new();
    let mut ports = std::collections::HashSet::new();
    let mut identities = std::collections::HashSet::new();
    for s in &input.slots {
        if !ids.insert(s.id)
            || !indices.insert(s.index)
            || s.index > 15
            || !identities.insert(&s.device_identity)
            || s.device_identity.is_empty()
            || s.device_identity.len() > 200
            || !s.qualified
            || s.qualification_reference.is_empty()
            || s.qualification_reference.len() > 500
            || !(1024..=32768).contains(&s.memory_mb)
            || !(1..=16).contains(&s.cpu_cores)
            || !(5554..=5680).contains(&s.console_port)
            || s.console_port % 2 != 0
            || s.adb_server_port < 1024
            || !ports.insert(s.console_port)
            || !ports.insert(s.console_port + 1)
            || !ports.insert(s.adb_server_port)
            || !s.system_image.starts_with("system-images;android-")
            || s.system_image.len() > 150
        {
            return Err(ApiFailure::invalid(
                "Invalid or overlapping qualified slot resources",
            ));
        }
    }
    if input.policy.warm_target as usize > input.slots.iter().filter(|s| s.warm_qualified).count() {
        return Err(ApiFailure::invalid(
            "Warm slots need explicit reset qualification",
        ));
    }
    Ok(())
}

/// Reboots fence credentials; an unresolved reservation always requires explicit recovery.
pub async fn register(
    ctx: &AppContext,
    id: Uuid,
    input: HostRegisterRequest,
) -> ApiResult<HostStatus> {
    if input.version != 1 {
        return Err(conflict("Unsupported host protocol"));
    }
    let tx = ctx.db.begin().await?;
    let row = host(&tx, id, true).await?;
    if row.model.toolchain_digest != input.toolchain_digest {
        return Err(conflict("Host toolchain has not been qualified"));
    }
    let state = row.model.state.clone();
    if matches!(state.as_str(), "stop_committed" | "stopping") {
        return Err(conflict(
            "Power operation must finish before registering a boot",
        ));
    }
    if row.model.boot_id != Some(input.boot_id) {
        let active = active_work(&tx, id).await?;
        let needs_recovery = active > 0 || state == "quarantined";
        let mut model = row.model.into_active_model();
        model.boot_id = Set(Some(input.boot_id));
        model.generation = Set(model.generation.take().unwrap_or_default() + 1);
        model.state = Set(if needs_recovery {
            "quarantined"
        } else {
            "ready"
        }
        .into());
        model.heartbeat_at = Set(Some(Utc::now()));
        model.clean_at = Set(None);
        model.reason =
            Set(needs_recovery.then(|| "Previous boot requires operator recovery".to_owned()));
        model.update(&tx).await?;
        let slot_ids = device_slots::Entity::find()
            .filter(device_slots::Column::HostId.eq(id))
            .all(&tx)
            .await?
            .into_iter()
            .map(|slot| slot.id)
            .collect::<Vec<_>>();
        if !slot_ids.is_empty() {
            execution_workers::Entity::update_many()
                .col_expr(
                    execution_workers::Column::Revoked,
                    sea_orm::sea_query::Expr::value(true),
                )
                .filter(execution_workers::Column::HostSlotId.is_in(slot_ids))
                .exec(&tx)
                .await?;
        }
        device_slots::Entity::update_many()
            .col_expr(
                device_slots::Column::State,
                sea_orm::sea_query::Expr::value(if needs_recovery {
                    "quarantined"
                } else {
                    "offline"
                }),
            )
            .col_expr(
                device_slots::Column::EmulatorBootId,
                sea_orm::sea_query::Expr::value(Option::<Uuid>::None),
            )
            .filter(device_slots::Column::HostId.eq(id))
            .exec(&tx)
            .await?;
    } else {
        let mut model = row.model.into_active_model();
        model.heartbeat_at = Set(Some(Utc::now()));
        model.update(&tx).await?;
    }
    tx.commit().await?;
    status(&ctx.db, id).await
}

pub async fn heartbeat(
    ctx: &AppContext,
    id: Uuid,
    input: HostHeartbeatRequest,
) -> ApiResult<HostStatus> {
    if input.cache_bytes < 0 || input.slots.len() > 16 {
        return Err(ApiFailure::invalid("Invalid host heartbeat"));
    }
    let tx = ctx.db.begin().await?;
    let row = host(&tx, id, true).await?;
    same_boot(&row, input.boot_id, input.generation)?;
    if matches!(
        row.model.state.as_str(),
        "stopped" | "stop_committed" | "stopping"
    ) {
        return Err(conflict("Host has been fenced for power-off"));
    }
    let mut seen = std::collections::HashSet::new();
    for slot in input.slots {
        if !seen.insert(slot.slot_id) {
            return Err(ApiFailure::invalid("Duplicate slot heartbeat"));
        }
        let old = device_slots::Entity::find_by_id(slot.slot_id)
            .filter(device_slots::Column::HostId.eq(id))
            .lock_exclusive()
            .one(&tx)
            .await?
            .ok_or_else(ApiFailure::missing)?;
        let busy = slot_active(&tx, slot.slot_id).await? > 0;
        // Reservations are authoritative, and quarantine cannot be cleared by a heartbeat.
        let new_state = if old.state == "quarantined" {
            "quarantined".to_owned()
        } else if busy && slot.state != SlotState::Quarantined {
            "leased".into()
        } else {
            word(&slot.state)
        };
        let mut old = old.into_active_model();
        old.state = Set(new_state);
        old.emulator_boot_id = Set(slot.emulator_boot_id);
        old.update(&tx).await?;
    }
    let quarantined = device_slots::Entity::find()
        .filter(device_slots::Column::HostId.eq(id))
        .filter(device_slots::Column::State.eq("quarantined"))
        .one(&tx)
        .await?
        .is_some();
    let active = active_work(&tx, id).await? > 0;
    let mut model = row.model.into_active_model();
    if quarantined {
        model.state = Set("quarantined".into());
        model.reason = Set(Some("A slot requires operator recovery".into()));
        model.clean_at = Set(None);
    }
    let now = Utc::now();
    model.heartbeat_at = Set(Some(now));
    model.cache_bytes = Set(input.cache_bytes);
    if active {
        model.last_demand_at = Set(now);
        model.clean_at = Set(None);
    }
    model.update(&tx).await?;
    tx.commit().await?;
    status(&ctx.db, id).await
}

pub async fn grant(
    ctx: &AppContext,
    id: Uuid,
    slot: Uuid,
    input: SlotGrantRequest,
) -> ApiResult<SlotGrantResponse> {
    let tx = ctx.db.begin().await?;
    // The same lock order is used by every scheduler: app, host, then slot.
    apps::Entity::find_by_id(input.app_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let row = host(&tx, id, true).await?;
    same_boot(&row, input.boot_id, input.generation)?;
    require_ready(&row)?;
    device_slot_bindings::Entity::find_by_id((slot, input.app_id, input.profile_id))
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let s = device_slots::Entity::find_by_id(slot)
        .filter(device_slots::Column::HostId.eq(id))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let definition: SlotDefinition = decode(s.definition.clone())?;
    let profile = test_definitions::profile(&tx, input.app_id, input.profile_id).await?;
    profile.validate().map_err(ApiFailure::invalid)?;
    if !definition.qualified
        || !profile.qualified
        || profile.device_identity != definition.device_identity
        || profile.image != definition.system_image
        || profile.adapter != "android_direct_v1"
        || profile.execution_context.is_none()
        || profile.driver != mobile_qa_contracts::execution::Driver::Direct
        || !matches!(s.state.as_str(), "idle" | "preparing")
        || slot_active(&tx, slot).await? > 0
    {
        return Err(conflict("Slot is not available for this qualified profile"));
    }
    // Rotation never interrupts accepted work: no reservation may remain at this point.
    execution_workers::Entity::update_many()
        .col_expr(
            execution_workers::Column::Revoked,
            sea_orm::sea_query::Expr::value(true),
        )
        .filter(execution_workers::Column::HostSlotId.eq(slot))
        .exec(&tx)
        .await?;
    let worker_id = Uuid::new_v4();
    let token = loco_rs::hash::random_string(64);
    execution_workers::ActiveModel {
        id: Set(worker_id),
        app_id: Set(input.app_id),
        profile_id: Set(input.profile_id),
        token_hash: Set(hash(&token)),
        host_slot_id: Set(Some(slot)),
        host_generation: Set(Some(input.generation)),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    tx.commit().await?;
    Ok(SlotGrantResponse {
        worker_id,
        app_id: input.app_id,
        profile_id: input.profile_id,
        worker_token: token,
    })
}

fn require_ready(row: &HostRecord) -> ApiResult<()> {
    if !row.policy.enabled
        || row.model.state != "ready"
        || row
            .model
            .heartbeat_at
            .is_none_or(|at| at < Utc::now() - chrono::Duration::seconds(HEARTBEAT_SECONDS))
    {
        return Err(conflict("Host is unavailable for new claims"));
    }
    Ok(())
}

/// Called after the scheduler has locked its app row, before selecting any work.
pub async fn claim_fence(
    db: &impl ConnectionTrait,
    worker: &Worker,
    claim_id: Uuid,
) -> ApiResult<bool> {
    let w = execution_workers::Entity::find_by_id(worker.id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if w.revoked {
        return Err(ApiFailure::unauthorized());
    }
    let Some(slot) = w.host_slot_id else {
        return Ok(device_slot_bindings::Entity::find()
            .filter(device_slot_bindings::Column::AppId.eq(worker.app_id))
            .filter(device_slot_bindings::Column::ProfileId.eq(worker.profile_id))
            .one(db)
            .await?
            .is_none());
    };
    let parent = device_slots::Entity::find_by_id(slot)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let h = host(db, parent.host_id, true).await?;
    let s = device_slots::Entity::find_by_id(slot)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    // A grant may rotate while this request waits for the host lock. Re-read authority under that lock.
    let w = execution_workers::Entity::find_by_id(worker.id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if w.revoked || w.host_generation != Some(h.model.generation) {
        return Err(ApiFailure::unauthorized());
    }
    let prior_attempt = execution_attempts::Entity::find()
        .filter(execution_attempts::Column::WorkerId.eq(worker.id))
        .filter(execution_attempts::Column::ClaimId.eq(claim_id))
        .one(db)
        .await?
        .is_some();
    let prior_phone = phone_sessions::Entity::find()
        .filter(phone_sessions::Column::WorkerId.eq(worker.id))
        .filter(phone_sessions::Column::ClaimId.eq(claim_id))
        .one(db)
        .await?
        .is_some();
    if prior_attempt || prior_phone {
        // Execution claims may recover a lost delivery. Phone claims preserve their explicit replay error.
        return Ok(matches!(h.model.state.as_str(), "ready" | "draining"));
    }
    if require_ready(&h).is_err()
        || !matches!(s.state.as_str(), "idle" | "preparing")
        || slot_active(db, slot).await? > 0
    {
        return Ok(false);
    }
    let d: SlotDefinition = decode(s.definition)?;
    let p = test_definitions::profile(db, worker.app_id, worker.profile_id).await?;
    Ok(p.validate().is_ok()
        && p.driver == mobile_qa_contracts::execution::Driver::Direct
        && d.qualified
        && p.qualified
        && d.device_identity == p.device_identity
        && d.system_image == p.image
        && p.adapter == "android_direct_v1"
        && p.execution_context.is_some())
}

pub async fn mark_leased(db: &impl ConnectionTrait, worker: Uuid) -> ApiResult<()> {
    let worker = execution_workers::Entity::find_by_id(worker)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let Some(slot_id) = worker.host_slot_id else {
        // Legacy fixture/local workers are intentionally not attached to managed hosts.
        return Ok(());
    };
    let slot = device_slots::Entity::find_by_id(slot_id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let host_id = slot.host_id;
    let mut slot = slot.into_active_model();
    slot.state = Set("leased".into());
    slot.update(db).await?;
    let host = host_rows::Entity::find_by_id(host_id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let mut host = host.into_active_model();
    host.clean_at = Set(None);
    host.last_demand_at = Set(Utc::now());
    host.update(db).await?;
    Ok(())
}

pub async fn slot_active(db: &impl ConnectionTrait, slot: Uuid) -> ApiResult<i64> {
    let slot = device_slots::Entity::find_by_id(slot)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    active_for_slots(db, vec![slot]).await
}
pub async fn active_work(db: &impl ConnectionTrait, id: Uuid) -> ApiResult<i64> {
    let slots = device_slots::Entity::find()
        .filter(device_slots::Column::HostId.eq(id))
        .all(db)
        .await?;
    active_for_slots(db, slots).await
}

async fn active_for_slots(
    db: &impl ConnectionTrait,
    slots: Vec<device_slots::Model>,
) -> ApiResult<i64> {
    if slots.is_empty() {
        return Ok(0);
    }
    let mut resources = std::collections::HashSet::new();
    let slot_ids = slots
        .into_iter()
        .map(|slot| {
            let definition: SlotDefinition = decode(slot.definition)?;
            resources.insert(format!("device:{}", definition.device_identity));
            Ok(slot.id)
        })
        .collect::<ApiResult<Vec<_>>>()?;
    let worker_ids = execution_workers::Entity::find()
        .filter(execution_workers::Column::HostSlotId.is_in(slot_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|worker| worker.id)
        .collect::<Vec<_>>();
    let attempt_ids = if worker_ids.is_empty() {
        Vec::new()
    } else {
        execution_attempts::Entity::find()
            .filter(execution_attempts::Column::WorkerId.is_in(worker_ids.clone()))
            .all(db)
            .await?
            .into_iter()
            .map(|attempt| attempt.id)
            .collect()
    };
    let session_ids = if worker_ids.is_empty() {
        Vec::new()
    } else {
        phone_sessions::Entity::find()
            .filter(phone_sessions::Column::WorkerId.is_in(worker_ids))
            .all(db)
            .await?
            .into_iter()
            .map(|session| session.id)
            .collect()
    };
    let reservations = execution_reservations::Entity::find().all(db).await?;
    let owners = reservations
        .into_iter()
        .filter(|reservation| {
            resources.contains(&reservation.resource)
                || reservation
                    .attempt_id
                    .is_some_and(|id| attempt_ids.contains(&id))
                || reservation
                    .session_id
                    .is_some_and(|id| session_ids.contains(&id))
        })
        .map(|reservation| (reservation.attempt_id, reservation.session_id))
        .collect::<std::collections::HashSet<_>>();
    Ok(owners.len() as i64)
}

pub async fn cleanup(
    ctx: &AppContext,
    id: Uuid,
    input: HostCleanupRequest,
) -> ApiResult<HostStatus> {
    let tx = ctx.db.begin().await?;
    let row = host(&tx, id, true).await?;
    same_boot(&row, input.boot_id, input.generation)?;
    if row.model.state != "draining" {
        return Err(conflict("Host is not draining"));
    }
    if !input.processes_stopped
        || !input.ports_released
        || !input.journals_resolved
        || active_work(&tx, id).await? > 0
    {
        let mut model = row.model.into_active_model();
        model.state = Set("quarantined".into());
        model.clean_at = Set(None);
        model.reason = Set(Some("Cleanup did not prove resource release".into()));
        model.update(&tx).await?;
        tx.commit().await?;
        return status(&ctx.db, id).await;
    }
    if row
        .model
        .heartbeat_at
        .is_none_or(|at| at < Utc::now() - chrono::Duration::seconds(HEARTBEAT_SECONDS))
    {
        return Err(conflict("A fresh host heartbeat is required"));
    }
    device_slots::Entity::update_many()
        .col_expr(
            device_slots::Column::State,
            sea_orm::sea_query::Expr::value("offline"),
        )
        .col_expr(
            device_slots::Column::EmulatorBootId,
            sea_orm::sea_query::Expr::value(Option::<Uuid>::None),
        )
        .filter(device_slots::Column::HostId.eq(id))
        .exec(&tx)
        .await?;
    let mut model = row.model.into_active_model();
    model.clean_at = Set(Some(Utc::now()));
    model.update(&tx).await?;
    tx.commit().await?;
    status(&ctx.db, id).await
}

/// Pause drains existing work; recovery requires an explicit operator after resolving reservations.
pub async fn maintain(
    ctx: &AppContext,
    actor: Uuid,
    pool: Uuid,
    action: &str,
    token: Option<&str>,
) -> ApiResult<()> {
    let bindings = pool_app_ids(&ctx.db, pool).await?;
    if bindings.is_empty() {
        return Err(ApiFailure::missing());
    }
    for app_id in bindings {
        test_definitions::operator(ctx, actor, app_id).await?;
    }
    let tx = ctx.db.begin().await?;
    let host = host_rows::Entity::find()
        .filter(host_rows::Column::PoolId.eq(pool))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let id = host.id;
    match action {
        "pause-pool" | "resume-pool" => {
            let pool_model = device_pools::Entity::find_by_id(pool)
                .one(&tx)
                .await?
                .ok_or_else(ApiFailure::missing)?;
            let mut policy: PoolPolicy = decode(pool_model.policy.clone())?;
            policy.enabled = action == "resume-pool";
            let mut pool_model = pool_model.into_active_model();
            pool_model.policy = Set(json(&policy)?);
            pool_model.update(&tx).await?;
        }
        "recover-host" => {
            if active_work(&tx, id).await? > 0 {
                return Err(conflict(
                    "Recover every attempt and phone reservation first",
                ));
            }
            if device_power_operations::Entity::find()
                .filter(device_power_operations::Column::HostId.eq(id))
                .filter(device_power_operations::Column::CompletedAt.is_null())
                .one(&tx)
                .await?
                .is_some()
            {
                return Err(conflict(
                    "Resolve the pending AWS operation before recovering this host",
                ));
            }
            if host.observed_power != "stopped" {
                return Err(conflict(
                    "Observe stopped AWS power before recovering this host",
                ));
            }
            let mut host = host.clone().into_active_model();
            host.state = Set("stopped".into());
            host.boot_id = Set(None);
            host.heartbeat_at = Set(None);
            host.clean_at = Set(None);
            host.reason = Set(None);
            host.control_version = Set(host.control_version.take().unwrap_or_default() + 1);
            host.update(&tx).await?;
        }
        "rotate-host-token" | "rotate-control-token" => {
            let t = token
                .filter(|s| (32..=256).contains(&s.len()))
                .ok_or_else(|| ApiFailure::invalid("Inject a replacement token"))?;
            if action == "rotate-host-token" {
                let mut host = host.clone().into_active_model();
                host.token_hash = Set(hash(t));
                host.update(&tx).await?;
            } else {
                let pool_model = device_pools::Entity::find_by_id(pool)
                    .one(&tx)
                    .await?
                    .ok_or_else(ApiFailure::missing)?;
                let mut pool_model = pool_model.into_active_model();
                pool_model.control_token_hash = Set(hash(t));
                pool_model.update(&tx).await?;
            }
        }
        _ => return Err(ApiFailure::invalid("Unknown host maintenance operation")),
    }
    tx.commit().await?;
    Ok(())
}

async fn pool_app_ids(db: &impl ConnectionTrait, pool: Uuid) -> ApiResult<Vec<Uuid>> {
    let host_ids = host_rows::Entity::find()
        .filter(host_rows::Column::PoolId.eq(pool))
        .all(db)
        .await?
        .into_iter()
        .map(|host| host.id)
        .collect::<Vec<_>>();
    if host_ids.is_empty() {
        return Ok(Vec::new());
    }
    let slot_ids = device_slots::Entity::find()
        .filter(device_slots::Column::HostId.is_in(host_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|slot| slot.id)
        .collect::<Vec<_>>();
    if slot_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut apps = device_slot_bindings::Entity::find()
        .filter(device_slot_bindings::Column::SlotId.is_in(slot_ids))
        .all(db)
        .await?
        .into_iter()
        .map(|binding| binding.app_id)
        .collect::<Vec<_>>();
    apps.sort();
    apps.dedup();
    Ok(apps)
}

/// Explicit incident reconciliation after disabling the sole controller and verifying AWS externally.
/// This records the operator's evidence; it never releases device reservations or calls AWS.
#[allow(clippy::too_many_arguments)]
pub async fn resolve_operation(
    ctx: &AppContext,
    actor: Uuid,
    pool: Uuid,
    operation: Uuid,
    expected_version: i32,
    power: ObservedPower,
    controller_disabled: bool,
    evidence: &str,
) -> ApiResult<()> {
    if !controller_disabled
        || !matches!(power, ObservedPower::Stopped | ObservedPower::Running)
        || !(10..=500).contains(&evidence.len())
    {
        return Err(ApiFailure::invalid("Disable the controller and supply verified stable power plus an incident evidence reference"));
    }
    let bindings = pool_app_ids(&ctx.db, pool).await?;
    if bindings.is_empty() {
        return Err(ApiFailure::missing());
    }
    for app_id in bindings {
        test_definitions::operator(ctx, actor, app_id).await?;
    }
    let tx = ctx.db.begin().await?;
    let record = host_rows::Entity::find()
        .filter(host_rows::Column::PoolId.eq(pool))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let id = record.id;
    let policy: PoolPolicy = decode(
        device_pools::Entity::find_by_id(pool)
            .one(&tx)
            .await?
            .ok_or_else(ApiFailure::missing)?
            .policy,
    )?;
    if policy.enabled || record.state != "quarantined" || record.control_version != expected_version
    {
        return Err(conflict(
            "Pause and quarantine this host, then reload its control version",
        ));
    }
    let operation = device_power_operations::Entity::find_by_id(operation)
        .filter(device_power_operations::Column::HostId.eq(id))
        .filter(device_power_operations::Column::CompletedAt.is_null())
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| conflict("Pending operation changed"))?;
    let mut operation = operation.into_active_model();
    operation.completed_at = Set(Some(Utc::now()));
    operation.update(&tx).await?;
    let mut record = record.into_active_model();
    record.observed_power = Set(word(&power));
    record.reason = Set(Some(format!(
        "Operator {actor} reconciled power: {evidence}"
    )));
    record.control_version = Set(record.control_version.take().unwrap_or_default() + 1);
    record.update(&tx).await?;
    tx.commit().await?;
    Ok(())
}
