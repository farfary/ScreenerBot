/// Enhanced Liquidity Pool Lock Detection Module
///
/// This module provides comprehensive functionality to detect whether a token's
/// liquidity pool is locked across all major DEXes on Solana. It supports:
/// - Multi-pool analysis across different DEXes
/// - DEX-specific LP mint extraction
/// - Comprehensive on-chain verification
/// - Special handling for bonding curves and concentrated liquidity
/// - Robust fallback strategies

use crate::logger::{ log, LogTag };
use crate::rpc::get_rpc_client;
use crate::tokens::dexscreener::{
    get_token_pools_from_dexscreener,
    get_cached_pools_for_token,
    TokenPair,
};
use crate::pools::decoders::RaydiumCpmmDecoder;
use crate::pools::types::{
    ProgramKind,
    RAYDIUM_CPMM_PROGRAM_ID,
    RAYDIUM_CLMM_PROGRAM_ID,
    RAYDIUM_LEGACY_AMM_PROGRAM_ID,
    ORCA_WHIRLPOOL_PROGRAM_ID,
    METEORA_DLMM_PROGRAM_ID,
    METEORA_DAMM_PROGRAM_ID,
    PUMP_FUN_AMM_PROGRAM_ID,
    PUMP_FUN_LEGACY_PROGRAM_ID,
};
use crate::utils::safe_truncate;
use base64;
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
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

/// Enhanced list of known LP lock/vesting program addresses on Solana
const KNOWN_LOCK_PROGRAMS: &[(&str, &str)] = &[
    // Team Finance
    ("J2ZDhSq8CWNaQ1UZAFALxLm4oJ7mS9tCKQCkSN8AiFJd", "Team Finance V2"),
    ("2e8b5FGnQhiFLY9qE3EHANzKy5ZVgS6aV6VgUJCyKJqk", "Team Finance V1"),

    // Streamflow
    ("6VPVDzZLpYEXvtUfhvl2rL1xF7o9NKP3h1qcZrJ9QvXm", "Streamflow"),
    ("strmRqUCoQUgGUan5YhzUZa6KqdzwX5L6FpUxfmKg5m", "Streamflow V2"),

    // Realms/SPL Governance
    ("GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw", "SPL Governance"),

    // Solana native programs for burning/locking
    ("11111111111111111111111111111111", "System Program (burned)"),
    ("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA", "SPL Token Program"),

    // Additional lock programs
    ("LocktDzaV1W2Bm9DeZeiyz4J9zs4fRqNiYqQyracRXw", "Lockt Protocol"),
    ("CLocKyM6DFBNnYWkXSdNk9xFAKpH1R1UvRFWpuNhzVE", "CLock"),
    ("9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM", "Paid Network"),
];

/// Extract LP mint using program-specific logic
fn extract_lp_mint(
    program_id: &str,
    data: &[u8],
    pool_address: &str,
    details: &mut Vec<String>
) -> Result<Option<String>, String> {
    log(
        LogTag::Security,
        "LP_EXTRACTOR",
        &format!(
            "Extracting LP mint for program {} on pool {}",
            safe_truncate(program_id, 12),
            safe_truncate(pool_address, 8)
        )
    );

    // Match on program ID and call appropriate extractor
    match program_id {
        RAYDIUM_CPMM_PROGRAM_ID => extract_raydium_cpmm_lp(data, pool_address),
        RAYDIUM_CLMM_PROGRAM_ID => extract_raydium_clmm_lp(data, pool_address),
        RAYDIUM_LEGACY_AMM_PROGRAM_ID => extract_raydium_legacy_lp(data, pool_address),
        ORCA_WHIRLPOOL_PROGRAM_ID => extract_orca_whirlpool_lp(data, pool_address),
        METEORA_DLMM_PROGRAM_ID => extract_meteora_dlmm_lp(data, pool_address),
        METEORA_DAMM_PROGRAM_ID => extract_meteora_damm_lp(data, pool_address),
        PUMP_FUN_AMM_PROGRAM_ID => extract_pumpfun_amm_lp(data, pool_address),
        PUMP_FUN_LEGACY_PROGRAM_ID => extract_pumpfun_legacy_lp(data, pool_address),
        _ => {
            // Unknown program ID
            details.push(format!("Unknown pool program: {}", safe_truncate(program_id, 12)));
            log(
                LogTag::Security,
                "UNKNOWN_PROGRAM",
                &format!(
                    "No LP extractor available for program {} on pool {}",
                    safe_truncate(program_id, 12),
                    safe_truncate(pool_address, 8)
                )
            );
            Ok(None)
        }
    }
}

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
    /// Pool uses position NFTs or other non-traditional LP mechanism
    PositionNft {
        dex: String,
        mechanism: String,
    },
    /// Bonding curve mechanism (inherently safe, no LP tokens)
    BondingCurve {
        dex: String,
    },
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
            LpLockStatus::PositionNft { .. } => true, // Position NFTs are generally safe
            LpLockStatus::BondingCurve { .. } => true, // Bonding curves are inherently safe
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
            LpLockStatus::PositionNft { .. } => "Uses position NFTs (safe mechanism)",
            LpLockStatus::BondingCurve { .. } => "Bonding curve (no LP tokens to rug)",
            LpLockStatus::Unknown => "Unable to determine lock status",
            LpLockStatus::NoPool => "No liquidity pool found",
        }
    }

    /// Get risk level indicator
    pub fn risk_level(&self) -> &'static str {
        match self {
            LpLockStatus::Burned => "Low",
            LpLockStatus::TimeLocked { .. } => "Low",
            LpLockStatus::ProgramLocked { .. } => "Low",
            LpLockStatus::PositionNft { .. } => "Low",
            LpLockStatus::BondingCurve { .. } => "Low",
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
    /// All analyzed pools (for comprehensive view)
    pub analyzed_pools: Vec<PoolAnalysis>,
}

