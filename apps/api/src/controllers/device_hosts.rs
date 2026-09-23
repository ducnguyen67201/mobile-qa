//! Internal capacity endpoints use dedicated scoped credentials, never browser sessions.
use crate::{
    errors::{ApiFailure, ApiResult},
    services::{capacity_control, device_hosts},
};
use axum::{
    extract::{Path, State},
    http::HeaderMap,
    Json,
};
use loco_rs::{app::AppContext, controller::Routes};
use mobile_qa_contracts::device_hosts::*;
use uuid::Uuid;
fn bearer(h: &HeaderMap) -> ApiResult<&str> {
    h.get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(ApiFailure::unauthorized)
}
async fn register(
    State(ctx): State<AppContext>,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<HostRegisterRequest>,
) -> ApiResult<Json<HostStatus>> {
    device_hosts::authenticate_host(&ctx.db, id, bearer(&h)?).await?;
    Ok(Json(device_hosts::register(&ctx, id, input).await?))
}
async fn heartbeat(
    State(ctx): State<AppContext>,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<HostHeartbeatRequest>,
) -> ApiResult<Json<HostStatus>> {
    device_hosts::authenticate_host(&ctx.db, id, bearer(&h)?).await?;
    Ok(Json(device_hosts::heartbeat(&ctx, id, input).await?))
}
async fn grant(
    State(ctx): State<AppContext>,
    Path((id, slot)): Path<(Uuid, Uuid)>,
    h: HeaderMap,
    Json(input): Json<SlotGrantRequest>,
) -> ApiResult<Json<SlotGrantResponse>> {
    device_hosts::authenticate_host(&ctx.db, id, bearer(&h)?).await?;
    Ok(Json(device_hosts::grant(&ctx, id, slot, input).await?))
}
async fn cleanup(
    State(ctx): State<AppContext>,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<HostCleanupRequest>,
) -> ApiResult<Json<HostStatus>> {
    device_hosts::authenticate_host(&ctx.db, id, bearer(&h)?).await?;
    Ok(Json(device_hosts::cleanup(&ctx, id, input).await?))
}
async fn snapshot(
    State(ctx): State<AppContext>,
    Path(pool): Path<Uuid>,
    h: HeaderMap,
) -> ApiResult<Json<CapacitySnapshot>> {
    device_hosts::authenticate_control(&ctx.db, pool, bearer(&h)?).await?;
    Ok(Json(capacity_control::snapshot(&ctx.db, pool).await?))
}
async fn action(
    State(ctx): State<AppContext>,
    Path(pool): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<CapacityActionRequest>,
) -> ApiResult<Json<CapacitySnapshot>> {
    device_hosts::authenticate_control(&ctx.db, pool, bearer(&h)?).await?;
    Ok(Json(capacity_control::action(&ctx, pool, input).await?))
}
async fn hint(
    State(ctx): State<AppContext>,
    Path(pool): Path<Uuid>,
    h: HeaderMap,
    Json(input): Json<ConsumeHintRequest>,
) -> ApiResult<Json<HintReceipt>> {
    device_hosts::authenticate_control(&ctx.db, pool, bearer(&h)?).await?;
    Ok(Json(
        capacity_control::consume_hint(&ctx, pool, input).await?,
    ))
}
pub fn routes() -> Routes {
    use axum::routing::{get, post};
    Routes::new()
        .add("/api/internal/hosts/{host_id}/register", post(register))
        .add("/api/internal/hosts/{host_id}/heartbeat", post(heartbeat))
        .add("/api/internal/hosts/{host_id}/cleanup", post(cleanup))
        .add(
            "/api/internal/hosts/{host_id}/slots/{slot_id}/grant",
            post(grant),
        )
        .add("/api/internal/capacity/pools/{pool_id}", get(snapshot))
        .add(
            "/api/internal/capacity/pools/{pool_id}/actions",
            post(action),
        )
        .add("/api/internal/capacity/pools/{pool_id}/hints", post(hint))
}
