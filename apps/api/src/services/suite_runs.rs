//! Saved suites enter the same immutable manifest and attempt pipeline as release plans.
use super::{
    apps, execution_store::*, run_baselines, runs, test_definitions as definitions, test_library,
};
use crate::{
    domain::regression,
    errors::{ApiFailure, ApiResult},
    models::_entities::{apps as app_rows, environments, execution_attempts, execution_runs},
};
use loco_rs::app::AppContext;
use mobile_qa_contracts::{execution::*, regression::*};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Set,
    TransactionTrait,
};
use uuid::Uuid;

pub(crate) async fn commercial_manifest(
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
    let preview = commercial_manifest(&ctx.db, app, &input).await?;
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
    create_with_quote(ctx, actor, app, key, input, None).await
}

pub async fn create_with_quote(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    key: &str,
    input: SuiteRunRequest,
    quote_id: Option<Uuid>,
) -> ApiResult<(RunResponse, bool)> {
    apps::authorized(ctx, actor, app).await?;
    if !bounded(key, 128) {
        return Err(ApiFailure::invalid(
            "Idempotency-Key must contain 1–128 printable bytes",
        ));
    }
    let fingerprint = if let Some(id) = quote_id {
        hash(
            serde_json::to_vec(&("saved_suite_v1", actor, &input, id))
                .map_err(|_| ApiFailure::internal())?,
        )
    } else {
        hash(
            serde_json::to_vec(&("saved_suite_v1", actor, &input))
                .map_err(|_| ApiFailure::internal())?,
        )
    };
    let tx = ctx.db.begin().await?;
    app_rows::Entity::find_by_id(app)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if let Some(row) = execution_runs::Entity::find()
        .filter(execution_runs::Column::AppId.eq(app))
        .filter(execution_runs::Column::IdempotencyKey.eq(key))
        .one(&tx)
        .await?
    {
        if row.fingerprint != fingerprint {
            return Err(conflict("Idempotency key belongs to another submission"));
        }
        let run = runs::detail(&tx, row.id).await?;
        tx.commit().await?;
        return Ok((run, false));
    }
    environments::Entity::find()
        .filter(environments::Column::AppId.eq(app))
        .lock_shared()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let preview = commercial_manifest(&tx, app, &input).await?;
    if !preview.blockers.is_empty() {
        return Err(ApiFailure::invalid(preview.blockers.join("; ")));
    }
    let manifest = preview.manifest.ok_or_else(ApiFailure::internal)?;
    if manifest.environment_revision != input.environment_revision {
        return Err(conflict("Environment changed; refresh the run setup"));
    }
    if let Some(baseline_id) = input.baseline_run_id {
        execution_runs::Entity::find_by_id(baseline_id)
            .filter(execution_runs::Column::AppId.eq(app))
            .one(&tx)
            .await?
            .ok_or_else(ApiFailure::missing)?;
        if runs::detail(&tx, baseline_id).await?.state != JobState::Finished {
            return Err(conflict("Baseline must be a completed run"));
        }
    }
    let id = Uuid::new_v4();
    execution_runs::ActiveModel {
        id: Set(id),
        app_id: Set(app),
        creator_id: Set(actor),
        build_id: Set(input.build_id),
        plan_id: Set(None),
        idempotency_key: Set(key.to_owned()),
        fingerprint: Set(fingerprint),
        manifest: Set(json(&manifest)?),
        baseline_run_id: Set(input.baseline_run_id),
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
        "saved_suite",
        &super::commercial::suite_hash(actor, &input)?,
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
