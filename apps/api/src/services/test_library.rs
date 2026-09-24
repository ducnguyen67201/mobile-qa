//! App-scoped catalog reads and shared lifecycle admission. Historical report reads
//! intentionally bypass admission: their manifests have already been frozen.
use super::{
    apps,
    execution_store::{conflict, decode, json, word},
    test_definitions as definitions,
};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        apps as app_rows, execution_definitions, execution_profiles, memberships,
        test_library_defaults, test_library_drafts, test_library_entries, test_library_versions,
        users,
    },
};
use loco_rs::app::AppContext;
use mobile_qa_contracts::{execution::*, test_library::*};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::HashMap;
use uuid::Uuid;

pub async fn authorize(ctx: &AppContext, actor: Uuid, app: Uuid) -> ApiResult<()> {
    apps::authorized(ctx, actor, app).await?;
    users::Entity::find_by_id(actor)
        .filter(users::Column::DisabledAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::unauthorized)?;
    Ok(())
}
pub async fn capabilities(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
) -> ApiResult<LibraryCapabilities> {
    let app_row = app_rows::Entity::find_by_id(app)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let operator = memberships::Entity::find()
        .filter(memberships::Column::OrganizationId.eq(app_row.organization_id))
        .filter(memberships::Column::UserId.eq(actor))
        .filter(memberships::Column::Active.eq(true))
        .filter(memberships::Column::Role.eq("operator"))
        .one(db)
        .await?
        .is_some();
    Ok(LibraryCapabilities {
        can_edit: true,
        can_archive: operator,
        can_set_default: operator,
    })
}

fn unsupported_saved_definition() -> ApiFailure {
    ApiFailure::new(
        409,
        "unsupported_test_schema",
        "This saved test uses a format this checkout cannot read",
    )
}

async fn require_supported_version(
    db: &impl ConnectionTrait,
    app: Uuid,
    version_id: Uuid,
) -> ApiResult<()> {
    let row = execution_definitions::Entity::find_by_id(version_id)
        .filter(execution_definitions::Column::AppId.eq(app))
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    serde_json::from_value::<TestDefinition>(row.payload)
        .map(|_| ())
        .map_err(|_| unsupported_saved_definition())
}

