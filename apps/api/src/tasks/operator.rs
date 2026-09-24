//! Explicit maintenance only. Sensitive input is process-injected, never a CLI argument.
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        app_memberships, apps as app_rows, commercial_pilot_requests, environment_checks,
        environments, execution_runs, memberships, secret_references, users,
    },
    services::{apps, auth, commercial},
};
use async_trait::async_trait;
use chrono::DateTime;
use chrono::Utc;
use loco_rs::{
    app::AppContext,
    task::{Task, TaskInfo, Vars},
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    TransactionTrait,
};
use uuid::Uuid;
pub struct Operator;
fn id(vars: &Vars, name: &str) -> ApiResult<Uuid> {
    Uuid::parse_str(arg(vars, name)?)
        .map_err(|_| ApiFailure::invalid(format!("{name} must be a UUID")))
}
fn arg<'a>(vars: &'a Vars, name: &str) -> ApiResult<&'a str> {
    vars.cli_arg(name)
        .map_err(|_| ApiFailure::invalid(format!("Missing {name}")))
}
fn secret(name: &str) -> ApiResult<String> {
    std::env::var(name).map_err(|_| ApiFailure::invalid(format!("Inject {name} into this process")))
}
fn timestamp(vars: &Vars, name: &str) -> ApiResult<chrono::DateTime<Utc>> {
    DateTime::parse_from_rfc3339(arg(vars, name)?)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| ApiFailure::invalid(format!("{name} must be an RFC3339 timestamp")))
}
pub async fn execute(ctx: &AppContext, vars: &Vars) -> ApiResult<()> {
    let action = arg(vars, "action")?;
    if action == "provision" {
        let (user, org) = auth::provision(
            ctx,
            arg(vars, "email")?,
            arg(vars, "name")?,
            arg(vars, "organization")?,
        )
        .await?;
        println!("user_id={user} organization_id={org}");
        return Ok(());
    }
    // Global account approval is a trusted process-only operation, never a workspace API permission.
    if action == "approval" {
        let status = arg(vars, "status")?;
        if !["pending", "approved"].contains(&status) {
            return Err(ApiFailure::invalid("Status must be pending or approved"));
        }
        let result = users::Entity::update_many()
            .col_expr(
                users::Column::ApprovalStatus,
                sea_orm::sea_query::Expr::value(status),
            )
            .filter(users::Column::Id.eq(id(vars, "user")?))
            .exec(&ctx.db)
            .await?;
        if result.rows_affected != 1 {
            return Err(ApiFailure::missing());
        }
        return Ok(());
    }
    let actor = id(vars, "actor")?;
    let org = id(vars, "organization")?;
    users::Entity::find_by_id(actor)
        .filter(users::Column::DisabledAt.is_null())
        .one(&ctx.db)
        .await?
        .ok_or_else(ApiFailure::unauthorized)?;
    if apps::membership(ctx, actor, org).await?.role != "operator" {
        return Err(ApiFailure::new(
            403,
            "operator_required",
            "An active organization operator is required",
        ));
    }
    match action {
        "commercial-activate" => {
            let app = apps::authorized(ctx, actor, id(vars, "app")?).await?;
            if app.organization_id != org {
                return Err(ApiFailure::missing());
            }
            let offer = match arg(vars, "offer")? {
                "pilot" => mobile_qa_contracts::commercial::CommercialOffer::Pilot,
                "recurring" => mobile_qa_contracts::commercial::CommercialOffer::Recurring,
                _ => return Err(ApiFailure::invalid("Offer must be pilot or recurring")),
            };
            let second = vars
                .cli
                .get("second-source")
                .map(|value| {
                    Uuid::parse_str(value)
                        .map_err(|_| ApiFailure::invalid("second-source must be a UUID"))
                })
                .transpose()?;
            let agreement = commercial::activate(
                ctx,
                actor,
                app.id,
                offer,
                timestamp(vars, "starts")?,
                timestamp(vars, "ends")?,
                id(vars, "first-source")?,
                second,
                id(vars, "profile")?,
                arg(vars, "reference")?,
                arg(vars, "reason")?,
            )
            .await?;
            println!("agreement_id={agreement}");
        }
        "commercial-pause" | "commercial-end" => {
            let app = apps::authorized(ctx, actor, id(vars, "app")?).await?;
            if app.organization_id != org {
                return Err(ApiFailure::missing());
            }
            commercial::set_status(
                ctx,
                actor,
                app.id,
                if action == "commercial-pause" {
                    "paused"
                } else {
                    "ended"
                },
                arg(vars, "reason")?,
            )
            .await?;
        }
        "commercial-review" => {
            let app = apps::authorized(ctx, actor, id(vars, "app")?).await?;
            if app.organization_id != org {
                return Err(ApiFailure::missing());
            }
            let run = id(vars, "run")?;
            let row = execution_runs::Entity::find_by_id(run)
                .one(&ctx.db)
                .await?
                .ok_or_else(ApiFailure::missing)?;
            if row.app_id != app.id {
                return Err(ApiFailure::missing());
            }
            let decision = match arg(vars, "decision")? {
                "delivered" => mobile_qa_contracts::commercial::CommercialUsageState::Delivered,
                "credited" => mobile_qa_contracts::commercial::CommercialUsageState::Credited,
                _ => {
                    return Err(ApiFailure::invalid(
                        "Decision must be delivered or credited",
                    ))
                }
            };
            commercial::review(ctx, actor, run, decision, arg(vars, "reason")?).await?;
        }
        "commercial-reconcile" => {
            let app = apps::authorized(ctx, actor, id(vars, "app")?).await?;
            if app.organization_id != org {
                return Err(ApiFailure::missing());
            }
            let summary = commercial::status(ctx, actor, app.id).await?;
            let base = summary
                .agreement
                .as_ref()
                .map_or(0, |agreement| agreement.base_cents);
            let add_on = summary
                .agreement
                .as_ref()
                .map_or(0, |agreement| agreement.second_suite_cents);
            println!("app_id={},state={:?},base_cents={},second_suite_cents={},delivered_check_cents={},subtotal_cents={},delivered_checks={},reserved_checks={},credited_checks={}",app.id,summary.state,base,add_on,summary.delivered_check_cents,base+add_on+summary.delivered_check_cents,summary.delivered_checks,summary.reserved_checks,summary.credited_checks);
            for item in summary.usage {
                println!(
                    "run_id={},state={:?},amount_cents={},reason={:?}",
                    item.run_id, item.state, item.amount_cents, item.reason
                );
            }
        }
        "commercial-pilot-requests" => {
            let app_ids = app_rows::Entity::find()
                .select_only()
                .column(app_rows::Column::Id)
                .filter(app_rows::Column::OrganizationId.eq(org))
                .into_tuple::<Uuid>()
                .all(&ctx.db)
                .await?;
            for row in commercial_pilot_requests::Entity::find()
                .filter(commercial_pilot_requests::Column::AppId.is_in(app_ids))
                .order_by_desc(commercial_pilot_requests::Column::CreatedAt)
                .all(&ctx.db)
                .await?
            {
                println!(
                    "request_id={},app_id={},actor_id={},created_at={},coverage_note={}",
                    row.id, row.app_id, row.actor_id, row.created_at, row.coverage_note
                );
            }
        }
        "link-google" => {
            let user = id(vars, "user")?;
            apps::membership(ctx, user, org).await?;
            let subject = apps::text(arg(vars, "subject")?, 255, "Google subject")?;
            let result = users::Entity::update_many()
                .col_expr(
                    users::Column::GoogleSubject,
                    sea_orm::sea_query::Expr::value(subject),
                )
                .filter(users::Column::Id.eq(user))
                .filter(users::Column::GoogleSubject.is_null())
                .exec(&ctx.db)
                .await?;
            if result.rows_affected != 1 {
                return Err(ApiFailure::new(
                    409,
                    "identity_already_linked",
                    "Google identity is already linked",
                ));
            }
        }
        "membership" => {
            let user = id(vars, "user")?;
            let role = arg(vars, "role")?;
            if !["operator", "member"].contains(&role) {
                return Err(ApiFailure::invalid("Role must be operator or member"));
            }
            let active = match arg(vars, "active")? {
                "true" => true,
                "false" => false,
                _ => return Err(ApiFailure::invalid("active must be true or false")),
            };
            let existing = memberships::Entity::find()
                .filter(memberships::Column::UserId.eq(user))
                .filter(memberships::Column::OrganizationId.eq(org))
                .one(&ctx.db)
                .await?;
            if let Some(row) = existing {
                let mut a: memberships::ActiveModel = row.into();
                a.role = Set(role.into());
                a.active = Set(active);
                a.update(&ctx.db).await?;
            } else {
                memberships::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    user_id: Set(user),
                    organization_id: Set(org),
                    role: Set(role.into()),
                    active: Set(active),
                }
                .insert(&ctx.db)
                .await?;
            }
        }
        "grant" | "revoke-grant" => {
            let app = apps::authorized(ctx, actor, id(vars, "app")?).await?;
            if app.organization_id != org {
                return Err(ApiFailure::missing());
            }
            let user = id(vars, "user")?;
            apps::membership(ctx, user, org).await?;
            if action == "revoke-grant" {
                app_memberships::Entity::delete_many()
                    .filter(app_memberships::Column::UserId.eq(user))
                    .filter(app_memberships::Column::AppId.eq(app.id))
                    .exec(&ctx.db)
                    .await?;
            } else if app_memberships::Entity::find()
                .filter(app_memberships::Column::UserId.eq(user))
                .filter(app_memberships::Column::AppId.eq(app.id))
                .one(&ctx.db)
                .await?
                .is_none()
            {
                app_memberships::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    user_id: Set(user),
                    organization_id: Set(org),
                    app_id: Set(app.id),
                }
                .insert(&ctx.db)
                .await?;
            }
        }
        "reference" => {
            let app = apps::authorized(ctx, actor, id(vars, "app")?).await?;
            if app.organization_id != org {
                return Err(ApiFailure::missing());
            }
            let kind = arg(vars, "kind")?;
            if !["account", "reset"].contains(&kind) {
                return Err(ApiFailure::invalid(
                    "Reference kind must be account or reset",
                ));
            }
            let label = apps::text(arg(vars, "label")?, 80, "Reference label")?;
            // Store only a locator. Resolving customer credentials is a later worker concern.
            let locator = secret("MOBILE_QA_SECRET_LOCATOR")?;
            if locator.len() > 1024
                || !locator.starts_with("doppler://")
                || locator.chars().any(char::is_control)
            {
                return Err(ApiFailure::invalid("Provide a Doppler reference locator"));
            }
            let reference = secret_references::ActiveModel {
                id: Set(Uuid::new_v4()),
                organization_id: Set(org),
                app_id: Set(app.id),
                label: Set(label),
                locator: Set(locator),
                kind: Set(kind.into()),
                created_at: Set(Utc::now()),
            }
            .insert(&ctx.db)
            .await?;
            println!("reference_id={}", reference.id);
        }
        "observe" => {
            let app = apps::authorized(ctx, actor, id(vars, "app")?).await?;
            if app.organization_id != org {
                return Err(ApiFailure::missing());
            }
            let kind = arg(vars, "kind")?;
            let state = arg(vars, "state")?;
            if !["backend", "account", "reset"].contains(&kind)
                || !["operator_reported_ok", "operator_reported_blocked"].contains(&state)
            {
                return Err(ApiFailure::invalid("Invalid observation kind or state"));
            }
            let revision = arg(vars, "revision")?
                .parse::<i32>()
                .map_err(|_| ApiFailure::invalid("Revision must be an integer"))?;
            let note = vars
                .cli
                .get("note")
                .map(|v| apps::text(v, 500, "Observation note"))
                .transpose()?;
            let tx = ctx.db.begin().await?;
            let env = environments::Entity::find()
                .filter(environments::Column::AppId.eq(app.id))
                .lock_exclusive()
                .one(&tx)
                .await?
                .ok_or_else(ApiFailure::missing)?;
            if env.revision != revision {
                return Err(ApiFailure::new(
                    409,
                    "environment_changed",
                    "Reload the environment revision before recording an observation",
                ));
            }
            environment_checks::ActiveModel {
                id: Set(Uuid::new_v4()),
                organization_id: Set(org),
                app_id: Set(app.id),
                environment_id: Set(env.id),
                environment_revision: Set(revision),
                kind: Set(kind.into()),
                state: Set(state.into()),
                note: Set(note),
                checked_by: Set(actor),
                checked_at: Set(Utc::now()),
            }
            .insert(&tx)
            .await?;
            tx.commit().await?;
        }
        _ => return Err(ApiFailure::invalid("Unknown operator action")),
    }
    tracing::info!(%actor,organization_id=%org,%action,"operator action completed");
    Ok(())
}
#[async_trait]
impl Task for Operator {
    fn task(&self) -> TaskInfo {
        TaskInfo {
            name: "operator".into(),
            detail: "Explicit approval/provision/link-google/membership/grant/reference/observe/commercial operations"
                .into(),
        }
    }
    async fn run(&self, ctx: &AppContext, vars: &Vars) -> loco_rs::Result<()> {
        execute(ctx, vars)
            .await
            .map_err(|e| loco_rs::Error::string(&e.message))
    }
}
