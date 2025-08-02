/// Centralized token filtering system for ScreenerBot
/// All token filtering logic consolidated into a single function
/// No structs or models - pure functional approach

use crate::tokens::Token;
use crate::tokens::{
    TokenDatabase,
    rugcheck::{ is_token_safe_for_trading, get_high_risk_issues },
    get_token_decimals_sync,
};
use crate::logger::{ log, LogTag };
use crate::global::is_debug_filtering_enabled;
use crate::positions::SAVED_POSITIONS;
use crate::trader::MAX_OPEN_POSITIONS;
use chrono::{ Duration as ChronoDuration, Utc };

// =============================================================================
// FILTERING CONFIGURATION PARAMETERS (CENTRALIZED FOR EASY ACCESS)
// =============================================================================
//
// 🚀 QUICK PARAMETER REFERENCE:
//   - MIN_TOKEN_AGE_HOURS = 1 (tokens must be at least 1 hour old)
//   - MAX_TOKEN_AGE_HOURS = 720 (30 days max age)
//   - POSITION_CLOSE_COOLDOWN_MINUTES = 1440 (24 hour cooldown)
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
/// Minimum token age in hours before trading
pub const MIN_TOKEN_AGE_HOURS: i64 = 1;

/// Maximum token age in hours (effectively unlimited)
pub const MAX_TOKEN_AGE_HOURS: i64 = 2 * 30 * 24; // 30 days

// ===== POSITION MANAGEMENT PARAMETERS =====
/// Cooldown period after closing position before re-entering same token (minutes)
pub const POSITION_CLOSE_COOLDOWN_MINUTES: i64 = 24 * 60; // 24 hours

// Note: MAX_OPEN_POSITIONS is imported from trader module above

// ===== PRICE ACTION FILTERING PARAMETERS =====
// Note: ATH checking moved to trader for more intelligent analysis

/// Minimum price in SOL to consider valid
pub const MIN_VALID_PRICE_SOL: f64 = 0.000000000001;

/// Maximum price in SOL to avoid (prevents overflow issues)
pub const MAX_VALID_PRICE_SOL: f64 = 0.1;

// ===== LIQUIDITY FILTERING PARAMETERS =====
/// Minimum liquidity in USD required for trading
pub const MIN_LIQUIDITY_USD: f64 = 1000.0;

/// Preferred minimum liquidity in USD for safer trading
pub const PREFERRED_MIN_LIQUIDITY_USD: f64 = 5000.0;

// ===== RUGCHECK SECURITY PARAMETERS =====
/// IMPORTANT: Rugcheck scores are RISK scores - higher values mean MORE risk, not less!
/// Maximum allowed rugcheck risk score (0-100 scale) - HIGHER MEANS MORE RISKY
/// This threshold overrides all other rugcheck analysis and immediately rejects high-risk tokens
pub const MAX_RUGCHECK_RISK_SCORE: i32 = 35; // Allow max 20 risk score (low-medium risk)

/// Emergency override for very risky tokens - any score above this is automatically rejected
pub const EMERGENCY_MAX_RISK_SCORE: i32 = 65; // Absolute maximum risk tolerance

/// Maximum number of high-risk issues to tolerate
pub const MAX_HIGH_RISK_ISSUES: usize = 1;

/// Maximum number of critical-risk issues to tolerate
pub const MAX_CRITICAL_RISK_ISSUES: usize = 0;

// ===== LP LOCK SECURITY PARAMETERS =====
/// Minimum percentage of LP tokens that must be locked
pub const MIN_LP_LOCK_PERCENTAGE: f64 = 80.0;

