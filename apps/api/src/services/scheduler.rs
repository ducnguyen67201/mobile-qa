//! Short PostgreSQL leases plus persistent reservations; never replay device effects on expiry.
use super::{
    execution_store::{conflict, decode, hash, json, word},
    model_registry, runs, test_definitions,
    worker_auth::Worker,
};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        apps, execution_attempts, execution_events, execution_recovery_events,
        execution_reservations, execution_runs, execution_workers,
    },
};
use chrono::{Duration, Utc};
use loco_rs::app::AppContext;
use mobile_qa_contracts::execution::*;
use sea_orm::{
    sea_query::{LockBehavior, LockType, OnConflict},
    ActiveModelTrait,
    ActiveValue::Set,
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};
use uuid::Uuid;

pub async fn reconcile(ctx: &AppContext) -> ApiResult<()> {
    let expired = execution_attempts::Entity::find()
        .filter(execution_attempts::Column::State.is_in([
            "leased",
            "running",
            "finalizing",
            "cancel_requested",
        ]))
        .filter(execution_attempts::Column::ExpiresAt.lt(Utc::now()))
        .all(&ctx.db)
        .await?;
    for row in expired {
        let mut active = row.into_active_model();
        let generation = active.generation.take().unwrap_or_default();
        if active.outcome.as_ref().is_none() {
            active.outcome = Set(Some("inconclusive".into()));
        }
        active.state = Set("recovery_required".into());
        active.generation = Set(generation + 1);
        active.cleanup = Set("quarantined".into());
        active.reason = Set(Some("lease_expired".into()));
        active.update(&ctx.db).await?;
    }
    super::run_comparisons::finalize_pending(ctx).await?;
    Ok(())
}

#[derive(Clone)]
pub struct LeaseRecord {
    pub attempt: execution_attempts::Model,
    pub run: execution_runs::Model,
}

pub async fn lease(
    db: &impl ConnectionTrait,
    worker: &Worker,
    id: Uuid,
    generation: i32,
    token: &str,
    lock: bool,
) -> ApiResult<LeaseRecord> {
    super::worker_auth::lease_authority(db, worker).await?;
    let select = execution_attempts::Entity::find_by_id(id)
        .filter(execution_attempts::Column::WorkerId.eq(worker.id));
    let attempt = if lock {
        select.lock_exclusive().one(db).await?
    } else {
        select.one(db).await?
    }
    .ok_or_else(ApiFailure::missing)?;
    let run = execution_runs::Entity::find_by_id(attempt.run_id)
        .filter(execution_runs::Column::AppId.eq(worker.app_id))
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if token.len() < 32
        || attempt.lease_hash.as_deref() != Some(hash(token).as_str())
        || attempt.generation != generation
    {
        return Err(conflict("Lease authority is stale"));
    }
    let alive = attempt.expires_at.is_some_and(|value| value > Utc::now());
    if attempt.state == "recovery_required" || (!alive && attempt.state != "finished") {
        return Err(conflict("Lease expired; resource requires recovery"));
    }
    Ok(LeaseRecord { attempt, run })
}
fn token() -> String {
    loco_rs::hash::random_string(64)
}

/// Wait without holding a transaction or device reservation. Subscribe before the
/// first query so a commit between checking the queue and waiting cannot be lost.
pub async fn claim_wait(
    ctx: &AppContext,
    worker: &Worker,
    input: ClaimRequest,
    wait: std::time::Duration,
) -> ApiResult<ClaimResponse> {
    use tokio::time::{Duration, Instant};
    let mut wakeup = super::execution_wakeup::subscribe(ctx);
    let deadline = Instant::now() + wait.min(Duration::from_secs(30));
    loop {
        // Authentication may have been revoked while this HTTP request was asleep.
        let registered = execution_workers::Entity::find_by_id(worker.id)
            .filter(execution_workers::Column::AppId.eq(worker.app_id))
            .filter(execution_workers::Column::ProfileId.eq(worker.profile_id))
            .filter(execution_workers::Column::Revoked.eq(false))
            .one(&ctx.db)
            .await?;
        if registered.is_none() {
            return Err(ApiFailure::unauthorized());
        }
        let mut response = claim(ctx, worker, input.clone()).await?;
        if response.lease.is_some() || Instant::now() >= deadline {
            response.poll_after_seconds = 0;
            return Ok(response);
        }
        // Local commits wake immediately. Other API processes are discovered within
        // five seconds; PostgreSQL remains authoritative if a notification is lost.
        let until = deadline.min(Instant::now() + Duration::from_secs(5));
        tokio::select! {
            _ = wakeup.changed() => {},
            _ = tokio::time::sleep_until(until) => {},
        }
    }
}

