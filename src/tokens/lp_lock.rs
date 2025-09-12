/// Liquidity Pool Lock Detection Module
///
/// This module provides functionality to detect whether a token's liquidity pool
/// is locked or not. It supports various AMM protocols on Solana, with primary
/// focus on Raydium pools.
///
/// Lock detection checks:
/// 1. Pool authority status - if LP token mint authority is None (burned)
/// 2. Lock programs - checks for common lock/vesting programs
/// 3. Burn validation - verifies LP tokens are actually burned
/// 4. Time-based locks - detects time-locked LP positions

use crate::arguments::is_debug_pool_discovery_enabled;
use crate::errors::ScreenerBotError;
use crate::logger::{ log, LogTag };
use crate::rpc::get_rpc_client;
use crate::tokens::holders::get_token_account_count_estimate;
use base64::Engine;
use chrono::{ DateTime, Utc };
use rusqlite::{ Connection, OptionalExtension };
use serde::{ Deserialize, Serialize };
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::{ SystemTime, UNIX_EPOCH };
use tokio::time::{ timeout, Duration };

/// Simple truncate function for addresses
fn truncate_address(address: &str, len: usize) -> String {
    if address.len() <= len { address.to_string() } else { format!("{}...", &address[..len]) }
}

/// Governance/DAO information for locked LP tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceInfo {
    /// Governance program used
    pub governance_program: String,
    /// DAO/Governance realm address
    pub governance_realm: Option<String>,
    /// Minimum time before changes can be made
    pub min_governance_delay: Option<u64>,
    /// Required proposal approval threshold
    pub approval_threshold: Option<f64>,
}

/// Liquidity pool lock status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LpLockStatus {
    /// LP tokens are burned (mint authority = None) - permanent lock
    Burned,
    /// LP tokens are locked in a time-based program
    TimeLocked {
        unlock_time: DateTime<Utc>,
        program: String,
    },
    /// LP tokens are held by a lock/vesting program
    ProgramLocked {
        program: String,
        amount: u64,
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
            LpLockStatus::ProgramLocked { program, .. } => {
                // Pump.fun bonding curves are considered relatively safe
                // as the liquidity is protocol-controlled
                program.contains("Pump.fun") ||
                    program.contains("Team Finance") ||
                    program.contains("Streamflow") ||
                    program.contains("Unvest")
            }
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
            LpLockStatus::ProgramLocked { program, .. } => {
                if program.contains("Pump.fun") {
                    "Bonding curve managed (protocol-controlled liquidity)"
                } else {
                    "LP tokens program-locked"
                }
            }
            LpLockStatus::CreatorHeld => "LP tokens held by creator (NOT LOCKED)",
            LpLockStatus::Unknown => "Lock status unknown",
            LpLockStatus::NoPool => "No liquidity pool found",
        }
    }

    /// Get risk level indicator
    pub fn risk_level(&self) -> &'static str {
        match self {
            LpLockStatus::Burned => "🟢 SAFE",
            LpLockStatus::TimeLocked { .. } => "🟡 LOCKED",
            LpLockStatus::ProgramLocked { .. } => "🟡 LOCKED",
            LpLockStatus::CreatorHeld => "🔴 RISKY",
            LpLockStatus::Unknown => "⚪ UNKNOWN",
            LpLockStatus::NoPool => "⚪ NO_POOL",
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
    /// LP token mint address (if found)
    pub lp_mint: Option<String>,
    /// Lock status
    pub status: LpLockStatus,
    /// Additional details about the analysis
    pub details: LpLockDetails,
    /// Analysis timestamp
    pub analyzed_at: DateTime<Utc>,
    /// Lock verification score (0-100, higher is more secure)
    pub lock_score: u8,
    /// Estimated locked liquidity in USD (if calculable)
    pub locked_liquidity_usd: Option<f64>,
}

/// Detailed information about LP lock analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LpLockDetails {
    /// Pool type detected (Raydium, Orca, etc.)
    pub pool_type: Option<String>,
    /// Total LP supply
    pub total_lp_supply: Option<u64>,
    /// LP tokens held by lock programs
    pub locked_lp_amount: u64,
    /// LP tokens held by creator/deployer
    pub creator_held_amount: u64,
    /// Known lock programs detected
    pub lock_programs: Vec<String>,
    /// Mint authority of LP token (None = burned)
    pub lp_mint_authority: Option<String>,
    /// Additional notes and warnings
    pub notes: Vec<String>,
    /// Burn verification status
    pub burn_verified: bool,
    /// Lock age in days (if determinable)
    pub lock_age_days: Option<u32>,
    /// Lock expiry information (for time-based locks)
    pub lock_expiry: Option<DateTime<Utc>>,
    /// Governance/DAO information (if applicable)
    pub governance_info: Option<GovernanceInfo>,
}

/// Known lock/vesting programs on Solana
pub struct LockPrograms;

impl LockPrograms {
    /// Get list of known lock/vesting program addresses
    pub fn known_programs() -> HashMap<&'static str, &'static str> {
        let mut programs = HashMap::new();

        // Common lock programs
        programs.insert("ADHS2E6D7YvbXF6y6EVPWyN6u6eDvAQj1QqRAEyrnKE4", "Team Finance Lock");
        programs.insert("6ebQNeTPZ1j7k3TtkCCtEPRvG7GQsucQrZ7sSEDQi9Ks", "Streamflow Lock");
        programs.insert("D3bbkYqzsE4gGzjJge15qRcEZQqg88qGd5TqE8mCw7Uv", "Streamflow Vesting");
        programs.insert("A9HAbnCwoD6f2NkZobKFf6buJoN9gUVVvX5PoUnDHS6u", "Unvest Lock");
        programs.insert("CrXKvzQ3LzCKMQnSwyqnFq3iA83ALGk5zcqRyTaFW7vK", "Realms Governance");
        programs.insert("GjWvbvfaJkiCNxtPrST6dGWdU2UuDkGp2VtznQyJQH7F", "Pinky Lock");
        programs.insert("7JXkQWfAXgXrJCL8pHFFGQn59VNv2H1rFbgYRH6Q6Q8Z", "Solana Lock");

        // Governance and DAO programs
        programs.insert("GovHgfDPyQ1GwazJTDY2avSVY8GGcpmCapmmCsymRaGe", "SPL Governance");
        programs.insert("gEyaSiRMG9P8JQJR6gg4W7Rf9PTKyqqtaDSzogpPfAF", "Governance V2");
        programs.insert("GqTPL6qRf5aUuqscLh8Rg2HTxPUXfhhAXDptTLhp1t2J", "DAO Governance");

        // Token vesting programs
        programs.insert("9HbJPTYi4uRdN5Jq7fVMPhtYhJhNaG3vxE5FCE1zrwgn", "Token Vesting");
        programs.insert("VestwqHo7CJLS8kzfFgp75zphpGy8RCeYBVDJEE3dLm", "Vesting Program");
        programs.insert("3AjCHHaWiPcNJoZuBfKWjnFZVaLUfNZw5DnV3TJV8zzP", "Linear Vesting");

        // Multisig programs that often hold LP tokens
        programs.insert("msigmtwzgXJHj2ext4XJjCDmpbcWUrbEyAr4XTmmRwW", "Multisig");
        programs.insert("SQDS4ep65T869zMMBKyuUq6aD6EgTu8psMjkvj52pCf", "Squads Multisig");

        // Timelock programs
        programs.insert("TLockEaFBt2gREyGSqSC1TvxiSABHYcnWWKeCRpgHBH", "Timelock");
        programs.insert("ELockBFuY3X3Vo1yf5jmzqN84rWqA9JwJ1cSA7iyF5x", "Extended Timelock");

        // Protocol-specific locks
        programs.insert("8tfDNiaEyrV6Q1U4DEXrEigs9DoDtkugzFbybENEbCDz", "Marinade Lock");
        programs.insert("PLockERGE1FPiDKrqKKkXiXYUzLXZ8VzNqkDX4j1w8Q", "Parrot Lock");

        programs
    }

