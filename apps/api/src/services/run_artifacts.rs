//! Evidence publication is pending until bounded bytes and their digest have been verified.
use super::{execution_store::*, scheduler, worker_auth::Worker};
use crate::{
    config::Setup,
    errors::{ApiFailure, ApiResult},
    storage::Scratch,
};
use axum::body::Body;
use futures_util::StreamExt;
use loco_rs::app::AppContext;
use mobile_qa_contracts::execution::*;
use sea_orm::{QueryResult, TransactionTrait};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

pub fn record(r: &QueryResult) -> ApiResult<RunArtifact> {
    Ok(RunArtifact {
        id: field(r, "id")?,
        attempt_id: field(r, "attempt_id")?,
        checkpoint_id: field(r, "checkpoint_id")?,
        name: field(r, "name")?,
        mime: field(r, "mime")?,
        byte_size: field::<i64>(r, "byte_size")? as u32,
        sha256: field(r, "sha256")?,
        state: decode(serde_json::Value::String(field(r, "state")?))?,
        reason: field(r, "reason")?,
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
    let r = scheduler::lease(&tx, w, id, input.generation, token, true).await?;
    let old = rows(
        &tx,
        "SELECT * FROM execution_artifacts WHERE attempt_id=$1 AND name=$2",
        vec![id.into(), input.name.clone().into()],
    )
    .await?;
    if let Some(r) = old.first() {
        let artifact = record(r)?;
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
    if !["leased", "running", "cancel_requested"].contains(&field::<String>(&r, "state")?.as_str())
    {
        return Err(conflict("Attempt no longer accepts artifacts"));
    }
    let manifest: RunManifest = decode(field(&r, "manifest")?)?;
    let c = &manifest.cases[field::<i32>(&r, "case_index")? as usize].case;
    if !c
        .actions
        .iter()
        .any(|a| a.checkpoint_id == input.checkpoint_id)
    {
        return Err(ApiFailure::invalid("Unknown checkpoint"));
    }
    let used: i64 = field(
        &one(
            &tx,
            "SELECT COALESCE(sum(byte_size),0)::bigint AS used FROM execution_artifacts WHERE \
            attempt_id=$1",
            vec![id.into()],
        )
        .await?,
        "used",
    )?;
    if used + i64::from(input.byte_size) > i64::from(c.budget.artifact_bytes) {
        return Err(ApiFailure::new(
            413,
            "evidence_budget",
            "Evidence byte budget exceeded",
        ));
    }
    let artifact = Uuid::new_v4();
    let run_id: Uuid = field(&r, "run_id")?;
    let key = format!("{}/{}/{}/{}", w.app_id, run_id, id, artifact);
    let setup = Setup::get(ctx);
    exec(
        &tx,
        "INSERT INTO execution_artifacts(id,attempt_id,checkpoint_id,name,mime,byte_size,\
            sha256,storage_key,storage_backend) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        vec![
            artifact.into(),
            id.into(),
            input.checkpoint_id.into(),
            input.name.into(),
            input.mime.into(),
            i64::from(input.byte_size).into(),
            input.sha256.into(),
            key.into(),
            setup.storage_backend.into(),
        ],
    )
    .await?;
    let result = record(
        &one(
            &tx,
            "SELECT * FROM execution_artifacts WHERE id=$1",
            vec![artifact.into()],
        )
        .await?,
    )?;
    tx.commit().await?;
    Ok(ArtifactReceipt { artifact: result })
}

pub async fn content(ctx: &AppContext, id: Uuid) -> ApiResult<(RunArtifact, Scratch)> {
    let r = one(
        &ctx.db,
        "SELECT * FROM execution_artifacts WHERE id=$1 AND state='sealed'",
        vec![id.into()],
    )
    .await?;
    let artifact = record(&r)?;
    let setup = Setup::get(ctx);
    let file = setup
        .store
        .materialize(
            &field::<String>(&r, "storage_key")?,
            &field::<String>(&r, "storage_backend")?,
        )
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
    let r = one(
        &ctx.db,
        "SELECT * FROM execution_artifacts WHERE id=$1 AND attempt_id=$2",
        vec![artifact.into(), id.into()],
    )
    .await?;
    let metadata = record(&r)?;
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
    let key: String = field(&r, "storage_key")?;
    let backend: String = field(&r, "storage_backend")?;
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
    if !["leased", "running", "cancel_requested"]
        .contains(&field::<String>(&attempt, "state")?.as_str())
        && metadata.state != EvidenceState::Sealed
    {
        return Err(conflict("Attempt no longer accepts evidence"));
    }
    exec(
        &tx,
        "UPDATE execution_artifacts SET state='sealed' WHERE id=$1",
        vec![artifact.into()],
    )
    .await?;
    let artifact = record(
        &one(
            &tx,
            "SELECT * FROM execution_artifacts WHERE id=$1",
            vec![artifact.into()],
        )
        .await?,
    )?;
    tx.commit().await?;
    Ok(ArtifactReceipt { artifact })
}
/// Explicit maintenance reclaims abandoned uploads only after their attempt has ended.
pub async fn cleanup_pending(ctx: &AppContext) -> ApiResult<()> {
    let abandoned = rows(
        &ctx.db,
        "SELECT f.id,f.storage_key,f.storage_backend FROM execution_artifacts f JOIN \
            execution_attempts a ON a.id=f.attempt_id WHERE f.state='pending' AND \
            f.created_at<now()-interval '1 day' AND a.state IN ('finished','recovery_required') \
            LIMIT 100",
        vec![],
    )
    .await?;
    for r in abandoned {
        let id: Uuid = field(&r, "id")?;
        // Missing objects are normal when transfer never published; retain the missing-evidence record.
        let setup = Setup::get(ctx);
        let key: String = field(&r, "storage_key")?;
        let backend: String = field(&r, "storage_backend")?;
        setup.store.delete(&key, &backend).await?;
        exec(
            &ctx.db,
            "UPDATE execution_artifacts SET state='unavailable',reason='upload_abandoned' WHERE \
            id=$1 AND state='pending'",
            vec![id.into()],
        )
        .await?;
    }
    Ok(())
}
