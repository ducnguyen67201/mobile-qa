//! Immutable qualification context and clean-start attestations. No device or DB dependencies.
use crate::execution::*;
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExecutionContextV1 {
    pub schema_version: u8,
    pub adapter_revision: String,
    pub verifier_revision: String,
    pub worker_runtime_revision: String,
    pub reset_policy_hash: String,
    pub qualified_profile_id: Uuid,
    pub package: String,
    pub launch_component: String,
    pub image: String,
    pub abi: String,
    pub width: u32,
    pub height: u32,
    pub density: u32,
    pub locale: String,
    pub timezone: String,
    pub state_scope: String,
    pub qualification_reference: String,
    pub starting_checks: Vec<ExpectedCheck>,
    pub stages: StageBudgets,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct StageBudgets {
    pub boot_seconds: u32,
    pub install_seconds: u32,
    pub start_seconds: u32,
    pub cleanup_seconds: u32,
}
impl StageBudgets {
    pub fn overhead(&self) -> u64 {
        u64::from(self.boot_seconds)
            + u64::from(self.install_seconds)
            + u64::from(self.start_seconds)
            + u64::from(self.cleanup_seconds)
    }
}
impl ExecutionContextV1 {
    pub fn validate(&self, profile: &ExecutionProfile) -> Result<(), &'static str> {
        if self.schema_version != 1
            || self.adapter_revision != "android_direct_v1"
            || self.verifier_revision != "ui_v1"
            || self.worker_runtime_revision != "direct_v1"
            || self.reset_policy_hash.len() != 64
            || !self
                .reset_policy_hash
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            || self.qualified_profile_id != profile.id
            || self.package != profile.package
            || self.image != profile.image
            || self.qualification_reference != profile.qualification_reference
            || self.state_scope != "local_only"
            || self.width != 1080
            || self.height != 1920
            || self.density != 420
            || self.locale != "en-US"
            || self.timezone != "Etc/UTC"
            || !["arm64-v8a", "x86_64"].contains(&self.abi.as_str())
            || self.image != format!("system-images;android-35;google_apis;{}", self.abi)
            || !self
                .launch_component
                .starts_with(&format!("{}/", self.package))
            || !bounded(&self.launch_component, 500)
            || !self
                .launch_component
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._/$".contains(&b))
            || self.launch_component.split('/').count() != 2
            || self.launch_component.ends_with('/')
            || self.starting_checks.is_empty()
            || self.starting_checks.len() > 10
            || [
                self.stages.boot_seconds,
                self.stages.install_seconds,
                self.stages.start_seconds,
                self.stages.cleanup_seconds,
            ]
            .iter()
            .any(|v| !(1..=600).contains(v))
        {
            return Err("Clean-start qualification does not match the app profile");
        }
        // Reuse ordinary assertion validation, including package boundaries and prerequisite order.
        let case = CaseDefinition {
            key: "start".into(),
            version: 1,
            title: "Clean start".into(),
            requirement: "Qualified starting state".into(),
            provenance: "operator_authored".into(),
            package: self.package.clone(),
            adapter: profile.adapter.clone(),
            preconditions: vec![],
            actions: vec![TestAction {
                id: "start".into(),
                kind: ActionKind::Checkpoint,
                instruction: String::new(),
                checkpoint_id: "preflight".into(),
                command: None,
            }],
            checks: self.starting_checks.clone(),
            budget: ExecutionBudget {
                duration_seconds: 600,
                max_steps: 20,
                artifact_bytes: 16777216,
            },
        };
        TestDefinition::Case(case).validate()?;
        if self.starting_checks.iter().any(|c| {
            c.method == CheckMethod::Manual
                || !c.required
                || c.expected.contains("${")
                || c.text_filter.contains("${")
        }) {
            return Err("Clean start requires fixed automatic assertions");
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PreflightRequest {
    pub generation: i32,
    pub receipt: PreflightReceipt,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PreflightReceipt {
    pub attempt_id: Uuid,
    pub instance_nonce: Uuid,
    pub context: ExecutionContextV1,
    pub build_sha256: String,
    pub started_at: DateTime<Utc>,
    pub ready_at: DateTime<Utc>,
    pub duration_ms: u32,
    pub artifact_ids: Vec<Uuid>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PreflightAcknowledgement {
    pub attempt_id: Uuid,
    pub generation: i32,
    pub accepted: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecoveryEvent {
    pub actor_id: Uuid,
    pub evidence_reference: String,
    pub created_at: DateTime<Utc>,
}
