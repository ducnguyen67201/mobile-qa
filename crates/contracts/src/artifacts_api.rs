//! Temporary storage capabilities are explicit transport exceptions: they are never build locators.
use crate::browser::{ApiError, UploadResponse};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::{OpenApi, ToSchema};
use uuid::Uuid;

/// JSON integers stay browser numbers without an int32 cap or int64 BigInt mapping.
pub fn apk_byte_schema() -> utoipa::openapi::schema::Object {
    utoipa::openapi::schema::ObjectBuilder::new()
        .schema_type(utoipa::openapi::schema::Type::Integer)
        .minimum(Some(1))
        .maximum(Some(2147483648_i64))
        .build()
}
pub fn optional_apk_byte_schema() -> utoipa::openapi::schema::Object {
    utoipa::openapi::schema::ObjectBuilder::new()
        .schema_type(utoipa::openapi::schema::SchemaType::Array(vec![
            utoipa::openapi::schema::Type::Integer,
            utoipa::openapi::schema::Type::Null,
        ]))
        .minimum(Some(1))
        .maximum(Some(2147483648_i64))
        .build()
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MultipartPolicy {
    pub part_size: u32,
    pub max_parallel_parts: u8,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MultipartState {
    Uploading,
    Completing,
    Sealed,
    Aborted,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UploadedPart {
    pub part_number: u16,
    pub byte_size: u32,
    pub sha256: String,
    pub etag: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MultipartUpload {
    pub upload_id: Uuid,
    pub part_size: u32,
    pub max_parallel_parts: u8,
    pub expires_at: DateTime<Utc>,
    pub state: MultipartState,
    pub parts: Vec<UploadedPart>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthorizeUploadPartRequest {
    #[schema(minimum = 1, maximum = 128)]
    pub part_number: u16,
    #[schema(minimum = 1, maximum = 16777216)]
    pub byte_size: u32,
    #[schema(pattern = "^[a-f0-9]{64}$")]
    pub sha256: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UploadPartAuthorization {
    pub part_number: u16,
    pub url: String,
    pub expires_at: DateTime<Utc>,
    pub headers: BTreeMap<String, String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfirmUploadPartRequest {
    pub etag: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryAuthentication {
    SignedUrl,
    WorkerLease,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BuildDelivery {
    pub url: String,
    pub expires_at: DateTime<Utc>,
    #[schema(schema_with = crate::artifacts_api::apk_byte_schema)]
    #[schemars(range(min = 1, max = 2147483648_u32))]
    pub byte_size: u32,
    pub sha256: String,
    pub app_id: Uuid,
    pub headers: BTreeMap<String, String>,
    pub authentication: DeliveryAuthentication,
}

#[utoipa::path(post,path="/api/apps/{app_id}/build-uploads/{upload_id}/multipart",operation_id="startMultipartUpload",security(("session_cookie"=[])),params(("app_id"=Uuid,Path),("upload_id"=Uuid,Path)),responses((status=200,description="Multipart session",body=MultipartUpload),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn start() {}
#[utoipa::path(get,path="/api/apps/{app_id}/build-uploads/{upload_id}/multipart",operation_id="getMultipartUpload",security(("session_cookie"=[])),params(("app_id"=Uuid,Path),("upload_id"=Uuid,Path)),responses((status=200,description="Multipart session",body=MultipartUpload),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn get() {}
#[utoipa::path(post,path="/api/apps/{app_id}/build-uploads/{upload_id}/multipart/parts",operation_id="authorizeUploadPart",security(("session_cookie"=[])),params(("app_id"=Uuid,Path),("upload_id"=Uuid,Path)),request_body=AuthorizeUploadPartRequest,responses((status=200,description="Temporary capability for one immutable part",body=UploadPartAuthorization),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn authorize() {}
#[utoipa::path(put,path="/api/apps/{app_id}/build-uploads/{upload_id}/multipart/parts/{part_number}",operation_id="confirmUploadPart",security(("session_cookie"=[])),params(("app_id"=Uuid,Path),("upload_id"=Uuid,Path),("part_number"=u16,Path)),request_body=ConfirmUploadPartRequest,responses((status=200,description="Part receipt persisted",body=MultipartUpload),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn confirm() {}
#[utoipa::path(post,path="/api/apps/{app_id}/build-uploads/{upload_id}/multipart/complete",operation_id="completeMultipartUpload",security(("session_cookie"=[])),params(("app_id"=Uuid,Path),("upload_id"=Uuid,Path)),responses((status=202,description="Durable sealing queued",body=UploadResponse),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn complete() {}
#[utoipa::path(delete,path="/api/apps/{app_id}/build-uploads/{upload_id}/multipart",operation_id="abortMultipartUpload",security(("session_cookie"=[])),params(("app_id"=Uuid,Path),("upload_id"=Uuid,Path)),responses((status=200,description="Multipart session aborted",body=MultipartUpload),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn abort() {}
#[utoipa::path(get,path="/api/worker/attempts/{attempt_id}/build/delivery",operation_id="getAttemptBuildDelivery",params(("attempt_id"=Uuid,Path),("generation"=i32,Query),("X-Lease-Token"=String,Header)),responses((status=200,description="Lease-scoped build capability",body=BuildDelivery),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn attempt_delivery() {}
#[utoipa::path(get,path="/api/worker/phones/{session_id}/build/delivery",operation_id="getPhoneBuildDelivery",params(("session_id"=Uuid,Path),("X-Lease-Token"=String,Header)),responses((status=200,description="Lease-scoped build capability",body=BuildDelivery),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn phone_delivery() {}
#[derive(OpenApi)]
#[openapi(
    paths(
        start,
        get,
        authorize,
        confirm,
        complete,
        abort,
        attempt_delivery,
        phone_delivery
    ),
    components(schemas(
        MultipartPolicy,
        MultipartState,
        UploadedPart,
        MultipartUpload,
        AuthorizeUploadPartRequest,
        UploadPartAuthorization,
        ConfirmUploadPartRequest,
        BuildDelivery,
        DeliveryAuthentication
    ))
)]
pub struct ArtifactsApi;
pub const OPERATIONS: &[(&str, &str, &str, u16)] = &[
    (
        "post",
        "/api/apps/{app_id}/build-uploads/{upload_id}/multipart",
        "startMultipartUpload",
        200,
    ),
    (
        "get",
        "/api/apps/{app_id}/build-uploads/{upload_id}/multipart",
        "getMultipartUpload",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/build-uploads/{upload_id}/multipart/parts",
        "authorizeUploadPart",
        200,
    ),
    (
        "put",
        "/api/apps/{app_id}/build-uploads/{upload_id}/multipart/parts/{part_number}",
        "confirmUploadPart",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/build-uploads/{upload_id}/multipart/complete",
        "completeMultipartUpload",
        202,
    ),
    (
        "delete",
        "/api/apps/{app_id}/build-uploads/{upload_id}/multipart",
        "abortMultipartUpload",
        200,
    ),
    (
        "get",
        "/api/worker/attempts/{attempt_id}/build/delivery",
        "getAttemptBuildDelivery",
        200,
    ),
    (
        "get",
        "/api/worker/phones/{session_id}/build/delivery",
        "getPhoneBuildDelivery",
        200,
    ),
];
