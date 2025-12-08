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

/// Security header name for token validation
pub const SECURITY_TOKEN_HEADER: &str = "X-ScreenerBot-Token";

/// Security gate middleware (GUI mode only)
///
/// In GUI mode, validates that all requests include a valid security token.
/// This prevents external access to the webserver via browser.
///
/// The security token is:
/// - Generated at startup (random 64-char alphanumeric)
/// - Injected into the HTML template by the server
/// - Required in X-ScreenerBot-Token header for all API requests
///
/// Allowed without token (required for initial page load):
/// - Root path (/) - returns HTML with embedded token
/// - Static assets (/assets/*, /scripts/*, /styles/*)
/// - Page HTML (/api/pages/*)
///
/// In CLI mode, this middleware does nothing (allows all requests).
pub async fn security_gate(request: Request, next: Next) -> Response {
  // Skip security check in CLI mode
  if !global::is_gui_mode() {
    return next.run(request).await;
  }

  let path = request.uri().path();

  // Allow initial page load and static assets without token
  // These are needed for the browser to receive the HTML (which contains the token)
  if path == "/"
    || path.starts_with("/assets/")
    || path.starts_with("/scripts/")
    || path.starts_with("/styles/")
    || path.starts_with("/api/pages/")
  {
    return next.run(request).await;
  }

  // GUI mode: validate security token for API endpoints
  let token = request
    .headers()
    .get(SECURITY_TOKEN_HEADER)
    .and_then(|v| v.to_str().ok());

  match token {
    Some(t) if global::validate_security_token(t) => {
      // Valid token, allow request
      next.run(request).await
    }
    Some(_) => {
      // Invalid token
      logger::warning(
        LogTag::Webserver,
        &format!(
          "Blocked request to {} - invalid security token",
          request.uri().path()
        ),
      );
      utils::error_response(
        StatusCode::FORBIDDEN,
        "INVALID_TOKEN",
        "Invalid security token",
        None,
      )
    }
    None => {
      // Missing token - only log for API endpoints (not for page loads)
      if path.starts_with("/api/") {
        logger::warning(
          LogTag::Webserver,
          &format!(
            "Blocked API request to {} - missing security token",
            path
          ),
        );
      }
      utils::error_response(
        StatusCode::FORBIDDEN,
        "MISSING_TOKEN",
        "Security token required",
        Some("This endpoint is only accessible from within ScreenerBot"),
      )
    }
  }
}

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

  // Allow actions endpoints (actions system works independently)
  if path.starts_with("/api/actions") {
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
 "Blocked pre-initialization request to {} (initialization required)",
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
