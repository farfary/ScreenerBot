// Additional wallet management utilities

use crate::core::{ BotResult, BotError };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

/// Wallet utilities and helper functions
pub struct WalletUtils;

impl WalletUtils {
    /// Validate a Solana address
    pub fn validate_address(address: &str) -> BotResult<Pubkey> {
        Pubkey::from_str(address).map_err(|e| BotError::Wallet(format!("Invalid address: {}", e)))
    }

    /// Convert lamports to SOL
    pub fn lamports_to_sol(lamports: u64) -> f64 {
        (lamports as f64) / (crate::core::LAMPORTS_PER_SOL as f64)
    }

    /// Convert SOL to lamports
    pub fn sol_to_lamports(sol: f64) -> u64 {
        (sol * (crate::core::LAMPORTS_PER_SOL as f64)) as u64
    }

    /// Format a token amount with proper decimals
    pub fn format_token_amount(amount: u64, decimals: u8) -> f64 {
        (amount as f64) / (10_f64).powi(decimals as i32)
    }

    /// Parse token amount from UI amount
    pub fn parse_token_amount(ui_amount: f64, decimals: u8) -> u64 {
        (ui_amount * (10_f64).powi(decimals as i32)) as u64
    }

    /// Check if an address is a known program
    pub fn is_known_program(address: &Pubkey) -> bool {
        let known_programs = [
            spl_token::id(),
            spl_associated_token_account::id(),
            solana_sdk::system_program::id(),
            // Add more known programs as needed
        ];

        known_programs.contains(address)
    }

    /// Get short address format for display
    pub fn short_address(address: &Pubkey) -> String {
        let addr_str = address.to_string();
        if addr_str.len() > 8 {
            format!("{}...{}", &addr_str[0..4], &addr_str[addr_str.len() - 4..])
        } else {
            addr_str
        }
    }
}
