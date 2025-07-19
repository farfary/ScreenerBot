use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use crate::logger::{ log, LogTag };
use std::str::FromStr;

/// Cache for storing token decimals to avoid repeated RPC calls
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecimalCache {
    pub decimals: HashMap<String, u8>,
}

impl DecimalCache {
    /// Create a new empty decimal cache
    pub fn new() -> Self {
        Self {
            decimals: HashMap::new(),
        }
    }

    /// Load cache from disk if it exists
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, Box<dyn Error>> {
        if path.as_ref().exists() {
            let data = fs::read_to_string(path)?;
            let cache: DecimalCache = serde_json::from_str(&data)?;
            Ok(cache)
        } else {
            Ok(Self::new())
        }
    }

    /// Save cache to disk
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), Box<dyn Error>> {
        let data = serde_json::to_string_pretty(self)?;
        fs::write(path, data)?;
        Ok(())
    }

    /// Get decimal count for a mint, returns None if not cached
    pub fn get(&self, mint: &str) -> Option<u8> {
        self.decimals.get(mint).copied()
    }

    /// Insert decimal count for a mint
    pub fn insert(&mut self, mint: String, decimals: u8) {
        self.decimals.insert(mint, decimals);
    }
}

/// Extract decimals from a mint account's data
fn extract_decimals_from_mint_account(account_data: &[u8]) -> Result<u8, Box<dyn Error>> {
    // Solana mint account layout:
    // - mint_authority: 36 bytes (32 bytes pubkey + 4 bytes COption)
    // - supply: 8 bytes
    // - decimals: 1 byte
    // - is_initialized: 1 byte
    // - freeze_authority: 36 bytes (32 bytes pubkey + 4 bytes COption)

    if account_data.len() < 82 {
        return Err("Invalid mint account data length".into());
    }

    // Decimals is at offset 44 (36 + 8)
    let decimals = account_data[44];
    Ok(decimals)
}

/// Fetch decimals for multiple mints using getMultipleAccounts RPC call
/// Updates the cache and saves it to disk
pub async fn fetch_or_cache_decimals(
    rpc_client: &RpcClient,
    mints: &[String],
    cache: &mut DecimalCache,
    cache_path: &Path
) -> Result<HashMap<String, u8>, Box<dyn Error>> {
    let mut result = HashMap::new();
    let mut mints_to_fetch = Vec::new();

    // Check cache first
    for mint in mints {
        if let Some(decimals) = cache.get(mint) {
            result.insert(mint.clone(), decimals);
        } else {
            mints_to_fetch.push(mint.clone());
        }
    }

    if mints_to_fetch.is_empty() {
        return Ok(result);
    }

    log(
        LogTag::Monitor,
        "INFO",
        &format!("Fetching decimals for {} new mints from chain", mints_to_fetch.len())
    );

    // Convert mint strings to Pubkeys
    let mut valid_mints = Vec::new();
    let mut pubkeys = Vec::new();

    for mint_str in &mints_to_fetch {
        match Pubkey::from_str(mint_str) {
            Ok(pubkey) => {
                valid_mints.push(mint_str.clone());
                pubkeys.push(pubkey);
            }
            Err(e) => {
                log(LogTag::Monitor, "WARN", &format!("Invalid mint address {}: {}", mint_str, e));
                // Use default decimals of 9 for invalid addresses
                result.insert(mint_str.clone(), 9);
                cache.insert(mint_str.clone(), 9);
            }
        }
    }

    if pubkeys.is_empty() {
        return Ok(result);
    }

    // Fetch multiple accounts in batches (max 100 per request)
    const BATCH_SIZE: usize = 100;
    let mut processed_valid_mints = 0;

    for chunk in pubkeys.chunks(BATCH_SIZE) {
        let chunk_mints: Vec<String> = valid_mints
            .iter()
            .skip(processed_valid_mints)
            .take(chunk.len())
            .cloned()
            .collect();

        match rpc_client.get_multiple_accounts(chunk) {
            Ok(accounts) => {
                for (i, account_opt) in accounts.iter().enumerate() {
                    let mint_str = &chunk_mints[i];

                    match account_opt {
                        Some(account) => {
                            match extract_decimals_from_mint_account(&account.data) {
                                Ok(decimals) => {
                                    result.insert(mint_str.clone(), decimals);
                                    cache.insert(mint_str.clone(), decimals);
                                }
                                Err(e) => {
                                    log(
                                        LogTag::Monitor,
                                        "WARN",
                                        &format!(
                                            "Failed to parse mint account for {}: {}, using default 9",
                                            mint_str,
                                            e
                                        )
                                    );
                                    result.insert(mint_str.clone(), 9);
                                    cache.insert(mint_str.clone(), 9);
                                }
                            }
                        }
                        None => {
                            log(
                                LogTag::Monitor,
                                "WARN",
                                &format!("Mint account not found for {}, using default 9", mint_str)
                            );
                            result.insert(mint_str.clone(), 9);
                            cache.insert(mint_str.clone(), 9);
                        }
                    }
                }
            }
            Err(e) => {
                log(
                    LogTag::Monitor,
                    "ERROR",
                    &format!("Failed to fetch mint accounts: {}, using default 9 for all", e)
                );
                // Fallback to default decimals for this batch
                for mint_str in &chunk_mints {
                    result.insert(mint_str.clone(), 9);
                    cache.insert(mint_str.clone(), 9);
                }
            }
        }

        // Increment the counter for the next batch
        processed_valid_mints += chunk.len();
    }

    // Save updated cache to disk
    if let Err(e) = cache.save_to_file(cache_path) {
        log(LogTag::Monitor, "WARN", &format!("Failed to save decimal cache: {}", e));
    }

    Ok(result)
}