    /// Check if an address is a known lock program
    pub fn is_lock_program(address: &str) -> Option<&'static str> {
        Self::known_programs().get(address).copied()
    }

    /// Get all lock program addresses
    pub fn all_addresses() -> Vec<&'static str> {
        Self::known_programs().keys().copied().collect()
    }

    /// Check if a program is time-based (has unlock dates)
    pub fn is_time_based_lock(program_name: &str) -> bool {
        let time_based_programs = vec![
            "Team Finance Lock",
            "Streamflow Lock",
            "Streamflow Vesting",
            "Unvest Lock",
            "Token Vesting",
            "Vesting Program",
            "Linear Vesting",
            "Timelock",
            "Extended Timelock"
        ];

        time_based_programs.iter().any(|&p| program_name.contains(p))
    }

    /// Check if a program is governance-based (DAO controlled)
    pub fn is_governance_lock(program_name: &str) -> bool {
        let governance_programs = vec![
            "Realms Governance",
            "SPL Governance",
            "Governance V2",
            "DAO Governance",
            "Multisig",
            "Squads Multisig"
        ];

        governance_programs.iter().any(|&p| program_name.contains(p))
    }
}

/// Check if a token's liquidity pool is locked
/// This is the main function that should be used everywhere
pub async fn check_lp_lock_status(token_mint: &str) -> Result<LpLockAnalysis, ScreenerBotError> {
    log(
        LogTag::Security,
        "DEBUG",
        &format!("Checking LP lock status for token {}", truncate_address(token_mint, 8))
    );

    // SAFETY CHECK: Pre-check token holder count using dataSlice to prevent hanging
    // Large tokens with millions of holders will cause RPC timeouts
    log(
        LogTag::Security,
        "DEBUG",
        &format!("Pre-checking token holder count for {}", truncate_address(token_mint, 8))
    );

    match get_token_account_count_estimate(token_mint).await {
        Ok(holder_count) => {
            const MAX_SAFE_HOLDERS: usize = 5000;
            if holder_count > MAX_SAFE_HOLDERS {
                log(
                    LogTag::Security,
                    "DEBUG",
                    &format!(
                        "Skipping LP analysis for token {} - {} holders exceeds maximum {} (prevents hanging)",
                        truncate_address(token_mint, 8),
                        holder_count,
                        MAX_SAFE_HOLDERS
                    )
                );

                return Ok(LpLockAnalysis {
                    token_mint: token_mint.to_string(),
                    pool_address: None,
                    lp_mint: None,
                    status: LpLockStatus::Unknown,
                    details: LpLockDetails {
                        pool_type: Some("Large Token".to_string()),
                        total_lp_supply: None,
                        locked_lp_amount: 0,
                        creator_held_amount: 0,
                        lock_programs: Vec::new(),
                        lp_mint_authority: None,
                        notes: vec![
                            format!("Analysis skipped - token has {} holders (too large, would cause RPC timeouts)", holder_count)
                        ],
                        burn_verified: false,
                        lock_age_days: None,
                        lock_expiry: None,
                        governance_info: None,
                    },
                    analyzed_at: Utc::now(),
                    lock_score: 50, // Neutral score for unknown status
                    locked_liquidity_usd: None,
                });
            }

            if crate::arguments::is_debug_security_enabled() {
                log(
                    LogTag::Security,
                    "DEBUG",
                    &format!(
                        "Token {} has {} holders - safe to analyze",
                        truncate_address(token_mint, 8),
                        holder_count
                    )
                );
            }
        }
        Err(e) => {
            if crate::arguments::is_debug_security_enabled() {
                log(
                    LogTag::Security,
                    "DEBUG",
                    &format!(
                        "Failed to get holder count for {}: {} - proceeding with caution",
                        truncate_address(token_mint, 8),
                        e
                    )
                );
            }
        }
    }

    let analysis_start = Utc::now();

    // Step 1: Find the liquidity pool for this token
    let pool_info = find_liquidity_pool(token_mint).await?;

    if pool_info.is_none() {
        if crate::arguments::is_debug_security_enabled() {
            log(
                LogTag::Security,
                "DEBUG",
                &format!("No liquidity pool found for token {}", truncate_address(token_mint, 8))
            );
        }

        return Ok(LpLockAnalysis {
            token_mint: token_mint.to_string(),
            pool_address: None,
            lp_mint: None,
            status: LpLockStatus::NoPool,
            details: LpLockDetails {
                pool_type: None,
                total_lp_supply: None,
                locked_lp_amount: 0,
                creator_held_amount: 0,
                lock_programs: Vec::new(),
                lp_mint_authority: None,
                notes: vec!["No liquidity pool found for this token".to_string()],
                burn_verified: false,
                lock_age_days: None,
                lock_expiry: None,
                governance_info: None,
            },
            analyzed_at: analysis_start,
            lock_score: 0,
            locked_liquidity_usd: None,
        });
    }

    let (pool_address, lp_mint, pool_type) = pool_info.unwrap();

    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "DEBUG",
            &format!(
                "Found {} pool {} with LP mint {} for token {}",
                pool_type,
                truncate_address(&pool_address, 8),
                truncate_address(&lp_mint, 8),
                truncate_address(token_mint, 8)
            )
        );
    }

    // Step 2: Analyze LP mint authority (skip for special cases)
    let lp_mint_analysis = if pool_type.contains("Pump.fun") {
        // Pump.fun doesn't have traditional LP mints
        LpMintAnalysis {
            mint_authority: None,
            supply: 0,
            is_burned: false, // Not applicable
        }
    } else {
        analyze_lp_mint_authority(&lp_mint).await?
    };

    // Step 3: Analyze LP token distribution (skip for special cases)
    let distribution_analysis = if pool_type.contains("Pump.fun") {
        // Pump.fun doesn't have traditional LP token distribution
        LpDistributionAnalysis {
            total_holders: 0,
            locked_amount: 0,
            creator_held_amount: 0,
            lock_programs: vec!["Pump.fun Bonding Curve".to_string()],
            largest_holders: Vec::new(),
        }
    } else {
        analyze_lp_distribution(&lp_mint).await?
    };

    // Step 4: Determine final lock status
    let status = determine_lock_status(&lp_mint_analysis, &distribution_analysis, &pool_type);

    let analysis = LpLockAnalysis {
        token_mint: token_mint.to_string(),
        pool_address: Some(pool_address.clone()),
        lp_mint: Some(lp_mint.clone()),
        status: status.clone(),
        details: merge_analysis_details(
            lp_mint_analysis.clone(),
            distribution_analysis.clone(),
            pool_type.clone()
        ),
        analyzed_at: analysis_start,
        lock_score: calculate_lock_score(&status, &lp_mint_analysis, &distribution_analysis),
        locked_liquidity_usd: None, // TODO: Calculate based on pool data
    };

    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "DEBUG",
            &format!(
                "LP lock analysis for {}: {} - {}",
                truncate_address(token_mint, 8),
                analysis.status.risk_level(),
                analysis.status.description()
            )
        );
    }

    Ok(analysis)
}

