//! Process-scoped settings. Development/test never load production storage credentials.
use base64::Engine;
use loco_rs::{
    app::AppContext,
    config::{Auth, JWTLocation, JWTLocationConfig, JWT},
    environment::Environment,
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct Setup {
    pub origin: String,
    pub secure_cookie: bool,
    pub root: PathBuf,
    pub sdk: PathBuf,
    pub storage_backend: String,
    pub store: Arc<crate::storage::ArtifactStore>,
    pub google: Option<Arc<crate::services::google::Google>>,
    pub transfers: Arc<Semaphore>,
    pub validators: Arc<Semaphore>,
    pub archives: Arc<Semaphore>,
}
pub const MAX_APK: i64 = 250 * 1024 * 1024;
pub const SESSION_SECONDS: i64 = 3600;
pub const UPLOAD_SECONDS: i64 = 1800;
impl Setup {
    pub fn get(ctx: &AppContext) -> Self {
        ctx.shared_store.get::<Self>().expect("setup initialized")
    }
    pub fn cookie_name(&self) -> &'static str {
        if self.secure_cookie {
            "__Host-mobile_qa_session"
        } else {
            "mobile_qa_session"
        }
    }
    pub fn new(ctx: &AppContext) -> loco_rs::Result<Self> {
        let production = matches!(ctx.environment, Environment::Production);
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let origin = if production {
            ctx.config.server.host.clone()
        } else if matches!(ctx.environment, Environment::Development) {
            std::env::var("MOBILE_QA_DEV_ORIGIN")
                .unwrap_or_else(|_| "http://127.0.0.1:5173".to_owned())
        } else {
            "http://127.0.0.1:5173".to_owned()
        };
        let origin_url = url::Url::parse(&origin).map_err(loco_rs::Error::wrap)?;
        if production && origin_url.scheme() != "https" {
            return Err(loco_rs::Error::string("production HOST must use HTTPS"));
        }
        if !production && !matches!(origin_url.host_str(), Some("localhost" | "127.0.0.1")) {
            return Err(loco_rs::Error::string(
                "development origin must use loopback",
            ));
        }
        let root = if matches!(ctx.environment, Environment::Test) {
            repo.join(".private/test-artifacts").join(
                std::env::var("MOBILE_QA_TEST_SCOPE")
                    .ok()
                    .and_then(|s| uuid::Uuid::parse_str(&s).ok())
                    .unwrap_or_else(uuid::Uuid::new_v4)
                    .to_string(),
            )
        } else if production {
            std::env::var_os("ARTIFACT_SCRATCH_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| std::env::temp_dir().join("mobile-qa-artifacts"))
        } else {
            repo.join(".private/artifacts")
        };
        let sdk = std::env::var_os("MOBILE_QA_ANDROID_SDK")
            .map(PathBuf::from)
            .unwrap_or_else(|| repo.join(".private/android-sdk"));
        let store = crate::storage::ArtifactStore::new(root.clone(), production)?;
        Ok(Self {
            origin: origin_url.origin().ascii_serialization(),
            secure_cookie: production,
            root,
            sdk,
            storage_backend: if production {
                "pilot".into()
            } else {
                "local".into()
            },
            store: Arc::new(store),
            google: crate::services::google::Google::from_environment(&ctx.environment)?
                .map(Arc::new),
            transfers: Arc::new(Semaphore::new(2)),
            validators: Arc::new(Semaphore::new(1)),
            archives: Arc::new(Semaphore::new(1)),
        })
    }
}
/// Configure framework cookie JWT before boot; ephemeral local keys never touch disk.
pub fn configure_auth(
    environment: &Environment,
    config: &mut loco_rs::config::Config,
) -> loco_rs::Result<()> {
    let production = matches!(environment, Environment::Production);
    let secret = if production {
        std::env::var("JWT_SECRET").map_err(|_| loco_rs::Error::string("JWT_SECRET is required"))?
    } else {
        base64::engine::general_purpose::STANDARD.encode(loco_rs::hash::random_string(64))
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(&secret)
        .map_err(|_| loco_rs::Error::string("JWT_SECRET must be base64"))?;
    if bytes.len() < 64 {
        return Err(loco_rs::Error::string(
            "JWT_SECRET must contain at least 64 random bytes",
        ));
    }
    config.auth = Some(Auth {
        jwt: Some(JWT {
            secret,
            expiration: SESSION_SECONDS as u64,
            location: Some(JWTLocationConfig::Single(JWTLocation::Cookie {
                name: if production {
                    "__Host-mobile_qa_session".into()
                } else {
                    "mobile_qa_session".into()
                },
            })),
        }),
    });
    Ok(())
}
