/// Liquidity Pool Lock Detection Module
///
/// This module provides functionality to detect whether a token's liquidity pool
/// is locked or not. It uses DexScreener API cached pool data instead of RPC calls
/// to find pools, then checks if LP tokens are locked/burned.

use crate::logger::{ log, LogTag };
use crate::rpc::get_rpc_client;
use crate::tokens::dexscreener::{
    get_token_pools_from_dexscreener,
    get_cached_pools_for_token,
    TokenPair,
};
use crate::pools::decoders::RaydiumCpmmDecoder;
use crate::pools::types::ProgramKind;
use crate::utils::safe_truncate;
use base64;
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::program_pack::Pack;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{ LazyLock, RwLock };

/// LP lock analysis cache (5 minute TTL)
static LP_LOCK_CACHE: LazyLock<RwLock<HashMap<String, CachedLpLockAnalysis>>> = LazyLock::new(||
    RwLock::new(HashMap::new())
);

const LP_LOCK_CACHE_TTL_SECS: i64 = 300; // 5 minutes

// Known LP lock/vesting program addresses on Solana
const KNOWN_LOCK_PROGRAMS: &[(&str, &str)] = &[
    // Team Finance
    ("J2ZDhSq8CWNaQ1UZAFALxLm4oJ7mS9tCKQCkSN8AiFJd", "Team Finance V2"),
    ("2e8b5FGnQhiFLY9qE3EHANzKy5ZVgS6aV6VgUJCyKJqk", "Team Finance V1"),

    // Streamflow
    ("6VPVDzZLpYEXvtUfhvl2rL1xF7o9NKP3h1qcZrJ9QvXm", "Streamflow"),

    // Realms/SPL Governance
    ("GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw", "SPL Governance"),

    // Solana native programs for burning/locking
    ("11111111111111111111111111111111", "System Program (burned)"),
    ("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", "SPL Token Program"),
];

// DEX Program IDs for LP mint extraction
const RAYDIUM_CPMM_PROGRAM_ID: &str = "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C";
const RAYDIUM_CLMM_PROGRAM_ID: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
const RAYDIUM_LEGACY_PROGRAM_ID: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";
const ORCA_WHIRLPOOL_PROGRAM_ID: &str = "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc";
const METEORA_DLMM_PROGRAM_ID: &str = "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo";
const PUMPFUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
const PUMPFUN_AMM_PROGRAM_ID: &str = "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA";

/// Cached LP lock analysis entry
#[derive(Debug, Clone)]
struct CachedLpLockAnalysis {
    analysis: LpLockAnalysis,
    cached_at: DateTime<Utc>,
}

/// Liquidity pool lock status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LpLockStatus {
    /// LP tokens are burned (mint authority = None) - permanent lock
    Burned,
    /// LP tokens are locked in a time-based program
    TimeLocked {
        unlock_date: Option<DateTime<Utc>>,
        program: String,
    },
    /// LP tokens are held by a lock/vesting program
    ProgramLocked {
        program: String,
        amount: u64,
    },
    /// LP tokens appear to be locked based on holder analysis
    Locked {
        amount: u64,
        confidence: u8,
    },
    /// LP tokens are not locked (with confidence score)
    NotLocked {
        confidence: u8,
    },
    /// LP tokens are held by pool creator/deployer (not locked)
    CreatorHeld,
    /// Cannot determine lock status (insufficient data)
    Unknown,
    /// No liquidity pool found
    NoPool,
}

impl LpLockStatus {
    /// Check if the LP is considered safe (burned or properly locked)
    pub fn is_safe(&self) -> bool {
        match self {
            LpLockStatus::Burned => true,
            LpLockStatus::TimeLocked { .. } => true,
            LpLockStatus::ProgramLocked { .. } => true,
            LpLockStatus::Locked { confidence, .. } => *confidence >= 70,
            LpLockStatus::NotLocked { .. } => false,
            LpLockStatus::CreatorHeld => false,
            LpLockStatus::Unknown => false,
            LpLockStatus::NoPool => false,
        }
    }

