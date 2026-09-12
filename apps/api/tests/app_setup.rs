//! Real PostgreSQL/routes and real signed APK tools. Fixtures are synthetic, not device proof.
use axum_test::{
    multipart::{MultipartForm, Part},
    TestRequest, TestResponse, TestServer,
};
use chrono::{Duration, Utc};
use loco_rs::{app::AppContext, testing::prelude::request};
use mobile_qa::{
    app::App,
    config::Setup,
    models::_entities::{build_uploads, builds, memberships, sessions, users},
    services::{apps, auth},
    tasks,
};
use mobile_qa_contracts::browser::*;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use uuid::Uuid;
// Serialize first boot/migration of the shared non-destructive local test database.
static DATABASE_BOOT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
const ORIGIN: &str = "http://127.0.0.1:5173";
struct Login {
    cookie: String,
    csrf: String,
    user: Uuid,
    org: Uuid,
    email: String,
    password: String,
}
impl Login {
    fn read(&self, req: TestRequest) -> TestRequest {
        req.add_header("cookie", &self.cookie)
    }
    fn write(&self, req: TestRequest) -> TestRequest {
        self.read(req)
            .add_header("origin", ORIGIN)
            .add_header("x-csrf-token", &self.csrf)
    }
}
async fn login(server: &TestServer, ctx: &AppContext) -> Login {
    let email = format!("{}@fixture.invalid", Uuid::new_v4());
    let password = loco_rs::hash::random_string(32);
    let (user, org) = auth::provision(
        ctx,
        &email,
        &password,
        "Synthetic operator",
        "Synthetic organization",
    )
    .await
    .unwrap();
    let response = server
        .post("/api/auth/login")
        .add_header("origin", ORIGIN)
        .add_header("x-mobile-qa-request", "1")
        .json(&LoginRequest {
            email: email.clone(),
            password: password.clone(),
        })
        .await;
    response.assert_status_ok();
    let cookie = response.header("set-cookie").to_str().unwrap().to_owned();
    assert!(
        cookie.contains("HttpOnly")
            && cookie.contains("SameSite=Strict")
            && cookie.contains("Path=/")
    );
    let body = response.json::<SessionResponse>();
    assert_eq!(body.user.id, user);
    Login {
        cookie: cookie.split(';').next().unwrap().into(),
        csrf: body.csrf_token,
        user,
        org,
        email,
        password,
    }
}
fn create_input(org: Uuid) -> CreateAppRequest {
    CreateAppRequest {
        organization_id: org,
        name: "Fixture App".into(),
        android_package: "com.mobileqa.fixture".into(),
        environment_name: "Staging".into(),
        backend_origins: vec!["https://staging.fixture.invalid".into()],
        login_origins: vec![],
    }
}
fn error(response: &TestResponse, status: u16) {
    assert_eq!(
        response.status_code().as_u16(),
        status,
        "{}",
        response.text()
    );
    let err = response.json::<ApiError>();
    assert_eq!(err.request_id.to_string(), response.header("x-request-id"));
    assert!(err.details.is_none());
    assert_eq!(response.header("cache-control"), "no-store");
}
async fn new_upload(server: &TestServer, login: &Login, app: Uuid, bytes: &[u8]) -> UploadResponse {
    let path = format!("/api/apps/{app}/build-uploads");
    let res = login
        .write(server.post(&path))
        .json(&CreateBuildUploadRequest {
            original_filename: "fixture.apk".into(),
            expected_size: bytes.len() as i64,
        })
        .await;
    res.assert_status(axum::http::StatusCode::CREATED);
    res.json()
}
async fn transfer(
    server: &TestServer,
    login: &Login,
    app: Uuid,
    id: Uuid,
    bytes: Vec<u8>,
) -> TestResponse {
    login
        .write(server.put(&format!("/api/apps/{app}/build-uploads/{id}/content")))
        .multipart(
            MultipartForm::new().add_part(
                "file",
                Part::bytes(bytes)
                    .file_name("fixture.apk")
                    .mime_type("application/octet-stream"),
            ),
        )
        .await
}
fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../.private/test-apks/{name}.apk"))).expect("Run python3 scripts/apk_fixtures.py with Android Build Tools 36.0.0 + JDK17 first; real-tool tests never skip")
}

