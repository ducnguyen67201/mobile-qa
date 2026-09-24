//! Freeze saved coverage before queuing. Reports are projections of durable facts.
use super::{apps, execution_store::*, model_registry, test_definitions as definitions};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        apps as app_rows, builds, environments, execution_artifacts, execution_attempts,
        execution_events, execution_preflight_receipts, execution_recovery_events,
        execution_reservations, execution_runs, execution_workers, phone_sessions,
    },
};
use loco_rs::app::AppContext;
use mobile_qa_contracts::execution::*;
use mobile_qa_contracts::model_registry::{ModelCapability, ModelPurpose};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use uuid::Uuid;

pub async fn preview(
    db: &impl ConnectionTrait,
    app: Uuid,
    build: Uuid,
    plan: Option<Uuid>,
) -> ApiResult<PlanPreviewResponse> {
    let mut out = PlanPreviewResponse {
        plan: None,
        manifest: None,
        blockers: vec![],
    };
    let plan = if let Some(id) = plan {
        id
    } else {
        if let Some(id) = super::test_library::default_plan(db, app)
            .await?
            .plan_version_id
        {
            id
        } else {
            out.blockers
                .push("Choose a saved default release plan in Tests".into());
            return Ok(out);
        }
    };
    let d = definitions::get(db, app, plan).await?;
    out.plan = Some(d.clone());
    if let Err(error) = super::test_library::admitted(db, app, plan).await {
        if error.status.is_server_error() {
            return Err(error);
        }
        out.blockers.push(error.message);
        return Ok(out);
    }
    let TestDefinition::Plan(p) = &d.definition else {
        return Err(ApiFailure::invalid("Expected a plan version"));
    };
    let cases = match definitions::resolve(db, app, p).await {
        Ok(cases) => cases,
        Err(error) if !error.status.is_server_error() => {
            out.blockers.push(error.message);
            return Ok(out);
        }
        Err(error) => return Err(error),
    };
    assemble(db, app, build, d.clone(), p.clone(), cases, None).await
}

pub(crate) async fn assemble(
    db: &impl ConnectionTrait,
    app: Uuid,
    build: Uuid,
    d: DefinitionResponse,
    p: PlanDefinition,
    cases: Vec<ResolvedCase>,
    source: Option<mobile_qa_contracts::regression::RunSource>,
) -> ApiResult<PlanPreviewResponse> {
    let mut out = PlanPreviewResponse {
        plan: if source.is_none() {
            Some(d.clone())
        } else {
            None
        },
        manifest: None,
        blockers: vec![],
    };
    let b = builds::Entity::find_by_id(build)
        .filter(builds::Column::AppId.eq(app))
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let profile = definitions::profile(db, app, p.profile_id).await?;
    let uses_navigation = cases.iter().any(|c| {
        c.case
            .actions
            .iter()
            .any(|a| a.kind == ActionKind::Navigate)
    });
    let assignment = if uses_navigation && profile.driver == Driver::Minitap {
        model_registry::active_assignment(db, app, profile.id, ModelPurpose::Navigation).await?
    } else {
        None
    };
    let resolved_model = if uses_navigation {
        match if let Some((model, _)) = &assignment {
            Ok(Some(model.clone()))
        } else {
            model_registry::resolve_for_new_work(
                db,
                profile.model.as_ref(),
                &[ModelCapability::MinitapNavigation],
            )
            .await
        } {
            Ok(model) => model,
            Err(error) if !error.status.is_server_error() => {
                out.blockers.push(error.message);
                None
            }
            Err(error) => return Err(error),
        }
    } else {
        None
    };
    if uses_navigation && profile.driver == Driver::Minitap && resolved_model.is_none() {
        out.blockers
            .push("An operator must activate a navigation model assignment".into());
    }
    for c in &cases {
        if let Err(e) = super::execution_readiness::case_matches(&profile, &c.case) {
            out.blockers.push(e.message);
        }
    }
    let env = environments::Entity::find()
        .filter(environments::Column::AppId.eq(app))
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let size = b.byte_size;
    if b.validation_state != "validated" {
        out.blockers.push("APK intake validation must pass".into());
    }
    if !profile.qualified {
        out.blockers
            .push("Device profile has not been qualified".into());
    }
    if size < 1 || size > i64::from(profile.max_apk_bytes) {
        out.blockers.push("APK exceeds worker capability".into());
    }
    let app_package = app_rows::Entity::find_by_id(app)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?
        .android_package;
    if app_package != profile.package
        || cases.iter().any(|c| {
            c.case.package != profile.package
                || c.case.adapter != profile.adapter
                || c.case
                    .checks
                    .iter()
                    .any(|check| check.method == CheckMethod::Manual)
        })
    {
        out.blockers
            .push("App or checks are incompatible with this adapter".into());
    }
    // This adapter owns its synthetic backend and has no customer account or remote reset.
    if env.account_secret_reference_id.is_some() || env.reset_secret_reference_id.is_some() {
        out.blockers
            .push("Customer credential/reset references need a qualified adapter".into());
    }
    let required_duration: u64 = cases
        .iter()
        .map(|c| {
            super::execution_readiness::duration(&profile, &c.case)
                * (u64::from(p.diagnostic_retries) + 1)
        })
        .sum();
    if required_duration > u64::from(p.budget.duration_seconds) {
        out.blockers
            .push("Plan duration does not cover its case budgets and permitted retries".into());
    }
    if cases.iter().any(|c| {
        c.case.budget.artifact_bytes > p.budget.artifact_bytes
            || c.case.budget.max_steps > p.budget.max_steps
    }) {
        out.blockers.push("Case budget exceeds plan policy".into());
    }
    out.manifest = Some(RunManifest {
        app_id: app,
        build_id: build,
        build_sha256: b.sha256,
        build_bytes: u32::try_from(size).map_err(|_| ApiFailure::internal())?,
        source: source.clone(),
        plan_version_id: if source.is_some() { None } else { Some(d.id) },
        plan_hash: if source.is_some() {
            None
        } else {
            Some(d.content_hash)
        },
        environment_revision: env.revision,
        profile,
        resolved_model,
        model_assignment_revision: assignment.map(|(_, revision)| revision),
        cases,
        budget: p.budget.clone(),
        diagnostic_retries: p.diagnostic_retries,
        exclusions: p.exclusions.clone(),
    });
    Ok(out)
}