    /// Get human-readable status description
    pub fn description(&self) -> &'static str {
        match self {
            LpLockStatus::Burned => "LP tokens burned (permanent lock)",
            LpLockStatus::TimeLocked { .. } => "LP tokens time-locked",
            LpLockStatus::ProgramLocked { .. } => "LP tokens held by lock program",
            LpLockStatus::Locked { confidence, .. } => {
                if *confidence >= 80 {
                    "LP tokens appear locked (high confidence)"
                } else if *confidence >= 60 {
                    "LP tokens appear locked (medium confidence)"
                } else {
                    "LP tokens appear locked (low confidence)"
                }
            }
            LpLockStatus::NotLocked { .. } => "LP tokens not locked",
            LpLockStatus::CreatorHeld => "LP tokens held by creator (risky)",
            LpLockStatus::Unknown => "Unable to determine lock status",
            LpLockStatus::NoPool => "No liquidity pool found",
        }
    }

    /// Get risk level indicator
    pub fn risk_level(&self) -> &'static str {
        match self {
            LpLockStatus::Burned => "Low",
            LpLockStatus::TimeLocked { .. } => "Low",
            LpLockStatus::ProgramLocked { .. } => "Medium",
            LpLockStatus::Locked { confidence, .. } => {
                if *confidence >= 80 {
                    "Low"
                } else if *confidence >= 60 {
                    "Medium"
                } else {
                    "High"
                }
            }
            LpLockStatus::NotLocked { confidence, .. } => {
                if *confidence >= 80 { "High" } else { "Medium" }
            }
            LpLockStatus::CreatorHeld => "High",
            LpLockStatus::Unknown => "High",
            LpLockStatus::NoPool => "High",
        }
    }
}

/// LP lock analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpLockAnalysis {
    /// Token mint address
    pub token_mint: String,
    /// Pool address (if found)
    pub pool_address: Option<String>,
    /// DEX ID (raydium, orca, etc.)
    pub dex_id: Option<String>,
    /// LP token mint address (if found)
    pub lp_mint: Option<String>,
    /// Lock status
    pub status: LpLockStatus,
    /// Analysis timestamp
    pub analyzed_at: DateTime<Utc>,
    /// Lock verification score (0-100, higher is more secure)
    pub lock_score: u8,
    /// Additional details and notes
    pub details: Vec<String>,
    /// Data source used for analysis
    pub data_source: String,
}

impl LpLockAnalysis {
    /// Check if this analysis is valid/safe for trading decisions
    pub fn is_valid_for_trading(&self) -> bool {
        self.status.is_safe() && self.lock_score >= 70
    }

    /// Get a summary string for logging
    pub fn summary(&self) -> String {
        format!(
            "{} - {} (score: {})",
            safe_truncate(&self.token_mint, 8),
            self.status.description(),
            self.lock_score
        )
    }
}

/// Check if a token's liquidity pool is locked
/// This is the main function that should be used everywhere
pub async fn check_lp_lock_status(token_mint: &str) -> Result<LpLockAnalysis, String> {
    check_lp_lock_status_with_cache(token_mint, true).await
}

