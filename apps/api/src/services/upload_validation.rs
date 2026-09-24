//! PostgreSQL is the work queue. Each bounded attempt has a renewable lease and fenced publication.
use crate::{
    config::Setup,
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        apps, artifact_jobs, artifact_multipart, artifact_parts, build_uploads, builds,
    },
    services::{
        apk_validation,
        execution_store::{json, word},
        multipart_uploads::PART_SIZE,
    },
    storage::multipart::MultipartPart,
};
use chrono::{Duration, Utc};
use loco_rs::{app::AppContext, environment::Environment};
use sea_orm::{
    sea_query::{LockBehavior, LockType},
    ActiveModelTrait,
    ActiveValue::Set,
    ColumnTrait, Condition, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;
use uuid::Uuid;
const DEADLINE_SECONDS: u64 = 1200;
const LEASE_SECONDS: i64 = 90;

pub fn start(ctx: &AppContext) {
    if matches!(ctx.environment, Environment::Test) {
        return;
    }
    let ctx = ctx.clone();
    tokio::spawn(async move {
        let mut sweep = tokio::time::interval(std::time::Duration::from_secs(300));
        loop {
            match run_once(&ctx).await {
                Ok(true) => continue,
                Ok(false) => {}
                Err(error) => tracing::warn!(
                    reason_code = error.code,
                    "Artifact job reconciliation failed"
                ),
            }
            tokio::select! {_=sweep.tick()=>{if let Err(error)=crate::services::multipart_uploads::sweep_expired(&ctx).await {tracing::warn!(reason_code=error.code,"Multipart expiry reconciliation failed");}},_=tokio::time::sleep(std::time::Duration::from_secs(5))=>{}}
        }
    });
}
/// One transaction claims only durable work; network/tool I/O happens after commit.
pub async fn run_once(ctx: &AppContext) -> ApiResult<bool> {
    // Keep the large SQL/transfer state machine off callers' stacks. In particular,
    // both seal and validation variants exist in the future even when only one runs.
    Box::pin(run_claimed_job(ctx)).await
}

async fn run_claimed_job(ctx: &AppContext) -> ApiResult<bool> {
    let setup = Setup::get(ctx);
    let Ok(_permit) = setup.validators.clone().try_acquire_owned() else {
        return Ok(false);
    };
    // A process crash on the final attempt is terminal; do not strand its live-looking state.
    let now = Utc::now();
    artifact_jobs::Entity::update_many()
        .col_expr(
            artifact_jobs::Column::State,
            sea_orm::sea_query::Expr::value("error"),
        )
        .col_expr(
            artifact_jobs::Column::ReasonCode,
            sea_orm::sea_query::Expr::value(Some("artifact_retries_exhausted")),
        )
        .col_expr(
            artifact_jobs::Column::LeaseUntil,
            sea_orm::sea_query::Expr::value(Option::<chrono::DateTime<Utc>>::None),
        )
        .filter(artifact_jobs::Column::State.eq("processing"))
        .filter(artifact_jobs::Column::LeaseUntil.lt(now))
        .filter(artifact_jobs::Column::Attempts.gte(5))
        .exec(&ctx.db)
        .await?;
    let failed_validate = artifact_jobs::Entity::find()
        .filter(artifact_jobs::Column::Phase.eq("validate"))
        .filter(artifact_jobs::Column::State.eq("error"))
        .all(&ctx.db)
        .await?
        .into_iter()
        .map(|job| job.upload_id)
        .collect::<Vec<_>>();
    if !failed_validate.is_empty() {
        builds::Entity::update_many()
            .col_expr(
                builds::Column::ValidationState,
                sea_orm::sea_query::Expr::value("error"),
            )
            .col_expr(
                builds::Column::ReasonCode,
                sea_orm::sea_query::Expr::value(Some("artifact_retries_exhausted")),
            )
            .col_expr(
                builds::Column::Message,
                sea_orm::sea_query::Expr::value(Some(
                    "Artifact processing exhausted its retries; retry validation",
                )),
            )
            .col_expr(
                builds::Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(Option::<chrono::DateTime<Utc>>::None),
            )
            .filter(builds::Column::ValidationState.eq("validating"))
            .filter(builds::Column::UploadId.is_in(failed_validate))
            .exec(&ctx.db)
            .await?;
    }
    let failed_seal = artifact_jobs::Entity::find()
        .filter(artifact_jobs::Column::Phase.eq("seal"))
        .filter(artifact_jobs::Column::State.eq("error"))
        .all(&ctx.db)
        .await?
        .into_iter()
        .map(|job| job.upload_id)
        .collect::<Vec<_>>();
    if !failed_seal.is_empty() {
        artifact_multipart::Entity::update_many()
            .col_expr(
                artifact_multipart::Column::State,
                sea_orm::sea_query::Expr::value("aborted"),
            )
            .filter(artifact_multipart::Column::State.eq("completing"))
            .filter(artifact_multipart::Column::UploadId.is_in(failed_seal.clone()))
            .exec(&ctx.db)
            .await?;
        build_uploads::Entity::update_many()
            .col_expr(
                build_uploads::Column::State,
                sea_orm::sea_query::Expr::value("expired"),
            )
            .col_expr(
                build_uploads::Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(Option::<chrono::DateTime<Utc>>::None),
            )
            .filter(build_uploads::Column::State.eq("receiving"))
            .filter(build_uploads::Column::Id.is_in(failed_seal))
            .exec(&ctx.db)
            .await?;
    }
    let tx = ctx.db.begin().await?;
    let claimable = Condition::any()
        .add(
            Condition::all()
                .add(artifact_jobs::Column::State.eq("pending"))
                .add(
                    Condition::any()
                        .add(artifact_jobs::Column::LeaseUntil.is_null())
                        .add(artifact_jobs::Column::LeaseUntil.lt(now)),
                ),
        )
        .add(
            Condition::all()
                .add(artifact_jobs::Column::State.eq("processing"))
                .add(artifact_jobs::Column::LeaseUntil.lt(now)),
        );
    let Some(candidate) = artifact_jobs::Entity::find()
        .filter(claimable.clone())
        .filter(artifact_jobs::Column::Attempts.lt(5))
        .order_by_asc(artifact_jobs::Column::UpdatedAt)
        .one(&tx)
        .await?
    else {
        return Ok(false);
    };
    let upload_id = candidate.upload_id;
    let phase = candidate.phase;
    // Match intake's upload → build → job lock order; never hold a job while waiting on intake.
    let upload = build_uploads::Entity::find_by_id(upload_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if phase == "validate" {
        builds::Entity::find()
            .filter(builds::Column::UploadId.eq(upload_id))
            .lock_exclusive()
            .all(&tx)
            .await?;
    }
    let Some(job) = artifact_jobs::Entity::find_by_id((upload_id, phase.clone()))
        .filter(claimable)
        .filter(artifact_jobs::Column::Attempts.lt(5))
        .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked)
        .one(&tx)
        .await?
    else {
        return Ok(false);
    };
    let attempt = Uuid::new_v4();
    let lease_until = Utc::now() + Duration::seconds(LEASE_SECONDS);
    let mut job = job.into_active_model();
    job.state = Set("processing".into());
    job.attempt_id = Set(Some(attempt));
    job.lease_until = Set(Some(lease_until));
    job.attempts = Set(job.attempts.take().unwrap_or_default() + 1);
    job.updated_at = Set(Utc::now());
    job.update(&tx).await?;
    let app = apps::Entity::find_by_id(upload.app_id)
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if phase == "validate" {
        builds::Entity::update_many()
            .col_expr(
                builds::Column::AttemptId,
                sea_orm::sea_query::Expr::value(Some(attempt)),
            )
            .col_expr(
                builds::Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(Some(lease_until)),
            )
            .filter(builds::Column::UploadId.eq(upload_id))
            .filter(builds::Column::ValidationState.eq("validating"))
            .exec(&tx)
            .await?;
    }
    tx.commit().await?;
    let result = if phase == "seal" {
        seal_claimed(ctx, &upload, attempt).await
    } else {
        let build = builds::Entity::find()
            .filter(builds::Column::UploadId.eq(upload_id))
            .one(&ctx.db)
            .await?
            .ok_or_else(ApiFailure::missing)?;
        validate_claimed(ctx, &app, &build, attempt).await
    };
    if let Err(error) = result {
        let terminal = error.status.is_client_error() || error.code == "artifact_changed";
        if let Some(job) = artifact_jobs::Entity::find_by_id((upload_id, phase.clone()))
            .filter(artifact_jobs::Column::AttemptId.eq(attempt))
            .one(&ctx.db)
            .await?
        {
            let next_state = if job.attempts >= 5 || terminal {
                "error"
            } else {
                "pending"
            };
            let mut job = job.into_active_model();
            job.state = Set(next_state.into());
            job.reason_code = Set(Some(error.code.to_owned()));
            job.lease_until = Set(Some(Utc::now() + Duration::seconds(30)));
            job.updated_at = Set(Utc::now());
            job.update(&ctx.db).await?;
        }
        if phase == "seal" {
            let failed = artifact_jobs::Entity::find_by_id((upload_id, "seal".to_owned()))
                .filter(artifact_jobs::Column::State.eq("error"))
                .one(&ctx.db)
                .await?
                .is_some();
            if failed {
                build_uploads::Entity::update_many()
                    .col_expr(
                        build_uploads::Column::State,
                        sea_orm::sea_query::Expr::value("expired"),
                    )
                    .col_expr(
                        build_uploads::Column::LeaseUntil,
                        sea_orm::sea_query::Expr::value(Option::<chrono::DateTime<Utc>>::None),
                    )
                    .filter(build_uploads::Column::Id.eq(upload_id))
                    .exec(&ctx.db)
                    .await?;
            }
        }
        return Err(error);
    }
    Ok(true)
}
async fn heartbeat(ctx: &AppContext, id: Uuid, phase: &str, attempt: Uuid) -> ApiResult<()> {
    let until = Utc::now() + Duration::seconds(LEASE_SECONDS);
    let job = artifact_jobs::Entity::find_by_id((id, phase.to_owned()))
        .filter(artifact_jobs::Column::State.eq("processing"))
        .filter(artifact_jobs::Column::AttemptId.eq(attempt))
        .filter(artifact_jobs::Column::LeaseUntil.gt(Utc::now()))
        .one(&ctx.db)
        .await?
        .ok_or_else(|| {
            ApiFailure::new(
                409,
                "validation_fenced",
                "Another artifact attempt owns this upload",
            )
        })?;
    let mut job = job.into_active_model();
    job.lease_until = Set(Some(until));
    job.updated_at = Set(Utc::now());
    job.update(&ctx.db).await?;
    if phase == "validate" {
        builds::Entity::update_many()
            .col_expr(
                builds::Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(Some(until)),
            )
            .filter(builds::Column::UploadId.eq(id))
            .filter(builds::Column::AttemptId.eq(attempt))
            .filter(builds::Column::ValidationState.eq("validating"))
            .exec(&ctx.db)
            .await?;
    }
    Ok(())
}
async fn with_lease<T>(
    ctx: &AppContext,
    id: Uuid,
    phase: &str,
    attempt: Uuid,
    work: impl std::future::Future<Output = ApiResult<T>>,
) -> ApiResult<T> {
    let work = tokio::time::timeout(std::time::Duration::from_secs(DEADLINE_SECONDS), work);
    tokio::pin!(work);
    let mut beat = tokio::time::interval(std::time::Duration::from_secs(30));
    loop {
        tokio::select! {result=&mut work=>return result.map_err(|_|ApiFailure::new(503,"validation_timeout","Artifact preparation timed out; retry validation"))?,_=beat.tick()=>heartbeat(ctx,id,phase,attempt).await?}
    }
}

// Seal and validate are separate because they have different storage requirements. Seal is a multipart upload to the backend, while validate is a download and inspection of the sealed object.
pub async fn validate_claimed(
    ctx: &AppContext,
    app: &apps::Model,
    row: &builds::Model,
    attempt: Uuid,
) -> ApiResult<()> {
    let setup = Setup::get(ctx);
    let work = async {
        let scratch = setup
            .store
            .materialize_sized(&row.storage_key, &row.storage_backend, row.byte_size)
            .await?;
        Ok(apk_validation::inspect(
            &setup,
            &scratch.0,
            row.byte_size,
            &row.sha256,
            &app.android_package,
        )
        .await)
    };
    let outcome = match with_lease(ctx, row.upload_id, "validate", attempt, work).await {
        Ok(outcome) => outcome,
        Err(error) if error.code == "validation_fenced" => return Err(error),
        Err(error) => apk_validation::Inspection::infrastructure(
            error.code,
            "Stored APK could not be validated; retry validation",
        ),
    };
    let state = word(&outcome.state);
    let metadata = outcome.metadata.map(|value| json(&value)).transpose()?;
    let tx = ctx.db.begin().await?;
    let current = builds::Entity::find_by_id(row.id)
        .filter(builds::Column::AttemptId.eq(attempt))
        .filter(builds::Column::ValidationState.eq("validating"))
        .filter(builds::Column::LeaseUntil.gt(Utc::now()))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| ApiFailure::new(409, "validation_fenced", "Validation ownership changed"))?;
    let mut current = current.into_active_model();
    current.validation_state = Set(state.clone());
    current.reason_code = Set(outcome.code);
    current.message = Set(outcome.message);
    current.metadata = Set(metadata);
    current.validated_at = Set(Some(Utc::now()));
    current.lease_until = Set(None);
    current.update(&tx).await?;
    let job = artifact_jobs::Entity::find_by_id((row.upload_id, "validate".to_owned()))
        .filter(artifact_jobs::Column::AttemptId.eq(attempt))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let mut job = job.into_active_model();
    job.state = Set("done".into());
    job.lease_until = Set(None);
    job.updated_at = Set(Utc::now());
    job.update(&tx).await?;
    tx.commit().await?;
    tracing::info!(app_id=%app.id,build_id=%row.id,phase=%state,"APK validation finished");
    Ok(())
}
async fn seal_claimed(
    ctx: &AppContext,
    upload: &build_uploads::Model,
    attempt: Uuid,
) -> ApiResult<()> {
    let setup = Setup::get(ctx);
    let multipart = artifact_multipart::Entity::find_by_id(upload.id)
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if multipart.state != "completing" {
        return Err(ApiFailure::new(
            409,
            "multipart_conflict",
            "Multipart sealing is no longer active",
        ));
    }
    let key = multipart.storage_key;
    let provider = multipart
        .provider_upload_id
        .ok_or_else(ApiFailure::internal)?;
    let parts = artifact_parts::Entity::find()
        .filter(artifact_parts::Column::UploadId.eq(upload.id))
        .order_by_asc(artifact_parts::Column::PartNumber)
        .all(&ctx.db)
        .await?;
    let completed = parts
        .iter()
        .map(|part| {
            Ok(MultipartPart {
                part_number: part.part_number as u16,
                etag: part.etag.clone().ok_or_else(ApiFailure::internal)?,
            })
        })
        .collect::<ApiResult<Vec<_>>>()?;
    let work = async {
        // Complete is safe to retry. An uncertain response may mean the immutable object exists.
        let completion = setup
            .store
            .multipart_complete(&key, &provider, &completed)
            .await;
        let scratch = match setup
            .store
            .materialize_sized(&key, &upload.storage_backend, upload.expected_size)
            .await
        {
            Ok(file) => file,
            Err(error) => return Err(completion.err().unwrap_or(error)),
        };
        let digest = verify_parts(&scratch.0, upload.expected_size, &parts).await?;
        Ok(digest)
    };
    let digest = with_lease(ctx, upload.id, "seal", attempt, work).await?;
    let tx = ctx.db.begin().await?;
    let locked_upload = build_uploads::Entity::find_by_id(upload.id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let locked_multipart = artifact_multipart::Entity::find_by_id(upload.id)
        .filter(artifact_multipart::Column::State.eq("completing"))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let job = artifact_jobs::Entity::find_by_id((upload.id, "seal".to_owned()))
        .filter(artifact_jobs::Column::State.eq("processing"))
        .filter(artifact_jobs::Column::AttemptId.eq(attempt))
        .filter(artifact_jobs::Column::LeaseUntil.gt(Utc::now()))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            ApiFailure::new(
                409,
                "validation_fenced",
                "Multipart sealing ownership changed",
            )
        })?;
    let mut job = job.into_active_model();
    job.state = Set("done".into());
    job.lease_until = Set(None);
    job.updated_at = Set(Utc::now());
    job.update(&tx).await?;
    let mut locked_upload = locked_upload.into_active_model();
    locked_upload.state = Set("uploaded".into());
    locked_upload.sealed_storage_key = Set(Some(key));
    locked_upload.actual_size = Set(Some(upload.expected_size));
    locked_upload.sha256 = Set(Some(digest));
    locked_upload.lease_until = Set(None);
    locked_upload.update(&tx).await?;
    let mut locked_multipart = locked_multipart.into_active_model();
    locked_multipart.state = Set("sealed".into());
    locked_multipart.update(&tx).await?;
    tx.commit().await?;
    Ok(())
}
async fn verify_parts(
    path: &std::path::Path,
    size: i64,
    parts: &[artifact_parts::Model],
) -> ApiResult<String> {
    let mut file = tokio::fs::File::open(path).await?;
    let mut buf = vec![0u8; 65536];
    let mut full = Sha256::new();
    let mut total = 0i64;
    for (index, part) in parts.iter().enumerate() {
        let number = part.part_number;
        let expected = part.byte_size as usize;
        if number != index as i32 + 1 || expected > PART_SIZE as usize {
            return Err(ApiFailure::internal());
        }
        let mut remaining = expected;
        let mut hash = Sha256::new();
        while remaining > 0 {
            let limit = remaining.min(buf.len());
            let n = file.read(&mut buf[..limit]).await?;
            if n == 0 {
                return Err(ApiFailure::new(
                    409,
                    "size_mismatch",
                    "Stored APK differs from its declared length",
                ));
            }
            hash.update(&buf[..n]);
            full.update(&buf[..n]);
            remaining -= n;
            total += n as i64;
        }
        if format!("{:x}", hash.finalize()) != part.sha256 {
            return Err(ApiFailure::new(
                409,
                "upload_file_changed",
                "Stored APK differs from the authorized part fingerprint",
            ));
        }
    }
    if total != size || file.read(&mut buf[..1]).await? != 0 {
        return Err(ApiFailure::new(
            409,
            "size_mismatch",
            "Stored APK differs from its declared length",
        ));
    }
    Ok(format!("{:x}", full.finalize()))
}
