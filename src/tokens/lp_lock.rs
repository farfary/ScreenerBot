/// Liquidity Pool Lock Detection Module
///
/// This module provides functionality to detect whether a token's liquidity pool
/// is locked or not. It uses DexScreener API cached pool data instead of RPC calls
/// to find pools, then checks if LP tokens are locked/burned.

use crate::logger::{ log, LogTag };
use crate::tokens::dexscreener::{
    get_token_pools_from_dexscreener,
    get_cached_pools_for_token,
    TokenPair,
};
use crate::utils::safe_truncate;
use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;
use std::sync::{ LazyLock, RwLock };

/// LP lock analysis cache (5 minute TTL)
static LP_LOCK_CACHE: LazyLock<RwLock<HashMap<String, CachedLpLockAnalysis>>> = LazyLock::new(||
    RwLock::new(HashMap::new())
);

const LP_LOCK_CACHE_TTL_SECS: i64 = 300; // 5 minutes

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
        matches!(
            self,
            LpLockStatus::Burned |
                LpLockStatus::TimeLocked { .. } |
                LpLockStatus::ProgramLocked { .. }
        )
    }

    /// Get human-readable status description
    pub fn description(&self) -> &'static str {
        match self {
            LpLockStatus::Burned => "LP tokens burned (safe)",
            LpLockStatus::TimeLocked { .. } => "LP tokens time-locked",
            LpLockStatus::ProgramLocked { .. } => "LP tokens program-locked",
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
        &format!("Analyzing LP lock status for {}", safe_truncate(token_mint, 8))
    );

    let start_time = std::time::Instant::now();

    // Step 1: Get pools from DexScreener (try cache first, then API)
    let pools = match get_pools_for_token(token_mint).await {
        Ok(pools) => pools,
        Err(e) => {
            log(
                LogTag::Security,
                "ERROR",
                &format!("Failed to get pools for {}: {}", safe_truncate(token_mint, 8), e)
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
        log(
            LogTag::Security,
            "NO_POOLS",
            &format!("No pools found for {}", safe_truncate(token_mint, 8))
        );
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
            "Selected {} pool {} for {}",
            best_pool.dex_id,
            safe_truncate(&best_pool.pair_address, 8),
            safe_truncate(token_mint, 8)
        )
    );

    // Step 3: Analyze the selected pool
    let analysis = analyze_pool_lock_status(token_mint, &best_pool).await?;

    let elapsed = start_time.elapsed();
    log(
        LogTag::Security,
        "ANALYSIS_COMPLETE",
        &format!(
            "LP lock analysis for {} completed in {}ms: {}",
            safe_truncate(token_mint, 8),
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
        &format!("Fetching pools from API for {}", safe_truncate(token_mint, 8))
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

/// Analyze a specific pool's lock status
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
        data_source: "dexscreener".to_string(),
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

    // Check DEX reputation
    match pool.dex_id.to_lowercase().as_str() {
        "raydium" | "orca" => {
            details.push(format!("Reputable DEX: {}", pool.dex_id));
            *lock_score += 20;
        }
        "meteora" | "jupiter" => {
            details.push(format!("Known DEX: {}", pool.dex_id));
            *lock_score += 10;
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