pub async fn claim(
    ctx: &AppContext,
    worker: &Worker,
    input: ClaimRequest,
) -> ApiResult<ClaimResponse> {
    if ![1, 2, 3, 4, 5, 6].contains(&input.version) || input.profile_id != worker.profile_id {
        return Err(conflict("Unsupported protocol or worker profile"));
    }
    if let Some(capabilities) = input.model_capabilities.as_ref() {
        capabilities
            .validate(input.version as u32)
            .map_err(ApiFailure::invalid)?;
    }
    reconcile(ctx).await?;
    let tx = ctx.db.begin().await?;
    apps::Entity::find_by_id(worker.app_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if !super::device_hosts::claim_fence(&tx, worker, input.claim_id).await? {
        return Ok(ClaimResponse {
            lease: None,
            poll_after_seconds: 5,
        });
    }
    let profile = test_definitions::profile(&tx, worker.app_id, worker.profile_id).await?;
    profile.validate().map_err(ApiFailure::invalid)?;
    model_registry::advertise_execution(
        &ctx.db,
        worker.id,
        input.version,
        input.model_capabilities.as_ref(),
    )
    .await?;
    if profile.execution_context.is_some() && input.version < 3 {
        return Ok(ClaimResponse {
            lease: None,
            poll_after_seconds: 5,
        });
    }
    let prior = execution_attempts::Entity::find()
        .filter(execution_attempts::Column::WorkerId.eq(worker.id))
        .filter(execution_attempts::Column::ClaimId.eq(input.claim_id))
        .one(&tx)
        .await?;
    let id = if let Some(attempt) = prior {
        let run = execution_runs::Entity::find_by_id(attempt.run_id)
            .one(&tx)
            .await?
            .ok_or_else(ApiFailure::missing)?;
        if input.version < 4 && run.manifest.get("source").is_some() {
            return Err(conflict("This run needs an updated execution worker"));
        }
        if !["leased", "running", "finalizing", "cancel_requested"]
            .contains(&attempt.state.as_str())
        {
            return Err(conflict("Claim already ended; reconcile local journal"));
        }
        attempt.id
    } else {
        if !profile.qualified {
            return Err(ApiFailure::invalid("Worker profile is not qualified"));
        }
        let resources = [
            format!("app:{}", worker.app_id),
            format!("device:{}", profile.device_identity),
        ];
        if execution_reservations::Entity::find()
            .filter(execution_reservations::Column::Resource.is_in(resources.clone()))
            .one(&tx)
            .await?
            .is_some()
        {
            return Ok(ClaimResponse {
                lease: None,
                poll_after_seconds: 5,
            });
        }
        // Suite cases have no claim dependency on their displayed membership order.
        // Keep plan order, then use stable IDs for ties; reservations serialize the app.
        let candidates = execution_attempts::Entity::find()
            .find_both_related(execution_runs::Entity)
            .filter(execution_attempts::Column::State.eq("queued"))
            .filter(execution_runs::Column::AppId.eq(worker.app_id))
            .filter(execution_runs::Column::CancelRequested.eq(false))
            .order_by_asc(execution_runs::Column::CreatedAt)
            .order_by_asc(execution_runs::Column::Id)
            .order_by_asc(execution_attempts::Column::CaseIndex)
            .order_by_asc(execution_attempts::Column::Number)
            .order_by_asc(execution_attempts::Column::Id)
            .all(&tx)
            .await?;
        let mut selected = None;
        for group in candidates.chunk_by(|left, right| left.1.id == right.1.id) {
            let run = &group[0].1;
            let manifest: RunManifest = decode(run.manifest.clone())?;
            let has_direct = manifest.cases.iter().any(|case| {
                case.case
                    .actions
                    .iter()
                    .any(|action| action.kind == ActionKind::Direct)
            });
            if manifest.profile.id != worker.profile_id
                || (input.version < 4 && manifest.source.is_some())
                || (input.version < 2 && has_direct)
                || manifest.resolved_model.as_ref().is_some_and(|resolved| {
                    input.version < 5
                        || !model_registry::worker_matches(
                            resolved,
                            input.model_capabilities.as_ref(),
                        )
                })
            {
                continue;
            }
            let mut attempts = group.iter().map(|(attempt, _)| attempt).collect::<Vec<_>>();
            attempts.sort_by_key(|attempt| {
                (
                    if manifest.source.as_ref().is_some_and(|source| {
                        matches!(
                            source,
                            mobile_qa_contracts::regression::RunSource::SavedSuiteV1 { .. }
                        )
                    }) {
                        0
                    } else {
                        attempt.case_index
                    },
                    attempt.number,
                    attempt.id,
                )
            });
            for candidate in attempts {
                let locked = execution_attempts::Entity::find_by_id(candidate.id)
                    .filter(execution_attempts::Column::State.eq("queued"))
                    .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked)
                    .one(&tx)
                    .await?;
                if let Some(locked) = locked {
                    selected = Some((locked, manifest));
                    break;
                }
            }
            if selected.is_some() {
                break;
            }
        }
        let Some((attempt, _manifest)) = selected else {
            return Ok(ClaimResponse {
                lease: None,
                poll_after_seconds: 5,
            });
        };
        let id = attempt.id;
        for resource in resources {
            let inserted =
                execution_reservations::Entity::insert(execution_reservations::ActiveModel {
                    resource: Set(resource),
                    attempt_id: Set(Some(id)),
                    session_id: Set(None),
                })
                .on_conflict(
                    OnConflict::column(execution_reservations::Column::Resource)
                        .do_nothing()
                        .to_owned(),
                )
                .exec_without_returning(&tx)
                .await?;
            if inserted != 1 {
                return Ok(ClaimResponse {
                    lease: None,
                    poll_after_seconds: 5,
                });
            }
        }
        let claimed_at = attempt.claimed_at.or_else(|| Some(Utc::now()));
        let mut active = attempt.into_active_model();
        active.worker_id = Set(Some(worker.id));
        active.claim_id = Set(Some(input.claim_id));
        active.state = Set("leased".into());
        active.claimed_at = Set(claimed_at);
        active.update(&tx).await?;
        id
    };
    super::device_hosts::mark_leased(&tx, worker.id).await?;
    let token = token();
    let attempt = execution_attempts::Entity::find_by_id(id)
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let mut active = attempt.into_active_model();
    active.lease_hash = Set(Some(hash(&token)));
    active.expires_at = Set(Some(Utc::now() + Duration::seconds(60)));
    let attempt = active.update(&tx).await?;
    let run_id = attempt.run_id;
    let manifest = decode(
        execution_runs::Entity::find_by_id(run_id)
            .one(&tx)
            .await?
            .ok_or_else(ApiFailure::missing)?
            .manifest,
    )?;
    let result = ClaimResponse {
        lease: Some(ExecutionLease {
            attempt_id: id,
            run_id,
            generation: attempt.generation,
            lease_token: token,
            expires_at: attempt.expires_at.ok_or_else(ApiFailure::internal)?,
            manifest,
            case_index: attempt.case_index as u32,
        }),
        poll_after_seconds: 5,
    };
    tx.commit().await?;
    Ok(result)
}

