//! RPC Module - Centralized RPC client management
//!
//! This module provides:
//! - Multi-provider support with automatic failover
//! - Per-provider rate limiting with Governor (GCRA)
//! - Circuit breaker pattern for reliability
//! - SQLite-based statistics
//! - Connection pooling
//!
//! # Architecture
//!
//! ```text
//! RpcManager (orchestrator)
//!   ├── ProviderConfigs (static configuration)
//!   ├── ProviderStates (runtime health/stats)
//!   ├── RateLimiterManager (per-provider rate limits)
//!   ├── CircuitBreakerManager (failover logic)
//!   ├── StatsManager (SQLite-backed statistics)
//!   └── Selectors (routing strategies)
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::rpc::{get_rpc_client, RpcClientMethods};
//!
//! let client = get_rpc_client();
//! let balance = client.get_sol_balance("wallet_address").await?;
//! ```

// ============================================================================
// Core Modules - New Architecture
// ============================================================================

pub mod circuit_breaker;
pub mod client;
pub mod compat;
pub mod errors;
pub mod legacy_types;
pub mod manager;
pub mod provider;
pub mod rate_limiter;
pub mod selector;
pub mod stats;
pub mod testing;
pub mod types;
pub mod utils;
pub mod websocket;

// ============================================================================
// Re-exports - Circuit Breaker
// ============================================================================

pub use circuit_breaker::{
    CircuitBreakerConfig, CircuitBreakerManager, CircuitBreakerStatus, ProviderCircuitBreaker,
};

// ============================================================================
// Re-exports - Client
// ============================================================================

// The new RpcClient is available as `rpc::client::RpcClient` (not re-exported at top level)
// This avoids confusion with the legacy RpcClient which is still used by most code
pub use client::{
    ProviderHealthInfo, RpcClientMethods,
    // Transaction history types
    SignatureInfo,
    // Program account types
    RpcFilterType,
    // Token supply types
    RpcTokenAccountBalance,
    TokenSupply,
};

// ============================================================================
// Re-exports - Errors
// ============================================================================

pub use errors::RpcError;

// ============================================================================
// Re-exports - Manager (main orchestrator)
// ============================================================================

pub use manager::{get_or_init_rpc_manager, get_rpc_manager, init_rpc_manager, RpcManager};

// ============================================================================
// Re-exports - Provider
// ============================================================================

pub use provider::{
    config::ProviderConfig, derive_websocket_url, detect_provider_kind, generate_provider_id,
    ProviderRef, RpcProvider,
};

// ============================================================================
// Re-exports - Rate Limiter
// ============================================================================

pub use rate_limiter::{
    ExponentialBackoff, ProviderRateLimiter, RateLimiterManager, RateLimiterStatus,
    SlidingWindowTracker,
};

// ============================================================================
// Re-exports - Selector
// ============================================================================

pub use selector::{create_selector, ProviderSelector};

// ============================================================================
// Re-exports - Stats
// ============================================================================

pub use stats::{
    get_rpc_stats_db_path, MethodStats, ProviderStats, RpcCallRecord, RpcStatsDatabase,
    RpcStatsResponse, SessionStats, StatsCollector, StatsManager, StatsMessage, StatsSnapshot,
    TimeBucketStats,
};

// ============================================================================
// Re-exports - Types
// ============================================================================

pub use types::{
    mask_url, CircuitState, ProviderKind, ProviderState, RpcCallResult, RpcMethod,
    SelectionStrategy,
};

// ============================================================================
// Re-exports - WebSocket Utilities
// ============================================================================

pub use websocket::{
    build_logs_subscribe_payload, create_account_subscribe_payload, get_websocket_url,
    get_websocket_url_from_http, logs_contains_initialize_account, logs_contains_initialize_mint,
};

// ============================================================================
// Re-exports - Testing Utilities
// ============================================================================

pub use testing::{
    get_rpc_version, test_rpc_endpoint, test_rpc_endpoints, validate_mainnet,
    RpcEndpointTestResult,
};

// ============================================================================
// Re-exports - Utility Functions
// ============================================================================

pub use utils::{
    get_ata_rent_from_chain, get_ata_rent_lamports, parse_pubkey_string, sol_to_lamports,
    AtaRentInfo, DEFAULT_ATA_RENT_LAMPORTS,
};

// ============================================================================
// Re-exports - Compatibility Layer (get_rpc_client, etc.)
// ============================================================================

pub use compat::{get_new_rpc_client, init_new_rpc_client, is_new_rpc_initialized};

// ============================================================================
// Re-exports - Legacy Types (for backward compatibility during migration)
// ============================================================================

pub use legacy_types::{
    // Transaction types used throughout the codebase
    PaginatedAccountsResponse, SignatureStatusData, SignatureStatusResponse, SignatureStatusResult,
    TokenAccountInfo, TokenBalance, TransactionData, TransactionDetails, TransactionMeta,
    UiTokenAmount,
    // Legacy stats types (prefer rpc::stats for new code)
    PersistedRpcStats, RpcMinuteBucket, RpcRateLimiter, RpcSessionSnapshot, RpcStats,
    // Legacy RpcClient (for gradual migration)
    LegacyRpcClient,
};

// ============================================================================
// Global RPC Client Access
// ============================================================================

// Re-export the global client access from rpc_legacy
// This provides the primary get_rpc_client() function and the legacy RpcClient type
pub use crate::rpc_legacy::{
    get_global_rpc_stats, get_rpc_client, init_rpc_client, parse_pubkey,
    save_global_rpc_stats, spl_token_program_id, start_rpc_stats_auto_save_service,
    // The legacy RpcClient type used by all existing code
    RpcClient,
};

// Re-export new RpcClient type under different name to avoid confusion
pub use client::RpcClient as NewRpcClient;

// ============================================================================
// Convenience Functions
// ============================================================================

/// Get primary RPC URL (masked for security)
///
/// Returns the primary configured RPC URL with sensitive parts masked.
pub async fn get_rpc_url() -> String {
    if let Some(client) = compat::try_get_new_rpc_client() {
        client.primary_url_masked().await
    } else {
        String::from("(not initialized)")
    }
}

/// Get WebSocket URL derived from primary RPC
///
/// Converts the primary HTTP RPC URL to its WebSocket equivalent.
pub fn get_ws_url() -> Result<String, crate::errors::ScreenerBotError> {
    websocket::get_websocket_url()
}

/// Test if RPC is healthy
///
/// Performs a health check on the RPC connection.
pub async fn is_rpc_healthy() -> bool {
    if let Some(client) = compat::try_get_new_rpc_client() {
        client.get_health().await.is_ok()
    } else {
        false
    }
}

/// Get RPC stats for API response
///
/// Returns aggregated RPC statistics suitable for API responses.
pub async fn get_new_rpc_stats() -> Option<stats::RpcStatsResponse> {
    compat::try_get_new_rpc_client().map(|client| {
        // Use block_on for sync contexts, or return a future
        // For now, we create a simple stats from the manager
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(client.get_stats())
        })
    })
}

/// Get health info for all RPC providers
///
/// Returns detailed health information for each configured provider.
pub async fn get_all_provider_health() -> Vec<client::ProviderHealthInfo> {
    if let Some(client) = compat::try_get_new_rpc_client() {
        client.get_provider_health().await
    } else {
        Vec::new()
    }
}