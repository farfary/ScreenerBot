/// Program IDs for transaction analysis and router detection
/// 
/// This module contains all program IDs needed for analyzing transactions
/// and detecting which platform/router was used for swaps. This is separate
/// from the pools module to maintain clean separation of concerns.

// =============================================================================
// DEX AGGREGATOR PROGRAM IDS
// =============================================================================

/// Jupiter - Main DEX aggregator program
pub const JUPITER_V6_PROGRAM_ID: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
pub const JUPITER_V4_PROGRAM_ID: &str = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB";
pub const JUPITER_V3_PROGRAM_ID: &str = "JUP3c2Uh3WA4Ng34tw6kPd2G4C5BB21Xo36Je1s32Ph";

/// GMGN - Gaming and social trading platform
pub const GMGN_PROGRAM_ID: &str = "GMGNjvGr7ddxt2u1XSf8Zo6LLnDjDm9mJahGfhq7j6gk";

// =============================================================================
// DIRECT DEX PROGRAM IDS
// =============================================================================

/// Raydium DEX variants
pub const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";
pub const RAYDIUM_LEGACY_AMM_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
pub const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";

/// Orca DEX
pub const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
pub const ORCA_V1_PROGRAM_ID: &str = "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP";

/// Meteora DEX variants
pub const METEORA_DAMM_PROGRAM_ID: &str = "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG";
pub const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
pub const METEORA_DBC_PROGRAM_ID: &str = "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN";

/// PumpFun variants
pub const PUMP_FUN_AMM_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";
pub const PUMP_FUN_LEGACY_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// Other DEXes
pub const MOONSHOT_PROGRAM_ID: &str = "MoonCVVNZFSYkqNXP6bxHLPL6QQJiMagDL3qcqUQTrG";
pub const FLUXBEAM_AMM_PROGRAM_ID: &str = "FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X";

// =============================================================================
// ROUTER DETECTION FUNCTIONS
// =============================================================================

/// Detect router/platform from program ID
pub fn detect_router_from_program_id(program_id: &str) -> Option<&'static str> {
    match program_id {
        // Jupiter variants
        JUPITER_V6_PROGRAM_ID | JUPITER_V4_PROGRAM_ID | JUPITER_V3_PROGRAM_ID => Some("jupiter"),
        
        // GMGN
        GMGN_PROGRAM_ID => Some("gmgn"),
        
        // Raydium variants
        RAYDIUM_CPMM_PROGRAM_ID | RAYDIUM_LEGACY_AMM_PROGRAM_ID | RAYDIUM_CLMM_PROGRAM_ID => {
            Some("raydium")
        }
        
        // Orca variants
        ORCA_WHIRLPOOL_PROGRAM_ID | ORCA_V1_PROGRAM_ID => Some("orca"),
        
        // Meteora variants
        METEORA_DAMM_PROGRAM_ID | METEORA_DLMM_PROGRAM_ID | METEORA_DBC_PROGRAM_ID => {
            Some("meteora")
        }
        
        // PumpFun variants
        PUMP_FUN_AMM_PROGRAM_ID | PUMP_FUN_LEGACY_PROGRAM_ID => Some("pumpfun"),
        
        // Other DEXes
        MOONSHOT_PROGRAM_ID => Some("moonshot"),
        FLUXBEAM_AMM_PROGRAM_ID => Some("fluxbeam"),
        
        _ => None,
    }
}

/// Detect router from log messages (fallback when program ID detection fails)
pub fn detect_router_from_logs(log_messages: &[String]) -> Option<&'static str> {
    for log_line in log_messages {
        let log_lower = log_line.to_lowercase();
        
        if log_lower.contains("jupiter") {
            return Some("jupiter");
        }
        if log_lower.contains("gmgn") {
            return Some("gmgn");
        }
        if log_lower.contains("raydium") {
            return Some("raydium");
        }
        if log_lower.contains("orca") {
            return Some("orca");
        }
        if log_lower.contains("meteora") {
            return Some("meteora");
        }
        if log_lower.contains("pumpfun") || log_lower.contains("pump.fun") {
            return Some("pumpfun");
        }
        if log_lower.contains("moonshot") {
            return Some("moonshot");
        }
        if log_lower.contains("fluxbeam") {
            return Some("fluxbeam");
        }
    }
    
    None
}

/// Get all known Jupiter program IDs for validation
pub fn get_jupiter_program_ids() -> &'static [&'static str] {
    &[JUPITER_V6_PROGRAM_ID, JUPITER_V4_PROGRAM_ID, JUPITER_V3_PROGRAM_ID]
}

/// Check if a program ID belongs to Jupiter
pub fn is_jupiter_program_id(program_id: &str) -> bool {
    matches!(
        program_id,
        JUPITER_V6_PROGRAM_ID | JUPITER_V4_PROGRAM_ID | JUPITER_V3_PROGRAM_ID
    )
}

/// Check if a program ID belongs to any known DEX aggregator
pub fn is_dex_aggregator_program_id(program_id: &str) -> bool {
    matches!(
        program_id,
        JUPITER_V6_PROGRAM_ID | JUPITER_V4_PROGRAM_ID | JUPITER_V3_PROGRAM_ID | GMGN_PROGRAM_ID
    )
}