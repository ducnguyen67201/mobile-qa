//! Structured device commands and bounded authoring. Rust owns every consumer shape.
use crate::execution::{CaseDefinition, CheckMethod, ExpectedCheck, TestAction, UiProperty};
use crate::task_sessions::PhoneFrame;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(tag = "by", rename_all = "snake_case", deny_unknown_fields)]
pub enum DirectTarget {
    ResourceId { value: String },
    Description { value: String },
}
impl DirectTarget {
    pub fn validate(&self, package: &str) -> Result<(), &'static str> {
        let value = match self {
            Self::ResourceId { value } => {
                if !value.starts_with(&format!("{package}:id/")) {
                    return Err("Target belongs to another app");
                }
                value
            }
            Self::Description { value } => value,
        };
        if value.trim().is_empty() || value.len() > 500 || value.chars().any(char::is_control) {
            return Err("Invalid target");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SwipeDirection {
    Up,
    Down,
    Left,
    Right,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub enum DirectCommand {
    Tap { target: DirectTarget },
    SetText { target: DirectTarget, text: String },
    Swipe { direction: SwipeDirection },
    Back {},
    Restart {},
    WaitFor { target: DirectTarget },
}
impl DirectCommand {
    pub fn target(&self) -> Option<&DirectTarget> {
        match self {
            Self::Tap { target } | Self::SetText { target, .. } | Self::WaitFor { target } => {
                Some(target)
            }
            _ => None,
        }
    }
    pub fn validate(&self, package: &str) -> Result<(), &'static str> {
        if let Some(target) = self.target() {
            target.validate(package)?;
        }
        if let Self::SetText { text, .. } = self {
            if text.len() > 4000 || text.contains('\0') {
                return Err("Text exceeds supported limits");
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AutomationSequence {
    pub actions: Vec<TestAction>,
    pub checks: Vec<ExpectedCheck>,
}
impl AutomationSequence {
    pub fn validate(&self, package: &str) -> Result<(), &'static str> {
        if self.actions.is_empty() || self.actions.len() > 20 || self.checks.len() > 20 {
            return Err("Use 1–20 steps and at most 20 checks");
        }
        let mut ids = std::collections::BTreeSet::new();
        let mut checkpoints = std::collections::BTreeSet::new();
        for action in &self.actions {
            action.validate(package)?;
            if !ids.insert(&action.id) || !checkpoints.insert(&action.checkpoint_id) {
                return Err("Step identifiers must be unique");
            }
        }
        let mut checks = std::collections::BTreeSet::new();
        for check in &self.checks {
            if check.id.is_empty()
                || check.id.len() > 100
                || !checks.insert(&check.id)
                || !checkpoints.contains(&check.checkpoint_id)
                || check.description.trim().is_empty()
                || check.description.len() > 1000
                || check.text_filter.len() > 1000
                || check.expected.len() > 1000
                || !(1..=60).contains(&check.observation_seconds)
                || [&check.resource_id, &check.ready_resource_id]
                    .iter()
                    .any(|v| v.len() > 255 || !v.starts_with(&format!("{package}:id/")))
            {
                return Err("Invalid expected result binding");
            }
            if check
                .prerequisite_check_ids
                .iter()
                .any(|id| id == &check.id || !checks.contains(id))
            {
                return Err("Checks must reference earlier prerequisites");
            }
            if (check.method == crate::execution::CheckMethod::UiElementPresenceV1
                || matches!(
                    check.property,
                    crate::execution::UiProperty::Checked | crate::execution::UiProperty::Enabled
                ))
                && !["true", "false"].contains(&check.expected.as_str())
            {
                return Err("Expected Boolean must be true or false");
            }
        }
        Ok(())
    }
    pub fn uses_ai(&self) -> bool {
        self.actions
            .iter()
            .any(|a| a.kind == crate::execution::ActionKind::Navigate)
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PhoneCommandRequest {
    pub id: Uuid,
    pub expected_revision: u32,
    pub frame_id: Option<Uuid>,
    pub title: String,
    pub sequence: AutomationSequence,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum StepState {
    Started,
    Completed,
    Failed,
    Blocked,
    Inconclusive,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StepReceipt {
    pub action_id: String,
    pub state: StepState,
    pub message: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CoverageKind {
    Smoke,
    HappyPath,
    Validation,
    Persistence,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TestTemplate {
    pub id: String,
    pub version: u32,
    pub title: String,
    pub description: String,
    pub category: CoverageKind,
    pub definition: CaseDefinition,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TestTemplates {
    pub items: Vec<TestTemplate>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerateTestsRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<DiscoveryEngine>,
    pub id: Uuid,
    pub session_id: Uuid,
    pub expected_revision: u32,
    pub category: CoverageKind,
    pub journey: String,
    pub allow_writes: bool,
    pub reuse_job_id: Option<Uuid>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum GenerationState {
    Queued,
    Discovering,
    Drafting,
    Ready,
    NeedsInput,
    Failed,
    Canceled,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerationProposal {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path_ids: Vec<Uuid>,
    pub id: Uuid,
    pub title: String,
    pub category: CoverageKind,
    pub sequence: AutomationSequence,
    pub requirement: String,
    pub questions: Vec<String>,
    pub source_ids: Vec<Uuid>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringUsage {
    pub calls: u32,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub unknown_calls: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoverySnapshot {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    pub id: Uuid,
    pub frame: PhoneFrame,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerationProgress {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<DiscoveryEngine>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_job_id: Option<Uuid>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub journal: Vec<DiscoveryReceipt>,
    pub state: GenerationState,
    pub proposals: Vec<GenerationProposal>,
    pub snapshots: Vec<DiscoverySnapshot>,
    pub trace: Vec<DirectCommand>,
    pub gaps: Vec<String>,
    pub usage: AuthoringUsage,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(tag = "decision", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiscoveryDecision {
    Act { command: DirectCommand },
    Finish { reason: String },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ProposalBatch {
    pub proposals: Vec<GenerationProposal>,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AuthoringModelRequest {
    Discover {
        package: String,
        journey: String,
        allow_writes: bool,
        frame: PhoneFrame,
        trace: Vec<DirectCommand>,
    },
    Propose {
        #[serde(default)]
        journal: Vec<DiscoveryReceipt>,
        package: String,
        journey: String,
        category: CoverageKind,
        snapshots: Vec<DiscoverySnapshot>,
        trace: Vec<DirectCommand>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AuthoringModelResponse {
    pub decision: Option<DiscoveryDecision>,
    pub batch: Option<ProposalBatch>,
    pub usage: AuthoringUsage,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveAuthoredTest {
    #[serde(default)]
    pub template_id: Option<String>,
    #[serde(default)]
    pub proposal_id: Option<Uuid>,
    pub title: String,
    pub requirement: String,
    pub sequence: AutomationSequence,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveAuthoredTestsRequest {
    pub mutation_id: Uuid,
    pub tests: Vec<SaveAuthoredTest>,
    pub source_task_id: Option<Uuid>,
    pub expectations_confirmed: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SavedAuthoredTests {
    pub entry_ids: Vec<Uuid>,
}

/// Discovery-only protocol. Absence on historical payloads means the legacy custom loop.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryEngine {
    LegacyCustom,
    MinitapV1,
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryOutcome {
    Pending,
    Completed,
    Failed,
    Uncertain,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryReceipt {
    pub id: Uuid,
    pub before_id: Uuid,
    pub command: DirectCommand,
    pub outcome: DiscoveryOutcome,
    pub after_id: Option<Uuid>,
}
/// Local child/parent IPC: no worker credentials, shell strings or filesystem paths.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum DiscoveryCall {
    Observe {
        id: Uuid,
    },
    Execute {
        id: Uuid,
        command: DirectCommand,
    },
    Reserve {
        id: Uuid,
    },
    Usage {
        id: Uuid,
        input_tokens: Option<u32>,
        output_tokens: Option<u32>,
    },
    Finish {
        id: Uuid,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryReply {
    pub id: Uuid,
    pub error: Option<String>,
    pub snapshot: Option<DiscoverySnapshot>,
}

/// The model selects a prefix and writes expectations; code owns commands and evidence IDs.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryDraft {
    #[schemars(range(min = 1, max = 12))]
    pub through_action: u8,
    #[schemars(length(min = 1, max = 200))]
    pub title: String,
    #[schemars(length(max = 4000))]
    pub requirement: String,
    #[schemars(length(max = 20))]
    pub questions: Vec<String>,
    #[schemars(length(max = 20))]
    pub checks: Vec<DiscoveryDraftCheck>,
}
/// Use an action number at the model boundary; the recorder resolves checkpoint identities.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryDraftCheck {
    #[schemars(range(min = 1, max = 12))]
    pub after_action: u8,
    pub description: String,
    pub method: CheckMethod,
    pub resource_id: String,
    pub ready_resource_id: String,
    pub text_filter: String,
    pub property: UiProperty,
    pub expected: String,
    pub required: bool,
    #[schemars(range(min = 1, max = 30))]
    pub observation_seconds: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryDraftBatch {
    #[schemars(length(min = 1, max = 5))]
    pub proposals: Vec<DiscoveryDraft>,
}
