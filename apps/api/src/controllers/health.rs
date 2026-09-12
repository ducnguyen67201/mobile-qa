//! Real foundation endpoints: liveness plus structured API fallback errors.
//! Tests in apps/api/tests/health.rs compare registered behavior with Rust's OpenAPI
//! declaration. App setup routes live in the setup module.

use axum::{http::StatusCode, Json};
use loco_rs::prelude::*;
use mobile_qa_contracts::browser::{ApiError, HealthResponse, HealthStatus, HEALTH_PATH};

// Keep liveness independent of external services; database readiness is tested separately.
#[debug_handler]
async fn current() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: HealthStatus::Ok,
        service: "mobile-qa".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}

// Reserve API misses for JSON errors so they cannot be mistaken for a successful SPA page.
async fn unknown() -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::NOT_FOUND,
        Json(ApiError {
            code: "not_found".into(),
            message: "API route not found".into(),
            details: None,
            request_id: uuid::Uuid::new_v4(),
        }),
    )
}

// Axum's default 405 has no matching JSON envelope; use the declared error shape.
async fn method_not_allowed() -> (StatusCode, Json<ApiError>) {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        Json(ApiError {
            code: "method_not_allowed".into(),
            message: "HTTP method not allowed".into(),
            details: None,
            request_id: uuid::Uuid::new_v4(),
        }),
    )
}

// Register /api only once: Loco normalizes /api/ to the same registration path.
// Registering both previously caused a duplicate-route panic.
pub fn routes() -> Routes {
    Routes::new()
        .add(HEALTH_PATH, get(current).fallback(method_not_allowed))
        .add("/api", axum::routing::any(unknown))
        .add("/api/{*path}", axum::routing::any(unknown))
}
