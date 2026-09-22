//! Commercial access is app-scoped and only the trusted process can change agreements.
//! A quote binds a visible price to an exact run manifest; reservation shares its transaction.
use super::{
    apps, credit_billing, execution_store::*, runs, suite_runs, test_definitions, test_library,
};
use crate::errors::{ApiFailure, ApiResult};
use chrono::{DateTime, Duration, Utc};
use loco_rs::app::AppContext;
use mobile_qa_contracts::{
    commercial::*,
    execution::{CreateRunRequest, Driver, RunManifest, TestDefinition},
    regression::SuiteRunRequest,
};
use sea_orm::{ConnectionTrait, TransactionTrait};
use uuid::Uuid;

const PILOT_BASE_CENTS: i32 = 50_000;
const RECURRING_BASE_CENTS: i32 = 25_000;
const SECOND_SUITE_CENTS: i32 = 25_000;
const CHECK_CENTS: i32 = 12_500;

#[derive(Clone)]
struct Agreement {
    id: Uuid,
    offer: CommercialOffer,
    status: String,
    price_revision: i32,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
    first_source: Uuid,
    second_source: Option<Uuid>,
    profile_id: Uuid,
    base_cents: i32,
    second_suite_cents: i32,
    check_cents: i32,
    check_cap: i32,
}

fn commercial_conflict(message: &str) -> ApiFailure {
    ApiFailure::new(409, "commercial_conflict", message)
}

async fn latest(db: &impl ConnectionTrait, app: Uuid) -> ApiResult<Option<Agreement>> {
    let Some(row) = rows(db,"SELECT id,offer,status,price_revision,starts_at,ends_at,first_source_version_id,second_source_version_id,profile_id,base_cents,second_suite_cents,check_cents,check_cap FROM commercial_agreements WHERE app_id=$1 ORDER BY created_at DESC,id DESC LIMIT 1",vec![app.into()]).await?.into_iter().next() else {
        return Ok(None);
    };
    let offer: String = field(&row, "offer")?;
    Ok(Some(Agreement {
        id: field(&row, "id")?,
        offer: match offer.as_str() {
            "pilot" => CommercialOffer::Pilot,
            "recurring" => CommercialOffer::Recurring,
            _ => return Err(ApiFailure::internal()),
        },
        status: field(&row, "status")?,
        price_revision: field(&row, "price_revision")?,
        starts_at: field(&row, "starts_at")?,
        ends_at: field(&row, "ends_at")?,
        first_source: field(&row, "first_source_version_id")?,
        second_source: field(&row, "second_source_version_id")?,
        profile_id: field(&row, "profile_id")?,
        base_cents: field(&row, "base_cents")?,
        second_suite_cents: field(&row, "second_suite_cents")?,
        check_cents: field(&row, "check_cents")?,
        check_cap: field(&row, "check_cap")?,
    }))
}

fn state(agreement: Option<&Agreement>, now: DateTime<Utc>) -> CommercialState {
    match agreement {
        None => CommercialState::Uncontracted,
        Some(a) if a.status == "paused" || a.status == "ended" => CommercialState::Paused,
        Some(a) if now < a.starts_at || now >= a.ends_at => CommercialState::Expired,
        Some(_) => CommercialState::Active,
    }
}

async fn active(db: &impl ConnectionTrait, app: Uuid) -> ApiResult<Agreement> {
    let a = latest(db, app)
        .await?
        .ok_or_else(|| commercial_conflict("Request a pilot before starting device work"))?;
    if state(Some(&a), Utc::now()) != CommercialState::Active {
        return Err(commercial_conflict(
            "Commercial access is not active for this app",
        ));
    }
    Ok(a)
}

async fn used(db: &impl ConnectionTrait, agreement: Uuid) -> ApiResult<i32> {
    let row = one(db,"SELECT COUNT(*)::integer AS n FROM commercial_usage WHERE agreement_id=$1 AND state IN ('reserved','delivered')",vec![agreement.into()]).await?;
    field(&row, "n")
}

