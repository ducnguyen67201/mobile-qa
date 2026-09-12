//! Thin HTTP adapters: authorization precedes body intake and service transitions.
use crate::{
    config::{Setup, MAX_APK, SESSION_SECONDS, UPLOAD_SECONDS},
    errors::{ApiFailure, ApiResult},
    services::{
        apps,
        auth::{self, LoginGuard, Session},
        google, uploads,
    },
};
use axum::{
    extract::{DefaultBodyLimit, Multipart, Path, Query, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use loco_rs::{app::AppContext, controller::Routes};
use mobile_qa_contracts::browser::*;
use uuid::Uuid;

async fn google_challenge(
    State(ctx): State<AppContext>,
    guard: LoginGuard,
    jar: CookieJar,
) -> ApiResult<(CookieJar, Json<GoogleLoginChallenge>)> {
    let (response, binding) = google::challenge(&ctx, &guard.network).await?;
    let setup = Setup::get(&ctx);
    let cookie = Cookie::build((google::cookie_name(&setup), binding))
        .path("/")
        .http_only(true)
        .secure(setup.secure_cookie)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::minutes(10))
        .build();
    Ok((jar.add(cookie), Json(response)))
}
async fn login(
    State(ctx): State<AppContext>,
    guard: LoginGuard,
    jar: CookieJar,
    Json(input): Json<LoginRequest>,
) -> ApiResult<(CookieJar, Json<SessionResponse>)> {
    let binding = jar
        .get(google::cookie_name(&Setup::get(&ctx)))
        .map(|cookie| cookie.value())
        .unwrap_or("");
    let (session, token) = google::login(&ctx, input, binding, &guard.network).await?;
    // Rotate an existing session only after successful credentials; a failed login cannot log out a user.
    let mut parts = axum::http::Request::new(()).into_parts().0;
    if let Some(cookie) = jar.get(Setup::get(&ctx).cookie_name()) {
        if let Ok(header) = format!("{}={}", cookie.name(), cookie.value()).parse() {
            parts.headers.insert(axum::http::header::COOKIE, header);
            if let Ok(previous) = auth::authenticate(&ctx, &parts).await {
                auth::revoke(&ctx, previous.session.id).await?;
            }
        }
    }
    let setup = Setup::get(&ctx);
    let cookie = Cookie::build((setup.cookie_name(), token))
        .path("/")
        .http_only(true)
        .secure(setup.secure_cookie)
        .same_site(SameSite::Strict)
        .max_age(time::Duration::seconds(SESSION_SECONDS))
        .build();
    Ok((jar.add(cookie), Json(auth::response(&ctx, &session).await?)))
}
async fn session(
    State(ctx): State<AppContext>,
    session: Session,
) -> ApiResult<Json<SessionResponse>> {
    Ok(Json(auth::response(&ctx, &session).await?))
}
async fn logout(
    State(ctx): State<AppContext>,
    session: Session,
    jar: CookieJar,
) -> ApiResult<(CookieJar, Json<LogoutResponse>)> {
    auth::revoke(&ctx, session.session.id).await?;
    let setup = Setup::get(&ctx);
    let cookie = Cookie::build((setup.cookie_name(), String::new()))
        .path("/")
        .http_only(true)
        .secure(setup.secure_cookie)
        .same_site(SameSite::Strict)
        .build();
    Ok((
        jar.remove(cookie),
        Json(LogoutResponse { signed_out: true }),
    ))
}
async fn list_apps(
    State(ctx): State<AppContext>,
    session: Session,
    Query(query): Query<apps::ListQuery>,
) -> ApiResult<Json<AppListResponse>> {
    Ok(Json(apps::list(&ctx, session.user.id, query).await?))
}
async fn create_app(
    State(ctx): State<AppContext>,
    session: Session,
    Json(input): Json<CreateAppRequest>,
) -> ApiResult<(StatusCode, Json<AppResponse>)> {
    Ok((
        StatusCode::CREATED,
        Json(apps::create(&ctx, session.user.id, input).await?),
    ))
}
async fn get_app(
    State(ctx): State<AppContext>,
    session: Session,
    Path(id): Path<Uuid>,
) -> ApiResult<Json<AppResponse>> {
    let app = apps::authorized(&ctx, session.user.id, id).await?;
    Ok(Json(apps::detail(&ctx, &app).await?))
}
async fn update_environment(
    State(ctx): State<AppContext>,
    session: Session,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateEnvironmentRequest>,
) -> ApiResult<Json<EnvironmentResponse>> {
    let app = apps::authorized(&ctx, session.user.id, id).await?;
    Ok(Json(apps::update_environment(&ctx, &app, input).await?))
}
async fn create_upload(
    State(ctx): State<AppContext>,
    session: Session,
    Path(id): Path<Uuid>,
    Json(input): Json<CreateBuildUploadRequest>,
) -> ApiResult<(StatusCode, Json<UploadResponse>)> {
    let app = apps::authorized(&ctx, session.user.id, id).await?;
    Ok((
        StatusCode::CREATED,
        Json(uploads::create(&ctx, &app, session.user.id, input).await?),
    ))
}
async fn get_upload(
    State(ctx): State<AppContext>,
    session: Session,
    Path((id, upload_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<UploadResponse>> {
    let app = apps::authorized(&ctx, session.user.id, id).await?;
    Ok(Json(
        uploads::upload_response(&ctx, &uploads::upload(&ctx, &app, upload_id).await?).await?,
    ))
}
async fn upload_content(
    State(ctx): State<AppContext>,
    session: Session,
    Path((id, upload_id)): Path<(Uuid, Uuid)>,
    headers: HeaderMap,
    multipart: Multipart,
) -> ApiResult<Json<UploadResponse>> {
    let app = apps::authorized(&ctx, session.user.id, id).await?;
    if headers
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .is_some_and(|v| v > MAX_APK as u64 + 65536)
    {
        return Err(ApiFailure::new(
            413,
            "file_too_large",
            "APK exceeds the upload limit",
        ));
    }
    Ok(Json(
        uploads::transfer(&ctx, &app, &session, upload_id, multipart).await?,
    ))
}
async fn complete_upload(
    State(ctx): State<AppContext>,
    session: Session,
    Path((id, upload_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<(StatusCode, Json<BuildResponse>)> {
    let app = apps::authorized(&ctx, session.user.id, id).await?;
    let (status, body) = uploads::complete(&ctx, &app, upload_id).await?;
    Ok((
        StatusCode::from_u16(status).map_err(|_| ApiFailure::internal())?,
        Json(body),
    ))
}
async fn list_builds(
    State(ctx): State<AppContext>,
    session: Session,
    Path(id): Path<Uuid>,
    Query(query): Query<apps::ListQuery>,
) -> ApiResult<Json<BuildListResponse>> {
    let app = apps::authorized(&ctx, session.user.id, id).await?;
    Ok(Json(uploads::list(&ctx, &app, query).await?))
}
async fn get_build(
    State(ctx): State<AppContext>,
    session: Session,
    Path((id, build_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<BuildResponse>> {
    let app = apps::authorized(&ctx, session.user.id, id).await?;
    Ok(Json(
        uploads::build_response(&ctx, &app, &uploads::build(&ctx, &app, build_id).await?).await?,
    ))
}
async fn settings(
    State(ctx): State<AppContext>,
    session: Session,
) -> ApiResult<Json<SettingsResponse>> {
    Ok(Json(SettingsResponse{memberships:auth::organization_memberships(&ctx,session.user.id).await?,max_apk_bytes:MAX_APK,upload_ttl_seconds:UPLOAD_SECONDS as u32,session_ttl_seconds:SESSION_SECONDS as u32,max_active_uploads:4,storage:if Setup::get(&ctx).store.is_remote(){"Private object storage"}else{"Private local storage"}.into(),accepted_build_retention:"Accepted builds are retained until an explicit operator deletion; automatic retention is not configured".into()}))
}
pub fn routes() -> Routes {
    use axum::routing::{get, patch, post, put};
    Routes::new()
        .add("/api/auth/google/challenge", post(google_challenge))
        .add("/api/auth/google/login", post(login))
        .add("/api/auth/session", get(session))
        .add("/api/auth/logout", post(logout))
        .add("/api/apps", get(list_apps).post(create_app))
        .add("/api/apps/{app_id}", get(get_app))
        .add("/api/apps/{app_id}/environment", patch(update_environment))
        .add("/api/apps/{app_id}/build-uploads", post(create_upload))
        .add(
            "/api/apps/{app_id}/build-uploads/{upload_id}",
            get(get_upload),
        )
        .add(
            "/api/apps/{app_id}/build-uploads/{upload_id}/content",
            put(upload_content).layer(DefaultBodyLimit::max(MAX_APK as usize + 65536)),
        )
        .add(
            "/api/apps/{app_id}/build-uploads/{upload_id}/complete",
            post(complete_upload),
        )
        .add("/api/apps/{app_id}/builds", get(list_builds))
        .add("/api/apps/{app_id}/builds/{build_id}", get(get_build))
        .add("/api/settings", get(settings))
}
