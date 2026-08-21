//! RPC Module - Chain-neutral RPC infrastructure
//!
//! This module provides the reusable, chain-agnostic machinery that any chain
//! adapter's RPC client sits on top of:
//! - Multi-provider support with automatic failover
//! - Per-provider rate limiting with Governor (GCRA)
//! - Circuit breaker pattern for reliability
//! - SQLite-based statistics
//! - Connection pooling
//!
//! Solana's RPC client, its global composition, its request/response types
//! and its `logsSubscribe` WebSocket transport live at
//! `crate::chains::solana::rpc` and are built on top of this module. This
//! module must never depend on `crate::chains::solana` or any Solana SDK
//! type — see `CLAUDE.md`'s chain-adapter organization rule.
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
//! use crate::chains::solana::rpc::{get_rpc_client, RpcClientMethods};
//!
//! let client = get_rpc_client();
//! let balance = client.get_sol_balance("wallet_address").await?;
//! ```

// ============================================================================
// Core Modules - Chain-neutral infrastructure
// ============================================================================

pub mod circuit_breaker;
pub mod errors;
pub mod gateway;
pub mod manager;
pub mod provider;
pub mod rate_limiter;
pub mod selector;
pub mod stats;
pub mod types;

// ============================================================================
// Re-exports - Circuit Breaker
// ============================================================================

pub use circuit_breaker::{
    CircuitBreakerConfig, CircuitBreakerManager, CircuitBreakerStatus, ProviderCircuitBreaker,
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
    get_global_rpc_stats, get_rpc_stats_db_path, start_rpc_stats_auto_save_service, MethodStats,
    ProviderStats, RpcCallRecord, RpcMinuteBucket, RpcSessionSnapshot, RpcStats, RpcStatsDatabase,
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
