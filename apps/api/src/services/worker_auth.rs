//! Worker bearer identity is independent of browser cookies and CSRF.
use super::{execution_store::hash, test_definitions};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{device_hosts, device_slots, execution_workers},
};
use axum::{
    extract::{FromRef, FromRequestParts},
    http::request::Parts,
};
use loco_rs::app::AppContext;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, Set,
};
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
        let row = execution_workers::Entity::find()
            .filter(execution_workers::Column::TokenHash.eq(hash(raw)))
            .filter(execution_workers::Column::Revoked.eq(false))
            .one(&ctx.db)
            .await?
            .ok_or_else(ApiFailure::unauthorized)?;
        require_live_host_grant(&ctx.db, &row).await?;
        Ok(Self {
            id: row.id,
            app_id: row.app_id,
            profile_id: row.profile_id,
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
    execution_workers::ActiveModel {
        id: Set(id),
        app_id: Set(app),
        profile_id: Set(profile),
        token_hash: Set(hash(token)),
        revoked: Set(false),
        model_capabilities: Set(None),
        model_last_seen_at: Set(None),
        execution_protocol_version: Set(None),
        execution_model_capabilities: Set(None),
        execution_last_seen_at: Set(None),
        phone_protocol_version: Set(None),
        phone_model_capabilities: Set(None),
        phone_last_seen_at: Set(None),
        host_slot_id: Set(None),
        host_generation: Set(None),
    }
    .insert(&ctx.db)
    .await?;
    Ok(())
}

async fn require_live_host_grant(
    db: &impl ConnectionTrait,
    worker: &execution_workers::Model,
) -> ApiResult<()> {
    let Some(slot_id) = worker.host_slot_id else {
        return Ok(());
    };
    let slot = device_slots::Entity::find_by_id(slot_id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::unauthorized)?;
    let host = device_hosts::Entity::find_by_id(slot.host_id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::unauthorized)?;
    if host.revoked || worker.host_generation != Some(host.generation) {
        return Err(ApiFailure::unauthorized());
    }
    Ok(())
}

/// Pin bearer authority for a lease transaction. Boot registration and grant rotation
/// update the worker row, so neither can acknowledge revocation before this work commits.
/// A shared worker-row lock avoids introducing a host↔run lock ordering dependency.
pub async fn lease_authority(db: &impl ConnectionTrait, worker: &Worker) -> ApiResult<()> {
    let row = execution_workers::Entity::find_by_id(worker.id)
        .filter(execution_workers::Column::AppId.eq(worker.app_id))
        .filter(execution_workers::Column::ProfileId.eq(worker.profile_id))
        .filter(execution_workers::Column::Revoked.eq(false))
        .lock_shared()
        .one(db)
        .await?
        .ok_or_else(ApiFailure::unauthorized)?;
    require_live_host_grant(db, &row).await
}
