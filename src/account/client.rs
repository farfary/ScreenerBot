// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Every HTTP call the account subsystem makes, and the only place the server
//! address appears.
//!
//! ============================================================================
//! THE HOST IS A CONSTANT. IT MUST NEVER BECOME A CONFIG FIELD.
//! ============================================================================
//! This module carries an email address and a password on one of its paths. A
//! configurable endpoint would mean anyone who can edit `config.toml` — or
//! anyone who can talk a user into pasting a "fix" into it — can redirect those
//! credentials to a server they own, and `config.toml` is a file people share
//! in bug reports without thinking.
//!
//! `referral.endpoint` is configurable and that is fine: it transmits nothing
//! but public wallet addresses, so redirecting it costs the user nothing.
//! Nothing in this file may follow that precedent.

use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::errors::{AccountError, Error, NetworkError, Result};

/// The only server this module will talk to.
const API_BASE: &str = "https://screenerbot.io";

/// A public identifier, not a credential. Safe in a public binary by design.
pub const CLIENT_ID: &str = "screenerbot-desktop";

const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

/// A refresh runs while data consumers wait on the refresh lock, so it gets a
/// shorter budget than an interactive sign-in. A response lost to this timeout
/// is recoverable: the server answers a spent token presented again within its
/// reuse window with a fresh pair.
const REFRESH_TIMEOUT: Duration = Duration::from_secs(10);

/// Revocation is best-effort and runs after the local sign-out has finished.
const REVOKE_TIMEOUT: Duration = Duration::from_secs(5);

/// What the app asks to be allowed to do. The user sees each of these spelled
/// out on the consent screen before anything is granted.
///
/// `data:read` must be requested explicitly even though the server grants the
/// full default set to a request it cannot parse: scopes are FROZEN on the
/// device at grant time, so a build that asks for the old four and is granted
/// exactly those has no path to data access short of signing in again.
pub const SCOPES: &str = "data:read rpc:submit vote referral:read account:read";

#[derive(Debug, Clone, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: String,
    #[serde(default)]
    pub scope: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WalletChallenge {
    pub message: String,
    #[serde(default)]
    pub has_account: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeviceCodeResponse {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    /// The same page with the code already filled in (RFC 8628 section 3.3.1).
    #[serde(default)]
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
    pub interval: u64,
}

/// One answer to a device-code poll that is not a final failure.
#[derive(Debug, Clone)]
pub enum DevicePoll {
    /// The user approved the device; the token pair is ready to adopt.
    Approved(TokenResponse),
    /// `authorization_pending`: keep polling at the current interval.
    Pending,
    /// `slow_down`, a rate limit or a transient server failure: add
    /// [`DEVICE_SLOW_DOWN_STEP`] to the interval and keep polling.
    SlowDown,
    /// `expired_token`: the code is dead and a new one is needed.
    Expired,
}

/// RFC 8628 section 3.5: every `slow_down` adds five seconds to the interval.
pub const DEVICE_SLOW_DOWN_STEP: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountProfile {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Deserialize)]
struct ErrorBody {
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    error_description: Option<String>,
}

fn client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        // TLS verification is reqwest's default and is never relaxed here. A
        // password crosses this connection.
        .build()
        .map_err(|e| {
            Error::Network(NetworkError::RequestFailed {
                endpoint: "account http client".to_owned(),
                detail: e.to_string(),
            })
        })
}

/// The URL the SYSTEM BROWSER is sent to. Never an embedded webview: an
/// embedded browser can read what the user types, which defeats the entire
/// point of brokering sign-in through a browser the app does not control (and
/// Google refuses OAuth from webviews for exactly this reason).
pub fn authorize_url(challenge: &str, state: &str, redirect_uri: &str) -> String {
    let query = [
        ("client_id", CLIENT_ID),
        ("code_challenge", challenge),
        ("code_challenge_method", "S256"),
        ("redirect_uri", redirect_uri),
        ("state", state),
        ("scope", SCOPES),
        ("device_label", &device_label()),
        ("platform", std::env::consts::OS),
        ("app_version", env!("CARGO_PKG_VERSION")),
    ]
    .iter()
    .map(|(key, value)| format!("{}={}", key, urlencode(value)))
    .collect::<Vec<_>>()
    .join("&");

    format!("{API_BASE}/app/authorize?{query}")
}

/// What the user will see in their Devices list. The machine's hostname is the
/// one label that lets somebody recognise which install to revoke.
pub fn device_label() -> String {
    match hostname() {
        Some(name) if !name.trim().is_empty() => format!("ScreenerBot on {}", name.trim()),
        _ => "ScreenerBot desktop".to_string(),
    }
}

