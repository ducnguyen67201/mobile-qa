//! Stripe-confirmed monthly grants and bounded, measured run consumption.
//! Checkout redirects never grant credits; only a verified paid invoice does.
use super::{apps, execution_store::*, runs};
use crate::{
    config::Setup,
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        apps as app_rows, billing_checkout_intents, billing_credit_periods, billing_credit_quotes,
        billing_credit_usage, commercial_agreements, execution_artifacts, execution_attempts,
    },
};
use chrono::{DateTime, TimeZone, Utc};
use hmac::{Hmac, Mac};
use loco_rs::app::AppContext;
use mobile_qa_contracts::{
    commercial::{
        CommercialQuoteResponse, CommercialUsageState, CreditAccessView, CreditCheckoutResponse,
        CreditPlan, CreditPlanView, CreditUsageView,
    },
    execution::{JobState, ModelUsage, RunManifest},
};
use sea_orm::{
    sea_query::{Alias, Func},
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, IntoActiveModel,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde_json::Value;
use sha2::Sha256;
use std::time::Duration;
use uuid::Uuid;

const RATE_REVISION: i32 = 1;
const INPUT_PER_MILLION: i128 = 200;
const OUTPUT_PER_MILLION: i128 = 800;
const DEVICE_PER_MINUTE: i128 = 60;
const EVIDENCE_PER_GIB: i128 = 100;
const TOKEN_RESERVE: i64 = 3_000;
const GIB: i128 = 1_073_741_824;

fn conflict(message: &str) -> ApiFailure {
    ApiFailure::new(409, "credit_conflict", message)
}

fn unavailable() -> ApiFailure {
    ApiFailure::new(
        503,
        "checkout_unavailable",
        "Checkout is not configured for this environment",
    )
}

fn terms(plan: CreditPlan) -> (i32, i32, &'static str) {
    match plan {
        CreditPlan::Starter => (50_000, 50_000, "starter"),
        CreditPlan::Plus => (75_000, 75_000, "plus"),
        CreditPlan::Business => (150_000, 150_000, "business"),
    }
}

fn parse_plan(value: &str) -> ApiResult<CreditPlan> {
    match value {
        "starter" => Ok(CreditPlan::Starter),
        "plus" => Ok(CreditPlan::Plus),
        "business" => Ok(CreditPlan::Business),
        _ => Err(ApiFailure::internal()),
    }
}

pub fn plans() -> Vec<CreditPlanView> {
    [CreditPlan::Starter, CreditPlan::Plus, CreditPlan::Business]
        .map(|plan| {
            let (monthly_cents, monthly_credits, _) = terms(plan);
            CreditPlanView {
                plan,
                monthly_cents,
                monthly_credits,
                currency: "USD".into(),
            }
        })
        .to_vec()
}

#[derive(Clone)]
struct Period {
    id: Uuid,
    plan: CreditPlan,
    grant: i64,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
    rate_revision: i32,
}

async fn period(db: &impl ConnectionTrait, app: Uuid, lock: bool) -> ApiResult<Option<Period>> {
    let now = Utc::now();
    let query = billing_credit_periods::Entity::find()
        .filter(billing_credit_periods::Column::AppId.eq(app))
        .filter(billing_credit_periods::Column::StartsAt.lte(now))
        .filter(billing_credit_periods::Column::EndsAt.gt(now))
        .order_by_desc(billing_credit_periods::Column::StartsAt);
    let row = if lock {
        query.lock_exclusive().one(db).await?
    } else {
        query.one(db).await?
    };
    row.map(|row| {
        Ok(Period {
            id: row.id,
            plan: parse_plan(&row.plan)?,
            grant: row.granted_credits,
            starts_at: row.starts_at,
            ends_at: row.ends_at,
            rate_revision: row.rate_revision,
        })
    })
    .transpose()
}

async fn balance(db: &impl ConnectionTrait, p: &Period) -> ApiResult<(i64, i64, i64)> {
    async fn usage_sum(
        db: &impl ConnectionTrait,
        period_id: Uuid,
        state: &str,
        column: billing_credit_usage::Column,
    ) -> ApiResult<i64> {
        Ok(billing_credit_usage::Entity::find()
            .filter(billing_credit_usage::Column::PeriodId.eq(period_id))
            .filter(billing_credit_usage::Column::State.eq(state))
            .select_only()
            .expr_as(Func::cast_as(column.sum(), Alias::new("bigint")), "total")
            .into_tuple::<Option<i64>>()
            .one(db)
            .await?
            .flatten()
            .unwrap_or_default())
    }
    let charged = usage_sum(
        db,
        p.id,
        "settled",
        billing_credit_usage::Column::ChargedCredits,
    )
    .await?;
    let held = usage_sum(db, p.id, "held", billing_credit_usage::Column::HeldCredits).await?;
    Ok((
        charged,
        held,
        p.grant.saturating_sub(charged).saturating_sub(held),
    ))
}

pub async fn status(db: &impl ConnectionTrait, app: Uuid) -> ApiResult<Option<CreditAccessView>> {
    let Some(p) = period(db, app, false).await? else {
        return Ok(None);
    };
    let (charged, held, available) = balance(db, &p).await?;
    let grant = billing_credit_periods::Entity::find_by_id(p.id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let pending = billing_checkout_intents::Entity::find_by_id(grant.checkout_intent_id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let pending_plan = pending
        .pending_plan
        .map(|value| parse_plan(&value))
        .transpose()?;
    let pending_effective_at = pending.pending_effective_at;
    let mut usage = Vec::new();
    for r in billing_credit_usage::Entity::find()
        .filter(billing_credit_usage::Column::PeriodId.eq(p.id))
        .order_by_desc(billing_credit_usage::Column::CreatedAt)
        .order_by_desc(billing_credit_usage::Column::RunId)
        .limit(100)
        .all(db)
        .await?
    {
        usage.push(CreditUsageView {
            run_id: r.run_id,
            state: r.state,
            held_credits: i32::try_from(r.held_credits).map_err(|_| ApiFailure::internal())?,
            measured_credits: r.measured_credits.map(|v| v.to_string()),
            charged_credits: i32::try_from(r.charged_credits)
                .map_err(|_| ApiFailure::internal())?,
            input_tokens: r.input_tokens.map(|v| v.to_string()),
            output_tokens: r.output_tokens.map(|v| v.to_string()),
            device_seconds: r
                .device_seconds
                .map(i32::try_from)
                .transpose()
                .map_err(|_| ApiFailure::internal())?,
            stored_bytes: r.stored_bytes.map(|v| v.to_string()),
            reason: r.reason,
        });
    }
    Ok(Some(CreditAccessView {
        plan: p.plan,
        pending_plan,
        pending_effective_at,
        period_start: p.starts_at,
        period_end: p.ends_at,
        granted_credits: i32::try_from(p.grant).map_err(|_| ApiFailure::internal())?,
        charged_credits: i32::try_from(charged).map_err(|_| ApiFailure::internal())?,
        held_credits: i32::try_from(held).map_err(|_| ApiFailure::internal())?,
        available_credits: i32::try_from(available).map_err(|_| ApiFailure::internal())?,
        rate_revision: p.rate_revision,
        usage,
    }))
}

async fn stripe_get(client: &reqwest::Client, secret: &str, path: &str) -> ApiResult<Value> {
    client
        .get(format!("https://api.stripe.com/v1/{path}"))
        .bearer_auth(secret)
        .send()
        .await
        .map_err(|_| unavailable())?
        .error_for_status()
        .map_err(|_| unavailable())?
        .json()
        .await
        .map_err(|_| unavailable())
}

async fn stripe_post(
    client: &reqwest::Client,
    secret: &str,
    path: &str,
    key: &str,
    form: &[(&str, String)],
) -> ApiResult<Value> {
    client
        .post(format!("https://api.stripe.com/v1/{path}"))
        .bearer_auth(secret)
        .header("Idempotency-Key", key)
        .form(form)
        .send()
        .await
        .map_err(|_| unavailable())?
        .error_for_status()
        .map_err(|_| unavailable())?
        .json()
        .await
        .map_err(|_| unavailable())
}

fn stripe_id<'a>(value: &'a Value, prefix: &str) -> ApiResult<&'a str> {
    let id = value.as_str().ok_or_else(ApiFailure::internal)?;
    if !id.starts_with(prefix) || !id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
        return Err(ApiFailure::internal());
    }
    Ok(id)
}

