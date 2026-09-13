//! App-scoped catalog reads and shared lifecycle admission. Historical report reads
//! intentionally bypass admission: their manifests have already been frozen.
use super::{apps, execution_store::*, test_definitions as definitions};
use crate::errors::{ApiFailure, ApiResult};
use loco_rs::app::AppContext;
use mobile_qa_contracts::{execution::*, test_library::*};
use sea_orm::ConnectionTrait;
use uuid::Uuid;

pub async fn authorize(ctx: &AppContext, actor: Uuid, app: Uuid) -> ApiResult<()> {
    apps::authorized(ctx, actor, app).await?;
    one(
        &ctx.db,
        "SELECT id FROM users WHERE id=$1 AND disabled_at IS NULL",
        vec![actor.into()],
    )
    .await
    .map_err(|_| ApiFailure::unauthorized())?;
    Ok(())
}
pub async fn capabilities(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
) -> ApiResult<LibraryCapabilities> {
    let operator: bool=field(&one(db,
        "SELECT EXISTS(SELECT 1 FROM memberships m JOIN apps a ON a.organization_id=m.organization_id
         WHERE a.id=$1 AND m.user_id=$2 AND m.active AND m.role='operator') AS allowed",
        vec![app.into(),actor.into()]).await?, "allowed")?;
    let grants = rows(
        db,
        "SELECT purpose FROM execution_reviewer_grants WHERE app_id=$1 AND user_id=$2",
        vec![app.into(), actor.into()],
    )
    .await?;
    let purposes = grants
        .iter()
        .map(|r| field::<String>(r, "purpose"))
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(LibraryCapabilities {
        can_edit: true,
        can_review_business: purposes.iter().any(|p| p == "business"),
        can_review_executability: purposes.iter().any(|p| p == "executability"),
        can_archive: operator,
        can_set_default: operator,
    })
}
pub async fn entry(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
    id: Uuid,
) -> ApiResult<LibraryEntryResponse> {
    let row=one(db,"SELECT e.*, d.version AS draft_version,
        COALESCE(d.payload->'content'->>'title',v.title,e.logical_key) AS title,
        v.definition_id AS latest_version_id,v.review_state AS latest_review_state
        FROM test_library_entries e LEFT JOIN test_library_drafts d ON d.entry_id=e.id
        LEFT JOIN LATERAL (SELECT lv.definition_id,lv.review_state,ed.payload->'content'->>'title' AS title
          FROM test_library_versions lv JOIN execution_definitions ed ON ed.id=lv.definition_id
          WHERE lv.entry_id=e.id ORDER BY ed.version DESC LIMIT 1) v ON true
        WHERE e.id=$1 AND e.app_id=$2",vec![id.into(),app.into()]).await?;
    let archived_at: Option<chrono::DateTime<chrono::Utc>> = field(&row, "archived_at")?;
    let mut capabilities = capabilities(db, actor, app).await?;
    if archived_at.is_some() {
        capabilities.can_edit = false;
        capabilities.can_review_business = false;
        capabilities.can_review_executability = false;
        capabilities.can_set_default = false;
    }
    Ok(LibraryEntryResponse {
        id,
        app_id: app,
        kind: decode(json(&field::<String>(&row, "kind")?)?)?,
        key: field(&row, "logical_key")?,
        title: field(&row, "title")?,
        revision: field(&row, "revision")?,
        archived_at,
        draft_version: field::<Option<i32>>(&row, "draft_version")?.map(|v| v as u32),
        latest_version_id: field(&row, "latest_version_id")?,
        latest_review_state: field::<Option<String>>(&row, "latest_review_state")?
            .map(|s| decode(json(&s)?))
            .transpose()?,
        updated_at: field(&row, "updated_at")?,
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
    let ids = rows(
        db,
        "SELECT e.id FROM test_library_entries e
        WHERE e.app_id=$1 AND ($2::text IS NULL OR e.kind=$2) AND (e.archived_at IS NOT NULL)=$3
        AND ($4::uuid IS NULL OR e.id>$4)
        AND ($5::text IS NULL OR (SELECT v.review_state FROM test_library_versions v
          JOIN execution_definitions d ON d.id=v.definition_id WHERE v.entry_id=e.id
          ORDER BY d.version DESC LIMIT 1)=$5) ORDER BY e.id LIMIT 51",
        vec![
            app.into(),
            q.kind.map(|k| word(&k)).into(),
            q.archived.unwrap_or(false).into(),
            q.cursor.into(),
            q.status.map(|s| word(&s)).into(),
        ],
    )
    .await?;
    let mut items = Vec::new();
    for r in ids.iter().take(50) {
        items.push(entry(db, actor, app, field(r, "id")?).await?);
    }
    let next_cursor = if ids.len() > 50 {
        items.last().map(|e| e.id)
    } else {
        None
    };
    Ok(LibraryListResponse { items, next_cursor })
}
pub async fn admitted(db: &impl ConnectionTrait, app: Uuid, id: Uuid) -> ApiResult<()> {
    let r = one(
        db,
        "SELECT e.archived_at,v.review_state FROM test_library_versions v
        JOIN test_library_entries e ON e.id=v.entry_id WHERE v.definition_id=$1 AND e.app_id=$2",
        vec![id.into(), app.into()],
    )
    .await?;
    if field::<Option<chrono::DateTime<chrono::Utc>>>(&r, "archived_at")?.is_some() {
        return Err(conflict("Referenced test is archived"));
    }
    if field::<String>(&r, "review_state")? != "approved" {
        return Err(conflict("Referenced version needs both approvals"));
    }
    Ok(())
}
/// Draft coverage does not require a build or profile. It resolves only approved,
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
            if !definitions::approved(&d) {
                return Err(conflict("Suite needs both approvals"));
            }
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
                    message: "Suite version is unavailable, archived or awaiting review".into(),
                });
            }
        }
    }
    for selection in selections {
        let result = async {
            admitted(db, app, selection.case_version_id).await?;
            let d = definitions::get(db, app, selection.case_version_id).await?;
            if !definitions::approved(&d) {
                return Err(conflict("Case needs both approvals"));
            }
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
                    message: "Case version is unavailable, archived or awaiting review".into(),
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
                Ok(profile) => {
                    if !profile.qualified {
                        issues.push(LibraryIssue::new(
                            LibraryIssueCode::UnsupportedCapability,
                            "profile_id",
                            "Profile is not qualified",
                        ));
                    }
                }
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
    let row = one(
        db,
        "SELECT payload,source_version_id FROM test_library_drafts WHERE entry_id=$1",
        vec![id.into()],
    )
    .await?;
    let definition = decode(field(&row, "payload")?)?;
    let (issues, coverage) = content_issues(db, app, &definition).await?;
    Ok(LibraryDraftResponse {
        entry,
        definition,
        source_version_id: field(&row, "source_version_id")?,
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
    let r = one(
        db,
        "SELECT review_state FROM test_library_versions WHERE entry_id=$1 AND definition_id=$2",
        vec![id.into(), version_id.into()],
    )
    .await?;
    let version = definitions::get(db, app, version_id).await?;
    let events = rows(
        db,
        "SELECT e.*, u.display_name AS actor_name FROM test_library_review_events e JOIN users u ON
        u.id=e.actor_id WHERE e.definition_id=$1 ORDER BY e.created_at, e.id",
        vec![version_id.into()],
    )
    .await?;
    let review_events = events
        .iter()
        .map(|r| {
            Ok(LibraryReviewEvent {
                id: field(r, "id")?,
                actor_id: field(r, "actor_id")?,
                actor_name: field(r, "actor_name")?,
                purpose: decode(json(&field::<String>(r, "purpose")?)?)?,
                decision: decode(json(&field::<String>(r, "decision")?)?)?,
                content_hash: field(r, "content_hash")?,
                reason: field(r, "reason")?,
                created_at: field(r, "created_at")?,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
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
        review_state: decode(json(&field::<String>(&r, "review_state")?)?)?,
        review_events,
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
        one(db,"SELECT definition_id FROM test_library_versions WHERE entry_id=$1 AND definition_id=$2",vec![id.into(),cursor.into()]).await?;
    }
    let ids = rows(
        db,
        "SELECT v.definition_id FROM test_library_versions v JOIN execution_definitions d ON
        d.id=v.definition_id WHERE v.entry_id=$1 AND ($2::uuid IS NULL OR d.version<(SELECT version
        FROM execution_definitions WHERE id=$2)) ORDER BY d.version DESC LIMIT 26",
        vec![id.into(), cursor.into()],
    )
    .await?;
    let mut items = Vec::new();
    for row in ids.iter().take(25) {
        items.push(version(db, actor, app, id, field(row, "definition_id")?).await?);
    }
    let next_cursor = if ids.len() > 25 {
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
    for row in rows(
        db,
        "SELECT payload FROM execution_profiles WHERE app_id=$1 ORDER BY id",
        vec![app.into()],
    )
    .await?
    {
        let p: ExecutionProfile = decode(field(&row, "payload")?)?;
        profiles.push(LibraryProfileChoice {
            id: p.id,
            name: p.name,
            driver: p.driver,
            package: p.package,
            adapter: p.adapter,
            qualified: p.qualified,
        });
    }
    let ids=rows(db,"SELECT v.entry_id, v.definition_id FROM test_library_versions v JOIN test_library_entries e
        ON e.id=v.entry_id WHERE e.app_id=$1 AND e.archived_at IS NULL AND v.review_state='approved'
        ORDER BY e.logical_key, v.created_at DESC, v.definition_id",vec![app.into()]).await?;
    let mut approved_versions = Vec::new();
    for r in ids {
        approved_versions.push(
            version(
                db,
                actor,
                app,
                field(&r, "entry_id")?,
                field(&r, "definition_id")?,
            )
            .await?,
        );
    }
    Ok(LibraryOptionsResponse {
        profiles,
        approved_versions,
        capabilities: capabilities(db, actor, app).await?,
    })
}
pub async fn default_plan(db: &impl ConnectionTrait, app: Uuid) -> ApiResult<DefaultPlanResponse> {
    let r = rows(
        db,
        "SELECT revision,plan_version_id FROM test_library_defaults WHERE app_id=$1",
        vec![app.into()],
    )
    .await?;
    match r.first() {
        Some(r) => Ok(DefaultPlanResponse {
            app_id: app,
            revision: field(r, "revision")?,
            plan_version_id: Some(field(r, "plan_version_id")?),
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
            let package: String = field(
                &one(
                    db,
                    "SELECT android_package FROM apps WHERE id=$1",
                    vec![app.into()],
                )
                .await?,
                "android_package",
            )?;
            if c.adapter != "demo_persistence_v1"
                || c.package != "ai.mobileqa.demo"
                || c.package != package
                || c.checks.iter().any(|c| c.method == CheckMethod::Manual)
            {
                return Err(ApiFailure::invalid("Automatic execution currently supports the demo persistence adapter and automated checks only"));
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
            if cases.iter().any(|c| {
                c.case.package != profile.package
                    || c.case.adapter != profile.adapter
                    || c.case.budget.max_steps > p.budget.max_steps
                    || c.case.budget.artifact_bytes > p.budget.artifact_bytes
            }) || cases
                .iter()
                .map(|c| {
                    u64::from(c.case.budget.duration_seconds)
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
        "Resolve the listed issues before requesting review",
    )
    .with_library_details(LibraryErrorDetails::Validation { issues })
}
