// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Webserver utility functions
//!
//! Helper functions for common webserver operations

use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

/// The HTTP status a typed domain error already carries.
///
/// Every module error implements [`ErrorClass::http_status`], so a route must
/// never re-derive the status by reading the error's message text: the prose is
/// free to change with the error vocabulary, and when it does a silently
/// unmatched `.contains(..)` downgrades a 404 or a 400 to a 500 with nothing
/// failing. Take the status from the value instead, and match the variant when
/// a route also needs its own error code.
pub fn status_for(error: &impl crate::errors::ErrorClass) -> StatusCode {
    StatusCode::from_u16(error.http_status()).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
}

/// Format success response with data
pub fn success_response<T: serde::Serialize>(data: T) -> Response {
    Json(data).into_response()
}

/// Format duration in human-readable format
pub fn format_duration(seconds: u64) -> String {
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if days > 0 {
        format!("{days}d {hours}h {minutes}m {secs}s")
    } else if hours > 0 {
        format!("{hours}h {minutes}m {secs}s")
    } else if minutes > 0 {
        format!("{minutes}m {secs}s")
    } else {
        format!("{secs}s")
    }
}

/// Build a 200 response that browsers must not cache. Embedded assets change
/// with every binary, so a stale cached copy is never valid.
pub fn no_store_response(content_type: &'static str, body: String) -> Response {
    (
        StatusCode::OK,
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-store, no-cache, must-revalidate"),
        ],
        body,
    )
        .into_response()
}
