//! Authenticated browser execution routes. Binary evidence remains private.
use crate::{
    errors::{ApiFailure, ApiResult},
    services::{apps, auth::Session, execution_store::*, run_artifacts, runs},
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
async fn case_preview(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Json(input): Json<mobile_qa_contracts::regression::CaseRunRequest>,
) -> ApiResult<Json<mobile_qa_contracts::regression::CaseRunPreview>> {
    Ok(Json(
        crate::services::case_runs::preview(&ctx, session.user.id, app, input).await?,
    ))
}
async fn case_create(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<mobile_qa_contracts::regression::CaseRunRequest>,
) -> ApiResult<(StatusCode, Json<RunResponse>)> {
    let key = headers
        .get("idempotency-key")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiFailure::invalid("Idempotency-Key is required"))?;
    let (run, new) =
        crate::services::case_runs::create(&ctx, session.user.id, app, key, input).await?;
    Ok((
        if new {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(run),
    ))
}
async fn suite_preview(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Json(input): Json<mobile_qa_contracts::regression::SuiteRunRequest>,
) -> ApiResult<Json<mobile_qa_contracts::regression::SuiteRunPreview>> {
    Ok(Json(
        crate::services::suite_runs::preview(&ctx, session.user.id, app, input).await?,
    ))
}
async fn suite_create(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    headers: HeaderMap,
    Json(input): Json<mobile_qa_contracts::regression::SuiteRunRequest>,
) -> ApiResult<(StatusCode, Json<RunResponse>)> {
    let key = headers
        .get("idempotency-key")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiFailure::invalid("Idempotency-Key is required"))?;
    let (run, new) =
        crate::services::suite_runs::create(&ctx, session.user.id, app, key, input).await?;
    Ok((
        if new {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(run),
    ))
}
#[derive(Deserialize)]
struct HistoryQuery {
    source: Option<String>,
    cursor: Option<String>,
}
async fn history(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Query(q): Query<HistoryQuery>,
) -> ApiResult<Json<mobile_qa_contracts::regression::RunHistory>> {
    Ok(Json(
        crate::services::run_history::list(&ctx, session.user.id, app, q.source, q.cursor).await?,
    ))
}
pub fn routes() -> Routes {
    use axum::routing::{get, post};
    Routes::new()
        .add("/api/apps/{app_id}/case-runs/preview", post(case_preview))
        .add("/api/apps/{app_id}/case-runs", post(case_create))
        .add("/api/apps/{app_id}/suite-runs/preview", post(suite_preview))
        .add("/api/apps/{app_id}/suite-runs", post(suite_create))
        .add("/api/apps/{app_id}/run-history", get(history))
        .add("/api/apps/{app_id}/execution-plan", get(preview))
        .add("/api/apps/{app_id}/runs", get(list).post(create))
        .add("/api/runs/{run_id}", get(detail))
        .add("/api/runs/{run_id}/cancel", post(cancel))
        .add(
            "/api/runs/{run_id}/artifacts/{artifact_id}/content",
            get(artifact),
        )
}
