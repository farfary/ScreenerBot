// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

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

use std::future::Future;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, RwLock, RwLockWriteGuard};
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;
use tokio::sync::Mutex;

use crate::errors::{AccountError, Error, Result};
use crate::logger::{self, LogTag};

use client::{DevicePoll, TokenResponse};
use pkce::PendingAuth;
use store::StoredSession;

/// The live session. `None` means signed out, which is a perfectly good state.
static SESSION: LazyLock<RwLock<Option<Session>>> = LazyLock::new(|| RwLock::new(None));

/// Serializes refresh-token rotation and remembers a recent transient failure.
///
/// A refresh token is single-use. On startup every authenticated data consumer
/// can need an access token at once, so the session must be checked again after
/// acquiring this lock: the first waiter rotates the token and every later
/// waiter reuses the access token it installed.
static TOKEN_REFRESH: LazyLock<Mutex<RefreshGate>> =
    LazyLock::new(|| Mutex::new(RefreshGate::default()));

/// How long a transient refresh failure answers every waiter without a new
/// attempt. Without it, each caller queued on the lock spends its own refresh
/// timeout against a server that has just failed to answer.
const REFRESH_BACKOFF: Duration = Duration::from_secs(30);

/// True after the server itself ended the session (a refresh answered
/// `invalid_grant`), until the next sign-in or an explicit sign-out. Lets the
/// panel say why the account is gone instead of silently showing sign-in.
static SESSION_ENDED_BY_SERVER: AtomicBool = AtomicBool::new(false);

/// Serializes loopback callbacks and remembers the last completed one.
///
/// A browser can deliver the same redirect twice (a doubled navigation, a
/// reload of the result page). The second request waits here for the first and
/// then finds its state already completed, so it shows the success page
/// instead of a refusal for a sign-in that worked.
static CALLBACKS: LazyLock<Mutex<CallbackLedger>> =
    LazyLock::new(|| Mutex::new(CallbackLedger::default()));

/// How long a completed callback state is recognised when it arrives again.
const CALLBACK_REPLAY_WINDOW: Duration = Duration::from_secs(60);

/// The in-flight browser authorization, if one is running.
///
/// In memory and nowhere else: it is worthless after the callback, and a
/// PENDING verifier written to disk would be a verifier somebody else can read.
static PENDING: LazyLock<RwLock<Option<PendingAuth>>> = LazyLock::new(|| RwLock::new(None));

#[derive(Debug, Clone)]
struct Session {
    access_token: String,
    /// When the access token stops being accepted, in wall-clock time.
    ///
    /// The data service checks the token's wall-clock `exp`. A monotonic clock
    /// stops while the machine sleeps, so an expiry kept on one would still
    /// look valid after a long sleep and every request would be refused.
    expires_at: SystemTime,
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
    /// Where this install stands with the ScreenerBot data service.
    ///
    /// Carried on the account status rather than on a route of its own because
    /// it is the SAME question the panel is already asking — the setup screen and
    /// Settings both need "are you signed in, and what does that get you" in one
    /// answer, and two fetches would let the two halves disagree on screen.
    pub data_access: crate::data_server::DataAccessStatus,
    /// True while signed out because screenerbot.io ended the session (revoked,
    /// expired or reused), as opposed to a sign-out chosen on this machine.
    pub session_ended: bool,
}

impl Session {
    /// A session from a token pair the server just issued.
    fn issued(
        tokens: TokenResponse,
        name: Option<String>,
        email: Option<String>,
        now: SystemTime,
    ) -> Self {
        let scopes = tokens
            .scope
            .split_whitespace()
            .map(|scope| scope.to_string())
            .collect();

        Self {
            access_token: tokens.access_token,
            expires_at: now + Duration::from_secs(tokens.expires_in),
            refresh_token: tokens.refresh_token,
            device_id: tokens.device_id,
            scopes,
            name,
            email,
        }
    }

    /// A session restored from disk. Only the refresh token survives a
    /// restart, so the first call that needs an access token mints it; an
    /// expiry at the epoch is what makes that happen without a separate flag.
    fn restored(stored: StoredSession) -> Self {
        Self {
            access_token: String::new(),
            expires_at: SystemTime::UNIX_EPOCH,
            refresh_token: stored.refresh_token,
            device_id: stored.device_id,
            scopes: stored.scopes,
            name: stored.account_label,
            email: stored.account_email,
        }
    }

