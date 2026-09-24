//! Read projection over canonical runs and creator-private phone trials. No duplicate writes.
use super::{apps, execution_store::decode, runs};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{builds, execution_runs, phone_sessions, phone_tasks},
};
use loco_rs::app::AppContext;
use mobile_qa_contracts::regression::*;
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::HashMap;
use uuid::Uuid;

struct HistoryRecord {
    id: Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    build_id: Uuid,
    source: &'static str,
    session_id: Option<Uuid>,
    payload: Option<serde_json::Value>,
}
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
    let cursor_filter = |time, id| {
        Condition::any()
            .add(execution_runs::Column::CreatedAt.lt(time))
            .add(
                Condition::all()
                    .add(execution_runs::Column::CreatedAt.eq(time))
                    .add(execution_runs::Column::Id.lt(id)),
            )
    };
    let mut records = vec![];
    if matches!(source.as_str(), "all" | "runs") {
        let mut query = execution_runs::Entity::find()
            .filter(execution_runs::Column::AppId.eq(app))
            .order_by_desc(execution_runs::Column::CreatedAt)
            .order_by_desc(execution_runs::Column::Id)
            .limit(26);
        if let (Some(time), Some(id)) = (time, id) {
            query = query.filter(cursor_filter(time, id));
        }
        records.extend(
            query
                .all(&ctx.db)
                .await?
                .into_iter()
                .map(|run| HistoryRecord {
                    id: run.id,
                    created_at: run.created_at,
                    build_id: run.build_id,
                    source: "runs",
                    session_id: None,
                    payload: None,
                }),
        );
    }
    if matches!(source.as_str(), "all" | "trials" | "legacy") {
        let sessions = phone_sessions::Entity::find()
            .filter(phone_sessions::Column::AppId.eq(app))
            .filter(phone_sessions::Column::CreatorId.eq(user))
            .all(&ctx.db)
            .await?;
        let session_map = sessions
            .into_iter()
            .map(|session| (session.id, session.build_id))
            .collect::<HashMap<_, _>>();
        let session_ids = session_map.keys().copied().collect::<Vec<_>>();
        let mut phone_cursor = (time, id);
        while !session_ids.is_empty() && records.iter().filter(|r| r.source != "runs").count() < 26
        {
            let mut query = phone_tasks::Entity::find()
                .filter(phone_tasks::Column::SessionId.is_in(session_ids.clone()))
                .order_by_desc(phone_tasks::Column::CreatedAt)
                .order_by_desc(phone_tasks::Column::Id)
                .limit(100);
            query = match source.as_str() {
                "legacy" => query.filter(phone_tasks::Column::Purpose.is_null()),
                _ => query.filter(phone_tasks::Column::Purpose.eq("trial")),
            };
            if let (Some(cursor_time), Some(cursor_id)) = phone_cursor {
                query = query.filter(
                    Condition::any()
                        .add(phone_tasks::Column::CreatedAt.lt(cursor_time))
                        .add(
                            Condition::all()
                                .add(phone_tasks::Column::CreatedAt.eq(cursor_time))
                                .add(phone_tasks::Column::Id.lt(cursor_id)),
                        ),
                );
            }
            let page = query.all(&ctx.db).await?;
            for task in &page {
                phone_cursor = (Some(task.created_at), Some(task.id));
                if task
                    .payload
                    .get("sequence")
                    .is_none_or(|sequence| sequence.is_null())
                {
                    continue;
                }
                let Some(build_id) = session_map.get(&task.session_id).copied() else {
                    continue;
                };
                records.push(HistoryRecord {
                    id: task.id,
                    created_at: task.created_at,
                    build_id,
                    source: if task.purpose.as_deref() == Some("trial") {
                        "trials"
                    } else {
                        "legacy"
                    },
                    session_id: Some(task.session_id),
                    payload: Some(task.payload.clone()),
                });
                if records.iter().filter(|r| r.source != "runs").count() >= 26 {
                    break;
                }
            }
            if page.len() < 100 {
                break;
            }
        }
    }
    records.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    records.truncate(26);
    let build_ids = records
        .iter()
        .map(|record| record.build_id)
        .collect::<Vec<_>>();
    let build_rows = if build_ids.is_empty() {
        vec![]
    } else {
        builds::Entity::find()
            .filter(builds::Column::Id.is_in(build_ids))
            .all(&ctx.db)
            .await?
    };
    let build_labels = build_rows
        .into_iter()
        .map(|build| {
            let label = build
                .metadata
                .as_ref()
                .and_then(|value| value.get("version_name"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or(&build.original_filename)
                .to_owned();
            (build.id, label)
        })
        .collect::<HashMap<_, _>>();
    let more = records.len() > 25;
    let mut items = vec![];
    for row in records.iter().take(25) {
        let id = row.id;
        let source = row.source.to_owned();
        items.push(RunHistoryItem {
            id: format!("{source}:{id}"),
            source: source.clone(),
            created_at: row.created_at,
            build_label: build_labels.get(&row.build_id).cloned().unwrap_or_default(),
            run: if source == "runs" {
                Some(runs::detail(&ctx.db, id).await?)
            } else {
                None
            },
            trial: row.payload.clone().map(decode).transpose()?,
            session_id: row.session_id,
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
