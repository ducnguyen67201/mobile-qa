use serde::{Deserialize, Serialize};
use ts_rs::TS;
#[derive(Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
}
#[derive(Debug, PartialEq, Serialize, Deserialize, TS)]
#[serde(deny_unknown_fields)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub service: String,
    pub version: String,
}
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    #[ts(type = "unknown | null")]
    pub details: Option<serde_json::Value>,
}
