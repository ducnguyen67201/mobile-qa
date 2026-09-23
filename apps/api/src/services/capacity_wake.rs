//! Best-effort wake latency backed by a leased outbox and the controller's scheduled demand scan.
use super::execution_store::hash;
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::capacity_wake_outbox,
};
use hmac::{Hmac, Mac};
use loco_rs::app::AppContext;
use mobile_qa_contracts::device_hosts::CapacityHint;
use sea_orm::{
    sea_query::{LockBehavior, LockType},
    ActiveModelTrait,
    ActiveValue::Set,
    ColumnTrait, Condition, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, QuerySelect,
    TransactionTrait,
};
use sha2::Sha256;
use uuid::Uuid;

#[derive(Clone)]
struct Dispatcher {
    url: String,
    key: String,
    client: reqwest::Client,
}
impl Dispatcher {
    fn from_env() -> ApiResult<Option<Self>> {
        let (url, key) = match (
            std::env::var("MOBILE_QA_CAPACITY_WAKE_URL"),
            std::env::var("MOBILE_QA_CAPACITY_WAKE_SECRET"),
        ) {
            (Ok(url), Ok(key)) => (url, key),
            (Err(std::env::VarError::NotPresent), Err(std::env::VarError::NotPresent)) => {
                return Ok(None)
            }
            _ => {
                return Err(ApiFailure::invalid(
                    "Configure both capacity wake endpoint and secret",
                ))
            }
        };
        let parsed = url::Url::parse(&url)
            .map_err(|_| ApiFailure::invalid("Invalid capacity wake endpoint"))?;
        if parsed.scheme() != "https"
            || parsed.path() != "/wake"
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
            || key.len() < 32
        {
            return Err(ApiFailure::invalid(
                "Capacity wake requires HTTPS /wake and a scoped secret",
            ));
        }
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|_| ApiFailure::internal())?;
        Ok(Some(Self { url, key, client }))
    }
    async fn dispatch(&self, ctx: &AppContext) -> ApiResult<()> {
        let lease = Uuid::new_v4();
        let now = chrono::Utc::now();
        let tx = ctx.db.begin().await?;
        let pending = capacity_wake_outbox::Entity::find()
            .filter(capacity_wake_outbox::Column::DeliveredAt.is_null())
            .filter(capacity_wake_outbox::Column::NextAttemptAt.lte(now))
            .filter(
                Condition::any()
                    .add(capacity_wake_outbox::Column::LeaseUntil.is_null())
                    .add(capacity_wake_outbox::Column::LeaseUntil.lt(now)),
            )
            .filter(capacity_wake_outbox::Column::Attempts.lt(12))
            .order_by_asc(capacity_wake_outbox::Column::CreatedAt)
            .limit(4)
            .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked)
            .all(&tx)
            .await?;
        let mut claimed = Vec::with_capacity(pending.len());
        for row in pending {
            let id = row.id;
            let pool = row.pool_id;
            let mut row = row.into_active_model();
            row.lease_id = Set(Some(lease));
            row.lease_until = Set(Some(now + chrono::Duration::seconds(60)));
            row.attempts = Set(row.attempts.take().unwrap_or_default() + 1);
            row.update(&tx).await?;
            claimed.push((id, pool));
        }
        tx.commit().await?;
        for (id, pool) in claimed {
            let body = serde_json::to_vec(&CapacityHint {
                pool_id: pool,
                request_id: id,
            })
            .map_err(|_| ApiFailure::internal())?;
            let timestamp = chrono::Utc::now().timestamp().to_string();
            let signature = sign(&self.key, &body, &timestamp, id)?;
            let accepted = self
                .client
                .post(&self.url)
                .header("content-type", "application/json")
                .header("x-mobile-qa-timestamp", timestamp)
                .header("x-mobile-qa-request-id", id.to_string())
                .header("x-mobile-qa-signature", signature)
                .body(body)
                .send()
                .await
                .is_ok_and(|r| {
                    r.status().is_success() || r.status() == reqwest::StatusCode::CONFLICT
                });
            if let Some(row) = capacity_wake_outbox::Entity::find_by_id(id)
                .filter(capacity_wake_outbox::Column::LeaseId.eq(lease))
                .one(&ctx.db)
                .await?
            {
                let completed_at = chrono::Utc::now();
                let mut row = row.into_active_model();
                row.delivered_at = Set(accepted.then_some(completed_at));
                row.lease_until = Set(None);
                row.lease_id = Set(None);
                row.next_attempt_at = Set(completed_at + chrono::Duration::seconds(30));
                row.update(&ctx.db).await?;
            }
        }
        Ok(())
    }
}
fn sign(key: &str, body: &[u8], timestamp: &str, id: Uuid) -> ApiResult<String> {
    let canonical = format!("POST\n/wake\n{}\n{timestamp}\n{id}", hash(body));
    let mut mac =
        Hmac::<Sha256>::new_from_slice(key.as_bytes()).map_err(|_| ApiFailure::internal())?;
    mac.update(canonical.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}
/// One process-local poller; PostgreSQL claims prevent duplicate dispatch across API replicas.
pub fn start(ctx: &AppContext) -> ApiResult<()> {
    let Some(dispatcher) = Dispatcher::from_env()? else {
        return Ok(());
    };
    let ctx = ctx.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            if dispatcher.dispatch(&ctx).await.is_err() {
                tracing::warn!(
                    reason_code = "capacity_wake_retry",
                    "Capacity wake dispatch will retry"
                );
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signature_binds_every_request_field() {
        let id = Uuid::nil();
        let signature = sign("12345678901234567890123456789012", b"{}", "123", id).unwrap();
        assert_eq!(signature.len(), 64);
        assert_ne!(
            signature,
            sign("12345678901234567890123456789012", b"{}", "124", id).unwrap()
        );
        assert_ne!(
            signature,
            sign(
                "12345678901234567890123456789012",
                b"{\"pool\":1}",
                "123",
                id
            )
            .unwrap()
        );
    }
}
