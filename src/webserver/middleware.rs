/// Webserver middleware
///
/// Request interceptors for authentication, validation, and gating
use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    global,
    logger::{self, LogTag},
    webserver::utils,
};

/// Pre-initialization gate middleware
///
/// Blocks all non-initialization API endpoints until INITIALIZATION_COMPLETE is true.
/// Allows:
/// - /api/initialization/* (all initialization endpoints)
/// - Static resources (HTML pages, scripts, styles)
/// - Root paths (/, /services, /tokens, etc. - for page HTML)
///
/// Blocks:
/// - All other /api/* endpoints when not initialized
pub async fn initialization_gate(request: Request, next: Next) -> Response {
    let path = request.uri().path();

    // Check if initialization is complete
    let initialized = global::is_initialization_complete();

    // If initialized, allow everything
    if initialized {
        return next.run(request).await;
    }

    // Not initialized - check if this is an allowed path

    // Allow initialization endpoints
    if path.starts_with("/api/initialization") || path.starts_with("/api/system/bootstrap") {
        return next.run(request).await;
    }

    // Allow static resources (scripts, styles, page HTML)
    if path.starts_with("/scripts/")
        || path.starts_with("/styles/")
        || path.starts_with("/api/pages/")
        || path == "/"
        || !path.starts_with("/api/")
    {
        return next.run(request).await;
    }

    // Block all other API endpoints with error response
    logger::debug(
        LogTag::Webserver,
        &format!(
            "❌ Blocked pre-initialization request to {} (initialization required)",
            path
        ),
    );

    utils::error_response(
        StatusCode::SERVICE_UNAVAILABLE,
        "INITIALIZATION_REQUIRED",
        "Bot initialization is required before accessing this endpoint",
        Some("Please complete the initialization process through the web interface"),
    )
}