    /// What is written to `account.dat`. The access token is absent on purpose.
    fn stored(&self) -> StoredSession {
        StoredSession {
            refresh_token: self.refresh_token.clone(),
            device_id: self.device_id.clone(),
            scopes: self.scopes.clone(),
            account_label: self.name.clone(),
            account_email: self.email.clone(),
        }
    }
}

/// The refresh lock's state: when the last refresh failed transiently.
#[derive(Debug, Default)]
struct RefreshGate {
    failed_at: Option<tokio::time::Instant>,
}

impl RefreshGate {
    fn backing_off(&self, now: tokio::time::Instant) -> bool {
        self.failed_at
            .is_some_and(|failed_at| now.saturating_duration_since(failed_at) < REFRESH_BACKOFF)
    }
}

/// What a failed refresh means for the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RefreshFailure {
    /// The server answered 400 `invalid_grant`: the token is unknown, expired,
    /// revoked or reused, and no retry can bring the session back.
    SessionOver,
    /// Rate limited, a server error, a network error or timeout, or a body that
    /// could not be read. The session and its refresh token stay.
    Transient,
}

/// Classify a refresh error. Only the server's explicit verdict ends a session:
/// ending it on a 429 or a 502 would sign out every install behind a shared
/// address, or every install refreshing during a deploy.
fn refresh_failure(error: &Error) -> RefreshFailure {
    match error {
        Error::Account(AccountError::Refused {
            code: Some(code), ..
        }) if code == "invalid_grant" => RefreshFailure::SessionOver,
        _ => RefreshFailure::Transient,
    }
}

/// The outcome of one device-code poll, as the account panel reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceSignIn {
    SignedIn,
    Pending,
    SlowDown,
    Expired,
}

/// The last loopback callback that completed a sign-in.
#[derive(Debug, Default)]
struct CallbackLedger {
    completed: Option<(String, Instant)>,
}

impl CallbackLedger {
    fn completed(&self, state: &str, now: Instant) -> bool {
        self.completed.as_ref().is_some_and(|(done, at)| {
            now.saturating_duration_since(*at) <= CALLBACK_REPLAY_WINDOW
                && pkce::constant_time_eq(done, state)
        })
    }
}

fn write_slot<T>(slot: &RwLock<T>) -> RwLockWriteGuard<'_, T> {
    match slot.write() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
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

    if restore_into(&SESSION, stored) {
        logger::info(LogTag::System, "ScreenerBot account session restored");
    }
}

/// Install a stored session unless one is already live.
///
/// The store is written FROM memory, so a live session is never older than the
/// file. Overwriting it would put back a refresh token that may already be
/// spent, and presenting a spent token revokes the device.
fn restore_into(slot: &RwLock<Option<Session>>, stored: StoredSession) -> bool {
    let mut guard = write_slot(slot);
    if guard.is_some() {
        return false;
    }
    *guard = Some(Session::restored(stored));
    true
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
        data_access: data_access_status(),
        session_ended: session.is_none() && SESSION_ENDED_BY_SERVER.load(Ordering::Relaxed),
    }
}

/// The data-service state, corrected for what this module already knows.
///
/// The recorded state is whatever the last CALL established, and there may not
/// have been one — on a cold launch, or because every caller checked
/// `is_usable` first and correctly declined to ask. Without a session the answer
/// is knowable without asking anybody, and saying so beats both "not checked
/// yet" and a stale "active" left over from before the user signed out.
fn data_access_status() -> crate::data_server::DataAccessStatus {
    let recorded = crate::data_server::status();
    if is_signed_in() {
        return recorded;
    }

    crate::data_server::access::describe(
        crate::data_server::DataAccess::SignedOut,
        recorded.checked_at,
    )
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
    *write_slot(&SESSION) = session;
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
    let margin = crate::config::with_config(|config| config.account.refresh_margin());
    access_token_with(&TOKEN_REFRESH, margin, read_session, refresh_session).await
}