/// Minimum percentage for new/risky tokens
pub const MIN_LP_LOCK_PERCENTAGE_NEW_TOKENS: f64 = 90.0;

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

    // Trading requirements
    LockAcquisitionFailed,

    // Decimal validation failures
    DecimalsNotAvailable {
        mint: String,
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

    // 1. RUGCHECK SECURITY VALIDATION (FIRST - HIGHEST PRIORITY)
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

    // 5. Basic Price Validation (Simplified - ATH checking moved to trader)
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_5",
            &format!("📈 Step 5: Checking basic price validity for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_basic_price_data(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_5",
                &format!("❌ {}: FAILED Step 5 (Price Validation) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_5",
            &format!("✅ {}: PASSED Step 5 (Price Validation)", token.symbol)
        );
    }

    // 6. Price validation
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_6",
            &format!("💰 Step 6: Checking price data for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_price_data(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_6",
                &format!("❌ {}: FAILED Step 6 (Price Data) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_6",
            &format!("✅ {}: PASSED Step 6 (Price Data)", token.symbol)
        );
    }

    // 7. Decimal availability validation
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_7",
            &format!("🔢 Step 7: Checking decimal availability for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_decimal_availability(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_7",
                &format!("❌ {}: FAILED Step 7 (Decimal Availability) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_7",
            &format!("✅ {}: PASSED Step 7 (Decimal Availability)", token.symbol)
        );
    }

    // 8. Position-related validation
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "STEP_8",
            &format!("🔒 Step 8: Checking position constraints for {}", token.symbol)
        );
    }
    if let Some(reason) = validate_position_constraints(token) {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "REJECT_STEP_8",
                &format!("❌ {}: FAILED Step 8 (Position Constraints) - {:?}", token.symbol, reason)
            );
        }
        return FilterResult::Rejected(reason);
    }
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "PASS_STEP_8",
            &format!("✅ {}: PASSED Step 8 (Position Constraints)", token.symbol)
        );
    }

    // Token passed all filters
    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "ALL_STEPS_PASSED",
            &format!("🎉 {}: PASSED ALL 8 FILTERING STEPS - ELIGIBLE FOR TRADING", token.symbol)
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

