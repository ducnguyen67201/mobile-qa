use axum::{http::StatusCode, Json};
use loco_rs::prelude::*;
use mobile_qa_contracts::browser::{ApiError, HealthResponse, HealthStatus, HEALTH_PATH};

#[debug_handler]
async fn current() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: HealthStatus::Ok,
        service: "mobile-qa".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

async fn unknown() -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            code: "not_found".into(),
            message: "API route not found".into(),
            details: None,
        }),
    )
}

async fn method_not_allowed() -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        Json(ApiError {
            code: "method_not_allowed".into(),
            message: "HTTP method not allowed".into(),
            details: None,
        }),
    )
}

pub fn routes() -> Routes {
    Routes::new()
        .add(HEALTH_PATH, get(current).fallback(method_not_allowed))
        .add("/api", axum::routing::any(unknown))
        .add("/api/{*path}", axum::routing::any(unknown))
}