#[tokio::test]
async fn persisted_workflow_auth_scope_and_real_validation() {
    let _database = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let foreign = login(&server, &ctx).await;
        let app_res = owner
            .write(server.post("/api/apps"))
            .json(&create_input(owner.org))
            .await;
        app_res.assert_status(axum::http::StatusCode::CREATED);
        let app = app_res.json::<AppResponse>();
        assert!(!app.readiness.execution_ready);
        assert_eq!(app.readiness.install, "not_checked");
        let base = format!("/api/apps/{}", app.id);
        owner
            .read(server.get("/api/auth/session"))
            .await
            .assert_status_ok();
        error(&server.get(&base).await, 401);
        error(&foreign.read(server.get(&base)).await, 404);
        error(
            &owner
                .read(server.post("/api/apps"))
                .json(&create_input(owner.org))
                .await,
            403,
        );
        error(
            &owner
                .write(server.post("/api/apps"))
                .json(&create_input(owner.org))
                .await,
            409,
        );
        let list = owner
            .read(server.get("/api/apps?limit=1"))
            .await
            .json::<AppListResponse>();
        assert_eq!(list.items[0].id, app.id);
        let settings = owner
            .read(server.get("/api/settings"))
            .await
            .json::<SettingsResponse>();
        assert_eq!(settings.upload_ttl_seconds, 1800);
        let metadata: serde_json::Value = serde_json::from_slice(
            &std::fs::read(
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../.private/test-apks/expected.json"),
            )
            .unwrap(),
        )
        .unwrap();
        let bytes = fixture("valid");
        let upload = new_upload(&server, &owner, app.id, &bytes).await;
        let upload_path = format!("{base}/build-uploads/{}", upload.id);
        transfer(&server, &owner, app.id, upload.id, bytes)
            .await
            .assert_status_ok();
        let sealed = owner
            .read(server.get(&upload_path))
            .await
            .json::<UploadResponse>();
        assert_eq!(sealed.state, UploadState::Uploaded);
        error(&foreign.read(server.get(&upload_path)).await, 404);
        let complete = format!("{upload_path}/complete");
        let (first, second) = tokio::join!(
            owner.write(server.post(&complete)),
            owner.write(server.post(&complete))
        );
        assert!([200, 202].contains(&first.status_code().as_u16()));
        assert!([200, 202].contains(&second.status_code().as_u16()));
        assert_eq!(
            first.json::<BuildResponse>().id,
            second.json::<BuildResponse>().id
        );
        let result = owner.write(server.post(&complete)).await;
        result.assert_status_ok();
        let build = result.json::<BuildResponse>();
        assert_eq!(
            build.validation.state,
            ValidationState::Validated,
            "{:?}",
            build.validation
        );
        assert_eq!(build.sha256, metadata["valid"]["sha256"]);
        assert_eq!(build.byte_size, metadata["valid"]["size"].as_i64().unwrap());
        let parsed = build.metadata.as_ref().unwrap();
        assert_eq!(parsed.package_name, "com.mobileqa.fixture");
        assert_eq!(parsed.version_name.as_deref(), Some("1.2 fixture"));
        assert_eq!(parsed.version_code, "7");
        assert_eq!(parsed.min_sdk, 23);
        assert_eq!(parsed.target_sdk, Some(35));
        assert!(parsed.signature_verified && parsed.native_abis.is_empty());
        assert!(!build.readiness.execution_ready);
        let body = result.text();
        for key in [
            "storage_key",
            "password_hash",
            "locator",
            "sealed_storage_key",
        ] {
            assert!(!body.contains(key));
        }
        let build_path = format!("{base}/builds/{}", build.id);
        assert_eq!(
            owner
                .read(server.get(&build_path))
                .await
                .json::<BuildResponse>(),
            build
        );
        error(&foreign.read(server.get(&build_path)).await, 404);
        let history = owner
            .read(server.get(&format!("{base}/builds")))
            .await
            .json::<BuildListResponse>();
        assert_eq!(history.items.len(), 1);
        assert_eq!(history.items[0].id, build.id);
        assert_eq!(
            builds::Entity::find()
                .filter(builds::Column::UploadId.eq(upload.id))
                .all(&ctx.db)
                .await
                .unwrap()
                .len(),
            1
        );
        for (name, state, code) in [
            ("corrupt", ValidationState::Invalid, "invalid_apk"),
            ("unsigned", ValidationState::Invalid, "invalid_signature"),
            ("tampered", ValidationState::Invalid, "invalid_signature"),
            ("mismatch", ValidationState::Invalid, "package_mismatch"),
            (
                "unsupported",
                ValidationState::Unsupported,
                "unsupported_device_policy",
            ),
        ] {
            let bytes = fixture(name);
            let next = new_upload(&server, &owner, app.id, &bytes).await;
            transfer(&server, &owner, app.id, next.id, bytes)
                .await
                .assert_status_ok();
            let result = owner
                .write(server.post(&format!("{base}/build-uploads/{}/complete", next.id)))
                .await;
            result.assert_status_ok();
            let parsed = result.json::<BuildResponse>();
            assert_eq!(
                parsed.validation.state, state,
                "{name}: {:?}",
                parsed.validation
            );
            assert_eq!(parsed.validation.reason_code.as_deref(), Some(code));
            assert!(!parsed.readiness.execution_ready);
        }
        let env = UpdateEnvironmentRequest {
            expected_revision: app.environment.revision,
            name: "QA".into(),
            backend_origins: vec!["https://qa.fixture.invalid".into()],
            login_origins: vec![],
            account_secret_reference_id: None,
            reset_secret_reference_id: None,
        };
        let mut vars = loco_rs::task::Vars::default();
        for (key, value) in [
            ("action", "observe".into()),
            ("actor", owner.user.to_string()),
            ("organization", owner.org.to_string()),
            ("app", app.id.to_string()),
            ("revision", "1".into()),
            ("kind", "backend".into()),
            ("state", "operator_reported_ok".into()),
        ] {
            vars.cli.insert(key.into(), value);
        }
        tasks::operator::execute(&ctx, &vars).await.unwrap();
        assert_eq!(
            owner
                .read(server.get(&base))
                .await
                .json::<AppResponse>()
                .readiness
                .backend,
            CheckState::OperatorReportedOk
        );
        let changed = owner
            .write(server.patch(&format!("{base}/environment")))
            .json(&env)
            .await;
        changed.assert_status_ok();
        assert_eq!(changed.json::<EnvironmentResponse>().revision, 2);
        assert!(changed
            .json::<EnvironmentResponse>()
            .checks
            .iter()
            .all(|c| c.state == CheckState::NotChecked));
        error(
            &owner
                .write(server.patch(&format!("{base}/environment")))
                .json(&env)
                .await,
            409,
        );
        // Active same-org member without app grant cannot discover this app.
        memberships::ActiveModel {
            id: Set(Uuid::new_v4()),
            user_id: Set(foreign.user),
            organization_id: Set(owner.org),
            role: Set("member".into()),
            active: Set(true),
        }
        .insert(&ctx.db)
        .await
        .unwrap();
        error(&foreign.read(server.get(&base)).await, 404);
        let logout = owner.write(server.post("/api/auth/logout")).await;
        logout.assert_status_ok();
        assert!(logout.json::<LogoutResponse>().signed_out);
        error(&owner.read(server.get(&build_path)).await, 401);
        // A fresh cookie/session can retrieve the same persisted record, without reuse of client cache.
        let login_res = server
            .post("/api/auth/login")
            .add_header("origin", ORIGIN)
            .add_header("x-mobile-qa-request", "1")
            .json(&LoginRequest {
                email: owner.email,
                password: owner.password,
            })
            .await;
        login_res.assert_status_ok();
        let fresh = login_res
            .header("set-cookie")
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_owned();
        assert_eq!(
            server
                .get(&build_path)
                .add_header("cookie", fresh)
                .await
                .json::<BuildResponse>()
                .id,
            build.id
        );
    })
    .await;
}

