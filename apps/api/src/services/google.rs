//! Google signs identities; durable membership grants still authorize app access.
use crate::{
    config::Setup,
    errors::{ApiFailure, ApiResult},
    models::_entities::{google_login_challenges, memberships, users},
    services::auth,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, decode_header, jwk::JwkSet, Algorithm, DecodingKey, Validation};
use loco_rs::{app::AppContext, environment::Environment};
use mobile_qa_contracts::browser::{GoogleLoginChallenge, LoginRequest};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect, Set, TransactionTrait,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::time::{Duration as StdDuration, Instant};
use subtle::ConstantTimeEq;
use tokio::sync::Mutex;
use uuid::Uuid;

pub struct Google {
    pub client_id: String,
    http: reqwest::Client,
    keys: Mutex<Option<(JwkSet, Instant)>>,
    fixture_keys: bool,
}
#[derive(Clone, Deserialize)]
struct Identity {
    sub: String,
    email: String,
    email_verified: bool,
    nonce: String,
    hd: Option<String>,
    azp: Option<String>,
}
fn invalid() -> ApiFailure {
    ApiFailure::new(
        401,
        "google_sign_in_rejected",
        "Google sign-in could not be verified. Please try again",
    )
}
fn unavailable() -> ApiFailure {
    ApiFailure::new(
        503,
        "google_sign_in_unavailable",
        "Google sign-in is unavailable. Contact your workspace operator",
    )
}
impl Google {
    pub fn from_environment(environment: &Environment) -> loco_rs::Result<Option<Self>> {
        let client_id = std::env::var("GOOGLE_CLIENT_ID")
            .ok()
            .filter(|v| !v.trim().is_empty());
        let Some(client_id) = client_id else {
            return Ok(None);
        };
        // Synthetic keys may enter only explicitly isolated test processes, never dev/production.
        let fixture = if matches!(environment, Environment::Test) {
            std::env::var("MOBILE_QA_TEST_GOOGLE_JWKS")
                .ok()
                .map(|v| serde_json::from_str(&v))
                .transpose()
                .map_err(loco_rs::Error::wrap)?
        } else {
            None
        };
        Self::new(client_id, fixture).map(Some)
    }
    fn new(client_id: String, fixture: Option<JwkSet>) -> loco_rs::Result<Self> {
        Ok(Self {
            client_id,
            http: reqwest::Client::builder()
                .timeout(StdDuration::from_secs(10))
                .redirect(reqwest::redirect::Policy::none())
                .build()
                .map_err(loco_rs::Error::wrap)?,
            fixture_keys: fixture.is_some(),
            keys: Mutex::new(
                fixture.map(|keys| (keys, Instant::now() + StdDuration::from_secs(3600))),
            ),
        })
    }
    /// In-memory fixture injection requires a test AppContext; no HTTP or production override.
    pub fn for_test(ctx: &AppContext, client_id: String, keys: JwkSet) -> loco_rs::Result<Self> {
        if !matches!(ctx.environment, Environment::Test) {
            return Err(loco_rs::Error::string("Test context required"));
        }
        Self::new(client_id, Some(keys))
    }
    async fn identity(&self, token: &str, nonce: &str) -> ApiResult<Identity> {
        if token.len() > 16384 {
            return Err(invalid());
        }
        let header = decode_header(token).map_err(|_| invalid())?;
        if header.alg != Algorithm::RS256 {
            return Err(invalid());
        }
        let kid = header.kid.as_deref().ok_or_else(invalid)?;
        let mut cache = self.keys.lock().await;
        if cache
            .as_ref()
            .is_none_or(|(_, until)| *until <= Instant::now())
            && !self.fixture_keys
        {
            // Fixed Google endpoint, bounded timeout, and a shared cache prevent per-login discovery.
            let response = self
                .http
                .get("https://www.googleapis.com/oauth2/v3/certs")
                .send()
                .await
                .map_err(|_| unavailable())?
                .error_for_status()
                .map_err(|_| unavailable())?;
            let ttl = response
                .headers()
                .get("cache-control")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| {
                    v.split(',')
                        .find_map(|part| part.trim().strip_prefix("max-age=")?.parse::<u64>().ok())
                })
                .unwrap_or(300)
                .clamp(30, 3600);
            let bytes = response.bytes().await.map_err(|_| unavailable())?;
            if bytes.len() > 262144 {
                return Err(unavailable());
            }
            let keys = serde_json::from_slice(&bytes).map_err(|_| unavailable())?;
            *cache = Some((keys, Instant::now() + StdDuration::from_secs(ttl)));
        }
        let key = cache
            .as_ref()
            .and_then(|(keys, _)| keys.find(kid))
            .ok_or_else(invalid)?;
        let decoding = DecodingKey::from_jwk(key).map_err(|_| invalid())?;
        drop(cache);
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_audience(&[&self.client_id]);
        validation.set_issuer(&["https://accounts.google.com", "accounts.google.com"]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation.leeway = 0;
        validation.validate_nbf = true;
        let claims = decode::<Identity>(token, &decoding, &validation)
            .map_err(|_| invalid())?
            .claims;
        if !claims.email_verified
            || claims.sub.is_empty()
            || claims.sub.len() > 255
            || claims.email.len() > 254
            || !bool::from(claims.nonce.as_bytes().ct_eq(nonce.as_bytes()))
            || claims
                .azp
                .as_ref()
                .is_some_and(|azp| azp != &self.client_id)
        {
            return Err(invalid());
        }
        Ok(claims)
    }
}
pub fn cookie_name(setup: &Setup) -> &'static str {
    if setup.secure_cookie {
        "__Host-mobile_qa_google"
    } else {
        "mobile_qa_google"
    }
}
pub async fn challenge(
    ctx: &AppContext,
    network: &str,
) -> ApiResult<(GoogleLoginChallenge, String)> {
    let setup = Setup::get(ctx);
    let google = setup.google.as_ref().ok_or_else(unavailable)?;
    auth::rate_limit(ctx, "google-start", network, 100).await?;
    google_login_challenges::Entity::delete_many()
        .filter(google_login_challenges::Column::ExpiresAt.lte(Utc::now()))
        .exec(&ctx.db)
        .await?;
    let cookie = loco_rs::hash::random_string(64);
    let row = google_login_challenges::ActiveModel {
        id: Set(Uuid::new_v4()),
        nonce: Set(loco_rs::hash::random_string(64)),
        cookie_hash: Set(format!("{:x}", Sha256::digest(cookie.as_bytes()))),
        expires_at: Set(Utc::now() + Duration::minutes(10)),
    }
    .insert(&ctx.db)
    .await?;
    Ok((
        GoogleLoginChallenge {
            challenge_id: row.id,
            client_id: google.client_id.clone(),
            nonce: row.nonce,
        },
        cookie,
    ))
}
pub async fn login(
    ctx: &AppContext,
    input: LoginRequest,
    binding: &str,
    network: &str,
) -> ApiResult<(auth::Session, String)> {
    auth::rate_limit(ctx, "google-login", network, 100).await?;
    auth::rate_limit(ctx, "google-challenge", &input.challenge_id.to_string(), 10).await?;
    let setup = Setup::get(ctx);
    let google = setup.google.as_ref().ok_or_else(unavailable)?;
    let tx = ctx.db.begin().await?;
    let challenge = google_login_challenges::Entity::find_by_id(input.challenge_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(invalid)?;
    let digest = format!("{:x}", Sha256::digest(binding.as_bytes()));
    if challenge.expires_at <= Utc::now()
        || !bool::from(digest.as_bytes().ct_eq(challenge.cookie_hash.as_bytes()))
    {
        return Err(invalid());
    }
    google_login_challenges::Entity::delete_by_id(challenge.id)
        .exec(&tx)
        .await?;
    tx.commit().await?; // Consume before verification: no replay, including parallel completion.
    let identity = google.identity(&input.credential, &challenge.nonce).await?;
    let tx = ctx.db.begin().await?;
    let linked = users::Entity::find()
        .filter(users::Column::GoogleSubject.eq(&identity.sub))
        .lock_exclusive()
        .one(&tx)
        .await?;
    let user = if let Some(user) = linked {
        user
    } else {
        let email = identity.email.trim().to_lowercase();
        // Only Google-hosted mail is authoritative enough to claim an existing email invitation.
        if !email.ends_with("@gmail.com") && identity.hd.as_ref().is_none_or(|hd| hd.is_empty()) {
            return Err(ApiFailure::new(
                403,
                "google_account_not_linked",
                "Ask your operator to link this Google account before signing in",
            ));
        }
        let user = users::Entity::find()
            .filter(users::Column::Email.eq(email))
            .filter(users::Column::GoogleSubject.is_null())
            .filter(users::Column::DisabledAt.is_null())
            .lock_exclusive()
            .one(&tx)
            .await?
            .ok_or_else(|| {
                ApiFailure::new(
                    403,
                    "workspace_access_required",
                    "Ask your workspace operator for access",
                )
            })?;
        let mut active: users::ActiveModel = user.into();
        active.google_subject = Set(Some(identity.sub));
        active.update(&tx).await?
    };
    if user.disabled_at.is_some() {
        return Err(invalid());
    }
    // This grant check is independent of Google's identity assertion.
    if memberships::Entity::find()
        .filter(memberships::Column::UserId.eq(user.id))
        .filter(memberships::Column::Active.eq(true))
        .one(&tx)
        .await?
        .is_none()
    {
        return Err(ApiFailure::new(
            403,
            "workspace_access_required",
            "Ask your workspace operator for access",
        ));
    }
    tx.commit().await?;
    auth::issue_session(ctx, user).await
}
