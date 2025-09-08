use crate::global::is_debug_filtering_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::is_token_excluded_from_trading;
use crate::tokens::cache::TokenDatabase;
use crate::tokens::types::{ Token, ApiToken };
use crate::trader::MAX_OPEN_POSITIONS;
use chrono::{ Duration as ChronoDuration, Utc };

// =============================================================================
// FILTERING CONFIGURATION PARAMETERS
// =============================================================================

const MAX_TOKENS_FOR_DETAILED_FILTERING: usize = 1000;

// ===== AGE FILTERING PARAMETERS =====
pub const MIN_TOKEN_AGE_SECONDS: i64 = 0;
pub const MAX_TOKEN_AGE_SECONDS: i64 = 24 * 30 * 24 * 60 * 60; // 2 years

// ===== PRICE ACTION FILTERING PARAMETERS =====
pub const MIN_VALID_PRICE_SOL: f64 = 0.0000000000001;
pub const MAX_VALID_PRICE_SOL: f64 = 0.1;

// ===== LIQUIDITY FILTERING PARAMETERS =====
pub const MIN_LIQUIDITY_USD: f64 = 1.0;
pub const MAX_LIQUIDITY_USD: f64 = 5_000_000.0;
pub const MAX_MARKET_CAP_USD: f64 = 1_000_000_000.0;
pub const MIN_VOLUME_LIQUIDITY_RATIO: f64 = 0.1;

// ===== TRANSACTION ACTIVITY FILTERING PARAMETERS =====
pub const MIN_TRANSACTIONS_5MIN: i64 = 5; // Reduced from 100 - much more realistic
pub const MAX_TRANSACTIONS_5MIN: i64 = 5000;
pub const MIN_TRANSACTIONS_1H: i64 = 3; // Reduced from 15 - more realistic
pub const MIN_BUY_SELL_RATIO: f64 = 0.45;
pub const MAX_BUY_SELL_RATIO: f64 = 2.5;

// ===== RUGCHECK SECURITY PARAMETERS =====
pub const MAX_RUGCHECK_RISK_SCORE: i32 = 100;
pub const EMERGENCY_MAX_RISK_SCORE: i32 = 100;
pub const MAX_CRITICAL_RISK_ISSUES: usize = 5;

// ===== LP LOCK SECURITY PARAMETERS =====
pub const MIN_LP_LOCK_PERCENTAGE: f64 = 80.0;
pub const MIN_LP_LOCK_PERCENTAGE_NEW_TOKENS: f64 = 80.0;

// ===== HISTORICAL PERFORMANCE PARAMETERS =====
pub const MAX_HISTORICAL_LOSS_RATE: f64 = 0.8;
pub const MAX_AVERAGE_LOSS_PERCENTAGE: f64 = 50.0;

// ===== VALIDATION TIMEOUTS =====
pub const DB_LOCK_TIMEOUT_MS: u64 = 5000;
pub const PRICE_HISTORY_LOCK_TIMEOUT_MS: u64 = 3000;

// =============================================================================
// FILTERING RESULT ENUM
// =============================================================================

/// Reasons why a token might be filtered out
#[derive(Debug, Clone)]
pub enum FilterReason {
    // Blacklist/exclusion filters (HIGHEST PRIORITY)
    TokenBlacklisted {
        reason: String,
    },
    SystemOrStableToken,

    // Basic validation failures
    EmptySymbol,
    EmptyMint,
    EmptyLogoUrl,
    EmptyWebsite,
    EmptyDescription,
    InvalidPrice,
    PriceTooLow {
        current_price: f64,
        minimum_price: f64,
    },
    PriceTooHigh {
        current_price: f64,
        maximum_price: f64,
    },
    ZeroLiquidity,
    InsufficientLiquidity {
        current_usd: f64,
        minimum_required: f64,
    },
    TooHighLiquidity {
        current_usd: f64,
        maximum_allowed: f64,
    },
    TooHighMarketCap {
        current_usd: f64,
        maximum_allowed: f64,
    },
    MissingLiquidityData,
    MissingPriceData,

    // Age-related failures
    TooYoung {
        age_hours: i64,
        min_required: i64,
    },
    TooOld {
        age_hours: i64,
        max_allowed: i64,
    },
    NoCreationDate,

    // Position-related failures (non-cooldown; cooldown is handled exclusively in trader)
    ExistingOpenPosition,
    MaxPositionsReached {
        current: usize,
        max: usize,
    },

    // Account/Token status issues
    AccountFrozen,
    TokenAccountFrozen,

    // Rugcheck security risks
    RugcheckRisk {
        risk_level: String,
        reasons: Vec<String>,
    },

    // LP lock security risks
    InsufficientLpLock {
        current_lock_percentage: f64,
        minimum_required: f64,
    },

    // Holder concentration risks (NEW - for micro-cap protection)
    WhaleConcentrationRisk {
        holder_rank: usize, // 0 = top-5 total, 1+ = individual holder rank
        percentage: f64,
        max_allowed: f64,
    },

    // Trading requirements
    LockAcquisitionFailed,

    // Decimal validation failures
    DecimalsNotAvailable {
        mint: String,
    },

    // Transaction activity failures
    InsufficientTransactionActivity {
        period: String,
        current_count: i64,
        minimum_required: i64,
    },
    ExcessiveTransactionActivity {
        period: String,
        current_count: i64,
        maximum_allowed: i64,
    },
    UnhealthyBuySellRatio {
        buys: i64,
        sells: i64,
        ratio: f64,
        min_ratio: f64,
        max_ratio: f64,
    },
    NoTransactionData,

    // Performance-related filtering
    PerformanceLimitExceeded {
        total_tokens: usize,
        max_allowed: usize,
    },
}

/// Result of token filtering
#[derive(Debug, Clone)]
pub enum FilterResult {
    Approved,
    Rejected(FilterReason),
}

// =============================================================================
// MAIN TOKEN ACQUISITION AND FILTERING FUNCTION
// =============================================================================