#[tokio::test]
async fn rejection_recovery_and_infrastructure_failure() {
    let _database = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let app = owner
            .write(server.post("/api/apps"))
            .json(&create_input(owner.org))
            .await
            .json::<AppResponse>();
        let base = format!("/api/apps/{}", app.id);
        error(
            &server
                .post("/api/auth/login")
                .add_header("origin", "https://foreign.invalid")
                .add_header("x-mobile-qa-request", "1")
                .json(&LoginRequest {
                    email: owner.email.clone(),
                    password: owner.password.clone(),
                })
                .await,
            403,
        );
        error(
            &owner
                .write(server.post("/api/apps"))
                .text("not json")
                .content_type("application/json")
                .await,
            400,
        );
        error(&owner.read(server.get("/api/apps/not-a-uuid")).await, 400);
        error(&owner.read(server.get("/api/apps?limit=0")).await, 422);
        let endpoint = format!("{base}/build-uploads");
        error(
            &owner
                .write(server.post(&endpoint))
                .json(&CreateBuildUploadRequest {
                    original_filename: "huge.apk".into(),
                    expected_size: mobile_qa::config::MAX_APK + 1,
                })
                .await,
            413,
        );
        let data = fixture("valid");
        let upload = new_upload(&server, &owner, app.id, &data).await;
        error(
            &transfer(&server, &owner, app.id, upload.id, b"short".to_vec()).await,
            409,
        );
        error(
            &owner
                .write(server.post(&format!("{endpoint}/{}/complete", upload.id)))
                .await,
            409,
        );
        let res = owner
            .read(server.get(&format!("{endpoint}/{}", upload.id)))
            .await
            .json::<UploadResponse>();
        assert_eq!(res.state, UploadState::Pending);
        // Simulate a crashed receiver lease, then reclaim it through the real HTTP operation.
        let mut row: build_uploads::ActiveModel = build_uploads::Entity::find_by_id(upload.id)
            .one(&ctx.db)
            .await
            .unwrap()
            .unwrap()
            .into();
        row.state = Set("receiving".into());
        row.attempt_id = Set(Some(Uuid::new_v4()));
        row.lease_until = Set(Some(Utc::now() - Duration::seconds(1)));
        row.update(&ctx.db).await.unwrap();
        transfer(&server, &owner, app.id, upload.id, data.clone())
            .await
            .assert_status_ok();
        error(
            &transfer(&server, &owner, app.id, upload.id, data).await,
            409,
        );
        let setup = Setup::get(&ctx);
        let mut unavailable = setup.clone();
        unavailable.sdk = setup.root.join("absent-tools");
        ctx.shared_store.insert(unavailable);
        let complete = format!("{endpoint}/{}/complete", upload.id);
        let failed = owner
            .write(server.post(&complete))
            .await
            .json::<BuildResponse>();
        assert_eq!(failed.validation.state, ValidationState::Error);
        assert!(failed.can_retry_validation);
        ctx.shared_store.insert(setup.clone());
        let retried = owner
            .write(server.post(&complete))
            .await
            .json::<BuildResponse>();
        assert_eq!(retried.id, failed.id);
        assert_eq!(retried.validation.state, ValidationState::Validated);
        let exp = new_upload(&server, &owner, app.id, b"test").await;
        let mut row: build_uploads::ActiveModel = build_uploads::Entity::find_by_id(exp.id)
            .one(&ctx.db)
            .await
            .unwrap()
            .unwrap()
            .into();
        row.expires_at = Set(Utc::now() - Duration::hours(2));
        row.update(&ctx.db).await.unwrap();
        error(
            &transfer(&server, &owner, app.id, exp.id, b"test".to_vec()).await,
            410,
        );
        // A publication interrupted before DB sealing leaves an orphan attempt object.
        let orphan_key = format!("{}/{}/{}/{}", owner.org, app.id, exp.id, Uuid::new_v4());
        let (scratch, mut file) = setup.store.scratch().await.unwrap();
        use tokio::io::AsyncWriteExt;
        file.write_all(b"orphan").await.unwrap();
        file.sync_all().await.unwrap();
        drop(file);
        setup.store.publish(&orphan_key, &scratch.0).await.unwrap();
        let orphan_path = setup.root.join("objects").join(&orphan_key);
        std::fs::File::open(&orphan_path)
            .unwrap()
            .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(7200))
            .unwrap();
        assert!(tasks::cleanup::execute(&ctx, false).await.unwrap() >= 1);
        assert!(orphan_path.exists());
        assert!(tasks::cleanup::execute(&ctx, true).await.unwrap() >= 1);
        assert!(!orphan_path.exists());
        assert!(setup
            .store
            .materialize(
                &builds::Entity::find_by_id(retried.id)
                    .one(&ctx.db)
                    .await
                    .unwrap()
                    .unwrap()
                    .storage_key,
                "local"
            )
            .await
            .is_ok());
        // Disabled and expired/revoked sessions never trust JWT validity alone.
        users::Entity::update_many()
            .col_expr(
                users::Column::DisabledAt,
                sea_orm::sea_query::Expr::value(Utc::now()),
            )
            .filter(users::Column::Id.eq(owner.user))
            .exec(&ctx.db)
            .await
            .unwrap();
        error(&owner.read(server.get(&base)).await, 401);
        users::Entity::update_many()
            .col_expr(
                users::Column::DisabledAt,
                sea_orm::sea_query::Expr::value(None::<chrono::DateTime<Utc>>),
            )
            .filter(users::Column::Id.eq(owner.user))
            .exec(&ctx.db)
            .await
            .unwrap();
        sessions::Entity::update_many()
            .col_expr(
                sessions::Column::ExpiresAt,
                sea_orm::sea_query::Expr::value(Utc::now() - Duration::seconds(1)),
            )
            .filter(sessions::Column::UserId.eq(owner.user))
            .exec(&ctx.db)
            .await
            .unwrap();
        error(&owner.read(server.get(&base)).await, 401);
    })
    .await;
}

