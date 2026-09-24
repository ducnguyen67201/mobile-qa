//! Authoritative model resolution. This module is the only legacy-name mapper.
use super::execution_store::{conflict, decode, hash, json};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        apps as app_rows, execution_workers, model_assignment_events, model_assignments,
        model_definitions,
    },
};
use mobile_qa_contracts::model_registry::{
    ModelAssignment, ModelAssignmentState, ModelBinding, ModelCapability, ModelDefinition,
    ModelProvider, ModelPurpose, ModelReference, ResolvedModel, WorkerModelCapabilities,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionSession, TransactionTrait,
};
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
    let now = chrono::Utc::now();
    let payload = json(&capabilities)?;
    execution_workers::Entity::update_many()
        .col_expr(
            execution_workers::Column::ModelCapabilities,
            sea_orm::sea_query::Expr::value(Some(payload.clone())),
        )
        .col_expr(
            execution_workers::Column::ModelLastSeenAt,
            sea_orm::sea_query::Expr::value(Some(now)),
        )
        .col_expr(
            execution_workers::Column::ExecutionModelCapabilities,
            sea_orm::sea_query::Expr::value(Some(payload)),
        )
        .col_expr(
            execution_workers::Column::ExecutionLastSeenAt,
            sea_orm::sea_query::Expr::value(Some(now)),
        )
        .col_expr(
            execution_workers::Column::ExecutionProtocolVersion,
            sea_orm::sea_query::Expr::value(Some(version as i32)),
        )
        .filter(execution_workers::Column::Id.eq(worker))
        .filter(execution_workers::Column::Revoked.eq(false))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn advertise_phone(
    db: &impl ConnectionTrait,
    worker: Uuid,
    version: u32,
    capabilities: Option<&WorkerModelCapabilities>,
) -> ApiResult<()> {
    let now = chrono::Utc::now();
    let payload = json(&capabilities)?;
    execution_workers::Entity::update_many()
        .col_expr(
            execution_workers::Column::ModelCapabilities,
            sea_orm::sea_query::Expr::value(Some(payload.clone())),
        )
        .col_expr(
            execution_workers::Column::ModelLastSeenAt,
            sea_orm::sea_query::Expr::value(Some(now)),
        )
        .col_expr(
            execution_workers::Column::PhoneModelCapabilities,
            sea_orm::sea_query::Expr::value(Some(payload)),
        )
        .col_expr(
            execution_workers::Column::PhoneLastSeenAt,
            sea_orm::sea_query::Expr::value(Some(now)),
        )
        .col_expr(
            execution_workers::Column::PhoneProtocolVersion,
            sea_orm::sea_query::Expr::value(Some(version as i32)),
        )
        .filter(execution_workers::Column::Id.eq(worker))
        .filter(execution_workers::Column::Revoked.eq(false))
        .exec(db)
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
    let existing = model_definitions::Entity::find_by_id((
        definition.reference.key.clone(),
        definition.reference.revision as i32,
    ))
    .lock_exclusive()
    .one(&tx)
    .await?;
    if let Some(existing) = existing {
        if existing.payload != payload {
            return Err(conflict(
                "Model revisions are immutable; register a new revision",
            ));
        }
    } else {
        model_definitions::ActiveModel {
            key: Set(definition.reference.key.clone()),
            revision: Set(definition.reference.revision as i32),
            payload: Set(payload),
            registered_by: Set(Some(actor)),
            payload_sha256: Set(Some(hash(
                serde_json::to_vec(definition).map_err(|_| ApiFailure::internal())?,
            ))),
            ..Default::default()
        }
        .insert(&tx)
        .await?;
    }
    tx.commit().await?;
    Ok(definition.resolve())
}

