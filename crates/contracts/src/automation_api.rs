//! Generated clients describe exactly the authenticated authoring routes.
use crate::{automation::*, browser::ApiError, task_sessions::*};
use utoipa::OpenApi;
#[allow(dead_code)]
#[utoipa::path(post,path="/api/phones/{session_id}/commands",operation_id="runPhoneCommand",security(("session_cookie"=[])),params(("session_id"=Uuid,Path)),request_body=PhoneCommandRequest,responses((status=200,description="Success",body=PhoneSession),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError)))]
fn endpoint_0() {}
#[allow(dead_code)]
#[utoipa::path(get,path="/api/apps/{app_id}/test-templates",operation_id="getTestTemplates",security(("session_cookie"=[])),params(("app_id"=Uuid,Path)),responses((status=200,description="Success",body=TestTemplates),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError)))]
fn endpoint_1() {}
#[allow(dead_code)]
#[utoipa::path(post,path="/api/apps/{app_id}/test-generations",operation_id="generateTests",security(("session_cookie"=[])),params(("app_id"=Uuid,Path)),request_body=GenerateTestsRequest,responses((status=200,description="Success",body=PhoneSession),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError)))]
fn endpoint_2() {}
#[allow(dead_code)]
#[utoipa::path(get,path="/api/apps/{app_id}/test-generations/{job_id}",operation_id="getTestGeneration",security(("session_cookie"=[])),params(("app_id"=Uuid,Path),("job_id"=Uuid,Path)),responses((status=200,description="Success",body=PhoneTask),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError)))]
fn endpoint_3() {}
#[allow(dead_code)]
#[utoipa::path(post,path="/api/apps/{app_id}/test-generations/{job_id}/cancel",operation_id="cancelTestGeneration",security(("session_cookie"=[])),params(("app_id"=Uuid,Path),("job_id"=Uuid,Path)),responses((status=200,description="Success",body=PhoneSession),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError)))]
fn endpoint_4() {}
#[allow(dead_code)]
#[utoipa::path(post,path="/api/apps/{app_id}/test-library/from-recording",operation_id="saveAuthoredTests",security(("session_cookie"=[])),params(("app_id"=Uuid,Path)),request_body=SaveAuthoredTestsRequest,responses((status=200,description="Success",body=SavedAuthoredTests),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError)))]
fn endpoint_5() {}
#[derive(OpenApi)]
#[openapi(paths(endpoint_0, endpoint_1, endpoint_2, endpoint_3, endpoint_4, endpoint_5))]
pub struct AutomationApi;
pub const OPERATIONS: &[(&str, &str, &str, u16)] = &[
    (
        "post",
        "/api/phones/{session_id}/commands",
        "runPhoneCommand",
        200,
    ),
    (
        "get",
        "/api/apps/{app_id}/test-templates",
        "getTestTemplates",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/test-generations",
        "generateTests",
        200,
    ),
    (
        "get",
        "/api/apps/{app_id}/test-generations/{job_id}",
        "getTestGeneration",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/test-generations/{job_id}/cancel",
        "cancelTestGeneration",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/test-library/from-recording",
        "saveAuthoredTests",
        200,
    ),
];