pub async fn heartbeat(
    ctx: &AppContext,
    w: &Worker,
    id: Uuid,
    g: i32,
    token: &str,
) -> ApiResult<LeaseStatusResponse> {
    let tx = ctx.db.begin().await?;
    let lease = lease(&tx, w, id, g, token, true).await?;
    let cancel = lease.run.cancel_requested;
    let mut active = lease.attempt.into_active_model();
    if active.state.as_ref() == "leased" {
        active.state = Set("running".into());
    }
    active.expires_at = Set(Some(Utc::now() + Duration::seconds(60)));
    let attempt = active.update(&tx).await?;
    let out = LeaseStatusResponse {
        state: decode(serde_json::Value::String(attempt.state))?,
        expires_at: attempt.expires_at.ok_or_else(ApiFailure::internal)?,
        cancel_requested: cancel,
    };
    tx.commit().await?;
    Ok(out)
}

pub async fn events(
    ctx: &AppContext,
    w: &Worker,
    id: Uuid,
    token: &str,
    input: EventRequest,
) -> ApiResult<EventReceipt> {
    if input.events.is_empty() || input.events.len() > 50 {
        return Err(ApiFailure::invalid("Send 1–50 events"));
    }
    let tx = ctx.db.begin().await?;
    let lease = lease(&tx, w, id, input.generation, token, true).await?;
    let manifest: RunManifest = decode(lease.run.manifest.clone())?;
    super::execution_preflight::require(&tx, id, input.generation, &manifest).await?;
    let case = &manifest.cases[lease.attempt.case_index as usize].case;
    let mut last = execution_events::Entity::find()
        .filter(execution_events::Column::AttemptId.eq(id))
        .order_by_desc(execution_events::Column::Sequence)
        .one(&tx)
        .await?
        .map_or(0_i64, |event| i64::from(event.sequence));
    for event in &input.events {
        if event.sequence == 0
            || event.sequence > 10000
            || !bounded(&event.phase, 80)
            || event.message.len() > 1000
            || !case.actions.iter().any(|a| a.id == event.action_id)
        {
            return Err(ApiFailure::invalid("Invalid event"));
        }
        let payload = json(event)?;
        let old = execution_events::Entity::find()
            .filter(
                Condition::any()
                    .add(execution_events::Column::Id.eq(event.id))
                    .add(
                        Condition::all()
                            .add(execution_events::Column::AttemptId.eq(id))
                            .add(execution_events::Column::Sequence.eq(event.sequence as i32)),
                    ),
            )
            .all(&tx)
            .await?;
        if !old.is_empty() {
            if old.len() != 1 || old[0].attempt_id != id || old[0].payload != payload {
                return Err(conflict("Event identity conflicts"));
            }
            continue;
        }
        if !["leased", "running", "cancel_requested"].contains(&lease.attempt.state.as_str())
            || i64::from(event.sequence) != last + 1
        {
            return Err(conflict(
                "Event sequence or attempt state does not accept new events",
            ));
        }
        execution_events::ActiveModel {
            id: Set(event.id),
            attempt_id: Set(id),
            sequence: Set(event.sequence as i32),
            payload: Set(payload),
        }
        .insert(&tx)
        .await?;
        last += 1;
    }
    tx.commit().await?;
    Ok(EventReceipt {
        last_sequence: last as u32,
    })
}

