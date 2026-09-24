//! One app lock serializes authoring with run admission. Receipts commit alongside
//! changes, so a lost response never creates another version.
use super::{
    execution_store::{conflict, decode, hash, json, word},
    test_definitions as definitions, test_library as library,
};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        apps as app_rows, test_library_defaults, test_library_drafts, test_library_entries,
        test_library_mutations, test_library_versions,
    },
};
use loco_rs::app::AppContext;
use mobile_qa_contracts::{execution::*, test_library::*};
use sea_orm::{
    sea_query::{Expr, OnConflict},
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, ExprTrait,
    IntoActiveModel, QueryFilter, QuerySelect, Set, TransactionTrait,
};
use serde::Serialize;
use uuid::Uuid;

#[derive(Serialize)]
#[serde(tag = "operation", content = "request", rename_all = "snake_case")]
pub enum Mutation {
    Authored(mobile_qa_contracts::automation::SaveAuthoredTestsRequest),
    Create(CreateLibraryEntryRequest),
    Save(Uuid, SaveLibraryDraftRequest),
    Archive(Uuid, ArchiveLibraryEntryRequest),
    Default(SetDefaultPlanRequest),
}
impl Mutation {
    fn id(&self) -> Uuid {
        match self {
            Self::Authored(r) => r.mutation_id,
            Self::Create(r) => r.mutation_id,
            Self::Save(_, r) => r.mutation_id,
            Self::Archive(_, r) => r.mutation_id,
            Self::Default(r) => r.mutation_id,
        }
    }
    fn entry_revision(&self) -> Option<(Uuid, i32)> {
        match self {
            Self::Save(id, r) => Some((*id, r.expected_revision)),
            Self::Archive(id, r) => Some((*id, r.expected_revision)),
            _ => None,
        }
    }
}
pub fn stale(id: Option<Uuid>, revision: i32) -> ApiFailure {
    ApiFailure::new(
        409,
        "stale_revision",
        "This record changed. Compare your draft with the current version before saving again",
    )
    .with_library_details(LibraryErrorDetails::StaleRevision {
        entry_id: id,
        current_revision: revision,
    })
}
async fn bump(db: &impl ConnectionTrait, id: Uuid, actor: Uuid) -> ApiResult<()> {
    let result = test_library_entries::Entity::update_many()
        .col_expr(
            test_library_entries::Column::Revision,
            Expr::col(test_library_entries::Column::Revision).add(1),
        )
        .col_expr(test_library_entries::Column::ActorId, Expr::value(actor))
        .col_expr(
            test_library_entries::Column::UpdatedAt,
            Expr::value(chrono::Utc::now()),
        )
        .filter(test_library_entries::Column::Id.eq(id))
        .filter(test_library_entries::Column::Revision.lt(i32::MAX))
        .exec(db)
        .await?;
    if result.rows_affected != 1 {
        return Err(conflict("Entry revision limit reached"));
    }
    Ok(())
}
pub async fn apply(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    mutation: Mutation,
) -> ApiResult<(LibraryMutationReceipt, bool)> {
    library::authorize(ctx, actor, app).await?;
    let tx = ctx.db.begin().await?;
    app_rows::Entity::find_by_id(app)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    // Rights are checked even for a replay; a receipt never grants access.
    let caps = library::capabilities(&tx, actor, app).await?;
    match &mutation {
        Mutation::Archive(..) | Mutation::Default(..) if !caps.can_archive => {
            return Err(ApiFailure::new(
                403,
                "operator_required",
                "An active operator is required",
            ))
        }
        _ => {}
    }
    let mutation_id = mutation.id();
    let fingerprint = hash(
        serde_json::to_vec(&("save_lifecycle_v2", &mutation))
            .map_err(|_| ApiFailure::internal())?,
    );
    let previous = test_library_mutations::Entity::find_by_id((app, actor, mutation_id))
        .one(&tx)
        .await?;
    if let Some(row) = previous {
        if row.fingerprint != fingerprint {
            return Err(ApiFailure::new(
                409,
                "idempotency_conflict",
                "Use a new mutation ID for changed content",
            )
            .with_library_details(LibraryErrorDetails::IdempotencyConflict { mutation_id }));
        }
        return Ok((decode(row.response)?, false));
    }
    if let Some((id, expected)) = mutation.entry_revision() {
        let entry = library::entry(&tx, actor, app, id).await?;
        if entry.revision != expected {
            return Err(stale(Some(id), entry.revision));
        }
        if entry.archived_at.is_some() && !matches!(mutation, Mutation::Archive(..)) {
            return Err(conflict("Unarchive this entry before making changes"));
        }
    }
    let response = match mutation {
        Mutation::Authored(r) => {
            LibraryMutationReceipt::Authored(super::test_authoring::save(&tx, actor, app, r).await?)
        }
        Mutation::Create(r) => {
            if !bounded(&r.key, 100) {
                return Err(ApiFailure::invalid(
                    "Key must contain 1–100 printable bytes",
                ));
            }
            let duplicate = test_library_entries::Entity::find()
                .filter(
                    Condition::any()
                        .add(test_library_entries::Column::Id.eq(r.entry_id))
                        .add(
                            Condition::all()
                                .add(test_library_entries::Column::AppId.eq(app))
                                .add(test_library_entries::Column::Kind.eq(word(&r.kind)))
                                .add(test_library_entries::Column::LogicalKey.eq(&r.key)),
                        ),
                )
                .one(&tx)
                .await?;
            if duplicate.is_some() {
                return Err(conflict("Entry identity already exists"));
            }
            let package = app_rows::Entity::find_by_id(app)
                .one(&tx)
                .await?
                .ok_or_else(ApiFailure::missing)?
                .android_package;
            let profile = if let Some(id) = r.template_profile_id {
                Some(definitions::profile(&tx, app, id).await?)
            } else {
                None
            };
            let budget = ExecutionBudget {
                duration_seconds: if r.kind == DefinitionKind::Plan {
                    1200
                } else {
                    300
                },
                max_steps: 80,
                artifact_bytes: 16777216,
            };
            let definition = match r.kind {
                DefinitionKind::Case => {
                    if profile.as_ref().is_some_and(|p| p.package != package) {
                        return Err(ApiFailure::invalid(
                            "Template profile is incompatible with this app",
                        ));
                    }
                    if profile.is_some() && package == "ai.mobileqa.demo" {
                        // This opt-in template is the existing synthetic demo case, never customer data.
                        let mut definition: TestDefinition = serde_json::from_str(include_str!(
                            "../../../../contracts/fixtures/execution/persistence-case.json"
                        ))
                        .map_err(|_| ApiFailure::internal())?;
                        let TestDefinition::Case(ref mut c) = definition else {
                            return Err(ApiFailure::internal());
                        };
                        c.key = r.key.clone();
                        c.version = 1;
                        c.provenance = "user_authored".into();
                        definition.into()
                    } else {
                        LibraryDraftDefinition::Case(CaseDefinition {
                            key: r.key.clone(),
                            version: 1,
                            title: String::new(),
                            requirement: String::new(),
                            provenance: "user_authored".into(),
                            adapter: super::execution_readiness::adapter(&tx, app, &package)
                                .await?,
                            package,
                            preconditions: vec![],
                            actions: vec![],
                            checks: vec![],
                            budget,
                        })
                    }
                }
                DefinitionKind::Suite => LibraryDraftDefinition::Suite(SuiteDefinition {
                    key: r.key.clone(),
                    version: 1,
                    title: String::new(),
                    cases: vec![],
                }),
                DefinitionKind::Plan => LibraryDraftDefinition::Plan(PlanDraftContent {
                    key: r.key.clone(),
                    version: 1,
                    title: String::new(),
                    suite_version_ids: vec![],
                    cases: vec![],
                    profile_id: profile.map(|p| p.id),
                    budget,
                    diagnostic_retries: 0,
                    exclusions: vec![],
                }),
            };
            test_library_entries::ActiveModel {
                id: Set(r.entry_id),
                app_id: Set(app),
                kind: Set(word(&r.kind)),
                logical_key: Set(r.key),
                next_version: Set(2),
                actor_id: Set(actor),
                ..Default::default()
            }
            .insert(&tx)
            .await?;
            test_library_drafts::ActiveModel {
                entry_id: Set(r.entry_id),
                version: Set(1),
                payload: Set(json(&definition)?),
                editor_id: Set(actor),
                ..Default::default()
            }
            .insert(&tx)
            .await?;
            LibraryMutationReceipt::Draft(Box::new(
                library::draft(&tx, actor, app, r.entry_id).await?,
            ))
        }
        Mutation::Save(id, r) => {
            super::test_library_save::save(&tx, actor, app, id, r.definition).await?;
            bump(&tx, id, actor).await?;
            LibraryMutationReceipt::Draft(Box::new(library::draft(&tx, actor, app, id).await?))
        }
        Mutation::Archive(id, r) => {
            let entry = test_library_entries::Entity::find_by_id(id)
                .one(&tx)
                .await?
                .ok_or_else(ApiFailure::missing)?;
            let archived_at = if r.archived {
                entry.archived_at.or_else(|| Some(chrono::Utc::now()))
            } else {
                None
            };
            let mut active = entry.into_active_model();
            active.archived_at = Set(archived_at);
            active.update(&tx).await?;
            bump(&tx, id, actor).await?;
            LibraryMutationReceipt::Entry(library::entry(&tx, actor, app, id).await?)
        }
        Mutation::Default(r) => {
            let current = library::default_plan(&tx, app).await?;
            if current.revision != r.expected_revision {
                return Err(stale(None, current.revision));
            }
            library::admitted(&tx, app, r.plan_version_id).await?;
            let definition = definitions::get(&tx, app, r.plan_version_id).await?;
            if !matches!(definition.definition, TestDefinition::Plan(_)) {
                return Err(ApiFailure::invalid("Choose a saved plan version"));
            }
            library::executable(&tx, app, &definition.definition).await?;
            let next = current
                .revision
                .checked_add(1)
                .ok_or_else(|| conflict("Default revision limit reached"))?;
            test_library_defaults::Entity::insert(test_library_defaults::ActiveModel {
                app_id: Set(app),
                plan_version_id: Set(r.plan_version_id),
                revision: Set(next),
                actor_id: Set(actor),
                ..Default::default()
            })
            .on_conflict(
                OnConflict::column(test_library_defaults::Column::AppId)
                    .update_columns([
                        test_library_defaults::Column::PlanVersionId,
                        test_library_defaults::Column::Revision,
                        test_library_defaults::Column::ActorId,
                        test_library_defaults::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec_without_returning(&tx)
            .await?;
            LibraryMutationReceipt::Default(library::default_plan(&tx, app).await?)
        }
    };
    test_library_mutations::ActiveModel {
        app_id: Set(app),
        actor_id: Set(actor),
        mutation_id: Set(mutation_id),
        fingerprint: Set(fingerprint),
        response: Set(json(&response)?),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    tx.commit().await?;
    Ok((response, true))
}

/// Register operator imports in the same catalog, respecting versions reserved by
/// drafts. Exact historical imports are still harmless, but never reset lifecycle.
pub async fn link_import(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
    definition: &DefinitionResponse,
) -> ApiResult<()> {
    let (kind, key, version) = definition.definition.identity();
    let next = version
        .checked_add(1)
        .filter(|v| *v <= i32::MAX as u32)
        .ok_or_else(|| conflict("Version limit reached"))? as i32;
    let kind_word = word(&kind);
    test_library_entries::Entity::insert(test_library_entries::ActiveModel {
        id: Set(Uuid::new_v4()),
        app_id: Set(app),
        kind: Set(kind_word.clone()),
        logical_key: Set(key.to_owned()),
        next_version: Set(next),
        actor_id: Set(actor),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::columns([
            test_library_entries::Column::AppId,
            test_library_entries::Column::Kind,
            test_library_entries::Column::LogicalKey,
        ])
        .do_nothing()
        .to_owned(),
    )
    .exec_without_returning(db)
    .await?;
    let entry = test_library_entries::Entity::find()
        .filter(test_library_entries::Column::AppId.eq(app))
        .filter(test_library_entries::Column::Kind.eq(kind_word))
        .filter(test_library_entries::Column::LogicalKey.eq(key))
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let id = entry.id;
    if test_library_versions::Entity::find_by_id(definition.id)
        .one(db)
        .await?
        .is_some()
    {
        return Ok(());
    }
    if entry.archived_at.is_some() {
        return Err(conflict("Entry is archived"));
    }
    if test_library_drafts::Entity::find_by_id(id)
        .filter(test_library_drafts::Column::Version.eq(version as i32))
        .one(db)
        .await?
        .is_some()
    {
        return Err(conflict("Version is reserved by an editable draft"));
    }
    test_library_versions::ActiveModel {
        definition_id: Set(definition.id),
        entry_id: Set(id),
        ..Default::default()
    }
    .insert(db)
    .await?;
    if entry.next_version < next {
        let mut active = entry.into_active_model();
        active.next_version = Set(next);
        active.update(db).await?;
    }
    bump(db, id, actor).await
}