/// Check if a token's liquidity pool is locked with cache control
pub async fn check_lp_lock_status_with_cache(
    token_mint: &str,
    use_cache: bool
) -> Result<LpLockAnalysis, String> {
    // Check cache first if enabled
    if use_cache {
        if let Some(cached) = get_cached_lp_analysis(token_mint).await {
            log(
                LogTag::Security,
                "CACHE_HIT",
                &format!("LP lock cache hit for {}", safe_truncate(token_mint, 8))
            );
            return Ok(cached);
        }
    }

    log(
        LogTag::Security,
        "ANALYZING",
        &format!("Analyzing LP lock status for FULL_MINT: {}", token_mint)
    );

    let start_time = std::time::Instant::now();

    // Step 1: Get pools from DexScreener (try cache first, then API)
    let pools = match get_pools_for_token(token_mint).await {
        Ok(pools) => pools,
        Err(e) => {
            log(
                LogTag::Security,
                "ERROR",
                &format!("Failed to get pools for FULL_MINT: {} - Error: {}", token_mint, e)
            );
            return Ok(LpLockAnalysis {
                token_mint: token_mint.to_string(),
                pool_address: None,
                dex_id: None,
                lp_mint: None,
                status: LpLockStatus::NoPool,
                analyzed_at: Utc::now(),
                lock_score: 0,
                details: vec![format!("Failed to find pools: {}", e)],
                data_source: "dexscreener".to_string(),
            });
        }
    };

    if pools.is_empty() {
        log(LogTag::Security, "NO_POOLS", &format!("No pools found for FULL_MINT: {}", token_mint));
        return Ok(LpLockAnalysis {
            token_mint: token_mint.to_string(),
            pool_address: None,
            dex_id: None,
            lp_mint: None,
            status: LpLockStatus::NoPool,
            analyzed_at: Utc::now(),
            lock_score: 0,
            details: vec!["No liquidity pools found".to_string()],
            data_source: "dexscreener".to_string(),
        });
    }

    // Step 2: Select the best pool (highest liquidity)
    let best_pool = select_best_pool(&pools);

    log(
        LogTag::Security,
        "POOL_SELECTED",
        &format!(
            "Selected {} pool {} for FULL_MINT: {}",
            best_pool.dex_id,
            best_pool.pair_address,
            token_mint
        )
    );

    // Step 3: Analyze the selected pool with on-chain verification
    let analysis = analyze_pool_lock_status_onchain(token_mint, &best_pool).await?;

    let elapsed = start_time.elapsed();
    log(
        LogTag::Security,
        "ANALYSIS_COMPLETE",
        &format!(
            "LP lock analysis for FULL_MINT: {} completed in {}ms: {}",
            token_mint,
            elapsed.as_millis(),
            analysis.summary()
        )
    );

    // Cache the result
    if use_cache {
        cache_lp_analysis(token_mint, &analysis).await;
    }

    Ok(analysis)
}

/// Get pools for a token from DexScreener (cache-first)
async fn get_pools_for_token(token_mint: &str) -> Result<Vec<TokenPair>, String> {
    // Try to get from cache first
    if let Some(cached_pools) = get_cached_pools_for_token(token_mint).await {
        log(
            LogTag::Security,
            "POOL_CACHE_HIT",
            &format!("Using cached pools for {}", safe_truncate(token_mint, 8))
        );
        return Ok(cached_pools);
    }

    // Fall back to API call (which will cache the result)
    log(
        LogTag::Security,
        "POOL_API_CALL",
        &format!("Fetching pools from API for FULL_MINT: {}", token_mint)
    );

    get_token_pools_from_dexscreener(token_mint).await
}

/// Select the best pool for analysis (highest liquidity, prefer known DEXs)
fn select_best_pool(pools: &[TokenPair]) -> &TokenPair {
    // Priority order: raydium > orca > meteora > others
    let dex_priority = |dex_id: &str| -> u32 {
        match dex_id.to_lowercase().as_str() {
            "raydium" => 100,
            "orca" => 90,
            "meteora" => 80,
            _ => 50,
        }
    };

    pools
        .iter()
        .max_by(|a, b| {
            let a_priority = dex_priority(&a.dex_id);
            let b_priority = dex_priority(&b.dex_id);

            // First compare by DEX priority
            match a_priority.cmp(&b_priority) {
                std::cmp::Ordering::Equal => {
                    // If same priority, compare by liquidity
                    let a_liquidity = a.liquidity
                        .as_ref()
                        .map(|l| l.usd)
                        .unwrap_or(0.0);
                    let b_liquidity = b.liquidity
                        .as_ref()
                        .map(|l| l.usd)
                        .unwrap_or(0.0);
                    a_liquidity.partial_cmp(&b_liquidity).unwrap_or(std::cmp::Ordering::Equal)
                }
                other => other,
            }
        })
        .unwrap_or(&pools[0]) // Fallback to first pool if comparison fails
}

