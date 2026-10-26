// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Types for ATA cleanup operations.

// =============================================================================
// ATA INFO
// =============================================================================

/// Information about an Associated Token Account
#[derive(Debug, Clone)]
pub struct AtaInfo {
    /// ATA public key address
    pub ata_address: String,
    /// Token mint address
    pub mint: String,
    /// Current token balance (raw)
    pub balance: u64,
    /// Token decimals
    pub decimals: u8,
    /// Whether this uses Token-2022 program
    pub is_token_2022: bool,
    /// Whether this is an NFT
    pub is_nft: bool,
    /// Lamports the account holds, returned to the wallet when it is closed
    pub lamports: u64,
}

impl AtaInfo {
    /// Check if this ATA is empty (zero balance)
    pub fn is_empty(&self) -> bool {
        self.balance == 0
    }
}

impl From<&crate::chains::solana::rpc::TokenAccountInfo> for AtaInfo {
    fn from(info: &crate::chains::solana::rpc::TokenAccountInfo) -> Self {
        Self {
            ata_address: info.account.clone(),
            mint: info.mint.clone(),
            balance: info.balance,
            decimals: info.decimals,
            is_token_2022: info.is_token_2022,
            is_nft: info.is_nft,
            lamports: info.lamports,
        }
    }
}

// =============================================================================
// CLEANUP RESULT
// =============================================================================

/// Result of a single ATA cleanup cycle
#[derive(Debug, Clone)]
pub struct AtaCleanupResult {
    /// Number of ATAs successfully closed
    pub closed_count: u32,
    /// Number of ATAs that failed to close
    pub failed_count: u32,
    /// Total SOL rent reclaimed
    pub rent_reclaimed: f64,
    /// Transaction signatures for successful closures
    pub signatures: Vec<String>,
}

impl Default for AtaCleanupResult {
    fn default() -> Self {
        Self {
            closed_count: 0,
            failed_count: 0,
            rent_reclaimed: 0.0,
            signatures: Vec::new(),
        }
    }
}
