//! One app lock serializes authoring with run admission. Receipts commit alongside
//! changes, so a lost response never creates another version or review event.
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
    Create(CreateLibraryEntryRequest),
    Fork(Uuid, ForkLibraryDraftRequest),
    Save(Uuid, SaveLibraryDraftRequest),
    Submit(Uuid, SubmitLibraryDraftRequest),
    Review(Uuid, Uuid, ReviewLibraryVersionRequest),
    Archive(Uuid, ArchiveLibraryEntryRequest),
    Default(SetDefaultPlanRequest),
}
impl Mutation {
    fn id(&self) -> Uuid {
        match self {
            Self::Create(r) => r.mutation_id,
            Self::Fork(_, r) => r.mutation_id,
            Self::Save(_, r) => r.mutation_id,
            Self::Submit(_, r) => r.mutation_id,
            Self::Review(_, _, r) => r.mutation_id,
            Self::Archive(_, r) => r.mutation_id,
            Self::Default(r) => r.mutation_id,
        }
    }
    fn entry_revision(&self) -> Option<(Uuid, i32)> {
        match self {
            Self::Fork(id, r) => Some((*id, r.expected_revision)),
            Self::Save(id, r) => Some((*id, r.expected_revision)),
            Self::Submit(id, r) => Some((*id, r.expected_revision)),
            Self::Review(id, _, r) => Some((*id, r.expected_revision)),
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
        Mutation::Review(_, _, r)
            if !match r.purpose {
                ApprovalPurpose::Business => caps.can_review_business,
                ApprovalPurpose::Executability => caps.can_review_executability,
            } =>
        {
            return Err(ApiFailure::new(
                403,
                "reviewer_required",
                "This review permission is required",
            ))
        }
        _ => {}
    }
    let mutation_id = mutation.id();
    let fingerprint = hash(serde_json::to_vec(&mutation).map_err(|_| ApiFailure::internal())?);
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
                            package,
                            adapter: "demo_persistence_v1".into(),
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
            LibraryMutationReceipt::Draft(library::draft(&tx, actor, app, r.entry_id).await?)
        }
        Mutation::Fork(id, r) => {
            let source = library::version(&tx, actor, app, id, r.source_version_id).await?;
            if source.entry.draft_version.is_some() {
                return Err(conflict("An editable draft already exists"));
            }
            let next: i32 = field(
                &one(
                    &tx,
                    "SELECT next_version FROM test_library_entries WHERE id=$1",
                    vec![id.into()],
                )
                .await?,
                "next_version",
            )?;
            if next == i32::MAX {
                return Err(conflict("Version limit reached"));
            }
            let mut definition: LibraryDraftDefinition = source.version.definition.into();
            definition.allocate(next as u32);
            exec(&tx,"INSERT INTO test_library_drafts(entry_id, version, source_version_id, payload, editor_id)
        VALUES($1, $2, $3, $4, $5)",vec![id.into(),next.into(),r.source_version_id.into(),json(&definition)?.into(),actor.into()]).await?;
            exec(
                &tx,
                "UPDATE test_library_entries SET next_version=next_version+1 WHERE id=$1",
                vec![id.into()],
            )
            .await?;
            bump(&tx, id, actor).await?;
            LibraryMutationReceipt::Draft(library::draft(&tx, actor, app, id).await?)
        }
        Mutation::Save(id, mut r) => {
            let original = library::draft(&tx, actor, app, id).await?;
            if original.definition.identity() != r.definition.identity() {
                return Err(conflict("Draft kind, key and version cannot change"));
            }
            if let LibraryDraftDefinition::Case(ref mut c) = r.definition {
                let LibraryDraftDefinition::Case(previous) = &original.definition else {
                    return Err(ApiFailure::internal());
                };
                if c.package != previous.package {
                    return Err(ApiFailure::invalid("The draft package belongs to its app"));
                }
                c.provenance = "user_authored".into();
            }
            r.definition.check_bounds().map_err(ApiFailure::invalid)?;
            let payload = json(&r.definition)?;
            if serde_json::to_vec(&payload)
                .map_err(|_| ApiFailure::internal())?
                .len()
                > 1048576
            {
                return Err(ApiFailure::new(
                    413,
                    "draft_too_large",
                    "Draft exceeds 1 MiB",
                ));
            }
            exec(&tx,"UPDATE test_library_drafts SET payload=$2,editor_id=$3,updated_at=now() WHERE entry_id=$1",vec![id.into(),payload.into(),actor.into()]).await?;
            bump(&tx, id, actor).await?;
            LibraryMutationReceipt::Draft(library::draft(&tx, actor, app, id).await?)
        }
        Mutation::Submit(id, _) => {
            let draft = library::draft(&tx, actor, app, id).await?;
            if !draft.issues.is_empty() {
                return Err(library::validation(draft.issues));
            }
            let definition = draft
                .definition
                .published()
                .map_err(|issue| library::validation(vec![issue]))?;
            definition.validate().map_err(ApiFailure::invalid)?;
            let payload = json(&definition)?;
            let digest = hash(serde_json::to_vec(&payload).map_err(|_| ApiFailure::internal())?);
            let version_id = Uuid::new_v4();
            let (kind, key, version) = definition.identity();
            exec(&tx,"INSERT INTO execution_definitions(id, app_id, kind, logical_key, version, content_hash,
        payload, author_id) VALUES($1, $2, $3, $4, $5, $6, $7, $8)",vec![version_id.into(),app.into(),word(&kind).into(),key.into(),(version as i32).into(),digest.into(),payload.into(),actor.into()]).await?;
            exec(&tx,"INSERT INTO test_library_versions(definition_id,entry_id,review_state) VALUES($1,$2,'in_review')",vec![version_id.into(),id.into()]).await?;
            exec(
                &tx,
                "DELETE FROM test_library_drafts WHERE entry_id=$1",
                vec![id.into()],
            )
            .await?;
            bump(&tx, id, actor).await?;
            LibraryMutationReceipt::Version(
                library::version(&tx, actor, app, id, version_id).await?,
            )
        }
        Mutation::Review(id, version_id, r) => {
            review(&tx, actor, app, id, version_id, &r).await?;
            LibraryMutationReceipt::Version(
                library::version(&tx, actor, app, id, version_id).await?,
            )
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
            if !matches!(definition.definition, TestDefinition::Plan(_))
                || !definitions::approved(&definition)
            {
                return Err(ApiFailure::invalid("Choose an approved plan version"));
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

/// Shared with the operator CLI. Caller holds the app lock; no legacy path may
/// approve a rejected/archived candidate or bypass purpose-specific grants.
pub async fn review(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
    entry_id: Uuid,
    version_id: Uuid,
    r: &ReviewLibraryVersionRequest,
) -> ApiResult<()> {
    let candidate = library::version(db, actor, app, entry_id, version_id).await?;
    if candidate.entry.revision != r.expected_revision {
        return Err(stale(Some(entry_id), candidate.entry.revision));
    }
    if candidate.entry.archived_at.is_some()
        || candidate.review_state != LibraryReviewState::InReview
    {
        return Err(conflict(
            "Only an active version awaiting review can receive a decision",
        ));
    }
    let caps = &candidate.entry.capabilities;
    if !match r.purpose {
        ApprovalPurpose::Business => caps.can_review_business,
        ApprovalPurpose::Executability => caps.can_review_executability,
    } {
        return Err(ApiFailure::new(
            403,
            "reviewer_required",
            "This review permission is required",
        ));
    }
    if candidate.version.content_hash != r.content_hash {
        return Err(conflict("Reviewed content hash changed"));
    }
    if candidate
        .review_events
        .iter()
        .any(|e| e.purpose == r.purpose)
    {
        return Err(conflict("This review purpose already has a decision"));
    }
    if r.reason.as_ref().is_some_and(|s| !bounded(s, 2000))
        || (r.decision != LibraryReviewDecision::Approve && r.reason.is_none())
    {
        return Err(ApiFailure::invalid(
            "Changes requested or rejection needs a reason of 1–2000 printable bytes",
        ));
    }
    if r.decision == LibraryReviewDecision::Approve {
        let coverage =
            library::coverage(db, app, &candidate.version.definition.clone().into()).await?;
        if !coverage.issues.is_empty() {
            return Err(library::validation(coverage.issues));
        }
    }
    if r.decision == LibraryReviewDecision::Approve && r.purpose == ApprovalPurpose::Executability {
        library::executable(db, app, &candidate.version.definition).await?;
    }
    exec(
        db,
        "INSERT INTO test_library_review_events(id, entry_id, definition_id, actor_id, purpose,
        decision, content_hash, reason) VALUES($1, $2, $3, $4, $5, $6, $7, $8)",
        vec![
            Uuid::new_v4().into(),
            entry_id.into(),
            version_id.into(),
            actor.into(),
            word(&r.purpose).into(),
            word(&r.decision).into(),
            r.content_hash.clone().into(),
            r.reason.clone().into(),
        ],
    )
    .await?;
    let state = match r.decision {
        LibraryReviewDecision::Approve => {
            exec(db,"INSERT INTO execution_approvals(definition_id,purpose,actor_id,content_hash) VALUES($1,$2,$3,$4)",vec![version_id.into(),word(&r.purpose).into(),actor.into(),r.content_hash.clone().into()]).await?;
            if definitions::approved(&definitions::get(db, app, version_id).await?) {
                LibraryReviewState::Approved
            } else {
                LibraryReviewState::InReview
            }
        }
        LibraryReviewDecision::NeedsInput => LibraryReviewState::NeedsInput,
        LibraryReviewDecision::Reject => LibraryReviewState::Rejected,
    };
    exec(
        db,
        "UPDATE test_library_versions SET review_state=$2,updated_at=now() WHERE definition_id=$1",
        vec![version_id.into(), word(&state).into()],
    )
    .await?;
    bump(db, entry_id, actor).await
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
    exec(db,"INSERT INTO test_library_versions(definition_id,entry_id,review_state) VALUES($1,$2,'in_review')",vec![definition.id.into(),id.into()]).await?;
    exec(
        db,
        "UPDATE test_library_entries SET next_version=GREATEST(next_version,$2) WHERE id=$1",
        vec![id.into(), next.into()],
    )
    .await?;
    bump(db, id, actor).await
}
