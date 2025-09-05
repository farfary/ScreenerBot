//! Pool Constants Module
//!
//! This module contains all pool-related constants, program IDs, and configuration values
//! used across the pool system. It centralizes all constants to avoid duplication and
//! ensure consistency across the codebase.

// =============================================================================
// PROGRAM IDs
// =============================================================================

/// Raydium CPMM Program ID
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";

/// Raydium Legacy AMM Program ID
pub const RAYDIUM_LEGACY_AMM_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

/// Meteora DLMM Program ID
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";

/// Meteora DAMM v2 Program ID
pub const METEORA_DAMM_V2_PROGRAM_ID: &str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";

/// Orca Whirlpool Program ID
pub const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";

/// Pump.fun AMM Program ID
pub const PUMP_FUN_AMM_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";

/// Raydium CLMM Program ID
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

// =============================================================================
// RPC AND NETWORKING CONSTANTS
// =============================================================================

/// Maximum number of accounts to fetch in a single RPC batch
pub const RPC_MULTIPLE_ACCOUNTS_BATCH_SIZE: usize = 100;

// =============================================================================
// CACHE TTL CONSTANTS (Time-To-Live)
// =============================================================================

/// Price cache TTL for pool service (seconds)
pub const PRICE_CACHE_TTL_SECS: i64 = 30;

/// Token account cache TTL for pool fetcher (seconds)
pub const TOKEN_ACCOUNT_CACHE_TTL_SECS: i64 = 300; // 5 minutes

/// Account data cache TTL for pool cleanup (seconds)
pub const ACCOUNT_DATA_CACHE_TTL_SECS: i64 = 300; // 5 minutes

/// Pool data cache TTL for pool cleanup (seconds)
pub const POOL_DATA_CACHE_TTL_SECS: i64 = 600; // 10 minutes

/// Tracked tokens TTL for pool cleanup (seconds)
pub const TRACKED_TOKENS_TTL_SECS: i64 = 1800; // 30 minutes

/// Pool metadata cache TTL (hours)
pub const POOL_METADATA_CACHE_TTL_HOURS: i64 = 24; // Keep for 24 hours

/// Pool metadata stale time (minutes)
pub const POOL_METADATA_STALE_MINUTES: i64 = 10;

/// Maximum price history age (hours)
pub const MAX_PRICE_HISTORY_AGE_HOURS: i64 = 24;

// =============================================================================
// SERVICE INTERVAL CONSTANTS (Background Tasks)
// =============================================================================

/// Tokens list update interval (seconds)
pub const TOKENS_LIST_INTERVAL_SECS: u64 = 300; // 5 minutes

/// Pool discovery interval (seconds)
pub const POOL_DISCOVERY_INTERVAL_SECS: u64 = 60; // 1 minute

/// Account fetch interval (seconds)
pub const ACCOUNT_FETCH_INTERVAL_SECS: u64 = 5; // 5 seconds

/// Price calculation interval (seconds)
pub const PRICE_CALC_INTERVAL_SECS: u64 = 1; // 1 second

/// Cleanup interval (seconds)
pub const CLEANUP_INTERVAL_SECS: u64 = 3600; // 1 hour

/// State monitor interval (seconds)
pub const STATE_MONITOR_INTERVAL_SECS: u64 = 30; // 30 seconds

/// Pool monitor interval (seconds)
pub const MONITOR_INTERVAL_SECS: u64 = 30; // 30 seconds

// =============================================================================
// CAPACITY AND LIMIT CONSTANTS
// =============================================================================

/// Maximum number of tracked tokens
pub const MAX_TRACKED_TOKENS: usize = 10000;

/// Maximum cleanup batch size
pub const MAX_CLEANUP_BATCH_SIZE: usize = 1000;

/// Task health timeout (seconds)
pub const TASK_HEALTH_TIMEOUT_SECS: i64 = 300; // 5 minutes

// =============================================================================
// DATABASE CONSTANTS
// =============================================================================

/// Pools database file path
pub const POOLS_DB_PATH: &str = "data/pools.db";

// =============================================================================
// POOL TYPE DISPLAY NAMES
// =============================================================================

/// Raydium CPMM display name
pub const RAYDIUM_CPMM_DISPLAY_NAME: &str = "Raydium CPMM";

/// Raydium Legacy AMM display name
pub const RAYDIUM_LEGACY_DISPLAY_NAME: &str = "Raydium Legacy AMM";

/// Meteora DLMM display name
pub const METEORA_DLMM_DISPLAY_NAME: &str = "Meteora DLMM";

/// Meteora DAMM v2 display name
pub const METEORA_DAMM_DISPLAY_NAME: &str = "Meteora DAMM v2";

/// Orca Whirlpool display name
pub const ORCA_WHIRLPOOL_DISPLAY_NAME: &str = "Orca Whirlpool";

/// Pump.fun AMM display name
pub const PUMP_FUN_DISPLAY_NAME: &str = "Pump.fun AMM";

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Get all supported program IDs as a vector
pub fn get_all_program_ids() -> Vec<&'static str> {
    vec![
        RAYDIUM_CPMM_PROGRAM_ID,
        RAYDIUM_LEGACY_AMM_PROGRAM_ID,
        METEORA_DLMM_PROGRAM_ID,
        METEORA_DAMM_V2_PROGRAM_ID,
        ORCA_WHIRLPOOL_PROGRAM_ID,
        PUMP_FUN_AMM_PROGRAM_ID,
        RAYDIUM_CLMM_PROGRAM_ID
    ]
}

/// Check if a program ID is supported
pub fn is_supported_program_id(program_id: &str) -> bool {
    get_all_program_ids().contains(&program_id)
}
