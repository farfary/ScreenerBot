// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Webserver middleware
//!
//! Request interceptors for authentication, validation, gating, and cache control

use crate::database::{self, catalog::StoreSpec, health::StoreHealth};
use crate::i18n::ids;
use crate::webserver::api_error::{ApiError, ApiErrorCode};
use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, header::HeaderValue, uri::Authority, HeaderMap, StatusCode, Uri},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::{
    config, global,
    logger::{self, LogTag},
};

/// Security header name for token validation
pub const SECURITY_TOKEN_HEADER: &str = "X-ScreenerBot-Token";

/// Security gate middleware
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
/// - Static assets (/assets/*, /scripts/*, /styles/*, /i18n/*)
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
/// In headless mode the dashboard has no security token. With password
/// authentication enabled the session cookie protects it and the server may be
/// reached through any address. Without it the server is bound to loopback
/// only (`validate_headless_bind`), and the same loopback `Host`/`Origin` check
/// applies, so a page that rebinds its own hostname to 127.0.0.1 cannot drive
/// the API from the user's browser.
pub async fn security_gate(request: Request, next: Next) -> Response {
    if !global::is_gui_mode() {
        let auth_enabled = config::with_config(|cfg| cfg.webserver.auth_enabled);
        if headless_request_allowed(auth_enabled, request.headers()) {
            return next.run(request).await;
        }
        logger::warning(
            LogTag::Webserver,
            &format!(
                "Blocked request to {} - not a local dashboard request and authentication is disabled",
                request.uri().path()
            ),
        );
        return ApiError::new(
            ApiErrorCode::InvalidLocalRequest,
            ids::ERRORS_SECURITY_INVALID_LOCAL_REQUEST,
        )
        .into_response();
    }

    if !has_valid_local_request_headers(request.headers()) {
        logger::warning(
            LogTag::Webserver,
            &format!(
                "Blocked request to {} - invalid local request headers",
                request.uri().path()
            ),
        );
        return ApiError::new(
            ApiErrorCode::InvalidLocalRequest,
            ids::ERRORS_SECURITY_INVALID_LOCAL_REQUEST,
        )
        .into_response();
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
            ApiError::new(
                ApiErrorCode::InvalidToken,
                ids::ERRORS_SECURITY_INVALID_TOKEN,
            )
            .into_response()
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
            ApiError::new(
                ApiErrorCode::MissingToken,
                ids::ERRORS_SECURITY_TOKEN_REQUIRED,
            )
            .details("This endpoint is only accessible from within ScreenerBot")
            .into_response()
        }
    }
}

/// Whether a headless request may proceed: password authentication guards the
/// dashboard on its own; without it only a loopback same-origin request does.
fn headless_request_allowed(auth_enabled: bool, headers: &HeaderMap) -> bool {
    auth_enabled || has_valid_local_request_headers(headers)
}