/// Find liquidity pool for a token (searches multiple DEXs)
async fn find_liquidity_pool(
    token_mint: &str
) -> Result<Option<(String, String, String)>, ScreenerBotError> {
    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "DEBUG",
            &format!(
                "Starting comprehensive pool search for token {}",
                truncate_address(token_mint, 8)
            )
        );
    }

    // Check if this might be a Pump.fun token based on the mint address pattern
    if token_mint.ends_with("pump") {
        if crate::arguments::is_debug_security_enabled() {
            log(
                LogTag::Security,
                "DEBUG",
                &format!(
                    "Token {} appears to be a Pump.fun token based on mint pattern",
                    truncate_address(token_mint, 8)
                )
            );
        }

        // For Pump.fun tokens, use a different approach
        // The "pool" is the bonding curve mechanism, not a traditional pool
        return Ok(
            Some((
                "pump_bonding_curve".to_string(), // Virtual pool address
                "pump_bonding_curve".to_string(), // Virtual LP mint
                "Pump.fun Bonding Curve".to_string(),
            ))
        );
    }

    // First try to find Raydium V4 pool
    if let Ok(Some(pool_info)) = search_dex_pool(token_mint, &PoolSearchConfig::raydium_v4()).await {
        if crate::arguments::is_debug_security_enabled() {
            log(LogTag::Security, "DEBUG", "Found Raydium V4 pool");
        }
        return Ok(Some(pool_info));
    }

    // Try Raydium CPMM pools
    if
        let Ok(Some(pool_info)) = search_dex_pool(
            token_mint,
            &PoolSearchConfig::raydium_cpmm()
        ).await
    {
        if crate::arguments::is_debug_security_enabled() {
            log(LogTag::Security, "DEBUG", "Found Raydium CPMM pool");
        }
        return Ok(Some(pool_info));
    }

    // Try Orca Whirlpools
    if
        let Ok(Some(pool_info)) = search_dex_pool(
            token_mint,
            &PoolSearchConfig::orca_whirlpool()
        ).await
    {
        if crate::arguments::is_debug_security_enabled() {
            log(LogTag::Security, "DEBUG", "Found Orca Whirlpool");
        }
        return Ok(Some(pool_info));
    }

    // Try Meteora DLMM pools
    if
        let Ok(Some(pool_info)) = search_dex_pool(
            token_mint,
            &PoolSearchConfig::meteora_dlmm()
        ).await
    {
        if crate::arguments::is_debug_security_enabled() {
            log(LogTag::Security, "DEBUG", "Found Meteora DLMM pool");
        }
        return Ok(Some(pool_info));
    }

    // Try Meteora DAMM v2 pools
    if
        let Ok(Some(pool_info)) = search_dex_pool(
            token_mint,
            &PoolSearchConfig::meteora_damm_v2()
        ).await
    {
        if crate::arguments::is_debug_security_enabled() {
            log(LogTag::Security, "DEBUG", "Found Meteora DAMM v2 pool");
        }
        return Ok(Some(pool_info));
    }

    // Try Meteora Pools (legacy)
    if
        let Ok(Some(pool_info)) = search_dex_pool(
            token_mint,
            &PoolSearchConfig::meteora_pools()
        ).await
    {
        if crate::arguments::is_debug_security_enabled() {
            log(LogTag::Security, "DEBUG", "Found Meteora legacy pool");
        }
        return Ok(Some(pool_info));
    }

    // Try Pump.fun AMM pools
    if
        let Ok(Some(pool_info)) = search_dex_pool(
            token_mint,
            &PoolSearchConfig::pumpfun_amm()
        ).await
    {
        if crate::arguments::is_debug_security_enabled() {
            log(LogTag::Security, "DEBUG", "Found Pump.fun AMM pool");
        }
        return Ok(Some(pool_info));
    }

    if crate::arguments::is_debug_security_enabled() {
        log(
            LogTag::Security,
            "DEBUG",
            &format!(
                "No liquidity pool found for token {} across all supported DEXs",
                truncate_address(token_mint, 8)
            )
        );
    }

    Ok(None)
}

/// Pool search configuration for different DEXs
#[derive(Debug, Clone)]
struct PoolSearchConfig {
    program_id: &'static str,
    pool_name: &'static str,
    data_size: u64,
    token_a_offset: u64,
    token_b_offset: u64,
    lp_extraction_method: LpExtractionMethod,
}

/// Method for extracting LP mint from pool data
#[derive(Debug, Clone)]
enum LpExtractionMethod {
    /// Extract from pool account data at specific offset
    FromPoolData {
        offset: usize,
    },
    /// Derive using PDA from pool address
    DerivePda {
        seeds: &'static [&'static [u8]],
    },
    /// Use pool address as LP mint (for special cases)
    UsePoolAddress,
}

/// Pool search configurations for all supported DEXs
impl PoolSearchConfig {
    fn raydium_v4() -> Self {
        Self {
            program_id: "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",
            pool_name: "Raydium V4",
            data_size: 752,
            token_a_offset: 400,
            token_b_offset: 432,
            lp_extraction_method: LpExtractionMethod::FromPoolData { offset: 112 },
        }
    }

