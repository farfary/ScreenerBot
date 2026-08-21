//! Program-specific swap implementations
//!
//! This module contains the actual swap logic for different DEX programs.
//! Each program has its own module with a standardized interface.

use crate::chains::solana::pools::fetcher::AccountData;
use crate::chains::solana::swaps::types::{SwapError, SwapRequest, SwapResult};

// Program implementations
pub mod raydium_clmm;
pub mod raydium_cpmm;

/// Common trait for all program swap implementations
pub trait ProgramSwap {
    /// Execute a swap for this specific program
    async fn execute_swap(
        request: SwapRequest,
        pool_data: AccountData,
    ) -> Result<SwapResult, SwapError>;
}
