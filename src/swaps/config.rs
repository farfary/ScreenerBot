/// Swap router configuration - hardcoded parameters
/// Simple configuration for GMGN, Jupiter, and Raydium swap routers

// =============================================================================
// ROUTER ENABLE/DISABLE FLAGS
// =============================================================================

/// Enable/disable GMGN router
pub const GMGN_ENABLED: bool = true;

/// Enable/disable Jupiter router
pub const JUPITER_ENABLED: bool = true;

/// Enable/disable Raydium router (deprecated - API no longer available)
pub const RAYDIUM_ENABLED: bool = false;

// =============================================================================
// COMMON CONFIGURATION (Used by all routers)
// =============================================================================

/// SOL token mint address
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Quote request timeout (seconds) - unified for all routers
pub const QUOTE_TIMEOUT_SECS: u64 = 15;

/// API request timeout (seconds) - unified for all routers
pub const API_TIMEOUT_SECS: u64 = 30;

/// Retry attempts for failed requests - unified for all routers
pub const RETRY_ATTEMPTS: u32 = 3;

/// Transaction confirmation timeout (seconds) - Regular transactions
pub const TRANSACTION_CONFIRMATION_TIMEOUT_SECS: u64 = 300;

/// Transaction confirmation timeout (seconds) - Priority transactions
pub const PRIORITY_CONFIRMATION_TIMEOUT_SECS: u64 = 30;

/// Transaction confirmation maximum attempts - Regular transactions (increased for better reliability)
pub const TRANSACTION_CONFIRMATION_MAX_ATTEMPTS: u32 = 20;

/// Transaction confirmation maximum attempts - Priority transactions
pub const PRIORITY_CONFIRMATION_MAX_ATTEMPTS: u32 = 15;

/// Transaction confirmation retry delay (milliseconds) - Regular transactions
pub const TRANSACTION_CONFIRMATION_RETRY_DELAY_MS: u64 = 3000;

/// Fast failure detection threshold - abort if transaction not found after this many attempts
pub const FAST_FAILURE_THRESHOLD_ATTEMPTS: u32 = 10;

/// Transaction confirmation retry delay (milliseconds) - Priority transactions
pub const PRIORITY_CONFIRMATION_RETRY_DELAY_MS: u64 = 1000;

// =============================================================================
// SLIPPAGE CONFIGURATION (Now loaded from centralized config)
// =============================================================================
// NOTE: Slippage values are now loaded from config system via:
// - with_config(|cfg| cfg.swaps.slippage_quote_default_pct)
// - with_config(|cfg| cfg.swaps.slippage_exit_retry_steps_pct)
// Legacy re-exports removed - use config access directly where needed

// =============================================================================
// GMGN ROUTER SPECIFIC CONFIGURATION
// =============================================================================

/// GMGN API base URL for quotes
pub const GMGN_QUOTE_API: &str = "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route";

/// GMGN partner identifier
pub const GMGN_PARTNER: &str = "screenerbot";

/// GMGN default anti-MEV setting (boolean)
pub const GMGN_ANTI_MEV: bool = false;

/// GMGN network and priority fees in SOL (required for GMGN API)
pub const GMGN_FEE_SOL: f64 = 0.0;

/// GMGN default swap mode - "ExactIn" or "ExactOut"
/// ExactIn: Specify exact input amount, output amount is calculated
/// ExactOut: Specify exact output amount, input amount is calculated
pub const GMGN_DEFAULT_SWAP_MODE: &str = "ExactIn";

// =============================================================================
// JUPITER ROUTER SPECIFIC CONFIGURATION
// =============================================================================

/// Jupiter quote API URL
pub const JUPITER_QUOTE_API: &str = "https://lite-api.jup.ag/swap/v1/quote";

/// Jupiter swap API URL
pub const JUPITER_SWAP_API: &str = "https://lite-api.jup.ag/swap/v1/swap";

/// Jupiter dynamic compute unit limit
pub const JUPITER_DYNAMIC_COMPUTE_UNIT_LIMIT: bool = false;

/// Jupiter default priority fee (lamports) - Used in transaction execution, not quotes
pub const JUPITER_DEFAULT_PRIORITY_FEE: u64 = 1_000;

/// Jupiter default swap mode - "ExactIn" or "ExactOut"
/// ExactIn: Specify exact input amount, output amount is calculated
/// ExactOut: Specify exact output amount, input amount is calculated
pub const JUPITER_DEFAULT_SWAP_MODE: &str = "ExactIn";

// =============================================================================
// RAYDIUM ROUTER CONFIGURATION (DEPRECATED)
// =============================================================================

/// Raydium quote API URL (deprecated - no longer available)
pub const RAYDIUM_QUOTE_API: &str = "https://api-v3.raydium.io/mint/price";

/// Raydium swap API URL (deprecated - no longer available)
pub const RAYDIUM_SWAP_API: &str = "https://api-v3.raydium.io/compute/swap-base-in";