/// Get filtered tokens ready for pool service monitoring
///
/// This is the main entry point for the pool service to get tokens.
/// It handles all database fetching, blacklist filtering (FIRST), freshness filtering,
/// and comprehensive token filtering in one place.
///
/// Returns a list of token mint addresses that are ready for pool monitoring.
pub async fn get_filtered_tokens() -> Result<Vec<String>, String> {
    if is_debug_filtering_enabled() {
        log(LogTag::Filtering, "DEBUG", "🔍 Starting token acquisition and filtering process");
    }

    // 1. Get ALL tokens from database (no filtering yet)
    let database = TokenDatabase::new().map_err(|e|
        format!("Failed to create token database: {}", e)
    )?;

    // Get all tokens from database with update time
    let all_tokens_with_time = database
        .get_all_tokens_with_update_time().await
        .map_err(|e| format!("Failed to get tokens from database: {}", e))?;

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG",
            &format!("📊 Retrieved {} total tokens from database", all_tokens_with_time.len())
        );
    }

    // 2. BLACKLIST FILTER FIRST - Remove blacklisted tokens immediately (HIGHEST PRIORITY)
    let pre_blacklist_count = all_tokens_with_time.len();
    let blacklist_filtered_tokens: Vec<
        (String, String, chrono::DateTime<chrono::Utc>, f64)
    > = all_tokens_with_time
        .into_iter()
        .filter(|(mint, _, _, _)| !is_token_excluded_from_trading(mint))
        .collect();

    let blacklisted_count = pre_blacklist_count - blacklist_filtered_tokens.len();
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG",
            &format!(
                "� STEP 1 - Blacklist filter: {} tokens → {} tokens ({} blacklisted removed)",
                pre_blacklist_count,
                blacklist_filtered_tokens.len(),
                blacklisted_count
            )
        );
    }

    // 3. Apply freshness filter (last 1 hour only) - AFTER blacklist filtering
    let now = chrono::Utc::now();
    let one_hour_ago = now - chrono::Duration::hours(1);

    let fresh_tokens: Vec<
        (String, String, chrono::DateTime<chrono::Utc>, f64)
    > = blacklist_filtered_tokens
        .into_iter()
        .filter(|(_, _, last_updated, _)| *last_updated >= one_hour_ago)
        .collect();

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG",
            &format!(
                "⏱️ STEP 2 - Freshness filter: {} tokens updated in last hour",
                fresh_tokens.len()
            )
        );
    }

    // 4. Apply 5000 token limit (hardcoded near MAX_WATCHED_TOKENS)
    const MAX_TOKENS_FOR_PROCESSING: usize = 5000;
    let limited_tokens = if fresh_tokens.len() > MAX_TOKENS_FOR_PROCESSING {
        // Sort by liquidity (highest first) and take top 5000
        let mut sorted_fresh = fresh_tokens;
        sorted_fresh.sort_by(|a, b| b.3.partial_cmp(&a.3).unwrap_or(std::cmp::Ordering::Equal));
        sorted_fresh.into_iter().take(MAX_TOKENS_FOR_PROCESSING).collect()
    } else {
        fresh_tokens
    };

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG",
            &format!(
                "📊 STEP 3 - Token limit applied: processing {} tokens (max: {})",
                limited_tokens.len(),
                MAX_TOKENS_FOR_PROCESSING
            )
        );
    }

    // 5. Convert to Token objects for filtering
    let token_mints: Vec<String> = limited_tokens
        .iter()
        .map(|(mint, _, _, _)| mint.clone())
        .collect();
    let mut all_tokens: Vec<Token> = Vec::new();

    for mint in &token_mints {
        if let Ok(Some(api_token)) = database.get_token_by_mint(mint) {
            all_tokens.push(api_token.into());
        }
    }

    // 6.1 Remove zero-liquidity tokens first to avoid unnecessary decimals work
    let before_zero_liq = all_tokens.len();
    let mut zero_liquidity_filtered = 0usize;
    all_tokens.retain(|token| {
        let liquidity_usd = token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0);
        if liquidity_usd <= 0.0 {
            zero_liquidity_filtered += 1;
            return false;
        }
        true
    });
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG",
            &format!(
                "🧹 STEP 4a - Zero-liquidity prefilter: {} → {} ({} removed)",
                before_zero_liq,
                all_tokens.len(),
                zero_liquidity_filtered
            )
        );
    }

    // Populate cached fields (decimals, rugcheck) before decimal/age early filtering
    let mut decimals_populated = 0usize;
    let mut populate_errors = 0usize;
    for token in all_tokens.iter_mut() {
        if token.decimals.is_none() {
            match token.populate_cached_data() {
                Ok(()) => {
                    if token.decimals.is_some() {
                        decimals_populated += 1;
                    }
                }
                Err(_) => {
                    populate_errors += 1;
                }
            }
        }
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG",
            &format!(
                "🧩 STEP 4b - Populated decimals for {} tokens ({} errors)",
                decimals_populated,
                populate_errors
            )
        );
    }

    // 6. EARLY PERFORMANCE FILTERING - Remove obvious rejects before detailed processing
    let initial_count = all_tokens.len();
    let mut no_decimals_filtered = 0;
    let mut old_tokens_filtered = 0;

    all_tokens.retain(|token| {
        // Filter tokens without decimals
        if token.decimals.is_none() {
            no_decimals_filtered += 1;
            return false;
        }

        // Filter tokens that are too old (older than MAX_TOKEN_AGE_SECONDS)
        if let Some(created_at) = token.created_at {
            let now = chrono::Utc::now();
            let token_age = now - created_at;
            let age_seconds = token_age.num_seconds();

            if age_seconds > MAX_TOKEN_AGE_SECONDS {
                old_tokens_filtered += 1;
                return false;
            }
        }

        true
    });

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG",
            &format!(
                "🚀 Early performance filtering: {} tokens → {} tokens ({} no decimals, {} too old filtered)",
                initial_count,
                all_tokens.len(),
                no_decimals_filtered,
                old_tokens_filtered
            )
        );
    }

    // 7. Get open position mints (always monitor these regardless of filtering)
    let open_position_mints = crate::positions::get_open_mints().await;
    let mut monitored_tokens: Vec<String> = Vec::new();
    let mut open_positions_added = 0;

    // First, add all open position tokens (priority monitoring)
    for mint in &open_position_mints {
        if !monitored_tokens.contains(mint) {
            monitored_tokens.push(mint.clone());
            open_positions_added += 1;
        }
    }

    // 8. Apply comprehensive filtering to all tokens
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG",
            &format!(
                "🔧 Applying comprehensive filtering to {} tokens from database",
                all_tokens.len()
            )
        );
    }

    let (eligible_tokens, rejected_tokens) = filter_tokens_with_reasons(&all_tokens);

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG",
            &format!(
                "📊 Filtering results: {} eligible, {} rejected out of {} total tokens",
                eligible_tokens.len(),
                rejected_tokens.len(),
                all_tokens.len()
            )
        );
    }

    // 7. Sort eligible tokens by liquidity (highest first) and take up to remaining slots
    const MAX_WATCHED_TOKENS: usize = 500; // Using constant from pool types
    let remaining_slots = MAX_WATCHED_TOKENS.saturating_sub(monitored_tokens.len());
    let mut sorted_eligible_tokens = eligible_tokens;
    sorted_eligible_tokens.sort_by(|a, b| {
        let a_liq = a.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0);
        let b_liq = b.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0);
        b_liq.partial_cmp(&a_liq).unwrap_or(std::cmp::Ordering::Equal)
    });

    // 8. Add filtered tokens up to the limit, avoiding duplicates
    let mut filtered_tokens_added = 0;
    for token in sorted_eligible_tokens.into_iter().take(remaining_slots) {
        if !monitored_tokens.contains(&token.mint) {
            monitored_tokens.push(token.mint.clone());
            filtered_tokens_added += 1;
        }
    }

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG",
            &format!(
                "✅ Selected {} tokens for monitoring: {} open positions + {} filtered tokens (total: {})",
                monitored_tokens.len(),
                open_positions_added,
                filtered_tokens_added,
                monitored_tokens.len()
            )
        );
    }

    Ok(monitored_tokens)
}

