//! Background service coordinating real-time transaction monitoring and WebSocket processing.
// Background service and coordination for the transactions module
//
// This module provides the main background service that coordinates
// real-time transaction monitoring, WebSocket integration, and periodic processing.
//
// Split into sub-modules:
// - `config`: Constants, ServiceConfig, deferred retry queue
// - `lifecycle`: Service start/stop/status, global state management
// - `bootstrap`: Initial transaction history loading
// - `processing`: Main service loop and periodic tasks
// - `websocket`: WebSocket integration for real-time notifications
// - `health`: Health monitoring and metrics

pub mod bootstrap;
pub mod config;
pub mod health;
pub mod lifecycle;
pub mod processing;
pub mod websocket;

// Re-export public API for backward compatibility
pub use lifecycle::{
    get_global_transaction_manager, get_transaction, is_global_transaction_service_running, reprocess_transaction,
    start_global_transaction_service, stop_global_transaction_service,
};
