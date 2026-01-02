use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU32};

// Startup timestamp to track when the bot started for trading logic
pub static STARTUP_TIME: Lazy<DateTime<Utc>> = Lazy::new(|| Utc::now());

// ================================================================================================
// STARTUP COORDINATION SYSTEM - ENSURES PROPER SERVICE INITIALIZATION ORDER
// ================================================================================================

// ──────────────────────────────────────────────────────────────────────────────────────────────
// INITIALIZATION FLAGS
// ──────────────────────────────────────────────────────────────────────────────────────────────

/// Master initialization gate - all services (except webserver) wait for this flag
/// Set to true only after: credentials validated + RPC tested + config saved
pub static INITIALIZATION_COMPLETE: AtomicBool = AtomicBool::new(false);

/// Optional health monitoring flags (for UI/observability, not for gating service startup)
pub static CREDENTIALS_VALID: AtomicBool = AtomicBool::new(false);
pub static RPC_VALID: AtomicBool = AtomicBool::new(false);

/// Check if initialization is complete and services can start
/// This is the single source of truth for service enablement (except webserver)
pub fn is_initialization_complete() -> bool {
    INITIALIZATION_COMPLETE.load(std::sync::atomic::Ordering::SeqCst)
}

// ──────────────────────────────────────────────────────────────────────────────────────────────
// CORE SERVICES READINESS FLAGS (Post-Initialization)
// ──────────────────────────────────────────────────────────────────────────────────────────────

/// Core services readiness flags - prevents trading until all critical services are ready
pub static CONNECTIVITY_SYSTEM_READY: AtomicBool = AtomicBool::new(false);
pub static TOKENS_SYSTEM_READY: AtomicBool = AtomicBool::new(false);
pub static POSITIONS_SYSTEM_READY: AtomicBool = AtomicBool::new(false);
pub static POOL_SERVICE_READY: AtomicBool = AtomicBool::new(false);
pub static TRANSACTIONS_SYSTEM_READY: AtomicBool = AtomicBool::new(false);

/// Check if all critical services are ready for trading operations
pub fn are_core_services_ready() -> bool {
    CONNECTIVITY_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst)
        && TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst)
        && POSITIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst)
        && POOL_SERVICE_READY.load(std::sync::atomic::Ordering::SeqCst)
        && TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst)
}

/// Get list of services that are not yet ready (for debugging)
pub fn get_pending_services() -> Vec<&'static str> {
    let mut pending = Vec::new();

    if !CONNECTIVITY_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst) {
        pending.push("Connectivity System");
    }
    if !TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst) {
        pending.push("Tokens System");
    }
    if !POSITIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst) {
        pending.push("Positions System");
    }
    if !POOL_SERVICE_READY.load(std::sync::atomic::Ordering::SeqCst) {
        pending.push("Pool Service");
    }
    if !TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst) {
        pending.push("Transactions System");
    }

    pending
}

// ================================================================================================
// GUI MODE SECURITY - PREVENTS EXTERNAL ACCESS TO WEBSERVER WHEN RUNNING IN ELECTRON
// ================================================================================================

use std::sync::atomic::AtomicU16;
use std::sync::RwLock;

/// Whether the application is running in GUI (Electron) mode
/// When true, webserver requires security token validation
pub static IS_GUI_MODE: AtomicBool = AtomicBool::new(false);

/// Dynamic port the webserver is bound to (0 = not started yet)
pub static WEBSERVER_PORT: AtomicU16 = AtomicU16::new(0);

/// Host address the webserver is bound to
static WEBSERVER_HOST: RwLock<String> = RwLock::new(String::new());

/// Security token required for all API requests in GUI mode
/// Generated at startup, must be passed in X-ScreenerBot-Token header
static SECURITY_TOKEN: RwLock<Option<String>> = RwLock::new(None);

/// Set GUI mode flag (called from Electron main process)
pub fn set_gui_mode(enabled: bool) {
    IS_GUI_MODE.store(enabled, std::sync::atomic::Ordering::SeqCst);
}