pub async fn complete(
    ctx: &AppContext,
    w: &Worker,
    id: Uuid,
    token: &str,
    input: CompleteRequest,
) -> ApiResult<AttemptReceipt> {
    if !bounded(&input.reason, 200)
        || input.usage.len() > 20
        || input
            .usage
            .iter()
            .any(|u| !bounded(&u.model, 100) || u.calls > 10000 || u.unknown_calls > u.calls)
    {
        return Err(ApiFailure::invalid("Invalid completion metadata"));
    }
    let digest = hash(serde_json::to_vec(&input).map_err(|_| ApiFailure::internal())?);
    let row = lease(&ctx.db, w, id, input.generation, token, false).await?;
    let manifest: RunManifest = decode(row.run.manifest.clone())?;
    let case = &manifest.cases[row.attempt.case_index as usize].case;
    // Capture/evaluation performs bounded storage I/O without a database lock.
    let (checks, mut outcome) = super::verification::evaluate(ctx, id, case).await?;
    if outcome != Outcome::Failed && input.execution_outcome != Outcome::Passed {
        outcome = match input.execution_outcome {
            Outcome::Blocked => Outcome::Blocked,
            Outcome::Canceled => Outcome::Canceled,
            _ => Outcome::Inconclusive,
        };
    }
    let tx = ctx.db.begin().await?;
    // Match cancellation's lock order: run before attempt.
    execution_runs::Entity::find_by_id(row.attempt.run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let lease = lease(&tx, w, id, input.generation, token, true).await?;
    if let Some(old) = lease.attempt.completion_hash.clone() {
        if old != digest {
            return Err(conflict("Completion payload changed"));
        }
        let result = runs::attempt(&tx, id).await?;
        tx.commit().await?;
        return Ok(AttemptReceipt { attempt: result });
    }
    if manifest.profile.execution_context.is_some()
        && !super::execution_preflight::recorded(&tx, id, input.generation).await?
    {
        outcome = Outcome::Inconclusive;
    }
    if lease.run.cancel_requested && outcome != Outcome::Failed {
        outcome = Outcome::Canceled;
    }
    let mut active = lease.attempt.into_active_model();
    active.state = Set("finalizing".into());
    active.outcome = Set(Some(word(&outcome)));
    active.checks = Set(json(&checks)?);
    active.usage = Set(json(&input.usage)?);
    active.reason = Set(Some(input.reason));
    active.completion_hash = Set(Some(digest));
    active.update(&tx).await?;
    let result = runs::attempt(&tx, id).await?;
    tx.commit().await?;
    Ok(AttemptReceipt { attempt: result })
}

pub async fn cleanup(
    ctx: &AppContext,
    w: &Worker,
    id: Uuid,
    token: &str,
    input: CleanupRequest,
) -> ApiResult<AttemptReceipt> {
    if !bounded(&input.evidence_reference, 1000)
        || !bounded(&input.boot_id, 200)
        || input.reset == CleanupState::Pending
    {
        return Err(ApiFailure::invalid(
            "Cleanup evidence and final reset state are required",
        ));
    }
    let digest = hash(serde_json::to_vec(&input).map_err(|_| ApiFailure::internal())?);
    let tx = ctx.db.begin().await?;
    let preliminary = lease(&tx, w, id, input.generation, token, false).await?;
    let run = preliminary.attempt.run_id;
    execution_runs::Entity::find_by_id(run)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let lease = lease(&tx, w, id, input.generation, token, true).await?;
    if let Some(old) = lease.attempt.cleanup_hash.clone() {
        if old != digest {
            return Err(conflict("Cleanup payload changed"));
        }
        let a = runs::attempt(&tx, id).await?;
        tx.commit().await?;
        return Ok(AttemptReceipt { attempt: a });
    }
    if lease.attempt.completion_hash.is_none() {
        return Err(conflict(
            "Persist attempt evidence before acknowledging cleanup",
        ));
    }
    let manifest: RunManifest = decode(lease.run.manifest.clone())?;
    let clean = input.stopped
        && input.reset == CleanupState::VerifiedClean
        && (manifest.profile.execution_context.is_none()
            || super::execution_preflight::recorded(&tx, id, input.generation).await?);
    let number = lease.attempt.number;
    let case_index = lease.attempt.case_index;
    let outcome = lease.attempt.outcome.clone();
    let mut active = lease.attempt.into_active_model();
    active.state = Set(if clean {
        "finished".into()
    } else {
        "recovery_required".into()
    });
    active.cleanup = Set(if clean {
        "verified_clean".into()
    } else {
        "quarantined".into()
    });
    active.cleanup_hash = Set(Some(digest));
    active.cleanup_receipt = Set(Some(json(&input)?));
    if clean {
        active.released_at = Set(Some(Utc::now()));
    }
    active.update(&tx).await?;
    if clean {
        execution_reservations::Entity::delete_many()
            .filter(execution_reservations::Column::AttemptId.eq(id))
            .exec(&tx)
            .await?;
        if manifest.diagnostic_retries == 1
            && number == 1
            && !lease.run.cancel_requested
            && matches!(outcome.as_deref(), Some("failed" | "inconclusive"))
        {
            execution_attempts::ActiveModel {
                id: Set(Uuid::new_v4()),
                run_id: Set(run),
                case_index: Set(case_index),
                number: Set(2),
                ..Default::default()
            }
            .insert(&tx)
            .await?;
        }
    }
    let result = runs::attempt(&tx, id).await?;
    tx.commit().await?;
    // Cleanup acknowledgement stays successful even if projection persistence is
    // temporarily unavailable; reconciliation retries from durable attempt facts.
    if let Err(error) = super::run_comparisons::finalize_pending(ctx).await {
        tracing::warn!(
            ?error,
            "Comparison finalization will retry during reconciliation"
        );
    }
    super::execution_wakeup::notify(ctx);
    Ok(AttemptReceipt { attempt: result })
}
/// Trusted operator reconciles physical recovery; never changes the original outcome.
pub async fn recover(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    id: Uuid,
    evidence: &str,
) -> ApiResult<()> {
    test_definitions::operator(ctx, actor, app).await?;
    if !bounded(evidence, 1000) {
        return Err(ApiFailure::invalid("Recovery evidence reference required"));
    }
    let tx = ctx.db.begin().await?;
    let attempt = execution_attempts::Entity::find_by_id(id)
        .filter(execution_attempts::Column::State.eq("recovery_required"))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    execution_runs::Entity::find_by_id(attempt.run_id)
        .filter(execution_runs::Column::AppId.eq(app))
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    execution_recovery_events::ActiveModel {
        id: Set(Uuid::new_v4()),
        attempt_id: Set(id),
        actor_id: Set(actor),
        evidence_reference: Set(evidence.to_owned()),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    let mut active = attempt.into_active_model();
    active.state = Set("finished".into());
    active.cleanup = Set("verified_clean".into());
    active.released_at = Set(Some(Utc::now()));
    active.update(&tx).await?;
    execution_reservations::Entity::delete_many()
        .filter(execution_reservations::Column::AttemptId.eq(id))
        .exec(&tx)
        .await?;
    tx.commit().await?;
    // Cleanup acknowledgement stays successful even if projection persistence is
    // temporarily unavailable; reconciliation retries from durable attempt facts.
    if let Err(error) = super::run_comparisons::finalize_pending(ctx).await {
        tracing::warn!(
            ?error,
            "Comparison finalization will retry during reconciliation"
        );
    }
    super::execution_wakeup::notify(ctx);
    Ok(())
}
