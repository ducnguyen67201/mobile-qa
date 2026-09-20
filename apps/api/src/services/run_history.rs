//! Read projection over canonical runs and creator-private phone trials. No duplicate writes.
use super::{apps, execution_store::*, runs};
use crate::errors::{ApiFailure, ApiResult};
use loco_rs::app::AppContext;
use mobile_qa_contracts::regression::*;
use uuid::Uuid;
pub async fn list(
    ctx: &AppContext,
    user: Uuid,
    app: Uuid,
    source: Option<String>,
    cursor: Option<String>,
) -> ApiResult<RunHistory> {
    apps::authorized(ctx, user, app).await?;
    let source = source.unwrap_or_else(|| "all".into());
    if !["all", "runs", "trials", "legacy"].contains(&source.as_str()) {
        return Err(ApiFailure::invalid("Unknown history filter"));
    }
    let (time, id) = if let Some(c) = cursor {
        let (t, i) = c
            .split_once('|')
            .ok_or_else(|| ApiFailure::invalid("Invalid history cursor"))?;
        (
            Some(
                chrono::DateTime::parse_from_rfc3339(t)
                    .map_err(|_| ApiFailure::invalid("Invalid history cursor"))?
                    .with_timezone(&chrono::Utc),
            ),
            Some(Uuid::parse_str(i).map_err(|_| ApiFailure::invalid("Invalid history cursor"))?),
        )
    } else {
        (None, None)
    };
    let records=rows(&ctx.db,r#"WITH history AS (
      SELECT r.id,r.created_at,r.build_id,'runs'::text AS source,NULL::uuid AS session_id,NULL::jsonb AS payload FROM execution_runs r WHERE r.app_id=$1
      UNION ALL
      SELECT t.id,t.created_at,s.build_id,CASE WHEN t.purpose='trial' THEN 'trials' ELSE 'legacy' END,s.id,t.payload FROM phone_tasks t JOIN phone_sessions s ON s.id=t.session_id WHERE s.app_id=$1 AND s.creator_id=$2 AND (t.purpose='trial' OR t.purpose IS NULL) AND t.payload->'sequence' IS NOT NULL AND t.payload->'sequence' <> 'null'::jsonb
    ) SELECT h.*,COALESCE(b.metadata->>'version_name',b.original_filename) AS label FROM history h JOIN builds b ON b.id=h.build_id WHERE (($3='all' AND h.source IN ('runs','trials')) OR h.source=$3) AND ($4::timestamptz IS NULL OR (h.created_at,h.id)<($4,$5::uuid)) ORDER BY h.created_at DESC,h.id DESC LIMIT 26"#,vec![app.into(),user.into(),source.into(),time.into(),id.into()]).await?;
    let more = records.len() > 25;
    let mut items = vec![];
    for row in records.iter().take(25) {
        let id: Uuid = field(row, "id")?;
        let source: String = field(row, "source")?;
        items.push(RunHistoryItem {
            id: format!("{source}:{id}"),
            source: source.clone(),
            created_at: field(row, "created_at")?,
            build_label: field(row, "label")?,
            run: if source == "runs" {
                Some(runs::detail(&ctx.db, id).await?)
            } else {
                None
            },
            trial: field::<Option<serde_json::Value>>(row, "payload")?
                .map(decode)
                .transpose()?,
            session_id: field(row, "session_id")?,
        });
    }
    let next_cursor = if more {
        items.last().map(|i| {
            format!(
                "{}|{}",
                i.created_at.to_rfc3339(),
                i.id.split(':').next_back().unwrap_or_default()
            )
        })
    } else {
        None
    };
    Ok(RunHistory { items, next_cursor })
}