pub async fn status(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
) -> ApiResult<CommercialAccessResponse> {
    apps::authorized(ctx, actor, app).await?;
    let agreement = latest(&ctx.db, app).await?;
    let pilot_request_id = rows(
        &ctx.db,
        "SELECT id FROM commercial_pilot_requests WHERE app_id=$1",
        vec![app.into()],
    )
    .await?
    .first()
    .map(|row| field(row, "id"))
    .transpose()?;
    let mut usage = Vec::new();
    let (reserved_checks, delivered_checks, credited_checks, delivered_check_cents) = if let Some(
        a,
    ) =
        &agreement
    {
        let totals = one(&ctx.db, "SELECT COUNT(*) FILTER (WHERE state='reserved')::integer AS reserved, COUNT(*) FILTER (WHERE state='delivered')::integer AS delivered, COUNT(*) FILTER (WHERE state='credited')::integer AS credited, COALESCE(SUM(amount_cents) FILTER (WHERE state='delivered'),0)::integer AS delivered_cents FROM commercial_usage WHERE agreement_id=$1", vec![a.id.into()]).await?;
        for row in rows(&ctx.db,"SELECT run_id,state,amount_cents,reason,created_at,reviewed_at FROM commercial_usage WHERE agreement_id=$1 ORDER BY created_at DESC,run_id DESC LIMIT 100",vec![a.id.into()]).await? {
            let word: String = field(&row,"state")?;
            let current = match word.as_str() {
                "reserved" => CommercialUsageState::Reserved,
                "delivered" => CommercialUsageState::Delivered,
                "credited" => CommercialUsageState::Credited,
                _ => return Err(ApiFailure::internal()),
            };
            let amount: i32 = field(&row,"amount_cents")?;
            usage.push(CommercialUsageView { run_id:field(&row,"run_id")?, state:current, amount_cents:amount,
                reason:field(&row,"reason")?, created_at:field(&row,"created_at")?, reviewed_at:field(&row,"reviewed_at")? });
        }
        (
            field(&totals, "reserved")?,
            field(&totals, "delivered")?,
            field(&totals, "credited")?,
            field(&totals, "delivered_cents")?,
        )
    } else {
        (0, 0, 0, 0)
    };
    let current_state = state(agreement.as_ref(), Utc::now());
    let credit = credit_billing::status(&ctx.db, app).await?;
    Ok(CommercialAccessResponse {
        app_id: app,
        state: if credit.is_some() {
            CommercialState::Active
        } else {
            current_state
        },
        plans: credit_billing::plans(),
        credit,
        agreement: agreement.map(|a| CommercialAgreementView {
            id: a.id,
            offer: a.offer,
            starts_at: a.starts_at,
            ends_at: a.ends_at,
            first_source_version_id: a.first_source,
            second_source_version_id: a.second_source,
            profile_id: a.profile_id,
            base_cents: a.base_cents,
            second_suite_cents: a.second_suite_cents,
            check_cents: a.check_cents,
            check_cap: a.check_cap,
            currency: "USD".into(),
            price_revision: a.price_revision,
        }),
        pilot_request_id,
        reserved_checks,
        delivered_checks,
        credited_checks,
        delivered_check_cents,
        usage,
    })
}

