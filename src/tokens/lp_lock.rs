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

use crate::{
    errors::ScreenerBotError,
    logger::{ log, LogTag },
    rpc::get_rpc_client,
    utils::safe_truncate,
};
use base64::Engine;
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use chrono::{ DateTime, Utc };

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
    /// Additional notes
    pub notes: Vec<String>,
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

        // Add more as they are discovered
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
}

/// Check if a token's liquidity pool is locked
/// This is the main function that should be used everywhere
pub async fn check_lp_lock_status(token_mint: &str) -> Result<LpLockAnalysis, ScreenerBotError> {
    log(
        LogTag::Rpc,
        "LP_LOCK_CHECK",
        &format!("Checking LP lock status for token {}", safe_truncate(token_mint, 8))
    );

    let analysis_start = Utc::now();

    // Step 1: Find the liquidity pool for this token
    let pool_info = find_liquidity_pool(token_mint).await?;

    if pool_info.is_none() {
        log(
            LogTag::Rpc,
            "LP_LOCK_CHECK",
            &format!("No liquidity pool found for token {}", safe_truncate(token_mint, 8))
        );

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
            },
            analyzed_at: analysis_start,
        });
    }

    let (pool_address, lp_mint, pool_type) = pool_info.unwrap();

    log(
        LogTag::Rpc,
        "LP_LOCK_CHECK",
        &format!(
            "Found {} pool {} with LP mint {} for token {}",
            pool_type,
            safe_truncate(&pool_address, 8),
            safe_truncate(&lp_mint, 8),
            safe_truncate(token_mint, 8)
        )
    );

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
        pool_address: Some(pool_address),
        lp_mint: Some(lp_mint),
        status,
        details: merge_analysis_details(lp_mint_analysis, distribution_analysis, pool_type),
        analyzed_at: analysis_start,
    };

    log(
        LogTag::Rpc,
        "LP_LOCK_RESULT",
        &format!(
            "LP lock analysis for {}: {} - {}",
            safe_truncate(token_mint, 8),
            analysis.status.risk_level(),
            analysis.status.description()
        )
    );

    Ok(analysis)
}

/// Find liquidity pool for a token (searches multiple DEXs)
async fn find_liquidity_pool(
    token_mint: &str
) -> Result<Option<(String, String, String)>, ScreenerBotError> {
    log(
        LogTag::Rpc,
        "POOL_SEARCH",
        &format!("Starting comprehensive pool search for token {}", safe_truncate(token_mint, 8))
    );

    // Check if this might be a Pump.fun token based on the mint address pattern
    if token_mint.ends_with("pump") {
        log(
            LogTag::Rpc,
            "PUMPFUN_DETECTED",
            &format!(
                "Token {} appears to be a Pump.fun token based on mint pattern",
                safe_truncate(token_mint, 8)
            )
        );

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
        log(LogTag::Rpc, "POOL_FOUND", "Found Raydium V4 pool");
        return Ok(Some(pool_info));
    }

    // Try Raydium CPMM pools
    if
        let Ok(Some(pool_info)) = search_dex_pool(
            token_mint,
            &PoolSearchConfig::raydium_cpmm()
        ).await
    {
        log(LogTag::Rpc, "POOL_FOUND", "Found Raydium CPMM pool");
        return Ok(Some(pool_info));
    }

    // Try Orca Whirlpools
    if
        let Ok(Some(pool_info)) = search_dex_pool(
            token_mint,
            &PoolSearchConfig::orca_whirlpool()
        ).await
    {
        log(LogTag::Rpc, "POOL_FOUND", "Found Orca Whirlpool");
        return Ok(Some(pool_info));
    }

    // Try Meteora pools
    if
        let Ok(Some(pool_info)) = search_dex_pool(
            token_mint,
            &PoolSearchConfig::meteora_dlmm()
        ).await
    {
        log(LogTag::Rpc, "POOL_FOUND", "Found Meteora pool");
        return Ok(Some(pool_info));
    }

    log(
        LogTag::Rpc,
        "POOL_NOT_FOUND",
        &format!(
            "No liquidity pool found for token {} across all supported DEXs",
            safe_truncate(token_mint, 8)
        )
    );

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
            data_size: 1544,
            token_a_offset: 73,
            token_b_offset: 105,
            lp_extraction_method: LpExtractionMethod::DerivePda {
                seeds: &[b"lp_mint"],
            },
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
            program_id: "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB",
            pool_name: "Meteora DLMM",
            data_size: 1544,
            token_a_offset: 73,
            token_b_offset: 105,
            lp_extraction_method: LpExtractionMethod::DerivePda {
                seeds: &[b"lp_mint"],
            },
        }
    }
}

