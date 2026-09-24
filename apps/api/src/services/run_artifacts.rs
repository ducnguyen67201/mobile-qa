//! Evidence publication is pending until bounded bytes and their digest have been verified.
use super::{
    execution_store::{conflict, decode, hash},
    scheduler,
    worker_auth::Worker,
};
use crate::{
    config::Setup,
    errors::{ApiFailure, ApiResult},
    models::_entities::{execution_artifacts, execution_attempts},
    storage::Scratch,
};
use axum::body::Body;
use futures_util::StreamExt;
use loco_rs::app::AppContext;
use mobile_qa_contracts::execution::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    QuerySelect, TransactionTrait,
};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

pub fn record(r: &execution_artifacts::Model) -> ApiResult<RunArtifact> {
    Ok(RunArtifact {
        id: r.id,
        attempt_id: r.attempt_id,
        checkpoint_id: r.checkpoint_id.clone(),
        name: r.name.clone(),
        mime: r.mime.clone(),
        byte_size: r.byte_size as u32,
        sha256: r.sha256.clone(),
        state: decode(serde_json::Value::String(r.state.clone()))?,
        reason: r.reason.clone(),
    })
}

pub async fn reserve(
    ctx: &AppContext,
    w: &Worker,
    id: Uuid,
    token: &str,
    input: ArtifactRequest,
) -> ApiResult<ArtifactReceipt> {
    if !bounded(&input.name, 120)
        || !input
            .name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        || input.sha256.len() != 64
        || !input
            .sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || input.byte_size == 0
        || input.byte_size > 16777216
        || !["image/png", "application/xml", "text/plain", "video/mp4"]
            .contains(&input.mime.as_str())
    {
        return Err(ApiFailure::invalid("Unsupported evidence metadata"));
    }
    let tx = ctx.db.begin().await?;
    let lease = scheduler::lease(&tx, w, id, input.generation, token, true).await?;
    let old = execution_artifacts::Entity::find()
        .filter(execution_artifacts::Column::AttemptId.eq(id))
        .filter(execution_artifacts::Column::Name.eq(input.name.clone()))
        .one(&tx)
        .await?;
    if let Some(row) = old {
        let artifact = record(&row)?;
        if artifact.sha256 != input.sha256
            || artifact.byte_size != input.byte_size
            || artifact.checkpoint_id != input.checkpoint_id
            || artifact.mime != input.mime
        {
            return Err(conflict("Artifact identity already used"));
        }
        tx.commit().await?;
        return Ok(ArtifactReceipt { artifact });
    }
    if !["leased", "running", "cancel_requested"].contains(&lease.attempt.state.as_str()) {
        return Err(conflict("Attempt no longer accepts artifacts"));
    }
    let manifest: RunManifest = decode(lease.run.manifest)?;
    let c = &manifest.cases[lease.attempt.case_index as usize].case;
    let preflight = manifest.profile.execution_context.is_some()
        && input.checkpoint_id == "preflight"
        && matches!(
            (input.name.as_str(), input.mime.as_str()),
            ("preflight.xml", "application/xml") | ("preflight.png", "image/png")
        );
    if !preflight {
        super::execution_preflight::require(&tx, id, input.generation, &manifest).await?;
    }
    if !preflight
        && !c
            .actions
            .iter()
            .any(|a| a.checkpoint_id == input.checkpoint_id)
    {
        return Err(ApiFailure::invalid("Unknown checkpoint"));
    }
    let used = execution_artifacts::Entity::find()
        .filter(execution_artifacts::Column::AttemptId.eq(id))
        .all(&tx)
        .await?
        .into_iter()
        .map(|artifact| artifact.byte_size)
        .sum::<i64>();
    if used + i64::from(input.byte_size) > i64::from(c.budget.artifact_bytes) {
        return Err(ApiFailure::new(
            413,
            "evidence_budget",
            "Evidence byte budget exceeded",
        ));
    }
    let artifact = Uuid::new_v4();
    let run_id = lease.attempt.run_id;
    let key = format!("{}/{}/{}/{}", w.app_id, run_id, id, artifact);
    let setup = Setup::get(ctx);
    let row = execution_artifacts::ActiveModel {
        id: Set(artifact),
        attempt_id: Set(id),
        checkpoint_id: Set(input.checkpoint_id),
        name: Set(input.name),
        mime: Set(input.mime),
        byte_size: Set(i64::from(input.byte_size)),
        sha256: Set(input.sha256),
        storage_key: Set(key),
        storage_backend: Set(setup.storage_backend),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    let result = record(&row)?;
    tx.commit().await?;
    Ok(ArtifactReceipt { artifact: result })
}

pub async fn content(ctx: &AppContext, id: Uuid) -> ApiResult<(RunArtifact, Scratch)> {
    let row = execution_artifacts::Entity::find_by_id(id)
        .filter(execution_artifacts::Column::State.eq("sealed"))
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let artifact = record(&row)?;
    let setup = Setup::get(ctx);
    let file = setup
        .store
        .materialize(&row.storage_key, &row.storage_backend)
        .await?;
    let bytes = tokio::fs::read(&file.0).await?;
    if bytes.len() != artifact.byte_size as usize || hash(&bytes) != artifact.sha256 {
        return Err(ApiFailure::new(
            503,
            "evidence_changed",
            "Stored evidence is unavailable",
        ));
    }
    Ok((artifact, file))
}

pub async fn upload(
    ctx: &AppContext,
    w: &Worker,
    id: Uuid,
    artifact: Uuid,
    g: i32,
    token: &str,
    body: Body,
) -> ApiResult<ArtifactReceipt> {
    scheduler::lease(&ctx.db, w, id, g, token, false).await?;
    let row = execution_artifacts::Entity::find_by_id(artifact)
        .filter(execution_artifacts::Column::AttemptId.eq(id))
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let metadata = record(&row)?;
    let setup = Setup::get(ctx);
    let (scratch, mut file) = setup.store.scratch().await?;
    let transfer = async {
        let mut stream = body.into_data_stream();
        let mut bytes = 0u64;
        let mut hash = Sha256::new();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|_| ApiFailure::invalid("Evidence transfer interrupted"))?;
            bytes += chunk.len() as u64;
            if bytes > u64::from(metadata.byte_size) {
                return Err(ApiFailure::new(
                    413,
                    "evidence_budget",
                    "Evidence size exceeds reservation",
                ));
            }
            hash.update(&chunk);
            file.write_all(&chunk).await?;
        }
        if bytes != u64::from(metadata.byte_size)
            || format!("{:x}", hash.finalize()) != metadata.sha256
        {
            return Err(ApiFailure::invalid("Evidence bytes or digest do not match"));
        }
        file.flush().await?;
        Ok::<_, ApiFailure>(())
    };
    tokio::time::timeout(std::time::Duration::from_secs(60), transfer)
        .await
        .map_err(|_| ApiFailure::new(408, "upload_timeout", "Evidence upload timed out"))??;
    drop(file);
    let key = row.storage_key.clone();
    let backend = row.storage_backend.clone();
    // A prior publication may have survived a DB outage. Only identical bytes are reusable.
    if let Ok(existing) = setup.store.materialize(&key, &backend).await {
        let bytes = tokio::fs::read(&existing.0).await?;
        if hash(bytes) != metadata.sha256 {
            return Err(conflict("Published artifact differs"));
        }
    } else {
        setup.store.publish(&key, &scratch.0).await?;
    }
    let tx = ctx.db.begin().await?;
    let attempt = scheduler::lease(&tx, w, id, g, token, true).await?;
    if !["leased", "running", "cancel_requested"].contains(&attempt.attempt.state.as_str())
        && metadata.state != EvidenceState::Sealed
    {
        return Err(conflict("Attempt no longer accepts evidence"));
    }
    let row = execution_artifacts::Entity::find_by_id(artifact)
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let mut active = row.into_active_model();
    active.state = Set("sealed".into());
    let row = active.update(&tx).await?;
    let artifact = record(&row)?;
    tx.commit().await?;
    Ok(ArtifactReceipt { artifact })
}
/// Explicit maintenance reclaims abandoned uploads only after their attempt has ended.
pub async fn cleanup_pending(ctx: &AppContext) -> ApiResult<()> {
    let candidates = execution_artifacts::Entity::find()
        .filter(execution_artifacts::Column::State.eq("pending"))
        .filter(
            execution_artifacts::Column::CreatedAt
                .lt(chrono::Utc::now() - chrono::Duration::days(1)),
        )
        .limit(100)
        .all(&ctx.db)
        .await?;
    for row in candidates {
        let ended = execution_attempts::Entity::find_by_id(row.attempt_id)
            .filter(execution_attempts::Column::State.is_in(["finished", "recovery_required"]))
            .one(&ctx.db)
            .await?
            .is_some();
        if !ended {
            continue;
        }
        // Missing objects are normal when transfer never published; retain the missing-evidence record.
        let setup = Setup::get(ctx);
        setup
            .store
            .delete(&row.storage_key, &row.storage_backend)
            .await?;
        let mut active = row.into_active_model();
        active.state = Set("unavailable".into());
        active.reason = Set(Some("upload_abandoned".into()));
        active.update(&ctx.db).await?;
    }
    Ok(())
}