/// The host and port of `value` when it names this machine: `localhost` or a
/// loopback IP literal (which no DNS answer can rebind), with an explicit port.
fn parse_loopback_authority(value: &str) -> Option<(String, u16)> {
    let authority = value.parse::<Authority>().ok()?;
    let host = authority.host().to_ascii_lowercase();
    let is_loopback = host == "localhost"
        || host
            .trim_start_matches('[')
            .trim_end_matches(']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback());
    if !is_loopback {
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

/// The loopback-only restart check. Its handler refuses any non-loopback peer, so the
/// gates that protect private data let it through.
fn is_restart_safety_path(path: &str) -> bool {
    path.strip_prefix("/api") == Some(crate::webserver::routes::status::RESTART_SAFETY_PATH)
}

fn is_security_token_exempt_path(path: &str) -> bool {
    path == "/"
        || path == "/api/health"
        || is_restart_safety_path(path)
        || path.starts_with("/assets/")
        || path.starts_with("/scripts/")
        || path.starts_with("/styles/")
        || path.starts_with("/i18n/")
        || path.starts_with("/api/pages/")
        || path == "/oauth/callback"
        || (path.starts_with("/api/") && path.ends_with("/stream"))
        // The live-app bridge for external agents. Exempt from the GUI security
        // token ONLY. This function is reached only from `security_gate`, which
        // runs only in GUI mode and has already enforced the loopback
        // `Host`/`Origin` check before consulting it. Every bridge handler also
        // authenticates a pairing credential unconditionally (see
        // `routes::agent_bridge`). Deliberately the exact prefix, nothing wider:
        // `tests/architecture_boundaries.rs` proves every other
        // agent-control/dashboard route stays token-protected.
        || path.starts_with(crate::webserver::routes::agent_bridge::BRIDGE_PREFIX)
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

    // If initialized, allow everything. Explore Mode (wallet + RPC skipped) is
    // also treated as "allowed": the dashboard is usable for token discovery/browsing,
    // wallet-backed routes sit behind `full_setup_gate`, and RPC-dependent endpoints
    // enforce their own deeper guards (are_core_services_ready / FORCE_STOP) so
    // trading still cannot happen.
    if global::is_initialization_complete() || global::is_explore_mode() {
        return next.run(request).await;
    }

    // Not initialized - check if this is an allowed path

    // Allow initialization endpoints and health check
    if path.starts_with("/api/initialization")
        || path.starts_with("/api/system/bootstrap")
        || path == "/api/health"
        || is_restart_safety_path(path)
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

    ApiError::new(
        ApiErrorCode::InitializationRequired,
        ids::ERRORS_INITIALIZATION_REQUIRED,
    )
    .details("Please complete the initialization process through the web interface")
    .into_response()
}

/// Explore Mode gate for routes that need the wallet a full setup opens
///
/// Explore Mode passes `initialization_gate` so the discovery pages work without a
/// wallet, but the wallet store, the watch store and every copy task are opened or
/// funded only by a full setup. Those routes answer `INITIALIZATION_REQUIRED` with a
/// translated message instead of failing inside a store that was never opened.
pub async fn full_setup_gate(request: Request, next: Next) -> Response {
    if !requires_full_setup(
        global::is_initialization_complete(),
        crate::webserver::promo::are_promo_fixtures_enabled(),
    ) {
        return next.run(request).await;
    }
    logger::debug(
        LogTag::Webserver,
        &format!(
            "Blocked Explore Mode request to {} (full setup required)",
            request.uri().path()
        ),
    );
    ApiError::new(
        ApiErrorCode::InitializationRequired,
        ids::ERRORS_INITIALIZATION_EXPLORE_UNAVAILABLE,
    )
    .into_response()
}

/// Store gate for the route group of a feature whose data lives in one catalog store
///
/// While the storage phase has recorded the store as not open (unavailable, or being
/// rebuilt), every route of the group answers `STORE_UNAVAILABLE` (503) naming the
/// store, instead of failing inside a store its owner never opened. Attached once per
/// feature with `route_layer(from_fn_with_state(&catalog::STORE, store_gate))`.
pub async fn store_gate(
    State(store): State<&'static StoreSpec>,
    request: Request,
    next: Next,
) -> Response {
    let Some(status) = database::health::first_unavailable(store) else {
        return next.run(request).await;
    };
    logger::debug(
        LogTag::Webserver,
        &format!(
            "Blocked request to {}: store {} is {}",
            request.uri().path(),
            store.id,
            status.health.as_str()
        ),
    );
    let details = match &status.health {
        StoreHealth::Unavailable {
            failure, detail, ..
        } => format!("{}: {detail}", failure.as_str()),
        other => other.as_str().to_owned(),
    };
    ApiError::store_unavailable(store.id)
        .details(details)
        .into_response()
}

/// Promotional fixtures answer wallet reads without a wallet, so a capture passes.
fn requires_full_setup(initialization_complete: bool, promo_fixtures: bool) -> bool {
    !initialization_complete && !promo_fixtures
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
            || path.starts_with("/styles/")
            || path.starts_with("/assets/")
            || path.starts_with("/i18n/")
            || path.starts_with("/fonts/"))
    {
        headers.insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=31536000, immutable"),
        );
    }
    // Static assets without version hash - short cache with revalidation
    else if path.starts_with("/scripts/")
        || path.starts_with("/styles/")
        || path.starts_with("/assets/")
        || path.starts_with("/i18n/")
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

/// Paths the dashboard password does not guard: the login page and its API, static
/// files, the health check (status, version and instance id only), the loopback-only
/// restart check, and the external-agent bridge.
fn is_auth_exempt_path(path: &str) -> bool {
    path == "/login"
        || path == "/api/health"
        || is_restart_safety_path(path)
        || path.starts_with("/api/auth/")
        || path.starts_with("/scripts/")
        || path.starts_with("/styles/")
        || path.starts_with("/i18n/")
        || path.starts_with("/assets/")
        // The external-agent bridge does not carry a dashboard session; every
        // handler authenticates a mandatory pairing bearer credential instead,
        // in every mode. That credential — not a loopback guarantee — is what
        // protects the surface: with password auth enabled a headless server
        // can be bound to a non-loopback address, and `security_gate` skips
        // its Host/Origin check for a headless server with authentication
        // enabled. Without authentication, and in GUI mode, that local
        // Host/Origin check applies to this path like any other.
        || path.starts_with(crate::webserver::routes::agent_bridge::BRIDGE_PREFIX)
}

/// Authentication gate middleware (headless mode only)
///
/// In headless/CLI mode with auth enabled, validates that all requests
/// include a valid session cookie. Redirects to /login if not authenticated.
///
/// Allowed without authentication: [`is_auth_exempt_path`].
///
/// In GUI mode, this middleware does nothing (GUI uses security token instead).
pub async fn auth_gate(request: Request, next: Next) -> Response {
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

    if is_auth_exempt_path(path) {
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
        return ApiError::new(
            ApiErrorCode::AuthenticationRequired,
            ids::ERRORS_AUTH_REQUIRED,
        )
        .details("Please log in to access this endpoint")
        .into_response();
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
    fn loopback_ip_literals_are_local_hosts() {
        assert!(has_valid_local_request_headers(&request_headers(
            Some("[::1]:8080"),
            Some("http://[::1]:8080")
        )));
        assert!(has_valid_local_request_headers(&request_headers(
            Some("127.0.0.2:8080"),
            None
        )));
        assert!(!has_valid_local_request_headers(&request_headers(
            Some("[::1]:8080"),
            Some("http://127.0.0.1:8080")
        )));
    }

    #[test]
    fn headless_without_auth_admits_only_local_same_origin_requests() {
        let local = request_headers(Some("127.0.0.1:8080"), Some("http://127.0.0.1:8080"));
        let rebound = request_headers(
            Some("rebind.example:8080"),
            Some("http://rebind.example:8080"),
        );
        let cross_origin = request_headers(Some("127.0.0.1:8080"), Some("http://evil.example"));

        assert!(headless_request_allowed(false, &local));
        assert!(!headless_request_allowed(false, &rebound));
        assert!(!headless_request_allowed(false, &cross_origin));
        // Password authentication protects a headless server on any address.
        assert!(headless_request_allowed(true, &rebound));
    }

    #[test]
    fn wallet_routes_require_a_full_setup_outside_promo_capture() {
        assert!(requires_full_setup(false, false));
        assert!(!requires_full_setup(true, false));
        assert!(!requires_full_setup(false, true));
        assert!(!requires_full_setup(true, true));
    }

    #[test]
    fn the_password_gate_spares_only_public_and_loopback_checked_routes() {
        for path in [
            "/login",
            "/api/health",
            "/api/health/restart-safety",
            "/api/auth/login",
            "/scripts/app.js",
            "/i18n/en/catalog.js",
        ] {
            assert!(is_auth_exempt_path(path), "{path}");
        }
        for path in [
            "/api/status",
            "/api/health/other",
            "/api/positions",
            "/api/wallet/export",
            "/health/restart-safety",
            "/",
        ] {
            assert!(!is_auth_exempt_path(path), "{path}");
        }
    }

    #[test]
    fn only_passive_local_routes_skip_the_token() {
        for path in [
            "/",
            "/api/health",
            "/api/health/restart-safety",
            "/assets/logo.png",
            "/scripts/app.js",
            "/styles/app.css",
            "/i18n/en/catalog.js",
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