pub async fn create(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    key: &str,
    input: CreateRunRequest,
) -> ApiResult<(RunResponse, bool)> {
    create_with_quote(ctx, actor, app, key, input, None).await
}

pub async fn create_with_quote(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    key: &str,
    input: CreateRunRequest,
    quote_id: Option<Uuid>,
) -> ApiResult<(RunResponse, bool)> {
    apps::authorized(ctx, actor, app).await?;
    if !bounded(key, 128) {
        return Err(ApiFailure::invalid(
            "Idempotency-Key must contain 1–128 printable bytes",
        ));
    }
    let fingerprint = if let Some(id) = quote_id {
        hash(serde_json::to_vec(&(actor, &input, id)).map_err(|_| ApiFailure::internal())?)
    } else {
        hash(serde_json::to_vec(&(actor, &input)).map_err(|_| ApiFailure::internal())?)
    };
    let tx = ctx.db.begin().await?;
    app_rows::Entity::find_by_id(app)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let old = execution_runs::Entity::find()
        .filter(execution_runs::Column::AppId.eq(app))
        .filter(execution_runs::Column::IdempotencyKey.eq(key))
        .one(&tx)
        .await?;
    if let Some(r) = old {
        if r.fingerprint != fingerprint {
            return Err(conflict("Idempotency key belongs to another submission"));
        }
        let result = detail(&tx, r.id).await?;
        tx.commit().await?;
        return Ok((result, false));
    }
    environments::Entity::find()
        .filter(environments::Column::AppId.eq(app))
        .lock_shared()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let preview = preview(&tx, app, input.build_id, Some(input.plan_version_id)).await?;
    if !preview.blockers.is_empty() {
        return Err(ApiFailure::invalid(preview.blockers.join("; ")));
    }
    let manifest = preview.manifest.ok_or_else(ApiFailure::internal)?;
    if input.environment_revision != manifest.environment_revision {
        return Err(conflict("Environment changed; refresh the preview"));
    }
    let id = Uuid::new_v4();
    execution_runs::ActiveModel {
        id: Set(id),
        app_id: Set(app),
        creator_id: Set(actor),
        build_id: Set(input.build_id),
        plan_id: Set(Some(input.plan_version_id)),
        idempotency_key: Set(key.to_owned()),
        fingerprint: Set(fingerprint),
        manifest: Set(json(&manifest)?),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    super::commercial::reserve_or_test_fixture(
        ctx,
        &tx,
        actor,
        app,
        quote_id,
        "release_plan",
        &super::commercial::plan_hash(actor, &input)?,
        &manifest,
        id,
    )
    .await?;
    for (index, _) in manifest.cases.iter().enumerate() {
        execution_attempts::ActiveModel {
            id: Set(Uuid::new_v4()),
            run_id: Set(id),
            case_index: Set(index as i32),
            ..Default::default()
        }
        .insert(&tx)
        .await?;
    }
    super::capacity_control::enqueue(&tx, app, manifest.profile.id).await?;
    let result = detail(&tx, id).await?;
    tx.commit().await?;
    super::execution_wakeup::notify(ctx);
    let reason_code = result
        .queue_status
        .as_ref()
        .map(|status| word(&status.reason))
        .unwrap_or_else(|| "unknown".into());
    tracing::info!(run_id=%id,app_id=%app,phase="queued",reason_code=%reason_code,"Approved execution queued");
    Ok((result, true))
}

pub async fn authorize(ctx: &AppContext, user: Uuid, id: Uuid) -> ApiResult<Uuid> {
    let app = execution_runs::Entity::find_by_id(id)
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?
        .app_id;
    apps::authorized(ctx, user, app).await?;
    Ok(app)
}

pub async fn attempt(db: &impl ConnectionTrait, id: Uuid) -> ApiResult<AttemptResponse> {
    let r = execution_attempts::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let manifest: RunManifest = decode(
        execution_runs::Entity::find_by_id(r.run_id)
            .one(db)
            .await?
            .ok_or_else(ApiFailure::missing)?
            .manifest,
    )?;
    let index = r.case_index;
    let artifacts = execution_artifacts::Entity::find()
        .filter(execution_artifacts::Column::AttemptId.eq(id))
        .order_by_asc(execution_artifacts::Column::Name)
        .all(db)
        .await?
        .iter()
        .map(super::run_artifacts::record)
        .collect::<ApiResult<Vec<_>>>()?;
    let preflight = execution_preflight_receipts::Entity::find()
        .filter(execution_preflight_receipts::Column::AttemptId.eq(id))
        .order_by_desc(execution_preflight_receipts::Column::Generation)
        .one(db)
        .await?
        .map(|row| decode(row.payload))
        .transpose()?;
    let recovery_events = execution_recovery_events::Entity::find()
        .filter(execution_recovery_events::Column::AttemptId.eq(id))
        .order_by_asc(execution_recovery_events::Column::CreatedAt)
        .order_by_asc(execution_recovery_events::Column::Id)
        .all(db)
        .await?
        .into_iter()
        .map(
            |row| mobile_qa_contracts::execution_lifecycle::RecoveryEvent {
                actor_id: row.actor_id,
                evidence_reference: row.evidence_reference,
                created_at: row.created_at,
            },
        )
        .collect();
    let events = execution_events::Entity::find()
        .filter(execution_events::Column::AttemptId.eq(id))
        .order_by_asc(execution_events::Column::Sequence)
        .all(db)
        .await?
        .into_iter()
        .map(|row| decode(row.payload))
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(AttemptResponse {
        preflight,
        recovery_events,
        original_cleanup: r
            .cleanup_receipt
            .and_then(|v| serde_json::from_value(v).ok()),
        id,
        case_version_id: manifest.cases[index as usize].definition_id,
        generation: r.generation,
        number: r.number as u32,
        state: decode(serde_json::Value::String(r.state))?,
        outcome: r
            .outcome
            .map(|s| decode(serde_json::Value::String(s)))
            .transpose()?,
        cleanup: decode(serde_json::Value::String(r.cleanup))?,
        reason: r.reason,
        checks: decode(r.checks)?,
        artifacts,
        usage: decode(r.usage)?,
        events,
    })
}

pub async fn detail(db: &impl ConnectionTrait, id: Uuid) -> ApiResult<RunResponse> {
    let row = execution_runs::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let build = builds::Entity::find_by_id(row.build_id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let build_label = build
        .metadata
        .as_ref()
        .and_then(|value| value.get("version_name"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .unwrap_or(build.original_filename);
    let manifest: RunManifest = decode(row.manifest.clone())?;
    let mut attempts = vec![];
    let attempt_rows = execution_attempts::Entity::find()
        .filter(execution_attempts::Column::RunId.eq(id))
        .order_by_asc(execution_attempts::Column::CaseIndex)
        .order_by_asc(execution_attempts::Column::Number)
        .all(db)
        .await?;
    for attempt_row in attempt_rows {
        attempts.push(attempt(db, attempt_row.id).await?);
    }
    let state = if attempts
        .iter()
        .any(|a| a.state == JobState::RecoveryRequired)
    {
        JobState::RecoveryRequired
    } else if attempts.iter().all(|a| a.state == JobState::Finished) {
        JobState::Finished
    } else if row.cancel_requested {
        JobState::CancelRequested
    } else if attempts.iter().any(|a| a.state == JobState::Finalizing) {
        JobState::Finalizing
    } else if attempts
        .iter()
        .any(|a| matches!(a.state, JobState::Running | JobState::Leased))
    {
        JobState::Running
    } else {
        JobState::Queued
    };
    let summary = summary(&manifest, &attempts);
    let created_at = row.created_at;
    let queue_status = if state == JobState::Queued {
        Some(queue_status(db, &manifest, created_at).await?)
    } else {
        None
    };
    Ok(RunResponse {
        cancel_requested: row.cancel_requested,
        build_label: Some(build_label),
        queue_status,
        comparison: row.comparison.map(decode).transpose()?,
        baseline_run_id: row.baseline_run_id,
        id,
        manifest,
        state,
        summary,
        created_at,
        attempts,
    })
}

async fn queue_status(
    db: &impl ConnectionTrait,
    manifest: &RunManifest,
    created_at: chrono::DateTime<chrono::Utc>,
) -> ApiResult<QueueStatus> {
    let workers = execution_workers::Entity::find()
        .filter(execution_workers::Column::AppId.eq(manifest.app_id))
        .filter(execution_workers::Column::ProfileId.eq(manifest.profile.id))
        .filter(execution_workers::Column::Revoked.eq(false))
        .all(db)
        .await?;
    let mut live = false;
    let mut model_live = false;
    let mut compatible_live = false;
    let mut last_compatible = None;
    let threshold = chrono::Utc::now() - chrono::Duration::seconds(90);
    for row in workers {
        let seen = row.execution_last_seen_at;
        let version = row.execution_protocol_version;
        let capabilities = row.execution_model_capabilities.and_then(|value| {
            decode::<mobile_qa_contracts::model_registry::WorkerModelCapabilities>(value).ok()
        });
        let model_matches = manifest
            .resolved_model
            .as_ref()
            .is_none_or(|model| model_registry::worker_matches(model, capabilities.as_ref()));
        let protocol_matches = version.is_some_and(|version| {
            version >= 1
                && (manifest.profile.execution_context.is_none() || version >= 3)
                && (manifest.source.is_none() || version >= 4)
                && (version >= 2
                    || !manifest.cases.iter().any(|case| {
                        case.case
                            .actions
                            .iter()
                            .any(|action| action.kind == ActionKind::Direct)
                    }))
                && (manifest.resolved_model.is_none() || version >= 5)
        });
        let matches = model_matches && protocol_matches;
        if let Some(seen) = seen {
            if seen > threshold {
                live = true;
                if model_matches {
                    model_live = true;
                }
                if matches {
                    compatible_live = true;
                }
            }
            if matches && last_compatible.is_none_or(|last| seen > last) {
                last_compatible = Some(seen);
            }
        }
    }
    let reservations = execution_reservations::Entity::find()
        .filter(execution_reservations::Column::Resource.is_in([
            format!("app:{}", manifest.app_id),
            format!("device:{}", manifest.profile.device_identity),
        ]))
        .all(db)
        .await?;
    let mut recovery_required = false;
    let mut blocking_run_id = None;
    for row in &reservations {
        if let Some(attempt_id) = row.attempt_id {
            if let Some(attempt) = execution_attempts::Entity::find_by_id(attempt_id)
                .one(db)
                .await?
            {
                if attempt.state == "recovery_required" {
                    recovery_required = true;
                    if let Some(run) = execution_runs::Entity::find_by_id(attempt.run_id)
                        .one(db)
                        .await?
                    {
                        if run.app_id == manifest.app_id {
                            blocking_run_id = Some(run.id);
                        }
                    }
                }
            }
        } else if let Some(session_id) = row.session_id {
            if let Some(session) = phone_sessions::Entity::find_by_id(session_id)
                .one(db)
                .await?
            {
                let session: mobile_qa_contracts::task_sessions::PhoneSession =
                    decode(session.payload)?;
                if session.state == mobile_qa_contracts::task_sessions::PhoneState::Quarantined {
                    recovery_required = true;
                }
            }
        }
    }
    let capacity_reason =
        super::capacity_control::queue_reason(db, manifest.app_id, manifest.profile.id).await?;
    let reason = if recovery_required {
        QueueReason::DeviceRecoveryRequired
    } else if !reservations.is_empty() {
        QueueReason::CapacityBusy
    } else if let Some(reason) = capacity_reason {
        reason
    } else if !live {
        QueueReason::WorkerOffline
    } else if !model_live {
        QueueReason::ModelUnavailable
    } else if !compatible_live {
        QueueReason::WorkerUpgradeRequired
    } else {
        QueueReason::AwaitingWorkerClaim
    };
    Ok(QueueStatus {
        reason,
        blocking_run_id,
        last_compatible_worker_at: last_compatible,
        wait_seconds: (chrono::Utc::now() - created_at)
            .num_seconds()
            .clamp(0, u32::MAX as i64) as u32,
    })
}

fn verified_required_pass(
    manifest: &RunManifest,
    case: &ResolvedCase,
    attempt: &AttemptResponse,
) -> bool {
    if attempt.outcome != Some(Outcome::Passed)
        || attempt.state != JobState::Finished
        || attempt.cleanup != CleanupState::VerifiedClean
        || !attempt.recovery_events.is_empty()
        || attempt
            .original_cleanup
            .as_ref()
            .is_some_and(|receipt| !receipt.stopped || receipt.reset != CleanupState::VerifiedClean)
    {
        return false;
    }
    let Some(context) = manifest.profile.execution_context.as_ref() else {
        return true;
    };
    if !attempt.preflight.as_ref().is_some_and(|proof| {
        &proof.context == context
            && proof.attempt_id == attempt.id
            && proof.build_sha256 == manifest.build_sha256
    }) || !attempt
        .original_cleanup
        .as_ref()
        .is_some_and(|receipt| receipt.stopped && receipt.reset == CleanupState::VerifiedClean)
    {
        return false;
    }
    case.case
        .checks
        .iter()
        .filter(|check| check.required)
        .all(|check| {
            attempt.checks.iter().any(|result| {
                result.check_id == check.id
                    && result.outcome == Outcome::Passed
                    && (result.observed.is_some()
                        || result.observation_kind
                            == Some(mobile_qa_contracts::regression::ObservationKind::Absent))
                    && !result.artifact_ids.is_empty()
                    && result.artifact_ids.iter().all(|id| {
                        attempt.artifacts.iter().any(|artifact| {
                            artifact.id == *id && artifact.state == EvidenceState::Sealed
                        })
                    })
            })
        })
}

pub fn summary(manifest: &RunManifest, attempts: &[AttemptResponse]) -> String {
    let required: Vec<_> = manifest.cases.iter().filter(|c| c.required).collect();
    if required.iter().any(|c| {
        attempts
            .iter()
            .any(|a| a.case_version_id == c.definition_id && a.outcome == Some(Outcome::Failed))
    }) {
        return "Failures detected; review all attempts".into();
    }
    if required.is_empty()
        || required.iter().any(|c| {
            let matching: Vec<_> = attempts
                .iter()
                .filter(|a| a.case_version_id == c.definition_id)
                .collect();
            matching.is_empty()
                || matching
                    .iter()
                    .any(|attempt| !verified_required_pass(manifest, c, attempt))
        })
    {
        return "Incomplete / review required".into();
    }
    if manifest.cases.iter().filter(|c| !c.required).any(|c| {
        attempts
            .iter()
            .any(|a| a.case_version_id == c.definition_id && a.outcome == Some(Outcome::Failed))
    }) {
        return "Required checks passed; optional failures detected".into();
    }
    if manifest.cases.iter().filter(|c| !c.required).any(|c| {
        !attempts
            .iter()
            .any(|a| a.case_version_id == c.definition_id)
            || attempts.iter().any(|a| {
                a.case_version_id == c.definition_id
                    && (a.state != JobState::Finished || a.outcome != Some(Outcome::Passed))
            })
    }) {
        return "Incomplete / review required; optional cases unresolved".into();
    }
    "Required checks passed".into()
}

pub async fn cancel(ctx: &AppContext, user: Uuid, id: Uuid) -> ApiResult<RunResponse> {
    authorize(ctx, user, id).await?;
    let tx = ctx.db.begin().await?;
    let run = execution_runs::Entity::find_by_id(id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let mut active = run.into_active_model();
    active.cancel_requested = Set(true);
    active.update(&tx).await?;
    let queued = execution_attempts::Entity::find()
        .filter(execution_attempts::Column::RunId.eq(id))
        .filter(execution_attempts::Column::State.eq("queued"))
        .all(&tx)
        .await?;
    for attempt in queued {
        let mut active = attempt.into_active_model();
        active.state = Set("finished".into());
        active.outcome = Set(Some("canceled".into()));
        active.cleanup = Set("verified_clean".into());
        active.reason = Set(Some("canceled_before_dispatch".into()));
        active.update(&tx).await?;
    }
    let active_attempts = execution_attempts::Entity::find()
        .filter(execution_attempts::Column::RunId.eq(id))
        .filter(execution_attempts::Column::State.is_in(["leased", "running"]))
        .all(&tx)
        .await?;
    for attempt in active_attempts {
        let mut active = attempt.into_active_model();
        active.state = Set("cancel_requested".into());
        active.update(&tx).await?;
    }
    // A run canceled before any worker claim consumed no delivered device check.
    super::commercial::credit_queued_cancel(&tx, user, id).await?;
    let result = detail(&tx, id).await?;
    tx.commit().await?;
    Ok(result)
}

pub async fn list(
    ctx: &AppContext,
    user: Uuid,
    app: Uuid,
    cursor: Option<Uuid>,
) -> ApiResult<RunListResponse> {
    apps::authorized(ctx, user, app).await?;
    let mut query = execution_runs::Entity::find()
        .filter(execution_runs::Column::AppId.eq(app))
        .order_by_desc(execution_runs::Column::Id)
        .limit(21);
    if let Some(cursor) = cursor {
        query = query.filter(execution_runs::Column::Id.lt(cursor));
    }
    let ids = query.all(&ctx.db).await?;
    let mut items = vec![];
    for row in ids.iter().take(20) {
        items.push(detail(&ctx.db, row.id).await?);
    }
    let next_cursor = if ids.len() > 20 {
        items.last().map(|r| r.id.to_string())
    } else {
        None
    };
    Ok(RunListResponse { items, next_cursor })
}

#[cfg(test)]
mod summary_tests {
    use super::*;
    #[test]
    fn required_proof_and_optional_failures_are_reported_separately() {
        let mut run: RunResponse = serde_json::from_str(include_str!(
            "../../tests/fixtures/execution/comparison.json"
        ))
        .unwrap();
        assert_eq!(
            summary(&run.manifest, &run.attempts),
            "Required checks passed"
        );
        let proof = run.attempts[0].preflight.clone();
        run.attempts[0].preflight = None;
        assert_eq!(
            summary(&run.manifest, &run.attempts),
            "Incomplete / review required"
        );
        run.attempts[0].preflight = proof;
        run.attempts[0].artifacts[0].state = EvidenceState::Unavailable;
        assert_eq!(
            summary(&run.manifest, &run.attempts),
            "Incomplete / review required"
        );
        run.attempts[0].artifacts[0].state = EvidenceState::Sealed;
        let mut optional = run.manifest.cases[0].clone();
        optional.definition_id = Uuid::new_v4();
        optional.case.key = "optional".into();
        optional.required = false;
        let mut attempt = run.attempts[0].clone();
        attempt.id = Uuid::new_v4();
        attempt.case_version_id = optional.definition_id;
        attempt.preflight.as_mut().unwrap().attempt_id = attempt.id;
        attempt.outcome = Some(Outcome::Failed);
        run.manifest.cases.push(optional);
        run.attempts.push(attempt);
        assert_eq!(
            summary(&run.manifest, &run.attempts),
            "Required checks passed; optional failures detected"
        );
    }
}
