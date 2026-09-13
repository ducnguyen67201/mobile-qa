//! Explicit worker HTTP protocol. Device execution runs only in the separate worker process.
use crate::{
    config::Setup,
    errors::{ApiFailure, ApiResult},
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
async fn complete(
    State(ctx): State<AppContext>,
    w: Worker,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<CompleteRequest>,
) -> ApiResult<Json<AttemptReceipt>> {
    Ok(Json(
        scheduler::complete(&ctx, &w, id, token(&h)?, input).await?,
    ))
}
async fn cleanup(
    State(ctx): State<AppContext>,
    w: Worker,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<CleanupRequest>,
) -> ApiResult<Json<AttemptReceipt>> {
    Ok(Json(
        scheduler::cleanup(&ctx, &w, id, token(&h)?, input).await?,
    ))
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
    let manifest: RunManifest = decode(field(&r, "manifest")?)?;
    let b = one(
        &ctx.db,
        "SELECT storage_key,storage_backend,sha256 FROM builds WHERE id=$1 AND app_id=$2",
        vec![manifest.build_id.into(), w.app_id.into()],
    )
    .await?;
    if field::<String>(&b, "sha256")? != manifest.build_sha256 {
        return Err(conflict("Build identity changed"));
    }
    let file = Setup::get(&ctx)
        .store
        .materialize(
            &field::<String>(&b, "storage_key")?,
            &field::<String>(&b, "storage_backend")?,
        )
        .await?;
    let bytes = tokio::fs::read(&file.0).await?;
    if hash(&bytes) != manifest.build_sha256 || bytes.len() != manifest.build_bytes as usize {
        return Err(conflict("Stored build differs from manifest"));
    }
    let mut r = Response::new(Body::from(bytes));
    r.headers_mut().insert(
        "content-type",
        "application/vnd.android.package-archive"
            .parse()
            .expect("literal"),
    );
    Ok(r)
}
pub fn routes() -> Routes {
    use axum::routing::{get, post, put};
    Routes::new()
        .add("/api/worker/claims", post(claim))
        .add(
            "/api/worker/attempts/{attempt_id}/heartbeat",
            post(heartbeat),
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
