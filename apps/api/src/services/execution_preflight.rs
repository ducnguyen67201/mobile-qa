//! Fenced start acknowledgement. A worker cannot report actions before retained start proof.
use super::{
    execution_store::{conflict, decode, hash, json},
    run_artifacts, scheduler, verification,
    worker_auth::Worker,
};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::execution_preflight_receipts,
};
use loco_rs::app::AppContext;
use mobile_qa_contracts::{execution::*, execution_lifecycle::*};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ConnectionTrait, EntityTrait, TransactionTrait};
use uuid::Uuid;

pub async fn recorded(db: &impl ConnectionTrait, id: Uuid, generation: i32) -> ApiResult<bool> {
    Ok(
        execution_preflight_receipts::Entity::find_by_id((id, generation))
            .one(db)
            .await?
            .is_some(),
    )
}
pub async fn require(
    db: &impl ConnectionTrait,
    id: Uuid,
    generation: i32,
    manifest: &RunManifest,
) -> ApiResult<()> {
    if manifest.profile.execution_context.is_some() && !recorded(db, id, generation).await? {
        return Err(conflict("The app's clean start has not been verified"));
    }
    Ok(())
}
pub async fn acknowledge(
    ctx: &AppContext,
    w: &Worker,
    id: Uuid,
    token: &str,
    input: PreflightRequest,
) -> ApiResult<PreflightAcknowledgement> {
    let row = scheduler::lease(&ctx.db, w, id, input.generation, token, false).await?;
    let manifest: RunManifest = decode(row.run.manifest)?;
    let expected = manifest
        .profile
        .execution_context
        .as_ref()
        .ok_or_else(|| conflict("This run does not accept a start receipt"))?;
    let r = &input.receipt;
    let elapsed = r
        .ready_at
        .signed_duration_since(r.started_at)
        .num_milliseconds();
    if r.attempt_id != id
        || r.instance_nonce.is_nil()
        || &r.context != expected
        || r.build_sha256 != manifest.build_sha256
        || r.artifact_ids.len() != 2
        || r.artifact_ids[0] == r.artifact_ids[1]
        || elapsed < 0
        || elapsed > (expected.stages.overhead() * 1000) as i64
        || u64::from(r.duration_ms) > expected.stages.overhead() * 1000
        || r.ready_at > chrono::Utc::now() + chrono::Duration::seconds(30)
    {
        return Err(ApiFailure::invalid(
            "Start evidence does not match this run",
        ));
    }
    let mut xml = None;
    let mut png = None;
    for artifact in &r.artifact_ids {
        let (a, file) = run_artifacts::content(ctx, *artifact).await?;
        if a.attempt_id != id || a.checkpoint_id != "preflight" {
            return Err(conflict("Start evidence belongs to another attempt"));
        }
        let bytes = tokio::fs::read(&file.0).await?;
        match (a.name.as_str(), a.mime.as_str()) {
            ("preflight.xml", "application/xml") => xml = Some(bytes),
            ("preflight.png", "image/png") => png = Some(bytes),
            _ => {
                return Err(ApiFailure::invalid(
                    "Start evidence must include the screen and controls",
                ))
            }
        }
    }
    let xml = xml.ok_or_else(|| conflict("Start controls are missing"))?;
    if !png.as_ref().is_some_and(|p| verification::png_valid(p)) {
        return Err(conflict("Start screen is invalid"));
    }
    for check in &expected.starting_checks {
        if verification::observe(&xml, &expected.package, check, "")
            .ok()
            .as_deref()
            != Some(check.expected.as_str())
        {
            return Err(conflict("The app did not reach its expected clean start"));
        }
    }
    let digest = hash(serde_json::to_vec(&input).map_err(|_| ApiFailure::internal())?);
    let tx = ctx.db.begin().await?;
    let locked = scheduler::lease(&tx, w, id, input.generation, token, true).await?;
    let old = execution_preflight_receipts::Entity::find_by_id((id, input.generation))
        .one(&tx)
        .await?;
    if let Some(old) = old {
        if old.digest != digest {
            return Err(conflict("Start receipt changed"));
        }
    } else {
        if !["leased", "running"].contains(&locked.attempt.state.as_str())
            || locked.run.cancel_requested
        {
            return Err(conflict("This attempt no longer accepts a start receipt"));
        }
        execution_preflight_receipts::ActiveModel {
            attempt_id: Set(id),
            generation: Set(input.generation),
            digest: Set(digest),
            payload: Set(json(r)?),
            ..Default::default()
        }
        .insert(&tx)
        .await?;
    }
    tx.commit().await?;
    Ok(PreflightAcknowledgement {
        attempt_id: id,
        generation: input.generation,
        accepted: true,
    })
}
