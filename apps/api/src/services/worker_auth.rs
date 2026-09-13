//! Worker bearer identity is independent of browser cookies and CSRF.
use super::{execution_store::*, test_definitions};
use crate::errors::{ApiFailure, ApiResult};
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use loco_rs::app::AppContext;
use uuid::Uuid;
#[derive(Clone)]
pub struct Worker {
    pub id: Uuid,
    pub app_id: Uuid,
    pub profile_id: Uuid,
}
impl<S> FromRequestParts<S> for Worker
where
    AppContext: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = ApiFailure;
    async fn from_request_parts(parts: &mut Parts, state: &S) -> ApiResult<Self> {
        let ctx = AppContext::from_ref(state);
        let raw = parts
            .headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .filter(|s| (32..=256).contains(&s.len()))
            .ok_or_else(ApiFailure::unauthorized)?;
        let r = one(
            &ctx.db,
            "SELECT * FROM execution_workers WHERE token_hash=$1 AND revoked=false",
            vec![hash(raw).into()],
        )
        .await
        .map_err(|_| ApiFailure::unauthorized())?;
        Ok(Self {
            id: field(&r, "id")?,
            app_id: field(&r, "app_id")?,
            profile_id: field(&r, "profile_id")?,
        })
    }
}

pub async fn register(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    id: Uuid,
    profile: Uuid,
    token: &str,
) -> ApiResult<()> {
    test_definitions::operator(ctx, actor, app).await?;
    test_definitions::profile(&ctx.db, app, profile).await?;
    if !(32..=256).contains(&token.len()) {
        return Err(ApiFailure::invalid(
            "Inject a worker token with at least 32 bytes",
        ));
    }
    exec(
        &ctx.db,
        "INSERT INTO execution_workers(id,app_id,profile_id,token_hash) VALUES($1,$2,$3,$4)",
        vec![id.into(), app.into(), profile.into(), hash(token).into()],
    )
    .await?;
    Ok(())
}