fn usable_access_token(session: &Session, margin: Duration, now: SystemTime) -> Option<String> {
    (!session.access_token.is_empty() && session.expires_at > now + margin)
        .then(|| session.access_token.clone())
}

async fn access_token_with<Read, Refresh, RefreshFuture>(
    refresh_lock: &Mutex<RefreshGate>,
    margin: Duration,
    read: Read,
    refresh: Refresh,
) -> Option<String>
where
    Read: Fn() -> Option<Session>,
    Refresh: FnOnce(Session) -> RefreshFuture,
    RefreshFuture: Future<Output = Result<String>>,
{
    let session = read()?;
    if let Some(token) = usable_access_token(&session, margin, SystemTime::now()) {
        return Some(token);
    }

    let mut gate = refresh_lock.lock().await;

    // Another caller may have completed the one permitted rotation while this
    // caller waited. Re-reading is what turns the lock into single-flight
    // behavior instead of merely issuing the same rotations sequentially.
    let session = read()?;
    if let Some(token) = usable_access_token(&session, margin, SystemTime::now()) {
        return Some(token);
    }

    // The previous attempt failed moments ago. Waiters answer from that result
    // instead of each spending a refresh timeout on a server that is not
    // answering.
    if gate.backing_off(tokio::time::Instant::now()) {
        return None;
    }

    match refresh(session).await {
        Ok(token) => {
            gate.failed_at = None;
            Some(token)
        }
        Err(Error::Account(AccountError::SessionEnded)) => None,
        Err(error) => {
            gate.failed_at = Some(tokio::time::Instant::now());
            logger::warning(
                LogTag::System,
                &format!(
                    "Account: token refresh failed, session kept, next attempt in {}s: {error}",
                    REFRESH_BACKOFF.as_secs()
                ),
            );
            None
        }
    }
}

/// Drop a cached access token the data service just rejected, so the next
/// [`access_token`] call refreshes it through the single-flight lock.
///
/// Compared against the token the caller presented: one that another caller
/// has already replaced is left alone, so concurrent rejections of the same
/// stale token cause one rotation, not one per caller.
pub fn invalidate_access_token(stale: &str) {
    invalidate_in(&SESSION, stale);
}

fn invalidate_in(slot: &RwLock<Option<Session>>, stale: &str) {
    if let Some(session) = write_slot(slot).as_mut() {
        if session.access_token == stale {
            session.access_token.clear();
            session.expires_at = SystemTime::UNIX_EPOCH;
        }
    }
}

/// Spend the refresh token and store the pair that comes back.
///
/// The new refresh token MUST replace the old one: the server marks each spent
/// on use, and presenting a spent token is treated as theft and revokes the
/// device. That is why the new pair goes to memory before anything else can
/// fail (see [`adopt_into`]).
async fn refresh_session(session: Session) -> Result<String> {
    if crate::connectivity::is_network_offline() {
        return Err(Error::Account(AccountError::Generic {
            message: "offline".to_string(),
        }));
    }

    match client::refresh(&session.refresh_token).await {
        Ok(tokens) => {
            let access = tokens.access_token.clone();
            let issued = Session::issued(
                tokens,
                session.name.clone(),
                session.email.clone(),
                SystemTime::now(),
            );
            if !adopt_into(&SESSION, issued, Some(&session.refresh_token), store::save) {
                // Signed out, or signed in again, while the rotation was in
                // flight. The newer local decision wins.
                return Err(Error::Account(AccountError::NotSignedIn));
            }
            session_adopted();
            Ok(access)
        }
        Err(error) => match refresh_failure(&error) {
            RefreshFailure::SessionOver => {
                // The server will not renew this session: revoked from the
                // dashboard, expired, or reuse-detected. Clearing it locally is
                // the honest response — leaving a dead token in place makes the
                // UI claim a sign-in that does not work.
                logger::warning(
                    LogTag::System,
                    &format!(
                        "ScreenerBot account session ended by screenerbot.io, signed out on this machine: {error}"
                    ),
                );
                sign_out_locally();
                SESSION_ENDED_BY_SERVER.store(true, Ordering::Relaxed);
                Err(Error::Account(AccountError::SessionEnded))
            }
            RefreshFailure::Transient => Err(error),
        },
    }
}

