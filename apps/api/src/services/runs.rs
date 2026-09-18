//! Freeze approved coverage before queuing. Reports are projections of durable facts.
use super::{apps, execution_store::*, test_definitions as definitions};
use crate::errors::{ApiFailure, ApiResult};
use loco_rs::app::AppContext;
use mobile_qa_contracts::execution::*;
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
    let b = one(
        db,
        "SELECT * FROM builds WHERE id=$1 AND app_id=$2",
        vec![build.into(), app.into()],
    )
    .await?;
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
                .push("Choose an approved default release plan in Tests".into());
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
    if !definitions::approved(&d) {
        out.blockers.push("Release check is not approved".into());
        return Ok(out);
    }
    let TestDefinition::Plan(p) = &d.definition else {
        return Err(ApiFailure::invalid("Expected a plan version"));
    };
    let profile = definitions::profile(db, app, p.profile_id).await?;
    let cases = match definitions::resolve(db, app, p).await {
        Ok(v) => v,
        Err(e) => {
            out.blockers.push(e.message);
            return Ok(out);
        }
    };
    if cases.iter().any(|c| {
        c.case
            .actions
            .iter()
            .any(|a| a.kind == ActionKind::Navigate)
    }) && profile.model.is_empty()
    {
        out.blockers
            .push("This plan includes Ask AI steps but the device has no model".into());
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
        plan_version_id: d.id,
        plan_hash: d.content_hash,
        environment_revision: field(&env, "revision")?,
        profile,
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
    tracing::info!(run_id=%id,app_id=%app,phase="queued","Approved execution queued");
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
        "SELECT * FROM execution_runs WHERE id=$1",
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
    Ok(RunResponse {
        id,
        manifest,
        state,
        summary,
        created_at: field(&row, "created_at")?,
        attempts,
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
            let a: Vec<_> = attempts
                .iter()
                .filter(|a| a.case_version_id == c.definition_id)
                .collect();
            a.is_empty()
                || a.iter().any(|a| {
                    a.outcome != Some(Outcome::Passed)
                        || a.state != JobState::Finished
                        || a.cleanup != CleanupState::VerifiedClean
                        || !a.recovery_events.is_empty()
                        || a.original_cleanup
                            .as_ref()
                            .is_some_and(|r| !r.stopped || r.reset != CleanupState::VerifiedClean)
                        || (manifest.profile.execution_context.is_some() && a.preflight.is_none())
                })
        })
    {
        return "Incomplete / review required".into();
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
