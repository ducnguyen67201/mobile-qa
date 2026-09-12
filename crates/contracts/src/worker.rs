//! Fixture-only protocol, not a production worker lease API.
use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Scenario {
    Pass,
    Fail,
    Blocked,
}
#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakeExecutionRequest {
    #[schemars(range(min = 1, max = 1))]
    pub version: u8,
    pub run_id: Uuid,
    pub scenario: Scenario,
}
impl FakeExecutionRequest {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != 1 {
            return Err("unsupported fixture protocol version");
        }
        Ok(())
    }
}
#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Passed,
    Failed,
    Blocked,
}
#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakeExecutionResult {
    #[schemars(range(min = 1, max = 1))]
    pub version: u8,
    pub run_id: Uuid,
    pub outcome: Outcome,
    pub message: String,
}
fn nullable_string_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    <Option<String> as JsonSchema>::json_schema(generator)
}
fn required_nullable<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)
}
#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContractProbe {
    pub run_id: Uuid,
    pub observed_at: DateTime<Utc>,
    pub scenario: Scenario,
    #[serde(deserialize_with = "required_nullable")]
    #[schemars(schema_with = "nullable_string_schema", required)]
    pub nullable_note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional_note: Option<String>,
    #[schemars(regex(pattern = "^(0|[1-9][0-9]*)$"))]
    pub counter: String,
}
impl ContractProbe {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.counter.is_empty()
            || !self.counter.bytes().all(|c| c.is_ascii_digit())
            || (self.counter.len() > 1 && self.counter.starts_with('0'))
        {
            return Err("counter must be a canonical nonnegative decimal string");
        }
        Ok(())
    }
}
#[derive(JsonSchema)]
pub struct WorkerContracts {
    pub request: FakeExecutionRequest,
    pub result: FakeExecutionResult,
    pub probe: ContractProbe,
}