    fn raydium_cpmm() -> Self {
        Self {
            program_id: "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",
            pool_name: "Raydium CPMM",
            data_size: 637,
            token_a_offset: 168, // 8 + 32*5 (token_0_mint position)
            token_b_offset: 200, // 8 + 32*6 (token_1_mint position)
            lp_extraction_method: LpExtractionMethod::FromPoolData { offset: 136 }, // 8 + 32*4 (lp_mint position)
        }
    }

    fn orca_whirlpool() -> Self {
        Self {
            program_id: "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc",
            pool_name: "Orca Whirlpool",
            data_size: 653,
            token_a_offset: 101,
            token_b_offset: 181,
            lp_extraction_method: LpExtractionMethod::UsePoolAddress,
        }
    }

    fn meteora_dlmm() -> Self {
        Self {
            program_id: "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo",
            pool_name: "Meteora DLMM",
            data_size: 1544,
            token_a_offset: 73,
            token_b_offset: 105,
            lp_extraction_method: LpExtractionMethod::DerivePda {
                seeds: &[b"lp_mint"],
            },
        }
    }

    fn meteora_damm_v2() -> Self {
        Self {
            program_id: "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG",
            pool_name: "Meteora DAMM v2",
            data_size: 1112, // From the account info we just fetched
            token_a_offset: 168, // Found via account analysis
            token_b_offset: 200, // Found via account analysis
            lp_extraction_method: LpExtractionMethod::DerivePda {
                seeds: &[b"lp_mint"],
            },
        }
    }

    fn meteora_pools() -> Self {
        Self {
            program_id: "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB",
            pool_name: "Meteora Pools",
            data_size: 1544,
            token_a_offset: 73,
            token_b_offset: 105,
            lp_extraction_method: LpExtractionMethod::DerivePda {
                seeds: &[b"lp_mint"],
            },
        }
    }

    fn pumpfun_amm() -> Self {
        Self {
            program_id: "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
            pool_name: "Pump.fun AMM",
            data_size: 300, // Exact size to reduce search scope and avoid pagination issues
            token_a_offset: 43, // SOL mint position (verified from pool analysis)
            token_b_offset: 75, // Token mint position (verified from pool analysis)
            lp_extraction_method: LpExtractionMethod::FromPoolData { offset: 107 }, // LP mint position (verified)
        }
    }
}

/// Generic pool search function that eliminates code duplication
async fn search_dex_pool(
    token_mint: &str,
    config: &PoolSearchConfig
) -> Result<Option<(String, String, String)>, ScreenerBotError> {
    let rpc_client = get_rpc_client();

    if is_debug_pool_discovery_enabled() {
        log(
            LogTag::Rpc,
            "DEX_SEARCH",
            &format!(
                "Searching {} pools for token {}",
                config.pool_name,
                truncate_address(token_mint, 8)
            )
        );
    }

    // Try both token positions (A and B)
    let positions = [
        ("token_a", config.token_a_offset),
        ("token_b", config.token_b_offset),
    ];

    for (position_name, offset) in positions {
        let filters = create_search_filters(config.data_size, offset, token_mint);

        // Check if this is a large program that needs pagination (Pump.fun AMM and Meteora DAMM v2)
        let needs_pagination = match config.program_id {
            "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" => true, // Pump.fun AMM
            "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG" => true, // Meteora DAMM v2
            "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => true, // Meteora DLMM (might be large)
            _ => false, // Use regular getProgramAccounts for smaller programs
        };

        let accounts = if needs_pagination {
            // Use getProgramAccountsV2 for large programs
            if is_debug_pool_discovery_enabled() {
                log(
                    LogTag::Rpc,
                    "DEBUG",
                    &format!("Using paginated search for {} (large program)", config.pool_name)
                );
            }

            match
                rpc_client.get_all_program_accounts_v2(
                    config.program_id,
                    Some(filters),
                    Some("base64"),
                    None, // No data slice needed since we have exact filters
                    Some(1000), // Batch size
                    Some(60) // Timeout
                ).await
            {
                Ok(accounts) => accounts,
                Err(e) => {
                    log(
                        LogTag::Rpc,
                        "DEX_SEARCH_ERROR",
                        &format!(
                            "Error searching {} {} position with pagination: {}",
                            config.pool_name,
                            position_name,
                            e
                        )
                    );
                    continue;
                }
            }
        } else {
            // Use regular getProgramAccounts for smaller programs
            match
                rpc_client.get_program_accounts(
                    config.program_id,
                    Some(filters),
                    Some("base64"),
                    Some(30)
                ).await
            {
                Ok(accounts) => accounts,
                Err(e) => {
                    log(
                        LogTag::Rpc,
                        "DEX_SEARCH_ERROR",
                        &format!(
                            "Error searching {} {} position: {}",
                            config.pool_name,
                            position_name,
                            e
                        )
                    );
                    continue;
                }
            }
        };

        if let Some(account) = accounts.first() {
            if let Some(pool_address) = account.get("pubkey").and_then(|v| v.as_str()) {
                if
                    let Some(lp_mint) = extract_lp_mint(
                        pool_address,
                        account,
                        &config.lp_extraction_method,
                        config.program_id
                    ).await
                {
                    log(
                        LogTag::Rpc,
                        "POOL_FOUND",
                        &format!(
                            "Found {} pool at {} ({}) {}",
                            config.pool_name,
                            truncate_address(pool_address, 8),
                            position_name,
                            if needs_pagination {
                                "(paginated)"
                            } else {
                                ""
                            }
                        )
                    );
                    return Ok(
                        Some((pool_address.to_string(), lp_mint, config.pool_name.to_string()))
                    );
                }
            }
        }
    }

    Ok(None)
}

/// Create search filters for pool discovery
fn create_search_filters(data_size: u64, token_offset: u64, token_mint: &str) -> serde_json::Value {
    serde_json::json!([
        {
            "dataSize": data_size
        },
        {
            "memcmp": {
                "offset": token_offset,
                "bytes": token_mint
            }
        }
    ])
}

/// Extract LP mint using the specified method
async fn extract_lp_mint(
    pool_address: &str,
    account_data: &serde_json::Value,
    method: &LpExtractionMethod,
    program_id: &str
) -> Option<String> {
    match method {
        LpExtractionMethod::FromPoolData { offset } => {
            extract_lp_mint_from_data(account_data, *offset)
        }
        LpExtractionMethod::DerivePda { seeds } => {
            derive_lp_mint_pda(pool_address, seeds, program_id).await
        }
        LpExtractionMethod::UsePoolAddress => { Some(pool_address.to_string()) }
    }
}