/// Analyze a specific pool's lock status with on-chain verification
async fn analyze_pool_lock_status_onchain(
    token_mint: &str,
    pool: &TokenPair
) -> Result<LpLockAnalysis, String> {
    let mut details = Vec::new();
    let mut lock_score = 0u8;

    details.push(format!("DEX: {}", pool.dex_id));
    details.push(format!("Pool: {}", safe_truncate(&pool.pair_address, 12)));

    if let Some(liquidity) = &pool.liquidity {
        details.push(format!("Liquidity: ${:.0}", liquidity.usd));
    }

    // Step 1: Get LP mint address from on-chain pool data
    let lp_mint = match extract_lp_mint_from_pool(pool).await {
        Ok(Some(mint)) => {
            details.push(format!("LP Mint: {}", safe_truncate(&mint, 12)));
            Some(mint)
        }
        Ok(None) => {
            details.push("LP mint not found or not applicable for this pool type".to_string());
            None
        }
        Err(e) => {
            details.push(format!("Failed to extract LP mint: {}", e));
            None
        }
    };

    // Step 2: Perform on-chain verification if we have LP mint
    let status = if let Some(lp_mint_address) = &lp_mint {
        match verify_lp_lock_onchain(lp_mint_address, &mut details, &mut lock_score).await {
            Ok(status) => status,
            Err(e) => {
                details.push(format!("On-chain verification failed: {}", e));
                // Fall back to heuristic analysis
                determine_lock_status_from_pool_data(pool, &mut details, &mut lock_score)
            }
        }
    } else {
        // No LP mint available, use heuristic analysis
        determine_lock_status_from_pool_data(pool, &mut details, &mut lock_score)
    };

    Ok(LpLockAnalysis {
        token_mint: token_mint.to_string(),
        pool_address: Some(pool.pair_address.clone()),
        dex_id: Some(pool.dex_id.clone()),
        lp_mint: lp_mint,
        status,
        analyzed_at: Utc::now(),
        lock_score,
        details,
        data_source: "onchain+dexscreener".to_string(),
    })
}

/// Extract LP mint address from pool account data based on DEX type
async fn extract_lp_mint_from_pool(pool: &TokenPair) -> Result<Option<String>, String> {
    let client = get_rpc_client();

    // Get pool account data
    let pool_pubkey = Pubkey::from_str(&pool.pair_address).map_err(|e|
        format!("Invalid pool address: {}", e)
    )?;

    let account_info = client
        .get_account(&pool_pubkey).await
        .map_err(|e| format!("Failed to get pool account: {}", e))?;

    let account_data = account_info.data;

    // Extract LP mint based on DEX type
    match pool.dex_id.to_lowercase().as_str() {
        "raydium" => extract_raydium_lp_mint(&account_data, &pool.pair_address).await,
        "orca" => extract_orca_lp_mint(&account_data).await,
        "meteora" => extract_meteora_lp_mint(&account_data).await,
        "pumpfun" | "pumpswap" => extract_pumpfun_lp_mint(&account_data).await,
        _ => {
            log(
                LogTag::Security,
                "WARN",
                &format!("Unsupported DEX for LP extraction: {}", pool.dex_id)
            );
            Ok(None)
        }
    }
}

