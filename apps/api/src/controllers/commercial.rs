//! Customer reads and short-lived prices; agreement mutations remain process-only.
use crate::{
    errors::ApiResult,
    services::{auth::Session, commercial, credit_billing},
};
use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use loco_rs::{app::AppContext, controller::Routes};
use mobile_qa_contracts::commercial::{
    CommercialAccessResponse, CommercialPilotRequest, CommercialPilotResponse,
    CommercialQuoteRequest, CommercialQuoteResponse, CreditCheckoutRequest, CreditCheckoutResponse,
    CreditPlanChangeRequest,
};
use uuid::Uuid;

async fn status(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
) -> ApiResult<Json<CommercialAccessResponse>> {
    Ok(Json(commercial::status(&ctx, session.user.id, app).await?))
}

async fn quote(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Json(input): Json<CommercialQuoteRequest>,
) -> ApiResult<(StatusCode, Json<CommercialQuoteResponse>)> {
    Ok((
        StatusCode::CREATED,
        Json(commercial::quote(&ctx, session.user.id, app, input).await?),
    ))
}

async fn pilot(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Json(input): Json<CommercialPilotRequest>,
) -> ApiResult<(StatusCode, Json<CommercialPilotResponse>)> {
    Ok((
        StatusCode::CREATED,
        Json(commercial::request_pilot(&ctx, session.user.id, app, input).await?),
    ))
}

async fn checkout(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Json(input): Json<CreditCheckoutRequest>,
) -> ApiResult<(StatusCode, Json<CreditCheckoutResponse>)> {
    Ok((
        StatusCode::CREATED,
        Json(credit_billing::checkout(&ctx, session.user.id, app, input.plan).await?),
    ))
}

async fn change_plan(
    State(ctx): State<AppContext>,
    session: Session,
    Path(app): Path<Uuid>,
    Json(input): Json<CreditPlanChangeRequest>,
) -> ApiResult<Json<CommercialAccessResponse>> {
    credit_billing::change_plan(&ctx, session.user.id, app, input.plan).await?;
    Ok(Json(commercial::status(&ctx, session.user.id, app).await?))
}

async fn stripe_webhook(
    State(ctx): State<AppContext>,
    headers: HeaderMap,
    body: Bytes,
) -> ApiResult<StatusCode> {
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(crate::errors::ApiFailure::unauthorized)?;
    credit_billing::webhook(&ctx, signature, &body).await?;
    Ok(StatusCode::OK)
}

pub fn routes() -> Routes {
    use axum::routing::{get, post};
    Routes::new()
        .add("/api/apps/{app_id}/commercial-access", get(status))
        .add("/api/apps/{app_id}/commercial-check-quotes", post(quote))
        .add("/api/apps/{app_id}/commercial-pilot-requests", post(pilot))
        .add("/api/apps/{app_id}/credit-checkout", post(checkout))
        .add("/api/apps/{app_id}/credit-plan-change", post(change_plan))
        .add("/api/billing/stripe-webhook", post(stripe_webhook))
}
