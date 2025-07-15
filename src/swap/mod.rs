pub mod types;
pub mod jupiter;
pub mod raydium;
pub mod gmgn;
pub mod routes;
pub mod manager;
pub mod tests;

pub use manager::SwapManager;
pub use types::*;

use crate::config::Config;
use crate::rpc_manager::RpcManager;
use crate::trading::transaction_manager::TransactionManager;
use anyhow::Result;
use std::sync::Arc;

/// Create a new SwapManager instance from the main config
pub fn create_swap_manager(
    config: &Config,
    rpc_manager: Arc<RpcManager>,
    transaction_manager: Arc<TransactionManager>,
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

/// Utility function to create a swap request
pub fn create_swap_request(
    input_mint: &str,
    output_mint: &str,
    amount: u64,
    slippage_bps: u32,
    user_public_key: &str,
    dex_preference: Option<DexType>,
    is_anti_mev: bool,
) -> SwapRequest {
    SwapRequest {
        input_mint: input_mint.to_string(),
        output_mint: output_mint.to_string(),
        amount,
        slippage_bps,
        user_public_key: user_public_key.to_string(),
        dex_preference,
        is_anti_mev,
    }
}

/// Convert SOL amount to lamports
pub fn sol_to_lamports(sol_amount: f64) -> u64 {
    (sol_amount * 1_000_000_000.0) as u64
}

/// Convert lamports to SOL
pub fn lamports_to_sol(lamports: u64) -> f64 {
    lamports as f64 / 1_000_000_000.0
}

/// Convert USDC amount to micro-USDC (6 decimals)
pub fn usdc_to_micro_usdc(usdc_amount: f64) -> u64 {
    (usdc_amount * 1_000_000.0) as u64
}

/// Convert micro-USDC to USDC
pub fn micro_usdc_to_usdc(micro_usdc: u64) -> f64 {
    micro_usdc as f64 / 1_000_000.0
}

/// Get token decimals for common tokens
pub fn get_token_decimals(mint: &str) -> u8 {
    match mint {
        SOL_MINT => 9,
        USDC_MINT => 6,
        USDT_MINT => 6,
        _ => 9, // Default to 9 decimals
    }
}

/// Convert token amount to raw amount based on decimals
pub fn token_to_raw_amount(token_amount: f64, decimals: u8) -> u64 {
    (token_amount * 10_f64.powi(decimals as i32)) as u64
}

/// Convert raw amount to token amount based on decimals
pub fn raw_amount_to_token(raw_amount: u64, decimals: u8) -> f64 {
    raw_amount as f64 / 10_f64.powi(decimals as i32)
}

#[cfg(test)]
mod test_utils {
    use super::*;

    #[test]
    fn test_sol_conversions() {
        let sol_amount = 0.001;
        let lamports = sol_to_lamports(sol_amount);
        assert_eq!(lamports, 1_000_000);
        
        let converted_back = lamports_to_sol(lamports);
        assert!((converted_back - sol_amount).abs() < f64::EPSILON);
    }

    #[test]
    fn test_usdc_conversions() {
        let usdc_amount = 100.0;
        let micro_usdc = usdc_to_micro_usdc(usdc_amount);
        assert_eq!(micro_usdc, 100_000_000);
        
        let converted_back = micro_usdc_to_usdc(micro_usdc);
        assert!((converted_back - usdc_amount).abs() < f64::EPSILON);
    }

    #[test]
    fn test_token_decimals() {
        assert_eq!(get_token_decimals(SOL_MINT), 9);
        assert_eq!(get_token_decimals(USDC_MINT), 6);
        assert_eq!(get_token_decimals(USDT_MINT), 6);
        assert_eq!(get_token_decimals("unknown"), 9);
    }

    #[test]
    fn test_raw_amount_conversions() {
        let token_amount = 1.5;
        let decimals = 6;
        let raw_amount = token_to_raw_amount(token_amount, decimals);
        assert_eq!(raw_amount, 1_500_000);
        
        let converted_back = raw_amount_to_token(raw_amount, decimals);
        assert!((converted_back - token_amount).abs() < f64::EPSILON);
    }

    #[test]
    fn test_create_swap_request() {
        let request = create_swap_request(
            SOL_MINT,
            USDC_MINT,
            1_000_000,
            50,
            "11111111111111111111111111111111",
            Some(DexType::Jupiter),
            false,
        );

        assert_eq!(request.input_mint, SOL_MINT);
        assert_eq!(request.output_mint, USDC_MINT);
        assert_eq!(request.amount, 1_000_000);
        assert_eq!(request.slippage_bps, 50);
        assert_eq!(request.dex_preference, Some(DexType::Jupiter));
        assert!(!request.is_anti_mev);
    }
}