/// Extract LP mint from pool account data at specific offset
fn extract_lp_mint_from_data(account: &serde_json::Value, offset: usize) -> Option<String> {
    if let Some(data) = account.get("account")?.get("data")?.as_array() {
        if data.len() >= 2 {
            if let Some(data_str) = data[0].as_str() {
                if let Ok(decoded) = base64::engine::general_purpose::STANDARD.decode(data_str) {
                    if decoded.len() >= offset + 32 {
                        let lp_mint_bytes = &decoded[offset..offset + 32];
                        let lp_mint = bs58::encode(lp_mint_bytes).into_string();
                        return Some(lp_mint);
                    }
                }
            }
        }
    }
    None
}

/// Derive LP mint using PDA
async fn derive_lp_mint_pda(
    pool_address: &str,
    seeds: &[&[u8]],
    program_id: &str
) -> Option<String> {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    let pool_pubkey = Pubkey::from_str(pool_address).ok()?;
    let program_pubkey = Pubkey::from_str(program_id).ok()?;

    // Build seed vector with pool address
    let mut seed_vec: Vec<&[u8]> = seeds.to_vec();
    seed_vec.push(pool_pubkey.as_ref());

    let (lp_mint_pda, _) = Pubkey::find_program_address(&seed_vec, &program_pubkey);
    Some(lp_mint_pda.to_string())
}
/// Derive LP mint for CPMM pool (it's a PDA derived from pool address)
async fn derive_cpmm_lp_mint(pool_address: &str) -> Option<String> {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    if let Ok(pool_pubkey) = Pubkey::from_str(pool_address) {
        // CPMM LP mint is derived as PDA from pool address
        let (lp_mint_pda, _) = Pubkey::find_program_address(
            &[b"lp_mint", pool_pubkey.as_ref()],
            &Pubkey::from_str("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C").ok()?
        );
        return Some(lp_mint_pda.to_string());
    }
    None
}

/// Derive Orca position mint (Orca uses NFT positions instead of traditional LP tokens)
async fn derive_orca_position_mint(pool_address: &str) -> Option<String> {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    if let Ok(pool_pubkey) = Pubkey::from_str(pool_address) {
        // Orca uses position NFTs - for now return the pool address as a placeholder
        // In practice, you'd need to find the position NFT mint for this pool
        // This is more complex as Orca uses concentrated liquidity with NFT positions
        return Some(pool_address.to_string());
    }
    None
}

/// Derive Meteora LP mint for different Meteora program versions
async fn derive_meteora_lp_mint(pool_address: &str) -> Option<String> {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    if let Ok(pool_pubkey) = Pubkey::from_str(pool_address) {
        // Try all three Meteora program IDs
        let program_ids = [
            "cpamdpZCGKUy5JxQXB4dcpGPiikHawvSWAd6mEn1sGG", // DAMM v2
            "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo", // DLMM
            "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB", // Legacy Pools
        ];

        for program_id in &program_ids {
            if let Ok(program_pubkey) = Pubkey::from_str(program_id) {
                let (lp_mint_pda, _) = Pubkey::find_program_address(
                    &[b"lp_mint", pool_pubkey.as_ref()],
                    &program_pubkey
                );

                // For now, return the first derivation
                // In practice, we should check which program actually owns the pool
                return Some(lp_mint_pda.to_string());
            }
        }
    }
    None
}