pub async fn retire(db: &impl ConnectionTrait, reference: &ModelReference) -> ApiResult<()> {
    reference.validate().map_err(ApiFailure::invalid)?;
    let row =
        model_definitions::Entity::find_by_id((reference.key.clone(), reference.revision as i32))
            .one(db)
            .await?
            .ok_or_else(ApiFailure::missing)?;
    if row.retired_at.is_none() {
        let mut active = row.into_active_model();
        active.retired_at = Set(Some(chrono::Utc::now()));
        active.update(db).await?;
    }
    Ok(())
}

pub async fn resolve(
    db: &impl ConnectionTrait,
    reference: &ModelReference,
) -> ApiResult<(ResolvedModel, bool)> {
    reference.validate().map_err(ApiFailure::invalid)?;
    let row =
        model_definitions::Entity::find_by_id((reference.key.clone(), reference.revision as i32))
            .one(db)
            .await?
            .ok_or_else(ApiFailure::missing)?;
    let retired = row.retired_at.is_some();
    let definition: ModelDefinition = decode(row.payload)?;
    definition.validate().map_err(ApiFailure::invalid)?;
    Ok((definition.resolve(), retired))
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
    let matches = model_definitions::Entity::find()
        .filter(model_definitions::Column::RetiredAt.is_null())
        .order_by_asc(model_definitions::Column::Key)
        .order_by_asc(model_definitions::Column::Revision)
        .all(db)
        .await?
        .into_iter()
        .map(|row| decode::<ModelDefinition>(row.payload))
        .collect::<ApiResult<Vec<_>>>()?
        .into_iter()
        .filter(|definition| {
            definition.provider == ModelProvider::OpenAi
                && definition.provider_model == provider_model
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [] => Err(ApiFailure::new(
            409,
            "legacy_model_unmapped",
            "The historical model name is not registered",
        )),
        [definition] => Ok(definition.resolve()),
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
    app_rows::Entity::find_by_id(assignment.app_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let purpose = purpose_name(assignment.purpose);
    let existing = model_assignments::Entity::find_by_id((
        assignment.app_id,
        assignment.profile_id,
        purpose.to_owned(),
        assignment.revision as i32,
    ))
    .one(&tx)
    .await?;
    if let Some(row) = existing {
        let prior: ModelAssignment = decode(row.payload)?;
        if prior != *assignment {
            return Err(conflict("Assignment revisions are immutable"));
        }
        return Ok(());
    }
    let next = model_assignments::Entity::find()
        .filter(model_assignments::Column::AppId.eq(assignment.app_id))
        .filter(model_assignments::Column::ProfileId.eq(assignment.profile_id))
        .filter(model_assignments::Column::Purpose.eq(purpose))
        .order_by_desc(model_assignments::Column::Revision)
        .one(&tx)
        .await?
        .map_or(1, |row| row.revision + 1);
    if assignment.revision != next as u32 {
        return Err(conflict("Assignment revision must be next"));
    }
    model_assignments::ActiveModel {
        app_id: Set(assignment.app_id),
        profile_id: Set(assignment.profile_id),
        purpose: Set(purpose.to_owned()),
        revision: Set(next),
        payload: Set(json(assignment)?),
        state: Set("staged".into()),
        created_by: Set(actor),
        changed_by: Set(actor),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
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
    app_rows::Entity::find_by_id(assignment.app_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let purpose = purpose_name(assignment.purpose);
    let row = model_assignments::Entity::find_by_id((
        assignment.app_id,
        assignment.profile_id,
        purpose.to_owned(),
        assignment.revision as i32,
    ))
    .lock_exclusive()
    .one(&tx)
    .await?
    .ok_or_else(ApiFailure::missing)?;
    if decode::<ModelAssignment>(row.payload.clone())? != *assignment {
        return Err(conflict("Assignment payload differs"));
    }
    let current = row.state.clone();
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
        let counterpart = model_assignments::Entity::find()
            .filter(model_assignments::Column::AppId.eq(assignment.app_id))
            .filter(model_assignments::Column::ProfileId.eq(assignment.profile_id))
            .filter(model_assignments::Column::Purpose.ne(purpose))
            .filter(model_assignments::Column::State.eq("active"))
            .one(&tx)
            .await?;
        let other = counterpart
            .map(|row| decode::<ModelAssignment>(row.payload))
            .transpose()?;
        let other = if let Some(other) = other {
            Some(resolve(&tx, &other.reference).await?.0)
        } else {
            None
        };
        let live = execution_workers::Entity::find()
            .filter(execution_workers::Column::AppId.eq(assignment.app_id))
            .filter(execution_workers::Column::ProfileId.eq(assignment.profile_id))
            .filter(execution_workers::Column::Revoked.eq(false))
            .all(&tx)
            .await?;
        let recent = chrono::Utc::now() - chrono::Duration::seconds(90);
        if !live.iter().any(|row| {
            let phone_caps = row
                .phone_model_capabilities
                .clone()
                .and_then(|value| decode::<WorkerModelCapabilities>(value).ok());
            let phone_version = row.phone_protocol_version.unwrap_or(0);
            let phone_seen = row.phone_last_seen_at;
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
            let execution_caps = row
                .execution_model_capabilities
                .clone()
                .and_then(|value| decode::<WorkerModelCapabilities>(value).ok());
            let execution_version = row.execution_protocol_version.unwrap_or(0);
            let execution_seen = row.execution_last_seen_at;
            execution_version >= 5
                && execution_seen.is_some_and(|seen| seen > recent)
                && worker_matches(&resolved, execution_caps.as_ref())
        }) {
            return Err(conflict(
                "A qualified compatible worker must be live before activation",
            ));
        }
        let prior = model_assignments::Entity::find()
            .filter(model_assignments::Column::AppId.eq(assignment.app_id))
            .filter(model_assignments::Column::ProfileId.eq(assignment.profile_id))
            .filter(model_assignments::Column::Purpose.eq(purpose))
            .filter(model_assignments::Column::State.eq("active"))
            .lock_exclusive()
            .all(&tx)
            .await?;
        for row in prior {
            let revision = row.revision;
            let mut active = row.into_active_model();
            active.state = Set("draining".into());
            active.changed_by = Set(actor);
            active.changed_at = Set(chrono::Utc::now());
            active.update(&tx).await?;
            model_assignment_events::ActiveModel {
                id: Set(Uuid::new_v4()),
                app_id: Set(assignment.app_id),
                profile_id: Set(assignment.profile_id),
                purpose: Set(purpose.into()),
                revision: Set(revision),
                old_state: Set("active".into()),
                new_state: Set("draining".into()),
                actor_id: Set(actor),
                ..Default::default()
            }
            .insert(&tx)
            .await?;
        }
    }
    let mut active = row.into_active_model();
    active.state = Set(target.into());
    active.changed_by = Set(actor);
    active.changed_at = Set(chrono::Utc::now());
    active.update(&tx).await?;
    model_assignment_events::ActiveModel {
        id: Set(Uuid::new_v4()),
        app_id: Set(assignment.app_id),
        profile_id: Set(assignment.profile_id),
        purpose: Set(purpose.into()),
        revision: Set(assignment.revision as i32),
        old_state: Set(current),
        new_state: Set(target.into()),
        actor_id: Set(actor),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

pub async fn active_assignment(
    db: &impl ConnectionTrait,
    app: Uuid,
    profile: Uuid,
    purpose: ModelPurpose,
) -> ApiResult<Option<(ResolvedModel, u32)>> {
    let row = model_assignments::Entity::find()
        .filter(model_assignments::Column::AppId.eq(app))
        .filter(model_assignments::Column::ProfileId.eq(profile))
        .filter(model_assignments::Column::Purpose.eq(purpose_name(purpose)))
        .filter(model_assignments::Column::State.eq("active"))
        .one(db)
        .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let assignment: ModelAssignment = decode(row.payload)?;
    let (model, retired) = resolve(db, &assignment.reference).await?;
    if retired {
        return Err(conflict("Active model definition is retired"));
    }
    Ok(Some((model, assignment.revision)))
}
