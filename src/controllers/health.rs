use loco_rs::prelude::*;
use mobile_qa_contracts::browser::{HealthResponse, HealthStatus};
#[debug_handler]
async fn current() -> Result<Response> {
    format::json(HealthResponse {
        status: HealthStatus::Ok,
        service: "mobile-qa".into(),
        version: env!("CARGO_PKG_VERSION").into(),
    })
}
async fn unknown() -> axum::http::StatusCode {
    axum::http::StatusCode::NOT_FOUND
}
pub fn routes() -> Routes {
    Routes::new()
        .prefix("/api")
        .add("/health", get(current))
        .add("/", axum::routing::any(unknown))
        .add("/{*path}", axum::routing::any(unknown))
}
