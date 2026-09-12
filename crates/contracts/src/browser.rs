//! Browser HTTP contract exported to OpenAPI, then to TypeScript SDK/Zod code.
//!
//! The only declared operation today is GET /api/health. Route implementations live
//! in apps/api; integration tests keep them aligned with this independent declaration.
//! These transport shapes are not SeaORM entities or an authentication policy.
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

// Shared with route registration; changing it also requires the declaration below.
pub const HEALTH_PATH: &str = "/api/health";

#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
}

// Application liveness only: this does not prove database or device readiness.
#[derive(Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub service: String,
    pub version: String,
}

// Public error envelope. The browser validates it before trusting code/message.
// Keep credentials, raw upstream bodies and internal diagnostics out of these fields.
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
// Metadata anchor for Utoipa; this function is never registered as the HTTP handler.
#[allow(dead_code)]
fn health_endpoint() {}

#[derive(OpenApi)]
#[openapi(
    paths(health_endpoint),
    components(schemas(HealthResponse, HealthStatus, ApiError)),
    info(title = "Mobile QA browser API", version = "0.1.0")
)]
pub struct BrowserApi;

/// Build the exportable document without starting Loco or connecting to PostgreSQL.
pub fn openapi() -> utoipa::openapi::OpenApi {
    BrowserApi::openapi()
}
