//! Commercial access is app-scoped and only the trusted process can change agreements.
//! A quote binds a visible price to an exact run manifest; reservation shares its transaction.
use super::{
    apps, credit_billing, execution_store::hash, runs, suite_runs, test_definitions, test_library,
};
use crate::{
    errors::{ApiFailure, ApiResult},
    models::_entities::{
        apps as app_rows, commercial_agreements, commercial_audit, commercial_pilot_requests,
        commercial_quotes, commercial_usage, execution_attempts, execution_definitions,
    },
};
use chrono::{DateTime, Duration, Utc};
use loco_rs::app::AppContext;
use mobile_qa_contracts::{
    commercial::*,
    execution::{CreateRunRequest, Driver, RunManifest, TestDefinition},
    regression::SuiteRunRequest,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, IntoActiveModel, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
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
    let Some(row) = commercial_agreements::Entity::find()
        .filter(commercial_agreements::Column::AppId.eq(app))
        .order_by_desc(commercial_agreements::Column::CreatedAt)
        .order_by_desc(commercial_agreements::Column::Id)
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    Ok(Some(Agreement {
        id: row.id,
        offer: match row.offer.as_str() {
            "pilot" => CommercialOffer::Pilot,
            "recurring" => CommercialOffer::Recurring,
            _ => return Err(ApiFailure::internal()),
        },
        status: row.status,
        price_revision: row.price_revision,
        starts_at: row.starts_at,
        ends_at: row.ends_at,
        first_source: row.first_source_version_id,
        second_source: row.second_source_version_id,
        profile_id: row.profile_id,
        base_cents: row.base_cents,
        second_suite_cents: row.second_suite_cents,
        check_cents: row.check_cents,
        check_cap: row.check_cap,
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
    let count = commercial_usage::Entity::find()
        .filter(commercial_usage::Column::AgreementId.eq(agreement))
        .filter(commercial_usage::Column::State.is_in(["reserved", "delivered"]))
        .count(db)
        .await?;
    i32::try_from(count).map_err(|_| ApiFailure::internal())
}

pub async fn status(
    ctx: &AppContext,
    actor: Uuid,
    app: Uuid,
) -> ApiResult<CommercialAccessResponse> {
    apps::authorized(ctx, actor, app).await?;
    let agreement = latest(&ctx.db, app).await?;
    let pilot_request_id = commercial_pilot_requests::Entity::find()
        .filter(commercial_pilot_requests::Column::AppId.eq(app))
        .one(&ctx.db)
        .await?
        .map(|row| row.id);
    let mut usage = Vec::new();
    let (reserved_checks, delivered_checks, credited_checks, delivered_check_cents) =
        if let Some(a) = &agreement {
            let all_usage = commercial_usage::Entity::find()
                .filter(commercial_usage::Column::AgreementId.eq(a.id))
                .order_by_desc(commercial_usage::Column::CreatedAt)
                .order_by_desc(commercial_usage::Column::RunId)
                .all(&ctx.db)
                .await?;
            let mut reserved = 0;
            let mut delivered = 0;
            let mut credited = 0;
            let mut delivered_cents = 0;
            for row in &all_usage {
                match row.state.as_str() {
                    "reserved" => reserved += 1,
                    "delivered" => {
                        delivered += 1;
                        delivered_cents += row.amount_cents;
                    }
                    "credited" => credited += 1,
                    _ => return Err(ApiFailure::internal()),
                }
            }
            for row in all_usage.into_iter().take(100) {
                let current = match row.state.as_str() {
                    "reserved" => CommercialUsageState::Reserved,
                    "delivered" => CommercialUsageState::Delivered,
                    "credited" => CommercialUsageState::Credited,
                    _ => return Err(ApiFailure::internal()),
                };
                usage.push(CommercialUsageView {
                    run_id: row.run_id,
                    state: current,
                    amount_cents: row.amount_cents,
                    reason: row.reason,
                    created_at: row.created_at,
                    reviewed_at: row.reviewed_at,
                });
            }
            (reserved, delivered, credited, delivered_cents)
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
    app_rows::Entity::find_by_id(app)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if latest(&tx, app).await?.is_some() {
        return Err(commercial_conflict("This app already has an agreement"));
    }
    if commercial_pilot_requests::Entity::find()
        .filter(commercial_pilot_requests::Column::AppId.eq(app))
        .one(&tx)
        .await?
        .is_some()
    {
        return Err(commercial_conflict(
            "A pilot request is already recorded for this app",
        ));
    }
    let id = Uuid::new_v4();
    commercial_pilot_requests::ActiveModel {
        id: Set(id),
        app_id: Set(app),
        actor_id: Set(actor),
        coverage_note: Set(note.to_owned()),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
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
    commercial_quotes::ActiveModel {
        id: Set(id),
        agreement_id: Set(agreement.id),
        app_id: Set(app),
        actor_id: Set(actor),
        kind: Set(kind.into()),
        request_hash: Set(request_hash),
        manifest_hash: Set(manifest_hash),
        build_id: Set(build),
        source_version_id: Set(source),
        profile_id: Set(manifest.profile.id),
        environment_revision: Set(manifest.environment_revision),
        price_revision: Set(agreement.price_revision),
        amount_cents: Set(amount),
        expires_at: Set(expiry),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await?;
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
    let q = commercial_quotes::Entity::find_by_id(quote_id)
        .one(db)
        .await?
        .ok_or_else(ApiFailure::missing)?;
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
    if q.agreement_id != a.id
        || q.app_id != app
        || q.actor_id != actor
        || q.kind != kind
        || q.request_hash != request_hash
        || q.manifest_hash != digest
        || q.build_id != manifest.build_id
        || q.source_version_id != expected_source
        || q.profile_id != manifest.profile.id
        || manifest.profile.id != a.profile_id
        || q.environment_revision != manifest.environment_revision
        || q.price_revision != a.price_revision
        || q.amount_cents != a.check_cents
        || q.expires_at <= Utc::now()
    {
        return Err(commercial_conflict(
            "Price or run setup changed; review a new quote",
        ));
    }
    if commercial_usage::Entity::find()
        .filter(commercial_usage::Column::QuoteId.eq(quote_id))
        .one(db)
        .await?
        .is_some()
    {
        return Err(commercial_conflict("This quote has already been used"));
    }
    if used(db, a.id).await? >= a.check_cap {
        return Err(commercial_conflict(
            "The agreed check limit has been reached",
        ));
    }
    commercial_usage::ActiveModel {
        run_id: Set(run_id),
        quote_id: Set(quote_id),
        agreement_id: Set(a.id),
        app_id: Set(app),
        period_start: Set(a.starts_at),
        period_end: Set(a.ends_at),
        amount_cents: Set(a.check_cents),
        ..Default::default()
    }
    .insert(db)
    .await?;
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
        let kind = execution_definitions::Entity::find_by_id(source)
            .filter(execution_definitions::Column::AppId.eq(app))
            .one(&ctx.db)
            .await?
            .ok_or_else(ApiFailure::missing)?
            .kind;
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
    app_rows::Entity::find_by_id(app)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if commercial_agreements::Entity::find()
        .filter(commercial_agreements::Column::AppId.eq(app))
        .filter(commercial_agreements::Column::StartsAt.lt(ends_at))
        .filter(commercial_agreements::Column::EndsAt.gt(starts_at))
        .one(&tx)
        .await?
        .is_some()
    {
        return Err(commercial_conflict("Agreement periods must not overlap"));
    }
    if commercial_agreements::Entity::find()
        .filter(commercial_agreements::Column::AppId.eq(app))
        .filter(commercial_agreements::Column::Status.eq("active"))
        .one(&tx)
        .await?
        .is_some()
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
    commercial_agreements::ActiveModel {
        id: Set(id),
        app_id: Set(app),
        offer: Set(if offer == CommercialOffer::Pilot {
            "pilot"
        } else {
            "recurring"
        }
        .into()),
        status: Set("active".into()),
        price_revision: Set(1),
        starts_at: Set(starts_at),
        ends_at: Set(ends_at),
        first_source_version_id: Set(first),
        second_source_version_id: Set(second),
        profile_id: Set(profile),
        base_cents: Set(base),
        second_suite_cents: Set(second_cents),
        check_cents: Set(check),
        check_cap: Set(cap),
        agreement_reference: Set(reference.to_owned()),
        created_by: Set(actor),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
    commercial_audit::ActiveModel {
        id: Set(Uuid::new_v4()),
        agreement_id: Set(id),
        actor_id: Set(actor),
        action: Set("activated".into()),
        reason: Set(reason.to_owned()),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
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
    app_rows::Entity::find_by_id(app)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    // Expiry only blocks new work. Operators must still be able to close the
    // expired row before activating the next nonoverlapping contract period.
    let a = latest(&tx, app).await?.ok_or_else(ApiFailure::missing)?;
    if a.status == "ended" || (a.status == "paused" && new_status == "paused") {
        return Err(commercial_conflict(
            "This agreement status is already settled",
        ));
    }
    let agreement = commercial_agreements::Entity::find_by_id(a.id)
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    let mut active = agreement.into_active_model();
    active.status = Set(new_status.into());
    active.update(&tx).await?;
    commercial_audit::ActiveModel {
        id: Set(Uuid::new_v4()),
        agreement_id: Set(a.id),
        actor_id: Set(actor),
        action: Set(new_status.into()),
        reason: Set(reason.to_owned()),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
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
    let row = commercial_usage::Entity::find_by_id(run)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(ApiFailure::missing)?;
    if runs::detail(&tx, run).await?.state != mobile_qa_contracts::execution::JobState::Finished {
        return Err(commercial_conflict(
            "Finish or cancel this run before settling its check",
        ));
    }
    let old = row.state.clone();
    if old != "reserved" {
        return Err(commercial_conflict("This check was already reviewed"));
    }
    let agreement = row.agreement_id;
    let app = row.app_id;
    let state = if decision == CommercialUsageState::Delivered {
        "delivered"
    } else {
        "credited"
    };
    let mut active = row.into_active_model();
    active.state = Set(state.into());
    active.reason = Set(Some(reason.to_owned()));
    active.reviewer_id = Set(Some(actor));
    active.reviewed_at = Set(Some(Utc::now()));
    active.update(&tx).await?;
    commercial_audit::ActiveModel {
        id: Set(Uuid::new_v4()),
        agreement_id: Set(agreement),
        run_id: Set(Some(run)),
        actor_id: Set(actor),
        action: Set(state.into()),
        reason: Set(reason.to_owned()),
        ..Default::default()
    }
    .insert(&tx)
    .await?;
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
    let attempts = execution_attempts::Entity::find()
        .filter(execution_attempts::Column::RunId.eq(run))
        .all(db)
        .await?;
    if attempts.iter().any(|attempt| {
        attempt.claim_id.is_some()
            || attempt.state != "finished"
            || attempt.reason.as_deref() != Some("canceled_before_dispatch")
    }) {
        return Ok(());
    }
    if credit_billing::release_queued_cancel(db, actor, run).await? {
        return Ok(());
    }
    let usage = commercial_usage::Entity::find_by_id(run)
        .filter(commercial_usage::Column::State.eq("reserved"))
        .lock_exclusive()
        .one(db)
        .await?;
    let Some(row) = usage else {
        return Ok(());
    };
    let agreement = row.agreement_id;
    let reason = "Canceled before device claim";
    let mut active = row.into_active_model();
    active.state = Set("credited".into());
    active.reason = Set(Some(reason.into()));
    active.reviewer_id = Set(Some(actor));
    active.reviewed_at = Set(Some(Utc::now()));
    active.update(db).await?;
    commercial_audit::ActiveModel {
        id: Set(Uuid::new_v4()),
        agreement_id: Set(agreement),
        run_id: Set(Some(run)),
        actor_id: Set(actor),
        action: Set("credited".into()),
        reason: Set(reason.into()),
        ..Default::default()
    }
    .insert(db)
    .await?;
    Ok(())
}
