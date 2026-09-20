//! Restart-safe finalization; browser reads never alter historical verdicts.
use super::{execution_store::*, runs};
use crate::{domain::regression, errors::ApiResult};
use loco_rs::app::AppContext;
use mobile_qa_contracts::execution::JobState;
pub async fn finalize_pending(ctx: &AppContext) -> ApiResult<()> {
    for row in rows(&ctx.db,"SELECT id FROM execution_runs r WHERE comparison IS NULL AND EXISTS (SELECT 1 FROM execution_attempts a WHERE a.run_id=r.id) AND NOT EXISTS (SELECT 1 FROM execution_attempts a WHERE a.run_id=r.id AND a.state <> 'finished') ORDER BY created_at LIMIT 100",vec![]).await? {
        let current = runs::detail(&ctx.db,field(&row,"id")?).await?;
        if current.state != JobState::Finished {continue;}
        let baseline = if let Some(id) = current.baseline_run_id {Some(runs::detail(&ctx.db,id).await?)} else {None};
        let comparison = regression::compare(&current,baseline.as_ref());
        exec(&ctx.db,"UPDATE execution_runs SET comparison=$2 WHERE id=$1 AND comparison IS NULL",vec![current.id.into(),json(&comparison)?.into()]).await?;
    }
    Ok(())
}
