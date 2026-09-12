//! Browser transport source. API route tests bind these pure declarations to Loco handlers.
//! ORM/storage credentials never belong in public transport shapes.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

pub const HEALTH_PATH: &str = "/api/health";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MembershipRole {
    Operator,
    Member,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SecretKind {
    Account,
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    Backend,
    Account,
    Reset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    NotChecked,
    OperatorReportedOk,
    OperatorReportedBlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UploadState {
    Pending,
    Receiving,
    Uploaded,
    Finalized,
    Expired,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ValidationState {
    Validating,
    Validated,
    Invalid,
    Unsupported,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HealthResponse {
    pub status: HealthStatus,
    pub service: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub request_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LoginRequest {
    #[schema(min_length = 1, max_length = 16384)]
    pub credential: String,
    pub challenge_id: Uuid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GoogleLoginChallenge {
    pub challenge_id: Uuid,
    pub client_id: String,
    pub nonce: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UserIdentity {
    pub id: Uuid,
    pub email: String,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationMembership {
    pub organization_id: Uuid,
    pub name: String,
    pub role: MembershipRole,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SessionResponse {
    pub user: UserIdentity,
    pub memberships: Vec<OrganizationMembership>,
    pub csrf_token: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LogoutResponse {
    pub signed_out: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateAppRequest {
    pub organization_id: Uuid,
    pub name: String,
    pub android_package: String,
    pub environment_name: String,
    pub backend_origins: Vec<String>,
    pub login_origins: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SecretReferenceSummary {
    pub id: Uuid,
    pub label: String,
    pub kind: SecretKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentCheck {
    pub kind: CheckKind,
    pub state: CheckState,
    pub note: Option<String>,
    pub checked_by: Option<Uuid>,
    pub checked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentResponse {
    pub id: Uuid,
    pub name: String,
    pub backend_origins: Vec<String>,
    pub login_origins: Vec<String>,
    pub revision: i32,
    pub account_secret_reference_id: Option<Uuid>,
    pub reset_secret_reference_id: Option<Uuid>,
    pub secret_references: Vec<SecretReferenceSummary>,
    pub checks: Vec<EnvironmentCheck>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UpdateEnvironmentRequest {
    pub expected_revision: i32,
    pub name: String,
    pub backend_origins: Vec<String>,
    pub login_origins: Vec<String>,
    pub account_secret_reference_id: Option<Uuid>,
    pub reset_secret_reference_id: Option<Uuid>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadinessResponse {
    pub execution_ready: bool,
    pub install: String,
    pub device_profile: Option<String>,
    pub backend: CheckState,
    pub account: CheckState,
    pub reset: CheckState,
    pub cases: String,
    pub account_configured: bool,
    pub reset_configured: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AppResponse {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub android_package: String,
    pub environment: EnvironmentResponse,
    pub readiness: ReadinessResponse,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AppSummary {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub android_package: String,
    pub environment_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AppListResponse {
    pub items: Vec<AppSummary>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateBuildUploadRequest {
    pub original_filename: String,
    // APK bytes are bounded to 250 MiB; keep browser JSON numbers, not int64/BigInt.
    #[schema(value_type = u32, minimum = 1, maximum = 262144000)]
    pub expected_size: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UploadResponse {
    pub id: Uuid,
    pub app_id: Uuid,
    pub original_filename: String,
    // APK bytes are bounded to 250 MiB; keep browser JSON numbers, not int64/BigInt.
    #[schema(value_type = u32, minimum = 1, maximum = 262144000)]
    pub expected_size: i64,
    #[schema(value_type = Option<u32>, minimum = 1, maximum = 262144000)]
    pub actual_size: Option<i64>,
    pub state: UploadState,
    pub expires_at: DateTime<Utc>,
    pub build_id: Option<Uuid>,
    pub retry_after_seconds: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ApkMetadata {
    pub package_name: String,
    pub version_name: Option<String>,
    pub version_code: String,
    pub min_sdk: u32,
    pub target_sdk: Option<u32>,
    pub native_abis: Vec<String>,
    pub signature_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildValidation {
    pub state: ValidationState,
    pub reason_code: Option<String>,
    pub message: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub validator_version: String,
    pub intake_policy_version: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildResponse {
    pub upload_id: Uuid,
    pub id: Uuid,
    pub app_id: Uuid,
    pub original_filename: String,
    #[schema(value_type = u32, minimum = 1, maximum = 262144000)]
    pub byte_size: i64,
    pub sha256: String,
    pub created_at: DateTime<Utc>,
    pub validation: BuildValidation,
    pub metadata: Option<ApkMetadata>,
    pub readiness: ReadinessResponse,
    pub can_retry_validation: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildListResponse {
    pub items: Vec<BuildResponse>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SettingsResponse {
    pub memberships: Vec<OrganizationMembership>,
    #[schema(value_type = u32, minimum = 1, maximum = 262144000)]
    pub max_apk_bytes: i64,
    pub upload_ttl_seconds: u32,
    pub session_ttl_seconds: u32,
    pub max_active_uploads: u32,
    pub storage: String,
    pub accepted_build_retention: String,
}

// Only the SDK serializes this transport declaration; Axum consumes streaming Multipart.
#[derive(ToSchema)]
pub struct UploadContentRequest {
    #[schema(value_type = String, format = Binary)]
    pub file: String,
}

#[utoipa::path(get, path = "/api/health", operation_id = "getHealth",
responses((status = 200, description = "Success", body = HealthResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_0() {}

#[utoipa::path(post, path = "/api/auth/google/challenge", operation_id = "startGoogleSignIn",
params(("X-Mobile-QA-Request" = String, Header)),
responses((status = 200, description = "Success", body = GoogleLoginChallenge),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_15() {}

#[utoipa::path(post, path = "/api/auth/google/login", operation_id = "login",
params(("X-Mobile-QA-Request" = String, Header)),
request_body(content = LoginRequest, content_type = "application/json"),
responses((status = 200, description = "Success", body = SessionResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_1() {}

#[utoipa::path(get, path = "/api/auth/session", operation_id = "getSession",
security(("session_cookie" = [])),
responses((status = 200, description = "Success", body = SessionResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_2() {}

#[utoipa::path(post, path = "/api/auth/logout", operation_id = "logout",
params(("X-CSRF-Token" = String, Header)),
security(("session_cookie" = [])),
responses((status = 200, description = "Success", body = LogoutResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_3() {}

#[utoipa::path(get, path = "/api/apps", operation_id = "listApps",
params(("cursor" = Option<String>, Query), ("limit" = Option<u32>, Query), ("organization_id" = Option<Uuid>, Query)),
security(("session_cookie" = [])),
responses((status = 200, description = "Success", body = AppListResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_4() {}

#[utoipa::path(post, path = "/api/apps", operation_id = "createApp",
params(("X-CSRF-Token" = String, Header)),
request_body(content = CreateAppRequest, content_type = "application/json"),
security(("session_cookie" = [])),
responses((status = 201, description = "Success", body = AppResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_5() {}

#[utoipa::path(get, path = "/api/apps/{app_id}", operation_id = "getApp",
params(("app_id" = Uuid, Path)),
security(("session_cookie" = [])),
responses((status = 200, description = "Success", body = AppResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_6() {}

#[utoipa::path(patch, path = "/api/apps/{app_id}/environment", operation_id = "updateEnvironment",
params(("app_id" = Uuid, Path), ("X-CSRF-Token" = String, Header)),
request_body(content = UpdateEnvironmentRequest, content_type = "application/json"),
security(("session_cookie" = [])),
responses((status = 200, description = "Success", body = EnvironmentResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_7() {}

#[utoipa::path(post, path = "/api/apps/{app_id}/build-uploads", operation_id = "createBuildUpload",
params(("app_id" = Uuid, Path), ("X-CSRF-Token" = String, Header)),
request_body(content = CreateBuildUploadRequest, content_type = "application/json"),
security(("session_cookie" = [])),
responses((status = 201, description = "Success", body = UploadResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_8() {}

#[utoipa::path(get, path = "/api/apps/{app_id}/build-uploads/{upload_id}", operation_id = "getBuildUpload",
params(("app_id" = Uuid, Path), ("upload_id" = Uuid, Path)),
security(("session_cookie" = [])),
responses((status = 200, description = "Success", body = UploadResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_9() {}

#[utoipa::path(put, path = "/api/apps/{app_id}/build-uploads/{upload_id}/content", operation_id = "uploadBuildContent",
params(("app_id" = Uuid, Path), ("upload_id" = Uuid, Path), ("X-CSRF-Token" = String, Header)),
request_body(content = UploadContentRequest, content_type = "multipart/form-data"),
security(("session_cookie" = [])),
responses((status = 200, description = "Success", body = UploadResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_10() {}

#[utoipa::path(post, path = "/api/apps/{app_id}/build-uploads/{upload_id}/complete", operation_id = "completeBuildUpload",
params(("app_id" = Uuid, Path), ("upload_id" = Uuid, Path), ("X-CSRF-Token" = String, Header)),
security(("session_cookie" = [])),
responses((status = 200, description = "Success", body = BuildResponse),
(status = 202, description = "Validation in progress", body = BuildResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_11() {}

#[utoipa::path(get, path = "/api/apps/{app_id}/builds", operation_id = "listBuilds",
params(("app_id" = Uuid, Path), ("cursor" = Option<String>, Query), ("limit" = Option<u32>, Query)),
security(("session_cookie" = [])),
responses((status = 200, description = "Success", body = BuildListResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_12() {}

#[utoipa::path(get, path = "/api/apps/{app_id}/builds/{build_id}", operation_id = "getBuild",
params(("app_id" = Uuid, Path), ("build_id" = Uuid, Path)),
security(("session_cookie" = [])),
responses((status = 200, description = "Success", body = BuildResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_13() {}

#[utoipa::path(get, path = "/api/settings", operation_id = "getSettings",
security(("session_cookie" = [])),
responses((status = 200, description = "Success", body = SettingsResponse),
(status = 400, description = "API error", body = ApiError),
(status = 401, description = "API error", body = ApiError),
(status = 403, description = "API error", body = ApiError),
(status = 404, description = "API error", body = ApiError),
(status = 408, description = "API error", body = ApiError),
(status = 409, description = "API error", body = ApiError),
(status = 410, description = "API error", body = ApiError),
(status = 413, description = "API error", body = ApiError),
(status = 415, description = "API error", body = ApiError),
(status = 422, description = "API error", body = ApiError),
(status = 429, description = "API error", body = ApiError),
(status = 500, description = "API error", body = ApiError),
(status = 503, description = "API error", body = ApiError),
(status = "default", description = "API error", body = ApiError)))]
#[allow(dead_code)]
fn endpoint_14() {}

#[derive(OpenApi)]
#[openapi(
    paths(
        endpoint_0,
        endpoint_1,
        endpoint_2,
        endpoint_3,
        endpoint_4,
        endpoint_5,
        endpoint_6,
        endpoint_7,
        endpoint_8,
        endpoint_9,
        endpoint_10,
        endpoint_11,
        endpoint_12,
        endpoint_13,
        endpoint_14,
        endpoint_15
    ),
    components(schemas(
        HealthStatus,
        MembershipRole,
        SecretKind,
        CheckKind,
        CheckState,
        UploadState,
        ValidationState,
        HealthResponse,
        ApiError,
        LoginRequest,
        GoogleLoginChallenge,
        UserIdentity,
        OrganizationMembership,
        SessionResponse,
        LogoutResponse,
        CreateAppRequest,
        SecretReferenceSummary,
        EnvironmentCheck,
        EnvironmentResponse,
        UpdateEnvironmentRequest,
        ReadinessResponse,
        AppResponse,
        AppSummary,
        AppListResponse,
        CreateBuildUploadRequest,
        UploadResponse,
        ApkMetadata,
        BuildValidation,
        BuildResponse,
        BuildListResponse,
        SettingsResponse,
        UploadContentRequest
    )),
    info(title = "Mobile QA browser API", version = "0.2.0")
)]
pub struct BrowserApi;

pub fn openapi() -> utoipa::openapi::OpenApi {
    let mut api = BrowserApi::openapi();
    if let Some(components) = api.components.as_mut() {
        components.add_security_scheme(
            "session_cookie",
            utoipa::openapi::security::SecurityScheme::ApiKey(
                utoipa::openapi::security::ApiKey::Cookie(
                    utoipa::openapi::security::ApiKeyValue::new("mobile_qa_session"),
                ),
            ),
        );
    }
    api
}

/// Route agreement inventory, shared with integration tests.
pub const OPERATIONS: &[(&str, &str, &str, u16)] = &[
    ("get", "/api/health", "getHealth", 200),
    ("post", "/api/auth/google/login", "login", 200),
    (
        "post",
        "/api/auth/google/challenge",
        "startGoogleSignIn",
        200,
    ),
    ("get", "/api/auth/session", "getSession", 200),
    ("post", "/api/auth/logout", "logout", 200),
    ("get", "/api/apps", "listApps", 200),
    ("post", "/api/apps", "createApp", 201),
    ("get", "/api/apps/{app_id}", "getApp", 200),
    (
        "patch",
        "/api/apps/{app_id}/environment",
        "updateEnvironment",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/build-uploads",
        "createBuildUpload",
        201,
    ),
    (
        "get",
        "/api/apps/{app_id}/build-uploads/{upload_id}",
        "getBuildUpload",
        200,
    ),
    (
        "put",
        "/api/apps/{app_id}/build-uploads/{upload_id}/content",
        "uploadBuildContent",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/build-uploads/{upload_id}/complete",
        "completeBuildUpload",
        200,
    ),
    ("get", "/api/apps/{app_id}/builds", "listBuilds", 200),
    (
        "get",
        "/api/apps/{app_id}/builds/{build_id}",
        "getBuild",
        200,
    ),
    ("get", "/api/settings", "getSettings", 200),
];
