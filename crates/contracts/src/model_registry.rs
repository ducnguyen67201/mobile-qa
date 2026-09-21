//! Nonsecret, versioned model identity shared by API, workers, and browser clients.
//!
//! `(key, revision)` is the stable identity. Provider-facing model names and labels
//! are resolved snapshots; callers must never use them as registry keys.
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use utoipa::ToSchema;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelReference {
    pub key: String,
    pub revision: u32,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ModelProvider {
    OpenAi,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, ToSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    MinitapNavigation,
    StructuredAuthoring,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ModelDefinition {
    pub reference: ModelReference,
    pub display_name: String,
    pub provider: ModelProvider,
    pub provider_model: String,
    pub capabilities: Vec<ModelCapability>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolvedModel {
    pub reference: ModelReference,
    pub display_name: String,
    pub provider: ModelProvider,
    pub provider_model: String,
    pub capabilities: Vec<ModelCapability>,
}

/// Historical profiles used a raw provider-model string. Consumers may read that
/// shape, but API registration rejects it for every new profile.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(untagged)]
pub enum ModelBinding {
    Registered(ModelReference),
    Legacy(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct WorkerModelCapabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelReference>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub providers: Vec<ModelProvider>,
}

impl ModelReference {
    pub fn validate(&self) -> Result<(), &'static str> {
        let bytes = self.key.as_bytes();
        if self.revision == 0
            || bytes.is_empty()
            || bytes.len() > 100
            || !bytes[0].is_ascii_lowercase() && !bytes[0].is_ascii_digit()
            || bytes.iter().any(|b| {
                !(b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'.' | b'_' | b'-'))
            })
        {
            return Err("invalid model reference");
        }
        Ok(())
    }
}

impl ModelDefinition {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.reference.validate()?;
        validate_resolved_fields(&self.display_name, &self.provider_model, &self.capabilities)
    }

    pub fn resolve(&self) -> ResolvedModel {
        ResolvedModel {
            reference: self.reference.clone(),
            display_name: self.display_name.clone(),
            provider: self.provider,
            provider_model: self.provider_model.clone(),
            capabilities: self.capabilities.clone(),
        }
    }
}

impl ResolvedModel {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.reference.validate()?;
        validate_resolved_fields(&self.display_name, &self.provider_model, &self.capabilities)
    }

    pub fn has(&self, capability: ModelCapability) -> bool {
        self.capabilities.contains(&capability)
    }
}

fn validate_resolved_fields(
    display_name: &str,
    provider_model: &str,
    capabilities: &[ModelCapability],
) -> Result<(), &'static str> {
    if !crate::execution::bounded(display_name, 200)
        || !crate::execution::bounded(provider_model, 200)
        || capabilities.is_empty()
        || capabilities.len() > 8
        || capabilities.iter().collect::<BTreeSet<_>>().len() != capabilities.len()
    {
        return Err("invalid model definition");
    }
    Ok(())
}