/// Check if running in GUI mode
pub fn is_gui_mode() -> bool {
    IS_GUI_MODE.load(std::sync::atomic::Ordering::SeqCst)
}

/// Set the webserver port (called from server.rs after binding)
pub fn set_webserver_port(port: u16) {
    WEBSERVER_PORT.store(port, std::sync::atomic::Ordering::SeqCst);
}

/// Get the current webserver port (0 if not started)
pub fn get_webserver_port() -> u16 {
    WEBSERVER_PORT.load(std::sync::atomic::Ordering::SeqCst)
}

/// Set the webserver host (called from server.rs after binding)
pub fn set_webserver_host(host: String) {
    if let Ok(mut h) = WEBSERVER_HOST.write() {
        *h = host;
    }
}

/// Get the current webserver host (empty string if not started)
pub fn get_webserver_host() -> String {
    WEBSERVER_HOST
        .read()
        .ok()
        .map(|h| h.clone())
        .unwrap_or_default()
}

/// Generate and store a new security token (called at webserver startup in GUI mode)
pub fn generate_security_token() -> String {
    use rand::distributions::Alphanumeric;
    use rand::Rng;
    let token: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(64)
        .map(char::from)
        .collect();

    let mut guard = SECURITY_TOKEN.write().unwrap();
    *guard = Some(token.clone());
    token
}

/// Get the current security token (None if not generated)
pub fn get_security_token() -> Option<String> {
    SECURITY_TOKEN.read().unwrap().clone()
}

/// Validate a token against the stored security token
/// Returns true if tokens match, or if not in GUI mode (no validation needed)
pub fn validate_security_token(token: &str) -> bool {
    if !is_gui_mode() {
        return true; // No validation in CLI mode
    }

    match SECURITY_TOKEN.read().unwrap().as_ref() {
        Some(stored) => stored == token,
        None => false, // No token generated yet
    }
}

// ================================================================================================
// TOOLS EXECUTION STATE - PAUSES BACKGROUND SERVICES WHEN TOOLS ARE RUNNING
// ================================================================================================

/// Number of tools currently running (0 = none, >0 = pause background services)
pub static TOOLS_ACTIVE_COUNT: AtomicU32 = AtomicU32::new(0);

/// Mark a tool as started (increments counter, pauses background services)
pub fn tool_started() {
    let prev = TOOLS_ACTIVE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    if prev == 0 {
        crate::logger::info(
            crate::logger::LogTag::Tools,
            "Tool started - pausing token discovery and market updates",
        );
    }
}

