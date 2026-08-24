//! The optional ScreenerBot account, as seen from inside the app.
//!
//! ============================================================================
//! WHAT SIGNING IN DOES AND DOES NOT DO
//! ============================================================================
//! It ADDS three things: free broadcasting of transactions you have already
//! signed, token voting that counts once per person, and your referral totals
//! in the app. It GATES nothing. Every feature that worked before an account
//! existed works identically with no account, and that is a rule rather than a
//! current state of affairs — see `is_signed_in()` callers, none of which may
//! guard a trading path.
//!
//! ============================================================================
//! WHAT LEAVES THIS MACHINE
//! ============================================================================
//! Only what a sign-in requires: on the browser path, nothing at all (the
//! browser talks to the server, not us); on the password path, the email and
//! password for the duration of one request; on the wallet path, a public
//! address and a signature over a message the user can read. Never a private
//! key, never a position, never a balance, never a trade.
//!
//! Three ways in, one result. See `client.rs` for the wire calls, `pkce.rs` for
//! why a public binary can do this safely, and `store.rs` for where the refresh
//! token rests.

pub mod client;
pub mod pkce;
pub mod store;

use std::sync::{LazyLock, RwLock};
use std::time::{Duration, Instant};

use serde::Serialize;

use crate::errors::{AccountError, Error, Result};
use crate::logger::{self, LogTag};

use client::TokenResponse;
use pkce::PendingAuth;
use store::StoredSession;

/// The live session. `None` means signed out, which is a perfectly good state.
static SESSION: LazyLock<RwLock<Option<Session>>> = LazyLock::new(|| RwLock::new(None));

/// The in-flight browser authorization, if one is running.
///
/// In memory and nowhere else: it is worthless after the callback, and a
/// PENDING verifier written to disk would be a verifier somebody else can read.
static PENDING: LazyLock<RwLock<Option<PendingAuth>>> = LazyLock::new(|| RwLock::new(None));

#[derive(Debug, Clone)]
struct Session {
    access_token: String,
    /// When the access token stops being accepted.
    expires_at: Instant,
    refresh_token: String,
    device_id: String,
    scopes: Vec<String>,
    name: Option<String>,
    email: Option<String>,
}

/// What the dashboard is told. Never a token, of either kind.
#[derive(Debug, Clone, Serialize)]
pub struct AccountStatus {
    pub signed_in: bool,
    pub name: Option<String>,
    pub email: Option<String>,
    pub scopes: Vec<String>,
    pub device_id: Option<String>,
    /// True when the main wallet is known to screenerbot.io, so the UI can
    /// offer "sign in as this wallet" rather than a generic prompt.
    pub wallet_has_account: bool,
    /// Whether this build can reach the account service at all.
    pub online: bool,
    /// The persisted preference shown by the setup screen's gateway checkbox.
    pub use_gateway_rpc: bool,
}

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

/// Load a stored session, if there is one.
///
/// Deliberately does NOT touch the network: this runs during boot, and an
/// account subsystem that can delay startup is an account subsystem that can
/// stop somebody trading. The token is refreshed lazily on first use and by the
/// account service in the background.
pub fn initialize() {
    let Some(stored) = store::load() else {
        return;
    };

    let mut guard = match SESSION.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };

    *guard = Some(Session {
        // No access token yet — only the refresh token survives a restart, so
        // the first call that needs one will mint it. `expires_at` in the past
        // is what makes that happen without a separate "needs refresh" flag.
        access_token: String::new(),
        expires_at: Instant::now() - Duration::from_secs(1),
        refresh_token: stored.refresh_token,
        device_id: stored.device_id,
        scopes: stored.scopes,
        name: stored.account_label,
        email: stored.account_email,
    });

    logger::info(LogTag::System, "ScreenerBot account session restored");
}

pub fn is_signed_in() -> bool {
    read_session().is_some()
}

pub fn status() -> AccountStatus {
    let session = read_session();

    AccountStatus {
        signed_in: session.is_some(),
        name: session.as_ref().and_then(|s| s.name.clone()),
        email: session.as_ref().and_then(|s| s.email.clone()),
        scopes: session
            .as_ref()
            .map(|s| s.scopes.clone())
            .unwrap_or_default(),
        device_id: session.as_ref().map(|s| s.device_id.clone()),
        wallet_has_account: false,
        online: !crate::connectivity::is_network_offline(),
        use_gateway_rpc: crate::config::with_config(|config| config.account.use_gateway_rpc),
    }
}

/// Does the signed-in device hold this permission?
///
/// Scopes are frozen at grant time on the server, so a device authorised before
/// a capability existed does not silently acquire it. Callers must check rather
/// than assume.
pub fn has_scope(scope: &str) -> bool {
    read_session()
        .map(|session| session.scopes.iter().any(|held| held == scope))
        .unwrap_or(false)
}

fn read_session() -> Option<Session> {
    match SESSION.read() {
        Ok(guard) => guard.clone(),
        Err(poisoned) => poisoned.into_inner().clone(),
    }
}

