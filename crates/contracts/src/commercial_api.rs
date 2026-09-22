//! App-scoped commercial reads and quotes share the authenticated browser contract.
use crate::{browser::ApiError, commercial::*};
use utoipa::OpenApi;

#[utoipa::path(get,path="/api/apps/{app_id}/commercial-access",operation_id="getCommercialAccess",security(("session_cookie"=[])),params(("app_id"=Uuid,Path)),responses((status=200,description="Commercial access",body=CommercialAccessResponse),(status=401,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code, non_snake_case)]
fn getCommercialAccess() {}

#[utoipa::path(post,path="/api/apps/{app_id}/commercial-check-quotes",operation_id="createCommercialCheckQuote",security(("session_cookie"=[])),params(("app_id"=Uuid,Path)),request_body=CommercialQuoteRequest,responses((status=201,description="Run quote",body=CommercialQuoteResponse),(status=401,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code, non_snake_case)]
fn createCommercialCheckQuote() {}

#[utoipa::path(post,path="/api/apps/{app_id}/commercial-pilot-requests",operation_id="requestCommercialPilot",security(("session_cookie"=[])),params(("app_id"=Uuid,Path)),request_body=CommercialPilotRequest,responses((status=201,description="Pilot request saved for manual review",body=CommercialPilotResponse),(status=401,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=422,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code, non_snake_case)]
fn requestCommercialPilot() {}

#[utoipa::path(post,path="/api/apps/{app_id}/credit-checkout",operation_id="createCreditCheckout",security(("session_cookie"=[])),params(("app_id"=Uuid,Path)),request_body=CreditCheckoutRequest,responses((status=201,description="Hosted payment URL",body=CreditCheckoutResponse),(status=401,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code, non_snake_case)]
fn createCreditCheckout() {}

#[utoipa::path(post,path="/api/apps/{app_id}/credit-plan-change",operation_id="changeCreditPlan",security(("session_cookie"=[])),params(("app_id"=Uuid,Path)),request_body=CreditPlanChangeRequest,responses((status=200,description="Updated plan schedule and current access",body=CommercialAccessResponse),(status=401,description="API error",body=ApiError),(status=404,description="API error",body=ApiError),(status=409,description="API error",body=ApiError),(status=503,description="API error",body=ApiError),(status="default",description="API error",body=ApiError)))]
#[allow(dead_code, non_snake_case)]
fn changeCreditPlan() {}

#[derive(OpenApi)]
#[openapi(
    paths(
        getCommercialAccess,
        createCommercialCheckQuote,
        requestCommercialPilot,
        createCreditCheckout,
        changeCreditPlan
    ),
    components(schemas(
        CommercialOffer,
        CommercialState,
        CommercialUsageState,
        CommercialAgreementView,
        CommercialUsageView,
        CommercialAccessResponse,
        CommercialRunIntent,
        CommercialQuoteRequest,
        CommercialQuoteResponse,
        CommercialPilotRequest,
        CommercialPilotResponse,
        CreditPlan,
        CreditPlanView,
        CreditUsageView,
        CreditAccessView,
        CreditCheckoutRequest,
        CreditCheckoutResponse,
        CreditPlanChangeRequest
    ))
)]
pub struct CommercialApi;

pub const OPERATIONS: &[(&str, &str, &str, u16)] = &[
    (
        "get",
        "/api/apps/{app_id}/commercial-access",
        "getCommercialAccess",
        200,
    ),
    (
        "post",
        "/api/apps/{app_id}/commercial-check-quotes",
        "createCommercialCheckQuote",
        201,
    ),
    (
        "post",
        "/api/apps/{app_id}/commercial-pilot-requests",
        "requestCommercialPilot",
        201,
    ),
    (
        "post",
        "/api/apps/{app_id}/credit-checkout",
        "createCreditCheckout",
        201,
    ),
    (
        "post",
        "/api/apps/{app_id}/credit-plan-change",
        "changeCreditPlan",
        200,
    ),
];