/// Take a fresh token pair from a sign-in, make it live, and persist it.
fn adopt_tokens(tokens: TokenResponse, name: Option<String>, email: Option<String>) {
    let issued = Session::issued(tokens, name, email, SystemTime::now());
    adopt_into(&SESSION, issued, None, store::save);
    session_adopted();
}

/// Make `session` live, then persist it.
///
/// Memory first: the server has already spent the previous refresh token, so
/// the new one must be live before anything can fail. A failed save keeps the
/// new session in memory (the next rotation saves again) and never falls back
/// to the spent token. `replaces` is the refresh token a rotation spent; when
/// the live session no longer holds it, the rotation is discarded.
fn adopt_into<Save>(
    slot: &RwLock<Option<Session>>,
    session: Session,
    replaces: Option<&str>,
    save: Save,
) -> bool
where
    Save: FnOnce(&StoredSession) -> Result<()>,
{
    let stored = session.stored();
    let mut guard = write_slot(slot);
    if let Some(spent) = replaces {
        if guard.as_ref().map(|live| live.refresh_token.as_str()) != Some(spent) {
            return false;
        }
    }
    *guard = Some(session);

    // Saved under the same guard, so two adoptions cannot reach the file in
    // the opposite order from the one they reached memory.
    if let Err(error) = save(&stored) {
        logger::warning(
            LogTag::System,
            &format!(
                "Account: could not save the new session; it stays live in memory and is saved again on the next rotation: {error}"
            ),
        );
    }
    true
}

/// Side effects of any new grant.
fn session_adopted() {
    // A "sign in again" refusal recorded under the previous grant is exactly
    // what this sign-in answers, so it must not outlive it.
    crate::data_server::access::forget_refusals();
    crate::chains::access::session_changed();
    SESSION_ENDED_BY_SERVER.store(false, Ordering::Relaxed);
}

