use crate::global::is_debug_filtering_enabled;
use crate::logger::{ log, LogTag };
/// Centralized token filtering system for ScreenerBot
/// All token filtering logic consolidated into a single function
/// No structs or models - pure functional approach
use crate::tokens::Token;
use crate::tokens::{
    get_token_decimals_sync,
    is_token_excluded_from_trading,
    rugcheck::{ get_high_risk_issues, is_token_safe_for_trading },
    TokenDatabase,
};
use crate::trader::MAX_OPEN_POSITIONS;
use chrono::{ Duration as ChronoDuration, Utc };

// =============================================================================
// FILTERING CONFIGURATION PARAMETERS (CENTRALIZED FOR EASY ACCESS)
// =============================================================================
//
// 🚀 QUICK PARAMETER REFERENCE:
//   - MIN_TOKEN_AGE_SECONDS = 3600 (tokens must be at least 1 hour old)
//   - MAX_TOKEN_AGE_SECONDS = 2592000 (30 days max age)
//   - Position re-entry cooldown moved to positions.rs (centralized)
//   - MIN_LIQUIDITY_USD = 1000.0 (minimum liquidity requirement)
//   - MIN_LP_LOCK_PERCENTAGE = 80.0 (minimum LP lock requirement)
//   - Note: ATH checking moved to trader for intelligent analysis
//
// 🔧 TO ADJUST TRADING BEHAVIOR:
//   - Make more aggressive: Lower minimums, shorter cooldowns
//   - Make more conservative: Higher minimums, longer cooldowns
//   - Adjust risk tolerance: Modify rugcheck and LP lock parameters
//
// =============================================================================

// ===== AGE FILTERING PARAMETERS =====
/// Minimum token age in seconds before trading
/// REDUCED: Allow newer tokens to catch fresh opportunities
pub const MIN_TOKEN_AGE_SECONDS: i64 = 0; // 2 hours - allow newer gems

/// Maximum token age in seconds
/// Extended to catch both new gems and established tokens
pub const MAX_TOKEN_AGE_SECONDS: i64 = 24 * 30 * 24 * 60 * 60; // 2 years for bigger range

// ===== PRICE ACTION FILTERING PARAMETERS =====

/// Minimum price in SOL to consider valid
pub const MIN_VALID_PRICE_SOL: f64 = 0.0000000000001;

/// Maximum price in SOL to avoid (prevents overflow issues)
pub const MAX_VALID_PRICE_SOL: f64 = 0.1;

// ===== LIQUIDITY FILTERING PARAMETERS =====
/// Minimum liquidity in USD required for trading
/// ULTRA AGGRESSIVE FOR MOONSHOT HUNTING: Reduced to $1 to catch legendary gems
pub const MIN_LIQUIDITY_USD: f64 = 1.0; // LEGENDARY MOONSHOT MODE: Catch ANY gem with >$1!

/// Maximum liquidity in USD - EXCLUDE BIG STABLE TOKENS that won't moon
/// MOONSHOT FOCUS: Cap at $75K to avoid large, stable tokens with low volatility
pub const MAX_LIQUIDITY_USD: f64 = 500_000.0; // Focus on micro-caps with moonshot potential!

/// MARKET CAP FILTERING - Avoid large market cap tokens that won't have big moves
/// Maximum market cap in USD to focus on micro-cap gems
pub const MAX_MARKET_CAP_USD: f64 = 50_000_000_000.0; // $500K max market cap for moonshot hunting

/// Minimum volume/liquidity ratio for activity detection
/// High ratio indicates active trading despite small liquidity (pump signals)
pub const MIN_VOLUME_LIQUIDITY_RATIO: f64 = 0.1; // 10% minimum volume/liquidity ratio

// ===== TRANSACTION ACTIVITY FILTERING PARAMETERS =====
/// Minimum transaction count in 5 minutes for trading eligibility
/// MODERATE MODE: Reasonable threshold for real activity
pub const MIN_TRANSACTIONS_5MIN: i64 = 20; // Minimum 5 transactions in 5 minutes

/// Maximum transaction count in 5 minutes to avoid overly pumped tokens
/// INCREASED: Allow higher activity for popular tokens
pub const MAX_TRANSACTIONS_5MIN: i64 = 2000; // Cap at 800 transactions in 5 minutes

