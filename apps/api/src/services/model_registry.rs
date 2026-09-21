//! Authoritative model resolution. This module is the only legacy-name mapper.
use super::execution_store::*;
use crate::errors::{ApiFailure, ApiResult};
use mobile_qa_contracts::model_registry::{
    ModelBinding, ModelCapability, ModelDefinition, ModelProvider, ModelReference, ResolvedModel,
    WorkerModelCapabilities,
};
use sea_orm::{ConnectionTrait, TransactionSession, TransactionTrait};
use uuid::Uuid;

/// Poll advertisements are committed independently of a lease transaction: an idle
/// worker must become visible in phone options before the first session exists.
pub async fn advertise(
    db: &impl ConnectionTrait,
    worker: Uuid,
    capabilities: Option<&WorkerModelCapabilities>,
) -> ApiResult<()> {
    exec(
        db,
        "UPDATE execution_workers SET model_capabilities=$2,model_last_seen_at=now() WHERE id=$1 AND revoked=false",
        vec![worker.into(), json(&capabilities)?.into()],
    )
    .await?;
    Ok(())
}

pub async fn register(
    db: &(impl ConnectionTrait + TransactionTrait),
    definition: &ModelDefinition,
) -> ApiResult<ResolvedModel> {
    definition.validate().map_err(ApiFailure::invalid)?;
    let payload = json(definition)?;
    let tx = db.begin().await?;
    let existing = rows(
        &tx,
        "SELECT payload FROM model_definitions WHERE key=$1 AND revision=$2 FOR UPDATE",
        vec![
            definition.reference.key.clone().into(),
            (definition.reference.revision as i32).into(),
        ],
    )
    .await?;
    if let Some(existing) = existing.first() {
        if field::<serde_json::Value>(existing, "payload")? != payload {
            return Err(conflict(
                "Model revisions are immutable; register a new revision",
            ));
        }
    } else {
        exec(
            &tx,
            "INSERT INTO model_definitions(key,revision,payload) VALUES($1,$2,$3)",
            vec![
                definition.reference.key.clone().into(),
                (definition.reference.revision as i32).into(),
                payload.into(),
            ],
        )
        .await?;
    }
    tx.commit().await?;
    Ok(definition.resolve())
}

pub async fn retire(db: &impl ConnectionTrait, reference: &ModelReference) -> ApiResult<()> {
    reference.validate().map_err(ApiFailure::invalid)?;
    if exec(
        db,
        "UPDATE model_definitions SET retired_at=COALESCE(retired_at,now()) WHERE key=$1 AND revision=$2",
        vec![reference.key.clone().into(), (reference.revision as i32).into()],
    )
    .await?
        == 0
    {
        return Err(ApiFailure::missing());
    }
    Ok(())
}

pub async fn resolve(
    db: &impl ConnectionTrait,
    reference: &ModelReference,
) -> ApiResult<(ResolvedModel, bool)> {
    reference.validate().map_err(ApiFailure::invalid)?;
    let row = one(
        db,
        "SELECT payload,retired_at IS NOT NULL AS retired FROM model_definitions WHERE key=$1 AND revision=$2",
        vec![reference.key.clone().into(), (reference.revision as i32).into()],
    )
    .await?;
    let definition: ModelDefinition = decode(field(&row, "payload")?)?;
    definition.validate().map_err(ApiFailure::invalid)?;
    Ok((definition.resolve(), field(&row, "retired")?))
}

pub async fn resolve_for_new_work(
    db: &impl ConnectionTrait,
    binding: Option<&ModelBinding>,
    required: &[ModelCapability],
) -> ApiResult<Option<ResolvedModel>> {
    let resolved = match binding {
        None => return Ok(None),
        Some(ModelBinding::Registered(reference)) => {
            let (resolved, retired) = resolve(db, reference).await?;
            if retired {
                return Err(ApiFailure::new(
                    409,
                    "model_definition_retired",
                    "The configured model revision is retired",
                ));
            }
            resolved
        }
        Some(ModelBinding::Legacy(value)) if value.trim().is_empty() || value == "none" => {
            return Ok(None)
        }
        Some(ModelBinding::Legacy(value)) => resolve_legacy(db, value).await?,
    };
    if required.iter().any(|capability| !resolved.has(*capability)) {
        return Err(ApiFailure::new(
            409,
            "model_capability_unavailable",
            "The configured model does not support this operation",
        ));
    }
    Ok(Some(resolved))
}

async fn resolve_legacy(
    db: &impl ConnectionTrait,
    provider_model: &str,
) -> ApiResult<ResolvedModel> {
    let matches = rows(
        db,
        "SELECT payload FROM model_definitions WHERE retired_at IS NULL AND payload->>'provider'=$1 AND payload->>'provider_model'=$2 ORDER BY key,revision",
        vec!["open_ai".into(), provider_model.into()],
    )
    .await?;
    match matches.as_slice() {
        [] => Err(ApiFailure::new(
            409,
            "legacy_model_unmapped",
            "The historical model name is not registered",
        )),
        [row] => {
            let definition: ModelDefinition = decode(field(row, "payload")?)?;
            Ok(definition.resolve())
        }
        _ => Err(ApiFailure::new(
            409,
            "legacy_model_ambiguous",
            "The historical model name matches multiple active definitions",
        )),
    }
}

pub fn supports_provider(provider: ModelProvider) -> bool {
    matches!(provider, ModelProvider::OpenAi)
}

pub fn worker_matches(
    resolved: &ResolvedModel,
    advertised: Option<&WorkerModelCapabilities>,
) -> bool {
    advertised.is_some_and(|advertised| {
        advertised.model.as_ref() == Some(&resolved.reference)
            && advertised.providers.contains(&resolved.provider)
            && supports_provider(resolved.provider)
    })
}
