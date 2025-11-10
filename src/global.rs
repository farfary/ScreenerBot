use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicBool;

// Startup timestamp to track when the bot started for trading logic
pub static STARTUP_TIME: Lazy<DateTime<Utc>> = Lazy::new(|| Utc::now());

// ================================================================================================
// 🚀 STARTUP COORDINATION SYSTEM - ENSURES PROPER SERVICE INITIALIZATION ORDER
// ================================================================================================

// ──────────────────────────────────────────────────────────────────────────────────────────────
// 🔐 LICENSE-GATED INITIALIZATION FLAGS
// ──────────────────────────────────────────────────────────────────────────────────────────────

/// Master initialization gate - all services (except webserver) wait for this flag
/// Set to true only after: credentials validated + RPC tested + license verified + config saved
pub static INITIALIZATION_COMPLETE: AtomicBool = AtomicBool::new(false);

/// Optional health monitoring flags (for UI/observability, not for gating service startup)
pub static CREDENTIALS_VALID: AtomicBool = AtomicBool::new(false);
pub static RPC_VALID: AtomicBool = AtomicBool::new(false);
pub static LICENSE_VALID: AtomicBool = AtomicBool::new(false);

/// Check if initialization is complete and services can start
/// This is the single source of truth for service enablement (except webserver)
pub fn is_initialization_complete() -> bool {
    INITIALIZATION_COMPLETE.load(std::sync::atomic::Ordering::SeqCst)
}

// ──────────────────────────────────────────────────────────────────────────────────────────────
// 🎯 CORE SERVICES READINESS FLAGS (Post-Initialization)
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