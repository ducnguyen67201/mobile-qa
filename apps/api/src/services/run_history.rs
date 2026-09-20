//! Source-aware history is a read projection; execution and phone records stay authoritative.
use super::{execution_store::*, runs};
use crate::errors::{ApiFailure, ApiResult};
use chrono::{DateTime, SecondsFormat, Utc};
use mobile_qa_contracts::{execution::*, task_sessions::PhoneTask};
use sea_orm::ConnectionTrait;
use uuid::Uuid;

struct Cursor {
    created_at: DateTime<Utc>,
    kind: String,
    id: Uuid,
}

fn parse_cursor(value: Option<&str>) -> ApiResult<Option<Cursor>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let mut parts = value.split('|');
    let created_at = parts
        .next()
        .and_then(|part| DateTime::parse_from_rfc3339(part).ok())
        .map(|value| value.with_timezone(&Utc));
    let kind = parts.next().map(str::to_owned);
    let id = parts.next().and_then(|part| Uuid::parse_str(part).ok());
    if parts.next().is_some()
        || created_at.is_none()
        || id.is_none()
        || !matches!(
            kind.as_deref(),
            Some("release_run" | "test_run" | "trial" | "legacy")
        )
    {
        return Err(ApiFailure::invalid("Invalid history cursor"));
    }
    Ok(Some(Cursor {
        created_at: created_at.expect("checked"),
        kind: kind.expect("checked"),
        id: id.expect("checked"),
    }))
}

fn cursor_for(row: &sea_orm::QueryResult) -> ApiResult<String> {
    Ok(format!(
        "{}|{}|{}",
        field::<DateTime<Utc>>(row, "created_at")?.to_rfc3339_opts(SecondsFormat::Nanos, true),
        field::<String>(row, "kind")?,
        field::<Uuid>(row, "id")?
    ))
}

pub async fn list(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
    query: RunHistoryQuery,
) -> ApiResult<RunHistoryResponse> {
    let filter = word(&query.filter.unwrap_or(RunHistoryFilter::All));
    let cursor = parse_cursor(query.cursor.as_deref())?;
    let records = rows(
        db,
        "SELECT created_at,id,kind FROM (
           SELECT r.created_at,r.id,
             CASE WHEN r.source_kind='saved_case' THEN 'test_run' ELSE 'release_run' END AS kind
           FROM execution_runs r
           WHERE r.app_id=$1 AND ($3='all'
             OR ($3='test_runs' AND r.source_kind='saved_case')
             OR ($3='release_runs' AND r.source_kind<>'saved_case'))
           UNION ALL
           SELECT t.created_at,t.id,
             CASE WHEN t.payload ? 'purpose' THEN 'trial' ELSE 'legacy' END AS kind
           FROM phone_tasks t JOIN phone_sessions s ON s.id=t.session_id
           WHERE s.app_id=$1 AND s.creator_id=$2 AND
             (($3 IN ('all','trials') AND t.payload->>'purpose'='trial')
              OR ($3='legacy' AND NOT (t.payload ? 'purpose')))
         ) history
         WHERE ($4::timestamptz IS NULL OR (created_at,kind,id)<($4,$5,$6))
         ORDER BY created_at DESC,kind DESC,id DESC LIMIT 21",
        vec![
            app.into(),
            actor.into(),
            filter.into(),
            cursor.as_ref().map(|value| value.created_at).into(),
            cursor.as_ref().map(|value| value.kind.clone()).into(),
            cursor.as_ref().map(|value| value.id).into(),
        ],
    )
    .await?;
    let mut items = Vec::new();
    for row in records.iter().take(20) {
        let id: Uuid = field(row, "id")?;
        let created_at: DateTime<Utc> = field(row, "created_at")?;
        let kind: String = field(row, "kind")?;
        match kind.as_str() {
            "test_run" | "release_run" => items.push(RunHistoryItem::Execution {
                stable_id: format!("run:{id}"),
                created_at,
                run: Box::new(runs::detail(db, id).await?),
            }),
            "trial" | "legacy" => {
                let task: PhoneTask = decode(field(
                    &one(
                        db,
                        "SELECT payload FROM phone_tasks WHERE id=$1",
                        vec![id.into()],
                    )
                    .await?,
                    "payload",
                )?)?;
                let values = (
                    format!("phone:{id}"),
                    created_at,
                    id,
                    task.goal,
                    word(&task.state),
                    task.message,
                );
                items.push(if kind == "trial" {
                    RunHistoryItem::Trial {
                        stable_id: values.0,
                        created_at: values.1,
                        task_id: values.2,
                        title: values.3,
                        state: values.4,
                        message: values.5,
                    }
                } else {
                    RunHistoryItem::LegacySessionActivity {
                        stable_id: values.0,
                        created_at: values.1,
                        task_id: values.2,
                        title: values.3,
                        state: values.4,
                        message: values.5,
                    }
                });
            }
            _ => return Err(ApiFailure::internal()),
        }
    }
    let next_cursor = if records.len() > 20 {
        records.get(19).map(cursor_for).transpose()?
    } else {
        None
    };
    Ok(RunHistoryResponse { items, next_cursor })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_rejects_unknown_sources() {
        assert!(parse_cursor(Some(
            "2026-09-20T10:00:00Z|manual_control|00000000-0000-0000-0000-000000000000"
        ))
        .is_err());
    }
}
