//! Freeze saved coverage before queuing. Reports are projections of durable facts.
use super::{apps, execution_store::*, model_registry, test_definitions as definitions};
use crate::errors::{ApiFailure, ApiResult};
use loco_rs::app::AppContext;
use mobile_qa_contracts::execution::*;
use mobile_qa_contracts::model_registry::{ModelCapability, ModelPurpose};
use sea_orm::{ConnectionTrait, TransactionTrait};
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
    let b = one(
        db,
        "SELECT * FROM builds WHERE id=$1 AND app_id=$2",
        vec![build.into(), app.into()],
    )
    .await?;
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
    let env = one(
        db,
        "SELECT revision,account_secret_reference_id,reset_secret_reference_id FROM \
            environments WHERE app_id=$1",
        vec![app.into()],
    )
    .await?;
    let size: i64 = field(&b, "byte_size")?;
    if field::<String>(&b, "validation_state")? != "validated" {
        out.blockers.push("APK intake validation must pass".into());
    }
    if !profile.qualified {
        out.blockers
            .push("Device profile has not been qualified".into());
    }
    if size < 1 || size > i64::from(profile.max_apk_bytes) {
        out.blockers.push("APK exceeds worker capability".into());
    }
    let app_package: String = field(
        &one(
            db,
            "SELECT android_package FROM apps WHERE id=$1",
            vec![app.into()],
        )
        .await?,
        "android_package",
    )?;
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
    if field::<Option<Uuid>>(&env, "account_secret_reference_id")?.is_some()
        || field::<Option<Uuid>>(&env, "reset_secret_reference_id")?.is_some()
    {
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
        build_sha256: field(&b, "sha256")?,
        build_bytes: u32::try_from(size).unwrap_or(0),
        source: source.clone(),
        plan_version_id: if source.is_some() { None } else { Some(d.id) },
        plan_hash: if source.is_some() {
            None
        } else {
            Some(d.content_hash)
        },
        environment_revision: field(&env, "revision")?,
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
    apps::authorized(ctx, actor, app).await?;
    if !bounded(key, 128) {
        return Err(ApiFailure::invalid(
            "Idempotency-Key must contain 1–128 printable bytes",
        ));
    }
    let fingerprint =
        hash(serde_json::to_vec(&(actor, &input)).map_err(|_| ApiFailure::internal())?);
    let tx = ctx.db.begin().await?;
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![app.into()],
    )
    .await?;
    let old = rows(
        &tx,
        "SELECT id,fingerprint FROM execution_runs WHERE app_id=$1 AND idempotency_key=$2",
        vec![app.into(), key.into()],
    )
    .await?;
    if let Some(r) = old.first() {
        if field::<String>(r, "fingerprint")? != fingerprint {
            return Err(conflict("Idempotency key belongs to another submission"));
        }
        let result = detail(&tx, field(r, "id")?).await?;
        tx.commit().await?;
        return Ok((result, false));
    }
    one(
        &tx,
        "SELECT id FROM environments WHERE app_id=$1 FOR SHARE",
        vec![app.into()],
    )
    .await?;
    let preview = preview(&tx, app, input.build_id, Some(input.plan_version_id)).await?;
    if !preview.blockers.is_empty() {
        return Err(ApiFailure::invalid(preview.blockers.join("; ")));
    }
    let manifest = preview.manifest.ok_or_else(ApiFailure::internal)?;
    if input.environment_revision != manifest.environment_revision {
        return Err(conflict("Environment changed; refresh the preview"));
    }
    let id = Uuid::new_v4();
    exec(
        &tx,
        "INSERT INTO execution_runs(id,app_id,creator_id,build_id,plan_id,idempotency_key,\
            fingerprint,manifest) VALUES($1,$2,$3,$4,$5,$6,$7,$8)",
        vec![
            id.into(),
            app.into(),
            actor.into(),
            input.build_id.into(),
            input.plan_version_id.into(),
            key.into(),
            fingerprint.into(),
            json(&manifest)?.into(),
        ],
    )
    .await?;
    for (index, _) in manifest.cases.iter().enumerate() {
        exec(
            &tx,
            "INSERT INTO execution_attempts(id,run_id,case_index) VALUES($1,$2,$3)",
            vec![Uuid::new_v4().into(), id.into(), (index as i32).into()],
        )
        .await?;
    }
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
    let app: Uuid = field(
        &one(
            &ctx.db,
            "SELECT app_id FROM execution_runs WHERE id=$1",
            vec![id.into()],
        )
        .await?,
        "app_id",
    )?;
    apps::authorized(ctx, user, app).await?;
    Ok(app)
}

