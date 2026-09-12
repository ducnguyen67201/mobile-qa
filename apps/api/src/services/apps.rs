//! Tenant-scoped app/environment operations. Every nested ID is checked against its parent.
use crate::{
    config::Setup,
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        app_memberships, apps, environment_checks, environments, memberships, secret_references,
    },
};
use chrono::Utc;
use loco_rs::app::AppContext;
use mobile_qa_contracts::browser::*;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    Set, TransactionTrait,
};
use uuid::Uuid;

pub async fn membership(ctx: &AppContext, user: Uuid, org: Uuid) -> ApiResult<memberships::Model> {
    memberships::Entity::find()
        .filter(memberships::Column::UserId.eq(user))
        .filter(memberships::Column::OrganizationId.eq(org))
        .filter(memberships::Column::Active.eq(true))
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)
}
pub async fn authorized(ctx: &AppContext, user: Uuid, id: Uuid) -> ApiResult<apps::Model> {
    let app = apps::Entity::find_by_id(id)
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let m = membership(ctx, user, app.organization_id).await?;
    if m.role != "operator"
        && app_memberships::Entity::find()
            .filter(app_memberships::Column::UserId.eq(user))
            .filter(app_memberships::Column::OrganizationId.eq(app.organization_id))
            .filter(app_memberships::Column::AppId.eq(id))
            .one(&ctx.db)
            .await?
            .is_none()
    {
        return Err(ApiFailure::missing());
    }
    Ok(app)
}
pub fn text(value: &str, max: usize, label: &str) -> ApiResult<String> {
    let v = value.trim();
    if v.is_empty() || v.chars().count() > max || v.chars().any(char::is_control) {
        return Err(ApiFailure::invalid(format!(
            "{label} must contain 1–{max} characters"
        )));
    }
    Ok(v.into())
}
pub fn package(value: &str) -> ApiResult<String> {
    let value = text(value, 255, "Android package")?;
    if !value.contains('.')
        || !value.split('.').all(|p| {
            !p.is_empty()
                && p.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
                && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        })
    {
        return Err(ApiFailure::invalid(
            "Enter a dot-separated Android package such as com.example.app",
        ));
    }
    Ok(value)
}
pub fn origins(values: &[String], required: bool, secure: bool) -> ApiResult<Vec<String>> {
    if values.len() > 20 || (required && values.is_empty()) {
        return Err(ApiFailure::invalid(
            "Provide 1–20 backend origins and at most 20 login origins",
        ));
    }
    let mut result = Vec::new();
    for value in values {
        if value.len() > 2048 {
            return Err(ApiFailure::invalid("Origin is too long"));
        }
        let u = url::Url::parse(value.trim()).map_err(|_| {
            ApiFailure::invalid("Enter an HTTPS origin such as https://staging.example.com")
        })?;
        let loopback = matches!(u.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
        if !(u.scheme() == "https" || (!secure && loopback && u.scheme() == "http"))
            || !u.username().is_empty()
            || u.password().is_some()
            || u.host_str().is_none()
            || u.host_str().is_some_and(|h| h.contains('*'))
            || u.query().is_some()
            || u.fragment().is_some()
            || u.path() != "/"
        {
            return Err(ApiFailure::invalid(
                "Use an HTTPS origin without path, credentials, query or fragment",
            ));
        }
        result.push(u.origin().ascii_serialization());
    }
    result.sort();
    result.dedup();
    Ok(result)
}
pub async fn create(
    ctx: &AppContext,
    user: Uuid,
    input: CreateAppRequest,
) -> ApiResult<AppResponse> {
    membership(ctx, user, input.organization_id).await?;
    let name = text(&input.name, 100, "App name")?;
    let android_package = package(&input.android_package)?;
    let env_name = text(&input.environment_name, 80, "Environment name")?;
    let secure = Setup::get(ctx).secure_cookie;
    let backend = origins(&input.backend_origins, true, secure)?;
    let login = origins(&input.login_origins, false, secure)?;
    let tx = ctx.db.begin().await?;
    let id = Uuid::new_v4();
    let now = Utc::now();
    let app = apps::ActiveModel {
        id: Set(id),
        organization_id: Set(input.organization_id),
        name: Set(name),
        android_package: Set(android_package),
        created_by: Set(user),
        created_at: Set(now),
    }
    .insert(&tx)
    .await
    .map_err(|e| {
        if matches!(
            e.sql_err(),
            Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
        ) {
            ApiFailure::new(
                409,
                "app_exists",
                "An app with this Android package already exists",
            )
        } else {
            e.into()
        }
    })?;
    environments::ActiveModel {
        id: Set(Uuid::new_v4()),
        organization_id: Set(app.organization_id),
        app_id: Set(id),
        name: Set(env_name),
        backend_origins: Set(serde_json::json!(backend)),
        login_origins: Set(serde_json::json!(login)),
        revision: Set(1),
        account_secret_reference_id: Set(None),
        reset_secret_reference_id: Set(None),
        updated_at: Set(now),
    }
    .insert(&tx)
    .await?;
    app_memberships::ActiveModel {
        id: Set(Uuid::new_v4()),
        user_id: Set(user),
        organization_id: Set(app.organization_id),
        app_id: Set(id),
    }
    .insert(&tx)
    .await?;
    tx.commit().await?;
    detail(ctx, &app).await
}
pub async fn environment(ctx: &AppContext, app: &apps::Model) -> ApiResult<EnvironmentResponse> {
    let env = environments::Entity::find()
        .filter(environments::Column::AppId.eq(app.id))
        .filter(environments::Column::OrganizationId.eq(app.organization_id))
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let references = secret_references::Entity::find()
        .filter(secret_references::Column::AppId.eq(app.id))
        .filter(secret_references::Column::OrganizationId.eq(app.organization_id))
        .all(&ctx.db)
        .await?;
    let rows = environment_checks::Entity::find()
        .filter(environment_checks::Column::EnvironmentId.eq(env.id))
        .filter(environment_checks::Column::EnvironmentRevision.eq(env.revision))
        .order_by_desc(environment_checks::Column::CheckedAt)
        .all(&ctx.db)
        .await?;
    let checks = [
        ("backend", CheckKind::Backend),
        ("account", CheckKind::Account),
        ("reset", CheckKind::Reset),
    ]
    .into_iter()
    .map(|(name, kind)| {
        let row = rows.iter().find(|r| r.kind == name);
        EnvironmentCheck {
            kind,
            state: row
                .map(|r| {
                    if r.state == "operator_reported_ok" {
                        CheckState::OperatorReportedOk
                    } else {
                        CheckState::OperatorReportedBlocked
                    }
                })
                .unwrap_or(CheckState::NotChecked),
            note: row.and_then(|r| r.note.clone()),
            checked_by: row.map(|r| r.checked_by),
            checked_at: row.map(|r| r.checked_at),
        }
    })
    .collect();
    Ok(EnvironmentResponse {
        id: env.id,
        name: env.name,
        backend_origins: serde_json::from_value(env.backend_origins)
            .map_err(|_| ApiFailure::internal())?,
        login_origins: serde_json::from_value(env.login_origins)
            .map_err(|_| ApiFailure::internal())?,
        revision: env.revision,
        account_secret_reference_id: env.account_secret_reference_id,
        reset_secret_reference_id: env.reset_secret_reference_id,
        secret_references: references
            .into_iter()
            .map(|r| SecretReferenceSummary {
                id: r.id,
                label: r.label,
                kind: if r.kind == "account" {
                    SecretKind::Account
                } else {
                    SecretKind::Reset
                },
            })
            .collect(),
        checks,
    })
}
pub fn readiness(env: &EnvironmentResponse) -> ReadinessResponse {
    let state = |kind| {
        env.checks
            .iter()
            .find(|c| c.kind == kind)
            .map(|c| c.state)
            .unwrap_or(CheckState::NotChecked)
    };
    ReadinessResponse {
        execution_ready: false,
        install: "not_checked".into(),
        device_profile: None,
        backend: state(CheckKind::Backend),
        account: state(CheckKind::Account),
        reset: state(CheckKind::Reset),
        cases: "not_configured".into(),
        account_configured: env.account_secret_reference_id.is_some(),
        reset_configured: env.reset_secret_reference_id.is_some(),
    }
}
pub async fn detail(ctx: &AppContext, app: &apps::Model) -> ApiResult<AppResponse> {
    let env = environment(ctx, app).await?;
    Ok(AppResponse {
        id: app.id,
        organization_id: app.organization_id,
        name: app.name.clone(),
        android_package: app.android_package.clone(),
        readiness: readiness(&env),
        environment: env,
        created_at: app.created_at,
    })
}
pub async fn update_environment(
    ctx: &AppContext,
    app: &apps::Model,
    input: UpdateEnvironmentRequest,
) -> ApiResult<EnvironmentResponse> {
    let secure = Setup::get(ctx).secure_cookie;
    let name = text(&input.name, 80, "Environment name")?;
    let backend = origins(&input.backend_origins, true, secure)?;
    let login = origins(&input.login_origins, false, secure)?;
    let tx = ctx.db.begin().await?;
    let row = environments::Entity::find()
        .filter(environments::Column::AppId.eq(app.id))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if row.revision != input.expected_revision {
        return Err(ApiFailure::new(
            409,
            "environment_changed",
            "Environment changed. Reload before saving",
        ));
    }
    for (id, kind) in [
        (input.account_secret_reference_id, "account"),
        (input.reset_secret_reference_id, "reset"),
    ] {
        if let Some(id) = id {
            if secret_references::Entity::find_by_id(id)
                .filter(secret_references::Column::AppId.eq(app.id))
                .filter(secret_references::Column::OrganizationId.eq(app.organization_id))
                .filter(secret_references::Column::Kind.eq(kind))
                .one(&tx)
                .await?
                .is_none()
            {
                return Err(ApiFailure::missing());
            }
        }
    }
    let mut active: environments::ActiveModel = row.into();
    active.name = Set(name);
    active.backend_origins = Set(serde_json::json!(backend));
    active.login_origins = Set(serde_json::json!(login));
    active.revision = Set(input
        .expected_revision
        .checked_add(1)
        .ok_or_else(ApiFailure::internal)?);
    active.account_secret_reference_id = Set(input.account_secret_reference_id);
    active.reset_secret_reference_id = Set(input.reset_secret_reference_id);
    active.updated_at = Set(Utc::now());
    active.update(&tx).await?;
    tx.commit().await?;
    environment(ctx, app).await
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListQuery {
    pub cursor: Option<String>,
    pub limit: Option<u32>,
    pub organization_id: Option<Uuid>,
}
pub fn page_limit(query: &ListQuery) -> ApiResult<u64> {
    let limit = query.limit.unwrap_or(20);
    if !(1..=100).contains(&limit) {
        return Err(ApiFailure::invalid("Limit must be between 1 and 100"));
    }
    Ok(limit as u64)
}
pub async fn list(ctx: &AppContext, user: Uuid, query: ListQuery) -> ApiResult<AppListResponse> {
    let limit = page_limit(&query)?;
    if let Some(org) = query.organization_id {
        membership(ctx, user, org).await?;
    }
    let ms = memberships::Entity::find()
        .filter(memberships::Column::UserId.eq(user))
        .filter(memberships::Column::Active.eq(true))
        .all(&ctx.db)
        .await?;
    let mut scope = Condition::any().add(apps::Column::Id.is_null());
    for m in ms {
        if query
            .organization_id
            .is_some_and(|id| id != m.organization_id)
        {
            continue;
        }
        if m.role == "operator" {
            scope = scope.add(apps::Column::OrganizationId.eq(m.organization_id));
        } else {
            let grants = app_memberships::Entity::find()
                .filter(app_memberships::Column::UserId.eq(user))
                .filter(app_memberships::Column::OrganizationId.eq(m.organization_id))
                .all(&ctx.db)
                .await?;
            scope = scope.add(
                Condition::all()
                    .add(apps::Column::OrganizationId.eq(m.organization_id))
                    .add(apps::Column::Id.is_in(grants.into_iter().map(|m| m.app_id))),
            );
        }
    }
    let mut select = apps::Entity::find().filter(scope);
    if let Some(cursor) = query.cursor {
        let id = Uuid::parse_str(&cursor).map_err(|_| ApiFailure::invalid("Invalid cursor"))?;
        let row = authorized(ctx, user, id).await?;
        if query
            .organization_id
            .is_some_and(|org| org != row.organization_id)
        {
            return Err(ApiFailure::invalid(
                "Cursor belongs to another organization",
            ));
        }
        select = select.filter(
            Condition::any()
                .add(apps::Column::CreatedAt.lt(row.created_at))
                .add(
                    Condition::all()
                        .add(apps::Column::CreatedAt.eq(row.created_at))
                        .add(apps::Column::Id.lt(row.id)),
                ),
        );
    }
    let mut rows = select
        .order_by_desc(apps::Column::CreatedAt)
        .order_by_desc(apps::Column::Id)
        .limit(limit + 1)
        .all(&ctx.db)
        .await?;
    let more = rows.len() > limit as usize;
    rows.truncate(limit as usize);
    let next_cursor = if more {
        rows.last().map(|r| r.id.to_string())
    } else {
        None
    };
    let mut items = Vec::new();
    for app in rows {
        let env = environment(ctx, &app).await?;
        items.push(AppSummary {
            id: app.id,
            organization_id: app.organization_id,
            name: app.name,
            android_package: app.android_package,
            environment_name: env.name,
            created_at: app.created_at,
        });
    }
    Ok(AppListResponse { items, next_cursor })
}
