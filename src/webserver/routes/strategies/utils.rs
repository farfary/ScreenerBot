use axum::http::StatusCode;
use axum::response::Response;

use crate::webserver::utils::error_response;

/// Helper to create error response with standard format
pub fn err(status: StatusCode, message: &str) -> Response {
    error_response(
        status,
        match status {
            StatusCode::BAD_REQUEST => "BAD_REQUEST",
            StatusCode::NOT_FOUND => "NOT_FOUND",
            StatusCode::CONFLICT => "CONFLICT",
            StatusCode::INTERNAL_SERVER_ERROR => "INTERNAL_SERVER_ERROR",
            _ => "ERROR",
        },
        message,
        None,
    )
}
