/// Configuration module for swap routers
/// 
/// This module provides hardcoded configuration parameters to enable/disable
/// specific swap routers (GMGN, Jupiter, Raydium) independently.

use crate::logger::{log, LogTag};

// =============================================================================
// ROUTER ENABLE/DISABLE CONFIGURATION
// =============================================================================

/// Enable/disable GMGN router
/// Set to `false` to completely disable GMGN swap routing
pub const ENABLE_GMGN_ROUTER: bool = true;

/// Enable/disable Jupiter router  
/// Set to `false` to completely disable Jupiter swap routing
pub const ENABLE_JUPITER_ROUTER: bool = true;

/// Enable/disable Raydium router
/// Set to `false` to completely disable Raydium swap routing
/// NOTE: Raydium direct API is deprecated - this controls legacy support
pub const ENABLE_RAYDIUM_ROUTER: bool = false; // DISABLED by default due to API deprecation

// =============================================================================
// CONFIGURATION VALIDATION AND LOGGING
// =============================================================================

/// Initialize and validate router configuration
pub fn init_router_config() {
    log(
        LogTag::Swap,
        "CONFIG",
        &format!(
            "🔧 Router Configuration:\n  • GMGN: {}\n  • Jupiter: {}\n  • Raydium: {} {}",
            if ENABLE_GMGN_ROUTER { "✅ ENABLED" } else { "❌ DISABLED" },
            if ENABLE_JUPITER_ROUTER { "✅ ENABLED" } else { "❌ DISABLED" },
            if ENABLE_RAYDIUM_ROUTER { "✅ ENABLED" } else { "❌ DISABLED" },
            if !ENABLE_RAYDIUM_ROUTER { "(Deprecated API)" } else { "" }
        )
    );

    // Validate that at least one router is enabled
    let enabled_count = [ENABLE_GMGN_ROUTER, ENABLE_JUPITER_ROUTER, ENABLE_RAYDIUM_ROUTER]
        .iter()
        .filter(|&&enabled| enabled)
        .count();

    if enabled_count == 0 {
        log(
            LogTag::Swap,
            "ERROR",
            "❌ CRITICAL: All swap routers are disabled! At least one router must be enabled."
        );
        panic!("At least one swap router must be enabled");
    }

    log(
        LogTag::Swap,
        "CONFIG",
        &format!("✅ Router configuration validated: {} router(s) enabled", enabled_count)
    );
}

/// Check if GMGN router is enabled
pub fn is_gmgn_enabled() -> bool {
    ENABLE_GMGN_ROUTER
}

/// Check if Jupiter router is enabled  
pub fn is_jupiter_enabled() -> bool {
    ENABLE_JUPITER_ROUTER
}

/// Check if Raydium router is enabled
pub fn is_raydium_enabled() -> bool {
    ENABLE_RAYDIUM_ROUTER
}

/// Get list of enabled routers for logging/debugging
pub fn get_enabled_routers() -> Vec<&'static str> {
    let mut enabled = Vec::new();
    
    if ENABLE_GMGN_ROUTER {
        enabled.push("GMGN");
    }
    if ENABLE_JUPITER_ROUTER {
        enabled.push("Jupiter");
    }
    if ENABLE_RAYDIUM_ROUTER {
        enabled.push("Raydium");
    }
    
    enabled
}

/// Get router configuration as formatted string for debugging
pub fn get_config_summary() -> String {
    format!(
        "GMGN: {}, Jupiter: {}, Raydium: {}",
        if ENABLE_GMGN_ROUTER { "ON" } else { "OFF" },
        if ENABLE_JUPITER_ROUTER { "ON" } else { "OFF" },
        if ENABLE_RAYDIUM_ROUTER { "ON" } else { "OFF" }
    )
}

// =============================================================================
// ROUTER-SPECIFIC CONFIGURATION HELPERS
// =============================================================================

/// Validate router availability before attempting to use
pub fn validate_router_availability(router_name: &str) -> Result<(), String> {
    match router_name.to_lowercase().as_str() {
        "gmgn" => {
            if !ENABLE_GMGN_ROUTER {
                return Err("GMGN router is disabled in configuration".to_string());
            }
        }
        "jupiter" => {
            if !ENABLE_JUPITER_ROUTER {
                return Err("Jupiter router is disabled in configuration".to_string());
            }
        }
        "raydium" => {
            if !ENABLE_RAYDIUM_ROUTER {
                return Err("Raydium router is disabled in configuration (deprecated API)".to_string());
            }
        }
        _ => {
            return Err(format!("Unknown router: {}", router_name));
        }
    }
    
    Ok(())
}

/// Check if any routers are available for trading
pub fn has_available_routers() -> bool {
    ENABLE_GMGN_ROUTER || ENABLE_JUPITER_ROUTER || ENABLE_RAYDIUM_ROUTER
}

/// Get preferred router order (for fallback logic)
/// Returns routers in order of preference (most reliable first)
pub fn get_preferred_router_order() -> Vec<&'static str> {
    let mut routers = Vec::new();
    
    // Order of preference: Jupiter (most reliable) -> GMGN -> Raydium (deprecated)
    if ENABLE_JUPITER_ROUTER {
        routers.push("Jupiter");
    }
    if ENABLE_GMGN_ROUTER {
        routers.push("GMGN");
    }
    if ENABLE_RAYDIUM_ROUTER {
        routers.push("Raydium");
    }
    
    routers
}
