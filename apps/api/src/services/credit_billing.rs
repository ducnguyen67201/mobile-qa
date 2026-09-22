//! Stripe-confirmed monthly grants and bounded, measured run consumption.
//! Checkout redirects never grant credits; only a verified paid invoice does.
use super::{apps, execution_store::*, runs};
use crate::{
    config::Setup,
    errors::{ApiFailure, ApiResult},
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
use sea_orm::{ConnectionTrait, TransactionTrait};
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
    let sql = if lock {
        "SELECT id,plan,granted_credits,starts_at,ends_at,rate_revision FROM billing_credit_periods WHERE app_id=$1 AND starts_at<=now() AND ends_at>now() ORDER BY starts_at DESC LIMIT 1 FOR UPDATE"
    } else {
        "SELECT id,plan,granted_credits,starts_at,ends_at,rate_revision FROM billing_credit_periods WHERE app_id=$1 AND starts_at<=now() AND ends_at>now() ORDER BY starts_at DESC LIMIT 1"
    };
    rows(db, sql, vec![app.into()])
        .await?
        .into_iter()
        .next()
        .map(|r| {
            Ok(Period {
                id: field(&r, "id")?,
                plan: parse_plan(&field::<String>(&r, "plan")?)?,
                grant: field(&r, "granted_credits")?,
                starts_at: field(&r, "starts_at")?,
                ends_at: field(&r, "ends_at")?,
                rate_revision: field(&r, "rate_revision")?,
            })
        })
        .transpose()
}