/// Minimum transaction count in 1 hour for established activity
pub const MIN_TRANSACTIONS_1H: i64 = 15; // Minimum 15 transactions in 1 hour

/// Minimum buy/sell ratio for balanced activity (healthy trading)
/// Range: 0.4 (40% buys) to 2.5 (2.5x more buys than sells)
pub const MIN_BUY_SELL_RATIO: f64 = 0.4; // At least 40% buys vs sells (more balanced)

/// Maximum buy/sell ratio to avoid pure pump scenarios
pub const MAX_BUY_SELL_RATIO: f64 = 2.5; // Max 2.5x more buys than sells (more balanced)

// ===== RUGCHECK SECURITY PARAMETERS =====
/// IMPORTANT: Rugcheck scores are RISK scores - higher values mean MORE risk, not less!
/// Maximum allowed rugcheck risk score (0-100 scale) - HIGHER MEANS MORE RISKY
/// LEGENDARY MOONSHOT MODE: Accept maximum risk for legendary gains
pub const MAX_RUGCHECK_RISK_SCORE: i32 = 100; // Accept ANY risk for moonshot potential!

/// Emergency override for very risky tokens - any score above this is automatically rejected
pub const EMERGENCY_MAX_RISK_SCORE: i32 = 100; // No emergency limit - we're fearless!

/// Maximum number of critical-risk issues to tolerate
pub const MAX_CRITICAL_RISK_ISSUES: usize = 5; // Accept critical issues for moonshot potential

// ===== LP LOCK SECURITY PARAMETERS =====
/// Minimum percentage of LP tokens that must be locked
pub const MIN_LP_LOCK_PERCENTAGE: f64 = 80.0;

/// Minimum percentage for new/risky tokens
pub const MIN_LP_LOCK_PERCENTAGE_NEW_TOKENS: f64 = 80.0;

// ===== HISTORICAL PERFORMANCE PARAMETERS =====
/// Maximum acceptable loss rate for historical performance (0.0-1.0)
pub const MAX_HISTORICAL_LOSS_RATE: f64 = 0.8; // 80% loss rate

/// Maximum acceptable average loss percentage
pub const MAX_AVERAGE_LOSS_PERCENTAGE: f64 = 50.0; // 50% average loss

// ===== VALIDATION TIMEOUTS =====
/// Maximum time to wait for database locks (milliseconds)
pub const DB_LOCK_TIMEOUT_MS: u64 = 5000;

/// Maximum time to wait for price history locks (milliseconds)
pub const PRICE_HISTORY_LOCK_TIMEOUT_MS: u64 = 3000;

