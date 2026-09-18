//! HTTP envelopes exported through the worker schema, separate from qualification.
use crate::execution::*;
use schemars::JsonSchema;
#[derive(JsonSchema)]
pub struct ExecutionContracts {
    pub preflight_request: crate::execution_lifecycle::PreflightRequest,
    pub preflight_ack: crate::execution_lifecycle::PreflightAcknowledgement,
    pub job: ExecutionJob,
    pub navigation: NavigationRequest,
    pub local_result: LocalExecutionResult,
    pub claim_request: ClaimRequest,
    pub claim_response: ClaimResponse,
    pub lease_request: LeaseRequest,
    pub lease_status: LeaseStatusResponse,
    pub events: EventRequest,
    pub event_receipt: EventReceipt,
    pub artifact_request: ArtifactRequest,
    pub artifact_receipt: ArtifactReceipt,
    pub completion: CompleteRequest,
    pub cleanup: CleanupRequest,
    pub attempt_receipt: AttemptReceipt,
}