// =============================================================================
// MAIN FILTERING FUNCTION
// =============================================================================

/// High-performance token filtering function for 1000 tokens per cycle
/// Note: Assumes tokens have already passed early performance filtering
/// (zero liquidity, missing decimals, and age constraints)
pub fn filter_token_for_trading(token: &Token) -> FilterResult {
    // Essential validations only - no debugging overhead for speed

    // Blacklist/exclusion check (highest priority)
    if let Some(reason) = validate_blacklist_exclusion(token) {
        return FilterResult::Rejected(reason);
    }

    // Security check using cached rugcheck data
    if let Some(reason) = validate_rugcheck_risks(token) {
        return FilterResult::Rejected(reason);
    }

    // Basic metadata validation
    if let Some(reason) = validate_basic_token_info(token) {
        return FilterResult::Rejected(reason);
    }

    // Note: Age validation skipped - handled by early filtering
    // Note: Liquidity validation skipped - handled by early filtering
    // Note: Decimal availability skipped - handled by early filtering

    // Price data validation (still needed for price ranges)
    if let Some(reason) = validate_price_data(token) {
        return FilterResult::Rejected(reason);
    }

    // Transaction activity validation
    if let Some(reason) = validate_transaction_activity(token) {
        return FilterResult::Rejected(reason);
    }

    FilterResult::Approved
}

// =============================================================================
// INDIVIDUAL FILTER FUNCTIONS
// =============================================================================

/// Validate blacklist and exclusion status (ABSOLUTE HIGHEST PRIORITY)
fn validate_blacklist_exclusion(token: &Token) -> Option<FilterReason> {
    // Check if token is excluded from trading (blacklisted OR system/stable)
    if is_token_excluded_from_trading(&token.mint) {
        // Determine if it's a system/stable token or blacklisted
        if crate::tokens::is_system_or_stable_token(&token.mint) {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "DEBUG_BLACKLIST",
                    &format!(
                        "🚫 Token {} ({}) is a system or stable token - excluded",
                        token.symbol,
                        token.mint
                    )
                );
            }
            return Some(FilterReason::SystemOrStableToken);
        } else {
            // It's in the dynamic blacklist
            let reason_description = if let Some(stats) = crate::tokens::get_blacklist_stats_db() {
                format!("Blacklisted (total: {})", stats.total_blacklisted)
            } else {
                "Blacklisted".to_string()
            };

            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "DEBUG_BLACKLIST",
                    &format!(
                        "🚫 Token {} ({}) is blacklisted - {}",
                        token.symbol,
                        token.mint,
                        reason_description
                    )
                );
            }

            return Some(FilterReason::TokenBlacklisted {
                reason: reason_description,
            });
        }
    }

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_BLACKLIST",
            &format!("✅ Token {} ({}) passed blacklist/exclusion check", token.symbol, token.mint)
        );
    }

    None
}

