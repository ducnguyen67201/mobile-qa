//! Saved suites enter the same immutable manifest and attempt pipeline as release plans.
use super::{
    apps, execution_store::*, run_baselines, runs, test_definitions as definitions, test_library,
};
use crate::{
    domain::regression,
    errors::{ApiFailure, ApiResult},
};
use loco_rs::app::AppContext;
use mobile_qa_contracts::{execution::*, regression::*};
use sea_orm::{ConnectionTrait, TransactionTrait};
use uuid::Uuid;

async fn manifest(
    db: &impl ConnectionTrait,
    app: Uuid,
    input: &SuiteRunRequest,
) -> ApiResult<PlanPreviewResponse> {
    test_library::admitted(db, app, input.suite_version_id).await?;
    let definition = definitions::get(db, app, input.suite_version_id).await?;
    let TestDefinition::Suite(suite) = &definition.definition else {
        return Err(ApiFailure::invalid("Choose a saved test suite"));
    };
    let resolution_policy = PlanDefinition {
        key: suite.key.clone(),
        version: suite.version,
        title: suite.title.clone(),
        suite_version_ids: vec![input.suite_version_id],
        cases: vec![],
        profile_id: input.profile_id,
        budget: ExecutionBudget {
            duration_seconds: 1800,
            max_steps: 400,
            artifact_bytes: 262_144_000,
        },
        diagnostic_retries: 0,
        exclusions: vec![],
    };
    let cases = definitions::resolve(db, app, &resolution_policy).await?;
    let profile = definitions::profile(db, app, input.profile_id).await?;
    let required_duration: u64 = cases
        .iter()
        .map(|case| super::execution_readiness::duration(&profile, &case.case))
        .sum();
    let budget = ExecutionBudget {
        duration_seconds: u32::try_from(required_duration.min(1800))
            .unwrap_or(1800)
            .max(1),
        max_steps: cases
            .iter()
            .map(|case| case.case.budget.max_steps)
            .max()
            .unwrap_or(1),
        artifact_bytes: cases
            .iter()
            .map(|case| case.case.budget.artifact_bytes)
            .max()
            .unwrap_or(1024),
    };
    let policy = PlanDefinition {
        budget,
        ..resolution_policy
    };
    runs::assemble(
        db,
        app,
        input.build_id,
        definition,
        policy,
        cases,
        Some(RunSource::SavedSuiteV1 {
            suite_version_id: input.suite_version_id,
        }),
    )
    .await
}

pub async fn preview(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    input: SuiteRunRequest,
) -> ApiResult<SuiteRunPreview> {
    apps::authorized(ctx, actor, app).await?;
    let preview = manifest(&ctx.db, app, &input).await?;
    let manifest = preview.manifest.ok_or_else(ApiFailure::internal)?;
    let (baselines, suggested_baseline_id) = run_baselines::choices(
        &ctx.db,
        app,
        |run| regression::suite_baseline_eligible(&manifest, run),
        "Same suite version and conclusive execution context",
    )
    .await?;
    Ok(SuiteRunPreview {
        blockers: preview.blockers,
        environment_revision: manifest.environment_revision,
        baselines,
        suggested_baseline_id,
    })
}

pub async fn create(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    key: &str,
    input: SuiteRunRequest,
) -> ApiResult<(RunResponse, bool)> {
    apps::authorized(ctx, actor, app).await?;
    if !bounded(key, 128) {
        return Err(ApiFailure::invalid(
            "Idempotency-Key must contain 1–128 printable bytes",
        ));
    }
    let fingerprint = hash(
        serde_json::to_vec(&("saved_suite_v1", actor, &input))
            .map_err(|_| ApiFailure::internal())?,
    );
    let tx = ctx.db.begin().await?;
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![app.into()],
    )
    .await?;
    if let Some(row) = rows(
        &tx,
        "SELECT id,fingerprint FROM execution_runs WHERE app_id=$1 AND idempotency_key=$2",
        vec![app.into(), key.into()],
    )
    .await?
    .first()
    {
        if field::<String>(row, "fingerprint")? != fingerprint {
            return Err(conflict("Idempotency key belongs to another submission"));
        }
        let run = runs::detail(&tx, field(row, "id")?).await?;
        tx.commit().await?;
        return Ok((run, false));
    }
    one(
        &tx,
        "SELECT id FROM environments WHERE app_id=$1 FOR SHARE",
        vec![app.into()],
    )
    .await?;
    let preview = manifest(&tx, app, &input).await?;
    if !preview.blockers.is_empty() {
        return Err(ApiFailure::invalid(preview.blockers.join("; ")));
    }
    let manifest = preview.manifest.ok_or_else(ApiFailure::internal)?;
    if manifest.environment_revision != input.environment_revision {
        return Err(conflict("Environment changed; refresh the run setup"));
    }
    if let Some(baseline_id) = input.baseline_run_id {
        one(
            &tx,
            "SELECT id FROM execution_runs WHERE id=$1 AND app_id=$2",
            vec![baseline_id.into(), app.into()],
        )
        .await?;
        if runs::detail(&tx, baseline_id).await?.state != JobState::Finished {
            return Err(conflict("Baseline must be a completed run"));
        }
    }
    let id = Uuid::new_v4();
    exec(
        &tx,
        "INSERT INTO execution_runs(id,app_id,creator_id,build_id,plan_id,idempotency_key,\
            fingerprint,manifest,baseline_run_id) VALUES($1,$2,$3,$4,NULL,$5,$6,$7,$8)",
        vec![
            id.into(),
            app.into(),
            actor.into(),
            input.build_id.into(),
            key.into(),
            fingerprint.into(),
            json(&manifest)?.into(),
            input.baseline_run_id.into(),
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
    let run = runs::detail(&tx, id).await?;
    tx.commit().await?;
    super::execution_wakeup::notify(ctx);
    let reason_code = run
        .queue_status
        .as_ref()
        .map(|status| word(&status.reason))
        .unwrap_or_else(|| "unknown".into());
    tracing::info!(run_id=%id,app_id=%app,phase="queued",source="saved_suite_v1",reason_code=%reason_code,"Saved suite queued");
    Ok((run, true))
}
