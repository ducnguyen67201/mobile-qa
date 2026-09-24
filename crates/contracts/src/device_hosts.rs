//! Internal host/control protocol. Host credentials cannot authorize customer artifacts.
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HostState {
    Stopped,
    Starting,
    Ready,
    Draining,
    StopCommitted,
    Stopping,
    Quarantined,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SlotState {
    Offline,
    Preparing,
    Idle,
    Leased,
    Cleaning,
    Quarantined,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObservedPower {
    Unknown,
    Stopped,
    Pending,
    Running,
    Stopping,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WarmWindow {
    pub weekdays: Vec<u8>,
    pub start: String,
    pub end: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PoolPolicy {
    pub enabled: bool,
    pub idle_seconds: u32,
    pub warm_target: u32,
    pub timezone: String,
    pub warm_windows: Vec<WarmWindow>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SlotDefinition {
    pub id: Uuid,
    pub index: u32,
    pub device_identity: String,
    pub system_image: String,
    pub memory_mb: u32,
    pub cpu_cores: u32,
    pub console_port: u16,
    pub adb_server_port: u16,
    pub qualified: bool,
    pub warm_qualified: bool,
    pub qualification_reference: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SlotBinding {
    pub slot_id: Uuid,
    pub app_id: Uuid,
    pub profile_id: Uuid,
}
/// Trusted CLI input; credentials are injected separately, never serialized in this file.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PoolRegistration {
    pub pool_id: Uuid,
    pub host_id: Uuid,
    pub instance_id: String,
    pub toolchain_digest: String,
    pub policy: PoolPolicy,
    pub slots: Vec<SlotDefinition>,
    pub bindings: Vec<SlotBinding>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HostRegisterRequest {
    pub version: u8,
    pub boot_id: Uuid,
    pub toolchain_digest: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SlotHeartbeat {
    pub slot_id: Uuid,
    pub state: SlotState,
    pub emulator_boot_id: Option<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HostHeartbeatRequest {
    pub generation: i32,
    pub boot_id: Uuid,
    pub slots: Vec<SlotHeartbeat>,
    pub cache_bytes: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HostSlotStatus {
    pub definition: SlotDefinition,
    pub state: SlotState,
    pub emulator_boot_id: Option<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HostStatus {
    pub host_id: Uuid,
    pub pool_id: Uuid,
    pub boot_id: Option<Uuid>,
    pub generation: i32,
    pub state: HostState,
    pub heartbeat_seconds: u32,
    pub policy: PoolPolicy,
    pub slots: Vec<HostSlotStatus>,
    pub bindings: Vec<SlotBinding>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SlotGrantRequest {
    pub generation: i32,
    pub boot_id: Uuid,
    pub app_id: Uuid,
    pub profile_id: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SlotGrantResponse {
    pub worker_id: Uuid,
    pub app_id: Uuid,
    pub profile_id: Uuid,
    pub worker_token: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HostCleanupRequest {
    pub generation: i32,
    pub boot_id: Uuid,
    pub processes_stopped: bool,
    pub ports_released: bool,
    pub journals_resolved: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapacityAction {
    Observe,
    Start,
    Drain,
    CommitStop,
    RecordPower,
    Quarantine,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CapacityActionRequest {
    pub request_id: Uuid,
    pub action: CapacityAction,
    pub host_id: Uuid,
    pub expected_control_version: i32,
    pub observed_power: Option<ObservedPower>,
    pub operation_id: Option<Uuid>,
    pub reason: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CapacityOperation {
    pub id: Uuid,
    pub action: CapacityAction,
    pub created_at: DateTime<Utc>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CapacitySnapshot {
    pub pool_id: Uuid,
    pub host_id: Uuid,
    pub instance_id: String,
    pub policy: PoolPolicy,
    pub state: HostState,
    pub observed_power: ObservedPower,
    pub boot_id: Option<Uuid>,
    pub generation: i32,
    pub control_version: i32,
    pub queued_jobs: i64,
    pub active_work: i64,
    pub desired_online: bool,
    pub last_demand_at: DateTime<Utc>,
    pub heartbeat_at: Option<DateTime<Utc>>,
    pub clean_at: Option<DateTime<Utc>>,
    pub startup_started_at: Option<DateTime<Utc>>,
    pub current_operation: Option<CapacityOperation>,
    pub reason: Option<String>,
    pub cache_bytes: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CapacityHint {
    pub pool_id: Uuid,
    pub request_id: Uuid,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConsumeHintRequest {
    pub request_id: Uuid,
    pub timestamp: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HintReceipt {
    pub accepted: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SlotDeviceOperation {
    Boot,
    Install,
    Launch,
    Adb,
    Snapshot,
    Stop,
    Discard,
    Direct,
    Hierarchy,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SlotDeviceRequest {
    pub token: String,
    pub operation: SlotDeviceOperation,
    pub package: Option<String>,
    pub launch_component: Option<String>,
    pub path: Option<String>,
    pub sha256: Option<String>,
    #[serde(default)]
    pub arguments: Vec<String>,
    pub timeout_seconds: Option<u32>,
    pub command: Option<crate::automation::DirectCommand>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SlotDeviceResponse {
    pub ok: bool,
    pub reason: Option<String>,
    pub data_base64: Option<String>,
    pub xml_base64: Option<String>,
}
/// Reachable schema root for the generated worker and controller consumers.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct HostContracts {
    pub registration: PoolRegistration,
    pub register: HostRegisterRequest,
    pub status: HostStatus,
    pub heartbeat: HostHeartbeatRequest,
    pub grant: SlotGrantRequest,
    pub grant_response: SlotGrantResponse,
    pub cleanup: HostCleanupRequest,
    pub action: CapacityActionRequest,
    pub snapshot: CapacitySnapshot,
    pub hint: CapacityHint,
    pub consume_hint: ConsumeHintRequest,
    pub hint_receipt: HintReceipt,
    pub device_request: SlotDeviceRequest,
    pub device_response: SlotDeviceResponse,
}
