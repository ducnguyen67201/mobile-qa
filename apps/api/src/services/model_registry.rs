//! Authoritative model resolution. This module is the only legacy-name mapper.
use super::execution_store::*;
use crate::errors::{ApiFailure, ApiResult};
use mobile_qa_contracts::model_registry::{
    ModelAssignment, ModelAssignmentState, ModelBinding, ModelCapability, ModelDefinition,
    ModelProvider, ModelPurpose, ModelReference, ResolvedModel, WorkerModelCapabilities,
};
use sea_orm::{ConnectionTrait, TransactionSession, TransactionTrait};
use uuid::Uuid;

/// Poll advertisements are committed independently of a lease transaction: an idle
/// worker must become visible before the first job exists. Keep claim protocols
/// separate because a phone poll cannot prove execution claim eligibility.
pub async fn advertise_execution(
    db: &impl ConnectionTrait,
    worker: Uuid,
    version: u8,
    capabilities: Option<&WorkerModelCapabilities>,
) -> ApiResult<()> {
    exec(
        db,
        "UPDATE execution_workers SET model_capabilities=$2,model_last_seen_at=now(),execution_model_capabilities=$2,execution_last_seen_at=now(),execution_protocol_version=$3 WHERE id=$1 AND revoked=false",
        vec![worker.into(), json(&capabilities)?.into(), (version as i32).into()],
    )
    .await?;
    Ok(())
}

pub async fn advertise_phone(
    db: &impl ConnectionTrait,
    worker: Uuid,
    version: u32,
    capabilities: Option<&WorkerModelCapabilities>,
) -> ApiResult<()> {
    exec(
        db,
        "UPDATE execution_workers SET model_capabilities=$2,model_last_seen_at=now(),phone_model_capabilities=$2,phone_last_seen_at=now(),phone_protocol_version=$3 WHERE id=$1 AND revoked=false",
        vec![worker.into(), json(&capabilities)?.into(), (version as i32).into()],
    )
    .await?;
    Ok(())
}

pub async fn register(
    db: &(impl ConnectionTrait + TransactionTrait),
    actor: Uuid,
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
                "INSERT INTO model_definitions(key,revision,payload,registered_by,payload_sha256) VALUES($1,$2,$3,$4,$5)",
            vec![
                definition.reference.key.clone().into(),
                (definition.reference.revision as i32).into(),
                payload.into(),
                actor.into(),
                hash(serde_json::to_vec(definition).map_err(|_| ApiFailure::internal())?).into(),
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
        (advertised.model.as_ref() == Some(&resolved.reference)
            || advertised.models.contains(&resolved.reference))
            && advertised.providers.contains(&resolved.provider)
            && supports_provider(resolved.provider)
    })
}

/// The SQL claim predicate uses the same exact reference/provider admission as
/// `worker_matches`; direct-only work does not need an advertised model.
pub fn eligible_models(advertised: Option<&WorkerModelCapabilities>) -> serde_json::Value {
    let Some(advertised) = advertised else {
        return serde_json::json!([]);
    };
    let references: Vec<_> = advertised
        .model
        .iter()
        .chain(advertised.models.iter())
        .collect();
    serde_json::Value::Array(references.into_iter().flat_map(|reference| advertised.providers.iter().filter(move |provider|supports_provider(**provider)).map(move |provider|serde_json::json!({"reference":reference,"provider":provider}))).collect())
}

fn purpose_name(purpose: ModelPurpose) -> &'static str {
    match purpose {
        ModelPurpose::Navigation => "navigation",
        ModelPurpose::StructuredAuthoring => "structured_authoring",
    }
}

fn state_name(state: ModelAssignmentState) -> &'static str {
    match state {
        ModelAssignmentState::Staged => "staged",
        ModelAssignmentState::Active => "active",
        ModelAssignmentState::Draining => "draining",
        ModelAssignmentState::Retired => "retired",
    }
}

