//! Saved cases enter the same immutable manifest/attempt pipeline as release plans.
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

async fn manifest(
    db: &impl ConnectionTrait,
    app: Uuid,
    input: &CaseRunRequest,
) -> ApiResult<PlanPreviewResponse> {
    test_library::admitted(db, app, input.case_version_id).await?;
    let d = definitions::get(db, app, input.case_version_id).await?;
    let TestDefinition::Case(case) = &d.definition else {
        return Err(ApiFailure::invalid("Choose a saved test case"));
    };
    let profile = definitions::profile(db, app, input.profile_id).await?;
    let mut budget = case.budget.clone();
    budget.duration_seconds =
        u32::try_from(super::execution_readiness::duration(&profile, case))
            .map_err(|_| ApiFailure::invalid("Case duration exceeds run limits"))?;
    let cases = vec![ResolvedCase {
        definition_id: d.id,
        content_hash: d.content_hash.clone(),
        data_variant: "default".into(),
        required: true,
        case: case.clone(),
    }];
    // A transient policy supplies scheduler limits; it is never a hidden library plan.
    let policy = PlanDefinition {
        key: case.key.clone(),
        version: case.version,
        title: case.title.clone(),
        suite_version_ids: vec![],
        cases: vec![],
        profile_id: input.profile_id,
        budget,
        diagnostic_retries: 0,
        exclusions: vec![],
    };
    runs::assemble(
        db,
        app,
        input.build_id,
        d,
        policy,
        cases,
        Some(RunSource::SavedCaseV1 {
            case_version_id: input.case_version_id,
        }),
    )
    .await
}
pub async fn preview(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    input: CaseRunRequest,
) -> ApiResult<CaseRunPreview> {
    apps::authorized(ctx, actor, app).await?;
    let p = manifest(&ctx.db, app, &input).await?;
    let m = p.manifest.ok_or_else(ApiFailure::internal)?;
    let (baselines, suggested_baseline_id) = run_baselines::choices(
        &ctx.db,
        app,
        |run| regression::eligible(&m, run),
        "Same test version and execution context",
    )
    .await?;
    Ok(CaseRunPreview {
        blockers: p.blockers,
        environment_revision: m.environment_revision,
        baselines,
        suggested_baseline_id,
    })
}
pub async fn create(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    key: &str,
    input: CaseRunRequest,
) -> ApiResult<(RunResponse, bool)> {
    apps::authorized(ctx, actor, app).await?;
    if !bounded(key, 128) {
        return Err(ApiFailure::invalid(
            "Idempotency-Key must contain 1–128 printable bytes",
        ));
    }
    let fingerprint = hash(
        serde_json::to_vec(&("saved_case_v1", actor, &input))
            .map_err(|_| ApiFailure::internal())?,
    );
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
        let r = runs::detail(&tx, row.id).await?;
        tx.commit().await?;
        return Ok((r, false));
    }
    super::commercial::require_separate_scope(ctx, actor, app).await?;
    environments::Entity::find()
        .filter(environments::Column::AppId.eq(app))
        .lock_shared()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let p = manifest(&tx, app, &input).await?;
    if !p.blockers.is_empty() {
        return Err(ApiFailure::invalid(p.blockers.join("; ")));
    }
    let m = p.manifest.ok_or_else(ApiFailure::internal)?;
    if m.environment_revision != input.environment_revision {
        return Err(conflict("Environment changed; refresh the run setup"));
    }
    if let Some(id) = input.baseline_run_id {
        execution_runs::Entity::find_by_id(id)
            .filter(execution_runs::Column::AppId.eq(app))
            .one(&tx)
            .await?
            .ok_or_else(ApiFailure::missing)?;
        let baseline = runs::detail(&tx, id).await?;
        if baseline.state != JobState::Finished {
            return Err(conflict("Baseline must be a completed run"));
        }
        // An explicit incompatible baseline is retained and explained, never silently replaced.
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
        manifest: Set(json(&m)?),
        baseline_run_id: Set(input.baseline_run_id),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    execution_attempts::ActiveModel {
        id: Set(Uuid::new_v4()),
        run_id: Set(id),
        case_index: Set(0),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    super::capacity_control::enqueue(&tx, app, m.profile.id).await?;
    let r = runs::detail(&tx, id).await?;
    tx.commit().await?;
    super::execution_wakeup::notify(ctx);
    let reason_code = r
        .queue_status
        .as_ref()
        .map(|status| word(&status.reason))
        .unwrap_or_else(|| "unknown".into());
    tracing::info!(run_id=%id,app_id=%app,phase="queued",source="saved_case_v1",reason_code=%reason_code,"Saved case queued");
    Ok((r, true))
}
