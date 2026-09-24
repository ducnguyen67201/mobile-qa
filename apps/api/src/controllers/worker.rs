//! Explicit worker HTTP protocol. Device execution runs only in the separate worker process.
use crate::{
    config::Setup,
    errors::{ApiFailure, ApiResult},
    models::_entities::builds,
    services::{execution_store::*, run_artifacts, scheduler, worker_auth::Worker},
};
use axum::{
    body::Body,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::HeaderMap,
    response::Response,
    Json,
};
use loco_rs::{app::AppContext, controller::Routes};
use mobile_qa_contracts::execution::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use uuid::Uuid;
fn token(h: &HeaderMap) -> ApiResult<&str> {
    h.get("x-lease-token")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiFailure::new(401, "lease_required", "A lease token is required"))
}
async fn claim(
    State(ctx): State<AppContext>,
    w: Worker,
    Json(input): Json<ClaimRequest>,
) -> ApiResult<Json<ClaimResponse>> {
    Ok(Json(
        scheduler::claim_wait(&ctx, &w, input, std::time::Duration::from_secs(30)).await?,
    ))
}
async fn heartbeat(
    State(ctx): State<AppContext>,
    w: Worker,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<LeaseRequest>,
) -> ApiResult<Json<LeaseStatusResponse>> {
    Ok(Json(
        scheduler::heartbeat(&ctx, &w, id, input.generation, token(&h)?).await?,
    ))
}
async fn preflight(
    State(ctx): State<AppContext>,
    w: Worker,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<mobile_qa_contracts::execution_lifecycle::PreflightRequest>,
) -> ApiResult<Json<mobile_qa_contracts::execution_lifecycle::PreflightAcknowledgement>> {
    Ok(Json(
        crate::services::execution_preflight::acknowledge(&ctx, &w, id, token(&h)?, input).await?,
    ))
}
async fn events(
    State(ctx): State<AppContext>,
    w: Worker,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<EventRequest>,
) -> ApiResult<Json<EventReceipt>> {
    Ok(Json(
        scheduler::events(&ctx, &w, id, token(&h)?, input).await?,
    ))
}
async fn reserve(
    State(ctx): State<AppContext>,
    w: Worker,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<ArtifactRequest>,
) -> ApiResult<(axum::http::StatusCode, Json<ArtifactReceipt>)> {
    Ok((
        axum::http::StatusCode::CREATED,
        Json(run_artifacts::reserve(&ctx, &w, id, token(&h)?, input).await?),
    ))
}
async fn upload(
    State(ctx): State<AppContext>,
    w: Worker,
    Path((id, artifact)): Path<(Uuid, Uuid)>,
    h: HeaderMap,
    body: Body,
) -> ApiResult<Json<ArtifactReceipt>> {
    let g = h
        .get("x-lease-generation")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| ApiFailure::invalid("Lease generation is required"))?;
    Ok(Json(
        run_artifacts::upload(&ctx, &w, id, artifact, g, token(&h)?, body).await?,
    ))
}
fn legacy_receipt(receipt: &mut AttemptReceipt) {
    // Cleanup transport does not need history. Keep protocol 1/2 readers strict and compatible.
    receipt.attempt.preflight = None;
    receipt.attempt.recovery_events.clear();
    receipt.attempt.original_cleanup = None;
}
async fn complete(
    State(ctx): State<AppContext>,
    w: Worker,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<CompleteRequest>,
) -> ApiResult<Json<AttemptReceipt>> {
    let mut receipt = scheduler::complete(&ctx, &w, id, token(&h)?, input).await?;
    legacy_receipt(&mut receipt);
    Ok(Json(receipt))
}
async fn cleanup(
    State(ctx): State<AppContext>,
    w: Worker,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<CleanupRequest>,
) -> ApiResult<Json<AttemptReceipt>> {
    let mut receipt = scheduler::cleanup(&ctx, &w, id, token(&h)?, input).await?;
    legacy_receipt(&mut receipt);
    Ok(Json(receipt))
}
#[derive(Deserialize)]
struct BuildQuery {
    generation: i32,
}
async fn build(
    State(ctx): State<AppContext>,
    w: Worker,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Query(q): Query<BuildQuery>,
) -> ApiResult<Response> {
    let r = scheduler::lease(&ctx.db, &w, id, q.generation, token(&h)?, false).await?;
    crate::services::build_delivery::require_active_attempt(&r.attempt.state)?;
    let manifest: RunManifest = decode(r.run.manifest)?;
    let b = builds::Entity::find_by_id(manifest.build_id)
        .filter(builds::Column::AppId.eq(w.app_id))
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if b.sha256 != manifest.build_sha256 || b.byte_size != i64::from(manifest.build_bytes) {
        return Err(conflict("Build identity changed"));
    }
    crate::services::build_delivery::stream(
        &Setup::get(&ctx),
        &b.storage_key,
        &b.storage_backend,
        i64::from(manifest.build_bytes),
        &manifest.build_sha256,
    )
    .await
}
async fn delivery(
    State(ctx): State<AppContext>,
    w: Worker,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Query(q): Query<BuildQuery>,
) -> ApiResult<Json<mobile_qa_contracts::artifacts_api::BuildDelivery>> {
    let r = scheduler::lease(&ctx.db, &w, id, q.generation, token(&h)?, false).await?;
    crate::services::build_delivery::require_active_attempt(&r.attempt.state)?;
    let manifest: RunManifest = decode(r.run.manifest)?;
    let b = builds::Entity::find_by_id(manifest.build_id)
        .filter(builds::Column::AppId.eq(w.app_id))
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if b.sha256 != manifest.build_sha256 || b.byte_size != i64::from(manifest.build_bytes) {
        return Err(conflict("Build identity changed"));
    }
    Ok(Json(crate::services::build_delivery::grant(
        &Setup::get(&ctx),
        &b.storage_key,
        &b.storage_backend,
        w.app_id,
        i64::from(manifest.build_bytes),
        manifest.build_sha256,
        format!(
            "/api/worker/attempts/{id}/build?generation={}",
            q.generation
        ),
    )?))
}

pub fn routes() -> Routes {
    use axum::routing::{get, post, put};
    Routes::new()
        .add(
            "/api/worker/attempts/{attempt_id}/build/delivery",
            get(delivery),
        )
        .add("/api/worker/claims", post(claim))
        .add(
            "/api/worker/attempts/{attempt_id}/heartbeat",
            post(heartbeat),
        )
        .add(
            "/api/worker/attempts/{attempt_id}/preflight",
            post(preflight),
        )
        .add("/api/worker/attempts/{attempt_id}/events", post(events))
        .add("/api/worker/attempts/{attempt_id}/artifacts", post(reserve))
        .add(
            "/api/worker/attempts/{attempt_id}/artifacts/{artifact_id}/content",
            put(upload).layer(DefaultBodyLimit::max(16777216)),
        )
        .add("/api/worker/attempts/{attempt_id}/complete", post(complete))
        .add("/api/worker/attempts/{attempt_id}/cleanup", post(cleanup))
        .add("/api/worker/attempts/{attempt_id}/build", get(build))
}
