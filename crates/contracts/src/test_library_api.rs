//! Pure route declarations; integration tests exercise the matching authenticated handlers.
use crate::{browser::ApiError, test_library::*};
use utoipa::OpenApi;
#[utoipa::path(get, path="/api/apps/{app_id}/test-library", operation_id="listTestLibrary", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path),LibraryListQuery),
responses((status=200,description="Success",body=LibraryListResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_0() {}
#[utoipa::path(get, path="/api/apps/{app_id}/test-library/options", operation_id="getTestLibraryOptions", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path)),
responses((status=200,description="Success",body=LibraryOptionsResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_1() {}
#[utoipa::path(post, path="/api/apps/{app_id}/test-library", operation_id="createTestLibraryEntry", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path),("X-CSRF-Token" = String, Header)),
request_body=CreateLibraryEntryRequest,
responses((status=201,description="Success",body=LibraryDraftResponse),(status=200,description="Mutation replay",body=LibraryDraftResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_2() {}
#[utoipa::path(get, path="/api/apps/{app_id}/test-library/{entry_id}", operation_id="getTestLibraryEntry", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path),("entry_id" = uuid::Uuid, Path)),
responses((status=200,description="Success",body=LibraryEntryResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_3() {}
#[utoipa::path(get, path="/api/apps/{app_id}/test-library/{entry_id}/draft", operation_id="getTestLibraryDraft", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path),("entry_id" = uuid::Uuid, Path)),
responses((status=200,description="Success",body=LibraryDraftResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_4() {}
#[utoipa::path(post, path="/api/apps/{app_id}/test-library/{entry_id}/draft", operation_id="forkTestLibraryDraft", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path),("entry_id" = uuid::Uuid, Path),("X-CSRF-Token" = String, Header)),
request_body=ForkLibraryDraftRequest,
responses((status=201,description="Success",body=LibraryDraftResponse),(status=200,description="Mutation replay",body=LibraryDraftResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_5() {}
#[utoipa::path(put, path="/api/apps/{app_id}/test-library/{entry_id}/draft", operation_id="saveTestLibraryDraft", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path),("entry_id" = uuid::Uuid, Path),("X-CSRF-Token" = String, Header)),
request_body=SaveLibraryDraftRequest,
responses((status=200,description="Success",body=LibraryDraftResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_6() {}
#[utoipa::path(post, path="/api/apps/{app_id}/test-library/{entry_id}/submit", operation_id="submitTestLibraryDraft", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path),("entry_id" = uuid::Uuid, Path),("X-CSRF-Token" = String, Header)),
request_body=SubmitLibraryDraftRequest,
responses((status=201,description="Success",body=LibraryVersionResponse),(status=200,description="Mutation replay",body=LibraryVersionResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_7() {}
#[utoipa::path(get, path="/api/apps/{app_id}/test-library/{entry_id}/versions", operation_id="listTestLibraryVersions", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path),("entry_id" = uuid::Uuid, Path),LibraryVersionQuery),
responses((status=200,description="Success",body=LibraryVersionListResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_8() {}
#[utoipa::path(get, path="/api/apps/{app_id}/test-library/{entry_id}/versions/{version_id}", operation_id="getTestLibraryVersion", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path),("entry_id" = uuid::Uuid, Path),("version_id" = uuid::Uuid, Path)),
responses((status=200,description="Success",body=LibraryVersionResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_9() {}
#[utoipa::path(post, path="/api/apps/{app_id}/test-library/{entry_id}/versions/{version_id}/review", operation_id="reviewTestLibraryVersion", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path),("entry_id" = uuid::Uuid, Path),("version_id" = uuid::Uuid, Path),("X-CSRF-Token" = String, Header)),
request_body=ReviewLibraryVersionRequest,
responses((status=200,description="Success",body=LibraryVersionResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_10() {}
#[utoipa::path(post, path="/api/apps/{app_id}/test-library/{entry_id}/archive", operation_id="archiveTestLibraryEntry", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path),("entry_id" = uuid::Uuid, Path),("X-CSRF-Token" = String, Header)),
request_body=ArchiveLibraryEntryRequest,
responses((status=200,description="Success",body=LibraryEntryResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_11() {}
#[utoipa::path(get, path="/api/apps/{app_id}/default-test-plan", operation_id="getDefaultTestPlan", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path)),
responses((status=200,description="Success",body=DefaultPlanResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_12() {}
#[utoipa::path(put, path="/api/apps/{app_id}/default-test-plan", operation_id="setDefaultTestPlan", security(("session_cookie"=[])),
params(("app_id" = uuid::Uuid, Path),("X-CSRF-Token" = String, Header)),
request_body=SetDefaultPlanRequest,
responses((status=200,description="Success",body=DefaultPlanResponse),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_13() {}
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
        endpoint_13
    ),
    components(schemas(
        LibraryReviewState,
        LibraryReviewDecision,
        LibraryIssueCode,
        LibraryIssue,
        LibraryErrorDetails,
        PlanDraftContent,
        LibraryDraftDefinition,
        LibraryCapabilities,
        LibraryEntryResponse,
        LibraryCoveragePreview,
        LibraryDraftResponse,
        LibraryReviewEvent,
        LibraryVersionResponse,
        LibraryListResponse,
        LibraryVersionListResponse,
        LibraryProfileChoice,
        LibraryOptionsResponse,
        DefaultPlanResponse,
        CreateLibraryEntryRequest,
        ForkLibraryDraftRequest,
        SaveLibraryDraftRequest,
        SubmitLibraryDraftRequest,
        ReviewLibraryVersionRequest,
        ArchiveLibraryEntryRequest,
        SetDefaultPlanRequest,
        LibraryListQuery,
        LibraryVersionQuery,
        ExecutionPlanQuery
    ))
)]
pub struct TestLibraryApi;
pub const OPERATIONS: &[(&str, &str, &str, u16)] = &[
    (
        "get",
        "/api/apps/{app_id}/test-library",
        "listTestLibrary",
        200,
    ),
    (
        "get",
        "/api/apps/{app_id}/test-library/options",
        "getTestLibraryOptions",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/test-library",
        "createTestLibraryEntry",
        201,
    ),
    (
        "get",
        "/api/apps/{app_id}/test-library/{entry_id}",
        "getTestLibraryEntry",
        200,
    ),
    (
        "get",
        "/api/apps/{app_id}/test-library/{entry_id}/draft",
        "getTestLibraryDraft",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/test-library/{entry_id}/draft",
        "forkTestLibraryDraft",
        201,
    ),
    (
        "put",
        "/api/apps/{app_id}/test-library/{entry_id}/draft",
        "saveTestLibraryDraft",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/test-library/{entry_id}/submit",
        "submitTestLibraryDraft",
        201,
    ),
    (
        "get",
        "/api/apps/{app_id}/test-library/{entry_id}/versions",
        "listTestLibraryVersions",
        200,
    ),
    (
        "get",
        "/api/apps/{app_id}/test-library/{entry_id}/versions/{version_id}",
        "getTestLibraryVersion",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/test-library/{entry_id}/versions/{version_id}/review",
        "reviewTestLibraryVersion",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/test-library/{entry_id}/archive",
        "archiveTestLibraryEntry",
        200,
    ),
    (
        "get",
        "/api/apps/{app_id}/default-test-plan",
        "getDefaultTestPlan",
        200,
    ),
    (
        "put",
        "/api/apps/{app_id}/default-test-plan",
        "setDefaultTestPlan",
        200,
    ),
];