// =============================================================================
// END OF FILTERING PARAMETERS
// =============================================================================
// 📋 PARAMETER CATEGORIES SUMMARY:
//   • Age Filtering: Controls token age requirements (1 hour - 30 days)
//   • Position Management: Prevents conflicts and overexposure
//   • Price Action: Avoids buying near peaks, validates price ranges
//   • Liquidity: Ensures sufficient trading volume
//   • Security: Rugcheck and LP lock safety requirements
//   • Decimal Validation: Ensures token decimals are available for calculations
//   • Performance: Logging, timeouts, and system limits
// =============================================================================

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

    // Position-related failures
    ExistingOpenPosition,
    RecentlyClosed {
        minutes_ago: i64,
        cooldown_minutes: i64,
    },
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
    LPLockRisk {
        lock_percentage: f64,
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
// MAIN FILTERING FUNCTION
// =============================================================================

/// Centralized token filtering function
/// Returns FilterResult::Approved if token passes all filters
/// Returns FilterResult::Rejected(reason) if token fails any filter
pub fn filter_token_for_trading(token: &Token) -> FilterResult {
    // Entry debug log with token basic info
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "START_FILTER",
            &format!(
                "🔍 Filtering token: {} ({}), Price: {:.10} SOL, Age: {}h, Liquidity: ${:.2}",
                token.symbol,
                &token.mint[..8],
                token.price_dexscreener_sol.unwrap_or(0.0),
                token.created_at.map_or("Unknown".to_string(), |created| {
                    let age = (Utc::now() - created).num_hours();
                    age.to_string()
                }),
                token.liquidity.as_ref().map_or(0.0, |l| l.usd.unwrap_or(0.0))
            )
        );
    }

    // 0. BLACKLIST AND EXCLUSION CHECK (ABSOLUTE FIRST - HIGHEST PRIORITY)
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_0",
            &format!("🚫 Step 0: Checking blacklist/exclusion for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_blacklist_exclusion(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_0",
                &format!("❌ {}: FAILED Step 0 (Blacklist/Exclusion) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_0",
            &format!("✅ {}: PASSED Step 0 (Blacklist/Exclusion)", token.symbol)
        );
    }

    // 1. RUGCHECK SECURITY VALIDATION (SECOND - HIGHEST PRIORITY)
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_1",
            &format!("🛡️ Step 1: Checking rugcheck security for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_rugcheck_risks(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_1",
                &format!("❌ {}: FAILED Step 1 (Rugcheck) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_1",
            &format!("✅ {}: PASSED Step 1 (Rugcheck)", token.symbol)
        );
    }

    // 2. Basic metadata validation
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_2",
            &format!("📝 Step 2: Checking metadata for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_basic_token_info(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_2",
                &format!("❌ {}: FAILED Step 2 (Metadata) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_2",
            &format!("✅ {}: PASSED Step 2 (Metadata)", token.symbol)
        );
    }

    // 3. Age validation
    if is_debug_filtering_enabled() {
        log(LogTag::Filtering, "STEP_3", &format!("⏰ Step 3: Checking age for {}", token.symbol));
    }
    if let Some(reason) = validate_token_age(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_3",
                &format!("❌ {}: FAILED Step 3 (Age) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(LogTag::Filtering, "PASS_STEP_3", &format!("✅ {}: PASSED Step 3 (Age)", token.symbol));
    }

    // 4. Liquidity validation
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_4",
            &format!("💧 Step 4: Checking liquidity for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_liquidity(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_4",
                &format!("❌ {}: FAILED Step 4 (Liquidity) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_4",
            &format!("✅ {}: PASSED Step 4 (Liquidity)", token.symbol)
        );
    }

    // 5. Holder Distribution Validation (CRITICAL FOR MICRO-CAPS)
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_5",
            &format!("👥 Step 5: Checking holder distribution for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_holder_distribution(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_5",
                &format!("❌ {}: FAILED Step 5 (Holder Distribution) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_5",
            &format!("✅ {}: PASSED Step 5 (Holder Distribution)", token.symbol)
        );
    }

    // 6. Basic Price Validation (Simplified - ATH checking moved to trader)
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_6",
            &format!("📈 Step 6: Checking basic price validity for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_basic_price_data(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_6",
                &format!("❌ {}: FAILED Step 6 (Price Validation) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_6",
            &format!("✅ {}: PASSED Step 6 (Price Validation)", token.symbol)
        );
    }

    // 7. Price validation
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_7",
            &format!("💰 Step 7: Checking price data for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_price_data(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_7",
                &format!("❌ {}: FAILED Step 7 (Price Data) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_7",
            &format!("✅ {}: PASSED Step 7 (Price Data)", token.symbol)
        );
    }

    // 8. Transaction Activity Validation (NEW - moved from trader.rs)
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_8",
            &format!("📊 Step 8: Checking transaction activity for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_transaction_activity(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_8",
                &format!("❌ {}: FAILED Step 8 (Transaction Activity) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_8",
            &format!("✅ {}: PASSED Step 8 (Transaction Activity)", token.symbol)
        );
    }

    // 9. Decimal availability validation
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_9",
            &format!("🔢 Step 9: Checking decimal availability for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_decimal_availability(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_9",
                &format!("❌ {}: FAILED Step 9 (Decimal Availability) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_9",
            &format!("✅ {}: PASSED Step 9 (Decimal Availability)", token.symbol)
        );
    }

    // 10. Position constraints validation
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_10",
            &format!("🔒 Step 10: Checking position constraints for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_position_constraints(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_10",
                &format!(
                    "❌ {}: FAILED Step 10 (Position Constraints) - {:?}",
                    token.symbol,
                    reason
                )
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_10",
            &format!("✅ {}: PASSED Step 10 (Position Constraints)", token.symbol)
        );
    }

    // Token passed all filters
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "ALL_STEPS_PASSED",
            &format!("🎉 {}: PASSED ALL 10 FILTERING STEPS - ELIGIBLE FOR TRADING", token.symbol)
        );
    }

    FilterResult::Approved
}

