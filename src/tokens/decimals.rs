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

    // Use the batch function for single token (more efficient than separate implementation)
    let results = batch_fetch_token_decimals(&[mint.to_string()]).await;

    if let Some((_, result)) = results.first() {
        result.clone()
    } else {
        Err("No results returned from batch fetch".to_string())
    }
}

/// Batch fetch token decimals from a specific RPC endpoint using get_multiple_accounts
async fn batch_fetch_decimals_from_rpc(
    rpc_url: &str,
    mint_pubkeys: &[Pubkey]
) -> Result<Vec<(Pubkey, Result<u8, String>)>, String> {
    let client = RpcClient::new(rpc_url);

    // Split into chunks of 100 (Solana RPC limit)
    const MAX_ACCOUNTS_PER_CALL: usize = 100;
    let mut all_results = Vec::new();

    for chunk in mint_pubkeys.chunks(MAX_ACCOUNTS_PER_CALL) {
        // Get multiple accounts in one RPC call
        let accounts = client
            .get_multiple_accounts(chunk)
            .map_err(|e| format!("Failed to get multiple accounts: {}", e))?;

        // Process each account result
        for (i, account_option) in accounts.iter().enumerate() {
            let mint_pubkey = chunk[i];

            let decimals_result = match account_option {
                Some(account) => {
                    // Check if account exists and has data
                    if account.data.is_empty() {
                        Err("Account not found or empty".to_string())
                    } else if
                        account.owner != spl_token::id() &&
                        account.owner != spl_token_2022::id()
                    {
                        Err(format!("Account owner is not SPL Token program: {}", account.owner))
                    } else {
                        // Parse mint data based on program type
                        if account.owner == spl_token::id() {
                            // Standard SPL Token
                            parse_spl_token_mint(&account.data)
                        } else {
                            // SPL Token-2022 (Token Extensions)
                            parse_token_2022_mint(&account.data)
                        }
                    }
                }
                None => Err("Account not found".to_string()),
            };

            all_results.push((mint_pubkey, decimals_result));
        }

        // Small delay between batches to avoid rate limiting
        if mint_pubkeys.len() > MAX_ACCOUNTS_PER_CALL {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    Ok(all_results)
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

/// Batch fetch decimals for multiple tokens using efficient batch RPC calls
pub async fn batch_fetch_token_decimals(mints: &[String]) -> Vec<(String, Result<u8, String>)> {
    if mints.is_empty() {
        return Vec::new();
    }

    // Convert mint strings to Pubkeys, filtering out invalid ones
    let mut valid_mints = Vec::new();
    let mut invalid_results = Vec::new();

    for mint in mints {
        match Pubkey::from_str(mint) {
            Ok(pubkey) => valid_mints.push((mint.clone(), pubkey)),
            Err(e) =>
                invalid_results.push((mint.clone(), Err(format!("Invalid mint address: {}", e)))),
        }
    }

    if valid_mints.is_empty() {
        return invalid_results;
    }

    log(
        LogTag::System,
        "DECIMALS",
        &format!("Batch fetching decimals for {} tokens", valid_mints.len())
    );

    // Load RPC configuration
    let config = match read_configs("configs.json") {
        Ok(config) => config,
        Err(e) => {
            let error_msg = format!("Failed to load config: {}", e);
            return valid_mints
                .into_iter()
                .map(|(mint, _)| (mint, Err(error_msg.clone())))
                .chain(invalid_results)
                .collect();
        }
    };

    // Prepare RPC URLs (main + fallbacks)
    let mut rpc_urls = vec![config.rpc_url.clone()];
    rpc_urls.extend(config.rpc_fallbacks.clone());

    let _pubkeys: Vec<Pubkey> = valid_mints
        .iter()
        .map(|(_, pubkey)| *pubkey)
        .collect();
    let mut results = Vec::new();
    let mut remaining_mints = valid_mints.clone();

    // Try each RPC endpoint until we get results
    for rpc_url in &rpc_urls {
        if remaining_mints.is_empty() {
            break;
        }

        let remaining_pubkeys: Vec<Pubkey> = remaining_mints
            .iter()
            .map(|(_, pubkey)| *pubkey)
            .collect();

        match batch_fetch_decimals_from_rpc(rpc_url, &remaining_pubkeys).await {
            Ok(batch_results) => {
                let mut successful_indices = Vec::new();

                for (i, (_pubkey, decimals_result)) in batch_results.iter().enumerate() {
                    let mint_str = &remaining_mints[i].0;

                    match decimals_result {
                        Ok(decimals) => {
                            // Cache the successful result
                            if let Ok(mut cache) = DECIMAL_CACHE.lock() {
                                cache.insert(mint_str.clone(), *decimals);
                            }

                            results.push((mint_str.clone(), Ok(*decimals)));
                            successful_indices.push(i);
                        }
                        Err(e) => {
                            log(
                                LogTag::System,
                                "WARN",
                                &format!(
                                    "Failed to get decimals for {} from {}: {}",
                                    mint_str,
                                    rpc_url,
                                    e
                                )
                            );
                        }
                    }
                }

                // Remove successful mints from remaining list (in reverse order to maintain indices)
                for &index in successful_indices.iter().rev() {
                    remaining_mints.remove(index);
                }

                log(
                    LogTag::System,
                    "SUCCESS",
                    &format!(
                        "Successfully fetched {} decimals from {}",
                        successful_indices.len(),
                        rpc_url
                    )
                );
            }
            Err(e) => {
                log(LogTag::System, "WARN", &format!("Batch fetch failed from {}: {}", rpc_url, e));
            }
        }
    }

    // For any remaining failed mints, add default decimals and cache them
    for (mint_str, _) in remaining_mints {
        log(
            LogTag::System,
            "ERROR",
            &format!("All RPC endpoints failed for token {}, using default decimals (9)", mint_str)
        );

        // Cache the default value to avoid repeated failures
        if let Ok(mut cache) = DECIMAL_CACHE.lock() {
            cache.insert(mint_str.clone(), 9);
        }

        results.push((mint_str, Ok(9))); // Default fallback
    }

    // Add back the invalid mint results
    results.extend(invalid_results);

    results
}

/// Get decimals from cache only (no RPC call)
pub fn get_cached_decimals(mint: &str) -> Option<u8> {
    DECIMAL_CACHE.lock().ok()?.get(mint).copied()
}

/// Batch get token decimals from blockchain with caching - efficient for multiple tokens
pub async fn get_multiple_token_decimals_from_chain(
    mints: &[String]
) -> Vec<(String, Result<u8, String>)> {
    if mints.is_empty() {
        return Vec::new();
    }

    // Check cache for all mints first
    let mut cached_results = Vec::new();
    let mut uncached_mints = Vec::new();

    if let Ok(cache) = DECIMAL_CACHE.lock() {
        for mint in mints {
            if let Some(&decimals) = cache.get(mint) {
                cached_results.push((mint.clone(), Ok(decimals)));
            } else {
                uncached_mints.push(mint.clone());
            }
        }
    } else {
        uncached_mints = mints.to_vec();
    }

    // If some mints are not cached, fetch them in batch
    let mut batch_results = Vec::new();
    if !uncached_mints.is_empty() {
        log(
            LogTag::System,
            "DECIMALS",
            &format!("Batch fetching decimals for {} uncached tokens", uncached_mints.len())
        );
        batch_results = batch_fetch_token_decimals(&uncached_mints).await;
    }

    // Combine cached and fetched results in original order
    let mut all_results = Vec::new();
    let _batch_index = 0;

    for mint in mints {
        // Check if this mint was cached
        if let Some(cached_result) = cached_results.iter().find(|(m, _)| m == mint) {
            all_results.push(cached_result.clone());
        } else {
            // Find in batch results
            if let Some(batch_result) = batch_results.iter().find(|(m, _)| m == mint) {
                all_results.push(batch_result.clone());
            } else {
                // This shouldn't happen, but handle gracefully
                all_results.push((mint.clone(), Err("Failed to fetch decimals".to_string())));
            }
        }
    }

    all_results
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