fn hostname() -> Option<String> {
    // No new dependency for one string: every supported platform exposes it in
    // the environment or through `hostname`.
    for key in ["HOSTNAME", "COMPUTERNAME", "NAME"] {
        if let Ok(value) = std::env::var(key) {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
    }

    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn urlencode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            b' ' => "%20".to_string(),
            other => format!("%{other:02X}"),
        })
        .collect()
}

fn token_endpoint() -> String {
    format!("{API_BASE}/api/app/token")
}

async fn post_token(body: serde_json::Value) -> Result<TokenResponse> {
    post_token_to(&token_endpoint(), body, REQUEST_TIMEOUT).await
}

async fn post_token_to(
    url: &str,
    body: serde_json::Value,
    timeout: Duration,
) -> Result<TokenResponse> {
    let response = client()?
        .post(url)
        .timeout(timeout)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            Error::Network(NetworkError::RequestFailed {
                endpoint: "screenerbot.io".to_owned(),
                detail: e.to_string(),
            })
        })?;

    let status = response.status();
    if status.is_success() {
        return response.json::<TokenResponse>().await.map_err(|e| {
            Error::Account(AccountError::UnexpectedResponse {
                message: e.to_string(),
            })
        });
    }

    let body = response.bytes().await.unwrap_or_default();
    Err(Error::Account(match token_failure(status, &body) {
        TokenFailure::Unavailable { detail } => AccountError::Unavailable {
            status: status.as_u16(),
            detail,
        },
        TokenFailure::Refused { code, message } => AccountError::Refused { code, message },
    }))
}

/// What a non-success answer from the token endpoint says about the grant.
#[derive(Debug, PartialEq, Eq)]
enum TokenFailure {
    /// Says nothing about the grant: retry later.
    Unavailable { detail: String },
    /// The server's verdict, in its own sentence; `code` only on a 400.
    Refused {
        code: Option<String>,
        message: String,
    },
}

/// Classify a non-success answer from the token endpoint.
///
/// Only a 400 carries an OAuth error code (RFC 6749 section 5.2). A 429 from the
/// rate limiter has a body of its own shape, and a 5xx or a gateway error may
/// have any body or none, so those are `Unavailable` whatever they contain: they
/// say nothing about the grant. Any other status is a refusal, worded by the
/// server's own sentence when it sent one.
fn token_failure(status: reqwest::StatusCode, body: &[u8]) -> TokenFailure {
    let parsed = serde_json::from_slice::<ErrorBody>(body).ok();
    let description = parsed
        .as_ref()
        .and_then(|body| body.error_description.clone());
    let error = parsed.as_ref().and_then(|body| body.error.clone());

    if status == reqwest::StatusCode::TOO_MANY_REQUESTS || status.is_server_error() {
        return TokenFailure::Unavailable {
            detail: description.or(error).unwrap_or_else(|| status.to_string()),
        };
    }

    let code = if status == reqwest::StatusCode::BAD_REQUEST {
        error.clone()
    } else {
        None
    };

    // The server writes a full sentence for every failure. Surfacing ITS
    // sentence rather than inventing one keeps the app and the website saying
    // the same thing about the same problem.
    match description.or(error) {
        Some(message) => TokenFailure::Refused { code, message },
        None => TokenFailure::Unavailable {
            detail: "the response carried no OAuth error".to_string(),
        },
    }
}

