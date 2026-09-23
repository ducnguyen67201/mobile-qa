//! Real HTTP/SQL boundaries; synthetic objects never contact a cloud provider or Android.
mod support;
use chrono::{Duration, Utc};
use mobile_qa::{
    models::_entities::{
        artifact_jobs, artifact_multipart, artifact_parts, builds, phone_sessions,
    },
    services::{execution_store::hash, upload_validation},
};
use mobile_qa_contracts::artifacts_api::*;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set};
use support::*;

#[tokio::test]
async fn inclusive_size_metadata_and_multipart_auth_boundaries() {
    let _database = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let foreign = login(&server, &ctx).await;
        let app = owner
            .write(server.post("/api/apps"))
            .json(&create_input(owner.org))
            .await
            .json::<AppResponse>();
        let base = format!("/api/apps/{}/build-uploads", app.id);
        for size in [2147483647, 2147483648] {
            let response = owner
                .write(server.post(&base))
                .json(&CreateBuildUploadRequest {
                    original_filename: "large.apk".into(),
                    expected_size: size,
                })
                .await;
            response.assert_status(axum::http::StatusCode::CREATED);
            assert_eq!(response.json::<UploadResponse>().expected_size, size);
        }
        let oversized = owner
            .write(server.post(&base))
            .json(&CreateBuildUploadRequest {
                original_filename: "huge.apk".into(),
                expected_size: 2147483649,
            })
            .await;
        error(&oversized, 413);

        let upload = new_upload(&server, &owner, app.id, b"synthetic").await;
        let path = format!("{base}/{}/multipart", upload.id);
        error(&foreign.write(server.post(&path)).await, 404);
        error(&owner.read(server.post(&path)).await, 403);
        error(&owner.write(server.post(&path)).await, 503);
        let settings = owner
            .read(server.get("/api/settings"))
            .await
            .json::<SettingsResponse>();
        assert_eq!(settings.max_apk_bytes, 2147483648);
        assert!(settings.multipart.is_none());

        assert_eq!(
            phone_sessions::Column::BuildBytes.def().get_column_type(),
            &sea_orm::ColumnType::BigInteger
        );
    })
    .await;
}

#[tokio::test]
async fn immutable_part_fingerprints_and_completion_are_fenced() {
    let _database = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let app = owner
            .write(server.post("/api/apps"))
            .json(&create_input(owner.org))
            .await
            .json::<AppResponse>();
        let upload = new_upload(&server, &owner, app.id, b"abc").await;
        let key = format!("{}/{}/{}/{}", owner.org, app.id, upload.id, Uuid::new_v4());
        artifact_multipart::ActiveModel {
            upload_id: Set(upload.id),
            storage_key: Set(key),
            provider_upload_id: Set(Some("synthetic".into())),
            state: Set("uploading".into()),
            operation_id: Set(Uuid::new_v4()),
            lease_until: Set(None),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .unwrap();
        artifact_parts::ActiveModel {
            upload_id: Set(upload.id),
            part_number: Set(1),
            byte_size: Set(3),
            sha256: Set(hash(b"abc")),
            etag: Set(None),
        }
        .insert(&ctx.db)
        .await
        .unwrap();

        let base = format!("/api/apps/{}/build-uploads/{}/multipart", app.id, upload.id);
        let changed = owner
            .write(server.post(&format!("{base}/parts")))
            .json(&AuthorizeUploadPartRequest {
                part_number: 1,
                byte_size: 3,
                sha256: hash(b"xyz"),
            })
            .await;
        error(&changed, 409);
        assert_eq!(changed.json::<ApiError>().code, "upload_file_changed");
        error(
            &owner.write(server.post(&format!("{base}/complete"))).await,
            409,
        );

        owner
            .write(server.put(&format!("{base}/parts/1")))
            .json(&ConfirmUploadPartRequest {
                etag: "\"synthetic-etag\"".into(),
            })
            .await
            .assert_status_ok();
        let receipt = owner
            .read(server.get(&base))
            .await
            .json::<MultipartUpload>();
        assert_eq!(receipt.parts[0].sha256, hash(b"abc"));

        let complete_path = format!("{base}/complete");
        let (first, second) = tokio::join!(
            owner.write(server.post(&complete_path)),
            owner.write(server.post(&complete_path))
        );
        first.assert_status(axum::http::StatusCode::ACCEPTED);
        second.assert_status(axum::http::StatusCode::ACCEPTED);
        let jobs = artifact_jobs::Entity::find()
            .filter(artifact_jobs::Column::UploadId.eq(upload.id))
            .count(&ctx.db)
            .await
            .unwrap();
        assert_eq!(jobs, 1);
        error(&owner.write(server.delete(&base)).await, 409);

        // This route fixture has no provider; leave no runnable cloud job for later tests.
        artifact_jobs::Entity::update_many()
            .col_expr(
                artifact_jobs::Column::State,
                sea_orm::sea_query::Expr::value("error"),
            )
            .col_expr(
                artifact_jobs::Column::ReasonCode,
                sea_orm::sea_query::Expr::value(Some("synthetic_fixture")),
            )
            .filter(artifact_jobs::Column::UploadId.eq(upload.id))
            .exec(&ctx.db)
            .await
            .unwrap();
    })
    .await;
}

