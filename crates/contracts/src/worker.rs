//! Shared Rust/Python fixture contract used to verify the source foundation.
//!
//! The Python fake executor reads a chosen scenario and returns its fixed outcome.
//! Nothing here installs an APK, controls a phone, asks a model, or leases a job.
//! Real device qualification belongs to spec 02; the worker HTTP protocol to spec 04
//! in docs/architect/implementation. Keep this fake path for fast, deterministic tests.
//!
//! Rust owns these shapes. `just types` exports JSON Schema and generates Python
//! Pydantic models. Serde handles JSON shape; callers must also invoke the explicit
//! validate methods below for semantic rules that Serde does not enforce.
pub mod execution;
pub mod qualification;

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

// Test input, not an observation: the fixture deliberately selects this outcome.
// The tagged JSON shape is {"kind":"pass"}, {"kind":"fail"}, or {"kind":"blocked"}.
#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Scenario {
    Pass,
    Fail,
    Blocked,
}

// A local fixture command. run_id is echoed for correlation; it is not a DB lookup,
// authorization token, device reservation, or evidence that a real run exists.
#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakeExecutionRequest {
    // Version 1 is the only fixture format. This schema constraint validates the
    // generated Python input, but Rust must call validate() after deserialization.
    #[schemars(range(min = 1, max = 1))]
    pub version: u8,
    pub run_id: Uuid,
    pub scenario: Scenario,
}

impl FakeExecutionRequest {
    /// Reject unsupported fixture versions after Serde has parsed the JSON shape.
    ///
    /// Schemars annotations describe a schema; they do not execute Rust validation.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.version != 1 {
            return Err("unsupported fixture protocol version");
        }
        Ok(())
    }
}

// Only outcomes produced by the fake executor. This is deliberately smaller than
// the planned production vocabulary (which also includes inconclusive/canceled/etc.).
#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Passed,
    Failed,
    Blocked,
}

// A simulated result, never proof that an app passed a customer test.
// The fake executor constructs version 1; this type has no separate validate method.
#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FakeExecutionResult {
    #[schemars(range(min = 1, max = 1))]
    pub version: u8,
    pub run_id: Uuid,
    pub outcome: Outcome,
    pub message: String,
}

// Keep the generated schema nullable even though the field itself is required.
// Both rules are needed so Pydantic distinguishes missing from explicit JSON null.
fn nullable_string_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
    <Option<String> as JsonSchema>::json_schema(generator)
}

// A normal Option accepts a missing field. A custom deserializer without a default
// makes Serde require the key while still accepting either a string or explicit null.
fn required_nullable<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(d)
}

// Serialization test data only; the fake executor does not execute this object.
// Shared fixtures exercise the easy-to-break Rust/Python boundary cases below.
#[derive(Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContractProbe {
    pub run_id: Uuid,
    // Wire timestamps are RFC3339 strings; Rust represents their instant in UTC.
    pub observed_at: DateTime<Utc>,
    pub scenario: Scenario,
    // Required key: {"nullable_note":null} is valid; omitting nullable_note is not.
    #[serde(deserialize_with = "required_nullable")]
    #[schemars(schema_with = "nullable_string_schema", required)]
    pub nullable_note: Option<String>,
    // Optional key: missing and null both deserialize to None. Rust omits None
    // when writing JSON, rather than adding a key the sender may never have supplied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional_note: Option<String>,
    // Decimal text preserves large values across JSON consumers, including JavaScript
    // numbers above 2^53 - 1. No signs, whitespace, fractions, or leading zeroes.
    #[schemars(regex(pattern = "^(0|[1-9][0-9]*)$"))]
    pub counter: String,
}

impl ContractProbe {
    /// Enforce the same decimal spelling promised by the generated schema.
    /// Call after deserialization: a Rust String alone permits arbitrary text.
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

// Schema-export root that makes all fixture definitions reachable by Schemars.
// It is not a message envelope sent to a worker and is never executed as a job.
#[derive(JsonSchema)]
pub struct WorkerContracts {
    pub phone_claim: crate::task_sessions::PhoneClaimResponse,
    pub phone_claim_request: crate::task_sessions::PhoneClaimRequest,
    pub phone_update: crate::task_sessions::PhoneUpdate,
    pub execution: execution::ExecutionContracts,
    pub request: FakeExecutionRequest,
    pub result: FakeExecutionResult,
    pub probe: ContractProbe,
    pub qualification: qualification::QualificationContracts,
}
