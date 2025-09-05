//! Legacy compatibility shim for old `tokens::pool_old` module.
//! This maps commonly used items to the new implementations so existing bins still compile.

use crate::pool_calculator as pc;

pub use pc::{
    METEORA_DAMM_V2_PROGRAM_ID,
    METEORA_DLMM_PROGRAM_ID,
    RAYDIUM_CPMM_PROGRAM_ID,
    RAYDIUM_CLMM_PROGRAM_ID,
    RAYDIUM_LEGACY_AMM_PROGRAM_ID,
};

/// Backward-compatible PoolInfo type alias to the new struct
pub type PoolInfo = pc::PoolInfo;

/// Backward-compatible price calculator with a subset of old API surface.
pub struct PoolPriceCalculator;

impl PoolPriceCalculator {
    pub fn new() -> Self {
        Self
    }

    /// Fetch raw pool account data (not implemented in new calculator). Kept for compatibility.
    pub async fn get_raw_pool_data(
        &mut self,
        _pool_address: &str
    ) -> Result<Option<Vec<u8>>, String> {
        // Not yet implemented in new architecture. Return None to keep callers functional.
        Ok(None)
    }

    /// Decode pool info for a given address (compat shim). Not implemented; returns None.
    pub async fn get_pool_info(&self, _pool_address: &str) -> Result<Option<PoolInfo>, String> {
        Ok(None)
    }
}

/// Return a human-friendly name for a program id
pub fn get_pool_program_display_name(program_id: &str) -> &'static str {
    match program_id {
        id if id == RAYDIUM_CPMM_PROGRAM_ID => "RAYDIUM CPMM",
        id if id == RAYDIUM_CLMM_PROGRAM_ID => "RAYDIUM CLMM",
        id if id == RAYDIUM_LEGACY_AMM_PROGRAM_ID => "RAYDIUM LEGACY AMM",
        id if id == METEORA_DAMM_V2_PROGRAM_ID => "METEORA DAMM v2",
        id if id == METEORA_DLMM_PROGRAM_ID => "METEORA DLMM",
        _ => "UNKNOWN",
    }
}