/// Validate rugcheck security risks using cached data (no API calls)
fn validate_rugcheck_risks(token: &Token) -> Option<FilterReason> {
    // STRICT REQUIREMENT: Token must have rugcheck data
    let rugcheck_data = match &token.rugcheck_data {
        Some(data) => data,
        None => {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "RUGCHECK_MISSING",
                    &format!(
                        "❌ Token {} ({}) REJECTED: No rugcheck data available",
                        token.symbol,
                        token.mint
                    )
                );
            }
            return Some(FilterReason::RugcheckRisk {
                risk_level: "MISSING_DATA".to_string(),
                reasons: vec!["No rugcheck security data available".to_string()],
            });
        }
    };

    // Check if token is marked as rugged
    if rugcheck_data.rugged.unwrap_or(false) {
        return Some(FilterReason::RugcheckRisk {
            risk_level: "CRITICAL".to_string(),
            reasons: vec!["Token is marked as RUGGED".to_string()],
        });
    }

    // Check risk score (higher scores mean more risk)
    if let Some(risk_score) = rugcheck_data.score_normalised.or(rugcheck_data.score) {
        if risk_score >= EMERGENCY_MAX_RISK_SCORE {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "EMERGENCY_RISK",
                    &format!(
                        "Token {} ({}) EMERGENCY REJECTED - Risk score {} >= {} (VERY HIGH RISK)",
                        token.symbol,
                        token.mint,
                        risk_score,
                        EMERGENCY_MAX_RISK_SCORE
                    )
                );
            }
            return Some(FilterReason::RugcheckRisk {
                risk_level: "EMERGENCY".to_string(),
                reasons: vec![
                    format!(
                        "Risk score {} is too high (max allowed: {})",
                        risk_score,
                        EMERGENCY_MAX_RISK_SCORE
                    )
                ],
            });
        }

        if risk_score > MAX_RUGCHECK_RISK_SCORE {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "HIGH_RISK_SCORE",
                    &format!(
                        "Token {} ({}) rejected - Risk score {} > {} (HIGH RISK)",
                        token.symbol,
                        token.mint,
                        risk_score,
                        MAX_RUGCHECK_RISK_SCORE
                    )
                );
            }
            return Some(FilterReason::RugcheckRisk {
                risk_level: "HIGH".to_string(),
                reasons: vec![
                    format!(
                        "Risk score {} exceeds maximum allowed {}",
                        risk_score,
                        MAX_RUGCHECK_RISK_SCORE
                    )
                ],
            });
        }
    }

    // Check for critical risks
    if let Some(risks) = &rugcheck_data.risks {
        let critical_risks: Vec<_> = risks
            .iter()
            .filter(|r| r.level.as_deref() == Some("critical"))
            .collect();

        if critical_risks.len() > MAX_CRITICAL_RISK_ISSUES {
            return Some(FilterReason::RugcheckRisk {
                risk_level: "CRITICAL".to_string(),
                reasons: critical_risks
                    .iter()
                    .map(|r| r.name.clone())
                    .collect(),
            });
        }
    }

    // Check LP lock status from rugcheck data
    if let Some(markets) = &rugcheck_data.markets {
        let mut best_lock_pct = 0.0f64;
        for market in markets {
            if let Some(lp) = &market.lp {
                if let Some(pct) = lp.lp_locked_pct {
                    best_lock_pct = best_lock_pct.max(pct);
                }
            }
        }

        if best_lock_pct < MIN_LP_LOCK_PERCENTAGE {
            return Some(FilterReason::InsufficientLpLock {
                current_lock_percentage: best_lock_pct,
                minimum_required: MIN_LP_LOCK_PERCENTAGE,
            });
        }
    }

    if is_debug_filtering_enabled() {
        let score = rugcheck_data.score_normalised.or(rugcheck_data.score).unwrap_or(0);
        log(
            LogTag::Filtering,
            "RUGCHECK_PASS",
            &format!(
                "Token {} ({}) passed rugcheck validation (risk score: {})",
                token.symbol,
                token.mint,
                score
            )
        );
    }

    None
}

/// Validate basic token metadata
fn validate_basic_token_info(token: &Token) -> Option<FilterReason> {
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_META",
            &format!(
                "📝 Validating metadata for {}: symbol='{}', mint='{}', logo={}, website={}, description={}",
                token.symbol,
                token.symbol,
                token.mint,
                token.logo_url.as_ref().map_or("None", |_| "Present"),
                if
                    token.website.as_ref().map_or(false, |w| !w.trim().is_empty()) ||
                    token.info
                        .as_ref()
                        .map_or(
                            false,
                            |info|
                                !info.websites.is_empty() &&
                                info.websites.iter().any(|w| !w.url.trim().is_empty())
                        )
                {
                    "Present"
                } else {
                    "Missing"
                },
                token.description
                    .as_ref()
                    .map_or("None", |desc| if desc.trim().is_empty() { "Empty" } else { "Present" })
            )
        );
    }

    if token.symbol.is_empty() {
        if is_debug_filtering_enabled() {
            log(LogTag::Filtering, "DEBUG_META", "❌ Symbol is empty");
        }
        return Some(FilterReason::EmptySymbol);
    }

    if token.mint.is_empty() {
        if is_debug_filtering_enabled() {
            log(LogTag::Filtering, "DEBUG_META", "❌ Mint address is empty");
        }
        return Some(FilterReason::EmptyMint);
    }

    None
}

/// Validate token age constraints
fn validate_token_age(token: &Token) -> Option<FilterReason> {
    let Some(created_at) = token.created_at else {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_AGE",
                &format!("⏰ Token {} ({}) has no creation date", token.symbol, token.mint)
            );
        }
        return Some(FilterReason::NoCreationDate);
    };

    let now = Utc::now();
    let token_age = now - created_at;
    let age_seconds = token_age.num_seconds();
    let age_hours = token_age.num_hours();
    let age_minutes = token_age.num_minutes();

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_AGE",
            &format!(
                "⏰ Age check for {} ({}): {}h {}m old (created: {}), min: {}s ({}h), max: {}s ({}h)",
                token.symbol,
                token.mint,
                age_hours,
                age_minutes % 60,
                created_at.format("%Y-%m-%d %H:%M:%S UTC"),
                MIN_TOKEN_AGE_SECONDS,
                MIN_TOKEN_AGE_SECONDS / 3600,
                MAX_TOKEN_AGE_SECONDS,
                MAX_TOKEN_AGE_SECONDS / 3600
            )
        );
    }

    if age_seconds < MIN_TOKEN_AGE_SECONDS {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_AGE",
                &format!(
                    "❌ Token {} ({}) too young: {}s < {}s minimum",
                    token.symbol,
                    token.mint,
                    age_seconds,
                    MIN_TOKEN_AGE_SECONDS
                )
            );
        }
        return Some(FilterReason::TooYoung {
            age_hours,
            min_required: MIN_TOKEN_AGE_SECONDS / 3600,
        });
    }

    if age_seconds > MAX_TOKEN_AGE_SECONDS {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_AGE",
                &format!(
                    "❌ Token {} ({}) too old: {}s > {}s maximum",
                    token.symbol,
                    token.mint,
                    age_seconds,
                    MAX_TOKEN_AGE_SECONDS
                )
            );
        }
        return Some(FilterReason::TooOld {
            age_hours,
            max_allowed: MAX_TOKEN_AGE_SECONDS / 3600,
        });
    }

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_AGE",
            &format!("✅ Token {} ({}) age within acceptable range", token.symbol, token.mint)
        );
    }

    None
}