pub async fn request_pilot(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    input: CommercialPilotRequest,
) -> ApiResult<CommercialPilotResponse> {
    apps::authorized(ctx, actor, app).await?;
    let note = input.coverage_note.trim();
    if note.is_empty() || note.chars().count() > 500 {
        return Err(ApiFailure::invalid(
            "Describe the app and coverage in 1–500 characters",
        ));
    }
    let tx = ctx.db.begin().await?;
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![app.into()],
    )
    .await?;
    if latest(&tx, app).await?.is_some() {
        return Err(commercial_conflict("This app already has an agreement"));
    }
    if !rows(
        &tx,
        "SELECT id FROM commercial_pilot_requests WHERE app_id=$1",
        vec![app.into()],
    )
    .await?
    .is_empty()
    {
        return Err(commercial_conflict(
            "A pilot request is already recorded for this app",
        ));
    }
    let id = Uuid::new_v4();
    exec(&tx,"INSERT INTO commercial_pilot_requests(id,app_id,actor_id,coverage_note) VALUES($1,$2,$3,$4)",vec![id.into(),app.into(),actor.into(),note.to_owned().into()]).await?;
    tx.commit().await?;
    Ok(CommercialPilotResponse { id, app_id: app })
}

pub fn plan_hash(actor: Uuid, input: &CreateRunRequest) -> ApiResult<String> {
    Ok(hash(
        serde_json::to_vec(&(actor, "release_plan", input)).map_err(|_| ApiFailure::internal())?,
    ))
}

pub fn suite_hash(actor: Uuid, input: &SuiteRunRequest) -> ApiResult<String> {
    Ok(hash(
        serde_json::to_vec(&(actor, "saved_suite", input)).map_err(|_| ApiFailure::internal())?,
    ))
}

pub async fn quote(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    input: CommercialQuoteRequest,
) -> ApiResult<CommercialQuoteResponse> {
    apps::authorized(ctx, actor, app).await?;
    let (kind, source, build, requested_revision, request_hash, preview) = match input.intent {
        CommercialRunIntent::ReleasePlan(request) => {
            let preview = runs::preview(
                &ctx.db,
                app,
                request.build_id,
                Some(request.plan_version_id),
            )
            .await?;
            (
                "release_plan",
                request.plan_version_id,
                request.build_id,
                request.environment_revision,
                plan_hash(actor, &request)?,
                preview,
            )
        }
        CommercialRunIntent::SavedSuite(request) => {
            let preview = suite_runs::commercial_manifest(&ctx.db, app, &request).await?;
            (
                "saved_suite",
                request.suite_version_id,
                request.build_id,
                request.environment_revision,
                suite_hash(actor, &request)?,
                preview,
            )
        }
    };
    if !preview.blockers.is_empty() {
        return Err(commercial_conflict(
            "Resolve run readiness before requesting a price",
        ));
    }
    let manifest = preview.manifest.ok_or_else(ApiFailure::internal)?;
    if manifest.cases.is_empty()
        || manifest.cases.len() > 10
        || manifest.budget.duration_seconds > 1800
    {
        return Err(commercial_conflict(
            "A check is limited to ten cases and 30 minutes",
        ));
    }
    if matches!(
        ctx.environment,
        loco_rs::environment::Environment::Production
    ) && matches!(&manifest.profile.driver, Driver::Fake)
    {
        return Err(commercial_conflict(
            "A qualified Android device is required",
        ));
    }
    if manifest.build_id != build
        || manifest.app_id != app
        || manifest.environment_revision != requested_revision
    {
        return Err(commercial_conflict(
            "Run setup changed; refresh before requesting a price",
        ));
    }
    if let Some(quote) = credit_billing::quote(
        ctx,
        actor,
        app,
        kind,
        source,
        build,
        requested_revision,
        &request_hash,
        &manifest,
    )
    .await?
    {
        return Ok(quote);
    }
    let agreement = active(&ctx.db, app).await?;
    if source != agreement.first_source && Some(source) != agreement.second_source {
        return Err(commercial_conflict(
            "This saved coverage is not in the agreed scope",
        ));
    }
    if manifest.profile.id != agreement.profile_id {
        return Err(commercial_conflict(
            "This device profile is outside the agreed scope",
        ));
    }
    let count = used(&ctx.db, agreement.id).await?;
    if count >= agreement.check_cap {
        return Err(commercial_conflict(
            "The agreed check limit has been reached",
        ));
    }
    let amount = agreement.check_cents;
    let id = Uuid::new_v4();
    let expiry = Utc::now() + Duration::minutes(10);
    let manifest_hash = hash(serde_json::to_vec(&manifest).map_err(|_| ApiFailure::internal())?);
    exec(&ctx.db,"INSERT INTO commercial_quotes(id,agreement_id,app_id,actor_id,kind,request_hash,manifest_hash,build_id,source_version_id,profile_id,environment_revision,price_revision,amount_cents,expires_at) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",vec![id.into(),agreement.id.into(),app.into(),actor.into(),kind.into(),request_hash.into(),manifest_hash.into(),build.into(),source.into(),manifest.profile.id.into(),manifest.environment_revision.into(),agreement.price_revision.into(),amount.into(),expiry.into()]).await?;
    Ok(CommercialQuoteResponse {
        id,
        app_id: app,
        kind: kind.into(),
        build_id: build,
        source_version_id: source,
        profile_id: manifest.profile.id,
        case_count: manifest.cases.len() as i32,
        amount_cents: amount,
        maximum_credits: None,
        credits_after_authorization: None,
        currency: "USD".into(),
        checks_after_authorization: count + 1,
        check_cap: agreement.check_cap,
        expires_at: expiry,
    })
}

