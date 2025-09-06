/// Pool system constants and configuration

// Cache TTL settings

pub const POOL_CACHE_TTL_SECONDS: i64 = 600; // 10 minutes
pub const PRICE_CACHE_TTL_SECONDS: i64 = 240; // 4 minutes

// Batch processing configuration
pub const MAX_TOKENS_PER_BATCH: usize = 30;
pub const WATCHLIST_BATCH_SIZE: usize = 150;

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
