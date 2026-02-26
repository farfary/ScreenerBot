//! Global state — shutdown signals, status tracking, and shared runtime state.
//!
//! Provides initialization flags, service readiness tracking, GUI mode security,
//! tools execution state, dashboard focus, and emergency force-stop controls.

mod security;
mod state;

pub use security::*;
pub use state::*;

use chrono::{DateTime, Utc};
use serde::Serialize;
use std::sync::atomic::AtomicBool;
use std::sync::LazyLock;
use std::sync::RwLock;

/// Startup timestamp to track when the bot started for trading logic.
pub static STARTUP_TIME: LazyLock<DateTime<Utc>> = LazyLock::new(|| Utc::now());

// =============================================================================
// FORCE STOP STATE
// =============================================================================

/// Force stop flag — when true, all trading operations are halted.
static FORCE_STOPPED: AtomicBool = AtomicBool::new(false);

/// Force stop timestamp — when force stop was activated.
static FORCE_STOPPED_AT: LazyLock<RwLock<Option<DateTime<Utc>>>> =
    LazyLock::new(|| RwLock::new(None));

/// Force stop reason — why force stop was activated.
static FORCE_STOPPED_REASON: LazyLock<RwLock<String>> =
    LazyLock::new(|| RwLock::new(String::new()));

/// Check if trading is force stopped.
pub fn is_force_stopped() -> bool {
    FORCE_STOPPED.load(std::sync::atomic::Ordering::SeqCst)
}

/// Set force stop state with reason.
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

/// Get force stop status details.
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

/// Force stop status structure.
#[derive(Debug, Clone, Serialize)]
pub struct ForceStopStatus {
    pub is_stopped: bool,
    pub stopped_at: Option<DateTime<Utc>>,
    pub reason: String,
}
