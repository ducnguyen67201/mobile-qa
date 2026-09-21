//! Recent completed runs for explicit, immutable baseline selection.
use super::{execution_store::*, runs};
use crate::errors::ApiResult;
use mobile_qa_contracts::{execution::RunResponse, regression::BaselineChoice};
use sea_orm::ConnectionTrait;
use uuid::Uuid;

pub async fn choices(
    db: &impl ConnectionTrait,
    app: Uuid,
    eligible: impl Fn(&RunResponse) -> bool,
    matching_reason: &str,
) -> ApiResult<(Vec<BaselineChoice>, Option<Uuid>)> {
    let mut baselines = vec![];
    // A page of unrelated runs must not hide an older, compatible baseline.
    let mut cursor_time: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut cursor_id: Option<Uuid> = None;
    loop {
        let page = rows(db,
            "SELECT r.id,r.created_at,COALESCE(b.metadata->>'version_name',b.original_filename) AS label \
             FROM execution_runs r JOIN builds b ON b.id=r.build_id WHERE r.app_id=$1 \
             AND EXISTS (SELECT 1 FROM execution_attempts a WHERE a.run_id=r.id) \
             AND NOT EXISTS (SELECT 1 FROM execution_attempts a WHERE a.run_id=r.id AND a.state <> 'finished') \
             AND ($2::timestamptz IS NULL OR (r.created_at,r.id)<($2,$3::uuid)) \
             ORDER BY r.created_at DESC,r.id DESC LIMIT 100",
            vec![app.into(), cursor_time.into(), cursor_id.into()]).await?;
        for row in &page {
            let run = runs::detail(db, field(row, "id")?).await?;
            let compatible = eligible(&run);
            cursor_time = Some(run.created_at);
            cursor_id = Some(run.id);
            if baselines.len() < 100 || compatible {
                baselines.push(BaselineChoice {
                    id: run.id,
                    build_id: run.manifest.build_id,
                    build_label: field(row, "label")?,
                    created_at: run.created_at,
                    compatible,
                    reason: if compatible {
                        matching_reason
                    } else {
                        "Different test/context or no conclusive, clean result"
                    }
                    .into(),
                });
            }
        }
        if page.len() < 100 || baselines.iter().any(|b| b.compatible) {
            break;
        }
    }
    let suggested = baselines.iter().find(|b| b.compatible).map(|b| b.id);
    Ok((baselines, suggested))
}
