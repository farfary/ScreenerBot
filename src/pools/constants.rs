/// Pool system constants and configuration

// Cache TTL settings

pub const POOL_CACHE_TTL_SECONDS: i64 = 600; // 10 minutes
pub const PRICE_CACHE_TTL_SECONDS: i64 = 240; // 4 minutes

// Price history settings
pub const MAX_PRICE_HISTORY_POINTS: usize = 500; // Memory efficiency limit

// Batch processing configuration
pub const MAX_TOKENS_PER_BATCH: usize = 30;
pub const WATCHLIST_BATCH_SIZE: usize = 150;
pub const DISCOVERY_BATCH_SIZE: usize = 10;
pub const INITIAL_TOKEN_LOAD_COUNT: usize = 100;

// Discovery timing
pub const DISCOVERY_CYCLE_DELAY_SECS: u64 = 5;
pub const DISCOVERY_BATCH_DELAY_MS: u64 = 200;
pub const DEXSCREENER_REQUEST_DELAY_MS: u64 = 500;

// Watchlist and priority management
pub const PRIORITY_UPDATE_INTERVAL_SECS: u64 = 3;
pub const WATCHLIST_UPDATE_INTERVAL_SECS: u64 = 1;
pub const MAX_WATCHLIST_SIZE: usize = 100;
pub const WATCHLIST_EXPIRY_HOURS: i64 = 24;
pub const MAX_CONSECUTIVE_FAILURES: u32 = 5;
pub const WATCHLIST_CLEANUP_INTERVAL_SECS: u64 = 300; // 5 minutes

// Ad-hoc warming configuration
pub const ADHOC_BATCH_SIZE: usize = 300;
pub const ADHOC_UPDATE_INTERVAL_SECS: u64 = 1;

// RPC configuration
pub const RPC_MULTIPLE_ACCOUNTS_BATCH_SIZE: usize = 20;

// Pool price history settings
pub const POOL_PRICE_HISTORY_MAX_AGE_HOURS: i64 = 24; // Keep 24 hours of history
pub const POOL_PRICE_HISTORY_MAX_ENTRIES: usize = 1000; // Max entries per pool cache
pub const POOL_PRICE_HISTORY_SAVE_INTERVAL_SECONDS: u64 = 30;

// Price validation thresholds
pub const MIN_CONFIDENCE_THRESHOLD: f64 = 0.5;
pub const DEFAULT_CONFIDENCE: f64 = 1.0;
pub const LIQUIDITY_MULTIPLIER: f64 = 2.0; // SOL reserves * 2 for total liquidity estimation

// SOL mint address
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

// Pool program IDs
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";
pub const RAYDIUM_LEGACY_AMM_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
pub const METEORA_DAMM_V2_PROGRAM_ID: &str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
pub const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
pub const PUMP_FUN_AMM_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";

// Minimum liquidity requirement
pub const MIN_POOL_LIQUIDITY_USD: f64 = 10.0;

/// Get display name for pool program ID
pub fn get_pool_program_display_name(program_id: &str) -> &'static str {
    match program_id {
        RAYDIUM_CPMM_PROGRAM_ID => "Raydium CPMM",
        RAYDIUM_LEGACY_AMM_PROGRAM_ID => "Raydium Legacy AMM",
        RAYDIUM_CLMM_PROGRAM_ID => "Raydium CLMM",
        METEORA_DAMM_V2_PROGRAM_ID => "Meteora DAMM v2",
        METEORA_DLMM_PROGRAM_ID => "Meteora DLMM",
        ORCA_WHIRLPOOL_PROGRAM_ID => "Orca Whirlpool",
        PUMP_FUN_AMM_PROGRAM_ID => "Pump.fun AMM",
        _ => "Unknown Pool",
    }
}
