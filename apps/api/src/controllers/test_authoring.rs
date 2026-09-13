//! Typed direct authoring endpoints, sharing existing app and session authorization.
use crate::{
    errors::{ApiFailure, ApiResult},
    services::{
        auth::Session,
        test_authoring as service,
        test_library_mutations::{self, Mutation},
    },
};
use axum::{
    extract::{Path, State},
    Json,
};
use loco_rs::{app::AppContext, controller::Routes};
use mobile_qa_contracts::{automation::*, task_sessions::*, test_library::LibraryMutationReceipt};
use uuid::Uuid;
async fn command(
    State(ctx): State<AppContext>,
    s: Session,
    Path(id): Path<Uuid>,
    Json(r): Json<PhoneCommandRequest>,
) -> ApiResult<Json<PhoneSession>> {
    Ok(Json(service::command(&ctx, s.user.id, id, r).await?))
}
async fn templates(
    State(ctx): State<AppContext>,
    s: Session,
    Path(app): Path<Uuid>,
) -> ApiResult<Json<TestTemplates>> {
    Ok(Json(service::templates(&ctx, s.user.id, app).await?))
}
async fn generate(
    State(ctx): State<AppContext>,
    s: Session,
    Path(app): Path<Uuid>,
    Json(r): Json<GenerateTestsRequest>,
) -> ApiResult<Json<PhoneSession>> {
    Ok(Json(service::generate(&ctx, s.user.id, app, r).await?))
}
async fn detail(
    State(ctx): State<AppContext>,
    s: Session,
    Path((app, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<PhoneTask>> {
    Ok(Json(service::job_detail(&ctx, s.user.id, app, id).await?))
}
async fn cancel(
    State(ctx): State<AppContext>,
    s: Session,
    Path((app, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<PhoneSession>> {
    Ok(Json(service::cancel(&ctx, s.user.id, app, id).await?))
}
async fn save(
    State(ctx): State<AppContext>,
    s: Session,
    Path(app): Path<Uuid>,
    Json(r): Json<SaveAuthoredTestsRequest>,
) -> ApiResult<Json<SavedAuthoredTests>> {
    let (LibraryMutationReceipt::Authored(result), _) =
        test_library_mutations::apply(&ctx, s.user.id, app, Mutation::Authored(r)).await?
    else {
        return Err(ApiFailure::internal());
    };
    Ok(Json(result))
}
pub fn routes() -> Routes {
    use axum::routing::{get, post};
    Routes::new()
        .add("/api/phones/{session_id}/commands", post(command))
        .add("/api/apps/{app_id}/test-templates", get(templates))
        .add("/api/apps/{app_id}/test-generations", post(generate))
        .add("/api/apps/{app_id}/test-generations/{job_id}", get(detail))
        .add(
            "/api/apps/{app_id}/test-generations/{job_id}/cancel",
            post(cancel),
        )
        .add("/api/apps/{app_id}/test-library/from-recording", post(save))
}
