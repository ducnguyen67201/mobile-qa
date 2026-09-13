//! Authenticated browser phone session contract; worker routes use bearer identity.
use crate::{browser::ApiError, task_sessions::*};
use utoipa::OpenApi;
#[utoipa::path(get,path="/api/apps/{app_id}/phone-options",operation_id="getPhoneOptions",security(("session_cookie"=[])),params(("app_id"=Uuid,Path)),
responses((status=200,description="Success",body=PhoneOptions),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_0() {}
#[utoipa::path(post,path="/api/apps/{app_id}/phones",operation_id="openPhone",security(("session_cookie"=[])),params(("app_id"=Uuid,Path)),
request_body=OpenPhoneRequest,
responses((status=200,description="Success",body=PhoneSession),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_1() {}
#[utoipa::path(get,path="/api/phones/{session_id}",operation_id="getPhone",security(("session_cookie"=[])),params(("session_id"=Uuid,Path)),
responses((status=200,description="Success",body=PhoneSession),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_2() {}
#[utoipa::path(post,path="/api/phones/{session_id}/tasks",operation_id="runPhoneTask",security(("session_cookie"=[])),params(("session_id"=Uuid,Path)),
request_body=PhoneTaskRequest,
responses((status=200,description="Success",body=PhoneSession),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_3() {}
#[utoipa::path(post,path="/api/phones/{session_id}/stop",operation_id="stopPhone",security(("session_cookie"=[])),params(("session_id"=Uuid,Path)),
responses((status=200,description="Success",body=PhoneSession),(status=400,description="API error",body=ApiError),(status=401,description="API error",body=ApiError),(status=403,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=413,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status=429,description="API error",body=ApiError),(status=500,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn endpoint_4() {}
#[derive(OpenApi)]
#[openapi(paths(endpoint_0, endpoint_1, endpoint_2, endpoint_3, endpoint_4))]
pub struct TaskSessionsApi;
pub const OPERATIONS: &[(&str, &str, &str, u16)] = &[
    (
        "get",
        "/api/apps/{app_id}/phone-options",
        "getPhoneOptions",
        200,
    ),
    ("post", "/api/apps/{app_id}/phones", "openPhone", 200),
    ("get", "/api/phones/{session_id}", "getPhone", 200),
    (
        "post",
        "/api/phones/{session_id}/tasks",
        "runPhoneTask",
        200,
    ),
    ("post", "/api/phones/{session_id}/stop", "stopPhone", 200),
];