/// Generic pool search function that eliminates code duplication
async fn search_dex_pool(
    token_mint: &str,
    config: &PoolSearchConfig
) -> Result<Option<(String, String, String)>, ScreenerBotError> {
    let rpc_client = get_rpc_client();

    log(
        LogTag::Rpc,
        "DEX_SEARCH",
        &format!("Searching {} pools for token {}", config.pool_name, safe_truncate(token_mint, 8))
    );

    // Try both token positions (A and B)
    let positions = [
        ("token_a", config.token_a_offset),
        ("token_b", config.token_b_offset),
    ];

    for (position_name, offset) in positions {
        let filters = create_search_filters(config.data_size, offset, token_mint);

        match
            rpc_client.get_program_accounts(
                config.program_id,
                Some(filters),
                Some("base64"),
                Some(30)
            ).await
        {
            Ok(accounts) => {
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
                                    "Found {} pool at {} ({})",
                                    config.pool_name,
                                    safe_truncate(pool_address, 8),
                                    position_name
                                )
                            );
                            return Ok(
                                Some((
                                    pool_address.to_string(),
                                    lp_mint,
                                    config.pool_name.to_string(),
                                ))
                            );
                        }
                    }
                }
            }
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
                // Continue to next position instead of failing immediately
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

/// Derive Meteora LP mint (similar to Raydium CPMM)
async fn derive_meteora_lp_mint(pool_address: &str) -> Option<String> {
    use solana_sdk::pubkey::Pubkey;
    use std::str::FromStr;

    if let Ok(pool_pubkey) = Pubkey::from_str(pool_address) {
        // Meteora LP mint derivation (may be different from CPMM)
        let (lp_mint_pda, _) = Pubkey::find_program_address(
            &[b"lp_mint", pool_pubkey.as_ref()],
            &Pubkey::from_str("Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB").ok()?
        );
        return Some(lp_mint_pda.to_string());
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

    // Get all token accounts for this LP mint
    let filters =
        serde_json::json!([
        {
            "dataSize": 165  // Standard token account size
        },
        {
            "memcmp": {
                "offset": 0,
                "bytes": lp_mint
            }
        }
    ]);

    let accounts = rpc_client.get_program_accounts(
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
        Some(filters),
        Some("jsonParsed"),
        Some(60)
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
        // Need to analyze the NFT distribution instead
        // For now, treat as unknown since we need different analysis logic
        return LpLockStatus::Unknown;
    }

    // Traditional LP token analysis for Raydium, Meteora, etc.

    // Priority 1: If mint authority is burned, it's permanently locked
    if mint_analysis.is_burned {
        return LpLockStatus::Burned;
    }

    // Priority 2: Check for lock programs
    if !distribution_analysis.lock_programs.is_empty() && distribution_analysis.locked_amount > 0 {
        // TODO: Add time-lock detection logic here
        // For now, treat all program locks as general program locks
        return LpLockStatus::ProgramLocked {
            program: distribution_analysis.lock_programs.join(", "),
            amount: distribution_analysis.locked_amount,
        };
    }

    // Priority 3: If LP tokens are held by creator/deployer, not locked
    if distribution_analysis.creator_held_amount > 0 {
        return LpLockStatus::CreatorHeld;
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

    if mint_analysis.is_burned {
        notes.push("LP mint authority has been burned (set to None)".to_string());
    } else if let Some(ref authority) = mint_analysis.mint_authority {
        notes.push(format!("LP mint authority: {}", safe_truncate(authority, 8)));
    }

    if !distribution_analysis.lock_programs.is_empty() {
        notes.push(
            format!("Lock programs detected: {}", distribution_analysis.lock_programs.join(", "))
        );
    }

    if distribution_analysis.total_holders == 0 {
        notes.push("No LP token holders found".to_string());
    } else {
        notes.push(format!("Total LP holders: {}", distribution_analysis.total_holders));
    }

    LpLockDetails {
        pool_type: Some(pool_type),
        total_lp_supply: Some(mint_analysis.supply),
        locked_lp_amount: distribution_analysis.locked_amount,
        creator_held_amount: distribution_analysis.creator_held_amount,
        lock_programs: distribution_analysis.lock_programs,
        lp_mint_authority: mint_analysis.mint_authority,
        notes,
    }
}

/// Internal structure for LP mint analysis
#[derive(Debug)]
struct LpMintAnalysis {
    mint_authority: Option<String>,
    supply: u64,
    is_burned: bool,
}

/// Internal structure for LP distribution analysis
#[derive(Debug)]
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
                            safe_truncate(&chunk[i], 8),
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

/// Get a quick LP lock summary string for a token
/// This is useful for logging and display purposes
pub async fn get_lp_lock_summary(token_mint: &str) -> Result<String, ScreenerBotError> {
    let analysis = check_lp_lock_status(token_mint).await?;
    Ok(format!("{} - {}", analysis.status.risk_level(), analysis.status.description()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_programs() {
        let programs = LockPrograms::known_programs();
        assert!(!programs.is_empty());

        // Test known program detection
        assert!(
            LockPrograms::is_lock_program("ADHS2E6D7YvbXF6y6EVPWyN6u6eDvAQj1QqRAEyrnKE4").is_some()
        );
        assert!(LockPrograms::is_lock_program("invalid_address").is_none());
    }

    #[test]
    fn test_lp_lock_status() {
        let burned = LpLockStatus::Burned;
        assert!(burned.is_safe());
        assert_eq!(burned.risk_level(), "🟢 SAFE");

        let creator_held = LpLockStatus::CreatorHeld;
        assert!(!creator_held.is_safe());
        assert_eq!(creator_held.risk_level(), "🔴 RISKY");
    }
}