/// A scheduled phase keeps the paid period untouched. The old plan stays active
/// until Stripe's next invoice is paid; an existing pending change must be
/// canceled before choosing another plan.
pub async fn change_plan(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    target: CreditPlan,
) -> ApiResult<()> {
    apps::authorized(ctx, actor, app).await?;
    let tx = ctx.db.begin().await?;
    app_rows::Entity::find_by_id(app)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let p = period(&tx, app, false)
        .await?
        .ok_or_else(|| conflict("A paid monthly allowance is required to change plans"))?;
    let grant = billing_credit_periods::Entity::find_by_id(p.id)
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let row = billing_checkout_intents::Entity::find_by_id(grant.checkout_intent_id)
        .filter(billing_checkout_intents::Column::State.eq("paid"))
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let intent_id = row.id;
    let current = parse_plan(&row.plan)?;
    if current != p.plan {
        return Err(conflict("The current paid period needs billing review"));
    }
    let pending = row
        .pending_plan
        .clone()
        .map(|value| parse_plan(&value))
        .transpose()?;
    if pending == Some(target) {
        tx.commit().await?;
        return Ok(());
    }
    if pending.is_some() && target != current {
        return Err(conflict(
            "Keep the current plan before scheduling a different plan",
        ));
    }
    if pending.is_none() && target == current {
        return Err(conflict("This is already your current plan"));
    }
    let secret = std::env::var("STRIPE_SECRET_KEY").map_err(|_| unavailable())?;
    if secret.is_empty() {
        return Err(unavailable());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| unavailable())?;
    let sid = row
        .stripe_subscription_id
        .clone()
        .ok_or_else(ApiFailure::internal)?;
    stripe_id(&Value::String(sid.clone()), "sub_")?;
    let subscription = stripe_get(&client, &secret, &format!("subscriptions/{sid}")).await?;
    let item = subscription["items"]["data"]
        .as_array()
        .filter(|items| items.len() == 1)
        .and_then(|items| items.first())
        .ok_or_else(|| conflict("Subscription items need billing review"))?;
    let price = &item["price"];
    let (cents, _, _) = terms(current);
    if subscription["id"] != sid
        || subscription["status"] != "active"
        || subscription["cancel_at_period_end"] == true
        || subscription["metadata"]["checkout_intent_id"] != intent_id.to_string()
        || item["quantity"] != 1
        || price["unit_amount"] != cents
        || price["currency"] != "usd"
        || price["recurring"]["interval"] != "month"
        || price["recurring"]["interval_count"] != 1
        || item["current_period_end"] != p.ends_at.timestamp()
    {
        return Err(conflict("Subscription and paid period need billing review"));
    }
    let stored_schedule = row.stripe_schedule_id.clone();
    if pending.is_some() {
        let schedule = stored_schedule
            .as_deref()
            .ok_or_else(ApiFailure::internal)?;
        stripe_id(&Value::String(schedule.into()), "sub_sched_")?;
        if subscription["schedule"].is_null() {
            // A release can succeed even if the local commit was interrupted.
            let remote = stripe_get(
                &client,
                &secret,
                &format!("subscription_schedules/{schedule}"),
            )
            .await?;
            if remote["status"] != "released" || remote["released_subscription"] != sid {
                return Err(conflict("Subscription schedule needs billing review"));
            }
        } else if subscription["schedule"] != schedule {
            return Err(conflict("Subscription schedule needs billing review"));
        } else {
            stripe_post(
                &client,
                &secret,
                &format!("subscription_schedules/{schedule}/release"),
                &format!("credit-plan-release-{intent_id}-{schedule}"),
                &[],
            )
            .await?;
        }
        let mut active = row.clone().into_active_model();
        active.pending_plan = Set(None);
        active.pending_effective_at = Set(None);
        active.stripe_schedule_id = Set(None);
        active.update(&tx).await?;
        tx.commit().await?;
        return Ok(());
    }
    let price_id = stripe_id(&price["id"], "price_")?;
    let already_scheduled = !subscription["schedule"].is_null();
    let schedule = if !already_scheduled {
        stripe_post(
            &client,
            &secret,
            "subscription_schedules",
            &format!("credit-plan-schedule-{intent_id}-{}", Uuid::new_v4()),
            &[("from_subscription", sid.clone())],
        )
        .await?
    } else {
        let schedule_id = stripe_id(&subscription["schedule"], "sub_sched_")?;
        if stored_schedule
            .as_deref()
            .is_some_and(|stored| stored != schedule_id)
        {
            return Err(conflict("Subscription schedule needs billing review"));
        }
        stripe_get(
            &client,
            &secret,
            &format!("subscription_schedules/{schedule_id}"),
        )
        .await?
    };
    let schedule_id = stripe_id(&schedule["id"], "sub_sched_")?;
    if schedule["subscription"] != sid
        || schedule["status"] != "active"
        || schedule["current_phase"]["end_date"] != p.ends_at.timestamp()
    {
        return Err(conflict("Subscription schedule needs billing review"));
    }
    let phases = schedule["phases"]
        .as_array()
        .ok_or_else(ApiFailure::internal)?;
    if phases.len() == 2
        && schedule["metadata"]["checkout_intent_id"] == intent_id.to_string()
        && schedule["metadata"]["pending_plan"] == terms(target).2
        && phases[1]["start_date"] == p.ends_at.timestamp()
    {
        // Stripe succeeded previously but the database commit was interrupted.
        let future_price = stripe_id(&phases[1]["items"][0]["price"], "price_")?;
        let future = stripe_get(&client, &secret, &format!("prices/{future_price}")).await?;
        if future["unit_amount"] == terms(target).0
            && future["currency"] == "usd"
            && future["recurring"]["interval"] == "month"
        {
            let mut active = row.clone().into_active_model();
            active.pending_plan = Set(Some(terms(target).2.into()));
            active.pending_effective_at = Set(Some(p.ends_at));
            active.stripe_schedule_id = Set(Some(schedule_id.into()));
            active.update(&tx).await?;
            tx.commit().await?;
            return Ok(());
        }
    }
    // A one-phase schedule already attached to the subscription might have
    // been created outside this app. Only recover a completed, tagged change.
    if already_scheduled || phases.len() != 1 || phases[0]["items"][0]["price"] != price_id {
        return Err(conflict("An existing schedule needs billing review"));
    }
    let start = schedule["current_phase"]["start_date"]
        .as_i64()
        .ok_or_else(ApiFailure::internal)?;
    let (next_cents, _, target_word) = terms(target);
    // Checkout creates a plan-named product. A new Stripe price with its own
    // product keeps the renewal invoice label aligned with the selected plan.
    let next_price = stripe_post(
        &client,
        &secret,
        "prices",
        &format!("credit-plan-price-{intent_id}-{schedule_id}-{target_word}"),
        &[
            ("currency", "usd".into()),
            ("unit_amount", next_cents.to_string()),
            ("recurring[interval]", "month".into()),
            (
                "product_data[name]",
                format!("Mobile QA {target_word} monthly credits"),
            ),
        ],
    )
    .await?;
    let next_price_id = stripe_id(&next_price["id"], "price_")?;
    if next_price["unit_amount"] != next_cents
        || next_price["currency"] != "usd"
        || next_price["recurring"]["interval"] != "month"
    {
        return Err(unavailable());
    }
    let form = [
        ("end_behavior", "release".into()),
        ("proration_behavior", "none".into()),
        ("metadata[checkout_intent_id]", intent_id.to_string()),
        ("metadata[pending_plan]", target_word.into()),
        ("phases[0][start_date]", start.to_string()),
        ("phases[0][end_date]", p.ends_at.timestamp().to_string()),
        ("phases[0][items][0][price]", price_id.into()),
        ("phases[0][items][0][quantity]", "1".into()),
        (
            "phases[0][metadata][checkout_intent_id]",
            intent_id.to_string(),
        ),
        ("phases[1][start_date]", p.ends_at.timestamp().to_string()),
        ("phases[1][duration][interval]", "month".into()),
        ("phases[1][duration][interval_count]", "1".into()),
        ("phases[1][proration_behavior]", "none".into()),
        ("phases[1][items][0][price]", next_price_id.into()),
        ("phases[1][items][0][quantity]", "1".into()),
        (
            "phases[1][metadata][checkout_intent_id]",
            intent_id.to_string(),
        ),
    ];
    let updated = stripe_post(
        &client,
        &secret,
        &format!("subscription_schedules/{schedule_id}"),
        &format!("credit-plan-set-{intent_id}-{schedule_id}-{target_word}"),
        &form,
    )
    .await?;
    if updated["id"] != schedule_id
        || updated["phases"]
            .as_array()
            .is_none_or(|items| items.len() != 2)
    {
        return Err(unavailable());
    }
    let mut active = row.into_active_model();
    active.pending_plan = Set(Some(target_word.into()));
    active.pending_effective_at = Set(Some(p.ends_at));
    active.stripe_schedule_id = Set(Some(schedule_id.into()));
    active.update(&tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Create a hosted subscription checkout with an app-bound, server-priced plan.
pub async fn checkout(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    plan: CreditPlan,
) -> ApiResult<CreditCheckoutResponse> {
    apps::authorized(ctx, actor, app).await?;
    let secret = std::env::var("STRIPE_SECRET_KEY").map_err(|_| unavailable())?;
    if secret.is_empty()
        || std::env::var("STRIPE_WEBHOOK_SECRET").map_or(true, |value| value.is_empty())
    {
        return Err(unavailable());
    }
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| unavailable())?;
    let (cents, _, word) = terms(plan);
    let origin = Setup::get(ctx).origin;
    let tx = ctx.db.begin().await?;
    let app_row = app_rows::Entity::find_by_id(app)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let workspace = app_row.organization_id;
    if period(&tx, app, false).await?.is_some() {
        return Err(conflict("This app already has an active monthly allowance"));
    }
    if billing_checkout_intents::Entity::find()
        .filter(billing_checkout_intents::Column::AppId.eq(app))
        .filter(billing_checkout_intents::Column::State.eq("paid"))
        .one(&tx)
        .await?
        .is_some()
    {
        return Err(conflict(
            "An existing subscription owns this app; avoid creating a second checkout",
        ));
    }
    if commercial_agreements::Entity::find()
        .filter(commercial_agreements::Column::AppId.eq(app))
        .filter(commercial_agreements::Column::Status.eq("active"))
        .filter(commercial_agreements::Column::EndsAt.gt(Utc::now()))
        .one(&tx)
        .await?
        .is_some()
    {
        return Err(conflict("This app already has an active check agreement"));
    }
    let open = billing_checkout_intents::Entity::find()
        .filter(billing_checkout_intents::Column::AppId.eq(app))
        .filter(billing_checkout_intents::Column::State.is_in(["pending", "checkout"]))
        .one(&tx)
        .await?;
    let reusable = if let Some(r) = open {
        let existing_id = r.id;
        let session_id = r.stripe_session_id.clone();
        if Utc::now() - r.created_at >= chrono::Duration::hours(24) {
            // An overdue local intent may have completed payment while its
            // webhook is delayed. Only Stripe-confirmed expiry frees the app.
            let session_id = session_id.ok_or_else(|| conflict("Checkout needs billing review"))?;
            stripe_id(&Value::String(session_id.clone()), "cs_")?;
            let remote =
                stripe_get(&client, &secret, &format!("checkout/sessions/{session_id}")).await?;
            if remote["id"] != session_id {
                return Err(unavailable());
            }
            if remote["status"] == "expired" {
                let mut active = r.into_active_model();
                active.state = Set("expired".into());
                active.update(&tx).await?;
                None
            } else {
                return Err(conflict("Checkout or payment is still being confirmed"));
            }
        } else {
            if r.plan != word {
                return Err(conflict(
                    "Finish the existing checkout before choosing another plan",
                ));
            }
            if let Some(url) = r.stripe_session_url {
                tx.commit().await?;
                return Ok(CreditCheckoutResponse { url });
            }
            Some(existing_id)
        }
    } else {
        None
    };
    let id = if let Some(id) = reusable {
        id
    } else {
        let id = Uuid::new_v4();
        billing_checkout_intents::ActiveModel {
            id: Set(id),
            app_id: Set(app),
            actor_id: Set(actor),
            plan: Set(word.into()),
            ..Default::default()
        }
        .insert(&tx)
        .await?;
        id
    };
    tx.commit().await?;
    let success =
        format!("{origin}/settings/commercial?workspace={workspace}&app={app}&checkout=return");
    let cancel =
        format!("{origin}/settings/commercial?workspace={workspace}&app={app}&checkout=cancel");
    let form = [
        ("mode", "subscription".to_owned()),
        ("client_reference_id", id.to_string()),
        ("success_url", success),
        ("cancel_url", cancel),
        ("line_items[0][quantity]", "1".into()),
        ("line_items[0][price_data][currency]", "usd".into()),
        ("line_items[0][price_data][unit_amount]", cents.to_string()),
        (
            "line_items[0][price_data][recurring][interval]",
            "month".into(),
        ),
        (
            "line_items[0][price_data][product_data][name]",
            format!("Mobile QA {word} monthly credits"),
        ),
        (
            "subscription_data[metadata][checkout_intent_id]",
            id.to_string(),
        ),
    ];
    // Reuse the same intent and idempotency key after an uncertain network or
    // database failure; a new key could create a second payable session.
    let body = stripe_post(
        &client,
        &secret,
        "checkout/sessions",
        &id.to_string(),
        &form,
    )
    .await?;
    let session = stripe_id(&body["id"], "cs_")?;
    let url = body["url"].as_str().ok_or_else(unavailable)?;
    if !url.starts_with("https://checkout.stripe.com/") {
        return Err(unavailable());
    }
    billing_checkout_intents::Entity::update_many()
        .col_expr(
            billing_checkout_intents::Column::State,
            sea_orm::sea_query::Expr::value("checkout"),
        )
        .col_expr(
            billing_checkout_intents::Column::StripeSessionId,
            sea_orm::sea_query::Expr::value(Some(session.to_owned())),
        )
        .col_expr(
            billing_checkout_intents::Column::StripeSessionUrl,
            sea_orm::sea_query::Expr::value(Some(url.to_owned())),
        )
        .filter(billing_checkout_intents::Column::Id.eq(id))
        .filter(billing_checkout_intents::Column::State.eq("pending"))
        .exec(&ctx.db)
        .await?;
    Ok(CreditCheckoutResponse { url: url.into() })
}

fn stripe_time(value: &Value) -> ApiResult<DateTime<Utc>> {
    let seconds = value.as_i64().ok_or_else(ApiFailure::internal)?;
    Utc.timestamp_opt(seconds, 0)
        .single()
        .ok_or_else(ApiFailure::internal)
}

fn verify_signature(body: &[u8], signature: &str, secret: &str) -> ApiResult<()> {
    let mut stamp = None;
    let mut signatures = Vec::new();
    for part in signature.split(',') {
        if let Some(v) = part.strip_prefix("t=") {
            stamp = v.parse::<i64>().ok();
        }
        if let Some(v) = part.strip_prefix("v1=") {
            signatures.push(v);
        }
    }
    let stamp = stamp.ok_or_else(ApiFailure::unauthorized)?;
    if (i128::from(Utc::now().timestamp()) - i128::from(stamp)).abs() > 300 {
        return Err(ApiFailure::unauthorized());
    }
    let mut signed = stamp.to_string().into_bytes();
    signed.push(b'.');
    signed.extend_from_slice(body);
    let valid = signatures.iter().any(|hex| {
        let Ok(bytes) = hex::decode(hex) else {
            return false;
        };
        let Ok(mut mac) = Hmac::<Sha256>::new_from_slice(secret.as_bytes()) else {
            return false;
        };
        mac.update(&signed);
        mac.verify_slice(&bytes).is_ok()
    });
    if valid {
        Ok(())
    } else {
        Err(ApiFailure::unauthorized())
    }
}

// Stripe's current Invoice shape uses status=paid; older events may contain a
// paid boolean instead. Only automatically collected invoices fund credits.
fn is_paid_invoice(invoice: &Value) -> bool {
    let paid = if invoice["status"].is_string() {
        invoice["status"] == "paid"
    } else {
        invoice["paid"] == true
    };
    paid && invoice["collection_method"] == "charge_automatically"
        && invoice["paid_out_of_band"] != true
}

/// A verified paid invoice is the sole authority to create a new monthly grant.
pub async fn webhook(ctx: &AppContext, signature: &str, raw: &[u8]) -> ApiResult<()> {
    let webhook_secret = std::env::var("STRIPE_WEBHOOK_SECRET").map_err(|_| unavailable())?;
    verify_signature(raw, signature, &webhook_secret)?;
    let event: Value =
        serde_json::from_slice(raw).map_err(|_| ApiFailure::invalid("Invalid webhook"))?;
    if event["type"] != "invoice.paid" {
        return Ok(());
    }
    let invoice = &event["data"]["object"];
    if invoice["currency"] != "usd" {
        return Ok(());
    }
    if !is_paid_invoice(invoice) {
        return Err(ApiFailure::internal());
    }
    let subscription_id = invoice["subscription"]
        .as_str()
        .or_else(|| invoice["parent"]["subscription_details"]["subscription"].as_str())
        .ok_or_else(ApiFailure::internal)?;
    let key = std::env::var("STRIPE_SECRET_KEY").map_err(|_| unavailable())?;
    let subscription: Value = reqwest::Client::new()
        .get(format!(
            "https://api.stripe.com/v1/subscriptions/{subscription_id}"
        ))
        .bearer_auth(key)
        .send()
        .await
        .map_err(|_| unavailable())?
        .error_for_status()
        .map_err(|_| unavailable())?
        .json()
        .await
        .map_err(|_| unavailable())?;
    apply_verified_invoice(ctx, invoice, &subscription).await
}

/// Process provider objects only after webhook signature verification and a live
/// subscription fetch. Kept separate so duplicate/invalid grant behavior is testable.
pub async fn apply_verified_invoice(
    ctx: &AppContext,
    invoice: &Value,
    subscription: &Value,
) -> ApiResult<()> {
    let subscription_id = invoice["subscription"]
        .as_str()
        .or_else(|| invoice["parent"]["subscription_details"]["subscription"].as_str())
        .ok_or_else(ApiFailure::internal)?;
    let invoice_id = invoice["id"].as_str().ok_or_else(ApiFailure::internal)?;
    if subscription["id"] != subscription_id {
        return Err(ApiFailure::internal());
    }
    let intent_id: Uuid = subscription["metadata"]["checkout_intent_id"]
        .as_str()
        .ok_or_else(ApiFailure::internal)?
        .parse()
        .map_err(|_| ApiFailure::internal())?;
    let line = invoice["lines"]["data"]
        .as_array()
        .and_then(|items| {
            items.iter().find(|item| {
                item["period"]["start"].is_number() && item["period"]["end"].is_number()
            })
        })
        .ok_or_else(ApiFailure::internal)?;
    let start = stripe_time(&line["period"]["start"])?;
    let end = stripe_time(&line["period"]["end"])?;
    if end <= start
        || end - start > chrono::Duration::days(32)
        || end - start < chrono::Duration::days(28)
    {
        return Err(ApiFailure::internal());
    }
    let tx = ctx.db.begin().await?;
    let app = billing_checkout_intents::Entity::find_by_id(intent_id)
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?
        .app_id;
    // All billing mutations lock the app before the intent, including checkout
    // and plan changes. Keeping the same order avoids renewal deadlocks.
    app_rows::Entity::find_by_id(app)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let row = billing_checkout_intents::Entity::find_by_id(intent_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if row.app_id != app {
        return Err(ApiFailure::internal());
    }
    let recorded_subscription = row.stripe_subscription_id.clone();
    if recorded_subscription
        .as_deref()
        .is_some_and(|id| id != subscription_id)
    {
        return Err(ApiFailure::internal());
    }
    // An old invoice can be replayed after Stripe has advanced to a new price.
    // Its already verified grant stays immutable and never applies a second time.
    let existing = billing_credit_periods::Entity::find()
        .filter(
            Condition::any()
                .add(billing_credit_periods::Column::StripeInvoiceId.eq(invoice_id))
                .add(
                    Condition::all()
                        .add(
                            billing_credit_periods::Column::StripeSubscriptionId
                                .eq(subscription_id),
                        )
                        .add(billing_credit_periods::Column::StartsAt.eq(start)),
                ),
        )
        .one(&tx)
        .await?;
    if let Some(grant) = existing {
        if grant.stripe_invoice_id == invoice_id
            && grant.stripe_subscription_id == subscription_id
            && grant.app_id == app
        {
            tx.commit().await?;
            return Ok(());
        }
        return Err(ApiFailure::internal());
    }
    let current = parse_plan(&row.plan)?;
    let pending = row
        .pending_plan
        .clone()
        .map(|value| parse_plan(&value))
        .transpose()?;
    let effective = row.pending_effective_at;
    let switching = pending.is_some() && effective == Some(start);
    if pending.is_some() && effective.is_some_and(|date| start > date) {
        return Err(ApiFailure::internal());
    }
    let plan = if switching { pending.unwrap() } else { current };
    let (cents, credits, word) = terms(plan);
    if !is_paid_invoice(invoice)
        || invoice["currency"] != "usd"
        || invoice["amount_paid"].as_i64().unwrap_or(0) < i64::from(cents)
    {
        return Err(ApiFailure::internal());
    }
    let items = subscription["items"]["data"]
        .as_array()
        .ok_or_else(ApiFailure::internal)?;
    if items.len() != 1 || items[0]["quantity"] != 1 {
        return Err(ApiFailure::internal());
    }
    let price = &subscription["items"]["data"][0]["price"];
    if price["unit_amount"] != cents
        || price["currency"] != "usd"
        || price["recurring"]["interval"] != "month"
    {
        return Err(ApiFailure::internal());
    }
    let price_id = price["id"].as_str().ok_or_else(ApiFailure::internal)?;
    let line_price = line["price"]["id"]
        .as_str()
        .or_else(|| line["pricing"]["price_details"]["price"].as_str())
        .ok_or_else(ApiFailure::internal)?;
    if line_price != price_id {
        return Err(ApiFailure::internal());
    }
    billing_credit_periods::ActiveModel {
        id: Set(Uuid::new_v4()),
        app_id: Set(app),
        checkout_intent_id: Set(intent_id),
        stripe_subscription_id: Set(subscription_id.into()),
        stripe_invoice_id: Set(invoice_id.into()),
        plan: Set(word.into()),
        granted_credits: Set(i64::from(credits)),
        starts_at: Set(start),
        ends_at: Set(end),
        rate_revision: Set(RATE_REVISION),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    let mut active = row.into_active_model();
    active.state = Set("paid".into());
    active.stripe_subscription_id = Set(Some(subscription_id.into()));
    active.plan = Set(word.into());
    if switching {
        active.pending_plan = Set(None);
        active.pending_effective_at = Set(None);
        active.stripe_schedule_id = Set(None);
    }
    active.update(&tx).await?;
    tx.commit().await?;
    Ok(())
}

fn ceil(n: i128, denominator: i128) -> i128 {
    (n + denominator - 1) / denominator
}

fn measured_credits(
    input: i64,
    output: i64,
    device_seconds: i64,
    stored_bytes: i64,
) -> ApiResult<i64> {
    let denominator = 1_000_000_i128 * 60 * GIB;
    let numerator = i128::from(input) * INPUT_PER_MILLION * 60 * GIB
        + i128::from(output) * OUTPUT_PER_MILLION * 60 * GIB
        + i128::from(device_seconds) * DEVICE_PER_MINUTE * 1_000_000 * GIB
        + i128::from(stored_bytes) * EVIDENCE_PER_GIB * 1_000_000 * 60;
    i64::try_from(ceil(numerator, denominator)).map_err(|_| ApiFailure::internal())
}

fn authorized_device_seconds(manifest: &RunManifest) -> ApiResult<i64> {
    let attempts = i64::try_from(manifest.cases.len())
        .map_err(|_| ApiFailure::internal())?
        .checked_mul(1 + i64::from(manifest.diagnostic_retries))
        .ok_or_else(ApiFailure::internal)?;
    let cleanup = manifest
        .profile
        .execution_context
        .as_ref()
        .map_or(610_i64, |context| {
            i64::from(context.stages.cleanup_seconds) + 10
        });
    let per_attempt = 1800_i64
        .checked_add(cleanup)
        .ok_or_else(ApiFailure::internal)?;
    i64::from(manifest.budget.duration_seconds)
        .checked_add(
            attempts
                .checked_mul(per_attempt)
                .ok_or_else(ApiFailure::internal)?,
        )
        .ok_or_else(ApiFailure::internal)
}

fn maximum(manifest: &RunManifest) -> ApiResult<i64> {
    // A hold bounds occupancy through preparation and cleanup; it is not a flat charge.
    // Billing still measures claim→release and caps settlement at the accepted quote.
    let seconds = authorized_device_seconds(manifest)?;
    let bytes = i64::from(manifest.budget.artifact_bytes);
    TOKEN_RESERVE
        .checked_add(measured_credits(0, 0, seconds, bytes)?)
        .ok_or_else(ApiFailure::internal)
}

#[allow(clippy::too_many_arguments)]
pub async fn quote(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    kind: &str,
    source: Uuid,
    build: Uuid,
    environment_revision: i32,
    request_hash: &str,
    manifest: &RunManifest,
) -> ApiResult<Option<CommercialQuoteResponse>> {
    let Some(p) = period(&ctx.db, app, false).await? else {
        return Ok(None);
    };
    let (_, _, available) = balance(&ctx.db, &p).await?;
    let max = maximum(manifest)?;
    if max > available {
        return Err(conflict(
            "Not enough credits for this run's maximum authorization",
        ));
    }
    let id = Uuid::new_v4();
    let expiry = Utc::now() + chrono::Duration::minutes(10);
    let manifest_hash = hash(serde_json::to_vec(manifest).map_err(|_| ApiFailure::internal())?);
    billing_credit_quotes::ActiveModel {
        id: Set(id),
        period_id: Set(p.id),
        app_id: Set(app),
        actor_id: Set(actor),
        kind: Set(kind.into()),
        request_hash: Set(request_hash.into()),
        manifest_hash: Set(manifest_hash),
        build_id: Set(build),
        source_version_id: Set(source),
        profile_id: Set(manifest.profile.id),
        environment_revision: Set(environment_revision),
        rate_revision: Set(p.rate_revision),
        max_credits: Set(max),
        expires_at: Set(expiry),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
    Ok(Some(CommercialQuoteResponse {
        id,
        app_id: app,
        kind: kind.into(),
        build_id: build,
        source_version_id: source,
        profile_id: manifest.profile.id,
        case_count: manifest.cases.len() as i32,
        amount_cents: 0,
        maximum_credits: Some(i32::try_from(max).map_err(|_| ApiFailure::internal())?),
        credits_after_authorization: Some(
            i32::try_from(available - max).map_err(|_| ApiFailure::internal())?,
        ),
        currency: "USD".into(),
        checks_after_authorization: 0,
        check_cap: 0,
        expires_at: expiry,
    }))
}

#[allow(clippy::too_many_arguments)]
pub async fn reserve(
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
    quote_id: Uuid,
    kind: &str,
    request_hash: &str,
    manifest: &RunManifest,
    run_id: Uuid,
) -> ApiResult<bool> {
    let Some(q) = billing_credit_quotes::Entity::find_by_id(quote_id)
        .one(db)
        .await?
    else {
        return Ok(false);
    };
    let p = period(db, app, true)
        .await?
        .ok_or_else(|| conflict("The monthly allowance has ended"))?;
    let digest = hash(serde_json::to_vec(manifest).map_err(|_| ApiFailure::internal())?);
    let source = manifest
        .source
        .as_ref()
        .map(|source| match source {
            mobile_qa_contracts::regression::RunSource::SavedCaseV1 { case_version_id } => {
                *case_version_id
            }
            mobile_qa_contracts::regression::RunSource::SavedSuiteV1 { suite_version_id } => {
                *suite_version_id
            }
            mobile_qa_contracts::regression::RunSource::ReleasePlanV1 { plan_version_id } => {
                *plan_version_id
            }
        })
        .or(manifest.plan_version_id)
        .ok_or_else(ApiFailure::internal)?;
    let max = q.max_credits;
    if q.period_id != p.id
        || q.app_id != app
        || q.actor_id != actor
        || q.kind != kind
        || q.request_hash != request_hash
        || q.manifest_hash != digest
        || q.build_id != manifest.build_id
        || q.source_version_id != source
        || q.profile_id != manifest.profile.id
        || q.environment_revision != manifest.environment_revision
        || q.rate_revision != p.rate_revision
        || q.expires_at <= Utc::now()
        || max != maximum(manifest)?
    {
        return Err(conflict(
            "Run setup or credit quote changed; refresh authorization",
        ));
    }
    let (_, _, available) = balance(db, &p).await?;
    if max > available {
        return Err(conflict(
            "Not enough credits for this run's maximum authorization",
        ));
    }
    billing_credit_usage::ActiveModel {
        run_id: Set(run_id),
        quote_id: Set(quote_id),
        period_id: Set(p.id),
        app_id: Set(app),
        held_credits: Set(max),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(true)
}

pub async fn review(
    ctx: &AppContext,
    actor: Uuid,
    run: Uuid,
    decision: CommercialUsageState,
    reason: &str,
) -> ApiResult<bool> {
    let tx = ctx.db.begin().await?;
    let Some(row) = billing_credit_usage::Entity::find_by_id(run)
        .lock_exclusive()
        .one(&tx)
        .await?
    else {
        return Ok(false);
    };
    if row.state != "held" {
        return Err(conflict("This run was already reviewed"));
    }
    let detail = runs::detail(&tx, run).await?;
    if detail.state != JobState::Finished {
        return Err(conflict("Finish this run before reviewing its usage"));
    }
    let (state, measured, charged, input, output, seconds, bytes) =
        if decision == CommercialUsageState::Credited {
            ("released", None, 0, None, None, None, None)
        } else {
            let mut input = 0_i64;
            let mut output = 0_i64;
            let mut seconds = 0_i64;
            let attempts = execution_attempts::Entity::find()
                .filter(execution_attempts::Column::RunId.eq(run))
                .all(&tx)
                .await?;
            for r in &attempts {
                let claim = r.claim_id;
                if claim.is_some() {
                    let start = r.claimed_at;
                    let end = r.released_at;
                    let (Some(start), Some(end)) = (start, end) else {
                        return Err(conflict("Device time is incomplete; review after recovery"));
                    };
                    seconds = seconds.saturating_add(
                        ((end - start).num_milliseconds() + 999)
                            .div_euclid(1000)
                            .max(1),
                    );
                }
                let usages: Vec<ModelUsage> =
                    serde_json::from_value(r.usage.clone()).map_err(|_| ApiFailure::internal())?;
                if claim.is_some()
                    && detail.manifest.resolved_model.is_some()
                    && !usages.iter().any(|usage| usage.calls > 0)
                {
                    return Err(conflict(
                    "Provider usage is missing for a model run; review the meter before settlement",
                ));
                }
                for usage in usages {
                    if usage.unknown_calls > 0
                        || (usage.calls > 0
                            && (usage.input_tokens.is_none() || usage.output_tokens.is_none()))
                    {
                        return Err(conflict(
                        "Provider token usage is incomplete; review the meter before settlement",
                    ));
                    }
                    input = input.saturating_add(i64::from(usage.input_tokens.unwrap_or(0)));
                    output = output.saturating_add(i64::from(usage.output_tokens.unwrap_or(0)));
                }
            }
            let attempt_ids = attempts
                .iter()
                .map(|attempt| attempt.id)
                .collect::<Vec<_>>();
            let bytes = if attempt_ids.is_empty() {
                0
            } else {
                execution_artifacts::Entity::find()
                    .filter(execution_artifacts::Column::AttemptId.is_in(attempt_ids))
                    .filter(execution_artifacts::Column::State.eq("sealed"))
                    .all(&tx)
                    .await?
                    .into_iter()
                    .fold(0_i64, |sum, artifact| {
                        sum.saturating_add(artifact.byte_size)
                    })
            };
            let measured = measured_credits(input, output, seconds, bytes)?;
            let held = row.held_credits;
            (
                "settled",
                Some(measured),
                measured.min(held),
                Some(input),
                Some(output),
                Some(seconds),
                Some(bytes),
            )
        };
    billing_credit_periods::Entity::find_by_id(row.period_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let mut active = row.into_active_model();
    active.state = Set(state.into());
    active.measured_credits = Set(measured);
    active.charged_credits = Set(charged);
    active.input_tokens = Set(input);
    active.output_tokens = Set(output);
    active.device_seconds = Set(seconds);
    active.stored_bytes = Set(bytes);
    active.reason = Set(Some(reason.into()));
    active.reviewer_id = Set(Some(actor));
    active.reviewed_at = Set(Some(Utc::now()));
    active.update(&tx).await?;
    tx.commit().await?;
    Ok(true)
}

pub async fn release_queued_cancel(
    db: &impl ConnectionTrait,
    actor: Uuid,
    run: Uuid,
) -> ApiResult<bool> {
    let Some(row) = billing_credit_usage::Entity::find_by_id(run)
        .lock_exclusive()
        .one(db)
        .await?
    else {
        return Ok(false);
    };
    if row.state != "held" {
        return Ok(true);
    }
    let claimed = execution_attempts::Entity::find()
        .filter(execution_attempts::Column::RunId.eq(run))
        .all(db)
        .await?
        .into_iter()
        .any(|attempt| {
            attempt.claim_id.is_some()
                || attempt.state != "finished"
                || attempt.reason.as_deref() != Some("canceled_before_dispatch")
        });
    if !claimed {
        let mut active = row.into_active_model();
        active.state = Set("released".into());
        active.reason = Set(Some("Canceled before device claim".into()));
        active.reviewer_id = Set(Some(actor));
        active.reviewed_at = Set(Some(Utc::now()));
        active.update(db).await?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quote_reserves_preparation_and_cleanup_for_each_possible_attempt() {
        let fixture: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/execution/comparison.json"
        ))
        .unwrap();
        let mut manifest: RunManifest =
            serde_json::from_value(fixture["manifest"].clone()).unwrap();
        manifest.diagnostic_retries = 1;
        let cleanup = i64::from(
            manifest
                .profile
                .execution_context
                .as_ref()
                .unwrap()
                .stages
                .cleanup_seconds,
        ) + 10;
        assert_eq!(
            authorized_device_seconds(&manifest).unwrap(),
            i64::from(manifest.budget.duration_seconds)
                + manifest.cases.len() as i64 * 2 * (1800 + cleanup)
        );
    }
    #[test]
    fn monthly_terms_and_single_rounding() {
        assert_eq!(terms(CreditPlan::Starter), (50_000, 50_000, "starter"));
        assert_eq!(terms(CreditPlan::Plus), (75_000, 75_000, "plus"));
        assert_eq!(terms(CreditPlan::Business), (150_000, 150_000, "business"));
        assert_eq!(measured_credits(1_000_000, 1_000_000, 60, 0).unwrap(), 1060);
        assert_eq!(measured_credits(1, 1, 0, 1).unwrap(), 1);
    }
    #[test]
    fn signed_webhook_requires_matching_digest_and_recent_timestamp() {
        let payload = br#"{"type":"invoice.paid"}"#;
        let stamp = Utc::now().timestamp();
        let signed = format!("{stamp}.{}", std::str::from_utf8(payload).unwrap());
        let mut mac = Hmac::<Sha256>::new_from_slice(b"whsec_test").unwrap();
        mac.update(signed.as_bytes());
        let signature = format!("t={stamp},v1={}", hex::encode(mac.finalize().into_bytes()));
        assert!(verify_signature(payload, &signature, "whsec_test").is_ok());
        assert!(
            verify_signature(br#"{"type":"invoice.failed"}"#, &signature, "whsec_test").is_err()
        );
        assert!(verify_signature(payload, &signature, "whsec_other").is_err());
        assert!(verify_signature(payload, "t=-9223372036854775808,v1=00", "whsec_test").is_err());
    }
}
