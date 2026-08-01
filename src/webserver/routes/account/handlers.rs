//! Handlers for the ScreenerBot account panel.

use axum::{extract::Query, http::StatusCode, response::Response, Json};
use serde::{Deserialize, Serialize};

use crate::{account, paths};
use crate::webserver::utils::{error_response, success_response};

#[derive(Serialize)]
struct Envelope<T: Serialize> {
    success: bool,
    data: T,
}

fn ok<T: Serialize>(data: T) -> Response {
    success_response(Envelope {
        success: true,
        data,
    })
}

/// Turn a domain error into the sentence the user reads.
///
/// The server's own wording is passed through unchanged wherever it exists, so
/// the app and the website never explain the same failure two different ways.
fn refused(error: crate::Error) -> Response {
    error_response(
        StatusCode::UNAUTHORIZED,
        "ACCOUNT_SIGNIN_FAILED",
        &error.to_string(),
        None,
    )
}

/// GET /api/account/status
pub async fn account_status() -> Response {
    ok(account::status())
}

#[derive(Serialize)]
pub struct BrowserSignInStarted {
    opened: bool,
}

/// POST /api/account/signin/browser
///
/// Starting the authorization and opening it are intentionally one backend
/// operation. The account panel is available before initialization, while the
/// generic system API is not; splitting this across those two API domains made
/// a valid sign-in depend on a route the initialization gate rejected.
pub async fn start_browser_signin() -> Response {
    match account::begin_browser_signin() {
        Ok(url) => match paths::open_url_in_browser(&url) {
            Ok(()) => ok(BrowserSignInStarted { opened: true }),
            Err(error) => {
                account::cancel_browser_signin();
                error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "ACCOUNT_BROWSER_OPEN_FAILED",
                    &error,
                    Some("Open your default browser and try again"),
                )
            }
        },
        Err(error) => error_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "ACCOUNT_SIGNIN_UNAVAILABLE",
            &error.to_string(),
            None,
        ),
    }
}

/// POST /api/account/signup
///
/// Like browser sign-in, account creation belongs wholly to the account API so
/// it remains usable on the setup screen without widening the generic system
/// API before initialization.
pub async fn open_signup() -> Response {
    const SIGN_UP_URL: &str = "https://screenerbot.io/signup";

    match paths::open_url_in_browser(SIGN_UP_URL) {
        Ok(()) => ok(BrowserSignInStarted { opened: true }),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ACCOUNT_BROWSER_OPEN_FAILED",
            &error,
            Some("Open screenerbot.io/signup in your browser"),
        ),
    }
}

#[derive(Deserialize)]
pub struct PasswordSignIn {
    email: String,
    /// Held for the length of this request and never stored, logged or written
    /// to config. It exists as this field and as an argument to one HTTPS call.
    password: String,
}

/// POST /api/account/signin/password
pub async fn signin_with_password(Json(body): Json<PasswordSignIn>) -> Response {
    let email = body.email.trim();
    if email.is_empty() || body.password.is_empty() {
        return error_response(
            StatusCode::BAD_REQUEST,
            "INVALID_REQUEST",
            "Enter your email address and password.",
            None,
        );
    }

    match account::sign_in_with_password(email, &body.password).await {
        Ok(()) => ok(account::status()),
        Err(error) => refused(error),
    }
}

#[derive(Deserialize)]
pub struct WalletSignIn {
    /// The user's explicit answer to "no account uses this wallet — create
    /// one?". Defaults to false: proving a key may adopt an existing account,
    /// never conjure a new one.
    #[serde(default)]
    create: bool,
}

/// POST /api/account/signin/wallet
pub async fn signin_with_wallet(Json(body): Json<WalletSignIn>) -> Response {
    match account::sign_in_with_wallet(body.create).await {
        Ok(()) => ok(account::status()),
        Err(error) => refused(error),
    }
}

#[derive(Serialize)]
pub struct WalletAccountCheck {
    has_account: bool,
}

/// GET /api/account/signin/wallet/check
///
/// Lets the setup screen offer the right thing — "sign in as this wallet"
/// rather than a generic prompt — before the user commits to signing anything.
pub async fn check_wallet_account() -> Response {
    ok(WalletAccountCheck {
        has_account: account::wallet_has_account().await,
    })
}

#[derive(Serialize)]
pub struct DeviceSignInStarted {
    user_code: String,
    verification_uri: String,
    device_code: String,
    interval: u64,
    expires_in: u64,
}

/// POST /api/account/signin/device
pub async fn start_device_signin() -> Response {
    match account::begin_device_signin().await {
        Ok(response) => ok(DeviceSignInStarted {
            user_code: response.user_code,
            verification_uri: response.verification_uri,
            device_code: response.device_code,
            interval: response.interval,
            expires_in: response.expires_in,
        }),
        Err(error) => refused(error),
    }
}