async fn balance(db: &impl ConnectionTrait, p: &Period) -> ApiResult<(i64, i64, i64)> {
    let r = one(db, "SELECT COALESCE(sum(CASE WHEN state='settled' THEN charged_credits ELSE 0 END),0)::bigint AS charged, COALESCE(sum(CASE WHEN state='held' THEN held_credits ELSE 0 END),0)::bigint AS held FROM billing_credit_usage WHERE period_id=$1", vec![p.id.into()]).await?;
    let charged: i64 = field(&r, "charged")?;
    let held: i64 = field(&r, "held")?;
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
    let pending = one(db, "SELECT i.pending_plan,i.pending_effective_at FROM billing_checkout_intents i JOIN billing_credit_periods p ON p.checkout_intent_id=i.id WHERE p.id=$1", vec![p.id.into()]).await?;
    let pending_plan = field::<Option<String>>(&pending, "pending_plan")?
        .map(|value| parse_plan(&value))
        .transpose()?;
    let pending_effective_at = field(&pending, "pending_effective_at")?;
    let mut usage = Vec::new();
    for r in rows(db, "SELECT run_id,state,held_credits,measured_credits,charged_credits,input_tokens,output_tokens,device_seconds,stored_bytes,reason FROM billing_credit_usage WHERE period_id=$1 ORDER BY created_at DESC,run_id DESC LIMIT 100", vec![p.id.into()]).await? {
        usage.push(CreditUsageView {
            run_id: field(&r,"run_id")?, state: field(&r,"state")?,
            held_credits: i32::try_from(field::<i64>(&r,"held_credits")?).map_err(|_| ApiFailure::internal())?,
            measured_credits: field::<Option<i64>>(&r,"measured_credits")?.map(|v|v.to_string()),
            charged_credits: i32::try_from(field::<i64>(&r,"charged_credits")?).map_err(|_| ApiFailure::internal())?,
            input_tokens: field::<Option<i64>>(&r,"input_tokens")?.map(|v|v.to_string()),
            output_tokens: field::<Option<i64>>(&r,"output_tokens")?.map(|v|v.to_string()),
            device_seconds: field::<Option<i64>>(&r,"device_seconds")?
                .map(i32::try_from).transpose().map_err(|_| ApiFailure::internal())?,
            stored_bytes: field::<Option<i64>>(&r,"stored_bytes")?.map(|v|v.to_string()),
            reason: field(&r,"reason")?,
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
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![app.into()],
    )
    .await?;
    let p = period(&tx, app, false)
        .await?
        .ok_or_else(|| conflict("A paid monthly allowance is required to change plans"))?;
    let row = one(&tx, "SELECT i.id,i.plan,i.pending_plan,i.pending_effective_at,i.stripe_subscription_id,i.stripe_schedule_id FROM billing_checkout_intents i JOIN billing_credit_periods p ON p.checkout_intent_id=i.id WHERE p.id=$1 AND i.state='paid' FOR UPDATE OF i", vec![p.id.into()]).await?;
    let intent_id: Uuid = field(&row, "id")?;
    let current = parse_plan(&field::<String>(&row, "plan")?)?;
    if current != p.plan {
        return Err(conflict("The current paid period needs billing review"));
    }
    let pending = field::<Option<String>>(&row, "pending_plan")?
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
    let sid = field::<String>(&row, "stripe_subscription_id")?;
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
    let stored_schedule: Option<String> = field(&row, "stripe_schedule_id")?;
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
        exec(&tx, "UPDATE billing_checkout_intents SET pending_plan=NULL,pending_effective_at=NULL,stripe_schedule_id=NULL WHERE id=$1", vec![intent_id.into()]).await?;
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
            exec(&tx, "UPDATE billing_checkout_intents SET pending_plan=$2,pending_effective_at=$3,stripe_schedule_id=$4 WHERE id=$1", vec![intent_id.into(),terms(target).2.into(),p.ends_at.into(),schedule_id.into()]).await?;
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
    exec(&tx, "UPDATE billing_checkout_intents SET pending_plan=$2,pending_effective_at=$3,stripe_schedule_id=$4 WHERE id=$1", vec![intent_id.into(),target_word.into(),p.ends_at.into(),schedule_id.into()]).await?;
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
    let app_row = one(
        &tx,
        "SELECT id,organization_id FROM apps WHERE id=$1 FOR UPDATE",
        vec![app.into()],
    )
    .await?;
    let workspace: Uuid = field(&app_row, "organization_id")?;
    if period(&tx, app, false).await?.is_some() {
        return Err(conflict("This app already has an active monthly allowance"));
    }
    if !rows(
        &tx,
        "SELECT id FROM billing_checkout_intents WHERE app_id=$1 AND state='paid' LIMIT 1",
        vec![app.into()],
    )
    .await?
    .is_empty()
    {
        return Err(conflict(
            "An existing subscription owns this app; avoid creating a second checkout",
        ));
    }
    if !rows(&tx,"SELECT id FROM commercial_agreements WHERE app_id=$1 AND status='active' AND ends_at>now()",vec![app.into()]).await?.is_empty() {
        return Err(conflict("This app already has an active check agreement"));
    }
    let open = rows(&tx, "SELECT id,plan,stripe_session_id,stripe_session_url,created_at FROM billing_checkout_intents WHERE app_id=$1 AND state IN ('pending','checkout')", vec![app.into()]).await?;
    let reusable = if let Some(r) = open.first() {
        let existing_id: Uuid = field(r, "id")?;
        let session_id: Option<String> = field(r, "stripe_session_id")?;
        if Utc::now() - field::<DateTime<Utc>>(r, "created_at")? >= chrono::Duration::hours(24) {
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
                exec(
                    &tx,
                    "UPDATE billing_checkout_intents SET state='expired' WHERE id=$1",
                    vec![existing_id.into()],
                )
                .await?;
                None
            } else {
                return Err(conflict("Checkout or payment is still being confirmed"));
            }
        } else {
            if field::<String>(r, "plan")? != word {
                return Err(conflict(
                    "Finish the existing checkout before choosing another plan",
                ));
            }
            if let Some(url) = field::<Option<String>>(r, "stripe_session_url")? {
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
        exec(
            &tx,
            "INSERT INTO billing_checkout_intents(id,app_id,actor_id,plan) VALUES($1,$2,$3,$4)",
            vec![id.into(), app.into(), actor.into(), word.into()],
        )
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
    exec(&ctx.db, "UPDATE billing_checkout_intents SET state='checkout',stripe_session_id=$2,stripe_session_url=$3 WHERE id=$1 AND state='pending'", vec![id.into(),session.into(),url.into()]).await?;
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
    let app_row = one(
        &tx,
        "SELECT app_id FROM billing_checkout_intents WHERE id=$1",
        vec![intent_id.into()],
    )
    .await?;
    let app: Uuid = field(&app_row, "app_id")?;
    // All billing mutations lock the app before the intent, including checkout
    // and plan changes. Keeping the same order avoids renewal deadlocks.
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![app.into()],
    )
    .await?;
    let row = one(
        &tx,
        "SELECT app_id,plan,pending_plan,pending_effective_at,stripe_subscription_id FROM billing_checkout_intents WHERE id=$1 FOR UPDATE",
        vec![intent_id.into()],
    )
    .await?;
    if field::<Uuid>(&row, "app_id")? != app {
        return Err(ApiFailure::internal());
    }
    let recorded_subscription: Option<String> = field(&row, "stripe_subscription_id")?;
    if recorded_subscription
        .as_deref()
        .is_some_and(|id| id != subscription_id)
    {
        return Err(ApiFailure::internal());
    }
    // An old invoice can be replayed after Stripe has advanced to a new price.
    // Its already verified grant stays immutable and never applies a second time.
    let existing = rows(&tx, "SELECT stripe_invoice_id,stripe_subscription_id,app_id FROM billing_credit_periods WHERE stripe_invoice_id=$1 OR (stripe_subscription_id=$2 AND starts_at=$3)", vec![invoice_id.into(),subscription_id.into(),start.into()]).await?;
    if let Some(grant) = existing.first() {
        if field::<String>(grant, "stripe_invoice_id")? == invoice_id
            && field::<String>(grant, "stripe_subscription_id")? == subscription_id
            && field::<Uuid>(grant, "app_id")? == app
        {
            tx.commit().await?;
            return Ok(());
        }
        return Err(ApiFailure::internal());
    }
    let current = parse_plan(&field::<String>(&row, "plan")?)?;
    let pending = field::<Option<String>>(&row, "pending_plan")?
        .map(|value| parse_plan(&value))
        .transpose()?;
    let effective: Option<DateTime<Utc>> = field(&row, "pending_effective_at")?;
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
    exec(&tx, "INSERT INTO billing_credit_periods(id,app_id,checkout_intent_id,stripe_subscription_id,stripe_invoice_id,plan,granted_credits,starts_at,ends_at,rate_revision) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)", vec![Uuid::new_v4().into(),app.into(),intent_id.into(),subscription_id.into(),invoice_id.into(),word.into(),credits.into(),start.into(),end.into(),RATE_REVISION.into()]).await?;
    exec(
        &tx,
        "UPDATE billing_checkout_intents SET state='paid',stripe_subscription_id=$2,plan=$3,pending_plan=CASE WHEN $4 THEN NULL ELSE pending_plan END,pending_effective_at=CASE WHEN $4 THEN NULL ELSE pending_effective_at END,stripe_schedule_id=CASE WHEN $4 THEN NULL ELSE stripe_schedule_id END WHERE id=$1",
        vec![intent_id.into(), subscription_id.into(),word.into(),switching.into()],
    )
    .await?;
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

fn maximum(manifest: &RunManifest) -> ApiResult<i64> {
    let seconds = i64::from(manifest.budget.duration_seconds);
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
    exec(&ctx.db,"INSERT INTO billing_credit_quotes(id,period_id,app_id,actor_id,kind,request_hash,manifest_hash,build_id,source_version_id,profile_id,environment_revision,rate_revision,max_credits,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",
        vec![id.into(),p.id.into(),app.into(),actor.into(),kind.into(),request_hash.into(),manifest_hash.into(),build.into(),source.into(),manifest.profile.id.into(),environment_revision.into(),p.rate_revision.into(),max.into(),expiry.into()]).await?;
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
    let Some(q)=rows(db,"SELECT period_id,app_id,actor_id,kind,request_hash,manifest_hash,build_id,source_version_id,profile_id,environment_revision,rate_revision,max_credits,expires_at FROM billing_credit_quotes WHERE id=$1",vec![quote_id.into()]).await?.into_iter().next() else { return Ok(false); };
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
    let max: i64 = field(&q, "max_credits")?;
    if field::<Uuid>(&q, "period_id")? != p.id
        || field::<Uuid>(&q, "app_id")? != app
        || field::<Uuid>(&q, "actor_id")? != actor
        || field::<String>(&q, "kind")? != kind
        || field::<String>(&q, "request_hash")? != request_hash
        || field::<String>(&q, "manifest_hash")? != digest
        || field::<Uuid>(&q, "build_id")? != manifest.build_id
        || field::<Uuid>(&q, "source_version_id")? != source
        || field::<Uuid>(&q, "profile_id")? != manifest.profile.id
        || field::<i32>(&q, "environment_revision")? != manifest.environment_revision
        || field::<i32>(&q, "rate_revision")? != p.rate_revision
        || field::<DateTime<Utc>>(&q, "expires_at")? <= Utc::now()
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
    exec(db,"INSERT INTO billing_credit_usage(run_id,quote_id,period_id,app_id,held_credits) VALUES($1,$2,$3,$4,$5)",
        vec![run_id.into(),quote_id.into(),p.id.into(),app.into(),max.into()]).await?;
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
    let Some(row) = rows(
        &tx,
        "SELECT period_id,held_credits,state FROM billing_credit_usage WHERE run_id=$1 FOR UPDATE",
        vec![run.into()],
    )
    .await?
    .into_iter()
    .next() else {
        return Ok(false);
    };
    if field::<String>(&row, "state")? != "held" {
        return Err(conflict("This run was already reviewed"));
    }
    let detail = runs::detail(&tx, run).await?;
    if detail.state != JobState::Finished {
        return Err(conflict("Finish this run before reviewing its usage"));
    }
    let (state, measured, charged, input, output, seconds, bytes) = if decision
        == CommercialUsageState::Credited
    {
        ("released", None, 0, None, None, None, None)
    } else {
        let mut input = 0_i64;
        let mut output = 0_i64;
        let mut seconds = 0_i64;
        for r in rows(
            &tx,
            "SELECT claim_id,claimed_at,released_at,usage FROM execution_attempts WHERE run_id=$1",
            vec![run.into()],
        )
        .await?
        {
            let claim: Option<Uuid> = field(&r, "claim_id")?;
            if claim.is_some() {
                let start: Option<DateTime<Utc>> = field(&r, "claimed_at")?;
                let end: Option<DateTime<Utc>> = field(&r, "released_at")?;
                let (Some(start), Some(end)) = (start, end) else {
                    return Err(conflict("Device time is incomplete; review after recovery"));
                };
                seconds = seconds.saturating_add(
                    ((end - start).num_milliseconds() + 999)
                        .div_euclid(1000)
                        .max(1),
                );
            }
            let value: Value = field(&r, "usage")?;
            let usages: Vec<ModelUsage> =
                serde_json::from_value(value).map_err(|_| ApiFailure::internal())?;
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
        let r=one(&tx,"SELECT COALESCE(sum(f.byte_size),0)::bigint AS bytes FROM execution_artifacts f JOIN execution_attempts a ON a.id=f.attempt_id WHERE a.run_id=$1 AND f.state='sealed'",vec![run.into()]).await?;
        let bytes: i64 = field(&r, "bytes")?;
        let measured = measured_credits(input, output, seconds, bytes)?;
        let held: i64 = field(&row, "held_credits")?;
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
    let period: Uuid = field(&row, "period_id")?;
    one(
        &tx,
        "SELECT id FROM billing_credit_periods WHERE id=$1 FOR UPDATE",
        vec![period.into()],
    )
    .await?;
    exec(&tx,"UPDATE billing_credit_usage SET state=$2,measured_credits=$3,charged_credits=$4,input_tokens=$5,output_tokens=$6,device_seconds=$7,stored_bytes=$8,reason=$9,reviewer_id=$10,reviewed_at=now() WHERE run_id=$1",
        vec![run.into(),state.into(),measured.into(),charged.into(),input.into(),output.into(),seconds.into(),bytes.into(),reason.into(),actor.into()]).await?;
    tx.commit().await?;
    Ok(true)
}

pub async fn release_queued_cancel(
    db: &impl ConnectionTrait,
    actor: Uuid,
    run: Uuid,
) -> ApiResult<bool> {
    let Some(row) = rows(
        db,
        "SELECT period_id,state FROM billing_credit_usage WHERE run_id=$1 FOR UPDATE",
        vec![run.into()],
    )
    .await?
    .into_iter()
    .next() else {
        return Ok(false);
    };
    if field::<String>(&row, "state")? != "held" {
        return Ok(true);
    }
    let claimed=rows(db,"SELECT id FROM execution_attempts WHERE run_id=$1 AND (claim_id IS NOT NULL OR state<>'finished' OR reason IS DISTINCT FROM 'canceled_before_dispatch') LIMIT 1",vec![run.into()]).await?;
    if claimed.is_empty() {
        exec(db,"UPDATE billing_credit_usage SET state='released',reason='Canceled before device claim',reviewer_id=$2,reviewed_at=now() WHERE run_id=$1",vec![run.into(),actor.into()]).await?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
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