#[tokio::test]
async fn every_declared_route_requires_the_declared_security_and_errors() {
    let _database = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, _ctx| async move {
        let spec = serde_json::to_value(mobile_qa_contracts::browser::openapi()).unwrap();
        for &(method, path, operation, status) in OPERATIONS {
            assert_eq!(
                spec["paths"][path][method.to_lowercase()]["operationId"],
                operation
            );
            assert!(
                spec["paths"][path][method.to_lowercase()]["responses"][status.to_string()]
                    .is_object()
            );
            if path == "/api/health" {
                continue;
            }
            let path = path
                .replace("{app_id}", &Uuid::new_v4().to_string())
                .replace("{upload_id}", &Uuid::new_v4().to_string())
                .replace("{build_id}", &Uuid::new_v4().to_string());
            let req = match method.to_lowercase().as_str() {
                "get" => server.get(&path),
                "post" => server.post(&path),
                "put" => server.put(&path),
                "patch" => server.patch(&path),
                _ => panic!("unknown operation"),
            };
            error(&req.await, if operation == "login" { 403 } else { 401 });
        }
    })
    .await;
}

#[test]
fn server_input_and_metadata_boundaries() {
    assert!(apps::package("com.example.app").is_ok());
    for input in ["single", "com.1x", "com..app", "com.example app"] {
        assert!(apps::package(input).is_err());
    }
    for origin in [
        "https://x.invalid/path",
        "https://user:pass@x.invalid",
        "https://x.invalid?token=x",
        "http://remote.invalid",
    ] {
        assert!(apps::origins(&[origin.into()], true, true).is_err());
    }
    assert!(apps::origins(&[], true, true).is_err());
    assert!(apps::origins(&["http://127.0.0.1:4000".into()], true, false).is_ok());
    let text="package: name='com.example.app' versionCode='2' versionCodeMajor='3' versionName='Name with spaces'\nsdkVersion:'23'\ntargetSdkVersion:'35'";
    let meta = mobile_qa::services::apk_validation::metadata(text, "", vec![]).unwrap();
    assert_eq!(meta.version_code, "12884901890");
    assert_eq!(meta.version_name.as_deref(), Some("Name with spaces"));
    assert!(mobile_qa::services::apk_validation::metadata(text, "E: uses-split", vec![]).is_err());
    assert!(mobile_qa::services::apk_validation::metadata(
        &text.replace("sdkVersion:'23'", "sdkVersion:'Preview'"),
        "",
        vec![]
    )
    .is_err());
}

