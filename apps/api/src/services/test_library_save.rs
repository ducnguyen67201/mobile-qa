//! Save editor content and its immutable executable snapshot in the caller's app transaction.
use super::{execution_store::*, test_definitions as definitions, test_library as library};
use crate::errors::{ApiFailure, ApiResult};
use mobile_qa_contracts::test_library::*;
use sea_orm::ConnectionTrait;
use uuid::Uuid;

pub async fn save(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
    id: Uuid,
    mut content: LibraryDraftDefinition,
) -> ApiResult<()> {
    let original = library::draft(db, actor, app, id).await?;
    if content.identity() != original.definition.identity() {
        return Err(conflict("Test kind, key and version cannot change"));
    }
    if let LibraryDraftDefinition::Case(ref mut case) = content {
        let LibraryDraftDefinition::Case(previous) = &original.definition else {
            return Err(ApiFailure::internal());
        };
        if case.package != previous.package {
            return Err(ApiFailure::invalid("The test package belongs to its app"));
        }
        case.provenance = previous.provenance.clone();
    }
    content.check_bounds().map_err(ApiFailure::invalid)?;
    if serde_json::to_vec(&content)
        .map_err(|_| ApiFailure::internal())?
        .len()
        > 1048576
    {
        return Err(ApiFailure::new(
            413,
            "draft_too_large",
            "Test exceeds 1 MiB",
        ));
    }
    let latest = if let Some(version) = original.entry.latest_version_id {
        Some(definitions::get(db, app, version).await?)
    } else {
        None
    };
    // Compare at the old server-owned version, including when an incomplete edit was saved.
    let mut comparable = content.clone();
    if let Some(ref latest) = latest {
        comparable.allocate(latest.definition.identity().2);
    }
    let (issues, _) = library::content_issues(db, app, &content).await?;
    let mut saved_id = None;
    if issues.is_empty() {
        if let Some(ref latest) = latest {
            if comparable.published().ok().as_ref()
                == LibraryDraftDefinition::from(latest.definition.clone())
                    .published()
                    .ok()
                    .as_ref()
            {
                saved_id = Some(latest.id);
                content.allocate(latest.definition.identity().2);
            }
        }
        if saved_id.is_none() {
            let next: i32 = if latest.is_none() {
                content.identity().2 as i32
            } else {
                field(
                    &one(
                        db,
                        "SELECT next_version FROM test_library_entries WHERE id=$1",
                        vec![id.into()],
                    )
                    .await?,
                    "next_version",
                )?
            };
            if next == i32::MAX {
                return Err(conflict("Version limit reached"));
            }
            content.allocate(next as u32);
            let definition = content
                .published()
                .map_err(|i| library::validation(vec![i]))?;
            definition.validate().map_err(ApiFailure::invalid)?;
            let payload = json(&definition)?;
            let digest = hash(serde_json::to_vec(&payload).map_err(|_| ApiFailure::internal())?);
            let version_id = Uuid::new_v4();
            let (kind, key, _) = definition.identity();
            exec(db, "INSERT INTO execution_definitions(id,app_id,kind,logical_key,version,content_hash,payload,author_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8)", vec![version_id.into(),app.into(),word(&kind).into(),key.into(),next.into(),digest.into(),payload.into(),actor.into()]).await?;
            exec(
                db,
                "INSERT INTO test_library_versions(definition_id,entry_id) VALUES($1,$2)",
                vec![version_id.into(), id.into()],
            )
            .await?;
            exec(db, "UPDATE test_library_entries SET next_version=GREATEST(next_version,$2) WHERE id=$1", vec![id.into(),(next+1).into()]).await?;
            saved_id = Some(version_id);
        }
    }
    // NULL means the current saved editor content is incomplete, not permission to run an older snapshot.
    exec(db, "INSERT INTO test_library_drafts(entry_id,version,source_version_id,payload,editor_id) VALUES($1,$2,$3,$4,$5) ON CONFLICT(entry_id) DO UPDATE SET version=EXCLUDED.version,source_version_id=EXCLUDED.source_version_id,payload=EXCLUDED.payload,editor_id=EXCLUDED.editor_id,updated_at=now()", vec![id.into(),(content.identity().2 as i32).into(),saved_id.into(),json(&content)?.into(),actor.into()]).await?;
    Ok(())
}