// =============================================================================
// INDIVIDUAL FILTER FUNCTIONS
// =============================================================================

/// Validate basic price data (simplified price validation)
fn validate_basic_price_data(token: &Token) -> Option<FilterReason> {
    // Basic price validation - much simpler than before
    let current_price = token.price_dexscreener_sol.unwrap_or(0.0);

    if current_price <= 0.0 || !current_price.is_finite() {
        return Some(FilterReason::InvalidPrice);
    }

    if current_price < MIN_VALID_PRICE_SOL {
        return Some(FilterReason::PriceTooLow {
            current_price,
            minimum_price: MIN_VALID_PRICE_SOL,
        });
    }

    if current_price > MAX_VALID_PRICE_SOL {
        return Some(FilterReason::PriceTooHigh {
            current_price,
            maximum_price: MAX_VALID_PRICE_SOL,
        });
    }

    None
}

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
                    &format!("🚫 Token {} is a system or stable token - excluded", token.symbol)
                );
            }
            return Some(FilterReason::SystemOrStableToken);
        } else {
            // It's in the dynamic blacklist
            let reason_description = if let Some(stats) = crate::tokens::get_blacklist_stats() {
                format!("Blacklisted (total: {})", stats.total_blacklisted)
            } else {
                "Blacklisted".to_string()
            };

            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "DEBUG_BLACKLIST",
                    &format!("🚫 Token {} is blacklisted - {}", token.symbol, reason_description)
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
            &format!("✅ Token {} passed blacklist/exclusion check", token.symbol)
        );
    }

    None
}

