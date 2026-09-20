//! Saved cases enter the same immutable manifest/attempt pipeline as release plans.
use super::{apps, execution_store::*, runs, test_definitions as definitions, test_library};
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
    let mut baselines = vec![];
    // Keep the recent choices compact, but continue keyset scanning until the
    // latest eligible result is found. Unrelated runs must not hide a baseline.
    let mut cursor_time: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut cursor_id: Option<Uuid> = None;
    loop {
        let page = rows(&ctx.db,
            "SELECT r.id,r.created_at,COALESCE(b.metadata->>'version_name',b.original_filename) AS label \
             FROM execution_runs r JOIN builds b ON b.id=r.build_id WHERE r.app_id=$1 \
             AND EXISTS (SELECT 1 FROM execution_attempts a WHERE a.run_id=r.id) \
             AND NOT EXISTS (SELECT 1 FROM execution_attempts a WHERE a.run_id=r.id AND a.state <> 'finished') \
             AND ($2::timestamptz IS NULL OR (r.created_at,r.id)<($2,$3::uuid)) \
             ORDER BY r.created_at DESC,r.id DESC LIMIT 100",
            vec![app.into(), cursor_time.into(), cursor_id.into()]).await?;
        for row in &page {
            let r = runs::detail(&ctx.db, field(row, "id")?).await?;
            let compatible = regression::eligible(&m, &r);
            cursor_time = Some(r.created_at);
            cursor_id = Some(r.id);
            if baselines.len() < 100 || compatible {
                baselines.push(BaselineChoice {
                    id: r.id,
                    build_id: r.manifest.build_id,
                    build_label: field(row, "label")?,
                    created_at: r.created_at,
                    compatible,
                    reason: if compatible {
                        "Same test version and execution context"
                    } else {
                        "Different test/context or no conclusive, clean result"
                    }
                    .into(),
                });
            }
        }
        if page.len() < 100 || baselines.iter().any(|b| b.compatible) {
            break;
        }
    }
    let suggested_baseline_id = baselines.iter().find(|b| b.compatible).map(|b| b.id);
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
        let r = runs::detail(&tx, field(row, "id")?).await?;
        tx.commit().await?;
        return Ok((r, false));
    }
    one(
        &tx,
        "SELECT id FROM environments WHERE app_id=$1 FOR SHARE",
        vec![app.into()],
    )
    .await?;
    let p = manifest(&tx, app, &input).await?;
    if !p.blockers.is_empty() {
        return Err(ApiFailure::invalid(p.blockers.join("; ")));
    }
    let m = p.manifest.ok_or_else(ApiFailure::internal)?;
    if m.environment_revision != input.environment_revision {
        return Err(conflict("Environment changed; refresh the run setup"));
    }
    if let Some(id) = input.baseline_run_id {
        one(
            &tx,
            "SELECT id FROM execution_runs WHERE id=$1 AND app_id=$2",
            vec![id.into(), app.into()],
        )
        .await?;
        let baseline = runs::detail(&tx, id).await?;
        if baseline.state != JobState::Finished {
            return Err(conflict("Baseline must be a completed run"));
        }
        // An explicit incompatible baseline is retained and explained, never silently replaced.
    }
    let id = Uuid::new_v4();
    exec(&tx,"INSERT INTO execution_runs(id,app_id,creator_id,build_id,plan_id,idempotency_key,fingerprint,manifest,baseline_run_id) VALUES($1,$2,$3,$4,NULL,$5,$6,$7,$8)",vec![id.into(),app.into(),actor.into(),input.build_id.into(),key.into(),fingerprint.into(),json(&m)?.into(),input.baseline_run_id.into()]).await?;
    exec(
        &tx,
        "INSERT INTO execution_attempts(id,run_id,case_index) VALUES($1,$2,0)",
        vec![Uuid::new_v4().into(), id.into()],
    )
    .await?;
    let r = runs::detail(&tx, id).await?;
    tx.commit().await?;
    super::execution_wakeup::notify(ctx);
    Ok((r, true))
}
