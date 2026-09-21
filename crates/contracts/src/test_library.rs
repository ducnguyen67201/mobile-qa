//! Browser authoring workspace. Drafts deliberately permit missing semantic content;
//! a complete save also records an immutable execution definition.
use crate::execution::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum LibraryIssueCode {
    Required,
    InvalidContent,
    InvalidReference,
    ConflictingSelection,
    UnsupportedCapability,
    Archived,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryIssue {
    pub code: LibraryIssueCode,
    pub field: String,
    pub item_id: Option<String>,
    pub message: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum LibraryErrorDetails {
    Validation {
        issues: Vec<LibraryIssue>,
    },
    StaleRevision {
        entry_id: Option<Uuid>,
        current_revision: i32,
    },
    IdempotencyConflict {
        mutation_id: Uuid,
    },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PlanDraftContent {
    pub key: String,
    pub version: u32,
    pub title: String,
    pub suite_version_ids: Vec<Uuid>,
    pub cases: Vec<CaseSelection>,
    pub profile_id: Option<Uuid>,
    pub budget: ExecutionBudget,
    pub diagnostic_retries: u8,
    pub exclusions: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(
    tag = "kind",
    content = "content",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum LibraryDraftDefinition {
    Case(CaseDefinition),
    Suite(SuiteDefinition),
    Plan(PlanDraftContent),
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryCapabilities {
    pub can_edit: bool,
    pub can_archive: bool,
    pub can_set_default: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryEntryResponse {
    pub id: Uuid,
    pub app_id: Uuid,
    pub kind: DefinitionKind,
    pub key: String,
    pub title: String,
    /// Derived from saved proposal provenance; remains true after editing the draft.
    #[serde(default)]
    pub ai_generated: bool,
    /// Current saved editor content still has semantic or reference setup issues.
    pub needs_setup: bool,
    pub revision: i32,
    pub archived_at: Option<DateTime<Utc>>,
    pub draft_version: Option<u32>,
    pub latest_version_id: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
    pub capabilities: LibraryCapabilities,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryCoveragePreview {
    pub cases: Vec<ResolvedCase>,
    pub required_count: u32,
    pub exclusions: Vec<String>,
    pub issues: Vec<LibraryIssue>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryDraftResponse {
    pub entry: LibraryEntryResponse,
    pub definition: LibraryDraftDefinition,
    pub source_version_id: Option<Uuid>,
    /// Snapshot matching this exact saved editor content, never a previous runnable edit.
    pub saved_version_id: Option<Uuid>,
    pub issues: Vec<LibraryIssue>,
    pub coverage: LibraryCoveragePreview,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryVersionResponse {
    pub entry: LibraryEntryResponse,
    pub version: DefinitionResponse,
    pub coverage: LibraryCoveragePreview,
    pub issues: Vec<LibraryIssue>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryListResponse {
    pub items: Vec<LibraryEntryResponse>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryVersionListResponse {
    pub items: Vec<LibraryVersionResponse>,
    pub next_cursor: Option<Uuid>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryProfileChoice {
    pub id: Uuid,
    pub name: String,
    pub driver: Driver,
    pub package: String,
    pub adapter: String,
    pub qualified: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_model: Option<crate::model_registry::ResolvedModel>,
    pub model_available: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct LibraryOptionsResponse {
    pub profiles: Vec<LibraryProfileChoice>,
    pub saved_versions: Vec<LibraryVersionResponse>,
    pub capabilities: LibraryCapabilities,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DefaultPlanResponse {
    pub app_id: Uuid,
    pub revision: i32,
    pub plan_version_id: Option<Uuid>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateLibraryEntryRequest {
    pub mutation_id: Uuid,
    pub entry_id: Uuid,
    pub kind: DefinitionKind,
    pub key: String,
    pub template_profile_id: Option<Uuid>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveLibraryDraftRequest {
    pub mutation_id: Uuid,
    pub expected_revision: i32,
    pub definition: LibraryDraftDefinition,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ArchiveLibraryEntryRequest {
    pub mutation_id: Uuid,
    pub expected_revision: i32,
    pub archived: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetDefaultPlanRequest {
    pub mutation_id: Uuid,
    pub expected_revision: i32,
    pub plan_version_id: Uuid,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub struct LibraryListQuery {
    pub kind: Option<DefinitionKind>,
    pub archived: Option<bool>,
    pub cursor: Option<Uuid>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub struct LibraryVersionQuery {
    pub cursor: Option<Uuid>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema, IntoParams)]
#[into_params(parameter_in = Query)]
#[serde(deny_unknown_fields)]
pub struct ExecutionPlanQuery {
    pub build_id: Uuid,
    pub plan_version_id: Option<Uuid>,
}

/// Persisted receipts remain a closed Rust-owned union, never arbitrary consumer JSON.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "response", rename_all = "snake_case")]
pub enum LibraryMutationReceipt {
    Authored(crate::automation::SavedAuthoredTests),
    Draft(Box<LibraryDraftResponse>),
    Entry(LibraryEntryResponse),
    Default(DefaultPlanResponse),
}
impl LibraryIssue {
    pub fn new(code: LibraryIssueCode, field: &str, message: impl Into<String>) -> Self {
        Self {
            code,
            field: field.into(),
            item_id: None,
            message: message.into(),
        }
    }
}
impl LibraryDraftDefinition {
    pub fn identity(&self) -> (DefinitionKind, &str, u32) {
        match self {
            Self::Case(c) => (DefinitionKind::Case, &c.key, c.version),
            Self::Suite(s) => (DefinitionKind::Suite, &s.key, s.version),
            Self::Plan(p) => (DefinitionKind::Plan, &p.key, p.version),
        }
    }
    pub fn title(&self) -> &str {
        match self {
            Self::Case(c) => &c.title,
            Self::Suite(s) => &s.title,
            Self::Plan(p) => &p.title,
        }
    }
    pub fn allocate(&mut self, version: u32) {
        match self {
            Self::Case(c) => {
                c.version = version;
            }
            Self::Suite(s) => s.version = version,
            Self::Plan(p) => p.version = version,
        }
    }
    pub fn published(&self) -> Result<TestDefinition, LibraryIssue> {
        Ok(match self {
            Self::Case(c) => {
                let mut executable = c.clone();
                // Detailed source provenance belongs to the editor; execution has a closed origin vocabulary.
                executable.provenance = "user_authored".into();
                TestDefinition::Case(executable)
            }
            Self::Suite(s) => TestDefinition::Suite(s.clone()),
            Self::Plan(p) => TestDefinition::Plan(PlanDefinition {
                key: p.key.clone(),
                version: p.version,
                title: p.title.clone(),
                suite_version_ids: p.suite_version_ids.clone(),
                cases: p.cases.clone(),
                profile_id: p.profile_id.ok_or_else(|| {
                    LibraryIssue::new(
                        LibraryIssueCode::Required,
                        "profile_id",
                        "Choose an execution profile",
                    )
                })?,
                budget: p.budget.clone(),
                diagnostic_retries: p.diagnostic_retries,
                exclusions: p.exclusions.clone(),
            }),
        })
    }
    /// Structural limits apply even to incomplete saves; semantic errors become visible issues.
    pub fn check_bounds(&self) -> Result<(), &'static str> {
        let (_, key, version) = self.identity();
        if key.len() > 100 || version == 0 || version > i32::MAX as u32 || self.title().len() > 200
        {
            return Err("Draft identity or title exceeds limits");
        }
        let oversized = match self {
            Self::Case(c) => {
                c.requirement.len() > 4000
                    || c.package.len() > 255
                    || c.adapter.len() > 100
                    || c.provenance.len() > 100
                    || c.preconditions.len() > 20
                    || c.preconditions.iter().any(|s| s.len() > 1000)
                    || c.actions.len() > 20
                    || c.checks.len() > 20
                    || c.actions.iter().any(|a| {
                        a.id.len() > 100
                            || a.checkpoint_id.len() > 100
                            || a.instruction.len() > 4000
                            || a.command.as_ref().is_some_and(|c| {
                                serde_json::to_vec(c).map_or(true, |v| v.len() > 8192)
                            })
                    })
                    || c.checks.iter().any(|c| {
                        c.id.len() > 100
                            || c.checkpoint_id.len() > 100
                            || c.description.len() > 1000
                            || c.resource_id.len() > 255
                            || c.ready_resource_id.len() > 255
                            || c.expected.len() > 1000
                            || c.text_filter.len() > 1000
                            || c.prerequisite_check_ids.len() > 20
                            || c.prerequisite_check_ids.iter().any(|id| id.len() > 100)
                    })
            }
            Self::Suite(s) => {
                s.cases.len() > 100 || s.cases.iter().any(|c| c.data_variant.len() > 100)
            }
            Self::Plan(p) => {
                p.cases.len() + p.suite_version_ids.len() > 100
                    || p.cases.iter().any(|c| c.data_variant.len() > 100)
                    || p.exclusions.len() > 100
                    || p.exclusions.iter().any(|s| s.len() > 1000)
            }
        };
        if oversized {
            Err("Draft fields or list sizes exceed limits")
        } else {
            Ok(())
        }
    }
    pub fn issues(&self) -> Vec<LibraryIssue> {
        let mut issues = Vec::new();
        let mut required = |field, message| {
            issues.push(LibraryIssue::new(
                LibraryIssueCode::Required,
                field,
                message,
            ))
        };
        if self.title().trim().is_empty() {
            required("title", "Give this test a descriptive title");
        }
        match self {
            Self::Case(c) => {
                if c.requirement.trim().is_empty() {
                    required("requirement", "Describe the behavior this test protects");
                }
                if c.actions.is_empty() {
                    required("actions", "Add at least one action");
                }
                if !c.checks.iter().any(|c| c.required) {
                    required("checks", "Add at least one required expected check");
                }
            }
            Self::Suite(s) if s.cases.is_empty() => required("cases", "Choose saved case versions"),
            Self::Plan(p) => {
                if p.profile_id.is_none() {
                    required("profile_id", "Choose an execution profile");
                }
                if p.cases.is_empty() && p.suite_version_ids.is_empty() {
                    required("cases", "Choose saved coverage");
                }
            }
            _ => {}
        }
        if let Self::Case(c) = self {
            let mut earlier = std::collections::BTreeSet::new();
            for action in &c.actions {
                if action.kind == ActionKind::Direct && action.validate(&c.package).is_err() {
                    issues.push(LibraryIssue::new(
                        LibraryIssueCode::Required,
                        "actions.command",
                        "Pick a control for each direct step",
                    ));
                }
                if action.kind == ActionKind::Navigate && action.instruction.trim().is_empty() {
                    issues.push(LibraryIssue {
                        code: LibraryIssueCode::Required,
                        field: "actions.instruction".into(),
                        item_id: Some(action.id.clone()),
                        message: "Describe what the agent should do".into(),
                    });
                }
            }
            for check in &c.checks {
                let mut add = |field: &str, message: &str| {
                    issues.push(LibraryIssue {
                        code: LibraryIssueCode::InvalidContent,
                        field: field.into(),
                        item_id: Some(check.id.clone()),
                        message: message.into(),
                    })
                };
                if check.description.trim().is_empty() {
                    add("checks.description", "Describe the expected result");
                }
                if !c
                    .actions
                    .iter()
                    .any(|a| a.checkpoint_id == check.checkpoint_id)
                {
                    add(
                        "checks.checkpoint_id",
                        "Choose an existing action checkpoint",
                    );
                }
                if check
                    .prerequisite_check_ids
                    .iter()
                    .any(|id| !earlier.contains(id))
                {
                    add(
                        "checks.prerequisite_check_ids",
                        "Prerequisites must refer to earlier checks",
                    );
                }
                if [&check.resource_id, &check.ready_resource_id]
                    .iter()
                    .any(|id| !id.starts_with(&format!("{}:id/", c.package)))
                {
                    add(
                        "checks.resource_id",
                        "Set evidence targets belonging to this Android package",
                    );
                }
                earlier.insert(check.id.clone());
            }
        }
        if let Ok(definition) = self.published() {
            if let Err(message) = definition.validate() {
                issues.push(LibraryIssue::new(
                    LibraryIssueCode::InvalidContent,
                    "definition",
                    message,
                ));
            }
        }
        issues.truncate(100);
        issues
    }
}
impl From<TestDefinition> for LibraryDraftDefinition {
    fn from(value: TestDefinition) -> Self {
        match value {
            TestDefinition::Case(c) => Self::Case(c),
            TestDefinition::Suite(s) => Self::Suite(s),
            TestDefinition::Plan(p) => Self::Plan(PlanDraftContent {
                key: p.key,
                version: p.version,
                title: p.title,
                suite_version_ids: p.suite_version_ids,
                cases: p.cases,
                profile_id: Some(p.profile_id),
                budget: p.budget,
                diagnostic_retries: p.diagnostic_retries,
                exclusions: p.exclusions,
            }),
        }
    }
}