/// Ask the server who this is, so the UI shows a name rather than "signed in".
pub async fn refresh_profile() {
    let Some(token) = access_token().await else {
        return;
    };

    if let Ok(profile) = client::fetch_profile(&token).await {
        let mut guard = write_slot(&SESSION);

        if let Some(session) = guard.as_mut() {
            session.name = profile.name;
            session.email = profile.email;

            if let Err(error) = store::save(&session.stored()) {
                log::debug!("Account: could not save the refreshed profile: {error}");
            }
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

    // Starting a second sign-in abandons the first. Keeping both would mean two
    // live states, and a callback that could satisfy either.
    *write_slot(&PENDING) = Some(pending);

    Ok(url)
}

/// The browser came back to our loopback route.
///
/// `state` is checked in constant time against the value THIS process
/// generated, and the pending authorization is consumed whether or not the
/// exchange succeeds — a state value is good for exactly one attempt. A repeat
/// of a callback that already completed, within [`CALLBACK_REPLAY_WINDOW`],
/// succeeds without a second exchange.
pub async fn complete_browser_signin(code: &str, state: &str) -> Result<()> {
    complete_callback(&CALLBACKS, &PENDING, state, |pending| async move {
        let tokens = client::exchange_code(code, &pending.verifier, &pending.redirect_uri).await?;
        adopt_tokens(tokens, None, None);
        refresh_profile().await;
        logger::info(LogTag::System, "Signed in to ScreenerBot account");
        Ok(())
    })
    .await
}

async fn complete_callback<Exchange, ExchangeFuture>(
    ledger: &Mutex<CallbackLedger>,
    pending_slot: &RwLock<Option<PendingAuth>>,
    state: &str,
    exchange: Exchange,
) -> Result<()>
where
    Exchange: FnOnce(PendingAuth) -> ExchangeFuture,
    ExchangeFuture: Future<Output = Result<()>>,
{
    let mut ledger = ledger.lock().await;
    if ledger.completed(state, Instant::now()) {
        logger::info(
            LogTag::Webserver,
            "Account: repeated OAuth callback for a sign-in that already completed",
        );
        return Ok(());
    }

    let pending = write_slot(pending_slot).take();

    let Some(pending) = pending else {
        return Err(Error::Account(AccountError::Refused {
            code: None,
            message: "No sign-in was in progress. Start again from the app.".to_string(),
        }));
    };

    if pending.is_expired() {
        return Err(Error::Account(AccountError::Refused {
            code: None,
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
            code: None,
            message: "That sign-in could not be verified. Start again from the app.".to_string(),
        }));
    }

    exchange(pending).await?;
    ledger.completed = Some((state.to_string(), Instant::now()));
    Ok(())
}

/// Email and password, typed into the app.
pub async fn sign_in_with_password(email: &str, password: &str) -> Result<()> {
    let tokens = client::sign_in_with_password(email, password).await?;
    adopt_tokens(tokens, None, Some(email.to_string()));
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
            code: None,
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
    adopt_tokens(tokens, None, None);
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

/// Poll the headless flow once. The account panel drives the interval: it adds
/// [`client::DEVICE_SLOW_DOWN_STEP`] on `SlowDown` and stops at the code's own
/// expiry or on `Expired`.
pub async fn poll_device_signin(device_code: &str) -> Result<DeviceSignIn> {
    match client::poll_device_flow(device_code).await? {
        DevicePoll::Approved(tokens) => {
            adopt_tokens(tokens, None, None);
            refresh_profile().await;
            logger::info(LogTag::System, "Signed in to ScreenerBot account");
            Ok(DeviceSignIn::SignedIn)
        }
        DevicePoll::Pending => Ok(DeviceSignIn::Pending),
        DevicePoll::SlowDown => Ok(DeviceSignIn::SlowDown),
        DevicePoll::Expired => Ok(DeviceSignIn::Expired),
    }
}

// ---------------------------------------------------------------------------
// Signing out
// ---------------------------------------------------------------------------

/// Forget the session on this machine, and ask the server to revoke it.
///
/// The local sign-out is unconditional and happens first. Revocation is
/// best-effort in the background with a short timeout: a server that cannot be
/// reached must never keep a session alive here, and a device it never hears
/// about can still be revoked from the website's device list.
pub fn sign_out() -> Result<()> {
    let refresh_token = read_session().map(|session| session.refresh_token);
    sign_out_locally();
    SESSION_ENDED_BY_SERVER.store(false, Ordering::Relaxed);
    logger::info(LogTag::System, "Signed out of ScreenerBot account");

    if let Some(refresh_token) = refresh_token {
        revoke_in_background(refresh_token);
    }
    Ok(())
}

fn revoke_in_background(refresh_token: String) {
    if crate::connectivity::is_network_offline() {
        return;
    }
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return;
    };

    runtime.spawn(async move {
        match client::revoke(&refresh_token).await {
            Ok(()) => logger::info(
                LogTag::System,
                "ScreenerBot account device revoked on screenerbot.io",
            ),
            Err(error) => logger::info(
                LogTag::System,
                &format!(
                    "Account: could not revoke the device on screenerbot.io; it remains listed there until revoked from the website: {error}"
                ),
            ),
        }
    });
}

fn sign_out_locally() {
    write_session(None);
    crate::data_server::access::forget_refusals();
    crate::chains::access::session_changed();
    if let Err(error) = store::clear() {
        log::debug!("Account: could not clear stored session: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;
    use std::sync::{Arc, RwLock as StdRwLock};

    fn session(access_token: &str, expires_at: SystemTime, refresh_token: &str) -> Session {
        Session {
            access_token: access_token.to_string(),
            expires_at,
            refresh_token: refresh_token.to_string(),
            device_id: "device".to_string(),
            scopes: vec!["data:read".to_string()],
            name: None,
            email: None,
        }
    }

    fn token_pair(access_token: &str, refresh_token: &str) -> TokenResponse {
        TokenResponse {
            access_token: access_token.to_string(),
            expires_in: 900,
            refresh_token: refresh_token.to_string(),
            scope: "data:read rpc:submit".to_string(),
            device_id: "device".to_string(),
        }
    }

    fn stored(refresh_token: &str) -> StoredSession {
        StoredSession {
            refresh_token: refresh_token.to_string(),
            device_id: "device".to_string(),
            scopes: vec!["data:read".to_string()],
            account_label: None,
            account_email: None,
        }
    }

    fn live_refresh_token(slot: &StdRwLock<Option<Session>>) -> Option<String> {
        slot.read()
            .unwrap()
            .as_ref()
            .map(|live| live.refresh_token.clone())
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_access_token_requests_rotate_refresh_token_once() {
        const CALLERS: usize = 32;

        let refresh_lock = Arc::new(Mutex::new(RefreshGate::default()));
        let refresh_calls = Arc::new(AtomicUsize::new(0));
        let session = Arc::new(StdRwLock::new(Some(session(
            "",
            SystemTime::UNIX_EPOCH,
            "single-use-refresh-token",
        ))));

        let mut tasks = Vec::with_capacity(CALLERS);
        for _ in 0..CALLERS {
            let refresh_lock = Arc::clone(&refresh_lock);
            let refresh_calls = Arc::clone(&refresh_calls);
            let read_state = Arc::clone(&session);
            let write_state = Arc::clone(&session);

            tasks.push(tokio::spawn(async move {
                access_token_with(
                    refresh_lock.as_ref(),
                    Duration::ZERO,
                    move || read_state.read().unwrap().clone(),
                    move |mut current| async move {
                        assert_eq!(current.refresh_token, "single-use-refresh-token");
                        refresh_calls.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(25)).await;

                        current.access_token = "shared-access-token".to_string();
                        current.expires_at = SystemTime::now() + Duration::from_secs(900);
                        current.refresh_token = "rotated-refresh-token".to_string();
                        *write_state.write().unwrap() = Some(current);

                        Ok("shared-access-token".to_string())
                    },
                )
                .await
            }));
        }

        for task in tasks {
            assert_eq!(task.await.unwrap().as_deref(), Some("shared-access-token"));
        }
        assert_eq!(refresh_calls.load(Ordering::SeqCst), 1);
        assert_eq!(
            session.read().unwrap().as_ref().unwrap().refresh_token,
            "rotated-refresh-token"
        );
    }

    #[test]
    fn only_an_invalid_grant_ends_the_session() {
        let invalid_grant = Error::Account(AccountError::Refused {
            code: Some("invalid_grant".to_string()),
            message: "Your session has ended.".to_string(),
        });
        assert_eq!(refresh_failure(&invalid_grant), RefreshFailure::SessionOver);

        let transient = [
            Error::Account(AccountError::Unavailable {
                status: 429,
                detail: "Too many requests.".to_string(),
            }),
            Error::Account(AccountError::Unavailable {
                status: 503,
                detail: "Service Unavailable".to_string(),
            }),
            Error::Account(AccountError::Unavailable {
                status: 502,
                detail: "the response carried no OAuth error".to_string(),
            }),
            Error::Network(crate::errors::NetworkError::RequestFailed {
                endpoint: "screenerbot.io".to_string(),
                detail: "operation timed out".to_string(),
            }),
            Error::Account(AccountError::UnexpectedResponse {
                message: "expected value at line 1".to_string(),
            }),
            Error::Account(AccountError::Refused {
                code: Some("invalid_request".to_string()),
                message: "Malformed request.".to_string(),
            }),
            Error::Account(AccountError::Refused {
                code: None,
                message: "invalid_grant".to_string(),
            }),
            Error::Account(AccountError::Generic {
                message: "offline".to_string(),
            }),
        ];
        for error in transient {
            assert_eq!(
                refresh_failure(&error),
                RefreshFailure::Transient,
                "{error:?}"
            );
        }
    }

    #[test]
    fn a_rotation_stays_live_in_memory_when_saving_fails() {
        let slot = StdRwLock::new(Some(session("old-access", SystemTime::UNIX_EPOCH, "spent")));
        let issued = Session::issued(
            token_pair("new-access", "rotated"),
            None,
            None,
            SystemTime::now(),
        );

        let adopted = adopt_into(&slot, issued, Some("spent"), |_| {
            Err(Error::Account(AccountError::Storage {
                message: "disk full".to_string(),
            }))
        });

        assert!(adopted);
        assert_eq!(live_refresh_token(&slot).as_deref(), Some("rotated"));
        assert_eq!(
            slot.read().unwrap().as_ref().unwrap().access_token,
            "new-access"
        );
    }

    #[test]
    fn a_rotation_saves_the_new_refresh_token_never_the_spent_one() {
        let slot = StdRwLock::new(Some(session("", SystemTime::UNIX_EPOCH, "spent")));
        let issued = Session::issued(
            token_pair("new-access", "rotated"),
            None,
            None,
            SystemTime::now(),
        );
        let saved = std::cell::RefCell::new(None);

        adopt_into(&slot, issued, Some("spent"), |stored| {
            saved.replace(Some(stored.refresh_token.clone()));
            Ok(())
        });

        assert_eq!(saved.into_inner().as_deref(), Some("rotated"));
        assert_eq!(live_refresh_token(&slot).as_deref(), Some("rotated"));
    }

    #[test]
    fn a_rotation_never_resurrects_a_session_signed_out_meanwhile() {
        let issued = || {
            Session::issued(
                token_pair("new-access", "rotated"),
                None,
                None,
                SystemTime::now(),
            )
        };

        let signed_out = StdRwLock::new(None);
        assert!(!adopt_into(
            &signed_out,
            issued(),
            Some("spent"),
            |_| Ok(())
        ));
        assert!(signed_out.read().unwrap().is_none());

        let signed_in_again =
            StdRwLock::new(Some(session("a", SystemTime::UNIX_EPOCH, "fresh-sign-in")));
        assert!(!adopt_into(
            &signed_in_again,
            issued(),
            Some("spent"),
            |_| { panic!("a discarded rotation must not be saved") }
        ));
        assert_eq!(
            live_refresh_token(&signed_in_again).as_deref(),
            Some("fresh-sign-in")
        );
    }

    #[test]
    fn restoring_from_disk_never_replaces_a_live_session() {
        let slot = StdRwLock::new(Some(session(
            "live",
            SystemTime::now() + Duration::from_secs(900),
            "rotated",
        )));
        assert!(!restore_into(&slot, stored("spent-on-disk")));
        assert_eq!(live_refresh_token(&slot).as_deref(), Some("rotated"));

        let empty = StdRwLock::new(None);
        assert!(restore_into(&empty, stored("on-disk")));
        assert_eq!(live_refresh_token(&empty).as_deref(), Some("on-disk"));
        let restored = empty.read().unwrap().clone().unwrap();
        assert!(usable_access_token(&restored, Duration::ZERO, SystemTime::now()).is_none());
    }

    #[test]
    fn access_token_expiry_is_measured_in_wall_clock_time() {
        let issued_at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_800_000_000);
        let live = Session::issued(token_pair("access", "refresh"), None, None, issued_at);
        let margin = Duration::from_secs(120);

        assert_eq!(
            usable_access_token(&live, margin, issued_at).as_deref(),
            Some("access")
        );
        // Wall-clock time keeps running while the machine sleeps; a token whose
        // `exp` passed during the sleep is not presented after waking.
        assert!(
            usable_access_token(&live, margin, issued_at + Duration::from_secs(13 * 60 + 1))
                .is_none()
        );
        assert!(
            usable_access_token(&live, margin, issued_at + Duration::from_secs(4 * 3600)).is_none()
        );
    }

    #[test]
    fn invalidating_drops_only_the_token_that_was_rejected() {
        let expires = SystemTime::now() + Duration::from_secs(900);
        let slot = StdRwLock::new(Some(session("replaced-already", expires, "refresh")));
        invalidate_in(&slot, "stale");
        let kept = slot.read().unwrap().clone().unwrap();
        assert_eq!(
            usable_access_token(&kept, Duration::ZERO, SystemTime::now()).as_deref(),
            Some("replaced-already")
        );

        invalidate_in(&slot, "replaced-already");
        let dropped = slot.read().unwrap().clone().unwrap();
        assert!(usable_access_token(&dropped, Duration::ZERO, SystemTime::now()).is_none());
        assert_eq!(dropped.refresh_token, "refresh");
    }

    #[tokio::test(start_paused = true)]
    async fn a_transient_refresh_failure_answers_waiters_without_another_attempt() {
        const CALLERS: usize = 8;

        let refresh_lock = Arc::new(Mutex::new(RefreshGate::default()));
        let attempts = Arc::new(AtomicUsize::new(0));
        let slot = Arc::new(StdRwLock::new(Some(session(
            "",
            SystemTime::UNIX_EPOCH,
            "refresh",
        ))));

        let failing_call = |lock: Arc<Mutex<RefreshGate>>,
                            attempts: Arc<AtomicUsize>,
                            slot: Arc<StdRwLock<Option<Session>>>| async move {
            access_token_with(
                lock.as_ref(),
                Duration::ZERO,
                move || slot.read().unwrap().clone(),
                move |_| async move {
                    attempts.fetch_add(1, Ordering::SeqCst);
                    Err(Error::Account(AccountError::Unavailable {
                        status: 503,
                        detail: "Service Unavailable".to_string(),
                    }))
                },
            )
            .await
        };

        let mut tasks = Vec::with_capacity(CALLERS);
        for _ in 0..CALLERS {
            tasks.push(tokio::spawn(failing_call(
                Arc::clone(&refresh_lock),
                Arc::clone(&attempts),
                Arc::clone(&slot),
            )));
        }
        for task in tasks {
            assert!(task.await.unwrap().is_none());
        }
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
        assert_eq!(live_refresh_token(&slot).as_deref(), Some("refresh"));

        tokio::time::advance(REFRESH_BACKOFF).await;
        assert!(failing_call(
            Arc::clone(&refresh_lock),
            Arc::clone(&attempts),
            Arc::clone(&slot)
        )
        .await
        .is_none());
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }

    fn pending_slot() -> (StdRwLock<Option<PendingAuth>>, String) {
        let pending = PendingAuth::new("http://127.0.0.1:50000/oauth/callback".to_string());
        let state = pending.state.clone();
        (StdRwLock::new(Some(pending)), state)
    }

    #[tokio::test]
    async fn a_repeated_callback_for_a_completed_sign_in_succeeds_without_a_second_exchange() {
        let ledger = Mutex::new(CallbackLedger::default());
        let (pending, state) = pending_slot();
        let exchanges = AtomicUsize::new(0);
        let exchanges = &exchanges;
        let exchange = |_| async move {
            exchanges.fetch_add(1, Ordering::SeqCst);
            Ok(())
        };

        complete_callback(&ledger, &pending, &state, exchange)
            .await
            .unwrap();
        complete_callback(&ledger, &pending, &state, exchange)
            .await
            .unwrap();
        assert_eq!(exchanges.load(Ordering::SeqCst), 1);

        // A state this process never completed is still refused.
        assert!(complete_callback(&ledger, &pending, "forged", exchange)
            .await
            .is_err());
        assert_eq!(exchanges.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn concurrent_duplicate_callbacks_both_succeed_with_one_exchange() {
        let ledger = Mutex::new(CallbackLedger::default());
        let (pending, state) = pending_slot();
        let exchanges = AtomicUsize::new(0);
        let exchanges = &exchanges;
        let exchange = |_| async move {
            exchanges.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_millis(20)).await;
            Ok(())
        };

        let (first, second) = tokio::join!(
            complete_callback(&ledger, &pending, &state, exchange),
            complete_callback(&ledger, &pending, &state, exchange)
        );
        assert!(first.is_ok() && second.is_ok());
        assert_eq!(exchanges.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_completed_callback_is_recognised_only_inside_the_replay_window() {
        let at = Instant::now();
        let ledger = CallbackLedger {
            completed: Some(("state".to_string(), at)),
        };
        assert!(ledger.completed("state", at + CALLBACK_REPLAY_WINDOW));
        assert!(!ledger.completed(
            "state",
            at + CALLBACK_REPLAY_WINDOW + Duration::from_secs(1)
        ));
        assert!(!ledger.completed("other", at));
    }

    #[tokio::test]
    async fn a_failed_exchange_is_not_remembered_as_completed() {
        let ledger = Mutex::new(CallbackLedger::default());
        let (pending, state) = pending_slot();
        let result = complete_callback(&ledger, &pending, &state, |_| async {
            Err(Error::Account(AccountError::Unavailable {
                status: 503,
                detail: "Service Unavailable".to_string(),
            }))
        })
        .await;
        assert!(result.is_err());
        assert!(ledger.lock().await.completed.is_none());
    }
}
