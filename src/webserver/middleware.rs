/// Webserver middleware
///
/// Request interceptors for authentication, validation, gating, and cache control
use axum::{
  body::Body,
  extract::Request,
  http::{header, StatusCode},
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
/// - SSE streams (/api/*/stream) - EventSource API doesn't support custom headers
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
  // Also allow SSE stream endpoints - EventSource API cannot send custom headers
  // Also allow initialization endpoints - needed before security token is injected
  // Page routes (non-API) are allowed - they return HTML with embedded token
  if path == "/"
    || path.starts_with("/assets/")
    || path.starts_with("/scripts/")
    || path.starts_with("/styles/")
    || path.starts_with("/api/pages/")
    || path.starts_with("/api/initialization")
    || path.starts_with("/api/system/bootstrap")
    || path.starts_with("/api/actions")
    || path.starts_with("/api/services")
    || path.ends_with("/stream")
    || !path.starts_with("/api/")  // All non-API routes (HTML pages) are allowed
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
      // Missing token - log for debugging
      logger::warning(
        LogTag::Webserver,
        &format!(
          "Blocked API request to {} - missing security token (GUI mode: {})",
          path,
          global::is_gui_mode()
        ),
      );
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

/// Cache control middleware
///
/// Adds Cache-Control headers to prevent WebView/browser caching of static resources.
/// This ensures fresh CSS, JS, and HTML are always fetched, especially important for:
/// - Tauri WebView (WKWebView on macOS) which caches aggressively
/// - Development mode where styles/scripts change frequently
///
/// Header values:
/// - `no-cache`: Force revalidation with server before using cached copy
/// - `no-store`: Don't store any version in cache
/// - `must-revalidate`: After expiration, must check with server
/// - `max-age=0`: Consider stale immediately
pub async fn cache_control(request: Request, next: Next) -> Response {
  let mut response = next.run(request).await;
  
  // Add cache control headers to prevent caching
  let headers = response.headers_mut();
  headers.insert(
    header::CACHE_CONTROL,
    "no-cache, no-store, must-revalidate, max-age=0".parse().unwrap(),
  );
  headers.insert(
    header::PRAGMA,
    "no-cache".parse().unwrap(),
  );
  headers.insert(
    header::EXPIRES,
    "0".parse().unwrap(),
  );
  
  response
}