/// Validate rugcheck security risks (HIGHEST PRIORITY - RUNS FIRST)
fn validate_rugcheck_risks(token: &Token) -> Option<FilterReason> {
    // Create database connection for rugcheck data lookup
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

    // Get rugcheck data from database
    let rugcheck_data = match database.get_rugcheck_data(&token.mint) {
        Ok(Some(data)) => data,
        Ok(None) => {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "RUGCHECK_MISSING",
                    &format!("No rugcheck data for token: {}", token.symbol)
                );
            }
            return None; // No rugcheck data available - let it pass
        }
        Err(e) => {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "ERROR",
                    &format!("Failed to get rugcheck data for {}: {}", token.symbol, e)
                );
            }
            return None; // Database error - let it pass
        }
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

    // Check logo URL
    // if
    //     token.logo_url.is_none() ||
    //     token.logo_url.as_ref().map_or(true, |url| url.trim().is_empty())
    // {
    //     if is_debug_filtering_enabled() {
    //         log(LogTag::Filtering, "DEBUG_META", "❌ Logo URL is missing or empty");
    //     }
    //     return Some(FilterReason::EmptyLogoUrl);
    // }

    // // Check website - can be from direct field or from info.websites
    // let has_website =
    //     token.website.as_ref().map_or(false, |w| !w.trim().is_empty()) ||
    //     token.info
    //         .as_ref()
    //         .map_or(false, |info| {
    //             !info.websites.is_empty() && info.websites.iter().any(|w| !w.url.trim().is_empty())
    //         });

    // if !has_website {
    //     if is_debug_filtering_enabled() {
    //         log(LogTag::Filtering, "DEBUG_META", "❌ Website is missing or empty");
    //     }
    //     return Some(FilterReason::EmptyWebsite);
    // }

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

    // if is_debug_filtering_enabled() {
    //     log(LogTag::Filtering, "DEBUG_META", "✅ All metadata checks passed");
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
    let age_hours = token_age.num_hours();
    let age_minutes = token_age.num_minutes();

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_AGE",
            &format!(
                "⏰ Age check for {}: {}h {}m old (created: {}), min: {}h, max: {}h",
                token.symbol,
                age_hours,
                age_minutes % 60,
                created_at.format("%Y-%m-%d %H:%M:%S UTC"),
                MIN_TOKEN_AGE_HOURS,
                MAX_TOKEN_AGE_HOURS
            )
        );
    }

    if age_hours < MIN_TOKEN_AGE_HOURS {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_AGE",
                &format!(
                    "❌ Token {} too young: {}h < {}h minimum",
                    token.symbol,
                    age_hours,
                    MIN_TOKEN_AGE_HOURS
                )
            );
        }
        return Some(FilterReason::TooYoung {
            age_hours,
            min_required: MIN_TOKEN_AGE_HOURS,
        });
    }

    if age_hours > MAX_TOKEN_AGE_HOURS {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_AGE",
                &format!(
                    "❌ Token {} too old: {}h > {}h maximum",
                    token.symbol,
                    age_hours,
                    MAX_TOKEN_AGE_HOURS
                )
            );
        }
        return Some(FilterReason::TooOld {
            age_hours,
            max_allowed: MAX_TOKEN_AGE_HOURS,
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
                "💧 Liquidity check for {}: ${:.2} (min: ${:.2}, preferred: ${:.2})",
                token.symbol,
                liquidity_usd,
                MIN_LIQUIDITY_USD,
                PREFERRED_MIN_LIQUIDITY_USD
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

    if is_debug_filtering_enabled() {
        let quality = if liquidity_usd >= PREFERRED_MIN_LIQUIDITY_USD {
            "excellent"
        } else {
            "adequate"
        };
        log(
            LogTag::Filtering,
            "DEBUG_LIQUIDITY",
            &format!("✅ Token {} liquidity {} (${:.2})", token.symbol, quality, liquidity_usd)
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
fn validate_position_constraints(token: &Token) -> Option<FilterReason> {
    let Ok(positions) = SAVED_POSITIONS.lock() else {
        log(
            LogTag::Filtering,
            "ERROR",
            &format!("Could not acquire lock on positions for {}", token.symbol)
        );
        return Some(FilterReason::LockAcquisitionFailed);
    };

    // Count all open positions for context
    let open_positions_count = positions
        .iter()
        .filter(|p| p.position_type == "buy" && p.exit_price.is_none())
        .count();

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_POSITION",
            &format!(
                "🔒 Position check for {}: open positions {}/{}, checking existing/cooldown",
                token.symbol,
                open_positions_count,
                MAX_OPEN_POSITIONS
            )
        );
    }

    // Check for existing open position
    let has_open_position = positions
        .iter()
        .any(|p| p.mint == token.mint && p.position_type == "buy" && p.exit_price.is_none());

    if has_open_position {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_POSITION",
                &format!("❌ Token {} already has an open position", token.symbol)
            );
        }
        return Some(FilterReason::ExistingOpenPosition);
    }

    // Check maximum open positions limit
    if open_positions_count >= MAX_OPEN_POSITIONS {
        if is_debug_filtering_enabled() {
            log(
                LogTag::Filtering,
                "DEBUG_POSITION",
                &format!(
                    "❌ Maximum positions reached: {}/{} open positions",
                    open_positions_count,
                    MAX_OPEN_POSITIONS
                )
            );
        }
        return Some(FilterReason::MaxPositionsReached {
            current: open_positions_count,
            max: MAX_OPEN_POSITIONS,
        });
    }

    // Check for recently closed positions (cooldown period)
    let cooldown_duration = ChronoDuration::minutes(POSITION_CLOSE_COOLDOWN_MINUTES);
    let now = Utc::now();

    for position in positions.iter() {
        if position.mint == token.mint && position.exit_time.is_some() {
            if let Some(exit_time) = position.exit_time {
                let time_since_close = now - exit_time;
                if time_since_close <= cooldown_duration {
                    let minutes_ago = time_since_close.num_minutes();
                    let hours_remaining = (POSITION_CLOSE_COOLDOWN_MINUTES - minutes_ago) / 60;
                    let minutes_remaining = (POSITION_CLOSE_COOLDOWN_MINUTES - minutes_ago) % 60;

                    if is_debug_filtering_enabled() {
                        log(
                            LogTag::Filtering,
                            "DEBUG_POSITION",
                            &format!(
                                "❌ Token {} in cooldown: closed {}m ago, {}h {}m remaining",
                                token.symbol,
                                minutes_ago,
                                hours_remaining,
                                minutes_remaining
                            )
                        );
                    }

                    return Some(FilterReason::RecentlyClosed {
                        minutes_ago,
                        cooldown_minutes: POSITION_CLOSE_COOLDOWN_MINUTES,
                    });
                }
            }
        }
    }

    if is_debug_filtering_enabled() {
        log(
            LogTag::Filtering,
            "DEBUG_POSITION",
            &format!("✅ Token {} position constraints satisfied", token.symbol)
        );
    }

    None
}