#[tokio::test]
async fn expired_validation_attempt_recovers_from_durable_job() {
    let _database = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let app = owner
            .write(server.post("/api/apps"))
            .json(&create_input(owner.org))
            .await
            .json::<AppResponse>();
        let bytes = fixture("valid");
        let upload = new_upload(&server, &owner, app.id, &bytes).await;
        transfer(&server, &owner, app.id, upload.id, bytes)
            .await
            .assert_status_ok();
        let path = format!("/api/apps/{}/build-uploads/{}/complete", app.id, upload.id);
        let build = owner
            .write(server.post(&path))
            .await
            .json::<BuildResponse>();
        let abandoned = Uuid::new_v4();
        let expired = Utc::now() - Duration::seconds(1);
        builds::Entity::update_many()
            .col_expr(
                builds::Column::ValidationState,
                sea_orm::sea_query::Expr::value("validating"),
            )
            .col_expr(
                builds::Column::AttemptId,
                sea_orm::sea_query::Expr::value(Some(abandoned)),
            )
            .col_expr(
                builds::Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(Some(expired)),
            )
            .col_expr(
                builds::Column::ValidatedAt,
                sea_orm::sea_query::Expr::value(None::<chrono::DateTime<Utc>>),
            )
            .filter(builds::Column::Id.eq(build.id))
            .exec(&ctx.db)
            .await
            .unwrap();
        artifact_jobs::Entity::update_many()
            .col_expr(
                artifact_jobs::Column::State,
                sea_orm::sea_query::Expr::value("processing"),
            )
            .col_expr(
                artifact_jobs::Column::AttemptId,
                sea_orm::sea_query::Expr::value(Some(abandoned)),
            )
            .col_expr(
                artifact_jobs::Column::LeaseUntil,
                sea_orm::sea_query::Expr::value(Some(expired)),
            )
            .filter(artifact_jobs::Column::UploadId.eq(upload.id))
            .filter(artifact_jobs::Column::Phase.eq("validate"))
            .exec(&ctx.db)
            .await
            .unwrap();

        let recovery = upload_validation::run_once(&ctx);
        // Polling may be nested in an HTTP task or a Tokio worker with a small stack.
        assert!(std::mem::size_of_val(&recovery) < 65536);
        assert!(recovery.await.unwrap());
        let recovered = owner
            .read(server.get(&format!("/api/apps/{}/builds/{}", app.id, build.id)))
            .await
            .json::<BuildResponse>();
        assert_eq!(recovered.validation.state, ValidationState::Validated);
        assert_eq!(recovered.sha256, build.sha256);
    })
    .await;
}