/// Called only inside an existing app-locked run-creation transaction.
// The reservation call keeps the quote, request and manifest identities explicit.
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
) -> ApiResult<()> {
    if credit_billing::reserve(
        db,
        actor,
        app,
        quote_id,
        kind,
        request_hash,
        manifest,
        run_id,
    )
    .await?
    {
        return Ok(());
    }
    let a = active(db, app).await?;
    let q = one(db,"SELECT agreement_id,app_id,actor_id,kind,request_hash,manifest_hash,build_id,source_version_id,profile_id,environment_revision,price_revision,amount_cents,expires_at FROM commercial_quotes WHERE id=$1",vec![quote_id.into()]).await?;
    let expected_source = match &manifest.source {
        Some(mobile_qa_contracts::regression::RunSource::SavedSuiteV1 { suite_version_id }) => {
            *suite_version_id
        }
        Some(mobile_qa_contracts::regression::RunSource::ReleasePlanV1 { plan_version_id }) => {
            *plan_version_id
        }
        _ => manifest.plan_version_id.ok_or_else(ApiFailure::internal)?,
    };
    let digest = hash(serde_json::to_vec(manifest).map_err(|_| ApiFailure::internal())?);
    if field::<Uuid>(&q, "agreement_id")? != a.id
        || field::<Uuid>(&q, "app_id")? != app
        || field::<Uuid>(&q, "actor_id")? != actor
        || field::<String>(&q, "kind")? != kind
        || field::<String>(&q, "request_hash")? != request_hash
        || field::<String>(&q, "manifest_hash")? != digest
        || field::<Uuid>(&q, "build_id")? != manifest.build_id
        || field::<Uuid>(&q, "source_version_id")? != expected_source
        || field::<Uuid>(&q, "profile_id")? != manifest.profile.id
        || manifest.profile.id != a.profile_id
        || field::<i32>(&q, "environment_revision")? != manifest.environment_revision
        || field::<i32>(&q, "price_revision")? != a.price_revision
        || field::<i32>(&q, "amount_cents")? != a.check_cents
        || field::<DateTime<Utc>>(&q, "expires_at")? <= Utc::now()
    {
        return Err(commercial_conflict(
            "Price or run setup changed; review a new quote",
        ));
    }
    if !rows(
        db,
        "SELECT run_id FROM commercial_usage WHERE quote_id=$1",
        vec![quote_id.into()],
    )
    .await?
    .is_empty()
    {
        return Err(commercial_conflict("This quote has already been used"));
    }
    if used(db, a.id).await? >= a.check_cap {
        return Err(commercial_conflict(
            "The agreed check limit has been reached",
        ));
    }
    exec(db,"INSERT INTO commercial_usage(run_id,quote_id,agreement_id,app_id,period_start,period_end,amount_cents) VALUES($1,$2,$3,$4,$5,$6,$7)",vec![run_id.into(),quote_id.into(),a.id.into(),app.into(),a.starts_at.into(),a.ends_at.into(),a.check_cents.into()]).await?;
    Ok(())
}

