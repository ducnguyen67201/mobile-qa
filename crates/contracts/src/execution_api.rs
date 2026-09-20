//! Browser execution OpenAPI merged into the canonical local export.
use crate::execution_lifecycle::*;
use crate::{browser::ApiError, execution::*};
use utoipa::OpenApi;
#[utoipa::path(get,path="/api/apps/{app_id}/execution-plan",operation_id="getExecutionPlan",security(("session_cookie"=[])),
params(("app_id" = Uuid, Path),crate::test_library::ExecutionPlanQuery),
responses((status=200,description="Success",body=PlanPreviewResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_0() {}
#[utoipa::path(post,path="/api/apps/{app_id}/runs",operation_id="createRun",security(("session_cookie"=[])),
params(("app_id" = Uuid, Path),("Idempotency-Key" = String, Header)),
request_body=CreateRunRequest,
responses((status=201,description="Success",body=RunResponse), (status=200,description="Idempotent replay",body=RunResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_1() {}
#[utoipa::path(get,path="/api/apps/{app_id}/runs",operation_id="listRuns",security(("session_cookie"=[])),
params(("app_id" = Uuid, Path),("cursor" = Option<Uuid>, Query)),
responses((status=200,description="Success",body=RunListResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_2() {}
#[utoipa::path(get,path="/api/runs/{run_id}",operation_id="getRun",security(("session_cookie"=[])),
params(("run_id" = Uuid, Path)),
responses((status=200,description="Success",body=RunResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_3() {}
#[utoipa::path(post,path="/api/runs/{run_id}/cancel",operation_id="cancelRun",security(("session_cookie"=[])),
params(("run_id" = Uuid, Path)),
responses((status=200,description="Success",body=RunResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_4() {}
#[utoipa::path(get,path="/api/runs/{run_id}/artifacts/{artifact_id}/content",operation_id="getRunArtifact",security(("session_cookie"=[])),
params(("run_id" = Uuid, Path),("artifact_id" = Uuid, Path)),
responses((status=200,description="Success",body=String,content_type="application/octet-stream"),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_5() {}
#[utoipa::path(get,path="/api/apps/{app_id}/saved-case-preview",operation_id="getSavedCasePreview",security(("session_cookie"=[])),
params(("app_id" = Uuid, Path),SavedCasePreviewQuery),
responses((status=200,description="Success",body=PlanPreviewResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=500,description="API error",body=ApiError)))]
#[allow(dead_code)]
fn saved_case_preview_endpoint() {}
#[utoipa::path(get,path="/api/apps/{app_id}/baseline-candidates",operation_id="listBaselineCandidates",security(("session_cookie"=[])),
params(("app_id" = Uuid, Path),BaselineCandidateQuery),
responses((status=200,description="Success",body=BaselineCandidateResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=500,description="API error",body=ApiError)))]
#[allow(dead_code)]
fn baseline_candidates_endpoint() {}
#[utoipa::path(get,path="/api/apps/{app_id}/run-history",operation_id="listRunHistory",security(("session_cookie"=[])),
params(("app_id" = Uuid, Path),RunHistoryQuery),
responses((status=200,description="Success",body=RunHistoryResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=500,description="API error",body=ApiError)))]
#[allow(dead_code)]
fn run_history_endpoint() {}
#[utoipa::path(get,path="/api/runs/{run_id}/comparison",operation_id="getRunComparison",security(("session_cookie"=[])),
params(("run_id" = Uuid, Path)),
responses((status=200,description="Success",body=RunComparisonResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=500,description="API error",body=ApiError)))]
#[allow(dead_code)]
fn run_comparison_endpoint() {}
#[utoipa::path(post,path="/api/worker/attempts/{attempt_id}/preflight",operation_id="acknowledgeExecutionStart",
params(("attempt_id" = Uuid, Path),("Authorization" = String, Header),("x-lease-token" = String, Header)),
request_body=PreflightRequest,
responses((status=200,description="Start evidence acknowledged",body=PreflightAcknowledgement),(status=401,description="Worker authentication required",body=ApiError),(status=404,description="Scoped attempt or evidence missing",body=ApiError),(status=409,description="Stale lease or conflicting receipt",body=ApiError),(status=422,description="Invalid start evidence",body=ApiError),(status=500,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn preflight_endpoint() {}
#[derive(OpenApi)]
#[openapi(
    paths(
        endpoint_0,
        endpoint_1,
        endpoint_2,
        endpoint_3,
        endpoint_4,
        endpoint_5,
        saved_case_preview_endpoint,
        baseline_candidates_endpoint,
        run_history_endpoint,
        run_comparison_endpoint,
        preflight_endpoint
    ),
    components(schemas(
        ExecutionContextV1,
        StageBudgets,
        PreflightReceipt,
        PreflightRequest,
        PreflightAcknowledgement,
        RecoveryEvent,
        Outcome,
        JobState,
        CleanupState,
        DefinitionKind,
        ApprovalPurpose,
        CheckMethod,
        UiProperty,
        ActionKind,
        EvidenceState,
        Driver,
        TestDefinition,
        ExecutionBudget,
        TestAction,
        ExpectedCheck,
        CaseDefinition,
        CaseSelection,
        SuiteDefinition,
        PlanDefinition,
        DefinitionImport,
        DefinitionApproval,
        DefinitionResponse,
        ExecutionProfile,
        ResolvedCase,
        ManifestSource,
        RunSourceRequest,
        RunManifest,
        PlanPreviewResponse,
        CreateRunRequest,
        SavedCasePreviewQuery,
        BaselineCandidateQuery,
        ComparisonLabel,
        CheckComparison,
        RunComparisonResponse,
        BaselineCandidate,
        BaselineCandidateResponse,
        CheckResult,
        RunArtifact,
        AttemptResponse,
        RunResponse,
        RunListResponse,
        RunHistoryFilter,
        RunHistoryQuery,
        RunHistoryItem,
        RunHistoryResponse,
        ClaimRequest,
        ExecutionLease,
        ClaimResponse,
        LeaseRequest,
        LeaseStatusResponse,
        ExecutionEvent,
        EventRequest,
        EventReceipt,
        ArtifactRequest,
        ArtifactReceipt,
        ModelUsage,
        CompleteRequest,
        CleanupRequest,
        AttemptReceipt
    ))
)]
pub struct ExecutionApi;
pub const OPERATIONS: &[(&str, &str, &str, u16)] = &[
    (
        "post",
        "/api/worker/attempts/{attempt_id}/preflight",
        "acknowledgeExecutionStart",
        200,
    ),
    (
        "get",
        "/api/apps/{app_id}/execution-plan",
        "getExecutionPlan",
        200,
    ),
    ("post", "/api/apps/{app_id}/runs", "createRun", 201),
    (
        "get",
        "/api/apps/{app_id}/saved-case-preview",
        "getSavedCasePreview",
        200,
    ),
    (
        "get",
        "/api/apps/{app_id}/baseline-candidates",
        "listBaselineCandidates",
        200,
    ),
    (
        "get",
        "/api/apps/{app_id}/run-history",
        "listRunHistory",
        200,
    ),
    ("get", "/api/apps/{app_id}/runs", "listRuns", 200),
    ("get", "/api/runs/{run_id}", "getRun", 200),
    (
        "get",
        "/api/runs/{run_id}/comparison",
        "getRunComparison",
        200,
    ),
    ("post", "/api/runs/{run_id}/cancel", "cancelRun", 200),
    (
        "get",
        "/api/runs/{run_id}/artifacts/{artifact_id}/content",
        "getRunArtifact",
        200,
    ),
];
