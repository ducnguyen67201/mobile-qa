//! Typed internal HTTP contracts, exported alongside the existing worker protocol.
use crate::{browser::ApiError, device_hosts::*};
use utoipa::OpenApi;
#[utoipa::path(post,path="/api/internal/hosts/{host_id}/register",operation_id="registerDeviceHost",params(("host_id" = uuid::Uuid, Path),("Authorization" = String, Header)),
request_body=HostRegisterRequest,
responses((status=200,description="Success",body=HostStatus),(status=401,description="Scoped bearer required",body=ApiError),(status=404,description="Resource not found",body=ApiError),(status=409,description="Stale authority or state",body=ApiError),(status=422,description="Invalid request",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn register_device_host() {}
#[utoipa::path(post,path="/api/internal/hosts/{host_id}/heartbeat",operation_id="heartbeatDeviceHost",params(("host_id" = uuid::Uuid, Path),("Authorization" = String, Header)),
request_body=HostHeartbeatRequest,
responses((status=200,description="Success",body=HostStatus),(status=401,description="Scoped bearer required",body=ApiError),(status=404,description="Resource not found",body=ApiError),(status=409,description="Stale authority or state",body=ApiError),(status=422,description="Invalid request",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn heartbeat_device_host() {}
#[utoipa::path(post,path="/api/internal/hosts/{host_id}/cleanup",operation_id="acknowledgeHostCleanup",params(("host_id" = uuid::Uuid, Path),("Authorization" = String, Header)),
request_body=HostCleanupRequest,
responses((status=200,description="Success",body=HostStatus),(status=401,description="Scoped bearer required",body=ApiError),(status=404,description="Resource not found",body=ApiError),(status=409,description="Stale authority or state",body=ApiError),(status=422,description="Invalid request",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn acknowledge_host_cleanup() {}
#[utoipa::path(post,path="/api/internal/hosts/{host_id}/slots/{slot_id}/grant",operation_id="createSlotGrant",params(("host_id" = uuid::Uuid, Path),("slot_id" = uuid::Uuid, Path),("Authorization" = String, Header)),
request_body=SlotGrantRequest,
responses((status=200,description="Success",body=SlotGrantResponse),(status=401,description="Scoped bearer required",body=ApiError),(status=404,description="Resource not found",body=ApiError),(status=409,description="Stale authority or state",body=ApiError),(status=422,description="Invalid request",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn create_slot_grant() {}
#[utoipa::path(get,path="/api/internal/capacity/pools/{pool_id}",operation_id="getCapacitySnapshot",params(("pool_id" = uuid::Uuid, Path),("Authorization" = String, Header)),
responses((status=200,description="Success",body=CapacitySnapshot),(status=401,description="Scoped bearer required",body=ApiError),(status=404,description="Resource not found",body=ApiError),(status=409,description="Stale authority or state",body=ApiError),(status=422,description="Invalid request",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn get_capacity_snapshot() {}
#[utoipa::path(post,path="/api/internal/capacity/pools/{pool_id}/actions",operation_id="applyCapacityAction",params(("pool_id" = uuid::Uuid, Path),("Authorization" = String, Header)),
request_body=CapacityActionRequest,
responses((status=200,description="Success",body=CapacitySnapshot),(status=401,description="Scoped bearer required",body=ApiError),(status=404,description="Resource not found",body=ApiError),(status=409,description="Stale authority or state",body=ApiError),(status=422,description="Invalid request",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn apply_capacity_action() {}
#[utoipa::path(post,path="/api/internal/capacity/pools/{pool_id}/hints",operation_id="consumeCapacityHint",params(("pool_id" = uuid::Uuid, Path),("Authorization" = String, Header)),
request_body=ConsumeHintRequest,
responses((status=200,description="Success",body=HintReceipt),(status=401,description="Scoped bearer required",body=ApiError),(status=404,description="Resource not found",body=ApiError),(status=409,description="Stale authority or state",body=ApiError),(status=422,description="Invalid request",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code)]
fn consume_capacity_hint() {}
#[derive(OpenApi)]
#[openapi(
    paths(
        register_device_host,
        heartbeat_device_host,
        acknowledge_host_cleanup,
        create_slot_grant,
        get_capacity_snapshot,
        apply_capacity_action,
        consume_capacity_hint
    ),
    components(schemas(
        HostRegisterRequest,
        HostStatus,
        HostHeartbeatRequest,
        SlotGrantRequest,
        SlotGrantResponse,
        HostCleanupRequest,
        CapacitySnapshot,
        CapacityActionRequest,
        ConsumeHintRequest,
        HintReceipt
    ))
)]
pub struct DeviceHostsApi;
pub const OPERATIONS: &[(&str, &str, &str, u16)] = &[
    (
        "post",
        "/api/internal/hosts/{host_id}/register",
        "registerDeviceHost",
        200,
    ),
    (
        "post",
        "/api/internal/hosts/{host_id}/heartbeat",
        "heartbeatDeviceHost",
        200,
    ),
    (
        "post",
        "/api/internal/hosts/{host_id}/cleanup",
        "acknowledgeHostCleanup",
        200,
    ),
    (
        "post",
        "/api/internal/hosts/{host_id}/slots/{slot_id}/grant",
        "createSlotGrant",
        200,
    ),
    (
        "get",
        "/api/internal/capacity/pools/{pool_id}",
        "getCapacitySnapshot",
        200,
    ),
    (
        "post",
        "/api/internal/capacity/pools/{pool_id}/actions",
        "applyCapacityAction",
        200,
    ),
    (
        "post",
        "/api/internal/capacity/pools/{pool_id}/hints",
        "consumeCapacityHint",
        200,
    ),
];