/// Exchange an authorization code, proving with the verifier that this process
/// is the one that started the flow.
pub async fn exchange_code(
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<TokenResponse> {
    post_token(serde_json::json!({
        "grant_type": "authorization_code",
        "code": code,
        "code_verifier": verifier,
        "redirect_uri": redirect_uri,
    }))
    .await
}

/// Rotate. The old refresh token is spent by this call and must be replaced
/// with the one that comes back — presenting it twice revokes the device.
pub async fn refresh(refresh_token: &str) -> Result<TokenResponse> {
    post_token_to(
        &token_endpoint(),
        serde_json::json!({
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
        }),
        REFRESH_TIMEOUT,
    )
    .await
}

/// Revoke the device behind a refresh token (RFC 7009).
///
/// The server answers 200 for an unknown token too, so any non-success status
/// is a transport-level failure rather than a verdict. Callers treat every
/// failure as non-fatal: the local sign-out has already happened.
pub async fn revoke(refresh_token: &str) -> Result<()> {
    revoke_at(&format!("{API_BASE}/api/app/revoke"), refresh_token).await
}

async fn revoke_at(url: &str, refresh_token: &str) -> Result<()> {
    let response = client()?
        .post(url)
        .timeout(REVOKE_TIMEOUT)
        .json(&serde_json::json!({ "refresh_token": refresh_token }))
        .send()
        .await
        .map_err(|e| {
            Error::Network(NetworkError::RequestFailed {
                endpoint: "screenerbot.io".to_owned(),
                detail: e.to_string(),
            })
        })?;

    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    Err(Error::Account(AccountError::Unavailable {
        status: status.as_u16(),
        detail: "revocation was not acknowledged".to_string(),
    }))
}

/// Email and password, straight from the app. The password exists only as this
/// argument: it is never written to config, never logged, and never retained.
pub async fn sign_in_with_password(email: &str, password: &str) -> Result<TokenResponse> {
    post_token(serde_json::json!({
        "grant_type": "password",
        "email": email,
        "password": password,
        "device_label": device_label(),
        "platform": std::env::consts::OS,
        "app_version": env!("CARGO_PKG_VERSION"),
        "scope": SCOPES,
    }))
    .await
}

/// Ask for the message this wallet should sign.
pub async fn wallet_challenge(wallet: &str) -> Result<WalletChallenge> {
    let response = client()?
        .post(format!("{API_BASE}/api/app/wallet/nonce"))
        .json(&serde_json::json!({ "wallet": wallet }))
        .send()
        .await
        .map_err(|e| {
            Error::Network(NetworkError::RequestFailed {
                endpoint: "screenerbot.io".to_owned(),
                detail: e.to_string(),
            })
        })?;

    if !response.status().is_success() {
        return Err(Error::Account(AccountError::Refused {
            code: None,
            message: "Could not start wallet sign-in.".to_string(),
        }));
    }

    response.json::<WalletChallenge>().await.map_err(|e| {
        Error::Account(AccountError::UnexpectedResponse {
            message: e.to_string(),
        })
    })
}

/// Redeem a wallet signature. `create` is false unless the user explicitly
/// asked for a new account — see the note on the server route.
pub async fn sign_in_with_wallet(
    wallet: &str,
    signature: &str,
    message: &str,
    create: bool,
) -> Result<TokenResponse> {
    post_token(serde_json::json!({
        "grant_type": "wallet_signature",
        "wallet": wallet,
        "signature": signature,
        "message": message,
        "create": create,
        "device_label": device_label(),
        "platform": std::env::consts::OS,
        "app_version": env!("CARGO_PKG_VERSION"),
        "scope": SCOPES,
    }))
    .await
}

/// Begin the headless flow, for an install with no browser to open.
pub async fn start_device_flow() -> Result<DeviceCodeResponse> {
    let response = client()?
        .post(format!("{API_BASE}/api/app/device"))
        .json(&serde_json::json!({
            "client_id": CLIENT_ID,
            "scope": SCOPES,
            "device_label": device_label(),
            "platform": std::env::consts::OS,
            "app_version": env!("CARGO_PKG_VERSION"),
        }))
        .send()
        .await
        .map_err(|e| {
            Error::Network(NetworkError::RequestFailed {
                endpoint: "screenerbot.io".to_owned(),
                detail: e.to_string(),
            })
        })?;

    if !response.status().is_success() {
        return Err(Error::Account(AccountError::Refused {
            code: None,
            message: "Could not start device sign-in.".to_string(),
        }));
    }

    response.json::<DeviceCodeResponse>().await.map_err(|e| {
        Error::Account(AccountError::UnexpectedResponse {
            message: e.to_string(),
        })
    })
}

/// Poll the headless flow once.
///
/// The OAuth error code decides, never the sentence: `authorization_pending`
/// and `slow_down` keep the flow alive, `expired_token` ends it with a clear
/// state, and every other refusal (`invalid_grant`, `access_denied`) ends it
/// with the server's sentence. A rate limit or a transient server failure backs
/// off like `slow_down`; the caller stops at the device code's own expiry.
pub async fn poll_device_flow(device_code: &str) -> Result<DevicePoll> {
    let answer = post_token(serde_json::json!({
        "grant_type": "device_code",
        "device_code": device_code,
    }))
    .await;
    device_poll_outcome(answer)
}

fn device_poll_outcome(answer: Result<TokenResponse>) -> Result<DevicePoll> {
    match answer {
        Ok(tokens) => Ok(DevicePoll::Approved(tokens)),
        Err(Error::Account(AccountError::Refused {
            code: Some(code), ..
        })) if code == "authorization_pending" => Ok(DevicePoll::Pending),
        Err(Error::Account(AccountError::Refused {
            code: Some(code), ..
        })) if code == "slow_down" => Ok(DevicePoll::SlowDown),
        Err(Error::Account(AccountError::Refused {
            code: Some(code), ..
        })) if code == "expired_token" => Ok(DevicePoll::Expired),
        Err(Error::Account(AccountError::Unavailable { .. }))
        | Err(Error::Network(NetworkError::RequestFailed { .. })) => Ok(DevicePoll::SlowDown),
        Err(error) => Err(error),
    }
}

/// Who is signed in, from the server rather than from a cached label.
pub async fn fetch_profile(access_token: &str) -> Result<AccountProfile> {
    let response = client()?
        .get(format!("{API_BASE}/api/account/session"))
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| {
            Error::Network(NetworkError::RequestFailed {
                endpoint: "screenerbot.io".to_owned(),
                detail: e.to_string(),
            })
        })?;

    if !response.status().is_success() {
        return Err(Error::Account(AccountError::SessionEnded));
    }

    #[derive(Deserialize)]
    struct Envelope {
        #[serde(default)]
        user: Option<AccountProfile>,
    }

    let envelope = response.json::<Envelope>().await.map_err(|e| {
        Error::Account(AccountError::UnexpectedResponse {
            message: e.to_string(),
        })
    })?;

    envelope
        .user
        .ok_or(Error::Account(AccountError::NotSignedIn))
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    fn failure_is_unavailable(failure: &TokenFailure) -> bool {
        matches!(failure, TokenFailure::Unavailable { .. })
    }

    fn refusal_code(error: &Error) -> Option<&str> {
        match error {
            Error::Account(AccountError::Refused { code, .. }) => code.as_deref(),
            _ => None,
        }
    }

    fn is_unavailable(error: &Error) -> bool {
        matches!(error, Error::Account(AccountError::Unavailable { .. }))
    }

    #[test]
    fn a_400_carries_the_oauth_code_and_the_server_sentence() {
        let failure = token_failure(
            StatusCode::BAD_REQUEST,
            br#"{"error":"invalid_grant","error_description":"Your session has ended."}"#,
        );
        assert_eq!(
            failure,
            TokenFailure::Refused {
                code: Some("invalid_grant".to_string()),
                message: "Your session has ended.".to_string(),
            }
        );
    }

    #[test]
    fn rate_limits_and_server_errors_are_unavailable_whatever_their_body() {
        let bodies: [&[u8]; 4] = [
            br#"{"error":"Too many requests. Try again in a minute."}"#,
            br#"{"error":"invalid_grant","error_description":"x"}"#,
            b"<html><body>502 Bad Gateway</body></html>",
            b"",
        ];
        for status in [
            StatusCode::TOO_MANY_REQUESTS,
            StatusCode::INTERNAL_SERVER_ERROR,
            StatusCode::BAD_GATEWAY,
            StatusCode::SERVICE_UNAVAILABLE,
            StatusCode::GATEWAY_TIMEOUT,
        ] {
            for body in bodies {
                let failure = token_failure(status, body);
                assert!(failure_is_unavailable(&failure), "{status}: {failure:?}");
            }
        }
    }

    #[test]
    fn an_unparsable_refusal_is_unavailable_not_a_verdict() {
        for status in [StatusCode::BAD_REQUEST, StatusCode::UNAUTHORIZED] {
            let failure = token_failure(status, b"<html>captive portal</html>");
            assert!(failure_is_unavailable(&failure), "{status}: {failure:?}");
        }
    }

    #[test]
    fn only_a_400_carries_an_oauth_code() {
        let failure = token_failure(
            StatusCode::UNAUTHORIZED,
            br#"{"error":"invalid_grant","error_description":"Continue in browser."}"#,
        );
        assert_eq!(
            failure,
            TokenFailure::Refused {
                code: None,
                message: "Continue in browser.".to_string(),
            }
        );
    }

    fn refusal(code: &str) -> Result<TokenResponse> {
        Err(Error::Account(AccountError::Refused {
            code: Some(code.to_string()),
            message: "A sentence the client must not match on.".to_string(),
        }))
    }

    #[test]
    fn device_polling_follows_the_oauth_code_not_the_sentence() {
        assert!(matches!(
            device_poll_outcome(refusal("authorization_pending")),
            Ok(DevicePoll::Pending)
        ));
        assert!(matches!(
            device_poll_outcome(refusal("slow_down")),
            Ok(DevicePoll::SlowDown)
        ));
        assert!(matches!(
            device_poll_outcome(refusal("expired_token")),
            Ok(DevicePoll::Expired)
        ));
        for code in ["invalid_grant", "access_denied"] {
            assert!(device_poll_outcome(refusal(code)).is_err(), "{code}");
        }
        // Wording that used to be matched no longer decides anything.
        let worded = Err(Error::Account(AccountError::Refused {
            code: None,
            message: "Waiting for approval. Polling...".to_string(),
        }));
        assert!(device_poll_outcome(worded).is_err());
    }

    #[test]
    fn a_rate_limited_or_failed_device_poll_backs_off_and_keeps_polling() {
        let limited = Err(Error::Account(AccountError::Unavailable {
            status: 429,
            detail: "Too many requests.".to_string(),
        }));
        assert!(matches!(
            device_poll_outcome(limited),
            Ok(DevicePoll::SlowDown)
        ));
        let network = Err(Error::Network(NetworkError::RequestFailed {
            endpoint: "screenerbot.io".to_string(),
            detail: "connection reset".to_string(),
        }));
        assert!(matches!(
            device_poll_outcome(network),
            Ok(DevicePoll::SlowDown)
        ));
    }

    async fn serve_once(response: &'static [u8]) -> String {
        serve_recording(response).await.0
    }

    /// Reads the head and, per its `content-length`, the whole body.
    async fn read_request(socket: &mut tokio::net::TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let read = socket.read(&mut buffer).await.unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            let text = String::from_utf8_lossy(&request).into_owned();
            if let Some(head_end) = text.find("\r\n\r\n") {
                let length = text[..head_end]
                    .lines()
                    .find_map(|line| {
                        line.to_ascii_lowercase()
                            .strip_prefix("content-length:")
                            .map(|value| value.trim().parse::<usize>().unwrap_or(0))
                    })
                    .unwrap_or(0);
                if request.len() >= head_end + 4 + length {
                    break;
                }
            }
        }
        String::from_utf8_lossy(&request).into_owned()
    }

    /// Answers one connection and hands back the raw request it received.
    async fn serve_recording(
        response: &'static [u8],
    ) -> (String, tokio::sync::oneshot::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let request = read_request(&mut socket).await;
            let _ = sender.send(request);
            socket.write_all(response).await.unwrap();
        });
        (format!("http://{address}/api/app/token"), receiver)
    }

    #[tokio::test]
    async fn revocation_sends_the_refresh_token_and_accepts_the_acknowledgement() {
        let (url, request) = serve_recording(
            b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 11\r\nConnection: close\r\n\r\n{\"ok\":true}",
        )
        .await;
        revoke_at(&url, "refresh-to-revoke").await.unwrap();
        let request = request.await.unwrap();
        assert!(request.starts_with("POST "), "{request}");
        assert!(
            request.contains(r#"{"refresh_token":"refresh-to-revoke"}"#),
            "{request}"
        );
    }

    #[tokio::test]
    async fn an_unacknowledged_revocation_is_reported_not_ignored() {
        let url = serve_once(
            b"HTTP/1.1 502 Bad Gateway\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        )
        .await;
        let error = revoke_at(&url, "refresh").await.unwrap_err();
        assert!(is_unavailable(&error), "{error:?}");
    }

    #[tokio::test]
    async fn a_gateway_error_page_from_the_token_endpoint_is_unavailable() {
        let url = serve_once(
            b"HTTP/1.1 502 Bad Gateway\r\nContent-Type: text/html\r\nContent-Length: 13\r\nConnection: close\r\n\r\n<html></html>",
        )
        .await;
        let error = post_token_to(&url, serde_json::json!({}), REFRESH_TIMEOUT)
            .await
            .unwrap_err();
        assert!(is_unavailable(&error), "{error:?}");
    }

    #[tokio::test]
    async fn an_invalid_grant_answer_from_the_token_endpoint_is_a_refusal() {
        let url = serve_once(
            b"HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: 67\r\nConnection: close\r\n\r\n{\"error\":\"invalid_grant\",\"error_description\":\"Your session ended.\"}",
        )
        .await;
        let error = post_token_to(&url, serde_json::json!({}), REFRESH_TIMEOUT)
            .await
            .unwrap_err();
        assert_eq!(refusal_code(&error), Some("invalid_grant"));
    }

    #[tokio::test]
    async fn an_unreachable_token_endpoint_is_a_network_failure() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        drop(listener);
        let error = post_token_to(
            &format!("http://{address}/api/app/token"),
            serde_json::json!({}),
            REFRESH_TIMEOUT,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(error, Error::Network(NetworkError::RequestFailed { .. })),
            "{error:?}"
        );
    }
}