/// Existing integration fixtures predate commercial contracts. Their test environment
/// may run without an agreement; any explicitly contracted test app follows the real gate.
// The test-only branch preserves legacy fixture runs without weakening production access.
#[allow(clippy::too_many_arguments)]
pub async fn reserve_or_test_fixture(
    ctx: &AppContext,
    db: &impl ConnectionTrait,
    actor: Uuid,
    app: Uuid,
    quote_id: Option<Uuid>,
    kind: &str,
    request_hash: &str,
    manifest: &RunManifest,
    run_id: Uuid,
) -> ApiResult<()> {
    match quote_id {
        Some(id) => reserve(db, actor, app, id, kind, request_hash, manifest, run_id).await,
        None if matches!(ctx.environment, loco_rs::environment::Environment::Test)
            && latest(db, app).await?.is_none() =>
        {
            Ok(())
        }
        None => Err(commercial_conflict(
            "Review and authorize a price before running this check",
        )),
    }
}

pub async fn require_separate_scope(ctx: &AppContext, actor: Uuid, app: Uuid) -> ApiResult<()> {
    apps::authorized(ctx, actor, app).await?;
    if matches!(ctx.environment, loco_rs::environment::Environment::Test)
        && latest(&ctx.db, app).await?.is_none()
    {
        return Ok(());
    }
    Err(commercial_conflict(
        "This action needs separately agreed operator scope",
    ))
}

