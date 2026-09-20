//! One app lock serializes authoring with run admission. Receipts commit alongside
//! changes, so a lost response never creates another version.
use super::{execution_store::*, test_definitions as definitions, test_library as library};
use crate::errors::{ApiFailure, ApiResult};
use loco_rs::app::AppContext;
use mobile_qa_contracts::{execution::*, test_library::*};
use sea_orm::{ConnectionTrait, TransactionTrait};
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
    let count = exec(
        db,
        "UPDATE test_library_entries SET revision=revision+1, actor_id=$2, updated_at=now() WHERE
        id=$1 AND revision<2147483647",
        vec![id.into(), actor.into()],
    )
    .await?;
    if count != 1 {
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
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![app.into()],
    )
    .await?;
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
    let previous=rows(&tx,"SELECT fingerprint, response FROM test_library_mutations WHERE app_id=$1 AND actor_id=$2 AND
        mutation_id=$3",vec![app.into(),actor.into(),mutation_id.into()]).await?;
    if let Some(row) = previous.first() {
        if field::<String>(row, "fingerprint")? != fingerprint {
            return Err(ApiFailure::new(
                409,
                "idempotency_conflict",
                "Use a new mutation ID for changed content",
            )
            .with_library_details(LibraryErrorDetails::IdempotencyConflict { mutation_id }));
        }
        return Ok((decode(field(row, "response")?)?, false));
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
            if !rows(&tx,"SELECT id FROM test_library_entries WHERE id=$1 OR (app_id=$2 AND kind=$3 AND logical_key=$4)",vec![r.entry_id.into(),app.into(),word(&r.kind).into(),r.key.clone().into()]).await?.is_empty() {return Err(conflict("Entry identity already exists"));}
            let package: String = field(
                &one(
                    &tx,
                    "SELECT android_package FROM apps WHERE id=$1",
                    vec![app.into()],
                )
                .await?,
                "android_package",
            )?;
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
            exec(&tx,"INSERT INTO test_library_entries(id, app_id, kind, logical_key, next_version, actor_id)
        VALUES($1, $2, $3, $4,2, $5)",vec![r.entry_id.into(),app.into(),word(&r.kind).into(),r.key.into(),actor.into()]).await?;
            exec(&tx,"INSERT INTO test_library_drafts(entry_id,version,payload,editor_id) VALUES($1,1,$2,$3)",vec![r.entry_id.into(),json(&definition)?.into(),actor.into()]).await?;
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
            exec(&tx,"UPDATE test_library_entries SET archived_at=CASE WHEN $2 THEN COALESCE(archived_at, now())
        ELSE NULL END WHERE id=$1",vec![id.into(),r.archived.into()]).await?;
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
            exec(&tx,"INSERT INTO test_library_defaults(app_id, plan_version_id, revision, actor_id) VALUES($1,
        $2, $3, $4) ON CONFLICT(app_id) DO UPDATE SET plan_version_id=EXCLUDED.plan_version_id,
        revision=EXCLUDED.revision, actor_id=EXCLUDED.actor_id, updated_at=now()",vec![app.into(),r.plan_version_id.into(),next.into(),actor.into()]).await?;
            LibraryMutationReceipt::Default(library::default_plan(&tx, app).await?)
        }
    };
    exec(
        &tx,
        "INSERT INTO test_library_mutations(app_id, actor_id, mutation_id, fingerprint, response)
        VALUES($1, $2, $3, $4, $5)",
        vec![
            app.into(),
            actor.into(),
            mutation_id.into(),
            fingerprint.into(),
            json(&response)?.into(),
        ],
    )
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
    exec(
        db,
        "INSERT INTO test_library_entries(id, app_id, kind, logical_key, next_version, actor_id)
        VALUES($1, $2, $3, $4, $5, $6) ON CONFLICT(app_id, kind, logical_key) DO NOTHING",
        vec![
            Uuid::new_v4().into(),
            app.into(),
            word(&kind).into(),
            key.into(),
            next.into(),
            actor.into(),
        ],
    )
    .await?;
    let entry=one(db,"SELECT id,archived_at FROM test_library_entries WHERE app_id=$1 AND kind=$2 AND logical_key=$3",vec![app.into(),word(&kind).into(),key.into()]).await?;
    let id: Uuid = field(&entry, "id")?;
    if !rows(
        db,
        "SELECT definition_id FROM test_library_versions WHERE definition_id=$1",
        vec![definition.id.into()],
    )
    .await?
    .is_empty()
    {
        return Ok(());
    }
    if field::<Option<chrono::DateTime<chrono::Utc>>>(&entry, "archived_at")?.is_some() {
        return Err(conflict("Entry is archived"));
    }
    if !rows(
        db,
        "SELECT entry_id FROM test_library_drafts WHERE entry_id=$1 AND version=$2",
        vec![id.into(), (version as i32).into()],
    )
    .await?
    .is_empty()
    {
        return Err(conflict("Version is reserved by an editable draft"));
    }
    exec(
        db,
        "INSERT INTO test_library_versions(definition_id,entry_id) VALUES($1,$2)",
        vec![definition.id.into(), id.into()],
    )
    .await?;
    exec(
        db,
        "UPDATE test_library_entries SET next_version=GREATEST(next_version,$2) WHERE id=$1",
        vec![id.into(), next.into()],
    )
    .await?;
    bump(db, id, actor).await
}
