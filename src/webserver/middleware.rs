//! Webserver middleware
//!
//! Request interceptors for authentication, validation, gating, and cache control

use axum::{
    body::Body,
    extract::Request,
    http::{header, header::HeaderValue, uri::Authority, HeaderMap, StatusCode, Uri},
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
/// In GUI mode, validates that requests target the loopback dashboard origin,
/// then requires a valid security token for protected APIs.
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
/// - /oauth/callback - the system browser returns here after a ScreenerBot
///   account sign-in and cannot send a custom header. Safe because the route
///   accepts only a `code` and a `state`, both checked against an in-memory
///   PendingAuth this process created, with the state compared in constant
///   time, and because it renders a fixed page that echoes nothing back.
///   NOTE: /api/account/* is deliberately NOT exempt - it is the sign-in API
///   and must keep requiring the token.
///
/// In CLI mode, this middleware does nothing (allows all requests).
pub async fn security_gate(request: Request, next: Next) -> Response {
    // Skip security check in CLI mode
    if !global::is_gui_mode() {
        return next.run(request).await;
    }

    if !has_valid_local_request_headers(request.headers()) {
        logger::warning(
            LogTag::Webserver,
            &format!(
                "Blocked request to {} - invalid local request headers",
                request.uri().path()
            ),
        );
        return utils::error_response(
            StatusCode::FORBIDDEN,
            "INVALID_LOCAL_REQUEST",
            "Request must originate from the local dashboard",
            None,
        );
    }

    let path = request.uri().path();

    // Allow initial page load and static assets without token
    // These are needed for the browser to receive the HTML (which contains the token)
    // Also allow SSE stream endpoints - EventSource API cannot send custom headers
    // Page routes (non-API) are allowed - they return HTML with embedded token
    if is_security_token_exempt_path(path) {
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

fn parse_loopback_authority(value: &str) -> Option<(String, u16)> {
    let authority = value.parse::<Authority>().ok()?;
    let host = authority.host().to_ascii_lowercase();
    if host != "127.0.0.1" && host != "localhost" {
        return None;
    }

    Some((host, authority.port_u16()?))
}

fn has_valid_local_request_headers(headers: &HeaderMap) -> bool {
    let host = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_loopback_authority);
    let Some(host) = host else {
        return false;
    };

    let Some(origin) = headers.get("origin") else {
        return true;
    };
    let Ok(origin) = origin.to_str() else {
        return false;
    };
    let Ok(origin) = origin.parse::<Uri>() else {
        return false;
    };
    if origin.scheme_str() != Some("http") {
        return false;
    }
    if origin
        .path_and_query()
        .is_some_and(|path| path.as_str() != "/")
    {
        return false;
    }

    origin
        .authority()
        .and_then(|authority| parse_loopback_authority(authority.as_str()))
        .is_some_and(|origin| origin == host)
}

fn is_security_token_exempt_path(path: &str) -> bool {
    path == "/"
        || path == "/api/health"
        || path.starts_with("/assets/")
        || path.starts_with("/scripts/")
        || path.starts_with("/styles/")
        || path.starts_with("/api/pages/")
        || path == "/oauth/callback"
        || (path.starts_with("/api/") && path.ends_with("/stream"))
        || !path.starts_with("/api/")
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

    // If initialized, allow everything. preview mode (wallet + RPC skipped) is
    // also treated as "allowed": the dashboard is usable for token discovery/browsing,
    // and wallet/RPC-dependent endpoints enforce their own deeper guards
    // (are_core_services_ready / FORCE_STOP) so trading still cannot happen.
    if global::is_initialization_complete() || global::is_preview_mode() {
        return next.run(request).await;
    }

    // Not initialized - check if this is an allowed path

    // Allow initialization endpoints and health check
    if path.starts_with("/api/initialization")
        || path.starts_with("/api/system/bootstrap")
        || path == "/api/health"
        || path == "/api/version"
    {
        return next.run(request).await;
    }

    // The ScreenerBot account panel sits on the setup screen, so it has to work
    // before setup is complete. Allowed HERE and not in `security_gate`: these
    // routes still require the GUI token, which is what keeps a page in another
    // browser tab from driving them over 127.0.0.1.
    if path.starts_with("/api/account") {
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
/// Adds appropriate Cache-Control headers based on resource type:
///
/// **Static assets** (`/scripts/*`, `/assets/*`, `/fonts/*`):
/// - These are compile-time embedded resources that only change on new builds
/// - Use aggressive caching: `public, max-age=31536000, immutable`
/// - Reduces bandwidth and improves page load performance
///
/// **API endpoints** (`/api/*`):
/// - Dynamic data that must never be cached
/// - Use `no-cache, no-store, must-revalidate, max-age=0`
///
/// **HTML pages** (everything else):
/// - Allow conditional caching with `no-cache`
/// - Browser validates freshness but can cache if unchanged
pub async fn cache_control(request: Request, next: Next) -> Response {
    let path = request.uri().path().to_string();
    let has_version = request.uri().query().is_some_and(|q| q.contains("v="));
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Static assets with version hash - aggressive caching (immutable, 1 year)
    if has_version
        && (path.starts_with("/scripts/")
            || path.starts_with("/assets/")
            || path.starts_with("/fonts/"))
    {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }
    // Static assets without version hash - short cache with revalidation
    else if path.starts_with("/scripts/")
        || path.starts_with("/assets/")
        || path.starts_with("/fonts/")
    {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=3600, must-revalidate"),
        );
    }
    // API endpoints - no caching
    else if path.starts_with("/api/") {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("no-cache, no-store, must-revalidate, max-age=0"),
        );
        headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
        headers.insert(header::EXPIRES, HeaderValue::from_static("0"));
    }
    // HTML pages - conditional caching
    else {
        headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    }

    response
}

/// Authentication gate middleware (headless mode only)
///
/// In headless/CLI mode with auth enabled, validates that all requests
/// include a valid session cookie. Redirects to /login if not authenticated.
///
/// Allowed without authentication:
/// - /login - login page
/// - /api/auth/* - authentication endpoints
/// - /scripts/* - static JavaScript files
/// - /styles/* - static CSS files  
/// - /assets/* - static assets (images, fonts)
///
/// In GUI mode, this middleware does nothing (GUI uses security token instead).
pub async fn auth_gate(request: Request, next: Next) -> Response {
    use crate::config;
    use crate::webserver::routes::auth::extract_session_token;
    use crate::webserver::session;

    // Skip auth check in GUI mode (uses security token instead)
    if crate::global::is_gui_mode() {
        return next.run(request).await;
    }

    // Check if auth is enabled
    let auth_enabled = config::with_config(|cfg| cfg.webserver.auth_enabled);
    if !auth_enabled {
        return next.run(request).await;
    }

    let path = request.uri().path();

    // Allow login page and auth API endpoints without authentication
    if path == "/login"
        || path.starts_with("/api/auth/")
        || path.starts_with("/scripts/")
        || path.starts_with("/styles/")
        || path.starts_with("/assets/")
    {
        return next.run(request).await;
    }

    // Check for valid session cookie
    if let Some(token) = extract_session_token(&request) {
        if session::validate_session(&token) {
            return next.run(request).await;
        }
    }

    // Not authenticated - redirect to login for page requests, return 401 for API
    if path.starts_with("/api/") {
        return utils::error_response(
            StatusCode::UNAUTHORIZED,
            "AUTHENTICATION_REQUIRED",
            "Authentication required",
            Some("Please log in to access this endpoint"),
        );
    }

    // Redirect to login page for HTML page requests
    Response::builder()
        .status(StatusCode::FOUND)
        .header(header::LOCATION, "/login")
        .body(Body::empty())
        .unwrap()
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request_headers(host: Option<&str>, origin: Option<&str>) -> HeaderMap {
        let mut headers = HeaderMap::new();
        if let Some(host) = host {
            headers.insert(header::HOST, host.parse().unwrap());
        }
        if let Some(origin) = origin {
            headers.insert("origin", origin.parse().unwrap());
        }
        headers
    }

    #[test]
    fn accepts_loopback_host_and_same_origin() {
        assert!(has_valid_local_request_headers(&request_headers(
            Some("127.0.0.1:54321"),
            Some("http://127.0.0.1:54321")
        )));
        assert!(has_valid_local_request_headers(&request_headers(
            Some("localhost:54321"),
            None
        )));
    }

    #[test]
    fn rejects_non_loopback_or_cross_origin_requests() {
        for headers in [
            request_headers(None, None),
            request_headers(Some("example.com:54321"), None),
            request_headers(Some("127.0.0.1"), None),
            request_headers(Some("127.0.0.1:54321"), Some("null")),
            request_headers(Some("127.0.0.1:54321"), Some("https://127.0.0.1:54321")),
            request_headers(Some("127.0.0.1:54321"), Some("http://127.0.0.1:54322")),
            request_headers(Some("127.0.0.1:54321"), Some("http://example.com:54321")),
        ] {
            assert!(!has_valid_local_request_headers(&headers));
        }
    }

    #[test]
    fn only_passive_local_routes_skip_the_token() {
        for path in [
            "/",
            "/api/health",
            "/assets/logo.png",
            "/scripts/app.js",
            "/styles/app.css",
            "/api/pages/dashboard",
            "/api/tokens/stream",
            "/oauth/callback",
            "/services",
        ] {
            assert!(is_security_token_exempt_path(path), "{path}");
        }

        for path in [
            "/api/initialization/start",
            "/api/system/bootstrap",
            "/api/actions/run",
            "/api/services/start",
            "/api/account/status",
            "/api/wallet/export",
        ] {
            assert!(!is_security_token_exempt_path(path), "{path}");
        }
    }
}