/// Validate liquidity requirements
fn validate_liquidity(token: &Token) -> Option<FilterReason> {
    let Some(liquidity) = &token.liquidity else {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_LIQUIDITY",
                &format!("💧 Token {} ({}) has no liquidity data", token.symbol, token.mint)
            );
        }
        return Some(FilterReason::MissingLiquidityData);
    };

    let liquidity_usd = liquidity.usd.unwrap_or(0.0);

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_LIQUIDITY",
            &format!(
                "💧 Liquidity check for {} ({}): ${:.2} (min: ${:.2}, max: ${:.2})",
                token.symbol,
                token.mint,
                liquidity_usd,
                MIN_LIQUIDITY_USD,
                MAX_LIQUIDITY_USD
            )
        );
    }

    if liquidity_usd <= 0.0 {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_LIQUIDITY",
                &format!("❌ Token {} ({}) has zero liquidity", token.symbol, token.mint)
            );
        }
        return Some(FilterReason::ZeroLiquidity);
    }

    // Apply minimum liquidity requirement
    if liquidity_usd < MIN_LIQUIDITY_USD {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_LIQUIDITY",
                &format!(
                    "❌ Token {} insufficient liquidity: ${:.2} < ${:.2} minimum",
                    token.symbol,
                    liquidity_usd,
                    MIN_LIQUIDITY_USD
                )
            );
        }
        return Some(FilterReason::InsufficientLiquidity {
            current_usd: liquidity_usd,
            minimum_required: MIN_LIQUIDITY_USD,
        });
    }

    // NEW: Apply maximum liquidity requirement - AVOID BIG STABLE TOKENS
    if liquidity_usd > MAX_LIQUIDITY_USD {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_LIQUIDITY",
                &format!(
                    "❌ Token {} too high liquidity (stable token): ${:.2} > ${:.2} maximum",
                    token.symbol,
                    liquidity_usd,
                    MAX_LIQUIDITY_USD
                )
            );
        }
        return Some(FilterReason::TooHighLiquidity {
            current_usd: liquidity_usd,
            maximum_allowed: MAX_LIQUIDITY_USD,
        });
    }

    // NEW: Check market cap if available - AVOID HIGH MARKET CAP TOKENS
    if let Some(market_cap) = token.market_cap {
        if market_cap > MAX_MARKET_CAP_USD {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "DEBUG_LIQUIDITY",
                    &format!(
                        "❌ Token {} too high market cap: ${:.2} > ${:.2} maximum",
                        token.symbol,
                        market_cap,
                        MAX_MARKET_CAP_USD
                    )
                );
            }
            return Some(FilterReason::TooHighMarketCap {
                current_usd: market_cap,
                maximum_allowed: MAX_MARKET_CAP_USD,
            });
        }
    }

    // NEW: Check volume/liquidity ratio for activity (pump detection)
    if let Some(volume_stats) = &token.volume {
        if let Some(volume_24h) = volume_stats.h24 {
            let volume_liquidity_ratio = volume_24h / liquidity_usd;

            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "DEBUG_LIQUIDITY",
                    &format!(
                        "💹 Token {} volume/liquidity ratio: {:.3} (24h vol: ${:.2})",
                        token.symbol,
                        volume_liquidity_ratio,
                        volume_24h
                    )
                );
            }

            // BONUS: High volume/liquidity ratio indicates activity despite small size (good for gems)
            if volume_liquidity_ratio >= MIN_VOLUME_LIQUIDITY_RATIO * 5.0 {
                if is_debug_filtering_enabled() {
                    log(
                        LogTag::Filtering,
                        "DEBUG_LIQUIDITY",
                        &format!(
                            "🚀 Token {} shows high activity (ratio: {:.3}) - POTENTIAL GEM!",
                            token.symbol,
                            volume_liquidity_ratio
                        )
                    );
                }
            }
        }
    }

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_LIQUIDITY",
            &format!(
                "✅ Token {} liquidity optimal for gem hunting (${:.2})",
                token.symbol,
                liquidity_usd
            )
        );
    }

    None
}

/// Validate price data availability and ranges
fn validate_price_data(token: &Token) -> Option<FilterReason> {
    let current_price = token.price_dexscreener_sol.unwrap_or(0.0);

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_PRICE",
            &format!(
                "💰 Price check for {} ({}): {:.10} SOL (range: {:.12} - {:.3} SOL)",
                token.symbol,
                token.mint,
                current_price,
                MIN_VALID_PRICE_SOL,
                MAX_VALID_PRICE_SOL
            )
        );
    }

    if current_price <= 0.0 {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_PRICE",
                &format!("❌ Token {} has invalid price: {:.10}", token.symbol, current_price)
            );
        }
        return Some(FilterReason::InvalidPrice);
    }

    if token.price_dexscreener_sol.is_none() {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_PRICE",
                &format!("❌ Token {} missing price data", token.symbol)
            );
        }
        return Some(FilterReason::MissingPriceData);
    }

    // Validate price is within acceptable range
    if current_price < MIN_VALID_PRICE_SOL {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_PRICE",
                &format!(
                    "❌ Token {} price too low: {:.12} < {:.12} minimum",
                    token.symbol,
                    current_price,
                    MIN_VALID_PRICE_SOL
                )
            );
        }
        return Some(FilterReason::PriceTooLow {
            current_price,
            minimum_price: MIN_VALID_PRICE_SOL,
        });
    }

    if current_price > MAX_VALID_PRICE_SOL {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_PRICE",
                &format!(
                    "❌ Token {} price too high: {:.10} > {:.3} maximum",
                    token.symbol,
                    current_price,
                    MAX_VALID_PRICE_SOL
                )
            );
        }
        return Some(FilterReason::PriceTooHigh {
            current_price,
            maximum_price: MAX_VALID_PRICE_SOL,
        });
    }

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_PRICE",
            &format!("✅ Token {} price within valid range", token.symbol)
        );
    }

    None
}

