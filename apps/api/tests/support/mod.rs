#![allow(dead_code, unused_imports)]
//! Real PostgreSQL/routes and real signed APK tools. Fixtures are synthetic, not device proof.
pub use axum_test::{
    multipart::{MultipartForm, Part},
    TestRequest, TestResponse, TestServer,
};
pub use chrono::{Duration, Utc};
pub use loco_rs::{app::AppContext, testing::prelude::request};
pub use mobile_qa::{
    app::App,
    config::Setup,
    models::_entities::{build_uploads, builds, memberships, sessions, users},
    services::{apps, auth, google},
    tasks,
};
pub use mobile_qa_contracts::browser::*;
pub use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
pub use uuid::Uuid;
// Serialize first boot/migration of the shared non-destructive local test database.
pub static DATABASE_BOOT: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
pub const ORIGIN: &str = "http://127.0.0.1:5173";
pub struct Login {
    pub cookie: String,
    pub csrf: String,
    pub user: Uuid,
    pub org: Uuid,
    pub email: String,
    pub subject: String,
}
impl Login {
    pub fn read(&self, req: TestRequest) -> TestRequest {
        req.add_header("cookie", &self.cookie)
    }
    pub fn write(&self, req: TestRequest) -> TestRequest {
        self.read(req)
            .add_header("origin", ORIGIN)
            .add_header("x-csrf-token", &self.csrf)
    }
}
pub const GOOGLE_CLIENT: &str = "synthetic.apps.googleusercontent.com";
pub fn google_keys() -> &'static (jsonwebtoken::EncodingKey, jsonwebtoken::jwk::JwkSet) {
    static KEYS: std::sync::OnceLock<(jsonwebtoken::EncodingKey, jsonwebtoken::jwk::JwkSet)> =
        std::sync::OnceLock::new();
    KEYS.get_or_init(|| {
        use base64::Engine;
        use rsa::{pkcs8::EncodePrivateKey, traits::PublicKeyParts};
        // Ephemeral signing material exists only in the test process, never in repository fixtures.
        let key = rsa::RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).unwrap();
        let pem = key.to_pkcs8_pem(rsa::pkcs8::LineEnding::LF).unwrap();
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let jwks = serde_json::json!({"keys":[{"kty":"RSA", "kid":"synthetic", "alg":"RS256", "use":"sig", "n":b64.encode(key.n().to_bytes_be()), "e":b64.encode(key.e().to_bytes_be())}]});
        (jsonwebtoken::EncodingKey::from_rsa_pem(pem.as_bytes()).unwrap(), serde_json::from_value(jwks).unwrap())
    })
}
pub fn configure_google(ctx: &AppContext) {
    let mut setup = Setup::get(ctx);
    setup.google = Some(std::sync::Arc::new(
        google::Google::for_test(ctx, GOOGLE_CLIENT.into(), google_keys().1.clone()).unwrap(),
    ));
    ctx.shared_store.insert(setup);
}
pub fn google_token(
    email: &str,
    subject: &str,
    nonce: &str,
    overrides: serde_json::Value,
) -> String {
    let mut claims = serde_json::json!({"iss":"https://accounts.google.com", "aud":GOOGLE_CLIENT,
        "exp":Utc::now().timestamp()+300, "sub":subject, "email":email, "email_verified":true, "nonce":nonce, "hd":"fixture.invalid"});
    if let Some(fields) = overrides.as_object() {
        claims.as_object_mut().unwrap().extend(fields.clone());
    }
    let mut header = jsonwebtoken::Header::new(jsonwebtoken::Algorithm::RS256);
    header.kid = Some("synthetic".into());
    jsonwebtoken::encode(&header, &claims, &google_keys().0).unwrap()
}
pub async fn challenge(server: &TestServer) -> (GoogleLoginChallenge, String) {
    let res = server
        .post("/api/auth/google/challenge")
        .add_header("origin", ORIGIN)
        .add_header("x-mobile-qa-request", "1")
        .await;
    res.assert_status_ok();
    (
        res.json(),
        res.header("set-cookie")
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .into(),
    )
}
pub async fn google_login(server: &TestServer, email: &str, subject: &str) -> TestResponse {
    let (challenge, cookie) = challenge(server).await;
    server
        .post("/api/auth/google/login")
        .add_header("origin", ORIGIN)
        .add_header("x-mobile-qa-request", "1")
        .add_header("cookie", cookie)
        .json(&LoginRequest {
            credential: google_token(email, subject, &challenge.nonce, serde_json::json!({})),
            challenge_id: challenge.challenge_id,
        })
        .await
}
pub async fn login(server: &TestServer, ctx: &AppContext) -> Login {
    configure_google(ctx);
    let email = format!("{}@fixture.invalid", Uuid::new_v4());
    let subject = Uuid::new_v4().to_string();
    let (user, org) = auth::provision(ctx, &email, "Synthetic operator", "Synthetic organization")
        .await
        .unwrap();
    let response = google_login(server, &email, &subject).await;
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
        subject,
    }
}
pub fn create_input(org: Uuid) -> CreateAppRequest {
    CreateAppRequest {
        organization_id: org,
        name: "Fixture App".into(),
        android_package: "com.mobileqa.fixture".into(),
        environment_name: "Staging".into(),
        backend_origins: vec!["https://staging.fixture.invalid".into()],
        login_origins: vec![],
    }
}
pub fn error(response: &TestResponse, status: u16) {
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
pub async fn new_upload(
    server: &TestServer,
    login: &Login,
    app: Uuid,
    bytes: &[u8],
) -> UploadResponse {
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
pub async fn transfer(
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
pub fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../.private/test-apks/{name}.apk"))).expect("Run python3 scripts/apk_fixtures.py with Android Build Tools 36.0.0 + JDK17 first; real-tool tests never skip")
}
