//! Saved-case execution and unified history use the canonical transport export.
use crate::{browser::ApiError, execution::RunResponse, regression::*};
use utoipa::OpenApi;
#[utoipa::path(post,path="/api/apps/{app_id}/case-runs/preview",operation_id="previewCaseRun",security(("session_cookie"=[])),params(("app_id"=Uuid,Path)),request_body=CaseRunRequest,responses((status=200,description="Success",body=CaseRunPreview),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code, non_snake_case)]
fn previewCaseRun() {}
#[utoipa::path(post,path="/api/apps/{app_id}/case-runs",operation_id="createCaseRun",security(("session_cookie"=[])),params(("app_id"=Uuid,Path), ("Idempotency-Key"=String,Header)),request_body=CaseRunRequest,responses((status=201,description="Success",body=RunResponse),(status=200,description="Idempotent replay",body=RunResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code, non_snake_case)]
fn createCaseRun() {}
#[utoipa::path(post,path="/api/apps/{app_id}/suite-runs/preview",operation_id="previewSuiteRun",security(("session_cookie"=[])),params(("app_id"=Uuid,Path)),request_body=SuiteRunRequest,responses((status=200,description="Success",body=SuiteRunPreview),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code, non_snake_case)]
fn previewSuiteRun() {}
#[utoipa::path(post,path="/api/apps/{app_id}/suite-runs",operation_id="createSuiteRun",security(("session_cookie"=[])),params(("app_id"=Uuid,Path), ("Idempotency-Key"=String,Header),("X-Commercial-Quote-Id"=Option<Uuid>,Header)),request_body=SuiteRunRequest,responses((status=201,description="Success",body=RunResponse),(status=200,description="Idempotent replay",body=RunResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code, non_snake_case)]
fn createSuiteRun() {}
#[utoipa::path(get,path="/api/apps/{app_id}/run-history",operation_id="getRunHistory",security(("session_cookie"=[])),params(("app_id"=Uuid,Path), ("source"=Option<String>,Query),("cursor"=Option<String>,Query)),responses((status=200,description="Success",body=RunHistory),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code, non_snake_case)]
fn getRunHistory() {}
#[derive(OpenApi)]
#[openapi(
    paths(
        previewCaseRun,
        createCaseRun,
        previewSuiteRun,
        createSuiteRun,
        getRunHistory
    ),
    components(schemas(
        ObservationKind,
        RunSource,
        CommandPurpose,
        ComparisonKind,
        CaseComparison,
        RunComparison,
        CaseRunRequest,
        BaselineChoice,
        CaseRunPreview,
        SuiteRunRequest,
        SuiteRunPreview,
        RunHistoryItem,
        RunHistory
    ))
)]
pub struct RegressionApi;
pub const OPERATIONS: &[(&str, &str, &str, u16)] = &[
    (
        "post",
        "/api/apps/{app_id}/case-runs/preview",
        "previewCaseRun",
        200,
    ),
    ("post", "/api/apps/{app_id}/case-runs", "createCaseRun", 201),
    (
        "post",
        "/api/apps/{app_id}/suite-runs/preview",
        "previewSuiteRun",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/suite-runs",
        "createSuiteRun",
        201,
    ),
    (
        "get",
        "/api/apps/{app_id}/run-history",
        "getRunHistory",
        200,
    ),
];
