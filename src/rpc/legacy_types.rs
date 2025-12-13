//! Legacy RPC types for backward compatibility
//!
//! This module re-exports types from the legacy RPC module that are still
//! used throughout the codebase. These will be migrated to proper modules
//! in future phases.
//!
//! # Types Included
//!
//! - `TokenAccountInfo` - Token account details
//! - `TransactionDetails`, `TransactionData`, `TransactionMeta` - Transaction types
//! - `TokenBalance`, `UiTokenAmount` - Token balance types
//! - `RpcStats`, `RpcMinuteBucket`, `RpcSessionSnapshot` - Statistics types
//! - `RpcEndpointTestResult` - Endpoint testing (now in rpc::testing)
//!
//! # Migration Note
//!
//! Code should gradually migrate to use the new types from appropriate modules:
//! - Stats types → `rpc::stats::*`
//! - Testing types → `rpc::testing::*`

// Re-export all types from rpc_legacy for backward compatibility
pub use crate::rpc_legacy::{
    // Token account types
    TokenAccountInfo,
    
    // Transaction types
    TransactionData,
    TransactionDetails,
    TransactionMeta,
    TokenBalance,
    UiTokenAmount,
    
    // Signature types
    SignatureStatusData,
    SignatureStatusResponse,
    SignatureStatusResult,
    
    // Pagination
    PaginatedAccountsResponse,
    
    // Legacy stats types (new stats system in rpc::stats)
    RpcMinuteBucket,
    RpcSessionSnapshot,
    RpcStats,
    PersistedRpcStats,
    
    // Rate limiter
    RpcRateLimiter,
    
    // ATA rent (also available via rpc::utils)
    AtaRentInfo,
    
    // Legacy RpcClient (for gradual migration)
    RpcClient as LegacyRpcClient,
};