/// Validate rugcheck security risks (HIGHEST PRIORITY - RUNS FIRST)
fn validate_rugcheck_risks(token: &Token) -> Option<FilterReason> {
    use crate::tokens::get_global_rugcheck_service;

    // Get rugcheck data using global service if available, fallback to database
    let rugcheck_data = match get_global_rugcheck_service() {
        Some(service) => {
            // Use blocking call to access async service from sync context
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    match service.get_rugcheck_data(&token.mint).await {
                        Ok(Some(data)) => Some(data),
                        Ok(None) => {
                            if is_debug_filtering_enabled() {
                                log(
                                    LogTag::Filtering,
                                    "RUGCHECK_MISSING",
                                    &format!("No rugcheck data for token: {}", token.symbol)
                                );
                            }
                            None
                        }
                        Err(e) => {
                            if is_debug_filtering_enabled() {
                                log(
                                    LogTag::Filtering,
                                    "ERROR",
                                    &format!(
                                        "Failed to get rugcheck data for {}: {}",
                                        token.symbol,
                                        e
                                    )
                                );
                            }
                            None
                        }
                    }
                })
            })
        }
        None => {
            // Fallback to direct database access if service not available
            let database = match TokenDatabase::new() {
                Ok(db) => db,
                Err(e) => {
                    if is_debug_filtering_enabled() {
                        log(
                            LogTag::Filtering,
                            "ERROR",
                            &format!("Failed to connect to database for rugcheck: {}", e)
                        );
                    }
                    return None; // Skip validation if database unavailable
                }
            };

            match database.get_rugcheck_data(&token.mint) {
                Ok(Some(data)) => Some(data),
                Ok(None) => {
                    if is_debug_filtering_enabled() {
                        log(
                            LogTag::Filtering,
                            "RUGCHECK_MISSING",
                            &format!("No rugcheck data for token: {}", token.symbol)
                        );
                    }
                    None
                }
                Err(e) => {
                    if is_debug_filtering_enabled() {
                        log(
                            LogTag::Filtering,
                            "ERROR",
                            &format!("Failed to get rugcheck data for {}: {}", token.symbol, e)
                        );
                    }
                    None
                }
            }
        }
    };

    let rugcheck_data = match rugcheck_data {
        Some(data) => data,
        None => {
            return None;
        } // No rugcheck data available - let it pass
    };

    // CRITICAL: Hard-coded risk score check - HIGHER SCORES MEAN MORE RISK!
    if let Some(risk_score) = rugcheck_data.score_normalised.or(rugcheck_data.score) {
        // Emergency override - immediately reject very risky tokens
        if risk_score >= EMERGENCY_MAX_RISK_SCORE {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "EMERGENCY_RISK",
                    &format!(
                        "Token {} EMERGENCY REJECTED - Risk score {} >= {} (VERY HIGH RISK)",
                        token.symbol,
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

        // Standard risk score check
        if risk_score > MAX_RUGCHECK_RISK_SCORE {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "HIGH_RISK_SCORE",
                    &format!(
                        "Token {} rejected - Risk score {} > {} (HIGH RISK)",
                        token.symbol,
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

        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "RISK_SCORE_OK",
                &format!(
                    "Token {} risk score {} <= {} (acceptable risk)",
                    token.symbol,
                    risk_score,
                    MAX_RUGCHECK_RISK_SCORE
                )
            );
        }
    }

    // Check if token is safe for trading (uses additional analysis beyond score)
    if !is_token_safe_for_trading(&rugcheck_data) {
        let risk_issues = get_high_risk_issues(&rugcheck_data);
        let risk_level = if rugcheck_data.rugged.unwrap_or(false) {
            "CRITICAL"
        } else if rugcheck_data.score_normalised.unwrap_or(0) >= 50 {
            // CORRECTED: high score = high risk
            "HIGH"
        } else {
            "MEDIUM"
        };

        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "RUGCHECK_FAIL",
                &format!(
                    "Token {} failed rugcheck validation. Risk level: {}, Issues: {:?}",
                    token.symbol,
                    risk_level,
                    risk_issues
                )
            );
        }

        return Some(FilterReason::RugcheckRisk {
            risk_level: risk_level.to_string(),
            reasons: risk_issues,
        });
    }

    if is_debug_filtering_enabled() {
        let score = rugcheck_data.score_normalised.or(rugcheck_data.score).unwrap_or(0);
        log(
            LogTag::Filtering,
            "RUGCHECK_PASS",
            &format!("Token {} passed rugcheck validation (risk score: {})", token.symbol, score)
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
                if token.mint.len() > 8 {
                    &token.mint[..8]
                } else {
                    &token.mint
                },
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

    // Check description - Make this optional since DexScreener API doesn't provide it
    // Only warn about missing description but don't reject the token
    // if
    //     token.description.is_none() ||
    //     token.description.as_ref().map_or(true, |desc| desc.trim().is_empty())
    // {
    //     if is_debug_filtering_enabled() {
    //         log(
    //             LogTag::Filtering,
    //             "DEBUG_META",
    //             "⚠️ Description is missing (not required for DexScreener tokens)"
    //         );
    //     }
    //     // Don't return FilterReason::EmptyDescription - just log it
    // }

    None
}

/// Validate token age constraints
fn validate_token_age(token: &Token) -> Option<FilterReason> {
    let Some(created_at) = token.created_at else {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_AGE",
                &format!("⏰ Token {} has no creation date", token.symbol)
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
                "⏰ Age check for {}: {}h {}m old (created: {}), min: {}s ({}h), max: {}s ({}h)",
                token.symbol,
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
                    "❌ Token {} too young: {}s < {}s minimum",
                    token.symbol,
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
                    "❌ Token {} too old: {}s > {}s maximum",
                    token.symbol,
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
            &format!("✅ Token {} age within acceptable range", token.symbol)
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
                &format!("💧 Token {} has no liquidity data", token.symbol)
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
                "💧 Liquidity check for {}: ${:.2} (min: ${:.2}, max: ${:.2})",
                token.symbol,
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
                &format!("❌ Token {} has zero liquidity", token.symbol)
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

/// Validate holder distribution to prevent whale concentration risk
/// CRITICAL FOR MICRO-CAPS: Ensure no single holder can cause >20% loss
fn validate_holder_distribution(token: &Token) -> Option<FilterReason> {
    use crate::tokens::get_global_rugcheck_service;

    // Get rugcheck data using global service if available, fallback to database
    let rugcheck_data = match get_global_rugcheck_service() {
        Some(service) => {
            // Use blocking call to access async service from sync context
            tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    match service.get_rugcheck_data(&token.mint).await {
                        Ok(Some(data)) => Some(data),
                        Ok(None) => {
                            if is_debug_filtering_enabled() {
                                log(
                                    LogTag::Filtering,
                                    "DEBUG_HOLDERS",
                                    &format!("No rugcheck/holder data for token: {}", token.symbol)
                                );
                            }
                            None
                        }
                        Err(e) => {
                            if is_debug_filtering_enabled() {
                                log(
                                    LogTag::Filtering,
                                    "DEBUG_HOLDERS",
                                    &format!(
                                        "Failed to get holder data for {}: {}",
                                        token.symbol,
                                        e
                                    )
                                );
                            }
                            None
                        }
                    }
                })
            })
        }
        None => {
            // Fallback to direct database access if service not available
            let database = match TokenDatabase::new() {
                Ok(db) => db,
                Err(e) => {
                    if is_debug_filtering_enabled() {
                        log(
                            LogTag::Filtering,
                            "DEBUG_HOLDERS",
                            &format!("Failed to connect to database for holders: {}", e)
                        );
                    }
                    return None; // Skip validation if database unavailable
                }
            };

            match database.get_rugcheck_data(&token.mint) {
                Ok(Some(data)) => Some(data),
                Ok(None) => {
                    if is_debug_filtering_enabled() {
                        log(
                            LogTag::Filtering,
                            "DEBUG_HOLDERS",
                            &format!("No rugcheck/holder data for token: {}", token.symbol)
                        );
                    }
                    None
                }
                Err(e) => {
                    if is_debug_filtering_enabled() {
                        log(
                            LogTag::Filtering,
                            "DEBUG_HOLDERS",
                            &format!("Failed to get holder data for {}: {}", token.symbol, e)
                        );
                    }
                    None
                }
            }
        }
    };

    let rugcheck_data = match rugcheck_data {
        Some(data) => data,
        None => {
            return None;
        } // No holder data - allow through (better than blocking)
    };

    // Check top holders for concentration risk
    if let Some(top_holders) = &rugcheck_data.top_holders {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_HOLDERS",
                &format!("Analyzing {} top holders for {}", top_holders.len(), token.symbol)
            );
        }

        let mut total_top_holder_percentage = 0.0;
        let mut dangerous_holders = Vec::new();

        for (i, holder) in top_holders.iter().take(10).enumerate() {
            if let Some(pct) = holder.pct {
                total_top_holder_percentage += pct;

                // Flag holders with >20% as dangerous for micro-caps
                if pct > 20.0 {
                    dangerous_holders.push((i + 1, pct));
                }

                if is_debug_filtering_enabled() {
                    log(
                        LogTag::Filtering,
                        "DEBUG_HOLDERS",
                        &format!("Holder #{}: {:.2}% (address: {})", i + 1, pct, &holder.address)
                    );
                }
            }
        }

        // For micro-cap gems, be more lenient but still prevent obvious whale concentration
        let liquidity_usd = token.liquidity
            .as_ref()
            .and_then(|l| l.usd)
            .unwrap_or(0.0);

        // Much more lenient thresholds for micro-caps since they naturally have higher concentration
        let (max_single_holder, max_top5_total) = if liquidity_usd < 1000.0 {
            (85.0, 95.0) // Micro-caps: max 85% single, 95% top 5 (very lenient for new tokens)
        } else if liquidity_usd < 10000.0 {
            (70.0, 90.0) // Small caps: max 70% single, 90% top 5
        } else if liquidity_usd < 50000.0 {
            (60.0, 85.0) // Medium caps: max 60% single, 85% top 5
        } else {
            (40.0, 75.0) // Larger tokens: max 40% single, 75% top 5
        };

        // Check for dangerous single holders
        for (rank, pct) in &dangerous_holders {
            if *pct > max_single_holder {
                if is_debug_filtering_enabled() {
                    log(
                        LogTag::Filtering,
                        "DEBUG_HOLDERS",
                        &format!(
                            "❌ Token {} rejected: Holder #{} has {:.2}% (max allowed: {:.1}%)",
                            token.symbol,
                            rank,
                            pct,
                            max_single_holder
                        )
                    );
                }
                return Some(FilterReason::WhaleConcentrationRisk {
                    holder_rank: *rank,
                    percentage: *pct,
                    max_allowed: max_single_holder,
                });
            }
        }

        // Check total top 5 concentration
        let top5_total: f64 = top_holders
            .iter()
            .take(5)
            .filter_map(|h| h.pct)
            .sum();

        if top5_total > max_top5_total {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "DEBUG_HOLDERS",
                    &format!(
                        "❌ Token {} rejected: Top 5 holders control {:.2}% (max allowed: {:.1}%)",
                        token.symbol,
                        top5_total,
                        max_top5_total
                    )
                );
            }
            return Some(FilterReason::WhaleConcentrationRisk {
                holder_rank: 0, // 0 indicates top-5 total
                percentage: top5_total,
                max_allowed: max_top5_total,
            });
        }

        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_HOLDERS",
                &format!(
                    "✅ Token {} holder distribution acceptable: Top holder: {:.2}%, Top 5: {:.2}%",
                    token.symbol,
                    dangerous_holders
                        .first()
                        .map(|(_, pct)| *pct)
                        .unwrap_or(0.0),
                    top5_total
                )
            );
        }
    } else {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_HOLDERS",
                &format!("No top holder data available for {}, allowing through", token.symbol)
            );
        }
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
                "💰 Price check for {}: {:.10} SOL (range: {:.12} - {:.3} SOL)",
                token.symbol,
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
                &format!("📊 Token {} has no transaction data", token.symbol)
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
            &format!("✅ Token {} passed transaction activity validation", token.symbol)
        );
    }

    None
}