fn write_session(session: Option<Session>) {
    let mut guard = match SESSION.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = session;
}

// ---------------------------------------------------------------------------
// Tokens
// ---------------------------------------------------------------------------

/// A valid access token, refreshing first if it is close to expiry.
///
/// Returns `None` rather than an error when signed out, because every caller of
/// this is an optional enhancement: the RPC gateway falls back to the user's own
/// endpoint, and the referral panel simply shows nothing.
pub async fn access_token() -> Option<String> {
    let session = read_session()?;

    let margin = crate::config::with_config(|config| config.account.refresh_margin());
    if !session.access_token.is_empty() && session.expires_at > Instant::now() + margin {
        return Some(session.access_token);
    }

    match refresh_now().await {
        Ok(token) => Some(token),
        Err(error) => {
            log::debug!("Account: token refresh failed: {error}");
            None
        }
    }
}

/// Spend the refresh token and store the pair that comes back.
///
/// The new refresh token MUST replace the old one: the server marks each spent
/// on use, and presenting a spent token is treated as theft and revokes the
/// device. Losing the response therefore signs this install out at the next
/// attempt, which is why the write happens before anything else can fail.
pub async fn refresh_now() -> Result<String> {
    let Some(session) = read_session() else {
        return Err(Error::Account(AccountError::NotSignedIn));
    };

    if crate::connectivity::is_network_offline() {
        return Err(Error::Account(AccountError::Generic {
            message: "offline".to_string(),
        }));
    }

    match client::refresh(&session.refresh_token).await {
        Ok(tokens) => {
            let access = tokens.access_token.clone();
            adopt_tokens(tokens, session.name.clone(), session.email.clone())?;
            Ok(access)
        }
        Err(Error::Account(AccountError::Refused { .. })) => {
            // The server will not renew this session: revoked from the
            // dashboard, expired, or reuse-detected. Clearing it locally is the
            // honest response — leaving a dead token in place makes the UI claim
            // a sign-in that does not work.
            sign_out_locally();
            Err(Error::Account(AccountError::SessionEnded))
        }
        Err(error) => Err(error),
    }
}

/// Take a fresh token pair, persist it, and make it live.
fn adopt_tokens(tokens: TokenResponse, name: Option<String>, email: Option<String>) -> Result<()> {
    let scopes: Vec<String> = tokens
        .scope
        .split_whitespace()
        .map(|scope| scope.to_string())
        .collect();

    store::save(&StoredSession {
        refresh_token: tokens.refresh_token.clone(),
        device_id: tokens.device_id.clone(),
        scopes: scopes.clone(),
        account_label: name.clone(),
        account_email: email.clone(),
    })
    .map_err(|message| Error::Account(AccountError::Storage { message }))?;

    write_session(Some(Session {
        access_token: tokens.access_token,
        expires_at: Instant::now() + Duration::from_secs(tokens.expires_in),
        refresh_token: tokens.refresh_token,
        device_id: tokens.device_id,
        scopes,
        name,
        email,
    }));

    Ok(())
}

