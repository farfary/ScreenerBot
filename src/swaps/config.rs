/// Swap router configuration - hardcoded parameters
/// Simple configuration for GMGN, Jupiter, and Raydium swap routers

// =============================================================================
// ROUTER ENABLE/DISABLE FLAGS
// =============================================================================
// 
// To enable/disable specific routers, change these boolean values:
// - Set to `true` to enable a router 
// - Set to `false` to disable a router
// 
// Example: To disable GMGN and only use Jupiter:
// pub const GMGN_ENABLED: bool = false;
// pub const JUPITER_ENABLED: bool = true;
// pub const RAYDIUM_ENABLED: bool = false;

/// Enable/disable GMGN router
pub const GMGN_ENABLED: bool = true;

/// Enable/disable Jupiter router
pub const JUPITER_ENABLED: bool = true;

/// Enable/disable Raydium router (deprecated - API no longer available)
pub const RAYDIUM_ENABLED: bool = false;

// =============================================================================
// GMGN ROUTER CONFIGURATION
// =============================================================================

/// GMGN API base URL for quotes
pub const GMGN_QUOTE_API: &str = "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route";

/// GMGN partner identifier
pub const GMGN_PARTNER: &str = "screenerbot";

/// GMGN default anti-MEV setting
pub const GMGN_ANTI_MEV: bool = false;

/// GMGN default slippage tolerance (percentage)
pub const GMGN_DEFAULT_SLIPPAGE: f64 = 15.0;

/// GMGN default fee (percentage)
pub const GMGN_DEFAULT_FEE: f64 = 0.5;

/// GMGN API timeout (seconds)
pub const GMGN_API_TIMEOUT_SECS: u64 = 30;

/// GMGN quote timeout (seconds)
pub const GMGN_QUOTE_TIMEOUT_SECS: u64 = 15;

/// GMGN retry attempts
pub const GMGN_RETRY_ATTEMPTS: u32 = 3;

// =============================================================================
// JUPITER ROUTER CONFIGURATION
// =============================================================================

/// Jupiter quote API URL
pub const JUPITER_QUOTE_API: &str = "https://lite-api.jup.ag/swap/v1/quote";

/// Jupiter swap API URL
pub const JUPITER_SWAP_API: &str = "https://lite-api.jup.ag/swap/v1/swap";

/// Jupiter default slippage tolerance (percentage)
pub const JUPITER_DEFAULT_SLIPPAGE: f64 = 15.0;

/// Jupiter default fee (percentage)
pub const JUPITER_DEFAULT_FEE: f64 = 0.5;

/// Jupiter API timeout (seconds)
pub const JUPITER_API_TIMEOUT_SECS: u64 = 30;

/// Jupiter quote timeout (seconds)
pub const JUPITER_QUOTE_TIMEOUT_SECS: u64 = 15;

/// Jupiter retry attempts
pub const JUPITER_RETRY_ATTEMPTS: u32 = 3;

/// Jupiter dynamic compute unit limit
pub const JUPITER_DYNAMIC_COMPUTE_UNIT_LIMIT: bool = true;

/// Jupiter default priority fee (lamports)
pub const JUPITER_DEFAULT_PRIORITY_FEE: u64 = 100_000;

// =============================================================================
// RAYDIUM ROUTER CONFIGURATION (DEPRECATED)
// =============================================================================

/// Raydium quote API URL (deprecated - no longer available)
pub const RAYDIUM_QUOTE_API: &str = "https://api-v3.raydium.io/mint/price";

/// Raydium swap API URL (deprecated - no longer available)
pub const RAYDIUM_SWAP_API: &str = "https://api-v3.raydium.io/compute/swap-base-in";

/// Raydium default slippage tolerance (percentage)
pub const RAYDIUM_DEFAULT_SLIPPAGE: f64 = 15.0;

/// Raydium default fee (percentage)
pub const RAYDIUM_DEFAULT_FEE: f64 = 0.5;

/// Raydium API timeout (seconds)
pub const RAYDIUM_API_TIMEOUT_SECS: u64 = 30;

/// Raydium quote timeout (seconds)
pub const RAYDIUM_QUOTE_TIMEOUT_SECS: u64 = 15;

/// Raydium retry attempts
pub const RAYDIUM_RETRY_ATTEMPTS: u32 = 3;

// =============================================================================
// GENERAL SWAP CONFIGURATION
// =============================================================================

/// SOL token mint address
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Maximum slippage tolerance allowed (percentage)
pub const MAX_SLIPPAGE_TOLERANCE: f64 = 50.0;

/// Minimum slippage tolerance allowed (percentage)
pub const MIN_SLIPPAGE_TOLERANCE: f64 = 1.0;

/// Maximum fee percentage allowed
pub const MAX_FEE_PERCENTAGE: f64 = 5.0;

/// Quote comparison timeout (seconds)
pub const QUOTE_COMPARISON_TIMEOUT_SECS: u64 = 20;

/// Transaction confirmation timeout (seconds)
pub const TRANSACTION_CONFIRMATION_TIMEOUT_SECS: u64 = 300;