/// Individual pool analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolAnalysis {
    pub pool_address: String,
    pub dex_id: String,
    pub program_id: String,
    pub lp_mint: Option<String>,
    pub status: LpLockStatus,
    pub confidence_score: u8,
    pub details: Vec<String>,
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

    /// Get the best pool analysis (highest confidence)
    pub fn best_pool(&self) -> Option<&PoolAnalysis> {
        self.analyzed_pools.iter().max_by_key(|p| p.confidence_score)
    }
}

/// Main entry point - check LP lock status across all pools
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
                &format!("LP lock cache hit for FULL_MINT: {}", token_mint)
            );
            return Ok(cached);
        }
    }

    log(
        LogTag::Security,
        "ANALYZING",
        &format!("Starting comprehensive LP lock analysis for FULL_MINT: {}", token_mint)
    );

    let start_time = std::time::Instant::now();

    // Get ALL pools for the token, not just the "best" one
    let pools = get_pools_for_token(token_mint).await?;

    if pools.is_empty() {
        log(LogTag::Security, "NO_POOLS", &format!("No pools found for FULL_MINT: {}", token_mint));

        // Special handling for bonding curve tokens
        let analysis = handle_no_pools_scenario(token_mint);

        if use_cache {
            cache_lp_analysis(token_mint, &analysis).await;
        }

        return Ok(analysis);
    }

    log(
        LogTag::Security,
        "POOL_COUNT",
        &format!("Found {} pools for FULL_MINT: {}", pools.len(), token_mint)
    );

    // Multi-pool analysis: Check each pool and collect results
    let mut pool_analyses = Vec::new();

    for (i, pool) in pools.iter().enumerate() {
        log(
            LogTag::Security,
            "ANALYZING_POOL",
            &format!(
                "Analyzing pool {}/{}: {} {} for FULL_MINT: {}",
                i + 1,
                pools.len(),
                pool.dex_id,
                safe_truncate(&pool.pair_address, 12),
                token_mint
            )
        );

        match analyze_single_pool(token_mint, pool).await {
            Ok(pool_analysis) => {
                log(
                    LogTag::Security,
                    "POOL_ANALYZED",
                    &format!(
                        "Pool {} analysis complete: {} (confidence: {})",
                        safe_truncate(&pool.pair_address, 8),
                        pool_analysis.status.description(),
                        pool_analysis.confidence_score
                    )
                );
                pool_analyses.push(pool_analysis);
            }
            Err(e) => {
                log(
                    LogTag::Security,
                    "POOL_ERROR",
                    &format!(
                        "Failed to analyze pool {} for FULL_MINT: {} - Error: {}",
                        safe_truncate(&pool.pair_address, 8),
                        token_mint,
                        e
                    )
                );
                // Continue with other pools
            }
        }
    }

    if pool_analyses.is_empty() {
        return Ok(create_unknown_analysis(token_mint, pools));
    }

    // Choose the most reliable result from pool analyses
    let final_analysis = select_best_analysis(token_mint, pool_analyses, pools);

    let elapsed = start_time.elapsed();
    log(
        LogTag::Security,
        "ANALYSIS_COMPLETE",
        &format!(
            "Comprehensive LP lock analysis for FULL_MINT: {} completed in {}ms: {}",
            token_mint,
            elapsed.as_millis(),
            final_analysis.summary()
        )
    );

    // Cache the result
    if use_cache {
        cache_lp_analysis(token_mint, &final_analysis).await;
    }

    Ok(final_analysis)
}

