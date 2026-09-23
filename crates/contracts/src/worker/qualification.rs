//! Operator-only Android experiment. These messages are not production job leases.
//! Serde callers must invoke validate(); generated Python validates the JSON boundary.
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QualificationRequest {
    #[schemars(range(min = 1, max = 1))]
    pub version: u8,
    pub attempt_id: Uuid,
    #[schemars(regex(pattern = "^persist-task-v1$"))]
    pub case_id: String,
    pub apk_path: String,
    #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
    pub expected_apk_sha256: String,
    #[schemars(regex(pattern = "^ai\\.mobileqa\\.demo$"))]
    pub package: String,
    #[schemars(regex(pattern = "^ai\\.mobileqa\\.demo/\\.MainActivity$"))]
    pub activity: String,
    #[schemars(regex(pattern = "^emulator-5554$"))]
    pub serial: String,
    pub profile_path: String,
    pub output_root: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_model: Option<crate::model_registry::ResolvedModel>,
}

impl QualificationRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != 1
            || self.case_id != "persist-task-v1"
            || self.package != "ai.mobileqa.demo"
            || self.activity != "ai.mobileqa.demo/.MainActivity"
            || self.serial != "emulator-5554"
            || !hash_valid(&self.expected_apk_sha256)
            || [&self.apk_path, &self.profile_path, &self.output_root]
                .iter()
                .any(|s| s.is_empty())
        {
            return Err("invalid qualification request");
        }
        if let Some(model) = &self.resolved_model {
            model.validate()?;
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QualificationOutcome {
    Passed,
    Failed,
    Blocked,
    Inconclusive,
}
#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QualificationReset {
    VerifiedClean,
    Quarantined,
    NotStarted,
}

// Missing evidence is a separate variant: it cannot accidentally advertise a file.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum QualificationArtifact {
    Available {
        name: String,
        path: String,
        mime: String,
        #[schemars(range(min = 1, max = 2147483648_u32))]
        bytes: u32,
        #[schemars(regex(pattern = "^[0-9a-f]{64}$"))]
        sha256: String,
    },
    Unavailable {
        name: String,
        reason: String,
    },
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QualificationUsage {
    pub model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_reference: Option<crate::model_registry::ModelReference>,
    #[schemars(range(min = 0, max = 10000))]
    pub calls: u32,
    #[schemars(range(min = 0, max = 10000))]
    pub unknown_calls: u32,
    // None means the provider did not report usage, never a measured zero.
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct QualificationResult {
    #[schemars(range(min = 1, max = 1))]
    pub version: u8,
    pub attempt_id: Uuid,
    pub case_id: String,
    pub outcome: QualificationOutcome,
    pub reason_code: String,
    pub expected_behavior: String,
    pub observed_behavior: String,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub requested_build_sha256: String,
    pub observed_build_sha256: Option<String>,
    pub device_inventory: std::collections::BTreeMap<String, String>,
    pub model_profile_sha256: String,
    pub reset: QualificationReset,
    pub phase_ms: std::collections::BTreeMap<String, u32>,
    pub artifacts: Vec<QualificationArtifact>,
    pub usage: Vec<QualificationUsage>,
}
impl QualificationResult {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != 1
            || self.case_id != "persist-task-v1"
            || self.ended_at < self.started_at
            || !hash_valid(&self.requested_build_sha256)
            || !hash_valid(&self.model_profile_sha256)
            || self
                .observed_build_sha256
                .as_ref()
                .is_some_and(|h| !hash_valid(h))
        {
            return Err("invalid result metadata");
        }
        for a in &self.artifacts {
            if let QualificationArtifact::Available {
                path,
                bytes,
                sha256,
                ..
            } = a
            {
                if *bytes == 0
                    || *bytes > 2147483648
                    || !hash_valid(sha256)
                    || path.is_empty()
                    || path.starts_with('/')
                    || path.contains('\\')
                    || path.split('/').any(|p| p == ".." || p.is_empty())
                {
                    return Err("invalid artifact");
                }
            }
        }
        if self
            .usage
            .iter()
            .any(|u| u.calls > 10000 || u.unknown_calls > u.calls)
        {
            return Err("invalid usage");
        }
        if matches!(
            self.outcome,
            QualificationOutcome::Passed | QualificationOutcome::Failed
        ) {
            for required in ["created.png", "created.xml", "reopened.png", "reopened.xml"] {
                if !self.artifacts.iter().any(|a| matches!(a, QualificationArtifact::Available { name, .. } if name == required)) {
                    return Err("decisive evidence missing");
                }
            }
            if self.observed_build_sha256.as_ref() != Some(&self.requested_build_sha256) {
                return Err("build not verified");
            }
        }
        Ok(())
    }
}
fn hash_valid(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
#[derive(JsonSchema)]
pub struct QualificationContracts {
    pub request: QualificationRequest,
    pub result: QualificationResult,
}
