/// Utility functions for swap operations
/// 
/// This module contains helper functions for:
/// - Token amount conversions
/// - Swap request creation helpers

use crate::swap::types::*;

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

/// Create a swap request with common defaults
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

/// Create a simple SOL to USDC swap request
pub fn create_sol_to_usdc_request(
    sol_amount: f64,
    user_public_key: &str,
    slippage_bps: Option<u32>,
) -> SwapRequest {
    create_swap_request(
        SOL_MINT,
        USDC_MINT,
        sol_to_lamports(sol_amount),
        slippage_bps.unwrap_or(50), // 0.5% default slippage
        user_public_key,
        None,
        false,
    )
}

/// Create a simple USDC to SOL swap request
pub fn create_usdc_to_sol_request(
    usdc_amount: f64,
    user_public_key: &str,
    slippage_bps: Option<u32>,
) -> SwapRequest {
    create_swap_request(
        USDC_MINT,
        SOL_MINT,
        usdc_to_micro_usdc(usdc_amount),
        slippage_bps.unwrap_or(50), // 0.5% default slippage
        user_public_key,
        None,
        false,
    )
}

/// Format amount for display with appropriate decimals
pub fn format_amount(amount: u64, mint: &str) -> String {
    let decimals = get_token_decimals(mint);
    let token_amount = raw_amount_to_token(amount, decimals);
    
    match mint {
        SOL_MINT => format!("{:.4} SOL", token_amount),
        USDC_MINT => format!("{:.2} USDC", token_amount),
        USDT_MINT => format!("{:.2} USDT", token_amount),
        _ => format!("{:.6} {}", token_amount, &mint[..8]),
    }
}

/// Calculate slippage percentage from amounts
pub fn calculate_slippage(expected_amount: u64, actual_amount: u64) -> f64 {
    if expected_amount == 0 {
        return 0.0;
    }
    
    let diff = if actual_amount > expected_amount {
        actual_amount - expected_amount
    } else {
        expected_amount - actual_amount
    };
    
    (diff as f64 / expected_amount as f64) * 100.0
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn test_helper_functions() {
        let request = create_sol_to_usdc_request(0.01, "test_key", None);
        assert_eq!(request.amount, sol_to_lamports(0.01));
        assert_eq!(request.input_mint, SOL_MINT);
        assert_eq!(request.output_mint, USDC_MINT);

        let request = create_usdc_to_sol_request(10.0, "test_key", Some(100));
        assert_eq!(request.amount, usdc_to_micro_usdc(10.0));
        assert_eq!(request.slippage_bps, 100);
    }

    #[test]
    fn test_format_amount() {
        assert_eq!(format_amount(sol_to_lamports(1.5), SOL_MINT), "1.5000 SOL");
        assert_eq!(format_amount(usdc_to_micro_usdc(100.0), USDC_MINT), "100.00 USDC");
    }

    #[test]
    fn test_calculate_slippage() {
        assert_eq!(calculate_slippage(1000, 950), 5.0); // 5% slippage
        assert_eq!(calculate_slippage(1000, 1050), 5.0); // 5% positive slippage
        assert_eq!(calculate_slippage(0, 100), 0.0); // Edge case
    }
}