/// Get pools for a token from DexScreener (cache-first)
async fn get_pools_for_token(token_mint: &str) -> Result<Vec<TokenPair>, String> {
    // Try to get from cache first
    if let Some(cached_pools) = get_cached_pools_for_token(token_mint).await {
        log(
            LogTag::Security,
            "POOL_CACHE_HIT",
            &format!("Using cached pools for FULL_MINT: {}", token_mint)
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

/// Analyze a single pool - returns pool-specific analysis
async fn analyze_single_pool(token_mint: &str, pool: &TokenPair) -> Result<PoolAnalysis, String> {
    let mut details = Vec::new();
    let mut confidence_score = 0u8;

    // Add basic pool info
    details.push(format!("DEX: {}", pool.dex_id));
    details.push(format!("Pool: {}", pool.pair_address));

    if let Some(liquidity) = &pool.liquidity {
        details.push(format!("Liquidity: ${:.0}", liquidity.usd));
    }

    // Get pool account data
    let client = get_rpc_client();
    let pool_pubkey = Pubkey::from_str(&pool.pair_address).map_err(|e|
        format!("Invalid pool address {}: {}", pool.pair_address, e)
    )?;

    let account = client
        .get_account(&pool_pubkey).await
        .map_err(|e| format!("Failed to get pool account {}: {}", pool.pair_address, e))?;

    let program_id = account.owner.to_string();
    details.push(format!("Program: {}", safe_truncate(&program_id, 12)));

    // Find and use appropriate extractor based on program ID
    let lp_mint = extract_lp_mint(&program_id, &account.data, &pool.pair_address, &mut details)?;

    // Determine status based on LP mint and pool type
    let status = if let Some(lp_mint_address) = &lp_mint {
        // Traditional LP token found - verify lock status
        verify_lp_lock_comprehensive(&lp_mint_address, &mut details, &mut confidence_score).await?
    } else {
        // Special case handling for different pool types
        determine_status_without_lp_mint(pool, &program_id, &mut details, &mut confidence_score)
    };

    Ok(PoolAnalysis {
        pool_address: pool.pair_address.clone(),
        dex_id: pool.dex_id.clone(),
        program_id,
        lp_mint,
        status,
        confidence_score,
        details,
    })
}

/// Comprehensive verification of LP lock status
async fn verify_lp_lock_comprehensive(
    lp_mint: &str,
    details: &mut Vec<String>,
    confidence_score: &mut u8
) -> Result<LpLockStatus, String> {
    let client = get_rpc_client();

    log(
        LogTag::Security,
        "LP_VERIFICATION",
        &format!("Verifying LP lock status for LP_MINT: {}", lp_mint)
    );

    // Step 1: Check mint authority (most reliable indicator)
    let mint_info = get_mint_info(&client, lp_mint).await?;

    details.push(format!("LP supply: {}", mint_info.supply));

    // Check if mint authority is None (burned)
    if mint_info.mint_authority.is_none() {
        details.push("✅ LP mint authority is None (burned)".to_string());
        *confidence_score = 100;
        log(
            LogTag::Security,
            "LP_BURNED",
            &format!("LP mint authority burned for LP_MINT: {}", lp_mint)
        );
        return Ok(LpLockStatus::Burned);
    }

    let mint_authority = mint_info.mint_authority.unwrap();
    details.push(format!("LP mint authority: {}", safe_truncate(&mint_authority, 12)));

    // Check if mint authority is a known lock program
    if let Some(lock_program_name) = is_known_lock_program(&mint_authority) {
        details.push(format!("✅ Mint authority is known lock program: {}", lock_program_name));
        *confidence_score = 95;
        log(
            LogTag::Security,
            "LP_PROGRAM_LOCKED",
            &format!("LP mint locked by program {} for LP_MINT: {}", lock_program_name, lp_mint)
        );
        return Ok(LpLockStatus::ProgramLocked {
            program: lock_program_name.to_string(),
            amount: mint_info.supply,
        });
    }

    // Step 2: Enhanced LP token holder analysis
    analyze_lp_token_holders_enhanced(lp_mint, details, confidence_score).await
}

/// Enhanced analysis of LP token holders with better heuristics
async fn analyze_lp_token_holders_enhanced(
    lp_mint: &str,
    details: &mut Vec<String>,
    confidence_score: &mut u8
) -> Result<LpLockStatus, String> {
    let client = get_rpc_client();
    let lp_mint_pubkey = Pubkey::from_str(lp_mint).map_err(|e|
        format!("Invalid LP mint address: {}", e)
    )?;

    // Get all token accounts for this LP mint
    let program_id_str = spl_token::id().to_string();
    let filters =
        serde_json::json!([
        {"dataSize": 165}, // SPL token account size
        {"memcmp": {"offset": 0, "bytes": lp_mint}} // mint filter
    ]);

    let response = client
        .get_program_accounts(&program_id_str, Some(filters), Some("base64"), Some(50)).await
        .map_err(|e| format!("Failed to get LP token accounts: {}", e))?;

    if response.is_empty() {
        details.push("No LP token holders found".to_string());
        return Ok(LpLockStatus::Unknown);
    }

    let total_accounts = response.len();
    details.push(format!("Found {} LP token accounts", total_accounts));

    let mut total_supply = 0u64;
    let mut locked_amount = 0u64;
    let mut creator_held_amount = 0u64;
    let mut burn_held_amount = 0u64;
    let mut accounts_with_balance = 0;
    let mut large_holder_count = 0;

    // Enhanced analysis of token accounts
    for (i, account_data) in response.iter().enumerate() {
        // Limit processing for performance but get good sample
        if i >= 20 {
            break;
        }

        if let Some(account_obj) = account_data.as_object() {
            if
                let (Some(pubkey_str), Some(account)) = (
                    account_obj.get("pubkey").and_then(|v| v.as_str()),
                    account_obj.get("account").and_then(|v| v.as_object()),
                )
            {
                if let Some(data_str) = account.get("data").and_then(|v| v.as_str()) {
                    if let Ok(data) = base64::decode(data_str) {
                        if let Ok(token_account) = spl_token::state::Account::unpack(&data) {
                            if token_account.amount > 0 {
                                accounts_with_balance += 1;
                                total_supply += token_account.amount;

                                let owner_str = token_account.owner.to_string();

                                // Calculate relative amount
                                let amount_percentage = if total_supply > 0 {
                                    (token_account.amount * 100) / total_supply
                                } else {
                                    0
                                };

                                if token_account.amount > total_supply / 10 {
                                    large_holder_count += 1;
                                }

                                details.push(
                                    format!(
                                        "LP holder {}: {} tokens ({}%) at {}",
                                        i + 1,
                                        token_account.amount,
                                        amount_percentage,
                                        safe_truncate(pubkey_str, 12)
                                    )
                                );

                                // Enhanced owner analysis
                                if let Some(lock_program_name) = is_known_lock_program(&owner_str) {
                                    details.push(
                                        format!("  ✅ Held by lock program: {}", lock_program_name)
                                    );
                                    locked_amount += token_account.amount;
                                    *confidence_score += 20;
                                } else if is_burn_address(&owner_str) {
                                    details.push("  ✅ Held by burn address".to_string());
                                    burn_held_amount += token_account.amount;
                                    *confidence_score += 25;
                                } else if is_likely_creator_wallet(&owner_str) {
                                    details.push("  ⚠️ Possibly held by creator".to_string());
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

    // Enhanced determination based on analysis
    let total_analyzed = total_supply;
    let locked_percentage = if total_analyzed > 0 {
        ((locked_amount + burn_held_amount) * 100) / total_analyzed
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
            "Summary: {} accounts, Locked/Burned: {}%, Creator: {}%, Large holders: {}",
            accounts_with_balance,
            locked_percentage,
            creator_percentage,
            large_holder_count
        )
    );

    // Enhanced decision logic
    if burn_held_amount > locked_amount && burn_held_amount > creator_held_amount {
        *confidence_score += 35;
        Ok(LpLockStatus::Burned)
    } else if locked_percentage >= 80 {
        *confidence_score += 30;
        Ok(LpLockStatus::ProgramLocked {
            program: "Various lock programs".to_string(),
            amount: locked_amount,
        })
    } else if locked_percentage >= 50 || (accounts_with_balance <= 2 && large_holder_count <= 1) {
        *confidence_score += 25;
        Ok(LpLockStatus::Locked {
            amount: locked_amount,
            confidence: 80,
        })
    } else if creator_percentage >= 70 {
        Ok(LpLockStatus::CreatorHeld)
    } else if accounts_with_balance <= 5 && locked_percentage >= 20 {
        *confidence_score += 15;
        Ok(LpLockStatus::Locked {
            amount: locked_amount,
            confidence: 60,
        })
    } else {
        Ok(LpLockStatus::NotLocked {
            confidence: 70,
        })
    }
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

    // Fallback: attempt to parse raw data if jsonParsed path missing
    if let Some(value) = mint_data.get("value") {
        if let Some(data_arr) = value.get("data").and_then(|d| d.as_array()) {
            if let Some(base64_str) = data_arr.get(0).and_then(|v| v.as_str()) {
                if let Ok(raw) = base64::decode(base64_str) {
                    // Standard SPL mint layout is 82 bytes minimum
                    if raw.len() >= 82 {
                        // Offset 36..68 = mint authority option+pubkey? Simpler: use spl_token::state::Mint unpack
                        if let Ok(mint_state) = spl_token::state::Mint::unpack(&raw) {
                            let mint_authority: Option<String> = mint_state.mint_authority
                                .map(|k| k.to_string())
                                .into();
                            return Ok(MintInfo { supply: mint_state.supply, mint_authority });
                        }
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

/// Helper for special pool types that don't use traditional LP tokens
fn determine_status_without_lp_mint(
    pool: &TokenPair,
    program_id: &str,
    details: &mut Vec<String>,
    confidence_score: &mut u8
) -> LpLockStatus {
    let program_kind = ProgramKind::from_program_id(program_id);

    match program_kind {
        ProgramKind::RaydiumClmm => {
            details.push("Raydium CLMM uses position NFTs instead of LP tokens".to_string());
            details.push("Position NFT system provides inherent safety".to_string());
            *confidence_score = 85;
            LpLockStatus::PositionNft {
                dex: "Raydium".to_string(),
                mechanism: "CLMM Position NFTs".to_string(),
            }
        }
        ProgramKind::OrcaWhirlpool => {
            details.push("Orca Whirlpools use position NFTs instead of LP tokens".to_string());
            details.push("Position NFT system provides inherent safety".to_string());
            *confidence_score = 85;
            LpLockStatus::PositionNft {
                dex: "Orca".to_string(),
                mechanism: "Whirlpool Position NFTs".to_string(),
            }
        }
        ProgramKind::MeteoraDlmm => {
            details.push("Meteora DLMM may use non-traditional LP mechanism".to_string());
            *confidence_score = 60;
            LpLockStatus::PositionNft {
                dex: "Meteora".to_string(),
                mechanism: "DLMM".to_string(),
            }
        }
        ProgramKind::MeteoraDamm => {
            // DAMM v2 pools do not expose a traditional lp_mint – liquidity is managed via dynamic virtual bins / position constructs.
            // Previous heuristic offsets (136/104/72/40) produced false-positive random 32-byte segments that failed mint parsing.
            // We classify as a non-traditional position-based mechanism (safe) unless a *validated* lp_mint field is ever formally documented.
            details.push(
                "Meteora DAMM v2 uses dynamic liquidity positions (no standard LP mint)".to_string()
            );
            details.push(
                "Treating as position-style mechanism – rug via mint inflation not possible".to_string()
            );
            *confidence_score = 85;
            LpLockStatus::PositionNft {
                dex: "Meteora".to_string(),
                mechanism: "DAMM v2 dynamic positions".to_string(),
            }
        }
        ProgramKind::PumpFunLegacy | ProgramKind::PumpFunAmm => {
            let dex_lower = pool.dex_id.to_lowercase();
            if dex_lower.contains("pumpfun") || dex_lower.contains("pumpswap") {
                details.push(
                    "PumpFun bonding-curve style / pumpswap pool (no traditional LP mint)".to_string()
                );
                details.push(
                    "✅ Bonding curve / virtual liquidity mechanism provides inherent safety".to_string()
                );
                *confidence_score = 90;
                LpLockStatus::BondingCurve { dex: "PumpFun".to_string() }
            } else {
                details.push(
                    "PumpFun token on unidentified derivative DEX without lp_mint".to_string()
                );
                *confidence_score = 40;
                LpLockStatus::Unknown
            }
        }
        _ => {
            details.push(format!("Unknown mechanism for program: {}", program_kind.display_name()));
            LpLockStatus::Unknown
        }
    }
}

/// Handle scenario where no pools are found
fn handle_no_pools_scenario(token_mint: &str) -> LpLockAnalysis {
    let mut details = Vec::new();

    // Heuristic checks for bonding curve tokens
    if token_mint.len() == 44 {
        // Standard Solana mint address length
        if token_mint.chars().all(|c| c.is_alphanumeric()) {
            // Check for common bonding curve patterns
            if token_mint.ends_with("pump") || token_mint.contains("pump") {
                details.push("Token appears to be on PumpFun bonding curve".to_string());
                details.push("✅ Bonding curve tokens have no LP tokens to rug".to_string());

                return LpLockAnalysis {
                    token_mint: token_mint.to_string(),
                    pool_address: None,
                    dex_id: Some("pumpfun".to_string()),
                    lp_mint: None,
                    status: LpLockStatus::BondingCurve {
                        dex: "PumpFun".to_string(),
                    },
                    analyzed_at: Utc::now(),
                    lock_score: 90,
                    details,
                    data_source: "bonding_curve_heuristic".to_string(),
                    analyzed_pools: vec![],
                };
            }
        }
    }

    details.push("No liquidity pools found for token".to_string());
    details.push("Unable to determine liquidity mechanism".to_string());

    LpLockAnalysis {
        token_mint: token_mint.to_string(),
        pool_address: None,
        dex_id: None,
        lp_mint: None,
        status: LpLockStatus::NoPool,
        analyzed_at: Utc::now(),
        lock_score: 0,
        details,
        data_source: "no_pools_found".to_string(),
        analyzed_pools: vec![],
    }
}

/// Create analysis result for unknown status
fn create_unknown_analysis(token_mint: &str, pools: Vec<TokenPair>) -> LpLockAnalysis {
    let mut details = vec!["Unable to analyze any pools".to_string()];

    // Add pool info for debugging
    for (i, pool) in pools.iter().take(3).enumerate() {
        details.push(
            format!(
                "Pool {}: {} {} (${:.0} liquidity)",
                i + 1,
                pool.dex_id,
                safe_truncate(&pool.pair_address, 8),
                pool.liquidity
                    .as_ref()
                    .map(|l| l.usd)
                    .unwrap_or(0.0)
            )
        );
    }

    LpLockAnalysis {
        token_mint: token_mint.to_string(),
        pool_address: pools.first().map(|p| p.pair_address.clone()),
        dex_id: pools.first().map(|p| p.dex_id.clone()),
        lp_mint: None,
        status: LpLockStatus::Unknown,
        analyzed_at: Utc::now(),
        lock_score: 0,
        details,
        data_source: "failed_analysis".to_string(),
        analyzed_pools: vec![],
    }
}

/// Select the most reliable analysis from multiple pool analyses
fn select_best_analysis(
    token_mint: &str,
    pool_analyses: Vec<PoolAnalysis>,
    original_pools: Vec<TokenPair>
) -> LpLockAnalysis {
    // Sort by status priority and confidence score
    let mut sorted_analyses = pool_analyses.clone();
    sorted_analyses.sort_by(|a, b| {
        // First priority: status safety
        let a_safe = a.status.is_safe();
        let b_safe = b.status.is_safe();

        match (a_safe, b_safe) {
            (true, false) => std::cmp::Ordering::Greater,
            (false, true) => std::cmp::Ordering::Less,
            _ => {
                // Same safety level, compare by confidence
                a.confidence_score.cmp(&b.confidence_score)
            }
        }
    });

    let best_analysis = sorted_analyses.first().unwrap();

    // Calculate overall lock score
    let lock_score = if pool_analyses.is_empty() {
        0
    } else {
        let total_confidence: u32 = pool_analyses
            .iter()
            .map(|p| p.confidence_score as u32)
            .sum();
        let avg_confidence = total_confidence / (pool_analyses.len() as u32);

        // Boost score if multiple pools agree
        let safe_pool_count = pool_analyses
            .iter()
            .filter(|p| p.status.is_safe())
            .count();

        let consensus_boost = if safe_pool_count > 1 {
            10
        } else if safe_pool_count == 1 {
            5
        } else {
            0
        };

        std::cmp::min(100, avg_confidence + consensus_boost) as u8
    };

    // Compile comprehensive details
    let mut all_details = vec![
        format!(
            "Analyzed {} pools across {} DEXes",
            pool_analyses.len(),
            pool_analyses
                .iter()
                .map(|p| p.dex_id.as_str())
                .collect::<std::collections::HashSet<_>>()
                .len()
        ),
        format!(
            "Best result from: {} {}",
            best_analysis.dex_id,
            safe_truncate(&best_analysis.pool_address, 8)
        )
    ];

    // Add pool summaries
    for (i, analysis) in pool_analyses.iter().take(5).enumerate() {
        all_details.push(
            format!(
                "Pool {}: {} {} - {} ({}%)",
                i + 1,
                analysis.dex_id,
                safe_truncate(&analysis.pool_address, 8),
                analysis.status.description(),
                analysis.confidence_score
            )
        );
    }

    all_details.extend(best_analysis.details.clone());

    let mut final_status = best_analysis.status.clone();
    let mut final_details = all_details;
    let mut final_lock_score = lock_score;

    // Augment: if best status is BondingCurve or PositionNft, check base token mint burn to boost confidence
    match final_status {
        LpLockStatus::BondingCurve { .. } | LpLockStatus::PositionNft { .. } => {
            if let Ok(client) = std::panic::catch_unwind(|| get_rpc_client()) {
                if
                    let Ok(mint_info) = futures::executor::block_on(async {
                        get_mint_info(&client, token_mint).await
                    })
                {
                    if mint_info.mint_authority.is_none() {
                        final_details.push(
                            "Base token mint authority burned (✅ additional safety)".to_string()
                        );
                        final_lock_score = std::cmp::min(100, final_lock_score.saturating_add(8));
                    } else if let Some(auth) = mint_info.mint_authority.as_ref() {
                        final_details.push(
                            format!(
                                "Base token mint authority present: {}",
                                safe_truncate(auth, 12)
                            )
                        );
                    }
                } else {
                    final_details.push("Base token mint authority check failed".to_string());
                }
            }
        }
        _ => {}
    }

    LpLockAnalysis {
        token_mint: token_mint.to_string(),
        pool_address: Some(best_analysis.pool_address.clone()),
        dex_id: Some(best_analysis.dex_id.clone()),
        lp_mint: best_analysis.lp_mint.clone(),
        status: final_status,
        analyzed_at: Utc::now(),
        lock_score: final_lock_score,
        details: final_details,
        data_source: "comprehensive_multi_pool".to_string(),
        analyzed_pools: pool_analyses,
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

/// Check if an address is a burn address
fn is_burn_address(address: &str) -> bool {
    address == "11111111111111111111111111111111" || // System Program
        address == "1nc1nerator11111111111111111111111111111111" || // Incinerator
        address.starts_with("1111111") // Other burn patterns
}

/// Heuristic to determine if an address is likely a creator wallet
fn is_likely_creator_wallet(address: &str) -> bool {
    // Enhanced heuristics for creator wallet detection
    !is_known_lock_program(address).is_some() &&
        !is_burn_address(address) &&
        address != "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" &&
        !address.starts_with("So1") // Common SOL addresses
}

/// Cache management functions
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

async fn cache_lp_analysis(token_mint: &str, analysis: &LpLockAnalysis) {
    if let Ok(mut cache) = LP_LOCK_CACHE.write() {
        cache.insert(token_mint.to_string(), CachedLpLockAnalysis {
            analysis: analysis.clone(),
            cached_at: Utc::now(),
        });
    }
}

// LP Extractor implementations for different DEX programs

/// Extract LP mint from Raydium CPMM pool data
fn extract_raydium_cpmm_lp(data: &[u8], pool_address: &str) -> Result<Option<String>, String> {
    if let Some(pool_info) = RaydiumCpmmDecoder::decode_raydium_cpmm_pool(data, pool_address) {
        log(
            LogTag::Security,
            "LP_EXTRACTED",
            &format!(
                "Extracted CPMM LP mint {} from pool {}",
                safe_truncate(&pool_info.lp_mint.to_string(), 12),
                safe_truncate(pool_address, 8)
            )
        );
        return Ok(Some(pool_info.lp_mint.to_string()));
    }

    log(
        LogTag::Security,
        "LP_EXTRACT_FAILED",
        &format!("Could not decode Raydium CPMM pool: {}", safe_truncate(pool_address, 8))
    );
    Ok(None)
}

/// Extract LP mint from Raydium CLMM pool data
fn extract_raydium_clmm_lp(data: &[u8], _pool_address: &str) -> Result<Option<String>, String> {
    // CLMM pools use position NFTs instead of traditional LP tokens
    log(LogTag::Security, "LP_INFO", "Raydium CLMM uses position NFTs, not LP tokens");
    Ok(None)
}

/// Extract LP mint from Raydium Legacy AMM pool data
fn extract_raydium_legacy_lp(data: &[u8], pool_address: &str) -> Result<Option<String>, String> {
    // Raydium Legacy AMM (v4) pools have LP tokens for liquidity provision
    if data.len() < 752 {
        log(
            LogTag::Security,
            "LP_ERROR",
            &format!(
                "Raydium Legacy AMM pool data too short: {} bytes for {}",
                data.len(),
                safe_truncate(pool_address, 8)
            )
        );
        return Ok(None);
    }

    // Try common offsets for LP mint in Legacy AMM pools
    // Based on Raydium Legacy AMM structure, LP mint is typically near the beginning
    // after various flags, status fields, and before vault addresses
    let potential_offsets = [8, 40, 72, 136, 168]; // Common Pubkey positions in Anchor programs

    for offset in potential_offsets {
        if data.len() > offset + 32 {
            if let Some(potential_lp_mint_str) = read_pubkey_str(data, offset) {
                // Parse back to Pubkey for validation
                if let Ok(potential_lp_mint) = potential_lp_mint_str.parse::<Pubkey>() {
                    // Validate it's not a zero key or system program
                    if
                        potential_lp_mint != Pubkey::default() &&
                        potential_lp_mint.to_string() != "11111111111111111111111111111111"
                    {
                        // Additional validation - check if this looks like a reasonable mint
                        // by trying to distinguish it from vault addresses (which come later)
                        if offset < 150 {
                            // LP mint should be near the beginning of the struct
                            log(
                                LogTag::Security,
                                "LP_INFO",
                                &format!(
                                    "Found potential Raydium Legacy LP mint at offset {}: {}",
                                    offset,
                                    potential_lp_mint
                                )
                            );
                            return Ok(Some(potential_lp_mint.to_string()));
                        }
                    }
                }
            }
        }
    }

    log(
        LogTag::Security,
        "LP_WARNING",
        &format!(
            "Raydium Legacy AMM LP mint not found at common offsets for: {}",
            safe_truncate(pool_address, 8)
        )
    );
    Ok(None)
}

/// Extract LP mint from Orca Whirlpool pool data
fn extract_orca_whirlpool_lp(data: &[u8], _pool_address: &str) -> Result<Option<String>, String> {
    // Orca Whirlpools (CLMM) don't have traditional LP tokens
    log(LogTag::Security, "LP_INFO", "Orca Whirlpools use position NFTs, not LP tokens");
    Ok(None)
}

/// Extract LP mint from Meteora DLMM pool data
fn extract_meteora_dlmm_lp(data: &[u8], _pool_address: &str) -> Result<Option<String>, String> {
    // Meteora DLMM pools use a different mechanism
    log(
        LogTag::Security,
        "LP_INFO",
        "Meteora DLMM uses bin-based liquidity, not traditional LP tokens"
    );
    Ok(None)
}

/// Extract LP mint from Meteora DAMM pool data
fn extract_meteora_damm_lp(data: &[u8], pool_address: &str) -> Result<Option<String>, String> {
    // Meteora DAMM v2: no standard lp_mint field; prior heuristics produced random bytes misclassified as pubkeys.
    // We explicitly skip extraction to avoid false-positive mint verification failures.
    log(
        LogTag::Security,
        "LP_INFO",
        &format!(
            "Meteora DAMM v2 pool {}: skipping lp_mint extraction (dynamic position mechanism)",
            safe_truncate(pool_address, 8)
        )
    );
    Ok(None)
}

/// Extract LP mint from PumpFun AMM pool data
fn extract_pumpfun_amm_lp(data: &[u8], pool_address: &str) -> Result<Option<String>, String> {
    // PumpFun AMM: prior heuristic offsets produced random 32-byte slices misclassified as LP mints.
    // There is no reliably documented dedicated lp_mint; liquidity typically lives on external AMMs (Raydium) AFTER migration.
    // We deliberately skip extraction to avoid false Unknown due to mint parse failures.
    log(
        LogTag::Security,
        "LP_INFO",
        &format!(
            "PumpFun AMM pool {}: skipping lp_mint heuristic extraction (treat as bonding curve style if no traditional LP)",
            safe_truncate(pool_address, 8)
        )
    );
    Ok(None)
}

/// Extract LP mint from PumpFun Legacy pool data
fn extract_pumpfun_legacy_lp(data: &[u8], _pool_address: &str) -> Result<Option<String>, String> {
    // PumpFun bonding curve pools don't have LP tokens
    log(
        LogTag::Security,
        "LP_INFO",
        "PumpFun bonding curve detected - no LP tokens exist (inherently safe)"
    );
    Ok(None)
}

// Public API functions

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
                    "BATCH_ERROR",
                    &format!("Failed to analyze LP lock for FULL_MINT: {} - Error: {}", mint, e)
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

/// Get detailed lock analysis for debugging
pub async fn get_detailed_lp_analysis(token_mint: &str) -> Result<LpLockAnalysis, String> {
    // Force fresh analysis without cache
    check_lp_lock_status_with_cache(token_mint, false).await
}

/// Legacy compatibility - check if a token's liquidity is locked (simple boolean)
pub async fn is_liquidity_locked(token_mint: &str) -> Result<bool, String> {
    let analysis = check_lp_lock_status(token_mint).await?;
    Ok(analysis.status.is_safe())
}

/// Legacy LockPrograms struct for compatibility
pub struct LockPrograms;

impl LockPrograms {
    /// Get list of known lock/vesting program addresses
    pub fn known_programs() -> std::collections::HashMap<&'static str, &'static str> {
        KNOWN_LOCK_PROGRAMS.iter().cloned().collect()
    }

    /// Check if an address is a known lock program
    pub fn is_lock_program(address: &str) -> Option<&'static str> {
        is_known_lock_program(address)
    }
}

// Helper functions for binary data parsing

/// Read a Pubkey from binary data at the given offset
fn read_pubkey_str(data: &[u8], offset: usize) -> Option<String> {
    let bytes: [u8; 32] = data
        .get(offset..offset + 32)?
        .try_into()
        .ok()?;
    Some(Pubkey::new_from_array(bytes).to_string())
}
