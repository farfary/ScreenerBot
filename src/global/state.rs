//! Initialization, services readiness, GUI mode, webserver config, tools, and dashboard state.

use std::sync::atomic::{AtomicBool, AtomicU16, AtomicU32};
use std::sync::RwLock;

// =============================================================================
// INITIALIZATION FLAGS
// =============================================================================

/// Master initialization gate — all services (except webserver) wait for this flag.
/// Set to true only after: credentials validated + RPC tested + config saved.
pub static INITIALIZATION_COMPLETE: AtomicBool = AtomicBool::new(false);

/// Optional health monitoring flags (for UI/observability, not for gating service startup).
pub static CREDENTIALS_VALID: AtomicBool = AtomicBool::new(false);
pub static RPC_VALID: AtomicBool = AtomicBool::new(false);

/// preview mode — set when the user skipped wallet + RPC setup at first run.
///
/// In this mode only the preview tier runs (connectivity, events, tokens,
/// filtering, webserver); everything that needs a wallet or RPC stays stopped.
/// Mutually exclusive with full initialization: when the user later completes
/// setup, this is cleared and `INITIALIZATION_COMPLETE` is set instead.
pub static PREVIEW_MODE: AtomicBool = AtomicBool::new(false);

/// Check if initialization is complete and services can start.
pub fn is_initialization_complete() -> bool {
    INITIALIZATION_COMPLETE.load(std::sync::atomic::Ordering::SeqCst)
}

/// Check if the bot is running in preview mode (wallet + RPC skipped).
pub fn is_preview_mode() -> bool {
    PREVIEW_MODE.load(std::sync::atomic::Ordering::SeqCst)
}

/// Set preview mode flag.
pub fn set_preview_mode(enabled: bool) {
    PREVIEW_MODE.store(enabled, std::sync::atomic::Ordering::SeqCst);
}

/// Whether discovery-tier services should run — true in either full mode or
/// preview mode. Used by the discovery-tier services' `is_enabled()`.
pub fn is_preview_or_full() -> bool {
    is_initialization_complete() || is_preview_mode()
}

// =============================================================================
// CORE SERVICES READINESS FLAGS
// =============================================================================

/// Core services readiness flags — prevents trading until all critical services are ready.
pub static CONNECTIVITY_SYSTEM_READY: AtomicBool = AtomicBool::new(false);
pub static TOKENS_SYSTEM_READY: AtomicBool = AtomicBool::new(false);
pub static POSITIONS_SYSTEM_READY: AtomicBool = AtomicBool::new(false);
pub static POOL_SERVICE_READY: AtomicBool = AtomicBool::new(false);
pub static TRANSACTIONS_SYSTEM_READY: AtomicBool = AtomicBool::new(false);

/// Check if all critical services are ready for trading operations.
pub fn are_core_services_ready() -> bool {
    CONNECTIVITY_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst)
        && TOKENS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst)
        && POSITIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst)
        && POOL_SERVICE_READY.load(std::sync::atomic::Ordering::SeqCst)
        && TRANSACTIONS_SYSTEM_READY.load(std::sync::atomic::Ordering::SeqCst)
}

/// Get list of services that are not yet ready (for debugging).
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

// =============================================================================
// GUI MODE AND WEBSERVER CONFIG
// =============================================================================

/// Whether the application is running in GUI (Electron) mode.
pub static IS_GUI_MODE: AtomicBool = AtomicBool::new(false);

/// Dynamic port the webserver is bound to (0 = not started yet).
pub static WEBSERVER_PORT: AtomicU16 = AtomicU16::new(0);

/// Host address the webserver is bound to.
static WEBSERVER_HOST: RwLock<String> = RwLock::new(String::new());

/// Set GUI mode flag (called from Electron main process).
pub fn set_gui_mode(enabled: bool) {
    IS_GUI_MODE.store(enabled, std::sync::atomic::Ordering::SeqCst);
}

