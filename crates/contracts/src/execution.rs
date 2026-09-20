//! Approved semantic test definitions and evidence records, shared by both transports.
//! Derives describe shape; every server import also invokes semantic validation.
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use utoipa::ToSchema;
use uuid::Uuid;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Passed,
    Failed,
    Blocked,
    Inconclusive,
    Skipped,
    Canceled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Leased,
    Running,
    Finalizing,
    Finished,
    CancelRequested,
    RecoveryRequired,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CleanupState {
    Pending,
    VerifiedClean,
    Quarantined,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionKind {
    Case,
    Suite,
    Plan,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPurpose {
    Business,
    Executability,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckMethod {
    UiPropertyEqualsV1,
    UiElementPresenceV1,
    Manual,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum UiProperty {
    Text,
    ContentDescription,
    Checked,
    Enabled,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    Direct,
    Navigate,
    RestartApp,
    Checkpoint,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceState {
    Pending,
    Sealed,
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Driver {
    Direct,
    Fake,
    Minitap,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(
    tag = "kind",
    content = "content",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TestDefinition {
    Case(CaseDefinition),
    Suite(SuiteDefinition),
    Plan(PlanDefinition),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionBudget {
    pub duration_seconds: u32,
    pub max_steps: u32,
    pub artifact_bytes: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TestAction {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<crate::automation::DirectCommand>,
    pub id: String,
    pub kind: ActionKind,
    pub instruction: String,
    pub checkpoint_id: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExpectedCheck {
    pub id: String,
    pub checkpoint_id: String,
    pub description: String,
    pub method: CheckMethod,
    pub resource_id: String,
    pub text_filter: String,
    pub property: UiProperty,
    pub expected: String,
    pub ready_resource_id: String,
    pub prerequisite_check_ids: Vec<String>,
    pub required: bool,
    pub observation_seconds: u32,
}
impl TestAction {
    pub fn validate(&self, package: &str) -> Result<(), &'static str> {
        for id in [&self.id, &self.checkpoint_id] {
            if id.is_empty()
                || id.len() > 100
                || !id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
            {
                return Err("Invalid step identity");
            }
        }
        match (&self.kind, &self.command) {
            (ActionKind::Direct, Some(command)) if self.instruction.is_empty() => {
                command.validate(package)
            }
            (ActionKind::Navigate, None)
                if !self.instruction.trim().is_empty() && self.instruction.len() <= 4000 =>
            {
                Ok(())
            }
            (ActionKind::RestartApp | ActionKind::Checkpoint, None)
                if self.instruction.is_empty() =>
            {
                Ok(())
            }
            _ => Err("Action and command do not agree"),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CaseDefinition {
    pub key: String,
    pub version: u32,
    pub title: String,
    pub requirement: String,
    pub provenance: String,
    pub package: String,
    pub adapter: String,
    pub preconditions: Vec<String>,
    pub actions: Vec<TestAction>,
    pub checks: Vec<ExpectedCheck>,
    pub budget: ExecutionBudget,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CaseSelection {
    pub case_version_id: Uuid,
    pub data_variant: String,
    pub required: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SuiteDefinition {
    pub key: String,
    pub version: u32,
    pub title: String,
    pub cases: Vec<CaseSelection>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanDefinition {
    pub key: String,
    pub version: u32,
    pub title: String,
    pub suite_version_ids: Vec<Uuid>,
    pub cases: Vec<CaseSelection>,
    pub profile_id: Uuid,
    pub budget: ExecutionBudget,
    pub diagnostic_retries: u8,
    pub exclusions: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DefinitionImport {
    pub app_id: Uuid,
    pub definition: TestDefinition,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DefinitionApproval {
    pub purpose: ApprovalPurpose,
    pub actor_id: Uuid,
    pub content_hash: String,
    pub approved_at: DateTime<Utc>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DefinitionResponse {
    pub id: Uuid,
    pub app_id: Uuid,
    pub content_hash: String,
    pub definition: TestDefinition,
    pub approvals: Vec<DefinitionApproval>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_context: Option<crate::execution_lifecycle::ExecutionContextV1>,
    pub id: Uuid,
    pub name: String,
    pub driver: Driver,
    pub package: String,
    pub adapter: String,
    pub device_identity: String,
    pub image: String,
    pub model: String,
    pub qualified: bool,
    pub qualification_reference: String,
    pub max_apk_bytes: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedCase {
    pub definition_id: Uuid,
    pub content_hash: String,
    pub data_variant: String,
    pub required: bool,
    pub case: CaseDefinition,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RunManifest {
    pub app_id: Uuid,
    pub build_id: Uuid,
    pub build_sha256: String,
    pub build_bytes: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<crate::regression::RunSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_version_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_hash: Option<String>,
    pub environment_revision: i32,
    pub profile: ExecutionProfile,
    pub cases: Vec<ResolvedCase>,
    pub budget: ExecutionBudget,
    pub diagnostic_retries: u8,
    pub exclusions: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanPreviewResponse {
    pub plan: Option<DefinitionResponse>,
    pub manifest: Option<RunManifest>,
    pub blockers: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateRunRequest {
    pub build_id: Uuid,
    pub plan_version_id: Uuid,
    pub environment_revision: i32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CheckResult {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observation_kind: Option<crate::regression::ObservationKind>,
    pub check_id: String,
    pub expected: String,
    pub observed: Option<String>,
    pub outcome: Outcome,
    pub reason: String,
    pub artifact_ids: Vec<Uuid>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RunArtifact {
    pub id: Uuid,
    pub attempt_id: Uuid,
    pub checkpoint_id: String,
    pub name: String,
    pub mime: String,
    pub byte_size: u32,
    pub sha256: String,
    pub state: EvidenceState,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AttemptResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preflight: Option<crate::execution_lifecycle::PreflightReceipt>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recovery_events: Vec<crate::execution_lifecycle::RecoveryEvent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_cleanup: Option<CleanupRequest>,
    pub id: Uuid,
    pub case_version_id: Uuid,
    pub generation: i32,
    pub number: u32,
    pub state: JobState,
    pub outcome: Option<Outcome>,
    pub cleanup: CleanupState,
    pub reason: Option<String>,
    pub checks: Vec<CheckResult>,
    pub artifacts: Vec<RunArtifact>,
    pub usage: Vec<ModelUsage>,
    pub events: Vec<ExecutionEvent>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RunResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comparison: Option<crate::regression::RunComparison>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_run_id: Option<Uuid>,
    pub id: Uuid,
    pub manifest: RunManifest,
    pub state: JobState,
    pub summary: String,
    pub created_at: DateTime<Utc>,
    pub attempts: Vec<AttemptResponse>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RunListResponse {
    pub items: Vec<RunResponse>,
    pub next_cursor: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimRequest {
    pub version: u8,
    pub claim_id: Uuid,
    pub profile_id: Uuid,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionLease {
    pub attempt_id: Uuid,
    pub run_id: Uuid,
    pub generation: i32,
    pub lease_token: String,
    pub expires_at: DateTime<Utc>,
    pub manifest: RunManifest,
    pub case_index: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ClaimResponse {
    pub lease: Option<ExecutionLease>,
    pub poll_after_seconds: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LeaseRequest {
    pub generation: i32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LeaseStatusResponse {
    pub state: JobState,
    pub expires_at: DateTime<Utc>,
    pub cancel_requested: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionEvent {
    pub id: Uuid,
    pub sequence: u32,
    pub action_id: String,
    pub phase: String,
    pub message: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EventRequest {
    pub generation: i32,
    pub events: Vec<ExecutionEvent>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EventReceipt {
    pub last_sequence: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRequest {
    pub generation: i32,
    pub checkpoint_id: String,
    pub name: String,
    pub mime: String,
    pub byte_size: u32,
    pub sha256: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ArtifactReceipt {
    pub artifact: RunArtifact,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelUsage {
    pub model: String,
    pub calls: u32,
    pub unknown_calls: u32,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CompleteRequest {
    pub generation: i32,
    pub execution_outcome: Outcome,
    pub reason: String,
    pub usage: Vec<ModelUsage>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CleanupRequest {
    pub generation: i32,
    pub stopped: bool,
    pub reset: CleanupState,
    pub evidence_reference: String,
    pub boot_id: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AttemptReceipt {
    pub attempt: AttemptResponse,
}

impl ExecutionBudget {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !(1..=1800).contains(&self.duration_seconds)
            || !(1..=400).contains(&self.max_steps)
            || !(1024..=262144000).contains(&self.artifact_bytes)
        {
            return Err("invalid execution budget");
        }
        Ok(())
    }
}
pub fn bounded(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max && !value.chars().any(|c| c.is_control())
}
impl TestDefinition {
    pub fn identity(&self) -> (DefinitionKind, &str, u32) {
        match self {
            Self::Case(c) => (DefinitionKind::Case, &c.key, c.version),
            Self::Suite(c) => (DefinitionKind::Suite, &c.key, c.version),
            Self::Plan(c) => (DefinitionKind::Plan, &c.key, c.version),
        }
    }
    pub fn validate(&self) -> Result<(), &'static str> {
        let (_, key, version) = self.identity();
        if !bounded(key, 100) || version == 0 {
            return Err("invalid definition identity");
        }
        match self {
            Self::Case(c) => {
                c.budget.validate()?;
                if !bounded(&c.title, 200)
                    || !bounded(&c.requirement, 4000)
                    || !["operator_authored", "user_authored"].contains(&c.provenance.as_str())
                    || !bounded(&c.package, 255)
                    || !bounded(&c.adapter, 100)
                    || c.actions.is_empty()
                    || c.actions.len() > 20
                    || c.checks.is_empty()
                    || c.checks.len() > 20
                    || c.preconditions.len() > 20
                    || c.preconditions.iter().any(|v| !bounded(v, 1000))
                {
                    return Err("invalid case definition");
                }
                let mut actions = BTreeSet::new();
                let mut checkpoints = BTreeSet::new();
                for a in &c.actions {
                    a.validate(&c.package)?;
                    if !bounded(&a.id, 100)
                        || !bounded(&a.checkpoint_id, 100)
                        || [&a.id, &a.checkpoint_id].iter().any(|v| {
                            !v.bytes()
                                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                        })
                        || !actions.insert(&a.id)
                        || !checkpoints.insert(&a.checkpoint_id)
                        || a.instruction.len() > 4000
                        || (a.kind == ActionKind::Navigate && !bounded(&a.instruction, 4000))
                        || (a.kind != ActionKind::Navigate && !a.instruction.is_empty())
                    {
                        return Err("invalid or duplicate action");
                    }
                    let without = a.instruction.replace("${task_title}", "");
                    if without.contains("${") {
                        return Err("unsupported data placeholder");
                    }
                }
                let mut checks = BTreeSet::new();
                for check in &c.checks {
                    if !bounded(&check.id, 100)
                        || !checks.insert(&check.id)
                        || !checkpoints.contains(&check.checkpoint_id)
                        || !bounded(&check.description, 1000)
                        || !bounded(&check.resource_id, 255)
                        || !bounded(&check.ready_resource_id, 255)
                        || check.text_filter.len() > 1000
                        || check.expected.len() > 1000
                        || !(1..=60).contains(&check.observation_seconds)
                    {
                        return Err("invalid check");
                    }
                    if check
                        .prerequisite_check_ids
                        .iter()
                        .any(|id| id == &check.id || !checks.contains(id))
                    {
                        return Err("checks must reference earlier prerequisites");
                    }
                    for target in [&check.resource_id, &check.ready_resource_id] {
                        if !target.starts_with(&format!("{}:id/", c.package)) {
                            return Err("check target is outside app");
                        }
                    }
                    if (check.method == CheckMethod::UiElementPresenceV1
                        || matches!(check.property, UiProperty::Checked | UiProperty::Enabled))
                        && !["true", "false"].contains(&check.expected.as_str())
                    {
                        return Err("expected Boolean must be true or false");
                    }
                    if check.expected.replace("${task_title}", "").contains("${") {
                        return Err("unsupported expected placeholder");
                    }
                }
                if !c.checks.iter().any(|check| check.required) {
                    return Err("case needs a required check");
                }
            }
            Self::Suite(s) => {
                if !bounded(&s.title, 200) || s.cases.is_empty() || s.cases.len() > 100 {
                    return Err("invalid suite");
                }
                validate_selections(&s.cases)?;
            }
            Self::Plan(p) => {
                p.budget.validate()?;
                if !bounded(&p.title, 200)
                    || p.suite_version_ids.len() + p.cases.len() == 0
                    || p.suite_version_ids.len() + p.cases.len() > 100
                    || p.diagnostic_retries > 1
                    || p.exclusions.len() > 100
                {
                    return Err("invalid plan");
                }
                validate_selections(&p.cases)?;
            }
        }
        Ok(())
    }
}
fn validate_selections(selections: &[CaseSelection]) -> Result<(), &'static str> {
    if selections.iter().any(|s| s.data_variant != "default") {
        return Err("only the default synthetic data variant is supported");
    }
    Ok(())
}
impl ExecutionProfile {
    pub fn validate(&self) -> Result<(), &'static str> {
        if !bounded(&self.name, 200)
            || !bounded(&self.device_identity, 200)
            || !bounded(&self.image, 200)
            || !bounded(&self.package, 255)
            || !(1..=262144000).contains(&self.max_apk_bytes)
            || (self.qualified && !bounded(&self.qualification_reference, 1000))
            || (self.driver == Driver::Minitap
                && (!bounded(&self.model, 100) || self.max_apk_bytes > 104857600))
        {
            return Err("invalid or unsupported execution profile");
        }
        match self.adapter.as_str() {
            "demo_persistence_v1"
                if self.package == "ai.mobileqa.demo" && self.execution_context.is_none() => {}
            "android_direct_v1"
                if self.driver == Driver::Direct
                    && self.qualified
                    && self.model.is_empty()
                    && self.max_apk_bytes <= 104857600 =>
            {
                self.execution_context
                    .as_ref()
                    .ok_or("Clean-start qualification is required")?
                    .validate(self)?;
            }
            _ => return Err("invalid or unsupported execution profile"),
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionJob {
    pub attempt_id: Uuid,
    pub manifest: RunManifest,
    pub case_index: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct NavigationRequest {
    pub attempt_id: Uuid,
    pub profile_path: String,
    pub serial: String,
    pub package: String,
    pub instruction: String,
    pub max_steps: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LocalExecutionResult {
    pub outcome: Outcome,
    pub reason: String,
    pub usage: Vec<ModelUsage>,
    pub reset: CleanupState,
    pub stopped: bool,
    pub boot_id: String,
    pub evidence_reference: String,
}