#[tokio::test]
async fn throttles_quota_and_revision_scope() {
    let _database = DATABASE_BOOT.lock().await;
    request::<App, _, _>(|server, ctx| async move {
        let owner = login(&server, &ctx).await;
        let app = owner
            .write(server.post("/api/apps"))
            .json(&create_input(owner.org))
            .await
            .json::<AppResponse>();
        let endpoint = format!("/api/apps/{}/build-uploads", app.id);
        for _ in 0..4 {
            new_upload(&server, &owner, app.id, b"test").await;
        }
        let quota = owner
            .write(server.post(&endpoint))
            .json(&CreateBuildUploadRequest {
                original_filename: "fixture.apk".into(),
                expected_size: 4,
            })
            .await;
        error(&quota, 429);
        assert!(quota.headers().contains_key("retry-after"));
        let bad = LoginRequest {
            email: format!("{}@invalid.example", Uuid::new_v4()),
            password: "incorrect".into(),
        };
        for _ in 0..10 {
            error(
                &server
                    .post("/api/auth/login")
                    .add_header("origin", ORIGIN)
                    .add_header("x-mobile-qa-request", "1")
                    .json(&bad)
                    .await,
                401,
            );
        }
        error(
            &server
                .post("/api/auth/login")
                .add_header("origin", ORIGIN)
                .add_header("x-mobile-qa-request", "1")
                .json(&bad)
                .await,
            429,
        );
        let update = UpdateEnvironmentRequest {
            expected_revision: 1,
            name: "Staging".into(),
            backend_origins: vec!["https://staging.fixture.invalid".into()],
            login_origins: vec![],
            account_secret_reference_id: Some(Uuid::new_v4()),
            reset_secret_reference_id: None,
        };
        error(
            &owner
                .write(server.patch(&format!("/api/apps/{}/environment", app.id)))
                .json(&update)
                .await,
            404,
        );
    })
    .await;
}
