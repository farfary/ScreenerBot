//! Background service coordinating own-wallet transaction bootstrap and housekeeping.
// Background service and coordination for the transactions module
//
// Real-time detection (WebSocket + poll fallback + gap-fill) lives in
// `wallets::watch` -- one funnel, three triggers, shared by every subject including
// the own wallet. This service now owns only what is genuinely own-wallet-specific:
// the one-time historical bootstrap and ongoing deferred-retry/cleanup housekeeping.
//
// Split into sub-modules:
// - `config`: Constants, ServiceConfig, deferred retry queue
// - `lifecycle`: Service start/stop/status, global state management
// - `bootstrap`: Initial transaction history loading
// - `processing`: Own-wallet activity consumer + periodic housekeeping
// - `health`: Housekeeping metrics

pub mod bootstrap;
pub mod config;
pub mod health;
pub mod lifecycle;
pub mod processing;

// Re-export public API for backward compatibility
pub use lifecycle::{
    get_global_transaction_manager, get_transaction, is_global_transaction_service_running,
    reprocess_transaction, start_global_transaction_service, stop_global_transaction_service,
};
