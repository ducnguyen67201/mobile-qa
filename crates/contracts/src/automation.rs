//! Structured device commands and bounded authoring. Rust owns every consumer shape.
use crate::execution::{CaseDefinition, ExpectedCheck, TestAction};
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
    pub id: Uuid,
    pub frame: PhoneFrame,
}
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct GenerationProgress {
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