pub async fn entry(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
    id: Uuid,
) -> ApiResult<LibraryEntryResponse> {
    let row = test_library_entries::Entity::find_by_id(id)
        .filter(test_library_entries::Column::AppId.eq(app))
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let draft = test_library_drafts::Entity::find_by_id(id).one(db).await?;
    let mappings = test_library_versions::Entity::find()
        .filter(test_library_versions::Column::EntryId.eq(id))
        .all(db)
        .await?;
    let definition_ids = mappings
        .iter()
        .map(|mapping| mapping.definition_id)
        .collect::<Vec<_>>();
    let latest = if definition_ids.is_empty() {
        None
    } else {
        execution_definitions::Entity::find()
            .filter(execution_definitions::Column::Id.is_in(definition_ids))
            .order_by_desc(execution_definitions::Column::Version)
            .one(db)
            .await?
    };
    let archived_at = row.archived_at;
    let mut capabilities = capabilities(db, actor, app).await?;
    if archived_at.is_some() {
        capabilities.can_edit = false;
        capabilities.can_set_default = false;
    }
    let current = draft.as_ref().map(|draft| draft.payload.clone());
    let needs_setup = match current.clone() {
        Some(payload) => match serde_json::from_value::<LibraryDraftDefinition>(payload) {
            Ok(definition) => !content_issues(db, app, &definition).await?.0.is_empty(),
            Err(_) => {
                // Forward-schema records stay visible and intact, but cannot be
                // edited or chosen as the default by this older checkout.
                capabilities.can_edit = false;
                capabilities.can_set_default = false;
                true
            }
        },
        None => false,
    };
    let content_text = |payload: &serde_json::Value, key: &str| {
        payload
            .get("content")
            .and_then(|content| content.get(key))
            .and_then(serde_json::Value::as_str)
            .map(ToOwned::to_owned)
    };
    let title = current
        .as_ref()
        .and_then(|payload| content_text(payload, "title"))
        .or_else(|| {
            latest
                .as_ref()
                .and_then(|row| content_text(&row.payload, "title"))
        })
        .unwrap_or_else(|| row.logical_key.clone());
    let provenance = current
        .as_ref()
        .and_then(|payload| content_text(payload, "provenance"))
        .or_else(|| {
            latest
                .as_ref()
                .and_then(|row| content_text(&row.payload, "provenance"))
        })
        .unwrap_or_default();
    Ok(LibraryEntryResponse {
        id,
        app_id: app,
        kind: decode(json(&row.kind)?)?,
        key: row.logical_key,
        title,
        // Proposal provenance is server-owned and already exists on older saved AI tests.
        ai_generated: {
            provenance.starts_with("authored-task:")
                && (provenance.contains(":p:") || provenance.contains(":proposal:"))
        },
        needs_setup,
        revision: row.revision,
        archived_at,
        draft_version: draft.map(|draft| draft.version as u32),
        latest_version_id: latest.map(|version| version.id),
        updated_at: row.updated_at,
        capabilities,
    })
}
pub async fn list(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
    q: LibraryListQuery,
) -> ApiResult<LibraryListResponse> {
    if let Some(cursor) = q.cursor {
        entry(db, actor, app, cursor).await?;
    }
    let mut query = test_library_entries::Entity::find()
        .filter(test_library_entries::Column::AppId.eq(app))
        .filter(test_library_entries::Column::Kind.is_in(["case", "suite", "plan"]))
        .order_by_asc(test_library_entries::Column::Id)
        .limit(51);
    if let Some(kind) = q.kind {
        query = query.filter(test_library_entries::Column::Kind.eq(word(&kind)));
    }
    query = if q.archived.unwrap_or(false) {
        query.filter(test_library_entries::Column::ArchivedAt.is_not_null())
    } else {
        query.filter(test_library_entries::Column::ArchivedAt.is_null())
    };
    if let Some(cursor) = q.cursor {
        query = query.filter(test_library_entries::Column::Id.gt(cursor));
    }
    let ids = query.all(db).await?;
    let mut items = Vec::new();
    for row in ids.iter().take(50) {
        items.push(entry(db, actor, app, row.id).await?);
    }
    let next_cursor = if ids.len() > 50 {
        items.last().map(|e| e.id)
    } else {
        None
    };
    Ok(LibraryListResponse { items, next_cursor })
}
pub async fn admitted(db: &impl ConnectionTrait, app: Uuid, id: Uuid) -> ApiResult<()> {
    let mapping = test_library_versions::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let entry = test_library_entries::Entity::find_by_id(mapping.entry_id)
        .filter(test_library_entries::Column::AppId.eq(app))
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if entry.archived_at.is_some() {
        return Err(conflict("Referenced test is archived"));
    }
    definitions::get(db, app, id)
        .await?
        .definition
        .validate()
        .map_err(ApiFailure::invalid)?;
    Ok(())
}
/// Draft coverage does not require a build or profile. It resolves saved,
/// active references and reports conflicts instead of silently picking precedence.
pub async fn coverage(
    db: &impl ConnectionTrait,
    app: Uuid,
    definition: &LibraryDraftDefinition,
) -> ApiResult<LibraryCoveragePreview> {
    let mut out = LibraryCoveragePreview {
        cases: vec![],
        required_count: 0,
        exclusions: vec![],
        issues: vec![],
    };
    let (mut selections, suites) = match definition {
        LibraryDraftDefinition::Case(_) => return Ok(out),
        LibraryDraftDefinition::Suite(s) => (s.cases.clone(), vec![]),
        LibraryDraftDefinition::Plan(p) => {
            out.exclusions = p.exclusions.clone();
            (p.cases.clone(), p.suite_version_ids.clone())
        }
    };
    for id in suites {
        let result = async {
            admitted(db, app, id).await?;
            let d = definitions::get(db, app, id).await?;
            match d.definition {
                TestDefinition::Suite(s) => Ok(s.cases),
                _ => Err(ApiFailure::invalid("Choose a suite version")),
            }
        }
        .await;
        match result {
            Ok(cases) => selections.extend(cases),
            Err(e) => {
                if e.status.is_server_error() {
                    return Err(e);
                }
                out.issues.push(LibraryIssue {
                    code: LibraryIssueCode::InvalidReference,
                    field: "suite_version_ids".into(),
                    item_id: Some(id.to_string()),
                    message: "Suite version is unavailable, archived or invalid".into(),
                });
            }
        }
    }
    for selection in selections {
        let result = async {
            admitted(db, app, selection.case_version_id).await?;
            let d = definitions::get(db, app, selection.case_version_id).await?;
            let TestDefinition::Case(case) = d.definition else {
                return Err(ApiFailure::invalid("Choose a case version"));
            };
            Ok(ResolvedCase {
                definition_id: d.id,
                content_hash: d.content_hash,
                data_variant: selection.data_variant.clone(),
                required: selection.required,
                case,
            })
        }
        .await;
        match result {
            Ok(case) => {
                if let Some(previous) = out.cases.iter().find(|c| c.case.key == case.case.key) {
                    if previous.definition_id != case.definition_id
                        || previous.required != case.required
                        || previous.data_variant != case.data_variant
                    {
                        out.issues.push(LibraryIssue {
                            code: LibraryIssueCode::ConflictingSelection,
                            field: "cases".into(),
                            item_id: Some(case.definition_id.to_string()),
                            message: "The same case has conflicting versions or requiredness"
                                .into(),
                        });
                    }
                } else {
                    out.cases.push(case);
                }
            }
            Err(e) => {
                if e.status.is_server_error() {
                    return Err(e);
                }
                out.issues.push(LibraryIssue {
                    code: LibraryIssueCode::InvalidReference,
                    field: "cases".into(),
                    item_id: Some(selection.case_version_id.to_string()),
                    message: "Case version is unavailable, archived or invalid".into(),
                });
            }
        }
    }
    out.required_count = out.cases.iter().filter(|c| c.required).count() as u32;
    if out.cases.is_empty() || out.cases.len() > 100 || out.required_count == 0 {
        out.issues.push(LibraryIssue::new(
            LibraryIssueCode::Required,
            "cases",
            "Choose 1–100 unique cases, including required coverage",
        ));
    }
    out.issues.truncate(100);
    Ok(out)
}
pub async fn content_issues(
    db: &impl ConnectionTrait,
    app: Uuid,
    definition: &LibraryDraftDefinition,
) -> ApiResult<(Vec<LibraryIssue>, LibraryCoveragePreview)> {
    let mut issues = definition.issues();
    let coverage = coverage(db, app, definition).await?;
    issues.extend(coverage.issues.clone());
    if let LibraryDraftDefinition::Plan(p) = definition {
        if let Some(id) = p.profile_id {
            match definitions::profile(db, app, id).await {
                Ok(_) => {}
                Err(e) => {
                    if e.status.is_server_error() {
                        return Err(e);
                    }
                    issues.push(LibraryIssue::new(
                        LibraryIssueCode::InvalidReference,
                        "profile_id",
                        "Choose an available execution profile",
                    ));
                }
            }
        }
    }
    issues.truncate(100);
    Ok((issues, coverage))
}
pub async fn draft(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
    id: Uuid,
) -> ApiResult<LibraryDraftResponse> {
    let entry = entry(db, actor, app, id).await?;
    let stored = test_library_drafts::Entity::find_by_id(id).one(db).await?;
    let (definition, source_version_id): (LibraryDraftDefinition, Option<Uuid>) =
        if let Some(row) = stored {
            (
                serde_json::from_value(row.payload).map_err(|_| unsupported_saved_definition())?,
                row.source_version_id,
            )
        } else {
            let version_id = entry.latest_version_id.ok_or_else(ApiFailure::missing)?;
            require_supported_version(db, app, version_id).await?;
            (
                definitions::get(db, app, version_id)
                    .await?
                    .definition
                    .into(),
                Some(version_id),
            )
        };
    let saved_version_id = if let Some(version_id) = source_version_id {
        let saved = definitions::get(db, app, version_id).await?;
        (definition.published().ok().as_ref()
            == LibraryDraftDefinition::from(saved.definition)
                .published()
                .ok()
                .as_ref())
        .then_some(version_id)
    } else {
        None
    };
    let (issues, coverage) = content_issues(db, app, &definition).await?;
    Ok(LibraryDraftResponse {
        entry,
        definition,
        source_version_id,
        saved_version_id,
        issues,
        coverage,
    })
}
pub async fn version(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
    id: Uuid,
    version_id: Uuid,
) -> ApiResult<LibraryVersionResponse> {
    let entry = entry(db, actor, app, id).await?;
    test_library_versions::Entity::find_by_id(version_id)
        .filter(test_library_versions::Column::EntryId.eq(id))
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    require_supported_version(db, app, version_id).await?;
    let version = definitions::get(db, app, version_id).await?;
    let (mut issues, coverage) =
        content_issues(db, app, &version.definition.clone().into()).await?;
    if let Err(e) = executable(db, app, &version.definition).await {
        if e.status.is_server_error() {
            return Err(e);
        }
        issues.push(LibraryIssue::new(
            LibraryIssueCode::UnsupportedCapability,
            "definition",
            e.message,
        ));
    }
    issues.truncate(100);
    Ok(LibraryVersionResponse {
        entry,
        version,
        coverage,
        issues,
    })
}
pub async fn versions(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
    id: Uuid,
    cursor: Option<Uuid>,
) -> ApiResult<LibraryVersionListResponse> {
    entry(db, actor, app, id).await?;
    if let Some(cursor) = cursor {
        test_library_versions::Entity::find_by_id(cursor)
            .filter(test_library_versions::Column::EntryId.eq(id))
            .one(db)
            .await?
            .ok_or_else(ApiFailure::missing)?;
    }
    let mappings = test_library_versions::Entity::find()
        .filter(test_library_versions::Column::EntryId.eq(id))
        .all(db)
        .await?;
    let definition_ids = mappings
        .into_iter()
        .map(|mapping| mapping.definition_id)
        .collect::<Vec<_>>();
    let mut definitions = if definition_ids.is_empty() {
        vec![]
    } else {
        execution_definitions::Entity::find()
            .filter(execution_definitions::Column::Id.is_in(definition_ids))
            .all(db)
            .await?
    };
    definitions.sort_by_key(|definition| std::cmp::Reverse(definition.version));
    if let Some(cursor) = cursor {
        let cursor_version = definitions
            .iter()
            .find(|definition| definition.id == cursor)
            .map(|definition| definition.version)
            .ok_or_else(ApiFailure::missing)?;
        definitions.retain(|definition| definition.version < cursor_version);
    }
    definitions.truncate(26);
    let mut items = Vec::new();
    for definition in definitions.iter().take(25) {
        items.push(version(db, actor, app, id, definition.id).await?);
    }
    let next_cursor = if definitions.len() > 25 {
        items.last().map(|v| v.version.id)
    } else {
        None
    };
    Ok(LibraryVersionListResponse { items, next_cursor })
}
pub async fn options(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
) -> ApiResult<LibraryOptionsResponse> {
    let mut profiles = Vec::new();
    for row in execution_profiles::Entity::find()
        .filter(execution_profiles::Column::AppId.eq(app))
        .order_by_asc(execution_profiles::Column::Id)
        .all(db)
        .await?
    {
        let p: ExecutionProfile = decode(row.payload)?;
        let assignment = super::model_registry::active_assignment(
            db,
            app,
            p.id,
            mobile_qa_contracts::model_registry::ModelPurpose::Navigation,
        )
        .await?;
        let resolved_model = if let Some((model, _)) = assignment {
            Some(model)
        } else {
            super::model_registry::resolve_for_new_work(db, p.model.as_ref(), &[])
                .await
                .ok()
                .flatten()
        };
        let model_available = !p.requires_model() || resolved_model.is_some();
        profiles.push(LibraryProfileChoice {
            id: p.id,
            name: p.name,
            driver: p.driver,
            package: p.package,
            adapter: p.adapter,
            qualified: p.qualified,
            resolved_model,
            model_available,
        });
    }
    let entries = test_library_entries::Entity::find()
        .filter(test_library_entries::Column::AppId.eq(app))
        .filter(test_library_entries::Column::ArchivedAt.is_null())
        .filter(test_library_entries::Column::Kind.is_in(["case", "suite", "plan"]))
        .order_by_asc(test_library_entries::Column::LogicalKey)
        .all(db)
        .await?;
    let entry_order = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.id, index))
        .collect::<HashMap<_, _>>();
    let entry_ids = entries.iter().map(|entry| entry.id).collect::<Vec<_>>();
    let mut mappings = if entry_ids.is_empty() {
        vec![]
    } else {
        test_library_versions::Entity::find()
            .filter(test_library_versions::Column::EntryId.is_in(entry_ids))
            .order_by_desc(test_library_versions::Column::CreatedAt)
            .order_by_asc(test_library_versions::Column::DefinitionId)
            .all(db)
            .await?
    };
    mappings.sort_by_key(|mapping| {
        entry_order
            .get(&mapping.entry_id)
            .copied()
            .unwrap_or(usize::MAX)
    });
    let definition_ids = mappings
        .iter()
        .map(|mapping| mapping.definition_id)
        .collect::<Vec<_>>();
    let definitions = if definition_ids.is_empty() {
        vec![]
    } else {
        execution_definitions::Entity::find()
            .filter(execution_definitions::Column::Id.is_in(definition_ids))
            .all(db)
            .await?
    };
    let definitions = definitions
        .into_iter()
        .map(|definition| (definition.id, definition))
        .collect::<HashMap<_, _>>();
    let mut saved_versions = Vec::new();
    for mapping in mappings {
        let Some(definition) = definitions.get(&mapping.definition_id) else {
            continue;
        };
        if serde_json::from_value::<TestDefinition>(definition.payload.clone()).is_err() {
            continue;
        }
        saved_versions
            .push(version(db, actor, app, mapping.entry_id, mapping.definition_id).await?);
    }
    Ok(LibraryOptionsResponse {
        profiles,
        saved_versions,
        capabilities: capabilities(db, actor, app).await?,
    })
}
pub async fn default_plan(db: &impl ConnectionTrait, app: Uuid) -> ApiResult<DefaultPlanResponse> {
    match test_library_defaults::Entity::find_by_id(app)
        .one(db)
        .await?
    {
        Some(row) => Ok(DefaultPlanResponse {
            app_id: app,
            revision: row.revision,
            plan_version_id: Some(row.plan_version_id),
        }),
        None => Ok(DefaultPlanResponse {
            app_id: app,
            revision: 0,
            plan_version_id: None,
        }),
    }
}
pub async fn executable(
    db: &impl ConnectionTrait,
    app: Uuid,
    definition: &TestDefinition,
) -> ApiResult<()> {
    match definition {
        TestDefinition::Case(c) => {
            let package = app_rows::Entity::find_by_id(app)
                .one(db)
                .await?
                .ok_or_else(ApiFailure::missing)?
                .android_package;
            if c.package != package {
                return Err(ApiFailure::invalid("Test belongs to another app"));
            }
            if c.adapter == "demo_persistence_v1" && c.package == "ai.mobileqa.demo" {
                if c.checks.iter().any(|c| c.method == CheckMethod::Manual) {
                    return Err(ApiFailure::invalid("Automatic checks are required"));
                }
            } else {
                let p = super::execution_readiness::assigned(db, app, &package).await?;
                super::execution_readiness::case_matches(&p, c)?;
            }
        }
        TestDefinition::Suite(s) => {
            let preview = coverage(db, app, &LibraryDraftDefinition::Suite(s.clone())).await?;
            if !preview.issues.is_empty() {
                return Err(validation(preview.issues));
            }
        }
        TestDefinition::Plan(p) => {
            let profile = definitions::profile(db, app, p.profile_id).await?;
            if !profile.qualified {
                return Err(ApiFailure::invalid("Execution profile is not qualified"));
            }
            let cases = definitions::resolve(db, app, p).await?;
            for c in &cases {
                super::execution_readiness::case_matches(&profile, &c.case)?;
            }
            if cases.iter().any(|c| {
                c.case.budget.max_steps > p.budget.max_steps
                    || c.case.budget.artifact_bytes > p.budget.artifact_bytes
            }) || cases
                .iter()
                .map(|c| {
                    super::execution_readiness::duration(&profile, &c.case)
                        * (u64::from(p.diagnostic_retries) + 1)
                })
                .sum::<u64>()
                > u64::from(p.budget.duration_seconds)
            {
                return Err(ApiFailure::invalid(
                    "Profile and plan budget must cover every selected case",
                ));
            }
        }
    }
    Ok(())
}
pub fn validation(issues: Vec<LibraryIssue>) -> ApiFailure {
    let status = if issues
        .iter()
        .any(|i| i.code == LibraryIssueCode::ConflictingSelection)
    {
        409
    } else {
        422
    };
    ApiFailure::new(
        status,
        "library_validation",
        "Resolve the listed issues before running",
    )
    .with_library_details(LibraryErrorDetails::Validation { issues })
}