// Agreement activation is a process-only operation with each commercial term supplied explicitly.
#[allow(clippy::too_many_arguments)]
pub async fn activate(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    offer: CommercialOffer,
    starts_at: DateTime<Utc>,
    ends_at: DateTime<Utc>,
    first: Uuid,
    second: Option<Uuid>,
    profile: Uuid,
    reference: &str,
    reason: &str,
) -> ApiResult<Uuid> {
    if reference.trim().is_empty()
        || reference.len() > 128
        || reason.trim().is_empty()
        || reason.len() > 500
    {
        return Err(ApiFailure::invalid(
            "Agreement reference and reason are required",
        ));
    }
    let days = (ends_at - starts_at).num_days();
    if ends_at <= starts_at
        || (offer == CommercialOffer::Pilot && ends_at - starts_at != Duration::days(14))
        || (offer == CommercialOffer::Recurring && !(28..=32).contains(&days))
    {
        return Err(ApiFailure::invalid(
            "Use a 14-day pilot or an agreed 28–32 day recurring period",
        ));
    }
    if offer == CommercialOffer::Pilot && second.is_some() {
        return Err(ApiFailure::invalid("A pilot has one agreed source"));
    }
    apps::authorized(ctx, actor, app).await?;
    if !test_definitions::profile(&ctx.db, app, profile)
        .await?
        .qualified
    {
        return Err(ApiFailure::invalid(
            "The agreed device profile must be qualified",
        ));
    }
    for source in [Some(first), second].into_iter().flatten() {
        test_library::admitted(&ctx.db, app, source).await?;
        let row = one(
            &ctx.db,
            "SELECT kind FROM execution_definitions WHERE app_id=$1 AND id=$2",
            vec![app.into(), source.into()],
        )
        .await?;
        let kind: String = field(&row, "kind")?;
        if (source == first && kind != "suite" && kind != "plan")
            || (Some(source) == second && kind != "suite")
        {
            return Err(ApiFailure::invalid(
                "Agreement coverage must be a first suite/plan and optional second suite",
            ));
        }
        if source == first && kind == "plan" {
            let definition = test_definitions::get(&ctx.db, app, source).await?;
            let TestDefinition::Plan(plan) = definition.definition else {
                return Err(ApiFailure::internal());
            };
            if plan.profile_id != profile {
                return Err(ApiFailure::invalid(
                    "The agreed profile must match the release plan",
                ));
            }
        }
    }
    let tx = ctx.db.begin().await?;
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![app.into()],
    )
    .await?;
    if !rows(&tx,"SELECT id FROM commercial_agreements WHERE app_id=$1 AND tstzrange(starts_at,ends_at,'[)') && tstzrange($2::timestamptz,$3::timestamptz,'[)') LIMIT 1",vec![app.into(),starts_at.into(),ends_at.into()]).await?.is_empty() {
        return Err(commercial_conflict("Agreement periods must not overlap"));
    }
    if !rows(
        &tx,
        "SELECT id FROM commercial_agreements WHERE app_id=$1 AND status='active'",
        vec![app.into()],
    )
    .await?
    .is_empty()
    {
        return Err(commercial_conflict(
            "Pause or end the current agreement before activating another",
        ));
    }
    let id = Uuid::new_v4();
    let (base, second_cents, check, cap) = if offer == CommercialOffer::Pilot {
        (PILOT_BASE_CENTS, 0, 0, 4)
    } else {
        (
            RECURRING_BASE_CENTS,
            if second.is_some() {
                SECOND_SUITE_CENTS
            } else {
                0
            },
            CHECK_CENTS,
            8,
        )
    };
    exec(&tx,"INSERT INTO commercial_agreements(id,app_id,offer,status,price_revision,starts_at,ends_at,first_source_version_id,second_source_version_id,profile_id,base_cents,second_suite_cents,check_cents,check_cap,agreement_reference,created_by) VALUES($1,$2,$3,'active',1,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)",vec![id.into(),app.into(),if offer==CommercialOffer::Pilot{"pilot".into()}else{"recurring".into()},starts_at.into(),ends_at.into(),first.into(),second.into(),profile.into(),base.into(),second_cents.into(),check.into(),cap.into(),reference.to_owned().into(),actor.into()]).await?;
    exec(&tx,"INSERT INTO commercial_audit(id,agreement_id,actor_id,action,reason) VALUES($1,$2,$3,'activated',$4)",vec![Uuid::new_v4().into(),id.into(),actor.into(),reason.to_owned().into()]).await?;
    tx.commit().await?;
    tracing::info!(agreement_id=%id,app_id=%app,actor_id=%actor,action="activated","Commercial agreement activated");
    Ok(id)
}

pub async fn set_status(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
    new_status: &str,
    reason: &str,
) -> ApiResult<()> {
    if !["paused", "ended"].contains(&new_status) || reason.trim().is_empty() || reason.len() > 500
    {
        return Err(ApiFailure::invalid("Status or reason is invalid"));
    }
    let tx = ctx.db.begin().await?;
    one(
        &tx,
        "SELECT id FROM apps WHERE id=$1 FOR UPDATE",
        vec![app.into()],
    )
    .await?;
    // Expiry only blocks new work. Operators must still be able to close the
    // expired row before activating the next nonoverlapping contract period.
    let a = latest(&tx, app).await?.ok_or_else(ApiFailure::missing)?;
    if a.status == "ended" || (a.status == "paused" && new_status == "paused") {
        return Err(commercial_conflict(
            "This agreement status is already settled",
        ));
    }
    exec(
        &tx,
        "UPDATE commercial_agreements SET status=$2 WHERE id=$1",
        vec![a.id.into(), new_status.into()],
    )
    .await?;
    exec(&tx,"INSERT INTO commercial_audit(id,agreement_id,actor_id,action,reason) VALUES($1,$2,$3,$4,$5)",vec![Uuid::new_v4().into(),a.id.into(),actor.into(),new_status.into(),reason.to_owned().into()]).await?;
    tx.commit().await?;
    Ok(())
}