/// Check if running in GUI mode.
pub fn is_gui_mode() -> bool {
    IS_GUI_MODE.load(std::sync::atomic::Ordering::SeqCst)
}

/// Set the webserver port (called from server.rs after binding).
pub fn set_webserver_port(port: u16) {
    WEBSERVER_PORT.store(port, std::sync::atomic::Ordering::SeqCst);
}

/// Get the current webserver port (0 if not started).
pub fn get_webserver_port() -> u16 {
    WEBSERVER_PORT.load(std::sync::atomic::Ordering::SeqCst)
}

/// Set the webserver host (called from server.rs after binding).
pub fn set_webserver_host(host: &str) {
    if let Ok(mut h) = WEBSERVER_HOST.write() {
        *h = host.to_string();
    }
}

/// Get the current webserver host (empty string if not started).
pub fn get_webserver_host() -> String {
    WEBSERVER_HOST
        .read()
        .ok()
        .map(|h| h.clone())
        .unwrap_or_default()
}

// =============================================================================
// TOOLS EXECUTION STATE
// =============================================================================

/// Number of tools currently running (0 = none, >0 = pause background services).
pub static TOOLS_ACTIVE_COUNT: AtomicU32 = AtomicU32::new(0);

/// Mark a tool as started (increments counter, pauses background services).
pub fn tool_started() {
    let prev = TOOLS_ACTIVE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    if prev == 0 {
        crate::logger::info(
            crate::logger::LogTag::Tools,
            "Tool started - pausing token discovery and market updates",
        );
    }
}

/// Mark a tool as finished (decrements counter, resumes when no tools running).
pub fn tool_finished() {
    let prev = TOOLS_ACTIVE_COUNT.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    if prev == 1 {
        crate::logger::info(
            crate::logger::LogTag::Tools,
            "All tools finished - resuming token discovery and market updates",
        );
    } else if prev == 0 {
        crate::logger::warning(
            crate::logger::LogTag::Tools,
            "tool_finished called when no tools were active",
        );
        TOOLS_ACTIVE_COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
    }
}

/// Check if any tools are currently running.
pub fn are_tools_active() -> bool {
    TOOLS_ACTIVE_COUNT.load(std::sync::atomic::Ordering::SeqCst) > 0
}

/// Get count of active tools (for diagnostics).
pub fn active_tools_count() -> u32 {
    TOOLS_ACTIVE_COUNT.load(std::sync::atomic::Ordering::SeqCst)
}

// =============================================================================
// DASHBOARD ACTIVE TOKEN
// =============================================================================

/// Token mint currently being viewed in dashboard (None if no token details open).
static DASHBOARD_ACTIVE_TOKEN: RwLock<Option<String>> = RwLock::new(None);

/// Set the token being actively viewed (called when token details opens).
pub fn set_dashboard_active_token(mint: Option<&str>) {
    match DASHBOARD_ACTIVE_TOKEN.write() {
        Ok(mut guard) => {
            if let Some(m) = mint {
                crate::logger::debug(
                    crate::logger::LogTag::Webserver,
                    &format!("Dashboard focus set: mint={m}"),
                );
            } else if guard.is_some() {
                crate::logger::debug(crate::logger::LogTag::Webserver, "Dashboard focus cleared");
            }
            *guard = mint.map(String::from);
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

/// Get the currently active dashboard token.
pub fn get_dashboard_active_token() -> Option<String> {
    match DASHBOARD_ACTIVE_TOKEN.read() {
        Ok(guard) => guard.clone(),
        Err(_) => None,
    }
}

/// Check if a specific token is being actively viewed in the dashboard.
pub fn is_token_active_in_dashboard(mint: &str) -> bool {
    match DASHBOARD_ACTIVE_TOKEN.read() {
        Ok(guard) => guard.as_deref().is_some_and(|m| m == mint),
        Err(_) => false,
    }
}
