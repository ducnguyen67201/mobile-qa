//! Save editor content and its immutable executable snapshot in the caller's app transaction.
use super::{
    execution_store::{conflict, hash, json, word},
    test_definitions as definitions, test_library as library,
};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        execution_definitions, test_library_drafts, test_library_entries, test_library_versions,
    },
};
use mobile_qa_contracts::test_library::*;
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ConnectionTrait, EntityTrait, IntoActiveModel, Set,
};
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
                test_library_entries::Entity::find_by_id(id)
                    .one(db)
                    .await?
                    .ok_or_else(ApiFailure::missing)?
                    .next_version
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
            execution_definitions::ActiveModel {
                id: Set(version_id),
                app_id: Set(app),
                kind: Set(word(&kind)),
                logical_key: Set(key.to_owned()),
                version: Set(next),
                content_hash: Set(digest),
                payload: Set(payload),
                author_id: Set(actor),
                ..Default::default()
            }
            .insert(db)
            .await?;
            test_library_versions::ActiveModel {
                definition_id: Set(version_id),
                entry_id: Set(id),
                ..Default::default()
            }
            .insert(db)
            .await?;
            let entry = test_library_entries::Entity::find_by_id(id)
                .one(db)
                .await?
                .ok_or_else(ApiFailure::missing)?;
            if entry.next_version < next + 1 {
                let mut active = entry.into_active_model();
                active.next_version = Set(next + 1);
                active.update(db).await?;
            }
            saved_id = Some(version_id);
        }
    }
    // NULL means the current saved editor content is incomplete, not permission to run an older snapshot.
    test_library_drafts::Entity::insert(test_library_drafts::ActiveModel {
        entry_id: Set(id),
        version: Set(content.identity().2 as i32),
        source_version_id: Set(saved_id),
        payload: Set(json(&content)?),
        editor_id: Set(actor),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(test_library_drafts::Column::EntryId)
            .update_columns([
                test_library_drafts::Column::Version,
                test_library_drafts::Column::SourceVersionId,
                test_library_drafts::Column::Payload,
                test_library_drafts::Column::EditorId,
                test_library_drafts::Column::UpdatedAt,
            ])
            .to_owned(),
    )
    .exec_without_returning(db)
    .await?;
    Ok(())
}
