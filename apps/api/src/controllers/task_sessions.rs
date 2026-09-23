//! Browser and worker identities remain separate; screenshots use the private session boundary.
use crate::{
    config::Setup,
    errors::{ApiFailure, ApiResult},
    models::_entities::builds,
    services::{
        auth::Session, execution_store::conflict, task_sessions as phones, worker_auth::Worker,
    },
};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    http::HeaderMap,
    response::Response,
    Json,
};
use loco_rs::{app::AppContext, controller::Routes};
use mobile_qa_contracts::task_sessions::*;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use uuid::Uuid;
async fn options(
    State(ctx): State<AppContext>,
    s: Session,
    Path(app): Path<Uuid>,
) -> ApiResult<Json<PhoneOptions>> {
    Ok(Json(phones::options(&ctx, s.user.id, app).await?))
}
async fn open(
    State(ctx): State<AppContext>,
    s: Session,
    Path(app): Path<Uuid>,
    Json(input): Json<OpenPhoneRequest>,
) -> ApiResult<Json<PhoneSession>> {
    Ok(Json(phones::open(&ctx, s.user.id, app, input).await?))
}
async fn detail(
    State(ctx): State<AppContext>,
    s: Session,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<PhoneSession>> {
    Ok(Json(phones::authorized(&ctx, s.user.id, id).await?))
}
async fn task(
    State(ctx): State<AppContext>,
    s: Session,
    Path(id): Path<Uuid>,
    Json(input): Json<PhoneTaskRequest>,
) -> ApiResult<Json<PhoneSession>> {
    Ok(Json(phones::task(&ctx, s.user.id, id, input).await?))
}
async fn stop(
    State(ctx): State<AppContext>,
    s: Session,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<PhoneSession>> {
    Ok(Json(phones::stop(&ctx, s.user.id, id).await?))
}
async fn claim(
    State(ctx): State<AppContext>,
    w: Worker,
    Json(input): Json<PhoneClaimRequest>,
) -> ApiResult<Json<PhoneClaimResponse>> {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        crate::services::worker_auth::lease_authority(&ctx.db, &w).await?;
        let reply = phones::claim(&ctx, &w, input.clone()).await?;
        if reply.lease.is_some() || tokio::time::Instant::now() >= deadline {
            return Ok(Json(reply));
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}
fn token(h: &HeaderMap) -> ApiResult<&str> {
    h.get("x-lease-token")
        .and_then(|s| s.to_str().ok())
        .ok_or_else(ApiFailure::unauthorized)
}
async fn update(
    State(ctx): State<AppContext>,
    w: Worker,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<PhoneUpdate>,
) -> ApiResult<Json<PhoneSession>> {
    Ok(Json(phones::update(&ctx, &w, id, token(&h)?, input).await?))
}
async fn build(
    State(ctx): State<AppContext>,
    w: Worker,
    Path(id): Path<Uuid>,
    h: HeaderMap,
) -> ApiResult<Response> {
    let r = phones::lease(&ctx.db, &w, id, token(&h)?).await?;
    let b = builds::Entity::find_by_id(r.build_id)
        .filter(builds::Column::AppId.eq(w.app_id))
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    crate::services::build_delivery::stream(
        &Setup::get(&ctx),
        &b.storage_key,
        &b.storage_backend,
        r.build_bytes,
        &r.build_sha256,
    )
    .await
}
async fn delivery(
    State(ctx): State<AppContext>,
    w: Worker,
    Path(id): Path<Uuid>,
    h: HeaderMap,
) -> ApiResult<Json<mobile_qa_contracts::artifacts_api::BuildDelivery>> {
    let r = phones::lease(&ctx.db, &w, id, token(&h)?).await?;
    let b = builds::Entity::find_by_id(r.build_id)
        .filter(builds::Column::AppId.eq(w.app_id))
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let sha = r.build_sha256;
    let size = r.build_bytes;
    if b.sha256 != sha || b.byte_size != size {
        return Err(conflict("Build identity changed"));
    }
    Ok(Json(crate::services::build_delivery::grant(
        &Setup::get(&ctx),
        &b.storage_key,
        &b.storage_backend,
        w.app_id,
        size,
        sha,
        format!("/api/worker/phones/{id}/build"),
    )?))
}

pub fn routes() -> Routes {
    use axum::routing::{get, post};
    Routes::new()
        .add(
            "/api/worker/phones/{session_id}/build/delivery",
            get(delivery),
        )
        .add("/api/apps/{app_id}/phone-options", get(options))
        .add("/api/apps/{app_id}/phones", post(open))
        .add("/api/phones/{session_id}", get(detail))
        .add("/api/phones/{session_id}/tasks", post(task))
        .add("/api/phones/{session_id}/stop", post(stop))
        .add("/api/worker/phone-claims", post(claim))
        .add(
            "/api/worker/phones/{session_id}/update",
            post(update).layer(DefaultBodyLimit::max(16777216)),
        )
        .add("/api/worker/phones/{session_id}/build", get(build))
}
