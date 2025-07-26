/// Token decimals fetching from Solana blockchain
use crate::logger::{ log, LogTag };
use crate::global::read_configs;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_program::program_pack::Pack;
use spl_token::state::Mint;
use std::str::FromStr;
use std::collections::HashMap;
use std::sync::{ Arc, Mutex };
use once_cell::sync::Lazy;

// Cache for token decimals to avoid repeated RPC calls
static DECIMAL_CACHE: Lazy<Arc<Mutex<HashMap<String, u8>>>> = Lazy::new(|| {
    Arc::new(Mutex::new(HashMap::new()))
});

/// Get token decimals from Solana blockchain with caching
pub async fn get_token_decimals_from_chain(mint: &str) -> Result<u8, String> {
    // Check cache first
    if let Ok(cache) = DECIMAL_CACHE.lock() {
        if let Some(&decimals) = cache.get(mint) {
            return Ok(decimals);
        }
    }

    log(LogTag::System, "DECIMALS", &format!("Fetching decimals for token: {}", mint));

    // Parse mint address
    let mint_pubkey = Pubkey::from_str(mint).map_err(|e|
        format!("Invalid mint address {}: {}", mint, e)
    )?;

    // Load RPC configuration
    let config = read_configs("configs.json").map_err(|e| format!("Failed to load config: {}", e))?;

    // Try main RPC first, then fallbacks
    let mut rpc_urls = vec![config.rpc_url.clone()];
    rpc_urls.extend(config.rpc_fallbacks.clone());

    let mut last_error = String::new();

    for rpc_url in &rpc_urls {
        match fetch_decimals_from_rpc(rpc_url, &mint_pubkey).await {
            Ok(decimals) => {
                // Cache the result
                if let Ok(mut cache) = DECIMAL_CACHE.lock() {
                    cache.insert(mint.to_string(), decimals);
                }

                log(
                    LogTag::System,
                    "SUCCESS",
                    &format!("Fetched decimals for {}: {}", mint, decimals)
                );

                return Ok(decimals);
            }
            Err(e) => {
                last_error = e;
                log(
                    LogTag::System,
                    "WARN",
                    &format!("Failed to fetch decimals from {}: {}", rpc_url, last_error)
                );
            }
        }
    }

    // If all RPCs fail, return default but log the error
    log(
        LogTag::System,
        "ERROR",
        &format!(
            "All RPC endpoints failed for token {}, using default decimals (9): {}",
            mint,
            last_error
        )
    );

    // Cache the default value to avoid repeated failures
    if let Ok(mut cache) = DECIMAL_CACHE.lock() {
        cache.insert(mint.to_string(), 9);
    }

    Ok(9) // Default fallback
}

/// Fetch token decimals from a specific RPC endpoint
async fn fetch_decimals_from_rpc(rpc_url: &str, mint_pubkey: &Pubkey) -> Result<u8, String> {
    let client = RpcClient::new(rpc_url);

    // Get account data for the mint
    let account = client
        .get_account(mint_pubkey)
        .map_err(|e| format!("Failed to get account data: {}", e))?;

    // Check if account exists
    if account.data.is_empty() {
        return Err("Account not found or empty".to_string());
    }

    // Check account owner (should be SPL Token program)
    if account.owner != spl_token::id() && account.owner != spl_token_2022::id() {
        return Err(format!("Account owner is not SPL Token program: {}", account.owner));
    }

    // Parse mint data based on program type
    let decimals = if account.owner == spl_token::id() {
        // Standard SPL Token
        parse_spl_token_mint(&account.data)?
    } else {
        // SPL Token-2022 (Token Extensions)
        parse_token_2022_mint(&account.data)?
    };

    Ok(decimals)
}

/// Parse SPL Token mint data to extract decimals
fn parse_spl_token_mint(data: &[u8]) -> Result<u8, String> {
    if data.len() < Mint::LEN {
        return Err(format!("Invalid mint data length: expected {}, got {}", Mint::LEN, data.len()));
    }

    // Parse using SPL Token library
    let mint = Mint::unpack(data).map_err(|e| format!("Failed to unpack SPL Token mint: {}", e))?;

    Ok(mint.decimals)
}

/// Parse SPL Token-2022 mint data to extract decimals
fn parse_token_2022_mint(data: &[u8]) -> Result<u8, String> {
    // For Token-2022, the decimals are at the same position as in standard SPL Token
    // The first 9 bytes are the same structure for both token programs
    if data.len() < 44 {
        return Err(
            format!("Invalid Token-2022 mint data length: expected at least 44, got {}", data.len())
        );
    }

    // Decimals are at offset 44 in both SPL Token and SPL Token-2022
    Ok(data[44])
}

/// Batch fetch decimals for multiple tokens
pub async fn batch_fetch_token_decimals(mints: &[String]) -> Vec<(String, Result<u8, String>)> {
    let mut results = Vec::new();

    for mint in mints {
        let result = get_token_decimals_from_chain(mint).await;
        results.push((mint.clone(), result));

        // Small delay to avoid rate limiting
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    results
}

/// Get decimals from cache only (no RPC call)
pub fn get_cached_decimals(mint: &str) -> Option<u8> {
    DECIMAL_CACHE.lock().ok()?.get(mint).copied()
}

/// Clear decimals cache
pub fn clear_decimals_cache() {
    if let Ok(mut cache) = DECIMAL_CACHE.lock() {
        cache.clear();
    }
}

/// Get cache statistics
pub fn get_cache_stats() -> (usize, usize) {
    if let Ok(cache) = DECIMAL_CACHE.lock() {
        let size = cache.len();
        let capacity = cache.capacity();
        (size, capacity)
    } else {
        (0, 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_token_decimals() {
        // Test with a known token (USDC)
        let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
        let result = get_token_decimals_from_chain(usdc_mint).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_cache_operations() {
        clear_decimals_cache();

        // Test cache is empty
        assert_eq!(get_cached_decimals("test"), None);

        // Cache should be empty
        let (size, _) = get_cache_stats();
        assert_eq!(size, 0);
    }
}
