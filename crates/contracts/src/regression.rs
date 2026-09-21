//! Comparison transport; verdict policy belongs to the API's pure domain module.
use crate::{execution::*, task_sessions::PhoneTask};
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RunSource {
    SavedCaseV1 { case_version_id: Uuid },
    SavedSuiteV1 { suite_version_id: Uuid },
    ReleasePlanV1 { plan_version_id: Uuid },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CommandPurpose {
    Trial,
    Manual,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ComparisonKind {
    Regression,
    StillFailing,
    Recovered,
    Unchanged,
    NewFailure,
    NoBaseline,
    NotComparable,
    Added,
    Removed,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct CaseComparison {
    pub case_key: String,
    pub title: String,
    pub data_variant: String,
    pub kind: ComparisonKind,
    pub reason: String,
    pub baseline_case_id: Option<Uuid>,
    pub current_case_id: Option<Uuid>,
    pub baseline_outcome: Option<Outcome>,
    pub current_outcome: Option<Outcome>,
    pub baseline_checks: Vec<CheckResult>,
    pub current_checks: Vec<CheckResult>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
pub struct RunComparison {
    pub policy_version: u32,
    pub baseline_run_id: Option<Uuid>,
    pub cases: Vec<CaseComparison>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CaseRunRequest {
    pub case_version_id: Uuid,
    pub build_id: Uuid,
    pub profile_id: Uuid,
    pub environment_revision: i32,
    pub baseline_run_id: Option<Uuid>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct BaselineChoice {
    pub id: Uuid,
    pub build_id: Uuid,
    pub build_label: String,
    pub created_at: DateTime<Utc>,
    pub compatible: bool,
    pub reason: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct CaseRunPreview {
    pub blockers: Vec<String>,
    pub environment_revision: i32,
    pub baselines: Vec<BaselineChoice>,
    pub suggested_baseline_id: Option<Uuid>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SuiteRunRequest {
    pub suite_version_id: Uuid,
    pub build_id: Uuid,
    pub profile_id: Uuid,
    pub environment_revision: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub baseline_run_id: Option<Uuid>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct SuiteRunPreview {
    pub blockers: Vec<String>,
    pub environment_revision: i32,
    pub baselines: Vec<BaselineChoice>,
    pub suggested_baseline_id: Option<Uuid>,
}
#[cfg(test)]
mod suite_request_tests {
    use super::*;
    #[test]
    fn omitted_baseline_keeps_old_request_identity() {
        let raw = serde_json::json!({
            "suite_version_id": Uuid::new_v4(), "build_id": Uuid::new_v4(),
            "profile_id": Uuid::new_v4(), "environment_revision": 3
        });
        let old: SuiteRunRequest = serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(old.baseline_run_id, None);
        assert_eq!(serde_json::to_value(&old).unwrap(), raw);
        let with_baseline = SuiteRunRequest {
            baseline_run_id: Some(Uuid::new_v4()),
            ..old
        };
        assert_eq!(
            serde_json::from_value::<SuiteRunRequest>(
                serde_json::to_value(&with_baseline).unwrap()
            )
            .unwrap(),
            with_baseline
        );
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RunHistoryItem {
    pub id: String,
    pub source: String,
    pub created_at: DateTime<Utc>,
    pub build_label: String,
    pub run: Option<RunResponse>,
    pub trial: Option<PhoneTask>,
    pub session_id: Option<Uuid>,
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
pub struct RunHistory {
    pub items: Vec<RunHistoryItem>,
    pub next_cursor: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ObservationKind {
    PresentValue,
    Absent,
}