/// Validate transaction activity requirements (moved from trader.rs and entry.rs)
fn validate_transaction_activity(token: &Token) -> Option<FilterReason> {
    let Some(txns) = &token.txns else {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_TXN_ACTIVITY",
                &format!("📊 Token {} ({}) has no transaction data", token.symbol, token.mint)
            );
        }
        return Some(FilterReason::NoTransactionData);
    };

    // Check 5-minute transaction activity
    if let Some(m5) = &txns.m5 {
        let buys_5min = m5.buys.unwrap_or(0);
        let sells_5min = m5.sells.unwrap_or(0);
        let total_5min = buys_5min + sells_5min;

        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_TXN_ACTIVITY",
                &format!(
                    "📊 Token {} 5min activity: {} buys, {} sells, {} total (min: {}, max: {})",
                    token.symbol,
                    buys_5min,
                    sells_5min,
                    total_5min,
                    MIN_TRANSACTIONS_5MIN,
                    MAX_TRANSACTIONS_5MIN
                )
            );
        }

        // Check minimum activity requirement
        if total_5min < MIN_TRANSACTIONS_5MIN {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "DEBUG_TXN_ACTIVITY",
                    &format!(
                        "❌ Token {} insufficient 5min activity: {} < {} minimum",
                        token.symbol,
                        total_5min,
                        MIN_TRANSACTIONS_5MIN
                    )
                );
            }
            return Some(FilterReason::InsufficientTransactionActivity {
                period: "5min".to_string(),
                current_count: total_5min,
                minimum_required: MIN_TRANSACTIONS_5MIN,
            });
        }

        // Check maximum activity cap (avoid manipulation)
        if total_5min > MAX_TRANSACTIONS_5MIN {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "DEBUG_TXN_ACTIVITY",
                    &format!(
                        "❌ Token {} excessive 5min activity: {} > {} maximum",
                        token.symbol,
                        total_5min,
                        MAX_TRANSACTIONS_5MIN
                    )
                );
            }
            return Some(FilterReason::ExcessiveTransactionActivity {
                period: "5min".to_string(),
                current_count: total_5min,
                maximum_allowed: MAX_TRANSACTIONS_5MIN,
            });
        }

        // Check buy/sell ratio for healthy activity
        if buys_5min > 0 && sells_5min > 0 {
            let buy_sell_ratio = (buys_5min as f64) / (sells_5min as f64);

            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "DEBUG_TXN_ACTIVITY",
                    &format!(
                        "📊 Token {} buy/sell ratio: {:.2} (range: {:.1} - {:.1})",
                        token.symbol,
                        buy_sell_ratio,
                        MIN_BUY_SELL_RATIO,
                        MAX_BUY_SELL_RATIO
                    )
                );
            }

            if buy_sell_ratio < MIN_BUY_SELL_RATIO || buy_sell_ratio > MAX_BUY_SELL_RATIO {
                if is_debug_filtering_enabled() {
                    log(
                        LogTag::Filtering,
                        "DEBUG_TXN_ACTIVITY",
                        &format!(
                            "❌ Token {} unhealthy buy/sell ratio: {:.2} (acceptable: {:.1} - {:.1})",
                            token.symbol,
                            buy_sell_ratio,
                            MIN_BUY_SELL_RATIO,
                            MAX_BUY_SELL_RATIO
                        )
                    );
                }
                return Some(FilterReason::UnhealthyBuySellRatio {
                    buys: buys_5min,
                    sells: sells_5min,
                    ratio: buy_sell_ratio,
                    min_ratio: MIN_BUY_SELL_RATIO,
                    max_ratio: MAX_BUY_SELL_RATIO,
                });
            }
        }
    } else {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_TXN_ACTIVITY",
                &format!("📊 Token {} has no 5-minute transaction data", token.symbol)
            );
        }
        return Some(FilterReason::NoTransactionData);
    }

    // Check 1-hour transaction activity for additional validation
    if let Some(h1) = &txns.h1 {
        let buys_1h = h1.buys.unwrap_or(0);
        let sells_1h = h1.sells.unwrap_or(0);
        let total_1h = buys_1h + sells_1h;

        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_TXN_ACTIVITY",
                &format!(
                    "📊 Token {} 1hour activity: {} buys, {} sells, {} total (min: {})",
                    token.symbol,
                    buys_1h,
                    sells_1h,
                    total_1h,
                    MIN_TRANSACTIONS_1H
                )
            );
        }

        if total_1h < MIN_TRANSACTIONS_1H {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "DEBUG_TXN_ACTIVITY",
                    &format!(
                        "❌ Token {} insufficient 1hour activity: {} < {} minimum",
                        token.symbol,
                        total_1h,
                        MIN_TRANSACTIONS_1H
                    )
                );
            }
            return Some(FilterReason::InsufficientTransactionActivity {
                period: "1hour".to_string(),
                current_count: total_1h,
                minimum_required: MIN_TRANSACTIONS_1H,
            });
        }
    }

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_TXN_ACTIVITY",
            &format!(
                "✅ Token {} ({}) passed transaction activity validation",
                token.symbol,
                token.mint
            )
        );
    }

    None
}

/// Validate that token decimals are available in cached data (no RPC calls)
fn validate_decimal_availability(token: &Token) -> Option<FilterReason> {
    if token.decimals.is_none() {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_DECIMALS",
                &format!(
                    "❌ Token {} ({}) decimals not available in cached data",
                    token.symbol,
                    token.mint
                )
            );
        }
        return Some(FilterReason::DecimalsNotAvailable {
            mint: token.mint.clone(),
        });
    }

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_DECIMALS",
            &format!("✅ Token {} ({}) decimals are available", token.symbol, token.mint)
        );
    }

    None
}

// =============================================================================
// TOKEN SORTING AND SELECTION UTILITIES (moved from trader.rs)
// =============================================================================

