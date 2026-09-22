//! Browser commercial terms and run quote transport; money is integer USD cents.
use crate::{execution::CreateRunRequest, regression::SuiteRunRequest};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CommercialOffer {
    Pilot,
    Recurring,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CreditPlan {
    Starter,
    Plus,
    Business,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreditPlanView {
    pub plan: CreditPlan,
    pub monthly_cents: i32,
    pub monthly_credits: i32,
    pub currency: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreditUsageView {
    pub run_id: Uuid,
    pub state: String,
    pub held_credits: i32,
    pub measured_credits: Option<String>,
    pub charged_credits: i32,
    pub input_tokens: Option<String>,
    pub output_tokens: Option<String>,
    pub device_seconds: Option<i32>,
    pub stored_bytes: Option<String>,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreditAccessView {
    pub plan: CreditPlan,
    pub pending_plan: Option<CreditPlan>,
    pub pending_effective_at: Option<DateTime<Utc>>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub granted_credits: i32,
    pub charged_credits: i32,
    pub held_credits: i32,
    pub available_credits: i32,
    pub rate_revision: i32,
    pub usage: Vec<CreditUsageView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreditCheckoutRequest {
    pub plan: CreditPlan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreditPlanChangeRequest {
    pub plan: CreditPlan,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreditCheckoutResponse {
    pub url: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CommercialState {
    Uncontracted,
    Active,
    Paused,
    Expired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CommercialUsageState {
    Reserved,
    Delivered,
    Credited,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommercialAgreementView {
    pub id: Uuid,
    pub offer: CommercialOffer,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub first_source_version_id: Uuid,
    pub second_source_version_id: Option<Uuid>,
    pub profile_id: Uuid,
    pub base_cents: i32,
    pub second_suite_cents: i32,
    pub check_cents: i32,
    pub check_cap: i32,
    pub currency: String,
    pub price_revision: i32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommercialUsageView {
    pub run_id: Uuid,
    pub state: CommercialUsageState,
    pub amount_cents: i32,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
    pub reviewed_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommercialAccessResponse {
    pub app_id: Uuid,
    pub state: CommercialState,
    pub plans: Vec<CreditPlanView>,
    pub credit: Option<CreditAccessView>,
    pub agreement: Option<CommercialAgreementView>,
    pub pilot_request_id: Option<Uuid>,
    pub reserved_checks: i32,
    pub delivered_checks: i32,
    pub credited_checks: i32,
    pub delivered_check_cents: i32,
    pub usage: Vec<CommercialUsageView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommercialPilotRequest {
    pub coverage_note: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommercialPilotResponse {
    pub id: Uuid,
    pub app_id: Uuid,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", content = "request", rename_all = "snake_case")]
pub enum CommercialRunIntent {
    ReleasePlan(CreateRunRequest),
    SavedSuite(SuiteRunRequest),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommercialQuoteRequest {
    pub intent: CommercialRunIntent,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommercialQuoteResponse {
    pub id: Uuid,
    pub app_id: Uuid,
    pub kind: String,
    pub build_id: Uuid,
    pub source_version_id: Uuid,
    pub profile_id: Uuid,
    pub case_count: i32,
    pub amount_cents: i32,
    pub maximum_credits: Option<i32>,
    pub credits_after_authorization: Option<i32>,
    pub currency: String,
    pub checks_after_authorization: i32,
    pub check_cap: i32,
    pub expires_at: DateTime<Utc>,
}
