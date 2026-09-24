//! Browser-controlled parts are immutable fingerprints; provider IDs and object keys stay private.
use crate::{
    config::Setup,
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        apps, artifact_jobs, artifact_multipart, artifact_parts, build_uploads, builds,
    },
    services::uploads,
};
use chrono::{Duration, Utc};
use loco_rs::app::AppContext;
use mobile_qa_contracts::{artifacts_api::*, browser::UploadResponse};
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait,
    IntoActiveModel, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, TransactionTrait,
};
use uuid::Uuid;

pub const PART_SIZE: u32 = 16 * 1024 * 1024; // 16 MiB
const MAX_PARALLEL_PARTS: u8 = 2;
pub fn policy() -> MultipartPolicy {
    MultipartPolicy {
        part_size: PART_SIZE,
        max_parallel_parts: MAX_PARALLEL_PARTS,
    }
}
fn state(value: &str) -> MultipartState {
    match value {
        "completing" => MultipartState::Completing,
        "sealed" => MultipartState::Sealed,
        "aborted" => MultipartState::Aborted,
        _ => MultipartState::Uploading,
    }
}
fn conflict(message: &str) -> ApiFailure {
    ApiFailure::new(409, "multipart_conflict", message)
}
async fn live_upload(
    ctx: &AppContext,
    app: &apps::Model,
    id: Uuid,
) -> ApiResult<build_uploads::Model> {
    let upload = uploads::upload(ctx, app, id).await?;
    if upload.expires_at <= Utc::now() {
        return Err(ApiFailure::new(
            410,
            "upload_expired",
            "Upload expired; choose the APK again",
        ));
    }
    Ok(upload)
}
async fn lock_upload(
    db: &impl sea_orm::ConnectionTrait,
    app: &apps::Model,
    id: Uuid,
    require_live: bool,
) -> ApiResult<build_uploads::Model> {
    let upload = build_uploads::Entity::find_by_id(id)
        .filter(build_uploads::Column::AppId.eq(app.id))
        .filter(build_uploads::Column::OrganizationId.eq(app.organization_id))
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if require_live && (upload.expires_at <= Utc::now() || upload.state == "expired") {
        return Err(ApiFailure::new(
            410,
            "upload_expired",
            "Upload expired; choose the APK again",
        ));
    }
    Ok(upload)
}
pub async fn detail(ctx: &AppContext, app: &apps::Model, id: Uuid) -> ApiResult<MultipartUpload> {
    let upload = uploads::upload(ctx, app, id).await?;
    let multipart = artifact_multipart::Entity::find_by_id(id)
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let parts = artifact_parts::Entity::find()
        .filter(artifact_parts::Column::UploadId.eq(id))
        .order_by_asc(artifact_parts::Column::PartNumber)
        .all(&ctx.db)
        .await?
        .into_iter()
        .map(|part| UploadedPart {
            part_number: part.part_number as u16,
            byte_size: part.byte_size as u32,
            sha256: part.sha256,
            etag: part.etag,
        })
        .collect();
    Ok(MultipartUpload {
        upload_id: id,
        part_size: PART_SIZE,
        max_parallel_parts: MAX_PARALLEL_PARTS,
        expires_at: upload.expires_at,
        state: state(&multipart.state),
        parts,
    })
}
pub async fn start(ctx: &AppContext, app: &apps::Model, id: Uuid) -> ApiResult<MultipartUpload> {
    let setup = Setup::get(ctx);
    if !setup.store.is_remote() {
        return Err(ApiFailure::new(
            503,
            "multipart_unavailable",
            "Multipart uploads require remote object storage",
        ));
    }
    let tx = ctx.db.begin().await?;
    let upload = build_uploads::Entity::find_by_id(id)
        .filter(build_uploads::Column::AppId.eq(app.id))
        .filter(build_uploads::Column::OrganizationId.eq(app.organization_id))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if upload.expires_at <= Utc::now() {
        return Err(ApiFailure::new(
            410,
            "upload_expired",
            "Upload expired; choose the APK again",
        ));
    }
    let existing = artifact_multipart::Entity::find_by_id(id).one(&tx).await?;
    if let Some(existing) = existing {
        if existing.state != "initiating" {
            tx.commit().await?;
            return detail(ctx, app, id).await;
        }
        if existing.lease_until.is_some_and(|until| until > Utc::now()) {
            return Err(conflict("Multipart initialization is still in progress"));
        }
    } else if upload.state != "pending" {
        return Err(conflict("This upload is already receiving or sealed"));
    }
    let operation = Uuid::new_v4();
    let key = format!("{}/{}/{}/{}", app.organization_id, app.id, id, operation);
    artifact_multipart::Entity::insert(artifact_multipart::ActiveModel {
        upload_id: Set(id),
        storage_key: Set(key.clone()),
        state: Set("initiating".into()),
        operation_id: Set(operation),
        lease_until: Set(Some(Utc::now() + Duration::minutes(2))),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::column(artifact_multipart::Column::UploadId)
            .update_columns([
                artifact_multipart::Column::StorageKey,
                artifact_multipart::Column::OperationId,
                artifact_multipart::Column::LeaseUntil,
            ])
            .to_owned(),
    )
    .exec(&tx)
    .await?;
    let expires_at = upload.expires_at;
    let mut upload = upload.into_active_model();
    upload.state = Set("receiving".into());
    upload.lease_until = Set(Some(expires_at));
    upload.update(&tx).await?;
    tx.commit().await?;
    // A crash after provider initiation leaves an orphan under this upload's private prefix.
    // The expiry sweeper inventories that prefix and aborts every provider attempt.
    let remote = setup.store.multipart_start(&key).await?;
    let changed = artifact_multipart::Entity::update_many()
        .col_expr(
            artifact_multipart::Column::ProviderUploadId,
            sea_orm::sea_query::Expr::value(Some(remote.upload_id.clone())),
        )
        .col_expr(
            artifact_multipart::Column::State,
            sea_orm::sea_query::Expr::value("uploading"),
        )
        .col_expr(
            artifact_multipart::Column::LeaseUntil,
            sea_orm::sea_query::Expr::value(Option::<chrono::DateTime<Utc>>::None),
        )
        .filter(artifact_multipart::Column::UploadId.eq(id))
        .filter(artifact_multipart::Column::OperationId.eq(operation))
        .filter(artifact_multipart::Column::State.eq("initiating"))
        .exec(&ctx.db)
        .await?;
    if changed.rows_affected != 1 {
        setup.store.multipart_abort(&key, &remote.upload_id).await?;
        return Err(conflict("Multipart initialization was replaced"));
    }
    detail(ctx, app, id).await
}
pub async fn authorize(
    ctx: &AppContext,
    app: &apps::Model,
    id: Uuid,
    input: AuthorizeUploadPartRequest,
) -> ApiResult<UploadPartAuthorization> {
    let upload = live_upload(ctx, app, id).await?;
    let expected = part_bytes(upload.expected_size, input.part_number)?;
    if expected != input.byte_size
        || input.sha256.len() != 64
        || !input
            .sha256
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(ApiFailure::invalid(
            "Part size or SHA-256 fingerprint is invalid",
        ));
    }
    let tx = ctx.db.begin().await?;
    lock_upload(&tx, app, id, true).await?;
    let multipart = artifact_multipart::Entity::find_by_id(id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if multipart.state != "uploading" {
        return Err(conflict("Upload is no longer accepting parts"));
    }
    let previous = artifact_parts::Entity::find_by_id((id, i32::from(input.part_number)))
        .one(&tx)
        .await?;
    if let Some(part) = previous {
        if part.sha256 != input.sha256 || part.byte_size != input.byte_size as i32 {
            return Err(ApiFailure::new(
                409,
                "upload_file_changed",
                "The selected file differs from the original upload",
            ));
        }
    }
    artifact_parts::Entity::insert(artifact_parts::ActiveModel {
        upload_id: Set(id),
        part_number: Set(i32::from(input.part_number)),
        byte_size: Set(input.byte_size as i32),
        sha256: Set(input.sha256.clone()),
        ..Default::default()
    })
    .on_conflict(
        OnConflict::columns([
            artifact_parts::Column::UploadId,
            artifact_parts::Column::PartNumber,
        ])
        .do_nothing()
        .to_owned(),
    )
    .exec(&tx)
    .await?;
    tx.commit().await?;
    let signed = Setup::get(ctx).store.multipart_part_url(
        &multipart.storage_key,
        multipart
            .provider_upload_id
            .as_deref()
            .ok_or_else(ApiFailure::internal)?,
        input.part_number,
        input.byte_size.into(),
        &input.sha256,
    )?;
    Ok(UploadPartAuthorization {
        part_number: input.part_number,
        url: signed.url,
        expires_at: signed.expires_at,
        headers: signed.headers,
    })
}
pub fn part_bytes(size: i64, number: u16) -> ApiResult<u32> {
    let offset = i64::from(number)
        .checked_sub(1)
        .filter(|_| number > 0)
        .ok_or_else(|| ApiFailure::invalid("Invalid part number"))?
        * i64::from(PART_SIZE);
    if offset >= size || number > 128 {
        return Err(ApiFailure::invalid("Part number exceeds the declared APK"));
    }
    Ok((size - offset).min(i64::from(PART_SIZE)) as u32)
}
pub async fn confirm(
    ctx: &AppContext,
    app: &apps::Model,
    id: Uuid,
    number: u16,
    input: ConfirmUploadPartRequest,
) -> ApiResult<MultipartUpload> {
    live_upload(ctx, app, id).await?;
    if input.etag.is_empty()
        || input.etag.len() > 256
        || input.etag.bytes().any(|b| b.is_ascii_control())
    {
        return Err(ApiFailure::invalid("Invalid part receipt"));
    }
    let tx = ctx.db.begin().await?;
    lock_upload(&tx, app, id, true).await?;
    let multipart = artifact_multipart::Entity::find_by_id(id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if multipart.state != "uploading" {
        return Err(conflict("Upload no longer accepts part receipts"));
    }
    let part = artifact_parts::Entity::find_by_id((id, i32::from(number)))
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let mut part = part.into_active_model();
    part.etag = Set(Some(input.etag));
    part.update(&tx).await?;
    tx.commit().await?;
    detail(ctx, app, id).await
}
pub async fn complete(ctx: &AppContext, app: &apps::Model, id: Uuid) -> ApiResult<UploadResponse> {
    let upload = live_upload(ctx, app, id).await?;
    let tx = ctx.db.begin().await?;
    let locked_upload = lock_upload(&tx, app, id, true).await?;
    let multipart = artifact_multipart::Entity::find_by_id(id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    match multipart.state.as_str() {
        "sealed" | "completing" => {}
        "uploading" => {
            let count = artifact_parts::Entity::find()
                .filter(artifact_parts::Column::UploadId.eq(id))
                .filter(artifact_parts::Column::Etag.is_not_null())
                .count(&tx)
                .await?;
            let expected = (upload.expected_size + i64::from(PART_SIZE) - 1) / i64::from(PART_SIZE);
            if count != expected as u64 {
                return Err(conflict("Confirm every uploaded part before completing"));
            }
            let mut multipart = multipart.into_active_model();
            multipart.state = Set("completing".into());
            multipart.update(&tx).await?;
            let expiry = locked_upload
                .expires_at
                .max(Utc::now() + Duration::minutes(21));
            let mut locked_upload = locked_upload.into_active_model();
            locked_upload.expires_at = Set(expiry);
            locked_upload.lease_until = Set(Some(expiry));
            locked_upload.update(&tx).await?;
            artifact_jobs::Entity::insert(artifact_jobs::ActiveModel {
                upload_id: Set(id),
                phase: Set("seal".into()),
                ..Default::default()
            })
            .on_conflict(
                OnConflict::columns([
                    artifact_jobs::Column::UploadId,
                    artifact_jobs::Column::Phase,
                ])
                .do_nothing()
                .to_owned(),
            )
            .exec(&tx)
            .await?;
        }
        _ => return Err(conflict("Multipart session cannot be completed")),
    }
    tx.commit().await?;
    uploads::upload_response(ctx, &uploads::upload(ctx, app, id).await?).await
}
pub async fn abort(ctx: &AppContext, app: &apps::Model, id: Uuid) -> ApiResult<MultipartUpload> {
    uploads::upload(ctx, app, id).await?;
    let tx = ctx.db.begin().await?;
    let upload = lock_upload(&tx, app, id, false).await?;
    let multipart = artifact_multipart::Entity::find_by_id(id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if matches!(multipart.state.as_str(), "completing" | "sealed") {
        return Err(conflict("A sealed or completing upload cannot be aborted"));
    }
    let provider = multipart.provider_upload_id.clone();
    let storage_key = multipart.storage_key.clone();
    let mut multipart = multipart.into_active_model();
    multipart.state = Set("aborted".into());
    multipart.update(&tx).await?;
    let mut upload = upload.into_active_model();
    upload.state = Set("expired".into());
    upload.lease_until = Set(None);
    upload.update(&tx).await?;
    tx.commit().await?;
    if let Some(provider) = provider {
        Setup::get(ctx)
            .store
            .multipart_abort(&storage_key, &provider)
            .await?;
    }
    detail(ctx, app, id).await
}
/// Provider lifecycle rules are unavailable on Railway; expiry cleanup is application-owned.
pub async fn sweep_expired(ctx: &AppContext) -> ApiResult<usize> {
    let setup = Setup::get(ctx);
    if !setup.store.is_remote() {
        return Ok(0);
    }
    let cutoff = Utc::now() - Duration::hours(1);
    let expired = build_uploads::Entity::find()
        .filter(build_uploads::Column::ExpiresAt.lt(cutoff))
        .all(&ctx.db)
        .await?;
    let mut removed = 0;
    for upload in expired {
        let Some(candidate) = artifact_multipart::Entity::find_by_id(upload.id)
            .one(&ctx.db)
            .await?
        else {
            continue;
        };
        if candidate.state == "completing" {
            continue;
        }
        let id = upload.id;
        let prefix = format!("{}/{}/{}", upload.organization_id, upload.app_id, id);
        // Fence expired intake before deleting. Capture retained builds while holding the same
        // upload lock as completion, then release SQL locks before provider I/O.
        let tx = ctx.db.begin().await?;
        let Some(locked) = build_uploads::Entity::find_by_id(id)
            .filter(build_uploads::Column::ExpiresAt.lt(cutoff))
            .lock_exclusive()
            .one(&tx)
            .await?
        else {
            continue;
        };
        let current = artifact_multipart::Entity::find_by_id(id)
            .lock_exclusive()
            .one(&tx)
            .await?
            .ok_or_else(ApiFailure::missing)?;
        if current.state == "completing" {
            continue;
        }
        let retained = builds::Entity::find()
            .filter(builds::Column::UploadId.eq(id))
            .all(&tx)
            .await?
            .into_iter()
            .map(|build| build.storage_key)
            .collect::<Vec<_>>();
        if retained.is_empty() {
            let mut locked = locked.into_active_model();
            locked.state = Set("expired".into());
            locked.lease_until = Set(None);
            locked.update(&tx).await?;
        }
        if !matches!(current.state.as_str(), "completing" | "sealed") {
            let mut current = current.into_active_model();
            current.state = Set("aborted".into());
            current.update(&tx).await?;
        }
        tx.commit().await?;
        for upload in setup.store.multipart_uploads(&prefix).await? {
            setup
                .store
                .multipart_abort(&upload.key, &upload.upload_id)
                .await?;
            removed += 1;
        }
        for (key, modified) in setup.store.list_upload(&prefix, "pilot").await? {
            if retained.contains(&key)
                || modified.is_none_or(|time| time >= Utc::now() - Duration::hours(1))
            {
                continue;
            }
            setup.store.delete(&key, "pilot").await?;
            removed += 1;
        }
    }
    Ok(removed)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parts_include_exact_two_gib() {
        assert_eq!(part_bytes(2147483648, 128).unwrap(), PART_SIZE);
        assert!(part_bytes(2147483648, 129).is_err());
        assert!(part_bytes(1, 0).is_err());
        assert_eq!(part_bytes(i64::from(PART_SIZE) + 7, 2).unwrap(), 7);
    }
}