/// Ask the server who this is, so the UI shows a name rather than "signed in".
pub async fn refresh_profile() {
    let Some(token) = access_token().await else {
        return;
    };

    if let Ok(profile) = client::fetch_profile(&token).await {
        let mut guard = match SESSION.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        if let Some(session) = guard.as_mut() {
            session.name = profile.name.clone();
            session.email = profile.email.clone();

            let _ = store::save(&StoredSession {
                refresh_token: session.refresh_token.clone(),
                device_id: session.device_id.clone(),
                scopes: session.scopes.clone(),
                account_label: profile.name,
                account_email: profile.email,
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Signing in
// ---------------------------------------------------------------------------

/// Begin the browser flow and return the URL to open.
///
/// The redirect points at THIS process's own webserver on loopback. The port is
/// whatever the dashboard is already listening on, so no second listener is
/// opened and no firewall prompt appears.
pub fn begin_browser_signin() -> Result<String> {
    let port = crate::global::get_webserver_port();
    if port == 0 {
        return Err(Error::Account(AccountError::Generic {
            message: "the local server is not ready yet".to_string(),
        }));
    }

    let redirect_uri = format!("http://127.0.0.1:{port}/oauth/callback");
    let pending = PendingAuth::new(redirect_uri.clone());
    let url = client::authorize_url(&pending.challenge, &pending.state, &redirect_uri);

    let mut guard = match PENDING.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    // Starting a second sign-in abandons the first. Keeping both would mean two
    // live states, and a callback that could satisfy either.
    *guard = Some(pending);

    Ok(url)
}

/// Discard an authorization attempt that could not be presented to the user.
/// A later attempt would replace it anyway, but clearing it here means a stale
/// callback can never complete a flow whose browser failed to open.
pub fn cancel_browser_signin() {
    let mut guard = match PENDING.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    };
    *guard = None;
}

/// The browser came back to our loopback route.
///
/// `state` is checked in constant time against the value THIS process
/// generated, and the pending authorization is consumed whether or not the
/// exchange succeeds — a state value is good for exactly one attempt.
pub async fn complete_browser_signin(code: &str, state: &str) -> Result<()> {
    let pending = {
        let mut guard = match PENDING.write() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };
        guard.take()
    };

    let Some(pending) = pending else {
        return Err(Error::Account(AccountError::Refused {
            message: "No sign-in was in progress. Start again from the app.".to_string(),
        }));
    };

    if pending.is_expired() {
        return Err(Error::Account(AccountError::Refused {
            message: "That sign-in took too long. Start again from the app.".to_string(),
        }));
    }

    if !pending.state_matches(state) {
        // Someone — or some page — reached our callback with a value we did not
        // issue. Refused without further explanation, and worth a log line
        // because it is not something that happens by accident.
        logger::warning(
            LogTag::Webserver,
            "Account: rejected an OAuth callback with an unrecognised state",
        );
        return Err(Error::Account(AccountError::Refused {
            message: "That sign-in could not be verified. Start again from the app.".to_string(),
        }));
    }

    let tokens = client::exchange_code(code, &pending.verifier, &pending.redirect_uri).await?;
    adopt_tokens(tokens, None, None)?;
    refresh_profile().await;

    logger::info(LogTag::System, "Signed in to ScreenerBot account");
    Ok(())
}

/// Email and password, typed into the app.
pub async fn sign_in_with_password(email: &str, password: &str) -> Result<()> {
    let tokens = client::sign_in_with_password(email, password).await?;
    adopt_tokens(tokens, None, Some(email.to_string()))?;
    refresh_profile().await;

    logger::info(LogTag::System, "Signed in to ScreenerBot account");
    Ok(())
}

/// Does the main wallet already belong to an account?
///
/// Asked so the UI can offer the right thing. It issues a challenge and throws
/// it away, which is harmless — a challenge is single-use and grants nothing.
pub async fn wallet_has_account() -> bool {
    let Ok(address) = crate::wallets::get_main_address().await else {
        return false;
    };

    match client::wallet_challenge(&address).await {
        Ok(challenge) => challenge.has_account,
        Err(_) => false,
    }
}

/// Sign in with the wallet this bot already trades with.
///
/// `create` is the user's explicit answer to "no account uses this wallet — make
/// one?". It is never inferred: signing a message with the trading key is
/// something the person should choose, and creating an account off the back of
/// it without being asked is exactly the behaviour a hostile fork of this
/// open-source bot would add.
pub async fn sign_in_with_wallet(create: bool) -> Result<()> {
    let address = crate::wallets::get_main_address().await.map_err(|e| {
        Error::Account(AccountError::Generic {
            message: e.to_string(),
        })
    })?;

    let challenge = client::wallet_challenge(&address).await?;

    if !challenge.has_account && !create {
        return Err(Error::Account(AccountError::Refused {
            message: "No ScreenerBot account uses this wallet yet.".to_string(),
        }));
    }

    // A signature over TEXT. It is not a transaction, cannot be replayed as
    // one, costs nothing and moves nothing — the message the user sees says so
    // in as many words, and this is the only place the bot ever signs anything
    // that is not a trade.
    let signature =
        crate::chains::solana::accounts::sign_message_with_main_wallet(&challenge.message).await?;

    let tokens =
        client::sign_in_with_wallet(&address, &signature, &challenge.message, create).await?;
    adopt_tokens(tokens, None, None)?;
    refresh_profile().await;

    logger::info(
        LogTag::System,
        "Signed in to ScreenerBot account with wallet",
    );
    Ok(())
}

/// Begin the headless flow: returns the code and URL to show the operator.
pub async fn begin_device_signin() -> Result<client::DeviceCodeResponse> {
    client::start_device_flow().await
}

/// Poll the headless flow. `Ok(false)` means "still waiting".
pub async fn poll_device_signin(device_code: &str) -> Result<bool> {
    match client::poll_device_flow(device_code).await? {
        Some(tokens) => {
            adopt_tokens(tokens, None, None)?;
            refresh_profile().await;
            logger::info(LogTag::System, "Signed in to ScreenerBot account");
            Ok(true)
        }
        None => Ok(false),
    }
}

// ---------------------------------------------------------------------------
// Signing out
// ---------------------------------------------------------------------------

/// Forget the session on this machine.
///
/// Local only, and deliberately so: the grant on the server is revoked from the
/// dashboard, where the user can see every device at once and revoke the one
/// they no longer have. Signing out here is "not on this machine any more",
/// which is what the button says.
pub fn sign_out() -> Result<()> {
    sign_out_locally();
    logger::info(LogTag::System, "Signed out of ScreenerBot account");
    Ok(())
}

fn sign_out_locally() {
    write_session(None);
    if let Err(error) = store::clear() {
        log::debug!("Account: could not clear stored session: {error}");
    }
}
