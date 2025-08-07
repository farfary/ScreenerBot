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
// COMMON CONFIGURATION
// =============================================================================

/// SOL token mint address
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

/// Quote request timeout (seconds) - unified for all routers
pub const QUOTE_TIMEOUT_SECS: u64 = 15;

/// API request timeout (seconds) - unified for all routers  
pub const API_TIMEOUT_SECS: u64 = 30;

/// Retry attempts for failed requests - unified for all routers
pub const RETRY_ATTEMPTS: u32 = 3;

/// Transaction confirmation timeout (seconds)
pub const TRANSACTION_CONFIRMATION_TIMEOUT_SECS: u64 = 300;

// =============================================================================
// GMGN ROUTER CONFIGURATION
// =============================================================================

/// GMGN API base URL for quotes
pub const GMGN_QUOTE_API: &str = "https://gmgn.ai/defi/router/v1/sol/tx/get_swap_route";

/// GMGN partner identifier
pub const GMGN_PARTNER: &str = "screenerbot";

/// GMGN default anti-MEV setting
pub const GMGN_ANTI_MEV: bool = false;

/// GMGN default swap mode - "ExactIn" or "ExactOut"
/// ExactIn: Specify exact input amount, output amount is calculated
/// ExactOut: Specify exact output amount, input amount is calculated
pub const GMGN_DEFAULT_SWAP_MODE: &str = "ExactIn";

// =============================================================================
// JUPITER ROUTER CONFIGURATION
// =============================================================================

/// Jupiter quote API URL
pub const JUPITER_QUOTE_API: &str = "https://lite-api.jup.ag/swap/v1/quote";

/// Jupiter swap API URL
pub const JUPITER_SWAP_API: &str = "https://lite-api.jup.ag/swap/v1/swap";

/// Jupiter dynamic compute unit limit
pub const JUPITER_DYNAMIC_COMPUTE_UNIT_LIMIT: bool = true;

/// Jupiter default priority fee (lamports) - Reduced to minimize transaction costs
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

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Get the default swap mode for GMGN router
pub fn get_gmgn_default_swap_mode() -> &'static str {
    GMGN_DEFAULT_SWAP_MODE
}

/// Get the default swap mode for Jupiter router
pub fn get_jupiter_default_swap_mode() -> &'static str {
    JUPITER_DEFAULT_SWAP_MODE
}

/// Get the appropriate default swap mode for a given router
pub fn get_default_swap_mode_for_router(router: &str) -> &'static str {
    match router.to_lowercase().as_str() {
        "gmgn" => GMGN_DEFAULT_SWAP_MODE,
        "jupiter" => JUPITER_DEFAULT_SWAP_MODE,
        _ => "ExactIn", // Default fallback
    }
}

/// Validate swap mode value
pub fn is_valid_swap_mode(swap_mode: &str) -> bool {
    matches!(swap_mode, "ExactIn" | "ExactOut")
}