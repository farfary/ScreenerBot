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

/// What the app asks to be allowed to do. The user sees each of these spelled
/// out on the consent screen before anything is granted.
pub const SCOPES: &str = "rpc:submit vote referral:read account:read";

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
    pub expires_in: u64,
    pub interval: u64,
}

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

async fn post_token(body: serde_json::Value) -> Result<TokenResponse> {
    let response = client()?
        .post(format!("{API_BASE}/api/app/token"))
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            Error::Network(NetworkError::RequestFailed {
                endpoint: "screenerbot.io".to_owned(),
                detail: e.to_string(),
            })
        })?;

    if response.status().is_success() {
        return response.json::<TokenResponse>().await.map_err(|e| {
            Error::Account(AccountError::UnexpectedResponse {
                message: e.to_string(),
            })
        });
    }

    // The server writes a full sentence for every failure. Surfacing ITS
    // sentence rather than inventing one keeps the app and the website saying
    // the same thing about the same problem.
    let parsed = response.json::<ErrorBody>().await.ok();
    let message = parsed
        .as_ref()
        .and_then(|body| body.error_description.clone())
        .or_else(|| parsed.as_ref().and_then(|body| body.error.clone()))
        .unwrap_or_else(|| "Sign-in was refused.".to_string());

    Err(Error::Account(AccountError::Refused { message }))
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
    post_token(serde_json::json!({
        "grant_type": "refresh_token",
        "refresh_token": refresh_token,
    }))
    .await
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
            message: "Could not start device sign-in.".to_string(),
        }));
    }

    response.json::<DeviceCodeResponse>().await.map_err(|e| {
        Error::Account(AccountError::UnexpectedResponse {
            message: e.to_string(),
        })
    })
}

/// Poll the headless flow. `Ok(None)` means "still waiting", which is the
/// normal answer for most of a device flow's life and must not be an error.
pub async fn poll_device_flow(device_code: &str) -> Result<Option<TokenResponse>> {
    match post_token(serde_json::json!({
        "grant_type": "device_code",
        "device_code": device_code,
    }))
    .await
    {
        Ok(tokens) => Ok(Some(tokens)),
        Err(Error::Account(AccountError::Refused { message }))
            if message.contains("Waiting") || message.contains("Polling") =>
        {
            Ok(None)
        }
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
