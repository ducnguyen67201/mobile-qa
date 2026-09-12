//! API response normalization; never exposes framework/extractor diagnostics.
use axum::{
    body::{to_bytes, Body},
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use mobile_qa_contracts::browser::ApiError;
use uuid::Uuid;
pub async fn request_context(request: Request, next: Next) -> Response {
    if !request.uri().path().starts_with("/api") {
        return next.run(request).await;
    }
    let request_id = Uuid::new_v4();
    let started = std::time::Instant::now();
    let response = next.run(request).await;
    let (mut parts, body) = response.into_parts();
    parts.headers.insert(
        "x-request-id",
        request_id.to_string().parse().expect("uuid header"),
    );
    parts
        .headers
        .insert("cache-control", "no-store".parse().expect("static header"));
    if parts.status.is_client_error() || parts.status.is_server_error() {
        let raw = to_bytes(body, 65536).await.unwrap_or_default();
        let mut error = serde_json::from_slice::<ApiError>(&raw).unwrap_or_else(|_| {
            let (code, message) = match parts.status.as_u16() {
                400 | 422 => ("invalid_input", "Request data is invalid"),
                401 => ("unauthenticated", "Sign in to continue"),
                403 => ("forbidden", "This operation is not permitted"),
                404 => ("not_found", "Record not found"),
                405 => ("method_not_allowed", "HTTP method not allowed"),
                413 => ("file_too_large", "The upload exceeds the allowed size"),
                415 => (
                    "unsupported_media_type",
                    "Use the declared request content type",
                ),
                _ => (
                    "internal_error",
                    "The operation could not be completed. Try again.",
                ),
            };
            ApiError {
                code: code.into(),
                message: message.into(),
                details: None,
                request_id,
            }
        });
        error.request_id = request_id;
        tracing::warn!(%request_id,status=parts.status.as_u16(),reason_code=%error.code,duration_ms=started.elapsed().as_millis(),"API operation rejected");
        parts.headers.remove("content-length");
        parts.headers.insert(
            "content-type",
            "application/json".parse().expect("static header"),
        );
        return Response::from_parts(parts, Json(error).into_response().into_body());
    }
    Response::from_parts(parts, Body::new(body))
}