pub async fn review(
    ctx: &AppContext,
    actor: Uuid,
    run: Uuid,
    decision: CommercialUsageState,
    reason: &str,
) -> ApiResult<()> {
    if decision == CommercialUsageState::Reserved || reason.trim().is_empty() || reason.len() > 500
    {
        return Err(ApiFailure::invalid(
            "A reviewed decision and reason are required",
        ));
    }
    if credit_billing::review(ctx, actor, run, decision, reason).await? {
        return Ok(());
    }
    let tx = ctx.db.begin().await?;
    let row = one(
        &tx,
        "SELECT agreement_id,app_id,state FROM commercial_usage WHERE run_id=$1 FOR UPDATE",
        vec![run.into()],
    )
    .await?;
    if runs::detail(&tx, run).await?.state != mobile_qa_contracts::execution::JobState::Finished {
        return Err(commercial_conflict(
            "Finish or cancel this run before settling its check",
        ));
    }
    let old: String = field(&row, "state")?;
    if old != "reserved" {
        return Err(commercial_conflict("This check was already reviewed"));
    }
    let agreement: Uuid = field(&row, "agreement_id")?;
    let app: Uuid = field(&row, "app_id")?;
    let state = if decision == CommercialUsageState::Delivered {
        "delivered"
    } else {
        "credited"
    };
    exec(&tx,"UPDATE commercial_usage SET state=$2,reason=$3,reviewer_id=$4,reviewed_at=now() WHERE run_id=$1",vec![run.into(),state.into(),reason.to_owned().into(),actor.into()]).await?;
    exec(&tx,"INSERT INTO commercial_audit(id,agreement_id,run_id,actor_id,action,reason) VALUES($1,$2,$3,$4,$5,$6)",vec![Uuid::new_v4().into(),agreement.into(),run.into(),actor.into(),state.into(),reason.to_owned().into()]).await?;
    tx.commit().await?;
    tracing::info!(run_id=%run,app_id=%app,actor_id=%actor,action=%state,"Commercial check reviewed");
    Ok(())
}

/// Credit only when every attempt was canceled while still queued. This runs in the
/// cancellation transaction so a concurrent claim cannot consume the same check.
pub async fn credit_queued_cancel(
    db: &impl ConnectionTrait,
    actor: Uuid,
    run: Uuid,
) -> ApiResult<()> {
    let claimed=rows(db,"SELECT id FROM execution_attempts WHERE run_id=$1 AND (claim_id IS NOT NULL OR state<>'finished' OR reason IS DISTINCT FROM 'canceled_before_dispatch') LIMIT 1",vec![run.into()]).await?;
    if !claimed.is_empty() {
        return Ok(());
    }
    if credit_billing::release_queued_cancel(db, actor, run).await? {
        return Ok(());
    }
    let usage = rows(
        db,
        "SELECT agreement_id FROM commercial_usage WHERE run_id=$1 AND state='reserved' FOR UPDATE",
        vec![run.into()],
    )
    .await?;
    let Some(row) = usage.first() else {
        return Ok(());
    };
    let agreement: Uuid = field(row, "agreement_id")?;
    let reason = "Canceled before device claim";
    exec(db,"UPDATE commercial_usage SET state='credited',reason=$2,reviewer_id=$3,reviewed_at=now() WHERE run_id=$1",vec![run.into(),reason.into(),actor.into()]).await?;
    exec(db,"INSERT INTO commercial_audit(id,agreement_id,run_id,actor_id,action,reason) VALUES($1,$2,$3,$4,'credited',$5)",vec![Uuid::new_v4().into(),agreement.into(),run.into(),actor.into(),reason.into()]).await?;
    Ok(())
}