/// Calculate 5-minute transaction activity for a token
pub fn get_token_5min_activity(token: &Token) -> i64 {
    token.txns
        .as_ref()
        .and_then(|t| t.m5.as_ref())
        .map(|m5| m5.buys.unwrap_or(0) + m5.sells.unwrap_or(0))
        .unwrap_or(0)
}

/// Count tokens with transaction data for logging
pub fn count_tokens_with_transaction_data(tokens: &[Token]) -> usize {
    tokens
        .iter()
        .filter(|token| get_token_5min_activity(token) > 0)
        .count()
}

/// Generate transaction activity statistics for a list of tokens
pub fn get_transaction_activity_stats(tokens: &[Token]) -> (i64, i64, f64, usize) {
    let txn_stats: Vec<i64> = tokens
        .iter()
        .map(|token| get_token_5min_activity(token))
        .collect();

    if txn_stats.is_empty() {
        return (0, 0, 0.0, 0);
    }

    let max_txns = *txn_stats.iter().max().unwrap_or(&0);
    let min_txns = *txn_stats.iter().min().unwrap_or(&0);
    let avg_txns = (txn_stats.iter().sum::<i64>() as f64) / (txn_stats.len() as f64);
    let tokens_with_10_plus = txn_stats
        .iter()
        .filter(|&&x| x >= 10)
        .count();

    (max_txns, min_txns, avg_txns, tokens_with_10_plus)
}

/// Log transaction activity statistics for processed tokens
pub fn log_transaction_activity_stats(tokens: &[Token]) {
    if !is_debug_filtering_enabled() || tokens.is_empty() {
        return;
    }

    let (max_txns, min_txns, avg_txns, tokens_with_10_plus) =
        get_transaction_activity_stats(tokens);

    log(
        LogTag::Filtering,
        "TXN_STATS",
        &format!(
            "📊 5min txn activity: max={}, min={}, avg={:.1}, ≥10txns: {}/{}",
            max_txns,
            min_txns,
            avg_txns,
            tokens_with_10_plus,
            tokens.len()
        )
    );
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Check if a specific token passes all filters (convenience function)
pub fn is_token_eligible_for_trading(token: &Token) -> bool {
    matches!(filter_token_for_trading(token), FilterResult::Approved)
}

/// Filter a list of tokens and return both eligible and rejected with reasons
pub fn filter_tokens_with_reasons(tokens: &[Token]) -> (Vec<Token>, Vec<(Token, FilterReason)>) {
    let (tokens_to_process, pre_filtered_rejected) = if
        tokens.len() > MAX_TOKENS_FOR_DETAILED_FILTERING
    {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "PERFORMANCE",
                &format!(
                    "⚡ Large token set detected: {} tokens. Prioritizing by data-availability score, then liquidity (cap {}).",
                    tokens.len(),
                    MAX_TOKENS_FOR_DETAILED_FILTERING
                )
            );
        }

        // Score tokens by cheap-to-compute signals that correlate with usable price history
        // Score components (0-5):
        //  - +1 has pool price now (price_pool_sol)
        //  - +1 has dexscreener price (price_dexscreener_sol)
        //  - +1 has m5 txn activity >= MIN_TRANSACTIONS_5MIN
        //  - +1 has m5 volume
        //  - +1 has non-zero liquidity USD
        let mut scored: Vec<(i32, f64, &Token)> = tokens
            .iter()
            .map(|t| {
                let mut score: i32 = 0;
                if t.price_pool_sol.unwrap_or(0.0) > 0.0 {
                    score += 1;
                }
                if t.price_dexscreener_sol.unwrap_or(0.0) > 0.0 {
                    score += 1;
                }
                // txn m5
                let m5_txn = t.txns
                    .as_ref()
                    .and_then(|x| x.m5.as_ref())
                    .map(|p| p.buys.unwrap_or(0) + p.sells.unwrap_or(0))
                    .unwrap_or(0);
                if (m5_txn as i64) >= MIN_TRANSACTIONS_5MIN {
                    score += 1;
                }
                // volume m5
                if
                    t.volume
                        .as_ref()
                        .and_then(|v| v.m5)
                        .unwrap_or(0.0) > 0.0
                {
                    score += 1;
                }
                // liquidity usd
                let liq = t.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);
                if liq > 0.0 {
                    score += 1;
                }

                (score, liq, t)
            })
            .collect();

        // Sort by score desc, then liquidity desc
        scored.sort_by(|a, b| {
            b.0.cmp(&a.0).then_with(|| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal))
        });

        // Take the top-N
        let mut top_tokens: Vec<Token> = Vec::with_capacity(MAX_TOKENS_FOR_DETAILED_FILTERING);
        for (_, _, tok) in scored.iter().take(MAX_TOKENS_FOR_DETAILED_FILTERING) {
            top_tokens.push((*tok).clone());
        }

        // Everything else is excluded for performance
        let mut excluded_tokens: Vec<(Token, FilterReason)> = Vec::new();
        for (_, _, tok) in scored.into_iter().skip(MAX_TOKENS_FOR_DETAILED_FILTERING) {
            excluded_tokens.push((
                tok.clone(),
                FilterReason::PerformanceLimitExceeded {
                    total_tokens: tokens.len(),
                    max_allowed: MAX_TOKENS_FOR_DETAILED_FILTERING,
                },
            ));
        }

        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "PERFORMANCE",
                &format!(
                    "📊 Performance limiting: Selected top {} by score→liquidity, excluded {}",
                    top_tokens.len(),
                    excluded_tokens.len()
                )
            );
        }

        (top_tokens, excluded_tokens)
    } else {
        (tokens.to_vec(), Vec::new())
    };

    let mut eligible = Vec::new();
    let mut rejected = pre_filtered_rejected; // Start with pre-filtered rejected tokens

    // FAST PRE-SCREEN: cheaply discard obvious rejects without verbose step-by-step logging
    // This dramatically reduces the workload of full filtering when token sets are large
    // Note: Skip liquidity, age, and decimals checks here since they're handled by early filtering
    let mut fast_pass: Vec<&Token> = Vec::with_capacity(tokens_to_process.len());
    for token in &tokens_to_process {
        // Only check price validity since liquidity, age, and decimals are pre-filtered
        if let Some(reason) = validate_price_data(token) {
            rejected.push((token.clone(), reason));
            continue;
        }
        // Passed fast pre-screen
        fast_pass.push(token);
    }

    // Now run the full filter only on pre-screened tokens (far fewer logs/work)
    for token in fast_pass {
        match filter_token_for_trading(token) {
            FilterResult::Approved => eligible.push(token.clone()),
            FilterResult::Rejected(reason) => rejected.push((token.clone(), reason)),
        }
    }

    // Log detailed breakdown only in debug mode
    if is_debug_filtering_enabled() && !tokens.is_empty() {
        log_filtering_breakdown(&rejected);
    }

    (eligible, rejected)
}