pub async fn attempt(db: &impl ConnectionTrait, id: Uuid) -> ApiResult<AttemptResponse> {
    let r = one(
        db,
        "SELECT a.*,r.manifest FROM execution_attempts a JOIN execution_runs r ON \
            r.id=a.run_id WHERE a.id=$1",
        vec![id.into()],
    )
    .await?;
    let manifest: RunManifest = decode(field(&r, "manifest")?)?;
    let index: i32 = field(&r, "case_index")?;
    let artifacts = rows(
        db,
        "SELECT * FROM execution_artifacts WHERE attempt_id=$1 ORDER BY name",
        vec![id.into()],
    )
    .await?
    .iter()
    .map(super::run_artifacts::record)
    .collect::<ApiResult<Vec<_>>>()?;
    Ok(AttemptResponse {
        preflight: rows(db,"SELECT payload FROM execution_preflight_receipts WHERE attempt_id=$1 ORDER BY generation DESC LIMIT 1",vec![id.into()]).await?.first().map(|r| decode(field(r,"payload")?)).transpose()?,
        recovery_events: rows(db,"SELECT actor_id,evidence_reference,created_at FROM execution_recovery_events WHERE attempt_id=$1 ORDER BY created_at,id",vec![id.into()]).await?.iter().map(|r| Ok(mobile_qa_contracts::execution_lifecycle::RecoveryEvent {actor_id:field(r,"actor_id")?,evidence_reference:field(r,"evidence_reference")?,created_at:field(r,"created_at")?})).collect::<ApiResult<Vec<_>>>()?,
        original_cleanup: field::<Option<serde_json::Value>>(&r,"cleanup_receipt")?.and_then(|v| serde_json::from_value(v).ok()),
        id,
        case_version_id: manifest.cases[index as usize].definition_id,
        generation: field(&r, "generation")?,
        number: field::<i32>(&r, "number")? as u32,
        state: decode(serde_json::Value::String(field(&r, "state")?))?,
        outcome: field::<Option<String>>(&r, "outcome")?
            .map(|s| decode(serde_json::Value::String(s)))
            .transpose()?,
        cleanup: decode(serde_json::Value::String(field(&r, "cleanup")?))?,
        reason: field(&r, "reason")?,
        checks: decode(field(&r, "checks")?)?,
        artifacts,
        usage: decode(field(&r, "usage")?)?,
        events: rows(
            db,
            "SELECT payload FROM execution_events WHERE attempt_id=$1 ORDER BY sequence",
            vec![id.into()],
        )
        .await?
        .iter()
        .map(|r| decode(field(r, "payload")?))
        .collect::<ApiResult<Vec<_>>>()?,
    })
}

