//! Restart-safe finalization; browser reads never alter historical verdicts.
use super::{execution_store::json, runs};
use crate::{
    domain::regression,
    errors::ApiResult,
    models::_entities::{execution_attempts, execution_runs},
};
use loco_rs::app::AppContext;
use mobile_qa_contracts::execution::JobState;
use sea_orm::{sea_query::Expr, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::HashMap;
pub async fn finalize_pending(ctx: &AppContext) -> ApiResult<()> {
    let candidates = execution_runs::Entity::find()
        .filter(execution_runs::Column::Comparison.is_null())
        .order_by_asc(execution_runs::Column::CreatedAt)
        .limit(100)
        .all(&ctx.db)
        .await?;
    let ids = candidates.iter().map(|run| run.id).collect::<Vec<_>>();
    let mut attempts_by_run: HashMap<_, Vec<_>> = HashMap::new();
    if !ids.is_empty() {
        for attempt in execution_attempts::Entity::find()
            .filter(execution_attempts::Column::RunId.is_in(ids))
            .all(&ctx.db)
            .await?
        {
            attempts_by_run
                .entry(attempt.run_id)
                .or_default()
                .push(attempt);
        }
    }
    for run in candidates {
        let Some(attempts) = attempts_by_run.get(&run.id) else {
            continue;
        };
        if attempts.iter().any(|attempt| attempt.state != "finished") {
            continue;
        }
        let current = runs::detail(&ctx.db, run.id).await?;
        if current.state != JobState::Finished {
            continue;
        }
        let baseline = if let Some(id) = current.baseline_run_id {
            Some(runs::detail(&ctx.db, id).await?)
        } else {
            None
        };
        let comparison = regression::compare(&current, baseline.as_ref());
        execution_runs::Entity::update_many()
            .col_expr(
                execution_runs::Column::Comparison,
                Expr::value(json(&comparison)?),
            )
            .filter(execution_runs::Column::Id.eq(current.id))
            .filter(execution_runs::Column::Comparison.is_null())
            .exec(&ctx.db)
            .await?;
    }
    Ok(())
}