/// Count how many tokens would pass filtering without processing them all
pub fn count_eligible_tokens(tokens: &[Token]) -> usize {
    tokens
        .iter()
        .filter(|token| is_token_eligible_for_trading(token))
        .count()
}

/// Get filtering statistics for a list of tokens
pub fn get_filtering_stats(tokens: &[Token]) -> (usize, usize, f64) {
    let total = tokens.len();
    let eligible = count_eligible_tokens(tokens);
    let pass_rate = if total > 0 { ((eligible as f64) / (total as f64)) * 100.0 } else { 0.0 };

    (total, eligible, pass_rate)
}

/// Log filtering summary statistics
pub fn log_filtering_summary(tokens: &[Token]) {
    // Only log summary if we have tokens and debug is enabled
    if tokens.is_empty() || !is_debug_filtering_enabled() {
        return;
    }

    let (total, eligible, pass_rate) = get_filtering_stats(tokens);

    log(
        LogTag::Filtering,
        "SUMMARY",
        &format!(
            "📊 FILTERING SUMMARY: Processed {} tokens → {} eligible ({:.1}% pass rate)",
            total,
            eligible,
            pass_rate
        )
    );

    // Log detailed breakdown if we have rejections
    if eligible < total && total > 0 {
        let rejected_count = total - eligible;
        log(
            LogTag::Filtering,
            "SUMMARY",
            &format!(
                "🚫 Rejected {} tokens ({:.1}% rejection rate) - use --debug-filtering for details",
                rejected_count,
                ((rejected_count as f64) / (total as f64)) * 100.0
            )
        );
    }

    if eligible == 0 && total > 0 {
        log(
            LogTag::Filtering,
            "WARN",
            "⚠️ NO TOKENS PASSED FILTERING - Consider reviewing filter criteria or token sources"
        );

        // Log current filter parameters for debugging
        log(
            LogTag::Filtering,
            "WARN",
            &format!(
                "Current filters: Age {}s-{}s ({}h-{}h), Liquidity ${}+, Price {:.12}-{:.3} SOL",
                MIN_TOKEN_AGE_SECONDS,
                MAX_TOKEN_AGE_SECONDS,
                MIN_TOKEN_AGE_SECONDS / 3600,
                MAX_TOKEN_AGE_SECONDS / 3600,
                MIN_LIQUIDITY_USD,
                MIN_VALID_PRICE_SOL,
                MAX_VALID_PRICE_SOL
            )
        );
    } else if eligible > 0 {
        log(
            LogTag::Filtering,
            "SUMMARY",
            &format!("✅ {} tokens ready for trading evaluation", eligible)
        );
    }
}

/// Log detailed breakdown of rejection reasons (debug mode only)
fn log_filtering_breakdown(rejected: &[(Token, FilterReason)]) {
    if rejected.is_empty() || !is_debug_filtering_enabled() {
        return;
    }

    use std::collections::HashMap;
    let mut reason_counts: HashMap<String, usize> = HashMap::new();

    for (_, reason) in rejected {
        let reason_type = match reason {
            FilterReason::TokenBlacklisted { .. } | FilterReason::SystemOrStableToken => {
                "Blacklist/Exclusion"
            }
            | FilterReason::EmptySymbol
            | FilterReason::EmptyMint
            | FilterReason::EmptyLogoUrl
            | FilterReason::EmptyWebsite
            | FilterReason::EmptyDescription => "Invalid Metadata",
            | FilterReason::InvalidPrice
            | FilterReason::PriceTooLow { .. }
            | FilterReason::PriceTooHigh { .. }
            | FilterReason::MissingPriceData => "Price Issues",
            | FilterReason::ZeroLiquidity
            | FilterReason::InsufficientLiquidity { .. }
            | FilterReason::TooHighLiquidity { .. }
            | FilterReason::TooHighMarketCap { .. }
            | FilterReason::MissingLiquidityData => "Liquidity Issues",
            | FilterReason::TooYoung { .. }
            | FilterReason::TooOld { .. }
            | FilterReason::NoCreationDate => "Age Constraints",
            FilterReason::ExistingOpenPosition | FilterReason::MaxPositionsReached { .. } =>
                "Position Constraints",
            FilterReason::AccountFrozen | FilterReason::TokenAccountFrozen => "Account Issues",
            FilterReason::RugcheckRisk { .. } => "Security Risks",
            FilterReason::InsufficientLpLock { .. } => "LP Lock Security",
            FilterReason::WhaleConcentrationRisk { .. } => "Whale Concentration Risk",
            FilterReason::LockAcquisitionFailed => "System Errors",
            FilterReason::DecimalsNotAvailable { .. } => "Decimal Issues",
            | FilterReason::InsufficientTransactionActivity { .. }
            | FilterReason::ExcessiveTransactionActivity { .. }
            | FilterReason::UnhealthyBuySellRatio { .. }
            | FilterReason::NoTransactionData => "Transaction Activity Issues",
            FilterReason::PerformanceLimitExceeded { .. } => "Performance Limiting",
        };

        *reason_counts.entry(reason_type.to_string()).or_insert(0) += 1;
    }

    log(LogTag::Filtering, "DEBUG", &format!("Rejection breakdown: {:?}", reason_counts));
}