/// A routing row is immutable except for its state; each transition has an audit event.
pub async fn stage_assignment(
    ctx: &loco_rs::app::AppContext,
    actor: Uuid,
    assignment: &ModelAssignment,
) -> ApiResult<()> {
    super::test_definitions::operator(ctx, actor, assignment.app_id).await?;
    assignment.validate().map_err(ApiFailure::invalid)?;
    let profile =
        super::test_definitions::profile(&ctx.db, assignment.app_id, assignment.profile_id).await?;
    if profile.driver != mobile_qa_contracts::execution::Driver::Minitap {
        return Err(ApiFailure::invalid(
            "Model assignment requires a Minitap profile",
        ));
    }
    let required = match assignment.purpose {
        ModelPurpose::Navigation => ModelCapability::MinitapNavigation,
        ModelPurpose::StructuredAuthoring => ModelCapability::StructuredAuthoring,
    };
    let (model, retired) = resolve(&ctx.db, &assignment.reference).await?;
    if retired || !model.has(required) || !supports_provider(model.provider) {
        return Err(conflict("Model revision is unavailable for this purpose"));
    }
    let tx = ctx.db.begin().await?;
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![assignment.app_id.into()],
    )
    .await?;
    let existing = rows(&tx,"SELECT payload FROM model_assignments WHERE app_id=$1 AND profile_id=$2 AND purpose=$3 AND revision=$4",vec![assignment.app_id.into(),assignment.profile_id.into(),purpose_name(assignment.purpose).into(),(assignment.revision as i32).into()]).await?;
    if let Some(row) = existing.first() {
        let prior: ModelAssignment = decode(field(row, "payload")?)?;
        if prior != *assignment {
            return Err(conflict("Assignment revisions are immutable"));
        }
        return Ok(());
    }
    let next: i32 = field(&one(&tx,"SELECT COALESCE(MAX(revision),0)+1 AS next FROM model_assignments WHERE app_id=$1 AND profile_id=$2 AND purpose=$3",vec![assignment.app_id.into(),assignment.profile_id.into(),purpose_name(assignment.purpose).into()]).await?,"next")?;
    if assignment.revision != next as u32 {
        return Err(conflict("Assignment revision must be next"));
    }
    exec(&tx,"INSERT INTO model_assignments(app_id,profile_id,purpose,revision,payload,state,created_by,changed_by) VALUES($1,$2,$3,$4,$5,'staged',$6,$6)",vec![assignment.app_id.into(),assignment.profile_id.into(),purpose_name(assignment.purpose).into(),next.into(),json(assignment)?.into(),actor.into()]).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn transition_assignment(
    ctx: &loco_rs::app::AppContext,
    actor: Uuid,
    assignment: &ModelAssignment,
    next: ModelAssignmentState,
) -> ApiResult<()> {
    super::test_definitions::operator(ctx, actor, assignment.app_id).await?;
    let tx = ctx.db.begin().await?;
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![assignment.app_id.into()],
    )
    .await?;
    let row = one(&tx,"SELECT payload,state FROM model_assignments WHERE app_id=$1 AND profile_id=$2 AND purpose=$3 AND revision=$4 FOR UPDATE",vec![assignment.app_id.into(),assignment.profile_id.into(),purpose_name(assignment.purpose).into(),(assignment.revision as i32).into()]).await?;
    if decode::<ModelAssignment>(field(&row, "payload")?)? != *assignment {
        return Err(conflict("Assignment payload differs"));
    }
    let current: String = field(&row, "state")?;
    let target = state_name(next);
    if current == target {
        return Ok(());
    }
    if !matches!(
        (current.as_str(), target),
        ("staged", "active")
            | ("staged", "retired")
            | ("active", "draining")
            | ("draining", "retired")
    ) {
        return Err(conflict("Invalid assignment state transition"));
    }
    if next == ModelAssignmentState::Active {
        let (resolved, retired) = resolve(&tx, &assignment.reference).await?;
        if retired {
            return Err(conflict("Model revision is retired"));
        }
        let counterpart = rows(&tx,"SELECT payload FROM model_assignments WHERE app_id=$1 AND profile_id=$2 AND purpose<>$3 AND state='active'",vec![assignment.app_id.into(),assignment.profile_id.into(),purpose_name(assignment.purpose).into()]).await?;
        let other = counterpart
            .first()
            .map(|row| decode::<ModelAssignment>(field(row, "payload")?))
            .transpose()?;
        let other = if let Some(other) = other {
            Some(resolve(&tx, &other.reference).await?.0)
        } else {
            None
        };
        let live = rows(&tx,"SELECT phone_model_capabilities,phone_protocol_version,phone_last_seen_at,execution_model_capabilities,execution_protocol_version,execution_last_seen_at FROM execution_workers WHERE app_id=$1 AND profile_id=$2 AND revoked=false",vec![assignment.app_id.into(),assignment.profile_id.into()]).await?;
        let recent = chrono::Utc::now() - chrono::Duration::seconds(90);
        if !live.iter().any(|row| {
            let phone_caps = field::<Option<serde_json::Value>>(row, "phone_model_capabilities")
                .ok()
                .flatten()
                .and_then(|value| decode::<WorkerModelCapabilities>(value).ok());
            let phone_version = field::<Option<i32>>(row, "phone_protocol_version")
                .ok()
                .flatten()
                .unwrap_or(0);
            let phone_seen =
                field::<Option<chrono::DateTime<chrono::Utc>>>(row, "phone_last_seen_at")
                    .ok()
                    .flatten();
            let phone_matches = phone_version >= 5
                && phone_seen.is_some_and(|seen| seen > recent)
                && worker_matches(&resolved, phone_caps.as_ref())
                && other
                    .as_ref()
                    .is_none_or(|model| worker_matches(model, phone_caps.as_ref()));
            if phone_matches {
                return true;
            }
            if assignment.purpose != ModelPurpose::Navigation || other.is_some() {
                return false;
            }
            let execution_caps =
                field::<Option<serde_json::Value>>(row, "execution_model_capabilities")
                    .ok()
                    .flatten()
                    .and_then(|value| decode::<WorkerModelCapabilities>(value).ok());
            let execution_version = field::<Option<i32>>(row, "execution_protocol_version")
                .ok()
                .flatten()
                .unwrap_or(0);
            let execution_seen =
                field::<Option<chrono::DateTime<chrono::Utc>>>(row, "execution_last_seen_at")
                    .ok()
                    .flatten();
            execution_version >= 5
                && execution_seen.is_some_and(|seen| seen > recent)
                && worker_matches(&resolved, execution_caps.as_ref())
        }) {
            return Err(conflict(
                "A qualified compatible worker must be live before activation",
            ));
        }
        let prior = rows(&tx,"SELECT revision FROM model_assignments WHERE app_id=$1 AND profile_id=$2 AND purpose=$3 AND state='active' FOR UPDATE",vec![assignment.app_id.into(),assignment.profile_id.into(),purpose_name(assignment.purpose).into()]).await?;
        for row in prior {
            let revision: i32 = field(&row, "revision")?;
            exec(&tx,"UPDATE model_assignments SET state='draining',changed_by=$5,changed_at=now() WHERE app_id=$1 AND profile_id=$2 AND purpose=$3 AND revision=$4",vec![assignment.app_id.into(),assignment.profile_id.into(),purpose_name(assignment.purpose).into(),revision.into(),actor.into()]).await?;
            exec(&tx,"INSERT INTO model_assignment_events(id,app_id,profile_id,purpose,revision,old_state,new_state,actor_id) VALUES($1,$2,$3,$4,$5,'active','draining',$6)",vec![Uuid::new_v4().into(),assignment.app_id.into(),assignment.profile_id.into(),purpose_name(assignment.purpose).into(),revision.into(),actor.into()]).await?;
        }
    }
    exec(&tx,"UPDATE model_assignments SET state=$5,changed_by=$6,changed_at=now() WHERE app_id=$1 AND profile_id=$2 AND purpose=$3 AND revision=$4",vec![assignment.app_id.into(),assignment.profile_id.into(),purpose_name(assignment.purpose).into(),(assignment.revision as i32).into(),target.into(),actor.into()]).await?;
    exec(&tx,"INSERT INTO model_assignment_events(id,app_id,profile_id,purpose,revision,old_state,new_state,actor_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8)",vec![Uuid::new_v4().into(),assignment.app_id.into(),assignment.profile_id.into(),purpose_name(assignment.purpose).into(),(assignment.revision as i32).into(),current.into(),target.into(),actor.into()]).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn active_assignment(
    db: &impl ConnectionTrait,
    app: Uuid,
    profile: Uuid,
    purpose: ModelPurpose,
) -> ApiResult<Option<(ResolvedModel, u32)>> {
    let rows=rows(db,"SELECT payload FROM model_assignments WHERE app_id=$1 AND profile_id=$2 AND purpose=$3 AND state='active'",vec![app.into(),profile.into(),purpose_name(purpose).into()]).await?;
    let Some(row) = rows.first() else {
        return Ok(None);
    };
    let assignment: ModelAssignment = decode(field(row, "payload")?)?;
    let (model, retired) = resolve(db, &assignment.reference).await?;
    if retired {
        return Err(conflict("Active model definition is retired"));
    }
    Ok(Some((model, assignment.revision)))
}
