//! Authenticated browser execution routes. Binary evidence remains private.
use crate::{
    errors::{ApiFailure, ApiResult},
    services::{
        apps, auth::Session, execution_store::*, run_artifacts, run_comparison, run_history, runs,
    },
};
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Response,
    Json,
};
use loco_rs::{app::AppContext, controller::Routes};
use mobile_qa_contracts::execution::*;
use mobile_qa_contracts::test_library::ExecutionPlanQuery;
use serde::Deserialize;
use uuid::Uuid;
#[derive(Deserialize)]
struct ListQuery {
    cursor: Option<Uuid>,
}
async fn preview(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Query(q): Query<ExecutionPlanQuery>,
) -> ApiResult<Json<PlanPreviewResponse>> {
    apps::authorized(&ctx, session.user.id, app).await?;
    Ok(Json(
        runs::preview(&ctx.db, app, q.build_id, q.plan_version_id).await?,
    ))
}
async fn preview_saved_case(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Query(query): Query<SavedCasePreviewQuery>,
) -> ApiResult<Json<PlanPreviewResponse>> {
    apps::authorized(&ctx, session.user.id, app).await?;
    Ok(Json(runs::preview_saved_case(&ctx.db, app, query).await?))
}
async fn baseline_candidates(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Query(query): Query<BaselineCandidateQuery>,
) -> ApiResult<Json<BaselineCandidateResponse>> {
    apps::authorized(&ctx, session.user.id, app).await?;
    Ok(Json(run_comparison::candidates(&ctx.db, app, query).await?))
}
async fn create(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<CreateRunRequest>,
) -> ApiResult<(StatusCode, Json<RunResponse>)> {
    let key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiFailure::invalid("Idempotency-Key is required"))?;
    let (run, new) = runs::create(&ctx, session.user.id, app, key, input).await?;
    Ok((
        if new {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(run),
    ))
}
async fn detail(
    State(ctx): State<AppContext>,
    session: Session,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RunResponse>> {
    runs::authorize(&ctx, session.user.id, id).await?;
    Ok(Json(runs::detail(&ctx.db, id).await?))
}
async fn list(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Query(q): Query<ListQuery>,
) -> ApiResult<Json<RunListResponse>> {
    Ok(Json(
        runs::list(&ctx, session.user.id, app, q.cursor).await?,
    ))
}
async fn history(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Query(query): Query<RunHistoryQuery>,
) -> ApiResult<Json<RunHistoryResponse>> {
    apps::authorized(&ctx, session.user.id, app).await?;
    Ok(Json(
        run_history::list(&ctx.db, session.user.id, app, query).await?,
    ))
}
async fn comparison(
    State(ctx): State<AppContext>,
    session: Session,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RunComparisonResponse>> {
    runs::authorize(&ctx, session.user.id, id).await?;
    Ok(Json(run_comparison::comparison(&ctx.db, id).await?))
}
async fn cancel(
    State(ctx): State<AppContext>,
    session: Session,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<RunResponse>> {
    Ok(Json(runs::cancel(&ctx, session.user.id, id).await?))
}
async fn artifact(
    State(ctx): State<AppContext>,
    session: Session,
    Path((run, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Response> {
    runs::authorize(&ctx, session.user.id, run).await?;
    one(&ctx.db,"SELECT f.id FROM execution_artifacts f JOIN execution_attempts a ON a.id=f.attempt_id WHERE f.id=$1 AND a.run_id=$2",vec![id.into(),run.into()]).await?;
    let (metadata, file) = run_artifacts::content(&ctx, id).await?;
    let bytes = tokio::fs::read(&file.0).await?;
    let mut response = Response::new(Body::from(bytes));
    let headers = response.headers_mut();
    headers.insert(
        "content-type",
        metadata.mime.parse().map_err(|_| ApiFailure::internal())?,
    );
    headers.insert(
        "x-content-type-options",
        "nosniff".parse().expect("literal"),
    );
    headers.insert(
        "content-security-policy",
        "default-src 'none'; sandbox".parse().expect("literal"),
    );
    let mode = if matches!(metadata.mime.as_str(), "image/png" | "video/mp4") {
        "inline"
    } else {
        "attachment"
    };
    headers.insert(
        "content-disposition",
        format!("{mode}; filename=\"{}\"", metadata.name)
            .parse()
            .map_err(|_| ApiFailure::internal())?,
    );
    Ok(response)
}
pub fn routes() -> Routes {
    use axum::routing::{get, post};
    Routes::new()
        .add("/api/apps/{app_id}/execution-plan", get(preview))
        .add(
            "/api/apps/{app_id}/saved-case-preview",
            get(preview_saved_case),
        )
        .add(
            "/api/apps/{app_id}/baseline-candidates",
            get(baseline_candidates),
        )
        .add("/api/apps/{app_id}/runs", get(list).post(create))
        .add("/api/apps/{app_id}/run-history", get(history))
        .add("/api/runs/{run_id}", get(detail))
        .add("/api/runs/{run_id}/comparison", get(comparison))
        .add("/api/runs/{run_id}/cancel", post(cancel))
        .add(
            "/api/runs/{run_id}/artifacts/{artifact_id}/content",
            get(artifact),
        )
}