/// Analyze LP mint authority to check if it's burned
async fn analyze_lp_mint_authority(lp_mint: &str) -> Result<LpMintAnalysis, ScreenerBotError> {
    let rpc_client = get_rpc_client();

    let mint_data = rpc_client.get_mint_account(lp_mint).await?;

    let mint_authority = mint_data
        .get("value")
        .and_then(|v| v.get("data"))
        .and_then(|d| d.get("parsed"))
        .and_then(|p| p.get("info"))
        .and_then(|i| i.get("mintAuthority"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let supply = mint_data
        .get("value")
        .and_then(|v| v.get("data"))
        .and_then(|d| d.get("parsed"))
        .and_then(|p| p.get("info"))
        .and_then(|i| i.get("supply"))
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    Ok(LpMintAnalysis {
        mint_authority: mint_authority.clone(),
        supply,
        is_burned: mint_authority.is_none(),
    })
}

/// Analyze LP token distribution to detect locks
async fn analyze_lp_distribution(
    lp_mint: &str
) -> Result<LpDistributionAnalysis, ScreenerBotError> {
    let rpc_client = get_rpc_client();

    // FIRST: Pre-check the number of LP token accounts to prevent RPC timeouts
    log(
        LogTag::Rpc,
        "LP_PRECHECK",
        &format!("Pre-checking LP token account count for {}", truncate_address(lp_mint, 8))
    );

    // Use dataSlice to efficiently count LP token accounts without downloading data
    let count_filters =
        serde_json::json!([
        {
            "dataSize": 165
        },
        {
            "memcmp": {
                "offset": 0,
                "bytes": lp_mint
            }
        }
    ]);

    let data_slice = serde_json::json!({
        "offset": 0,
        "length": 0
    });

    // Get account count using dataSlice optimization
    let account_count = match
        rpc_client.get_program_accounts_with_dateslice(
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
            Some(count_filters.clone()),
            Some("base64"),
            Some(data_slice),
            Some(10) // Short timeout for counting
        ).await
    {
        Ok(accounts) => accounts.len(),
        Err(e) => {
            log(
                LogTag::Rpc,
                "LP_PRECHECK_ERROR",
                &format!("Failed to count LP accounts for {}: {}", truncate_address(lp_mint, 8), e)
            );
            // Return safe defaults if count fails
            return Ok(LpDistributionAnalysis {
                total_holders: 0,
                locked_amount: 0,
                creator_held_amount: 0,
                lock_programs: Vec::new(),
                largest_holders: Vec::new(),
            });
        }
    };

    log(
        LogTag::Rpc,
        "LP_PRECHECK",
        &format!("LP token {} has {} accounts", truncate_address(lp_mint, 8), account_count)
    );

    // SAFETY CHECK: Skip analysis for tokens with too many LP holders (>1000)
    // Large tokens like RAY, SOL have thousands of LP holders and will timeout
    const MAX_LP_ACCOUNTS: usize = 1000;
    if account_count > MAX_LP_ACCOUNTS {
        log(
            LogTag::Rpc,
            "LP_SKIP_LARGE",
            &format!(
                "Skipping LP analysis for {} - {} accounts exceeds maximum {} (prevents hanging)",
                truncate_address(lp_mint, 8),
                account_count,
                MAX_LP_ACCOUNTS
            )
        );

        // Return safe analysis for large tokens - assume they're liquid and not locked
        return Ok(LpDistributionAnalysis {
            total_holders: account_count,
            locked_amount: 0,
            creator_held_amount: 0,
            lock_programs: Vec::new(),
            largest_holders: Vec::new(),
        });
    }

    // PROCEED: Safe to analyze - fetch full account data
    log(
        LogTag::Rpc,
        "LP_ANALYZE",
        &format!(
            "Analyzing LP distribution for {} ({} accounts)",
            truncate_address(lp_mint, 8),
            account_count
        )
    );

    let accounts = rpc_client.get_program_accounts(
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
        Some(count_filters),
        Some("jsonParsed"),
        Some(30) // Reduced timeout since we've already pre-checked account count
    ).await?;

    let mut locked_amount = 0u64;
    let mut creator_held_amount = 0u64;
    let mut lock_programs = Vec::new();
    let mut holders = Vec::new();

    for account in &accounts {
        if let Some(account_data) = account.get("account") {
            if let Some(data) = account_data.get("data") {
                if let Some(parsed) = data.get("parsed") {
                    if let Some(info) = parsed.get("info") {
                        if let Some(owner) = info.get("owner").and_then(|v| v.as_str()) {
                            if let Some(token_amount) = info.get("tokenAmount") {
                                if
                                    let Some(amount_str) = token_amount
                                        .get("amount")
                                        .and_then(|v| v.as_str())
                                {
                                    if let Ok(amount) = amount_str.parse::<u64>() {
                                        if amount > 0 {
                                            // Check if owner is a known lock program
                                            if
                                                let Some(program_name) =
                                                    LockPrograms::is_lock_program(owner)
                                            {
                                                locked_amount += amount;
                                                if
                                                    !lock_programs.contains(
                                                        &program_name.to_string()
                                                    )
                                                {
                                                    lock_programs.push(program_name.to_string());
                                                }
                                            } else {
                                                // Assume non-lock-program holders are creators/deployers
                                                creator_held_amount += amount;
                                            }

                                            holders.push((owner.to_string(), amount));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(LpDistributionAnalysis {
        total_holders: holders.len(),
        locked_amount,
        creator_held_amount,
        lock_programs,
        largest_holders: {
            holders.sort_by(|a, b| b.1.cmp(&a.1));
            holders.into_iter().take(5).collect()
        },
    })
}

/// Determine final lock status based on analyses
fn determine_lock_status(
    mint_analysis: &LpMintAnalysis,
    distribution_analysis: &LpDistributionAnalysis,
    pool_type: &str
) -> LpLockStatus {
    // Special handling for Pump.fun bonding curves
    if pool_type.contains("Pump.fun") {
        // Pump.fun bonding curves don't have traditional LP tokens
        // The liquidity is managed by the bonding curve contract itself
        // This can be considered "locked" in the sense that it's not rugpullable
        return LpLockStatus::ProgramLocked {
            program: "Pump.fun Bonding Curve".to_string(),
            amount: 0, // No traditional LP amount
        };
    }

    // Special handling for Orca Whirlpools (NFT-based positions)
    if pool_type.contains("Orca") {
        // Orca uses NFT positions instead of traditional LP tokens
        // For now, we consider Orca pools as relatively safe due to protocol design
        // but this needs more sophisticated NFT position analysis
        return LpLockStatus::ProgramLocked {
            program: "Orca Whirlpool (NFT Positions)".to_string(),
            amount: 0, // NFT-based, no traditional amount
        };
    }

    // Traditional LP token analysis for Raydium, Meteora, etc.

    // Priority 1: If mint authority is burned AND supply is zero, it's permanently locked
    if mint_analysis.is_burned {
        // Additional verification: check if supply is actually zero (proper burn)
        if mint_analysis.supply == 0 {
            return LpLockStatus::Burned;
        } else {
            // Mint authority burned but supply exists - still safe but different mechanism
            return LpLockStatus::Burned;
        }
    }

    // Priority 2: Check for time-based lock programs with detailed analysis
    if !distribution_analysis.lock_programs.is_empty() && distribution_analysis.locked_amount > 0 {
        // Check if any of the lock programs are time-based
        let time_lock_programs = vec![
            "Team Finance Lock",
            "Streamflow Lock",
            "Streamflow Vesting",
            "Unvest Lock"
        ];

        for program in &distribution_analysis.lock_programs {
            if time_lock_programs.iter().any(|&p| program.contains(p)) {
                // For time-based locks, we'd need to parse the actual lock account data
                // to get the unlock time. For now, return as generic time lock
                // TODO: Implement specific lock program data parsing
                return LpLockStatus::TimeLocked {
                    unlock_time: chrono::Utc::now() + chrono::Duration::days(365), // Placeholder
                    program: program.clone(),
                };
            }
        }

        // General program lock (governance, DAO, etc.)
        return LpLockStatus::ProgramLocked {
            program: distribution_analysis.lock_programs.join(", "),
            amount: distribution_analysis.locked_amount,
        };
    }

    // Priority 3: If LP tokens are held by creator/deployer, not locked
    if distribution_analysis.creator_held_amount > 0 {
        return LpLockStatus::CreatorHeld;
    }

    // Priority 4: Check for zero supply (burned without setting authority to None)
    if mint_analysis.supply == 0 {
        return LpLockStatus::Burned;
    }

    // Default: Unknown status
    LpLockStatus::Unknown
}

/// Merge analysis details from different checks
fn merge_analysis_details(
    mint_analysis: LpMintAnalysis,
    distribution_analysis: LpDistributionAnalysis,
    pool_type: String
) -> LpLockDetails {
    let mut notes = Vec::new();

    // Burn verification with detailed analysis
    let burn_verified = if mint_analysis.is_burned {
        if mint_analysis.supply == 0 {
            notes.push(
                "LP mint authority burned and total supply is zero (fully verified burn)".to_string()
            );
            true
        } else {
            notes.push(
                format!(
                    "LP mint authority burned but supply exists: {} (partial burn)",
                    mint_analysis.supply
                )
            );
            true // Still considered burned since authority is None
        }
    } else if mint_analysis.supply == 0 {
        notes.push("LP total supply is zero but mint authority exists (supply burned)".to_string());
        true
    } else if let Some(ref authority) = mint_analysis.mint_authority {
        notes.push(format!("LP mint authority: {} (NOT BURNED)", truncate_address(authority, 8)));
        false
    } else {
        false
    };

    // Analyze lock programs with categorization
    let mut governance_info = None;
    if !distribution_analysis.lock_programs.is_empty() {
        for program in &distribution_analysis.lock_programs {
            if LockPrograms::is_governance_lock(program) {
                governance_info = Some(GovernanceInfo {
                    governance_program: program.clone(),
                    governance_realm: None, // TODO: Extract from account data
                    min_governance_delay: None,
                    approval_threshold: None,
                });
                notes.push(format!("Governance lock detected: {}", program));
            } else if LockPrograms::is_time_based_lock(program) {
                notes.push(format!("Time-based lock detected: {}", program));
            } else {
                notes.push(format!("Program lock detected: {}", program));
            }
        }

        notes.push(
            format!("Total locked amount: {} LP tokens", distribution_analysis.locked_amount)
        );
    }

    // Analyze holder distribution
    if distribution_analysis.total_holders == 0 {
        notes.push("No LP token holders found".to_string());
    } else {
        notes.push(format!("Total LP holders: {}", distribution_analysis.total_holders));

        if distribution_analysis.creator_held_amount > 0 {
            let creator_percentage = if mint_analysis.supply > 0 {
                ((distribution_analysis.creator_held_amount as f64) /
                    (mint_analysis.supply as f64)) *
                    100.0
            } else {
                0.0
            };
            notes.push(format!("Creator holds {:.1}% of LP tokens", creator_percentage));
        }
    }

    // Add pool-specific notes
    match pool_type.as_str() {
        t if t.contains("Pump.fun") => {
            notes.push("Pump.fun bonding curve - liquidity managed by protocol".to_string());
        }
        t if t.contains("Orca") => {
            notes.push(
                "Orca Whirlpool - uses NFT positions instead of traditional LP tokens".to_string()
            );
        }
        t if t.contains("Raydium") => {
            notes.push("Raydium pool - traditional AMM liquidity".to_string());
        }
        _ => {}
    }

    LpLockDetails {
        pool_type: Some(pool_type),
        total_lp_supply: Some(mint_analysis.supply),
        locked_lp_amount: distribution_analysis.locked_amount,
        creator_held_amount: distribution_analysis.creator_held_amount,
        lock_programs: distribution_analysis.lock_programs,
        lp_mint_authority: mint_analysis.mint_authority,
        notes,
        burn_verified,
        lock_age_days: None, // TODO: Calculate from on-chain data
        lock_expiry: None, // TODO: Parse from lock program data
        governance_info,
    }
}

/// Calculate lock security score (0-100, higher is more secure)
fn calculate_lock_score(
    status: &LpLockStatus,
    mint_analysis: &LpMintAnalysis,
    distribution_analysis: &LpDistributionAnalysis
) -> u8 {
    let mut score = 0u8;

    // Base score based on lock status
    match status {
        LpLockStatus::Burned => {
            score += 90; // Highest security
            // Bonus for verified zero supply
            if mint_analysis.supply == 0 {
                score += 10;
            }
        }
        LpLockStatus::TimeLocked { .. } => {
            score += 70; // Good security, but time-limited
        }
        LpLockStatus::ProgramLocked { program, .. } => {
            if program.contains("Governance") || program.contains("DAO") {
                score += 75; // Governance locks are quite secure
            } else if program.contains("Pump.fun") || program.contains("Orca") {
                score += 65; // Protocol locks are reasonably secure
            } else {
                score += 60; // Generic program locks
            }
        }
        LpLockStatus::CreatorHeld => {
            score += 10; // Very low security
        }
        LpLockStatus::Unknown => {
            score += 5; // Minimal security
        }
        LpLockStatus::NoPool => {
            score += 0; // No security (no pool exists)
        }
    }

    // Adjustments based on distribution
    if distribution_analysis.locked_amount > distribution_analysis.creator_held_amount {
        score = score.saturating_add(5); // More locked than creator-held is better
    }

    if distribution_analysis.total_holders > 10 {
        score = score.saturating_add(3); // More holders generally better
    }

    // Ensure score doesn't exceed 100
    score.min(100)
}

/// Internal structure for LP mint analysis
#[derive(Debug, Clone)]
struct LpMintAnalysis {
    mint_authority: Option<String>,
    supply: u64,
    is_burned: bool,
}

/// Internal structure for LP distribution analysis
#[derive(Debug, Clone)]
struct LpDistributionAnalysis {
    total_holders: usize,
    locked_amount: u64,
    creator_held_amount: u64,
    lock_programs: Vec<String>,
    largest_holders: Vec<(String, u64)>,
}

/// Batch check LP lock status for multiple tokens
/// This is more efficient than checking them one by one
pub async fn check_multiple_lp_locks(
    token_mints: &[String]
) -> Result<Vec<LpLockAnalysis>, ScreenerBotError> {
    if token_mints.is_empty() {
        return Ok(Vec::new());
    }

    log(
        LogTag::Rpc,
        "LP_LOCK_BATCH",
        &format!("Checking LP lock status for {} tokens", token_mints.len())
    );

    let mut results = Vec::with_capacity(token_mints.len());
    let mut successful_checks = 0;
    let mut failed_checks = 0;

    // Process tokens in small batches to avoid overwhelming the RPC
    const BATCH_SIZE: usize = 5; // Smaller batch for LP checks as they're more intensive
    for chunk in token_mints.chunks(BATCH_SIZE) {
        let mut batch_futures = Vec::new();

        for mint in chunk {
            batch_futures.push(check_lp_lock_status(mint));
        }

        // Execute batch concurrently
        let batch_results = futures::future::join_all(batch_futures).await;

        for (i, result) in batch_results.into_iter().enumerate() {
            match result {
                Ok(analysis) => {
                    results.push(analysis);
                    successful_checks += 1;
                }
                Err(e) => {
                    failed_checks += 1;
                    log(
                        LogTag::Rpc,
                        "LP_LOCK_ERROR",
                        &format!(
                            "Failed to check LP lock for {}: {}",
                            truncate_address(&chunk[i], 8),
                            e
                        )
                    );
                }
            }
        }

        // Small delay between batches to be respectful to RPC
        if token_mints.len() > BATCH_SIZE {
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
    }

    log(
        LogTag::Rpc,
        "LP_LOCK_BATCH_COMPLETE",
        &format!(
            "LP lock batch check complete: {}/{} successful",
            successful_checks,
            successful_checks + failed_checks
        )
    );

    Ok(results)
}

/// Quick check if a token's LP is considered safe
/// This is a convenience function for quick safety checks
pub async fn is_lp_safe(token_mint: &str) -> Result<bool, ScreenerBotError> {
    let analysis = check_lp_lock_status(token_mint).await?;
    Ok(analysis.status.is_safe())
}

/// Validate if a string is a valid Solana address
fn is_valid_solana_address(address: &str) -> bool {
    // Basic validation - Solana addresses are base58 encoded and 32-44 characters
    if address.len() < 32 || address.len() > 44 {
        return false;
    }

    // Check if all characters are valid base58
    const BASE58_CHARS: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
    address.chars().all(|c| BASE58_CHARS.contains(&(c as u8)))
}

/// Extract numeric value from time string (e.g., "30 days" -> 30)
fn extract_time_value(time_str: &str) -> Option<u64> {
    let parts: Vec<&str> = time_str.trim().split_whitespace().collect();
    if let Some(first_part) = parts.first() {
        first_part.parse::<u64>().ok()
    } else {
        None
    }
}

/// Convert time description to days for comparison
fn parse_lock_duration_to_days(duration_str: &str) -> Option<u64> {
    let lower = duration_str.to_lowercase();
    let value = extract_time_value(&lower)?;

    if lower.contains("year") {
        Some(value * 365)
    } else if lower.contains("month") {
        Some(value * 30)
    } else if lower.contains("week") {
        Some(value * 7)
    } else if lower.contains("day") {
        Some(value)
    } else if lower.contains("hour") {
        Some(value / 24)
    } else {
        None
    }
}

/// Check if an account is likely a multisig based on its program owner
async fn is_multisig_account(account_address: &str) -> Result<bool, ScreenerBotError> {
    // Known multisig program IDs
    const MULTISIG_PROGRAMS: &[&str] = &[
        "msigmtwzgXJHj2ext4XJjCDmpbRmGGPAZ3KHPgKC82A", // Multisig
        "2ZhW9kJNxSWdXq8bPLjzx9s7Zx2PVK3C5hfG9v3C2XY", // Squads v3
        "SMPLecH534NA9acpos4G6x7uf3LWbCAwZQE9e8ZekMu", // Squads v4
    ];

    let rpc_client = get_rpc_client();

    match
        rpc_client.get_account(
            &Pubkey::from_str(account_address).map_err(|_| {
                ScreenerBotError::api_error("Invalid account address")
            })?
        ).await
    {
        Ok(account) => {
            let owner_str = account.owner.to_string();
            Ok(MULTISIG_PROGRAMS.contains(&owner_str.as_str()))
        }
        Err(_) => Ok(false), // If we can't fetch the account, assume it's not a multisig
    }
}

/// Enhanced validation for LP lock analysis results
impl LpLockAnalysis {
    /// Validate the consistency of the analysis results
    pub fn validate(&self) -> Result<(), String> {
        // Check if lock score is within valid range
        if self.lock_score > 100 {
            return Err("Lock score cannot exceed 100".to_string());
        }

        // Validate consistency between status and score
        match &self.status {
            LpLockStatus::Burned => {
                if self.lock_score < 80 {
                    return Err("Burned LP should have high lock score".to_string());
                }
            }
            LpLockStatus::CreatorHeld => {
                if self.lock_score > 30 {
                    return Err("Creator held LP should have low lock score".to_string());
                }
            }
            LpLockStatus::NoPool => {
                if self.lock_score > 10 {
                    return Err("No pool should have very low lock score".to_string());
                }
            }
            _ => {} // Other statuses can have varying scores
        }

        // Validate pool address if provided
        if let Some(pool_addr) = &self.pool_address {
            if !is_valid_solana_address(pool_addr) {
                return Err("Invalid pool address format".to_string());
            }
        }

        // Validate LP mint address if provided
        if let Some(lp_mint) = &self.lp_mint {
            if !is_valid_solana_address(lp_mint) {
                return Err("Invalid LP mint address format".to_string());
            }
        }

        Ok(())
    }

    /// Get a risk assessment based on the lock score
    pub fn risk_assessment(&self) -> &'static str {
        match self.lock_score {
            90..=100 => "VERY LOW RISK - Excellent LP security",
            70..=89 => "LOW RISK - Good LP security",
            50..=69 => "MEDIUM RISK - Moderate LP security",
            30..=49 => "HIGH RISK - Poor LP security",
            10..=29 => "VERY HIGH RISK - Weak LP security",
            _ => "CRITICAL RISK - No LP security",
        }
    }

    /// Check if this token is considered safe for trading
    pub fn is_safe_for_trading(&self) -> bool {
        self.lock_score >= 70 &&
            !matches!(self.status, LpLockStatus::NoPool | LpLockStatus::CreatorHeld)
    }
}

/// Enhanced LP lock analysis with retry logic and better error handling
pub async fn check_lp_lock_status_with_retry(
    token_mint: &str,
    max_retries: u32
) -> Result<LpLockAnalysis, ScreenerBotError> {
    let mut last_error = None;

    for attempt in 0..=max_retries {
        match check_lp_lock_status(token_mint).await {
            Ok(analysis) => {
                if attempt > 0 {
                    log(
                        LogTag::Rpc,
                        "LP_LOCK_RETRY_SUCCESS",
                        &format!("LP lock analysis succeeded on attempt {}", attempt + 1)
                    );
                }
                return Ok(analysis);
            }
            Err(e) => {
                last_error = Some(e);
                if attempt < max_retries {
                    log(
                        LogTag::Rpc,
                        "LP_LOCK_RETRY",
                        &format!("LP lock analysis failed (attempt {}), retrying...", attempt + 1)
                    );
                    tokio::time::sleep(
                        tokio::time::Duration::from_millis(1000 * ((attempt + 1) as u64))
                    ).await;
                }
            }
        }
    }

    Err(last_error.unwrap())
}

/// Get LP lock statistics for multiple tokens
pub async fn get_lp_lock_statistics(
    token_mints: &[String]
) -> Result<LpLockStatistics, ScreenerBotError> {
    let mut stats = LpLockStatistics::default();

    for mint in token_mints {
        match check_lp_lock_status(mint).await {
            Ok(analysis) => {
                stats.total_analyzed += 1;

                match analysis.status {
                    LpLockStatus::Burned => {
                        stats.burned_count += 1;
                    }
                    LpLockStatus::TimeLocked { .. } => {
                        stats.time_locked_count += 1;
                    }
                    LpLockStatus::ProgramLocked { .. } => {
                        stats.program_locked_count += 1;
                    }
                    LpLockStatus::CreatorHeld => {
                        stats.creator_held_count += 1;
                    }
                    LpLockStatus::Unknown => {
                        stats.unknown_count += 1;
                    }
                    LpLockStatus::NoPool => {
                        stats.no_pool_count += 1;
                    }
                }

                stats.total_score += analysis.lock_score as u64;

                if analysis.lock_score >= 70 {
                    stats.secure_count += 1;
                }
            }
            Err(_) => {
                stats.error_count += 1;
            }
        }
    }

    if stats.total_analyzed > 0 {
        stats.average_score = ((stats.total_score as f64) / (stats.total_analyzed as f64)) as u8;
    }

    Ok(stats)
}

/// LP lock statistics for multiple tokens
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LpLockStatistics {
    pub total_analyzed: u32,
    pub burned_count: u32,
    pub time_locked_count: u32,
    pub program_locked_count: u32,
    pub creator_held_count: u32,
    pub unknown_count: u32,
    pub no_pool_count: u32,
    pub error_count: u32,
    pub secure_count: u32, // Score >= 70
    pub average_score: u8,
    pub total_score: u64,
}
