//! Recent completed runs for explicit, immutable baseline selection.
use super::runs;
use crate::{
    errors::ApiResult,
    models::_entities::{builds, execution_attempts, execution_runs},
};
use mobile_qa_contracts::{execution::RunResponse, regression::BaselineChoice};
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use std::collections::HashMap;
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
        let mut query = execution_runs::Entity::find()
            .filter(execution_runs::Column::AppId.eq(app))
            .order_by_desc(execution_runs::Column::CreatedAt)
            .order_by_desc(execution_runs::Column::Id)
            .limit(100);
        if let (Some(time), Some(id)) = (cursor_time, cursor_id) {
            query = query.filter(
                Condition::any()
                    .add(execution_runs::Column::CreatedAt.lt(time))
                    .add(
                        Condition::all()
                            .add(execution_runs::Column::CreatedAt.eq(time))
                            .add(execution_runs::Column::Id.lt(id)),
                    ),
            );
        }
        let page = query.all(db).await?;
        let run_ids = page.iter().map(|run| run.id).collect::<Vec<_>>();
        let mut attempts: HashMap<Uuid, Vec<String>> = HashMap::new();
        if !run_ids.is_empty() {
            for attempt in execution_attempts::Entity::find()
                .filter(execution_attempts::Column::RunId.is_in(run_ids))
                .all(db)
                .await?
            {
                attempts
                    .entry(attempt.run_id)
                    .or_default()
                    .push(attempt.state);
            }
        }
        let build_ids = page.iter().map(|run| run.build_id).collect::<Vec<_>>();
        let build_rows = if build_ids.is_empty() {
            vec![]
        } else {
            builds::Entity::find()
                .filter(builds::Column::Id.is_in(build_ids))
                .all(db)
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
        for row in &page {
            cursor_time = Some(row.created_at);
            cursor_id = Some(row.id);
            let Some(states) = attempts.get(&row.id) else {
                continue;
            };
            if states.iter().any(|state| state != "finished") {
                continue;
            }
            let run = runs::detail(db, row.id).await?;
            let compatible = eligible(&run);
            if baselines.len() < 100 || compatible {
                baselines.push(BaselineChoice {
                    id: run.id,
                    build_id: run.manifest.build_id,
                    build_label: build_labels.get(&row.build_id).cloned().unwrap_or_default(),
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
