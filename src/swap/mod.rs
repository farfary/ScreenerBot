// Core modules
pub mod types;
pub mod core;
pub mod dex;
pub mod utils;

// Testing module (conditionally compiled)
#[cfg(test)]
pub mod testing;

// Re-export main components
pub use core::{ SwapManager, RouteSelector };
pub use dex::{ DexInstances, JupiterSwap, RaydiumSwap, GmgnSwap };
pub use types::*;

// Re-export specific utils to avoid conflicts
pub use utils::{
    sol_to_lamports,
    lamports_to_sol,
    usdc_to_micro_usdc,
    micro_usdc_to_usdc,
    get_token_decimals,
    token_to_raw_amount,
    raw_amount_to_token,
    create_swap_request,
    create_sol_to_usdc_request,
    create_usdc_to_sol_request,
    format_amount,
    calculate_slippage,
};

use crate::config::Config;
use crate::rpc_manager::RpcManager;
use crate::trading::transaction_manager::TransactionManager;
use anyhow::Result;
use std::sync::Arc;

/// Create a new SwapManager instance from the main config
pub fn create_swap_manager(
    config: &Config,
    rpc_manager: Arc<RpcManager>,
    transaction_manager: Arc<TransactionManager>
) -> Result<SwapManager> {
    let swap_config = SwapConfig {
        enabled: config.swap.enabled,
        default_dex: config.swap.default_dex.clone(),
        is_anti_mev: config.swap.is_anti_mev,
        max_slippage: config.swap.max_slippage,
        timeout_seconds: config.swap.timeout_seconds,
        retry_attempts: config.swap.retry_attempts,
        dex_preferences: config.swap.dex_preferences.clone(),
        jupiter: config.swap.jupiter.clone(),
        raydium: config.swap.raydium.clone(),
        gmgn: config.swap.gmgn.clone(),
    };

    Ok(SwapManager::new(swap_config, rpc_manager, transaction_manager))
}