/// Validate that token decimals are available (critical for trading calculations)
fn validate_decimal_availability(token: &Token) -> Option<FilterReason> {
    // Use the synchronous decimal access function to check if decimals are available
    // This checks both cache and can fallback to blockchain if needed
    if get_token_decimals_sync(&token.mint).is_none() {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_DECIMALS",
                &format!("❌ Token {} decimals not available in cache or blockchain", token.symbol)
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
            &format!("✅ Token {} decimals are available", token.symbol)
        );
    }

    None
}

/// Validate position-related constraints
/// Validate position-related constraints
/// Note: Position validation is deferred to async trading decision point
/// to avoid runtime blocking issues in high-throughput filtering
fn validate_position_constraints(_token: &Token) -> Option<FilterReason> {
    // Position constraints are now validated at trading decision point
    // to avoid "Cannot start a runtime from within a runtime" errors
    // when filtering large numbers of tokens in async context

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_POSITION",
            &format!("✅ {}: Position constraints deferred to trading decision", _token.symbol)
        );
    }

    // Always pass at filtering stage - position checks happen during trading
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
    // Performance fix: Limit tokens to prevent timeout on large datasets
    // SMART PRIORITIZATION: prefer tokens likely to have usable data (price, txns, volume)
    const MAX_TOKENS_FOR_DETAILED_FILTERING: usize = 15000;

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
    let mut fast_pass: Vec<&Token> = Vec::with_capacity(tokens_to_process.len());
    for token in &tokens_to_process {
        // Cheap checks (no logging): price validity, liquidity, age, decimals
        if let Some(reason) = validate_basic_price_data(token) {
            rejected.push((token.clone(), reason));
            continue;
        }
        if let Some(reason) = validate_liquidity(token) {
            rejected.push((token.clone(), reason));
            continue;
        }
        if let Some(reason) = validate_token_age(token) {
            rejected.push((token.clone(), reason));
            continue;
        }
        if let Some(reason) = validate_decimal_availability(token) {
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
            | FilterReason::ExistingOpenPosition
            | FilterReason::RecentlyClosed { .. }
            | FilterReason::MaxPositionsReached { .. } => "Position Constraints",
            FilterReason::AccountFrozen | FilterReason::TokenAccountFrozen => "Account Issues",
            FilterReason::RugcheckRisk { .. } => "Security Risks",
            FilterReason::LPLockRisk { .. } => "LP Lock Security",
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