/// Mark a tool as finished (decrements counter, resumes when no tools running)
pub fn tool_finished() {
    let prev = TOOLS_ACTIVE_COUNT.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    if prev == 1 {
        crate::logger::info(
            crate::logger::LogTag::Tools,
            "All tools finished - resuming token discovery and market updates",
        );
    } else if prev == 0 {
        // Safety: called when already 0, log warning
        crate::logger::warning(
            crate::logger::LogTag::Tools,
            "tool_finished called when no tools were active",
        );
        // Reset to 0 to prevent underflow
        TOOLS_ACTIVE_COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Check if any tools are currently running
pub fn are_tools_active() -> bool {
    TOOLS_ACTIVE_COUNT.load(std::sync::atomic::Ordering::SeqCst) > 0
}

/// Get count of active tools (for diagnostics)
pub fn active_tools_count() -> u32 {
    TOOLS_ACTIVE_COUNT.load(std::sync::atomic::Ordering::SeqCst)
}

// ================================================================================================
// DASHBOARD ACTIVE TOKEN - PRIORITY BOOST FOR USER-FOCUSED TOKEN
// ================================================================================================

/// Token mint currently being viewed in dashboard (None if no token details open)
/// When a user opens a token details dialog, this token gets highest priority for data fetching.
/// Background batch updates should yield to dashboard-triggered requests.
static DASHBOARD_ACTIVE_TOKEN: RwLock<Option<String>> = RwLock::new(None);

/// Set the token being actively viewed (called when token details opens)
pub fn set_dashboard_active_token(mint: Option<String>) {
    match DASHBOARD_ACTIVE_TOKEN.write() {
        Ok(mut guard) => {
            if let Some(ref m) = mint {
                crate::logger::debug(
                    crate::logger::LogTag::Webserver,
                    &format!("Dashboard focus set: mint={}", m),
                );
            } else if guard.is_some() {
                crate::logger::debug(crate::logger::LogTag::Webserver, "Dashboard focus cleared");
            }
            *guard = mint;
        }
        Err(e) => {
            crate::logger::warning(
                crate::logger::LogTag::Webserver,
                &format!(
                    "Failed to set dashboard active token (poisoned lock): {}",
                    e
                ),
            );
        }
    }
}

/// Get the currently active dashboard token
pub fn get_dashboard_active_token() -> Option<String> {
    match DASHBOARD_ACTIVE_TOKEN.read() {
        Ok(guard) => guard.clone(),
        Err(_) => None,
    }
}

/// Check if a specific token is being actively viewed in the dashboard
/// Used by background update loops to skip tokens that are getting priority updates via the UI
pub fn is_token_active_in_dashboard(mint: &str) -> bool {
    match DASHBOARD_ACTIVE_TOKEN.read() {
        Ok(guard) => guard.as_ref().map(|m| m == mint).unwrap_or(false),
        Err(_) => false,
    }
}

// ================================================================================================
// FORCE STOP STATE - EMERGENCY HALT FOR ALL TRADING OPERATIONS
// ================================================================================================

/// Force stop flag - when true, all trading operations are halted
/// This is an emergency stop that does NOT stop essential services (webserver, events)
/// but prevents any new trades, position updates, or swap executions
static FORCE_STOPPED: AtomicBool = AtomicBool::new(false);

/// Force stop timestamp - when force stop was activated
static FORCE_STOPPED_AT: Lazy<RwLock<Option<DateTime<Utc>>>> = Lazy::new(|| RwLock::new(None));

/// Force stop reason - why force stop was activated
static FORCE_STOPPED_REASON: Lazy<RwLock<String>> = Lazy::new(|| RwLock::new(String::new()));

/// Check if trading is force stopped
pub fn is_force_stopped() -> bool {
    FORCE_STOPPED.load(std::sync::atomic::Ordering::SeqCst)
}

/// Set force stop state with reason
pub fn set_force_stopped(stopped: bool, reason: Option<&str>) {
    FORCE_STOPPED.store(stopped, std::sync::atomic::Ordering::SeqCst);
    if stopped {
        if let Ok(mut ts) = FORCE_STOPPED_AT.write() {
            *ts = Some(Utc::now());
        }
        if let Ok(mut r) = FORCE_STOPPED_REASON.write() {
            *r = reason.unwrap_or("Manual force stop").to_string();
        }
    } else {
        if let Ok(mut ts) = FORCE_STOPPED_AT.write() {
            *ts = None;
        }
        if let Ok(mut r) = FORCE_STOPPED_REASON.write() {
            r.clear();
        }
    }
}

/// Get force stop status details
pub fn get_force_stop_status() -> ForceStopStatus {
    ForceStopStatus {
        is_stopped: is_force_stopped(),
        stopped_at: FORCE_STOPPED_AT.read().ok().and_then(|ts| *ts),
        reason: FORCE_STOPPED_REASON
            .read()
            .ok()
            .map(|r| r.clone())
            .unwrap_or_default(),
    }
}

/// Force stop status structure
#[derive(Debug, Clone, Serialize)]
pub struct ForceStopStatus {
    pub is_stopped: bool,
    pub stopped_at: Option<DateTime<Utc>>,
    pub reason: String,
}
