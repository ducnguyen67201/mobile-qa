//! Task sessions are interactive device work, not approved regression verdicts.
use crate::execution::ExecutionProfile;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PhoneState {
    Queued,
    Preparing,
    Ready,
    Acting,
    Stopping,
    Closed,
    Quarantined,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PhoneTaskState {
    Queued,
    Acting,
    Completed,
    Failed,
    Stopped,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhoneControl {
    #[serde(default)]
    pub editable: bool,
    #[serde(default)]
    pub description: String,
    pub id: String,
    pub label: String,
    pub resource_id: String,
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhoneFrame {
    pub id: Uuid,
    pub png_base64: String,
    pub width: u32,
    pub height: u32,
    pub controls: Vec<PhoneControl>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhoneSelection {
    pub frame_id: Uuid,
    pub control_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenPhoneRequest {
    pub id: Uuid,
    pub build_id: Option<Uuid>,
    pub profile_id: Option<Uuid>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhoneTaskRequest {
    pub id: Uuid,
    pub goal: String,
    pub selection: Option<PhoneSelection>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhoneTask {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sequence: Option<crate::automation::AutomationSequence>,
    #[serde(default)]
    pub steps: Vec<crate::automation::StepReceipt>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generation: Option<crate::automation::GenerateTestsRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<crate::automation::GenerationProgress>,
    pub id: Uuid,
    pub goal: String,
    pub control: Option<PhoneControl>,
    pub state: PhoneTaskState,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhoneSession {
    #[serde(default)]
    pub environment_revision: u32,
    #[serde(default)]
    pub revision: u32,
    #[serde(default)]
    pub protocol_version: u32,
    pub id: Uuid,
    pub app_id: Uuid,
    pub build_id: Uuid,
    pub profile: ExecutionProfile,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_model: Option<crate::model_registry::ResolvedModel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_assignment_revision: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authoring_model: Option<crate::model_registry::ResolvedModel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authoring_assignment_revision: Option<u32>,
    pub state: PhoneState,
    pub message: String,
    pub frame: Option<PhoneFrame>,
    pub tasks: Vec<PhoneTask>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhoneBuildChoice {
    pub id: Uuid,
    pub name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhoneOptions {
    pub builds: Vec<PhoneBuildChoice>,
    pub profiles: Vec<ExecutionProfile>,
    pub active_session: Option<Uuid>,
    pub blockers: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhoneLease {
    pub session: PhoneSession,
    pub lease_token: String,
    pub build_sha256: String,
    #[schema(schema_with = crate::artifacts_api::apk_byte_schema)]
    #[schemars(range(min = 1, max = 2147483648_u32))]
    pub build_bytes: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhoneClaimRequest {
    #[serde(default)]
    pub protocol_version: u32,
    pub claim_id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_capabilities: Option<crate::model_registry::WorkerModelCapabilities>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhoneClaimResponse {
    pub lease: Option<PhoneLease>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhoneUpdate {
    pub state: PhoneState,
    pub frame: Option<PhoneFrame>,
    pub task: Option<PhoneTask>,
    pub message: String,
    pub clean: bool,
}
