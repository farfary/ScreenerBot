//! RPC utility functions
//!
//! Common utilities for RPC operations.

use crate::constants::LAMPORTS_PER_SOL;
use crate::errors::ScreenerBotError;
use crate::logger::{self, LogTag};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Parse a pubkey from string safely
///
/// Wrapper around `Pubkey::from_str` with better error messages.
pub fn parse_pubkey_string(s: &str) -> Result<Pubkey, String> {
    Pubkey::from_str(s).map_err(|e| format!("Invalid pubkey '{}': {}", s, e))
}

/// Convert SOL amount to lamports
///
/// # Example
/// ```ignore
/// let lamports = sol_to_lamports(1.5);
/// assert_eq!(lamports, 1_500_000_000);
/// ```
pub fn sol_to_lamports(sol_amount: f64) -> u64 {
    (sol_amount * LAMPORTS_PER_SOL as f64) as u64
}

/// Get minimum rent for ATA from chain with caching
///
/// Uses a 10-second cache to avoid excessive RPC calls.
/// Falls back to default ATA rent (2039280 lamports) on errors.
pub async fn get_ata_rent_from_chain() -> Result<u64, String> {
    use crate::rpc::compat::get_new_rpc_client;
    use crate::rpc::RpcClientMethods;

    // Use the new client
    let client = get_new_rpc_client();

    // ATA data size is 165 bytes
    client.get_minimum_balance_for_rent_exemption(165).await
}

/// Cached ATA rent information
#[derive(Debug, Clone)]
pub struct AtaRentInfo {
    pub rent_lamports: u64,
    pub cached_at: Instant,
}

/// Global cache for ATA rent amounts (10-second cache)
static ATA_RENT_CACHE: once_cell::sync::Lazy<
    Arc<std::sync::Mutex<Option<AtaRentInfo>>>,
> = once_cell::sync::Lazy::new(|| Arc::new(std::sync::Mutex::new(None)));

/// Default ATA rent in lamports (0.00203928 SOL)
pub const DEFAULT_ATA_RENT_LAMPORTS: u64 = 2_039_280;

/// Get ATA rent with caching (10-second TTL)
///
/// Attempts to fetch from chain, uses cache, falls back to default.
pub async fn get_ata_rent_lamports() -> Result<u64, ScreenerBotError> {
    // Check cache first
    {
        let cache = match ATA_RENT_CACHE.try_lock() {
            Ok(cache) => cache,
            Err(_) => {
                logger::debug(
                    LogTag::Rpc,
                    "ATA rent cache lock contention - using default ATA rent",
                );
                return Ok(DEFAULT_ATA_RENT_LAMPORTS);
            }
        };
        if let Some(ref info) = *cache {
            if info.cached_at.elapsed() < Duration::from_secs(10) {
                return Ok(info.rent_lamports);
            }
        }
    }

    // Fetch from chain
    match get_ata_rent_from_chain().await {
        Ok(rent) => {
            // Update cache
            if let Ok(mut cache) = ATA_RENT_CACHE.try_lock() {
                *cache = Some(AtaRentInfo {
                    rent_lamports: rent,
                    cached_at: Instant::now(),
                });
            }
            Ok(rent)
        }
        Err(e) => {
            logger::warning(
                LogTag::Rpc,
                &format!(
                    "Failed to get ATA rent from chain: {} - using default",
                    e
                ),
            );
            Ok(DEFAULT_ATA_RENT_LAMPORTS)
        }
    }
}
