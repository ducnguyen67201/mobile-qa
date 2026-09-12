//! Browser endpoint declaration and transport schemas. No Loco, Axum or DB dependency.
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

pub const HEALTH_PATH: &str = "/api/health";

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub service: String,
    pub version: String,
}

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

// Pure declaration: apps/api/tests/health.rs verifies path, method, input and
// success/error schema agreement against the registered, typed Loco handlers.
#[utoipa::path(
    get,
    path = "/api/health",
    operation_id = "getHealth",
    responses(
        (status = 200, description = "Application is alive", body = HealthResponse),
        (status = "default", description = "API error", body = ApiError)
    )
)]
#[allow(dead_code)]
fn health_endpoint() {}

#[derive(OpenApi)]
#[openapi(
    paths(health_endpoint),
    components(schemas(HealthResponse, HealthStatus, ApiError)),
    info(title = "Mobile QA browser API", version = "0.1.0")
)]
pub struct BrowserApi;

pub fn openapi() -> utoipa::openapi::OpenApi {
    BrowserApi::openapi()
}