// =============================================================================
// UTILITY FUNCTIONS
// =============================================================================

/// Check if a specific token passes all filters (convenience function)
pub fn is_token_eligible_for_trading(token: &Token) -> bool {
    matches!(filter_token_for_trading(token), FilterResult::Approved)
}

/// Filter a list of tokens and return only eligible ones
pub fn filter_eligible_tokens(tokens: &[Token]) -> Vec<Token> {
    tokens
        .iter()
        .filter(|token| is_token_eligible_for_trading(token))
        .cloned()
        .collect()
}

/// Filter a list of tokens and return both eligible and rejected with reasons
pub fn filter_tokens_with_reasons(tokens: &[Token]) -> (Vec<Token>, Vec<(Token, FilterReason)>) {
    let mut eligible = Vec::new();
    let mut rejected = Vec::new();

    for token in tokens {
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
                "Current filters: Age {}h-{}h, Liquidity ${}+, Price {:.12}-{:.3} SOL",
                MIN_TOKEN_AGE_HOURS,
                MAX_TOKEN_AGE_HOURS,
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
            FilterReason::LockAcquisitionFailed => "System Errors",
            FilterReason::DecimalsNotAvailable { .. } => "Decimal Issues",
        };

        *reason_counts.entry(reason_type.to_string()).or_insert(0) += 1;
    }

    log(LogTag::Filtering, "DEBUG", &format!("Rejection breakdown: {:?}", reason_counts));
}

/// Log specific filtering error for important cases
pub fn log_filtering_error(token: &Token, reason: &FilterReason) {
    let should_log = match reason {
        FilterReason::LockAcquisitionFailed => true,
        FilterReason::MaxPositionsReached { .. } => true,
        _ => false,
    };

    if should_log {
        let message = match reason {
            FilterReason::LockAcquisitionFailed =>
                format!("🔒 Lock acquisition failed for {}", token.symbol),
            FilterReason::MaxPositionsReached { current, max } =>
                format!("📊 Max positions reached ({}/{})", current, max),
            _ => {
                return;
            }
        };

        log(LogTag::Filtering, "ERROR", &message);
    }
}

/// Main public interface function that combines filtering with logging
/// This should be used by the trader instead of filter_token_for_trading directly
pub fn should_buy_token(token: &Token) -> bool {
    match filter_token_for_trading(token) {
        FilterResult::Approved => {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "FINAL_APPROVE",
                    &format!("✅ {} FINAL RESULT: APPROVED for trading", token.symbol)
                );
            }
            true
        }
        FilterResult::Rejected(reason) => {
            // Always log rejections for better debugging
            log(
                LogTag::Filtering,
                "FINAL_REJECT",
                &format!("❌ {} FINAL RESULT: REJECTED - {:?}", token.symbol, reason)
            );
            log_filtering_error(token, &reason);
            false
        }
    }
}