#[derive(Deserialize)]
pub struct DevicePoll {
    device_code: String,
}

#[derive(Serialize)]
pub struct DevicePollResult {
    signed_in: bool,
}

/// POST /api/account/signin/device/poll
pub async fn poll_device_signin(Json(body): Json<DevicePoll>) -> Response {
    match account::poll_device_signin(&body.device_code).await {
        Ok(signed_in) => ok(DevicePollResult { signed_in }),
        Err(error) => refused(error),
    }
}

/// POST /api/account/signout
pub async fn signout() -> Response {
    match account::sign_out() {
        Ok(()) => ok(account::status()),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "ACCOUNT_SIGNOUT_FAILED",
            &error.to_string(),
            None,
        ),
    }
}

#[derive(Deserialize)]
pub struct GatewayToggle {
    enabled: bool,
}

/// POST /api/account/gateway
///
/// The setup screen needs to write this before initialization is complete, and
/// `/api/config/*` is blocked until then. Rather than punching a second hole in
/// `initialization_gate` for the whole config API, the one field the setup
/// screen touches gets its own route inside the prefix that is already allowed.
/// It stays in memory until setup completion or Preview Mode persists the full
/// configuration, so changing this optional switch cannot create config.toml
/// before the primary setup decision.
pub async fn set_gateway_enabled(Json(body): Json<GatewayToggle>) -> Response {
    let result = crate::config::update_config_section(
        |cfg| {
            cfg.account.use_gateway_rpc = body.enabled;
        },
        false,
    );

    match result {
        Ok(()) => ok(account::status()),
        Err(error) => error_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "CONFIG_WRITE_FAILED",
            &error.to_string(),
            None,
        ),
    }
}

#[derive(Deserialize)]
pub struct OAuthCallback {
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    error: Option<String>,
}

/// GET /oauth/callback — where the system browser lands after consent.
///
/// ============================================================================
/// WHY THIS ROUTE IS EXEMPT FROM THE SECURITY TOKEN, AND WHY THAT IS SAFE
/// ============================================================================
/// A browser redirect cannot carry `X-ScreenerBot-Token`, so this route has to
/// be reachable without it. Three things make that harmless:
///
///   1. It accepts nothing but a `code` and a `state`, and BOTH are checked
///      against a PendingAuth that only exists in memory, only while a sign-in
///      this process started is running.
///   2. `state` is compared in constant time. A page that guessed the port and
///      fired a request at it has no way to produce the right value.
///   3. The response is a fixed page. It echoes no parameter back, so there is
///      nothing here to reflect.
///
/// It is also why the sign-in API itself lives under `/api/account` and NOT
/// under one of the token-exempt prefixes: this is the only route that needs
/// the exemption, and it is the only one that gets it.
pub async fn oauth_callback(Query(params): Query<OAuthCallback>) -> Response {
    if let Some(error) = params.error {
        return callback_page(
            "Sign-in was cancelled",
            &format!("You can close this window and try again from ScreenerBot. ({error})"),
        );
    }

    let (Some(code), Some(state)) = (params.code, params.state) else {
        return callback_page(
            "That link is incomplete",
            "Close this window and start sign-in again from ScreenerBot.",
        );
    };

    match account::complete_browser_signin(&code, &state).await {
        Ok(()) => callback_page(
            "You are signed in",
            "You can close this window and go back to ScreenerBot.",
        ),
        Err(error) => callback_page("Sign-in could not be completed", &error.to_string()),
    }
}

/// A self-contained page. No script, no asset, no reflected input — it has to
/// render in whatever browser the OS chose, including one that has never loaded
/// the dashboard.
fn callback_page(title: &str, detail: &str) -> Response {
    let body = format!(
        r#"<!DOCTYPE html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>ScreenerBot</title>
<style>
  body {{ margin:0; min-height:100vh; display:flex; align-items:center; justify-content:center;
         background:#0d0d0f; color:#f0f0f2;
         font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,Helvetica,Arial,sans-serif; }}
  .card {{ max-width:26rem; padding:2rem; text-align:center; }}
  h1 {{ font-size:1.15rem; margin:0 0 .75rem; }}
  p {{ font-size:.9rem; line-height:1.6; color:#aaaab1; margin:0; }}
</style></head>
<body><div class="card"><h1>{}</h1><p>{}</p></div></body></html>"#,
        escape_html(title),
        escape_html(detail)
    );

    axum::response::Html(body).into_response()
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

use axum::response::IntoResponse;
