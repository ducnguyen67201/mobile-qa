//! Upload/session transitions fenced by attempt IDs. File publication and DB commits are separate.
use crate::{
    config::{Setup, MAX_APK, UPLOAD_SECONDS},
    errors::{ApiFailure, ApiResult},
    models::_entities::{apps, artifact_jobs, build_uploads, builds},
    services::{apk_validation, apps as app_service},
};
use axum::extract::Multipart;
use chrono::{Duration, Utc};
use loco_rs::app::AppContext;
use mobile_qa_contracts::browser::*;
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

pub async fn upload(
    ctx: &AppContext,
    app: &apps::Model,
    id: Uuid,
) -> ApiResult<build_uploads::Model> {
    build_uploads::Entity::find_by_id(id)
        .filter(build_uploads::Column::AppId.eq(app.id))
        .filter(build_uploads::Column::OrganizationId.eq(app.organization_id))
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)
}
pub async fn upload_response(
    ctx: &AppContext,
    row: &build_uploads::Model,
) -> ApiResult<UploadResponse> {
    let now = Utc::now();
    let state = if row.state != "finalized" && row.expires_at <= now {
        UploadState::Expired
    } else if row.state == "receiving" && row.lease_until.is_some_and(|v| v <= now) {
        UploadState::Pending
    } else {
        match row.state.as_str() {
            "pending" => UploadState::Pending,
            "receiving" => UploadState::Receiving,
            "uploaded" => UploadState::Uploaded,
            "finalized" => UploadState::Finalized,
            _ => UploadState::Expired,
        }
    };
    let build_id = builds::Entity::find()
        .filter(builds::Column::UploadId.eq(row.id))
        .one(&ctx.db)
        .await?
        .map(|b| b.id);
    Ok(UploadResponse {
        id: row.id,
        app_id: row.app_id,
        original_filename: row.original_filename.clone(),
        expected_size: row.expected_size,
        actual_size: row.actual_size,
        state,
        expires_at: row.expires_at,
        build_id,
        retry_after_seconds: if state == UploadState::Receiving {
            Some(2)
        } else {
            None
        },
    })
}
pub async fn create(
    ctx: &AppContext,
    app: &apps::Model,
    user: Uuid,
    input: CreateBuildUploadRequest,
) -> ApiResult<UploadResponse> {
    let name = app_service::text(&input.original_filename, 255, "Filename")?;
    if !name.to_lowercase().ends_with(".apk") {
        return Err(ApiFailure::invalid("Choose an Android APK file"));
    }
    if input.expected_size <= 0 || input.expected_size > MAX_APK {
        return Err(ApiFailure::new(
            413,
            "file_too_large",
            "Choose a nonempty APK up to 2 GiB",
        ));
    }
    let tx = ctx.db.begin().await?;
    // Lock parent to serialize quota admission for this app.
    apps::Entity::find_by_id(app.id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let active = build_uploads::Entity::find()
        .filter(build_uploads::Column::AppId.eq(app.id))
        .filter(build_uploads::Column::State.is_in(["pending", "receiving", "uploaded"]))
        .filter(build_uploads::Column::ExpiresAt.gt(Utc::now()))
        .all(&tx)
        .await?;
    if active.len() >= 4 {
        return Err(ApiFailure::new(
            429,
            "upload_limit",
            "Finish an existing upload or wait for it to expire",
        ));
    }
    let setup = Setup::get(ctx);
    let now = Utc::now();
    let row = build_uploads::ActiveModel {
        id: Set(Uuid::new_v4()),
        organization_id: Set(app.organization_id),
        app_id: Set(app.id),
        created_by: Set(user),
        original_filename: Set(name),
        expected_size: Set(input.expected_size),
        state: Set("pending".into()),
        expires_at: Set(now + Duration::seconds(UPLOAD_SECONDS)),
        attempt_id: Set(None),
        lease_until: Set(None),
        storage_backend: Set(setup.storage_backend),
        sealed_storage_key: Set(None),
        actual_size: Set(None),
        sha256: Set(None),
        created_at: Set(now),
    }
    .insert(&tx)
    .await?;
    tx.commit().await?;
    upload_response(ctx, &row).await
}
pub async fn transfer(
    ctx: &AppContext,
    app: &apps::Model,
    session: &crate::services::auth::Session,
    id: Uuid,
    mut multipart: Multipart,
) -> ApiResult<UploadResponse> {
    let setup = Setup::get(ctx);
    let _permit = setup.transfers.clone().try_acquire_owned().map_err(|_| {
        ApiFailure::new(
            429,
            "upload_busy",
            "Upload capacity is busy. Try again shortly",
        )
    })?;
    let attempt = Uuid::new_v4();
    let tx = ctx.db.begin().await?;
    let row = build_uploads::Entity::find_by_id(id)
        .filter(build_uploads::Column::AppId.eq(app.id))
        .filter(build_uploads::Column::OrganizationId.eq(app.organization_id))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if row.expires_at <= Utc::now() {
        return Err(ApiFailure::new(
            410,
            "upload_expired",
            "Upload expired. Choose the APK again",
        ));
    }
    if row.state != "pending"
        && !(row.state == "receiving" && row.lease_until.is_some_and(|t| t <= Utc::now()))
    {
        return Err(ApiFailure::new(
            409,
            "upload_state_conflict",
            "Upload is already receiving or sealed. Refresh its status",
        ));
    }
    let expected = row.expected_size;
    let mut active: build_uploads::ActiveModel = row.into();
    active.state = Set("receiving".into());
    active.attempt_id = Set(Some(attempt));
    active.lease_until = Set(Some(Utc::now() + Duration::minutes(31)));
    active.update(&tx).await?;
    tx.commit().await?;
    let key = format!("{}/{}/{}/{}", app.organization_id, app.id, id, attempt);
    let work = async {
        let (scratch, mut file) = setup.store.scratch_with_budget(expected).await?;
        let mut field = multipart
            .next_field()
            .await
            .map_err(|_| ApiFailure::new(400, "invalid_multipart", "Select exactly one APK file"))?
            .ok_or_else(|| ApiFailure::invalid("APK file is required"))?;
        if field.name() != Some("file") {
            return Err(ApiFailure::invalid("Expected the APK file field"));
        }
        let mut count = 0i64;
        let mut hash = Sha256::new();
        loop {
            let chunk = tokio::time::timeout(std::time::Duration::from_secs(30), field.chunk())
                .await
                .map_err(|_| ApiFailure::new(408, "upload_timeout", "Upload stalled. Try again"))?
                .map_err(|_| {
                    ApiFailure::new(
                        400,
                        "upload_interrupted",
                        "Upload was interrupted. Choose the file again",
                    )
                })?;
            let Some(chunk) = chunk else { break };
            count += chunk.len() as i64;
            if count > MAX_APK || count > expected {
                return Err(ApiFailure::new(
                    413,
                    "file_too_large",
                    "APK byte count exceeds the declared size or limit",
                ));
            }
            hash.update(&chunk);
            file.write_all(&chunk).await?;
        }
        drop(field);
        if multipart
            .next_field()
            .await
            .map_err(|_| ApiFailure::invalid("Malformed multipart upload"))?
            .is_some()
        {
            return Err(ApiFailure::invalid("Upload exactly one file"));
        }
        if count != expected {
            return Err(ApiFailure::new(
                409,
                "size_mismatch",
                "APK byte count does not match. Choose the complete file again",
            ));
        }
        file.sync_all().await?;
        drop(file);
        setup.store.publish(&key, &scratch.0).await?;
        crate::services::auth::ensure_live(ctx, session).await?;
        app_service::authorized(ctx, session.user.id, app.id).await?;
        let result = build_uploads::Entity::update_many()
            .col_expr(
                build_uploads::Column::State,
                sea_orm::sea_query::Expr::value("uploaded"),
            )
            .col_expr(
                build_uploads::Column::SealedStorageKey,
                sea_orm::sea_query::Expr::value(key.clone()),
            )
            .col_expr(
                build_uploads::Column::ActualSize,
                sea_orm::sea_query::Expr::value(count),
            )
            .col_expr(
                build_uploads::Column::Sha256,
                sea_orm::sea_query::Expr::value(format!("{:x}", hash.finalize())),
            )
            .col_expr(
                build_uploads::Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(None::<chrono::DateTime<Utc>>),
            )
            .filter(build_uploads::Column::Id.eq(id))
            .filter(build_uploads::Column::AttemptId.eq(attempt))
            .filter(build_uploads::Column::State.eq("receiving"))
            .filter(build_uploads::Column::ExpiresAt.gt(Utc::now()))
            .exec(&ctx.db)
            .await?;
        if result.rows_affected != 1 {
            return Err(ApiFailure::new(
                409,
                "upload_attempt_expired",
                "Upload attempt expired. Reload its status",
            ));
        }
        Ok::<(), ApiFailure>(())
    };
    let result = tokio::time::timeout(std::time::Duration::from_secs(1800), work)
        .await
        .unwrap_or_else(|_| {
            Err(ApiFailure::new(
                408,
                "upload_timeout",
                "Upload took too long. Try again",
            ))
        });
    if let Err(error) = result {
        // A cancelled request may leave a receiving lease; a later attempt can reclaim it.
        build_uploads::Entity::update_many()
            .col_expr(
                build_uploads::Column::State,
                sea_orm::sea_query::Expr::value("pending"),
            )
            .col_expr(
                build_uploads::Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(None::<chrono::DateTime<Utc>>),
            )
            .filter(build_uploads::Column::Id.eq(id))
            .filter(build_uploads::Column::AttemptId.eq(attempt))
            .filter(build_uploads::Column::State.eq("receiving"))
            .exec(&ctx.db)
            .await?;
        return Err(error);
    }
    tracing::info!(upload_id=%id,app_id=%app.id,phase="uploaded","APK bytes sealed");
    upload_response(ctx, &upload(ctx, app, id).await?).await
}
pub async fn build(ctx: &AppContext, app: &apps::Model, id: Uuid) -> ApiResult<builds::Model> {
    builds::Entity::find_by_id(id)
        .filter(builds::Column::AppId.eq(app.id))
        .filter(builds::Column::OrganizationId.eq(app.organization_id))
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)
}
fn state(value: &str) -> ValidationState {
    match value {
        "validated" => ValidationState::Validated,
        "invalid" => ValidationState::Invalid,
        "unsupported" => ValidationState::Unsupported,
        "error" => ValidationState::Error,
        _ => ValidationState::Validating,
    }
}
pub async fn build_response(
    ctx: &AppContext,
    app: &apps::Model,
    row: &builds::Model,
) -> ApiResult<BuildResponse> {
    let env = app_service::environment(ctx, app).await?;
    Ok(BuildResponse {
        upload_id: row.upload_id,
        id: row.id,
        app_id: row.app_id,
        original_filename: row.original_filename.clone(),
        byte_size: row.byte_size,
        sha256: row.sha256.clone(),
        created_at: row.created_at,
        validation: BuildValidation {
            state: state(&row.validation_state),
            reason_code: row.reason_code.clone(),
            message: row.message.clone(),
            started_at: row.started_at,
            completed_at: row.validated_at,
            validator_version: row.validator_version.clone(),
            intake_policy_version: row.intake_policy_version.clone(),
        },
        metadata: row
            .metadata
            .clone()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| ApiFailure::internal())?,
        readiness: app_service::readiness(&env),
        can_retry_validation: row.validation_state == "error"
            || (row.validation_state == "validating"
                && row.lease_until.is_some_and(|t| t <= Utc::now())),
    })
}
pub async fn complete(
    ctx: &AppContext,
    app: &apps::Model,
    id: Uuid,
) -> ApiResult<(u16, BuildResponse)> {
    let setup = Setup::get(ctx);
    let tx = ctx.db.begin().await?;
    let now = Utc::now();
    let upload = build_uploads::Entity::find_by_id(id)
        .filter(build_uploads::Column::AppId.eq(app.id))
        .filter(build_uploads::Column::OrganizationId.eq(app.organization_id))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    // Local small fixtures retain synchronous behavior; hosted and large work uses the durable queue.
    let inline = !setup.store.is_remote()
        && upload.expected_size <= i64::from(crate::services::multipart_uploads::PART_SIZE);
    let existing = builds::Entity::find()
        .filter(builds::Column::UploadId.eq(id))
        .one(&tx)
        .await?;
    if let Some(row) = &existing {
        if matches!(
            row.validation_state.as_str(),
            "validated" | "invalid" | "unsupported"
        ) {
            let row = row.clone();
            tx.commit().await?;
            return Ok((200, build_response(ctx, app, &row).await?));
        }
        if row.validation_state == "validating" && row.lease_until.is_some_and(|t| t > now) {
            let row = row.clone();
            tx.commit().await?;
            return Ok((202, build_response(ctx, app, &row).await?));
        }
    } else {
        if upload.expires_at <= now {
            return Err(ApiFailure::new(
                410,
                "upload_expired",
                "Upload expired. Choose the file again",
            ));
        }
        if upload.state != "uploaded" {
            return Err(ApiFailure::new(
                409,
                "upload_incomplete",
                "Finish uploading the APK before validating",
            ));
        }
    }
    let _permit = setup.validators.clone().try_acquire_owned().map_err(|_| {
        ApiFailure::new(
            429,
            "validator_busy",
            "Another APK is being validated. Retry shortly",
        )
    })?;
    let attempt = Uuid::new_v4();
    let lease = now + Duration::seconds(90);
    let row = if let Some(row) = existing {
        let mut a: builds::ActiveModel = row.into();
        a.validation_state = Set("validating".into());
        a.reason_code = Set(None);
        a.message = Set(None);
        a.attempt_id = Set(Some(attempt));
        a.lease_until = Set(Some(lease));
        a.started_at = Set(now);
        a.validated_at = Set(None);
        a.update(&tx).await?
    } else {
        builds::ActiveModel {
            id: Set(Uuid::new_v4()),
            organization_id: Set(app.organization_id),
            app_id: Set(app.id),
            upload_id: Set(id),
            storage_backend: Set(upload.storage_backend.clone()),
            storage_key: Set(upload
                .sealed_storage_key
                .clone()
                .ok_or_else(ApiFailure::internal)?),
            sha256: Set(upload.sha256.clone().ok_or_else(ApiFailure::internal)?),
            byte_size: Set(upload.actual_size.ok_or_else(ApiFailure::internal)?),
            original_filename: Set(upload.original_filename.clone()),
            validation_state: Set("validating".into()),
            reason_code: Set(None),
            message: Set(None),
            metadata: Set(None),
            validator_version: Set(apk_validation::TOOL_VERSION.into()),
            intake_policy_version: Set(apk_validation::POLICY.into()),
            attempt_id: Set(Some(attempt)),
            lease_until: Set(Some(lease)),
            created_at: Set(now),
            started_at: Set(now),
            validated_at: Set(None),
        }
        .insert(&tx)
        .await?
    };
    artifact_jobs::Entity::insert(artifact_jobs::ActiveModel {
        upload_id: Set(id),
        phase: Set("validate".into()),
        state: Set("processing".into()),
        attempt_id: Set(Some(attempt)),
        lease_until: Set(Some(lease)),
        attempts: Set(1),
        reason_code: Set(None),
        updated_at: Set(now),
    })
    .on_conflict(
        OnConflict::columns([
            artifact_jobs::Column::UploadId,
            artifact_jobs::Column::Phase,
        ])
        .update_columns([
            artifact_jobs::Column::State,
            artifact_jobs::Column::AttemptId,
            artifact_jobs::Column::LeaseUntil,
            artifact_jobs::Column::Attempts,
            artifact_jobs::Column::ReasonCode,
            artifact_jobs::Column::UpdatedAt,
        ])
        .to_owned(),
    )
    .exec(&tx)
    .await?;
    let mut u: build_uploads::ActiveModel = upload.into();
    u.state = Set("finalized".into());
    u.update(&tx).await?;
    if !inline {
        artifact_jobs::Entity::update_many()
            .col_expr(
                artifact_jobs::Column::State,
                sea_orm::sea_query::Expr::value("pending"),
            )
            .col_expr(
                artifact_jobs::Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(None::<chrono::DateTime<Utc>>),
            )
            .filter(artifact_jobs::Column::UploadId.eq(id))
            .filter(artifact_jobs::Column::Phase.eq("validate"))
            .filter(artifact_jobs::Column::AttemptId.eq(attempt))
            .exec(&tx)
            .await?;
    }
    tx.commit().await?;
    if !inline {
        return Ok((202, build_response(ctx, app, &row).await?));
    }
    crate::services::upload_validation::validate_claimed(ctx, app, &row, attempt).await?;
    let row = build(ctx, app, row.id).await?;
    Ok((
        if row.validation_state == "validating" {
            202
        } else {
            200
        },
        build_response(ctx, app, &row).await?,
    ))
}
pub async fn list(
    ctx: &AppContext,
    app: &apps::Model,
    query: app_service::ListQuery,
) -> ApiResult<BuildListResponse> {
    let limit = app_service::page_limit(&query)?;
    let mut select = builds::Entity::find()
        .filter(builds::Column::AppId.eq(app.id))
        .filter(builds::Column::OrganizationId.eq(app.organization_id));
    if let Some(cursor) = query.cursor {
        let row = build(
            ctx,
            app,
            Uuid::parse_str(&cursor).map_err(|_| ApiFailure::invalid("Invalid cursor"))?,
        )
        .await?;
        select = select.filter(
            Condition::any()
                .add(builds::Column::CreatedAt.lt(row.created_at))
                .add(
                    Condition::all()
                        .add(builds::Column::CreatedAt.eq(row.created_at))
                        .add(builds::Column::Id.lt(row.id)),
                ),
        );
    }
    let mut rows = select
        .order_by_desc(builds::Column::CreatedAt)
        .order_by_desc(builds::Column::Id)
        .limit(limit + 1)
        .all(&ctx.db)
        .await?;
    let more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let next_cursor = if more {
        rows.last().map(|r| r.id.to_string())
    } else {
        None
    };
    let mut items = Vec::new();
    for row in rows {
        items.push(build_response(ctx, app, &row).await?);
    }
    Ok(BuildListResponse { items, next_cursor })
}
