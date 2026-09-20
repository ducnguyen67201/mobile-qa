//! Cookie-session authoring routes. Session applies CSRF/origin checks to mutations;
//! services reauthorize app membership and optimistic revisions.
use crate::{
    errors::{ApiFailure, ApiResult},
    services::{
        auth::Session,
        test_library as library,
        test_library_mutations::{self, Mutation},
    },
};
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use loco_rs::{app::AppContext, controller::Routes};
use mobile_qa_contracts::test_library::*;
use uuid::Uuid;
async fn list(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Query(q): Query<LibraryListQuery>,
) -> ApiResult<Json<LibraryListResponse>> {
    library::authorize(&ctx, session.user.id, app).await?;
    Ok(Json(library::list(&ctx.db, session.user.id, app, q).await?))
}
async fn options(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
) -> ApiResult<Json<LibraryOptionsResponse>> {
    library::authorize(&ctx, session.user.id, app).await?;
    Ok(Json(library::options(&ctx.db, session.user.id, app).await?))
}
async fn entry(
    State(ctx): State<AppContext>,
    session: Session,
    Path((app, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<LibraryEntryResponse>> {
    library::authorize(&ctx, session.user.id, app).await?;
    Ok(Json(
        library::entry(&ctx.db, session.user.id, app, id).await?,
    ))
}
async fn draft(
    State(ctx): State<AppContext>,
    session: Session,
    Path((app, id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<LibraryDraftResponse>> {
    library::authorize(&ctx, session.user.id, app).await?;
    Ok(Json(
        library::draft(&ctx.db, session.user.id, app, id).await?,
    ))
}
async fn versions(
    State(ctx): State<AppContext>,
    session: Session,
    Path((app, id)): Path<(Uuid, Uuid)>,
    Query(q): Query<LibraryVersionQuery>,
) -> ApiResult<Json<LibraryVersionListResponse>> {
    library::authorize(&ctx, session.user.id, app).await?;
    Ok(Json(
        library::versions(&ctx.db, session.user.id, app, id, q.cursor).await?,
    ))
}
async fn version(
    State(ctx): State<AppContext>,
    session: Session,
    Path((app, id, v)): Path<(Uuid, Uuid, Uuid)>,
) -> ApiResult<Json<LibraryVersionResponse>> {
    library::authorize(&ctx, session.user.id, app).await?;
    Ok(Json(
        library::version(&ctx.db, session.user.id, app, id, v).await?,
    ))
}
async fn default_plan(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
) -> ApiResult<Json<DefaultPlanResponse>> {
    library::authorize(&ctx, session.user.id, app).await?;
    Ok(Json(library::default_plan(&ctx.db, app).await?))
}
async fn create(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Json(r): Json<CreateLibraryEntryRequest>,
) -> ApiResult<(StatusCode, Json<LibraryDraftResponse>)> {
    let (LibraryMutationReceipt::Draft(response), new) =
        test_library_mutations::apply(&ctx, session.user.id, app, Mutation::Create(r)).await?
    else {
        return Err(ApiFailure::internal());
    };
    Ok((
        if new {
            StatusCode::CREATED
        } else {
            StatusCode::OK
        },
        Json(*response),
    ))
}
async fn save(
    State(ctx): State<AppContext>,
    session: Session,
    Path((app, id)): Path<(Uuid, Uuid)>,
    Json(r): Json<SaveLibraryDraftRequest>,
) -> ApiResult<Json<LibraryDraftResponse>> {
    let (LibraryMutationReceipt::Draft(response), _) =
        test_library_mutations::apply(&ctx, session.user.id, app, Mutation::Save(id, r)).await?
    else {
        return Err(ApiFailure::internal());
    };
    Ok(Json(*response))
}
async fn archive(
    State(ctx): State<AppContext>,
    session: Session,
    Path((app, id)): Path<(Uuid, Uuid)>,
    Json(r): Json<ArchiveLibraryEntryRequest>,
) -> ApiResult<Json<LibraryEntryResponse>> {
    let (LibraryMutationReceipt::Entry(response), _) =
        test_library_mutations::apply(&ctx, session.user.id, app, Mutation::Archive(id, r)).await?
    else {
        return Err(ApiFailure::internal());
    };
    Ok(Json(response))
}
async fn set_default(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Json(r): Json<SetDefaultPlanRequest>,
) -> ApiResult<Json<DefaultPlanResponse>> {
    let (LibraryMutationReceipt::Default(response), _) =
        test_library_mutations::apply(&ctx, session.user.id, app, Mutation::Default(r)).await?
    else {
        return Err(ApiFailure::internal());
    };
    Ok(Json(response))
}
pub fn routes() -> Routes {
    use axum::routing::{get, post};
    Routes::new()
        .add("/api/apps/{app_id}/test-library", get(list).post(create))
        .add("/api/apps/{app_id}/test-library/options", get(options))
        .add("/api/apps/{app_id}/test-library/{entry_id}", get(entry))
        .add(
            "/api/apps/{app_id}/test-library/{entry_id}/draft",
            get(draft).put(save),
        )
        .add(
            "/api/apps/{app_id}/test-library/{entry_id}/versions",
            get(versions),
        )
        .add(
            "/api/apps/{app_id}/test-library/{entry_id}/versions/{version_id}",
            get(version),
        )
        .add(
            "/api/apps/{app_id}/test-library/{entry_id}/archive",
            post(archive),
        )
        .add(
            "/api/apps/{app_id}/default-test-plan",
            get(default_plan).put(set_default),
        )
}
