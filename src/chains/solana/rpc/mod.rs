//! Solana-specific RPC mechanisms.
//!
//! Owns everything that is Solana-typed: the SDK-facing client, the global
//! client/manager composition, Solana JSON-RPC request/response shapes, the
//! `logsSubscribe` WebSocket transport, and Solana-specific endpoint
//! validation. Built entirely on top of the chain-neutral infrastructure in
//! `crate::rpc` (provider health, retry/backoff, circuit breaking, generic
//! rate limiting, generic stats) — this module depends on that one, never the
//! other way around.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
//!
//! let client = get_rpc_client();
//! let balance = client.get_sol_balance("wallet_address").await?;
//! ```

pub mod client;
pub mod global;
pub mod helpers;
pub mod subscriptions;
pub mod testing;
pub mod types;
pub mod utils;

// ============================================================================
// Re-exports - Client
// ============================================================================

pub use client::{
    ProviderHealthInfo,
    RpcClient,
    RpcClientMethods,
    // Program account types
    RpcFilterType,
    // Token supply types
    RpcTokenAccountBalance,
    // Transaction history types
    SignatureInfo,
    TokenSupply,
};

// ============================================================================
// Re-exports - Global Access Layer (get_rpc_client, etc.)
// ============================================================================

pub use global::{get_rpc_client, init_rpc_client, try_get_rpc_client};

// ============================================================================
// Re-exports - Small helpers
// ============================================================================

pub use helpers::{parse_pubkey, spl_token_program_id};

// ============================================================================
// Re-exports - Subscription Transport
// ============================================================================

pub use subscriptions::{
    connection_state, get_websocket_url, get_websocket_url_from_http, get_websocket_urls,
    subscribe_logs_mentions, subscription_metrics, websocket_url_for_attempt, ConnectionState,
    LogsSubscription, SubscriptionEvent, SubscriptionMetrics,
};

// ============================================================================
// Re-exports - Testing Utilities
// ============================================================================

pub use testing::{
    get_rpc_version, test_rpc_endpoint, test_rpc_endpoints, validate_mainnet, RpcEndpointTestResult,
};

// ============================================================================
// Re-exports - Utility Functions
// ============================================================================

pub use utils::{
    get_ata_rent_from_chain, get_ata_rent_lamports, parse_pubkey_string, sol_to_lamports,
    AtaRentInfo, DEFAULT_ATA_RENT_LAMPORTS,
};

// ============================================================================
// Re-exports - Transaction & Account Types
// ============================================================================

pub use types::{
    PaginatedAccountsResponse, SignatureStatusData, SignatureStatusResponse, SignatureStatusResult,
    TokenAccountInfo, TokenBalance, TransactionData, TransactionDetails, TransactionMeta,
    UiTokenAmount,
};

// ============================================================================
// Convenience Functions
// ============================================================================

/// Get primary RPC URL (masked for security)
///
/// Returns the primary configured RPC URL with sensitive parts masked.
pub async fn get_rpc_url() -> String {
    if let Some(client) = global::try_get_rpc_client() {
        client.primary_url_masked().await
    } else {
        "(not initialized)".to_owned()
    }
}

/// Get WebSocket URL derived from primary RPC
///
/// Converts the primary HTTP RPC URL to its WebSocket equivalent.
pub fn get_ws_url() -> crate::Result<String> {
    subscriptions::get_websocket_url()
}

/// Test if RPC is healthy
///
/// Performs a health check on the RPC connection.
pub async fn is_rpc_healthy() -> bool {
    if let Some(client) = global::try_get_rpc_client() {
        client.get_health().await.is_ok()
    } else {
        false
    }
}

/// Get RPC stats for API response
///
/// Returns aggregated RPC statistics suitable for API responses.
pub async fn get_rpc_stats() -> Option<crate::rpc::stats::RpcStatsResponse> {
    let client = global::try_get_rpc_client()?;
    Some(client.get_stats().await)
}

/// Get health info for all RPC providers
///
/// Returns detailed health information for each configured provider.
pub async fn get_all_provider_health() -> Vec<client::ProviderHealthInfo> {
    if let Some(client) = global::try_get_rpc_client() {
        client.get_provider_health().await
    } else {
        Vec::new()
    }
}
