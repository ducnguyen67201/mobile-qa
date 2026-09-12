//! Dry-run by default. Accepted builds and live leases are never retention candidates.
use crate::{
    config::Setup,
    errors::ApiResult,
    models::_entities::{build_uploads, builds, google_login_challenges, login_attempts, sessions},
};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use loco_rs::{
    app::AppContext,
    task::{Task, TaskInfo, Vars},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect, TransactionTrait};
pub struct Cleanup;
pub async fn execute(ctx: &AppContext, apply: bool) -> ApiResult<usize> {
    let setup = Setup::get(ctx);
    let cutoff = Utc::now() - Duration::hours(1);
    let mut candidates = 0;
    if apply {
        google_login_challenges::Entity::delete_many()
            .filter(google_login_challenges::Column::ExpiresAt.lt(Utc::now()))
            .exec(&ctx.db)
            .await?;
    }
    // Inventory is scoped to each expired upload prefix, never the entire bucket. The row
    // lock serializes against completion; a full safety hour outlives transfer/validator leases.
    let rows = build_uploads::Entity::find()
        .filter(build_uploads::Column::ExpiresAt.lt(cutoff))
        .all(&ctx.db)
        .await?;
    for row in rows {
        let tx = ctx.db.begin().await?;
        let Some(row) = build_uploads::Entity::find_by_id(row.id)
            .lock_exclusive()
            .one(&tx)
            .await?
        else {
            continue;
        };
        if row.lease_until.is_some_and(|t| t > cutoff) {
            continue;
        }
        let accepted = builds::Entity::find()
            .filter(builds::Column::UploadId.eq(row.id))
            .one(&tx)
            .await?;
        let prefix = format!("{}/{}/{}", row.organization_id, row.app_id, row.id);
        let keys = setup
            .store
            .list_upload(&prefix, &row.storage_backend)
            .await?;
        for (key, modified) in keys {
            if modified.is_none_or(|t| t >= cutoff)
                || accepted.as_ref().is_some_and(|b| b.storage_key == key)
            {
                continue;
            }
            candidates += 1;
            if apply {
                setup.store.delete(&key, &row.storage_backend).await?;
            }
        }
        if apply && accepted.is_none() {
            build_uploads::Entity::update_many()
                .col_expr(
                    build_uploads::Column::State,
                    sea_orm::sea_query::Expr::value("expired"),
                )
                .filter(build_uploads::Column::Id.eq(row.id))
                .exec(&tx)
                .await?;
        }
        tx.commit().await?;
    }
    candidates += setup.store.cleanup_scratch(cutoff, apply)?;
    if apply {
        login_attempts::Entity::delete_many()
            .filter(login_attempts::Column::ExpiresAt.lt(cutoff))
            .exec(&ctx.db)
            .await?;
        sessions::Entity::delete_many()
            .filter(sessions::Column::ExpiresAt.lt(cutoff))
            .exec(&ctx.db)
            .await?;
    }
    Ok(candidates)
}
#[async_trait]
impl Task for Cleanup {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "artifact-cleanup".into(),
            detail: "List expired upload/scratch candidates; apply:true deletes eligible artifacts"
                .into(),
        }
    }
    async fn run(&self, ctx: &AppContext, vars: &Vars) -> loco_rs::Result<()> {
        let apply = vars.cli.get("apply").is_some_and(|v| v == "true");
        let count = execute(ctx, apply)
            .await
            .map_err(|e| loco_rs::Error::string(&e.message))?;
        println!("eligible_artifacts={count} applied={apply}");
        Ok(())
    }
}
