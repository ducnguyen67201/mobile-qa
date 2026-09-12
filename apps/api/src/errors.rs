//! Safe public failures. Final middleware attaches the request's server-generated ID.
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use mobile_qa_contracts::browser::ApiError;
use uuid::Uuid;
#[derive(Debug)]
pub struct ApiFailure {
    pub status: StatusCode,
    pub code: &'static str,
    pub message: String,
}
pub type ApiResult<T> = Result<T, ApiFailure>;
impl ApiFailure {
    pub fn new(status: u16, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::from_u16(status).expect("static status"),
            code,
            message: message.into(),
        }
    }
    pub fn missing() -> Self {
        Self::new(404, "not_found", "Record not found")
    }
    pub fn unauthorized() -> Self {
        Self::new(401, "unauthenticated", "Sign in to continue")
    }
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(422, "invalid_input", message)
    }
    pub fn internal() -> Self {
        Self::new(
            500,
            "internal_error",
            "The operation could not be completed. Try again.",
        )
    }
}
impl IntoResponse for ApiFailure {
    fn into_response(self) -> Response {
        let mut response = (
            self.status,
            Json(ApiError {
                code: self.code.into(),
                message: self.message,
                details: None,
                request_id: Uuid::new_v4(),
            }),
        )
            .into_response();
        if self.status == StatusCode::TOO_MANY_REQUESTS {
            response
                .headers_mut()
                .insert("retry-after", "60".parse().expect("static header"));
        }
        response
    }
}
impl From<sea_orm::DbErr> for ApiFailure {
    fn from(_error: sea_orm::DbErr) -> Self {
        tracing::error!(
            reason_code = "database_failure",
            "database operation failed"
        );
        Self::internal()
    }
}
impl From<std::io::Error> for ApiFailure {
    fn from(error: std::io::Error) -> Self {
        tracing::error!(reason_code="storage_io", error_kind=?error.kind(), "artifact IO failed");
        Self::new(
            503,
            "storage_unavailable",
            "Artifact storage is unavailable. Try again.",
        )
    }
}