pub async fn detail(db: &impl ConnectionTrait, id: Uuid) -> ApiResult<RunResponse> {
    let row = one(
        db,
        "SELECT r.*,COALESCE(b.metadata->>'version_name',b.original_filename) AS build_label FROM execution_runs r JOIN builds b ON b.id=r.build_id WHERE r.id=$1",
        vec![id.into()],
    )
    .await?;
    let manifest: RunManifest = decode(field(&row, "manifest")?)?;
    let mut attempts = vec![];
    for r in rows(
        db,
        "SELECT id FROM execution_attempts WHERE run_id=$1 ORDER BY case_index,number",
        vec![id.into()],
    )
    .await?
    {
        attempts.push(attempt(db, field(&r, "id")?).await?);
    }
    let state = if attempts
        .iter()
        .any(|a| a.state == JobState::RecoveryRequired)
    {
        JobState::RecoveryRequired
    } else if attempts.iter().all(|a| a.state == JobState::Finished) {
        JobState::Finished
    } else if field::<bool>(&row, "cancel_requested")? {
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
    let created_at: chrono::DateTime<chrono::Utc> = field(&row, "created_at")?;
    let queue_status = if state == JobState::Queued {
        Some(queue_status(db, &manifest, created_at).await?)
    } else {
        None
    };
    Ok(RunResponse {
        cancel_requested: field(&row, "cancel_requested")?,
        build_label: Some(field(&row, "build_label")?),
        queue_status,
        comparison: field::<Option<serde_json::Value>>(&row, "comparison")?
            .map(decode)
            .transpose()?,
        baseline_run_id: field(&row, "baseline_run_id")?,
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
    let workers=rows(db,"SELECT execution_model_capabilities,execution_protocol_version,execution_last_seen_at FROM execution_workers WHERE app_id=$1 AND profile_id=$2 AND revoked=false",vec![manifest.app_id.into(),manifest.profile.id.into()]).await?;
    let mut live = false;
    let mut model_live = false;
    let mut compatible_live = false;
    let mut last_compatible = None;
    let threshold = chrono::Utc::now() - chrono::Duration::seconds(90);
    for row in workers {
        let seen: Option<chrono::DateTime<chrono::Utc>> = field(&row, "execution_last_seen_at")?;
        let version: Option<i32> = field(&row, "execution_protocol_version")?;
        let capabilities =
            field::<Option<serde_json::Value>>(&row, "execution_model_capabilities")?.and_then(
                |v| decode::<mobile_qa_contracts::model_registry::WorkerModelCapabilities>(v).ok(),
            );
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
    let reservations=rows(db,"SELECT a.state AS attempt_state,s.payload->>'state' AS phone_state,br.id AS blocking_run_id,br.app_id AS blocking_app_id FROM execution_reservations r LEFT JOIN execution_attempts a ON a.id=r.attempt_id LEFT JOIN execution_runs br ON br.id=a.run_id LEFT JOIN phone_sessions s ON s.id=r.session_id WHERE r.resource=$1 OR r.resource=$2",vec![format!("app:{}",manifest.app_id).into(),format!("device:{}",manifest.profile.device_identity).into()]).await?;
    let mut recovery_required = false;
    let mut blocking_run_id = None;
    for row in &reservations {
        let attempt_state: Option<String> = field(row, "attempt_state")?;
        let phone_state: Option<String> = field(row, "phone_state")?;
        if attempt_state.as_deref() == Some("recovery_required") {
            recovery_required = true;
            if field::<Option<Uuid>>(row, "blocking_app_id")? == Some(manifest.app_id) {
                blocking_run_id = field(row, "blocking_run_id")?;
            }
        } else if phone_state.as_deref() == Some("quarantined") {
            recovery_required = true;
        }
    }
    let reason = if recovery_required {
        QueueReason::DeviceRecoveryRequired
    } else if !reservations.is_empty() {
        QueueReason::CapacityBusy
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
    one(
        &tx,
        "SELECT id FROM execution_runs WHERE id=$1 FOR UPDATE",
        vec![id.into()],
    )
    .await?;
    exec(
        &tx,
        "UPDATE execution_runs SET cancel_requested=true WHERE id=$1",
        vec![id.into()],
    )
    .await?;
    exec(
        &tx,
        "UPDATE execution_attempts SET state='finished',outcome='canceled',\
            cleanup='verified_clean',reason='canceled_before_dispatch' WHERE run_id=$1 AND \
            state='queued'",
        vec![id.into()],
    )
    .await?;
    exec(
        &tx,
        "UPDATE execution_attempts SET state='cancel_requested' WHERE run_id=$1 AND state IN \
            ('leased','running')",
        vec![id.into()],
    )
    .await?;
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
    let ids = rows(
        &ctx.db,
        "SELECT id FROM execution_runs WHERE app_id=$1 AND ($2::uuid IS NULL OR id<$2) ORDER \
            BY id DESC LIMIT 21",
        vec![app.into(), cursor.into()],
    )
    .await?;
    let mut items = vec![];
    for r in ids.iter().take(20) {
        items.push(detail(&ctx.db, field(r, "id")?).await?);
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
