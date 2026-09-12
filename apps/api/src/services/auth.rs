//! Framework password/JWT authentication plus durable revocation and tenant grants.
use crate::{
    config::{Setup, SESSION_SECONDS},
    errors::{ApiFailure, ApiResult},
    models::_entities::{login_attempts, memberships, organizations, sessions, users},
};
use axum::{
    extract::{ConnectInfo, FromRef, FromRequestParts},
    http::{request::Parts, HeaderMap, Method},
};
use chrono::{Duration, Utc};
use loco_rs::app::AppContext;
use mobile_qa_contracts::browser::*;
use sea_orm::{
    sea_query::OnConflict, ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QuerySelect,
    Set, TransactionTrait,
};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use uuid::Uuid;

#[derive(Clone)]
pub struct Session {
    pub user: users::Model,
    pub session: sessions::Model,
}
impl<S> FromRequestParts<S> for Session
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiFailure;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> ApiResult<Self> {
        let ctx = AppContext::from_ref(state);
        let auth = authenticate(&ctx, parts).await?;
        if !matches!(parts.method, Method::GET | Method::HEAD | Method::OPTIONS) {
            origin(&Setup::get(&ctx), &parts.headers)?;
            let token = parts
                .headers
                .get("x-csrf-token")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            if !bool::from(token.as_bytes().ct_eq(auth.session.csrf_token.as_bytes())) {
                return Err(ApiFailure::new(
                    403,
                    "csrf_rejected",
                    "Refresh the page and try again",
                ));
            }
        }
        Ok(auth)
    }
}
pub async fn authenticate(ctx: &AppContext, parts: &Parts) -> ApiResult<Session> {
    let jwt = loco_rs::controller::extractor::auth::extract_jwt_from_request_parts(parts, ctx)
        .map_err(|_| ApiFailure::unauthorized())?;
    let user_id = Uuid::parse_str(&jwt.claims.pid).map_err(|_| ApiFailure::unauthorized())?;
    let sid = jwt
        .claims
        .claims
        .get("sid")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .ok_or_else(ApiFailure::unauthorized)?;
    let session = sessions::Entity::find_by_id(sid)
        .filter(sessions::Column::UserId.eq(user_id))
        .filter(sessions::Column::RevokedAt.is_null())
        .filter(sessions::Column::ExpiresAt.gt(Utc::now()))
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::unauthorized)?;
    let user = users::Entity::find_by_id(user_id)
        .filter(users::Column::DisabledAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::unauthorized)?;
    Ok(Session { user, session })
}
pub fn origin(setup: &Setup, headers: &HeaderMap) -> ApiResult<()> {
    let source = if let Some(value) = headers.get("origin") {
        value.to_str().ok().map(str::to_owned)
    } else {
        headers
            .get("referer")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| url::Url::parse(v).ok())
            .map(|v| v.origin().ascii_serialization())
    };
    if source.as_deref() != Some(&setup.origin) {
        return Err(ApiFailure::new(
            403,
            "origin_rejected",
            "This request origin is not permitted",
        ));
    }
    Ok(())
}
pub struct LoginGuard {
    pub network: String,
}
impl<S> FromRequestParts<S> for LoginGuard
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiFailure;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> ApiResult<Self> {
        origin(&Setup::get(&AppContext::from_ref(state)), &parts.headers)?;
        if parts
            .headers
            .get("x-mobile-qa-request")
            .and_then(|v| v.to_str().ok())
            != Some("1")
        {
            return Err(ApiFailure::new(
                403,
                "csrf_rejected",
                "Use the sign-in form",
            ));
        }
        // Forwarded headers are intentionally untrusted. Unknown peer shares a global bucket.
        let network = parts
            .extensions
            .get::<ConnectInfo<std::net::SocketAddr>>()
            .map(|v| v.0.ip().to_string())
            .unwrap_or_else(|| "unknown-peer".into());
        Ok(Self { network })
    }
}
pub async fn organization_memberships(
    ctx: &AppContext,
    user_id: Uuid,
) -> ApiResult<Vec<OrganizationMembership>> {
    let mut result = Vec::new();
    for membership in memberships::Entity::find()
        .filter(memberships::Column::UserId.eq(user_id))
        .filter(memberships::Column::Active.eq(true))
        .all(&ctx.db)
        .await?
    {
        if let Some(org) = organizations::Entity::find_by_id(membership.organization_id)
            .one(&ctx.db)
            .await?
        {
            result.push(OrganizationMembership {
                organization_id: org.id,
                name: org.name,
                role: if membership.role == "operator" {
                    MembershipRole::Operator
                } else {
                    MembershipRole::Member
                },
            });
        }
    }
    Ok(result)
}
pub async fn response(ctx: &AppContext, session: &Session) -> ApiResult<SessionResponse> {
    Ok(SessionResponse {
        user: UserIdentity {
            id: session.user.id,
            email: session.user.email.clone(),
            display_name: session.user.display_name.clone(),
        },
        memberships: organization_memberships(ctx, session.user.id).await?,
        csrf_token: session.session.csrf_token.clone(),
        expires_at: session.session.expires_at,
    })
}
async fn rate_limit(ctx: &AppContext, kind: &str, value: &str, limit: i32) -> ApiResult<()> {
    let key = format!("{kind}:{:x}", Sha256::digest(value.as_bytes()));
    let now = Utc::now();
    let tx = ctx.db.begin().await?;
    // One row per normalized bucket and bounded lifetime; upsert prevents concurrent first-login races.
    login_attempts::Entity::insert(login_attempts::ActiveModel {
        id: Set(Uuid::new_v4()),
        bucket: Set(key.clone()),
        window_start: Set(now),
        count: Set(0),
        expires_at: Set(now + Duration::minutes(15)),
    })
    .on_conflict(
        OnConflict::column(login_attempts::Column::Bucket)
            .do_nothing()
            .to_owned(),
    )
    .try_insert()
    .exec(&tx)
    .await?;
    let row = login_attempts::Entity::find()
        .filter(login_attempts::Column::Bucket.eq(key))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::internal)?;
    let count = if row.expires_at <= now { 0 } else { row.count };
    if count >= limit {
        return Err(ApiFailure::new(
            429,
            "login_rate_limited",
            "Too many sign-in attempts. Try again later",
        ));
    }
    let reset = row.expires_at <= now;
    let mut active: login_attempts::ActiveModel = row.into();
    active.count = Set(count + 1);
    if reset {
        active.window_start = Set(now);
        active.expires_at = Set(now + Duration::minutes(15));
    }
    active.update(&tx).await?;
    tx.commit().await?;
    Ok(())
}
pub async fn login(
    ctx: &AppContext,
    input: LoginRequest,
    network: &str,
) -> ApiResult<(Session, String)> {
    let email = input.email.trim().to_lowercase();
    if email.len() > 254 || input.password.len() > 1024 {
        return Err(ApiFailure::new(
            401,
            "invalid_credentials",
            "Email or password is incorrect",
        ));
    }
    // Network bucket first also bounds creation of attacker-controlled email buckets.
    rate_limit(ctx, "network", network, 100).await?;
    rate_limit(ctx, "email", &email, 10).await?;
    let setup = Setup::get(ctx);
    let permit =
        setup.hashing.clone().try_acquire_owned().map_err(|_| {
            ApiFailure::new(429, "login_busy", "Sign-in is busy. Try again shortly")
        })?;
    let user = users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(&ctx.db)
        .await?;
    let hash = user
        .as_ref()
        .map(|u| u.password_hash.clone())
        .unwrap_or(setup.dummy_password_hash);
    let valid = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        loco_rs::hash::verify_password(&input.password, &hash)
    })
    .await
    .map_err(|_| ApiFailure::internal())?;
    let user = user
        .filter(|u| valid && u.disabled_at.is_none())
        .ok_or_else(|| {
            ApiFailure::new(401, "invalid_credentials", "Email or password is incorrect")
        })?;
    let now = Utc::now();
    let session = sessions::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user.id),
        csrf_token: Set(loco_rs::hash::random_string(64)),
        expires_at: Set(now + Duration::seconds(SESSION_SECONDS)),
        revoked_at: Set(None),
        created_at: Set(now),
    }
    .insert(&ctx.db)
    .await?;
    let jwt = ctx
        .config
        .get_jwt_config()
        .map_err(|_| ApiFailure::internal())?;
    let claims = serde_json::json!({"sid":session.id.to_string()})
        .as_object()
        .expect("object")
        .clone();
    let token = loco_rs::auth::jwt::JWT::new(&jwt.secret)
        .generate_token(SESSION_SECONDS as u64, user.id.to_string(), claims)
        .map_err(|_| ApiFailure::internal())?;
    Ok((Session { user, session }, token))
}
pub async fn revoke(ctx: &AppContext, id: Uuid) -> ApiResult<()> {
    sessions::Entity::update_many()
        .col_expr(
            sessions::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(Utc::now()),
        )
        .filter(sessions::Column::Id.eq(id))
        .exec(&ctx.db)
        .await?;
    Ok(())
}
/// Explicit operator provisioning, also used by isolated test fixtures. Never runs at startup.
pub async fn provision(
    ctx: &AppContext,
    email: &str,
    password: &str,
    name: &str,
    organization: &str,
) -> ApiResult<(Uuid, Uuid)> {
    if password.len() < 12 || password.len() > 1024 {
        return Err(ApiFailure::invalid("Password must contain 12–1024 bytes"));
    }
    let email = email.trim().to_lowercase();
    if !email.contains('@')
        || email.len() > 254
        || name.trim().is_empty()
        || organization.trim().is_empty()
    {
        return Err(ApiFailure::invalid("Provide email, name and organization"));
    }
    if users::Entity::find()
        .filter(users::Column::Email.eq(&email))
        .one(&ctx.db)
        .await?
        .is_some()
    {
        return Err(ApiFailure::new(
            409,
            "account_exists",
            "Account exists; use an explicit reset operation",
        ));
    }
    let password = password.to_owned();
    let hash = tokio::task::spawn_blocking(move || loco_rs::hash::hash_password(&password))
        .await
        .map_err(|_| ApiFailure::internal())?
        .map_err(|_| ApiFailure::internal())?;
    let tx = ctx.db.begin().await?;
    let user_id = Uuid::new_v4();
    let org_id = Uuid::new_v4();
    let now = Utc::now();
    users::ActiveModel {
        id: Set(user_id),
        email: Set(email),
        password_hash: Set(hash),
        display_name: Set(name.trim().into()),
        disabled_at: Set(None),
        created_at: Set(now),
    }
    .insert(&tx)
    .await?;
    organizations::ActiveModel {
        id: Set(org_id),
        name: Set(organization.trim().into()),
        created_at: Set(now),
    }
    .insert(&tx)
    .await?;
    memberships::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user_id),
        organization_id: Set(org_id),
        role: Set("operator".into()),
        active: Set(true),
    }
    .insert(&tx)
    .await?;
    tx.commit().await?;
    Ok((user_id, org_id))
}

/// Long transfers recheck durable revocation/expiry immediately before accepting bytes.
pub async fn ensure_live(ctx: &AppContext, session: &Session) -> ApiResult<()> {
    sessions::Entity::find_by_id(session.session.id)
        .filter(sessions::Column::UserId.eq(session.user.id))
        .filter(sessions::Column::RevokedAt.is_null())
        .filter(sessions::Column::ExpiresAt.gt(Utc::now()))
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::unauthorized)?;
    users::Entity::find_by_id(session.user.id)
        .filter(users::Column::DisabledAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::unauthorized)?;
    Ok(())
}