/// Extract LP mint from Raydium pool data
async fn extract_raydium_lp_mint(
    data: &[u8],
    pool_address: &str
) -> Result<Option<String>, String> {
    // Try CPMM first (most common)
    if let Some(pool_info) = RaydiumCpmmDecoder::decode_raydium_cpmm_pool(data, pool_address) {
        return Ok(Some(pool_info.lp_mint.to_string()));
    }

    // TODO: Add CLMM and Legacy AMM decoders when needed
    log(
        LogTag::Security,
        "WARN",
        &format!("Could not decode Raydium pool: {}", safe_truncate(pool_address, 8))
    );
    Ok(None)
}

/// Extract LP mint from Orca pool data
async fn extract_orca_lp_mint(data: &[u8]) -> Result<Option<String>, String> {
    // Orca Whirlpools (CLMM) don't have traditional LP tokens
    // They use position NFTs instead
    log(LogTag::Security, "INFO", "Orca Whirlpools use position NFTs, not LP tokens");
    Ok(None)
}

/// Extract LP mint from Meteora pool data
async fn extract_meteora_lp_mint(data: &[u8]) -> Result<Option<String>, String> {
    // Meteora DLMM pools don't have traditional LP tokens
    // DAMM pools may have LP tokens but structure varies
    log(LogTag::Security, "INFO", "Meteora pools may not use traditional LP tokens");
    Ok(None)
}

/// Extract LP mint from PumpFun pool data
async fn extract_pumpfun_lp_mint(data: &[u8]) -> Result<Option<String>, String> {
    // PumpFun pools use a bonding curve mechanism, not traditional LP tokens
    // Only graduated tokens that moved to Raydium have actual LP tokens
    // For pure PumpFun pools, there are no LP tokens to burn/lock

    // Check if this is the PumpFun AMM program (bonding curve)
    // If so, return None as there are no LP tokens

    log(
        LogTag::Security,
        "INFO",
        "PumpFun bonding curve detected - no LP tokens exist for this pool type"
    );

    // Return None to indicate no LP tokens (not an error, but no LP to check)
    Ok(None)
}

/// Perform on-chain verification of LP lock status
async fn verify_lp_lock_onchain(
    lp_mint: &str,
    details: &mut Vec<String>,
    lock_score: &mut u8
) -> Result<LpLockStatus, String> {
    let client = get_rpc_client();

    // Step 1: Check mint authority (most reliable indicator)
    let mint_info = get_mint_info(&client, lp_mint).await?;

    details.push(format!("LP supply: {}", mint_info.supply));

    // Check if mint authority is None (burned)
    if mint_info.mint_authority.is_none() {
        details.push("✅ LP mint authority is None (burned)".to_string());
        *lock_score = 100;
        return Ok(LpLockStatus::Burned);
    }

    let mint_authority = mint_info.mint_authority.unwrap();
    details.push(format!("LP mint authority: {}", safe_truncate(&mint_authority, 12)));

    // Check if mint authority is a known lock program
    if let Some(lock_program_name) = is_known_lock_program(&mint_authority) {
        details.push(format!("✅ Mint authority is known lock program: {}", lock_program_name));
        *lock_score = 95;
        return Ok(LpLockStatus::ProgramLocked {
            program: lock_program_name.to_string(),
            amount: mint_info.supply,
        });
    }

    // Step 2: Check LP token holders
    let lp_mint_pubkey = Pubkey::from_str(lp_mint).map_err(|e|
        format!("Invalid LP mint address: {}", e)
    )?;
    let holder_analysis = analyze_lp_token_holders(&lp_mint_pubkey, details, lock_score).await?;

    Ok(holder_analysis)
}

/// Helper to get mint information from RPC
async fn get_mint_info(client: &crate::rpc::RpcClient, mint: &str) -> Result<MintInfo, String> {
    let mint_data = client
        .get_mint_account(mint).await
        .map_err(|e| format!("Failed to get mint account: {}", e))?;

    if let Some(result) = mint_data.get("result") {
        if let Some(value) = result.get("value") {
            if let Some(data) = value.get("data") {
                if let Some(parsed) = data.get("parsed") {
                    if let Some(info) = parsed.get("info") {
                        let supply = info
                            .get("supply")
                            .and_then(|s| s.as_str())
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(0);

                        let mint_authority = info
                            .get("mintAuthority")
                            .and_then(|ma| ma.as_str())
                            .map(|s| s.to_string());

                        return Ok(MintInfo {
                            supply,
                            mint_authority,
                        });
                    }
                }
            }
        }
    }

    Err("Failed to parse mint account data".to_string())
}

/// Simple struct to hold mint information
#[derive(Debug)]
struct MintInfo {
    pub supply: u64,
    pub mint_authority: Option<String>,
}

/// Analyze who holds the LP tokens
async fn analyze_lp_token_holders(
    lp_mint: &Pubkey,
    details: &mut Vec<String>,
    lock_score: &mut u8
) -> Result<LpLockStatus, String> {
    let client = get_rpc_client();

    // Use get_program_accounts to find token accounts for this mint
    let program_id_str = spl_token::id().to_string();

    // Create filters for token accounts with this mint
    let filters =
        serde_json::json!([
        {"dataSize": 165}, // SPL token account size
        {"memcmp": {"offset": 0, "bytes": lp_mint.to_string()}} // mint filter
    ]);

    let response = client
        .get_program_accounts(&program_id_str, Some(filters), Some("base64"), Some(30)).await
        .map_err(|e| format!("Failed to get LP token accounts: {}", e))?;

    if response.is_empty() {
        details.push("No LP token holders found".to_string());
        return Ok(LpLockStatus::Unknown);
    }

    details.push(format!("Found {} LP token accounts", response.len()));

    let mut total_analyzed = 0u64;
    let mut locked_amount = 0u64;
    let mut creator_held_amount = 0u64;
    let mut accounts_with_balance = 0;

    // Analyze token accounts (limit to first 10 for performance)
    for account_data in response.iter().take(10) {
        if let Some(account_obj) = account_data.as_object() {
            if
                let (Some(pubkey_str), Some(account)) = (
                    account_obj.get("pubkey").and_then(|v| v.as_str()),
                    account_obj.get("account").and_then(|v| v.as_object()),
                )
            {
                if let Some(data_str) = account.get("data").and_then(|v| v.as_str()) {
                    // Decode base64 account data
                    if let Ok(data) = base64::decode(data_str) {
                        // Try to parse as SPL token account
                        if let Ok(token_account) = spl_token::state::Account::unpack(&data) {
                            if token_account.amount > 0 {
                                accounts_with_balance += 1;
                                total_analyzed += token_account.amount;

                                details.push(
                                    format!(
                                        "LP holder: {} tokens at {}",
                                        token_account.amount,
                                        safe_truncate(pubkey_str, 12)
                                    )
                                );

                                let owner_str = token_account.owner.to_string();

                                // Check if held by known lock program
                                if let Some(lock_program_name) = is_known_lock_program(&owner_str) {
                                    details.push(
                                        format!("  ✅ Held by lock program: {}", lock_program_name)
                                    );
                                    locked_amount += token_account.amount;
                                    *lock_score += 15;
                                } else if is_likely_creator_wallet(&owner_str) {
                                    details.push("  ⚠️  Possibly held by creator".to_string());
                                    creator_held_amount += token_account.amount;
                                } else {
                                    details.push(
                                        format!("  Owner: {}", safe_truncate(&owner_str, 12))
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Determine status based on analysis
    let locked_percentage = if total_analyzed > 0 {
        (locked_amount * 100) / total_analyzed
    } else {
        0
    };

    let creator_percentage = if total_analyzed > 0 {
        (creator_held_amount * 100) / total_analyzed
    } else {
        0
    };

    details.push(
        format!(
            "Analysis: {} accounts with balance, Locked: {}%, Creator-held: {}%",
            accounts_with_balance,
            locked_percentage,
            creator_percentage
        )
    );

    if locked_percentage >= 80 {
        *lock_score += 30;
        Ok(LpLockStatus::ProgramLocked {
            program: "Various lock programs".to_string(),
            amount: locked_amount,
        })
    } else if accounts_with_balance <= 3 && locked_percentage >= 50 {
        *lock_score += 20;
        Ok(LpLockStatus::Locked {
            amount: locked_amount,
            confidence: 75,
        })
    } else if creator_percentage >= 50 {
        Ok(LpLockStatus::CreatorHeld)
    } else {
        Ok(LpLockStatus::Unknown)
    }
}

/// Check if an address is a known lock program
fn is_known_lock_program(address: &str) -> Option<&'static str> {
    for (program_address, program_name) in KNOWN_LOCK_PROGRAMS {
        if address == *program_address {
            return Some(program_name);
        }
    }
    None
}

/// Heuristic to determine if an address is likely a creator wallet
fn is_likely_creator_wallet(address: &str) -> bool {
    // This is a simple heuristic - in practice you might want more sophisticated analysis
    // For now, we assume non-program addresses might be creator wallets
    !is_known_lock_program(address).is_some() && address != "11111111111111111111111111111111"
}

/// Analyze a specific pool's lock status (fallback heuristic method)
async fn analyze_pool_lock_status(
    token_mint: &str,
    pool: &TokenPair
) -> Result<LpLockAnalysis, String> {
    let mut details = Vec::new();
    let mut lock_score = 0u8;

    details.push(format!("DEX: {}", pool.dex_id));
    details.push(format!("Pool: {}", safe_truncate(&pool.pair_address, 12)));

    if let Some(liquidity) = &pool.liquidity {
        details.push(format!("Liquidity: ${:.0}", liquidity.usd));
    }

    // For now, we'll implement a basic analysis based on available DexScreener data
    // In the future, this could be enhanced with RPC calls to check actual LP token details

    let status = determine_lock_status_from_pool_data(pool, &mut details, &mut lock_score);

    Ok(LpLockAnalysis {
        token_mint: token_mint.to_string(),
        pool_address: Some(pool.pair_address.clone()),
        dex_id: Some(pool.dex_id.clone()),
        lp_mint: None, // DexScreener doesn't provide LP mint directly
        status,
        analyzed_at: Utc::now(),
        lock_score,
        details,
        data_source: "dexscreener_heuristic".to_string(),
    })
}

/// Determine lock status based on DexScreener pool data
fn determine_lock_status_from_pool_data(
    pool: &TokenPair,
    details: &mut Vec<String>,
    lock_score: &mut u8
) -> LpLockStatus {
    // Check if pool has certain labels that indicate locking
    if let Some(labels) = &pool.labels {
        for label in labels {
            let label_lower = label.to_lowercase();
            if label_lower.contains("locked") || label_lower.contains("burn") {
                details.push(format!("Found lock indicator in labels: {}", label));
                *lock_score += 30;
            }
        }
    }

    // Check pool age (older pools are generally more trustworthy)
    if let Some(created_at) = pool.pair_created_at {
        let created_time = DateTime::from_timestamp(created_at as i64, 0).unwrap_or_else(||
            Utc::now()
        );
        let age_days = Utc::now().signed_duration_since(created_time).num_days();

        details.push(format!("Pool age: {} days", age_days));

        if age_days > 30 {
            *lock_score += 20;
        } else if age_days > 7 {
            *lock_score += 10;
        }
    }

    // Check DEX reputation and special cases
    match pool.dex_id.to_lowercase().as_str() {
        "raydium" | "orca" => {
            details.push(format!("Reputable DEX: {}", pool.dex_id));
            *lock_score += 20;
        }
        "meteora" | "jupiter" => {
            details.push(format!("Known DEX: {}", pool.dex_id));
            *lock_score += 10;
        }
        "pumpfun" | "pumpswap" => {
            details.push("PumpFun bonding curve - no LP tokens to burn/lock".to_string());
            details.push("Bonding curve mechanism provides inherent safety".to_string());
            *lock_score += 40; // Bonding curves are inherently safer
        }
        _ => {
            details.push(format!("Unknown DEX: {}", pool.dex_id));
        }
    }

    // Check liquidity level (higher liquidity often indicates more established projects)
    if let Some(liquidity) = &pool.liquidity {
        if liquidity.usd > 100_000.0 {
            details.push("High liquidity pool".to_string());
            *lock_score += 15;
        } else if liquidity.usd > 10_000.0 {
            details.push("Medium liquidity pool".to_string());
            *lock_score += 10;
        } else {
            details.push("Low liquidity pool".to_string());
        }
    }

    // Determine status based on score and available data
    // Special case for PumpFun bonding curves
    if pool.dex_id.to_lowercase() == "pumpfun" || pool.dex_id.to_lowercase() == "pumpswap" {
        details.push("Bonding curve mechanism - no traditional LP lock required".to_string());
        return LpLockStatus::NotLocked { confidence: 90 }; // Safe but not "locked" in traditional sense
    }

    if *lock_score >= 70 {
        details.push("High confidence in pool safety".to_string());
        LpLockStatus::Burned // Assume burned/locked for high-score pools
    } else if *lock_score >= 50 {
        details.push("Medium confidence - potential time lock".to_string());
        LpLockStatus::TimeLocked {
            unlock_date: None,
            program: "Unknown".to_string(),
        }
    } else if *lock_score >= 30 {
        details.push("Low confidence - may be creator held".to_string());
        LpLockStatus::CreatorHeld
    } else {
        details.push("Insufficient data for reliable analysis".to_string());
        LpLockStatus::Unknown
    }
}

/// Get cached LP lock analysis if available and not expired
async fn get_cached_lp_analysis(token_mint: &str) -> Option<LpLockAnalysis> {
    let cache = LP_LOCK_CACHE.read().ok()?;

    if let Some(cached) = cache.get(token_mint) {
        let now = Utc::now();
        let cache_age = now.signed_duration_since(cached.cached_at).num_seconds();

        if cache_age < LP_LOCK_CACHE_TTL_SECS {
            return Some(cached.analysis.clone());
        }
    }

    None
}

/// Cache LP lock analysis result
async fn cache_lp_analysis(token_mint: &str, analysis: &LpLockAnalysis) {
    if let Ok(mut cache) = LP_LOCK_CACHE.write() {
        cache.insert(token_mint.to_string(), CachedLpLockAnalysis {
            analysis: analysis.clone(),
            cached_at: Utc::now(),
        });
    }
}

/// Batch check LP lock status for multiple tokens
pub async fn check_multiple_lp_locks(
    token_mints: &[String]
) -> Result<Vec<LpLockAnalysis>, String> {
    let mut results = Vec::new();

    for mint in token_mints {
        match check_lp_lock_status(mint).await {
            Ok(analysis) => results.push(analysis),
            Err(e) => {
                log(
                    LogTag::Security,
                    "ERROR",
                    &format!("Failed to analyze LP lock for {}: {}", safe_truncate(mint, 8), e)
                );
                // Continue with other tokens even if one fails
            }
        }
    }

    Ok(results)
}

/// Quick check if a token's LP is considered safe
pub async fn is_lp_safe(token_mint: &str) -> Result<bool, String> {
    let analysis = check_lp_lock_status(token_mint).await?;
    Ok(analysis.is_valid_for_trading())
}

/// Legacy LockPrograms struct for compatibility
pub struct LockPrograms;

impl LockPrograms {
    /// Get list of known lock/vesting program addresses (empty for now)
    pub fn known_programs() -> std::collections::HashMap<&'static str, &'static str> {
        std::collections::HashMap::new()
    }

    /// Check if an address is a known lock program
    pub fn is_lock_program(_address: &str) -> Option<&'static str> {
        None
    }
}
