/// Clean and efficient token filtering system
///
/// This module provides a single, focused function to get filtered tokens ready for pool monitoring.
/// All filtering logic is consolidated here for clarity and efficiency.

use crate::global::is_debug_filtering_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::cache::TokenDatabase;
use crate::tokens::decimals::get_cached_decimals;
use crate::tokens::security::{ get_security_analyzer, SecurityRiskLevel, TokenSecurityInfo };
use crate::tokens::types::{ ApiToken, Token };
use chrono::{ Duration as ChronoDuration, Utc };
use std::collections::HashMap;

// =============================================================================
// FILTERING CONFIGURATION
// =============================================================================

/// Target number of tokens to return from filtering
const TARGET_FILTERED_TOKENS: usize = 1000;

/// Maximum tokens to process in one filtering cycle for performance
const MAX_TOKENS_TO_PROCESS: usize = 5000;

// ===== BASIC TOKEN INFO REQUIREMENTS =====
/// Token must have name and symbol (always required)
const REQUIRE_NAME_AND_SYMBOL: bool = true;
/// Token must have logo URL
const REQUIRE_LOGO_URL: bool = false;
/// Token must have website URL
const REQUIRE_WEBSITE_URL: bool = false;

// ===== TRANSACTION ACTIVITY REQUIREMENTS =====
/// Minimum transactions in 5 minutes (only minimum, no maximum)
const MIN_TRANSACTIONS_5MIN: i64 = 5;
/// Minimum transactions in 1 hour (only minimum, no maximum)
const MIN_TRANSACTIONS_1H: i64 = 10;

// ===== LIQUIDITY REQUIREMENTS =====
/// Minimum liquidity in USD (only minimum, no maximum)
const MIN_LIQUIDITY_USD: f64 = 0.0;

// ===== MARKET CAP REQUIREMENTS =====
/// Minimum market cap in USD
const MIN_MARKET_CAP_USD: f64 = 0.0;
/// Maximum market cap in USD
const MAX_MARKET_CAP_USD: f64 = 100_000_000.0;

// ===== SECURITY REQUIREMENTS =====
/// Require LP to be locked (disabled for broader token discovery)
const REQUIRE_LP_LOCKED: bool = true;
/// Block tokens with mint authority
const BLOCK_MINT_AUTHORITY: bool = true;
/// Block tokens with freeze authority
const BLOCK_FREEZE_AUTHORITY: bool = true;
/// Minimum security score (0-100)
const MIN_SECURITY_SCORE: u8 = 0;

// =============================================================================
// MAIN FILTERING FUNCTION
// =============================================================================

/// Get filtered tokens ready for pool service monitoring
///
/// This is the ONLY function used by the pool service to get tokens.
/// Returns up to 1000 token mint addresses that pass all filtering criteria.
///
/// Filtering order (for efficiency):
/// 1. Get tokens from database (ordered by liquidity descending)
/// 2. Check decimals availability in database
/// 3. Check basic token info completeness (name, symbol, logo)
/// 4. Check minimum transaction activity
/// 5. Check minimum liquidity
/// 6. Check market cap range
/// 7. Check security requirements (LP locks, no mint/freeze authority)
///
/// Returns: Vec<String> - List of token mint addresses ready for monitoring
pub async fn get_filtered_tokens() -> Result<Vec<String>, String> {
    let start_time = std::time::Instant::now();
    let debug_enabled = is_debug_filtering_enabled();

    if debug_enabled {
        log(LogTag::Filtering, "START", "Starting token filtering cycle");
    }

    // Step 1: Get tokens from database ordered by liquidity
    let db = TokenDatabase::new().map_err(|e| format!("Failed to create database: {}", e))?;

    let all_tokens = db
        .get_all_tokens().await
        .map_err(|e| format!("Failed to get tokens from database: {}", e))?;

    if all_tokens.is_empty() {
        log(LogTag::Filtering, "WARN", "No tokens found in database");
        return Ok(Vec::new());
    }

    // Limit processing for performance
    let tokens_to_process = if all_tokens.len() > MAX_TOKENS_TO_PROCESS {
        &all_tokens[..MAX_TOKENS_TO_PROCESS]
    } else {
        &all_tokens
    };

    if debug_enabled {
        log(
            LogTag::Filtering,
            "INFO",
            &format!(
                "Processing {} tokens from database (total: {})",
                tokens_to_process.len(),
                all_tokens.len()
            )
        );
    }

    // Step 2: Apply all filtering criteria
    let mut filtered_tokens = Vec::new();
    let mut filtering_stats = FilteringStats::new();

    for token in tokens_to_process {
        filtering_stats.total_processed += 1;

        // Convert ApiToken to Token for filtering
        let token_obj = Token::from(token.clone());

        // Apply filtering criteria in order of efficiency
        if let Some(reason) = apply_all_filters(&token_obj, &mut filtering_stats).await {
            filtering_stats.record_rejection(reason);
            continue;
        }

        // Token passed all filters
        filtered_tokens.push(token.mint.clone());
        filtering_stats.passed_basic_filters += 1;

        // Stop when we have enough tokens
        if filtered_tokens.len() >= TARGET_FILTERED_TOKENS {
            break;
        }
    }

    // Step 3: Security filtering (cache-only for performance)
    let (security_filtered, security_stats) =
        apply_cached_security_filtering_with_stats(filtered_tokens)?;

    // Update main stats with security filtering results
    filtering_stats.passed_security_filters = security_stats.passed;
    filtering_stats.final_passed = security_filtered.len();

    let elapsed = start_time.elapsed();

    // Log results
    log(
        LogTag::Filtering,
        "COMPLETE",
        &format!(
            "Filtering complete: {} tokens passed from {} processed in {:.2}ms",
            security_filtered.len(),
            filtering_stats.total_processed,
            elapsed.as_millis()
        )
    );

    if debug_enabled {
        log_filtering_stats(&filtering_stats, &security_stats, all_tokens.len());
    }

    Ok(security_filtered)
}

// =============================================================================
// FILTERING LOGIC
// =============================================================================

/// Apply all filtering criteria to a token
/// Returns Some(reason) if token should be rejected, None if it passes
/// Also records which filtering stages were passed for statistics
async fn apply_all_filters(
    token: &Token,
    stats: &mut FilteringStats
) -> Option<FilterRejectionReason> {
    // 1. Check decimals availability in database
    if !has_decimals_in_database(&token.mint) {
        return Some(FilterRejectionReason::NoDecimalsInDatabase);
    }
    stats.record_stage_pass("decimals");

    // 2. Check cooldown period for recently closed positions
    if check_cooldown_filter(&token.mint).await {
        return Some(FilterRejectionReason::CooldownFiltered);
    }

    // 3. Check basic token info completeness
    if let Some(reason) = check_basic_token_info(token) {
        return Some(reason);
    }
    stats.record_stage_pass("basic_info");

    // 4. Check minimum transaction activity
    if let Some(reason) = check_transaction_activity(token) {
        return Some(reason);
    }
    stats.record_stage_pass("transactions");

    // 5. Check minimum liquidity
    if let Some(reason) = check_liquidity_requirements(token) {
        return Some(reason);
    }
    stats.record_stage_pass("liquidity");

    // 6. Check market cap range
    if let Some(reason) = check_market_cap_requirements(token) {
        return Some(reason);
    }
    stats.record_stage_pass("market_cap");

    None // Token passed all filters
}

/// Check if token is in cooldown period after recent position closure
async fn check_cooldown_filter(mint: &str) -> bool {
    crate::positions::is_token_in_cooldown(mint).await
}

/// Check if token has decimals in database
fn has_decimals_in_database(mint: &str) -> bool {
    // SOL always has decimals
    if mint == "So11111111111111111111111111111111111111112" {
        return true;
    }

    // Check cached decimals
    get_cached_decimals(mint).is_some()
}

/// Check basic token information completeness
fn check_basic_token_info(token: &Token) -> Option<FilterRejectionReason> {
    // Always check name and symbol if required
    if REQUIRE_NAME_AND_SYMBOL {
        // Check name
        if token.name.trim().is_empty() {
            return Some(FilterRejectionReason::EmptyName);
        }

        // Check symbol
        if token.symbol.trim().is_empty() {
            return Some(FilterRejectionReason::EmptySymbol);
        }
    }

    // Check logo URL if required
    if REQUIRE_LOGO_URL {
        if token.logo_url.as_ref().map_or(true, |url| url.trim().is_empty()) {
            return Some(FilterRejectionReason::EmptyLogoUrl);
        }
    }

    // Check website URL if required
    if REQUIRE_WEBSITE_URL {
        if token.website.as_ref().map_or(true, |url| url.trim().is_empty()) {
            return Some(FilterRejectionReason::EmptyWebsiteUrl);
        }
    }

    None
}

/// Check transaction activity requirements
fn check_transaction_activity(token: &Token) -> Option<FilterRejectionReason> {
    let txns = token.txns.as_ref()?;

    // Check 5-minute transactions
    if let Some(m5) = &txns.m5 {
        let total_5min = m5.buys.unwrap_or(0) + m5.sells.unwrap_or(0);
        if total_5min < MIN_TRANSACTIONS_5MIN {
            return Some(FilterRejectionReason::InsufficientTransactions5Min);
        }
    } else {
        return Some(FilterRejectionReason::NoTransactionData);
    }

    // Check 1-hour transactions
    if let Some(h1) = &txns.h1 {
        let total_1h = h1.buys.unwrap_or(0) + h1.sells.unwrap_or(0);
        if total_1h < MIN_TRANSACTIONS_1H {
            return Some(FilterRejectionReason::InsufficientTransactions1H);
        }
    } else {
        return Some(FilterRejectionReason::NoTransactionData);
    }

    None
}

/// Check liquidity requirements
fn check_liquidity_requirements(token: &Token) -> Option<FilterRejectionReason> {
    let liquidity = token.liquidity.as_ref()?;

    let liquidity_usd = liquidity.usd?;

    if liquidity_usd <= 0.0 {
        return Some(FilterRejectionReason::ZeroLiquidity);
    }

    if liquidity_usd < MIN_LIQUIDITY_USD {
        return Some(FilterRejectionReason::InsufficientLiquidity);
    }

    None
}

/// Check market cap requirements
fn check_market_cap_requirements(token: &Token) -> Option<FilterRejectionReason> {
    let market_cap = token.market_cap?;

    if market_cap < MIN_MARKET_CAP_USD {
        return Some(FilterRejectionReason::MarketCapTooLow);
    }

    if market_cap > MAX_MARKET_CAP_USD {
        return Some(FilterRejectionReason::MarketCapTooHigh);
    }

    None
}

/// Apply security filtering using cached data only (no live blockchain analysis)
/// Returns both filtered tokens and detailed statistics
fn apply_cached_security_filtering_with_stats(
    token_mints: Vec<String>
) -> Result<(Vec<String>, SecurityFilteringStats), String> {
    if token_mints.is_empty() {
        return Ok((Vec::new(), SecurityFilteringStats::new()));
    }

    let debug_enabled = is_debug_filtering_enabled();

    if debug_enabled {
        log(
            LogTag::Filtering,
            "SECURITY_CHECK",
            &format!("Checking cached security data for {} tokens", token_mints.len())
        );
    }

    let mut passed_tokens = Vec::new();
    let mut security_stats = SecurityFilteringStats::new();

    for mint in token_mints {
        security_stats.total_checked += 1;

        // Check cached security data only - no live analysis
        if let Some(security_info) = get_cached_security_info(&mint) {
            // Check security requirements
            if security_info.security_score < MIN_SECURITY_SCORE {
                security_stats.rejected_low_score += 1;
                continue;
            }

            if
                security_info.risk_level == SecurityRiskLevel::Critical ||
                security_info.risk_level == SecurityRiskLevel::High
            {
                security_stats.rejected_high_risk += 1;
                continue;
            }

            // Check LP lock requirement
            if REQUIRE_LP_LOCKED && !security_info.security_flags.lp_locked {
                security_stats.rejected_lp_not_locked += 1;
                continue;
            }

            // Check mint authority requirement
            if BLOCK_MINT_AUTHORITY && security_info.security_flags.can_mint {
                security_stats.rejected_mint_authority += 1;
                continue;
            }

            // Check freeze authority requirement
            if BLOCK_FREEZE_AUTHORITY && security_info.security_flags.can_freeze {
                security_stats.rejected_freeze_authority += 1;
                continue;
            }

            // Token passed all security checks
            passed_tokens.push(mint);
            security_stats.passed += 1;
        } else {
            // No cached security data - skip token for safety
            security_stats.rejected_no_cache += 1;
        }
    }

    Ok((passed_tokens, security_stats))
}

/// Apply security filtering using cached data only (no live blockchain analysis)
fn apply_cached_security_filtering(token_mints: Vec<String>) -> Result<Vec<String>, String> {
    let (tokens, _) = apply_cached_security_filtering_with_stats(token_mints)?;
    Ok(tokens)
}

/// Get cached security info without triggering live analysis
fn get_cached_security_info(mint: &str) -> Option<TokenSecurityInfo> {
    // First check in-memory cache for recently accessed data
    let analyzer = get_security_analyzer();
    if let Some(cached_info) = analyzer.cache.get(mint) {
        return Some(cached_info);
    }

    // If not in cache, check database (this is still "cached" vs live analysis)
    // The database contains previously analyzed data, so it's cached in that sense
    if let Ok(Some(db_info)) = analyzer.database.get_security_info(mint) {
        // Check if the data is not too old (avoid stale data)
        let now = chrono::Utc::now();
        let age = now.signed_duration_since(db_info.timestamps.last_updated);

        // Only use database data if it's less than 1 day old
        if age < chrono::Duration::days(1) {
            // Cache it for future requests
            analyzer.cache.set(db_info.clone());
            return Some(db_info);
        }
    }

    None
}

// =============================================================================
// FILTERING STATISTICS
// =============================================================================

/// Filter rejection reasons for statistics
#[derive(Debug, Clone)]
enum FilterRejectionReason {
    NoDecimalsInDatabase,
    EmptyName,
    EmptySymbol,
    EmptyLogoUrl,
    EmptyWebsiteUrl,
    NoTransactionData,
    InsufficientTransactions5Min,
    InsufficientTransactions1H,
    ZeroLiquidity,
    InsufficientLiquidity,
    MarketCapTooLow,
    MarketCapTooHigh,
    CooldownFiltered,
}

impl FilterRejectionReason {
    fn as_str(&self) -> &'static str {
        match self {
            Self::NoDecimalsInDatabase => "no_decimals",
            Self::EmptyName => "empty_name",
            Self::EmptySymbol => "empty_symbol",
            Self::EmptyLogoUrl => "empty_logo",
            Self::EmptyWebsiteUrl => "empty_website",
            Self::NoTransactionData => "no_txn_data",
            Self::InsufficientTransactions5Min => "low_txn_5m",
            Self::InsufficientTransactions1H => "low_txn_1h",
            Self::ZeroLiquidity => "zero_liquidity",
            Self::InsufficientLiquidity => "low_liquidity",
            Self::MarketCapTooLow => "mcap_too_low",
            Self::MarketCapTooHigh => "mcap_too_high",
            Self::CooldownFiltered => "cooldown_filtered",
        }
    }
}

/// Filtering statistics tracker
struct FilteringStats {
    total_processed: usize,
    passed_basic_filters: usize,
    passed_security_filters: usize,
    final_passed: usize,
    rejection_counts: HashMap<String, usize>,
    // Detailed breakdown
    decimals_check_passed: usize,
    basic_info_check_passed: usize,
    transaction_check_passed: usize,
    liquidity_check_passed: usize,
    market_cap_check_passed: usize,
}

impl FilteringStats {
    fn new() -> Self {
        Self {
            total_processed: 0,
            passed_basic_filters: 0,
            passed_security_filters: 0,
            final_passed: 0,
            rejection_counts: HashMap::new(),
            decimals_check_passed: 0,
            basic_info_check_passed: 0,
            transaction_check_passed: 0,
            liquidity_check_passed: 0,
            market_cap_check_passed: 0,
        }
    }

    fn record_rejection(&mut self, reason: FilterRejectionReason) {
        let key = reason.as_str().to_string();
        *self.rejection_counts.entry(key).or_insert(0) += 1;
    }

    fn record_stage_pass(&mut self, stage: &str) {
        match stage {
            "decimals" => {
                self.decimals_check_passed += 1;
            }
            "basic_info" => {
                self.basic_info_check_passed += 1;
            }
            "transactions" => {
                self.transaction_check_passed += 1;
            }
            "liquidity" => {
                self.liquidity_check_passed += 1;
            }
            "market_cap" => {
                self.market_cap_check_passed += 1;
            }
            _ => {}
        }
    }
}

/// Log filtering statistics for debugging
fn log_filtering_stats(
    filtering_stats: &FilteringStats,
    security_stats: &SecurityFilteringStats,
    total_in_db: usize
) {
    use colored::*;

    // Build comprehensive summary in a single message
    let mut summary = String::new();

    // Header with bright cyan color
    summary.push_str(&format!("{}\n", "🔍 FILTERING PIPELINE RESULTS".bright_cyan().bold()));

    // Database overview with total tokens context
    summary.push_str(
        &format!(
            "{} {} tokens in database, {} processed {}\n",
            "💾 Database:".bright_white().bold(),
            format!("{}", total_in_db).bright_cyan().bold(),
            format!("{}", filtering_stats.total_processed).bright_yellow().bold(),
            if total_in_db > MAX_TOKENS_TO_PROCESS {
                format!("(limited to {})", MAX_TOKENS_TO_PROCESS).dimmed().to_string()
            } else {
                "(all processed)".dimmed().to_string()
            }
        )
    );

    // Pipeline flow with colorized numbers and percentages
    let basic_pass_rate = if filtering_stats.total_processed > 0 {
        ((filtering_stats.passed_basic_filters as f64) / (filtering_stats.total_processed as f64)) *
            100.0
    } else {
        0.0
    };

    let security_pass_rate = if security_stats.total_checked > 0 {
        ((security_stats.passed as f64) / (security_stats.total_checked as f64)) * 100.0
    } else {
        0.0
    };

    summary.push_str(
        &format!(
            "{} {} {} → {} {} ({:.1}%) → {} {} ({:.1}%) → {} {}\n",
            "📊 Pipeline Flow:".bright_white().bold(),
            format!("{}", filtering_stats.total_processed).bright_yellow().bold(),
            "processed".dimmed(),
            format!("{}", filtering_stats.passed_basic_filters).bright_green().bold(),
            "basic passed".dimmed(),
            format!("{:.1}", basic_pass_rate).bright_green().bold(),
            format!("{}", security_stats.passed).bright_blue().bold(),
            "security passed".dimmed(),
            format!("{:.1}", security_pass_rate).bright_blue().bold(),
            format!("{}", filtering_stats.final_passed).bright_magenta().bold(),
            "final tokens".dimmed()
        )
    );

    // Detailed stage breakdown showing losses at each step
    summary.push_str(
        &format!(
            "{} Decimals: {} → {} (lost {}), Info: {} → {} (lost {}), Transactions: {} → {} (lost {}), Liquidity: {} → {} (lost {}), MarketCap: {} → {} (lost {})\n",
            "📈 Stage Details:".bright_white().bold(),
            format!("{}", filtering_stats.total_processed).bright_yellow().bold(),
            format!("{}", filtering_stats.decimals_check_passed).bright_cyan().bold(),
            format!("{}", filtering_stats.total_processed - filtering_stats.decimals_check_passed)
                .bright_red()
                .bold(),
            format!("{}", filtering_stats.decimals_check_passed).bright_cyan().bold(),
            format!("{}", filtering_stats.basic_info_check_passed).bright_green().bold(),
            format!(
                "{}",
                filtering_stats.decimals_check_passed - filtering_stats.basic_info_check_passed
            )
                .bright_red()
                .bold(),
            format!("{}", filtering_stats.basic_info_check_passed).bright_green().bold(),
            format!("{}", filtering_stats.transaction_check_passed).bright_blue().bold(),
            format!(
                "{}",
                filtering_stats.basic_info_check_passed - filtering_stats.transaction_check_passed
            )
                .bright_red()
                .bold(),
            format!("{}", filtering_stats.transaction_check_passed).bright_blue().bold(),
            format!("{}", filtering_stats.liquidity_check_passed).bright_yellow().bold(),
            format!(
                "{}",
                filtering_stats.transaction_check_passed - filtering_stats.liquidity_check_passed
            )
                .bright_red()
                .bold(),
            format!("{}", filtering_stats.liquidity_check_passed).bright_yellow().bold(),
            format!("{}", filtering_stats.market_cap_check_passed).bright_magenta().bold(),
            format!(
                "{}",
                filtering_stats.liquidity_check_passed - filtering_stats.market_cap_check_passed
            )
                .bright_red()
                .bold()
        )
    );

    // Basic rejections summary with top causes highlighted
    let total_basic_rejections =
        filtering_stats.total_processed - filtering_stats.passed_basic_filters;
    summary.push_str(
        &format!(
            "{} {} total ({:.1}% of processed)\n",
            "❌ Basic Rejections:".bright_white().bold(),
            format!("{}", total_basic_rejections).bright_red().bold(),
            if filtering_stats.total_processed > 0 {
                ((total_basic_rejections as f64) / (filtering_stats.total_processed as f64)) * 100.0
            } else {
                0.0
            }
        )
    );

    // Top rejection reasons with colorized counts and percentages
    let mut rejection_vec: Vec<_> = filtering_stats.rejection_counts.iter().collect();
    rejection_vec.sort_by(|a, b| b.1.cmp(a.1));

    if !rejection_vec.is_empty() {
        summary.push_str(&format!("{} ", "📋 Rejection Breakdown:".bright_white().bold()));
        let rejection_details: Vec<String> = rejection_vec
            .iter()
            .take(5) // Show top 5 rejection reasons
            .map(|(reason, count)| {
                let percentage = if filtering_stats.total_processed > 0 {
                    ((**count as f64) / (filtering_stats.total_processed as f64)) * 100.0
                } else {
                    0.0
                };
                format!(
                    "{}={} ({:.1}%)",
                    reason.bright_white(),
                    format!("{}", count).bright_red().bold(),
                    format!("{:.1}", percentage).bright_red().bold()
                )
            })
            .collect();
        summary.push_str(&rejection_details.join(", "));
        summary.push('\n');
    }

    // Security summary with colorized numbers and pass rate
    let total_security_rejections = security_stats.total_checked - security_stats.passed;
    summary.push_str(
        &format!(
            "{} {} checked, {} passed ({:.1}%), {} rejected\n",
            "🔒 Security Filtering:".bright_white().bold(),
            format!("{}", security_stats.total_checked).bright_cyan().bold(),
            format!("{}", security_stats.passed).bright_green().bold(),
            if security_stats.total_checked > 0 {
                ((security_stats.passed as f64) / (security_stats.total_checked as f64)) * 100.0
            } else {
                0.0
            },
            format!("{}", total_security_rejections).bright_red().bold()
        )
    );

    // Security rejection breakdown with colorized counts
    summary.push_str(
        &format!(
            "{} LowScore={}, HighRisk={}, LPNotLocked={}, MintAuth={}, FreezeAuth={}, NoCache={}",
            "🚫 Security Rejects:".bright_white().bold(),
            format!("{}", security_stats.rejected_low_score).bright_red().bold(),
            format!("{}", security_stats.rejected_high_risk).bright_red().bold(),
            format!("{}", security_stats.rejected_lp_not_locked).bright_red().bold(),
            format!("{}", security_stats.rejected_mint_authority).bright_red().bold(),
            format!("{}", security_stats.rejected_freeze_authority).bright_red().bold(),
            format!("{}", security_stats.rejected_no_cache).bright_red().bold()
        )
    );

    // Log the entire summary in one call
    log(LogTag::Filtering, "SUMMARY", &summary);
}

/// Security filtering statistics tracker
struct SecurityFilteringStats {
    total_checked: usize,
    passed: usize,
    rejected_low_score: usize,
    rejected_high_risk: usize,
    rejected_lp_not_locked: usize,
    rejected_mint_authority: usize,
    rejected_freeze_authority: usize,
    rejected_no_cache: usize,
}

impl SecurityFilteringStats {
    fn new() -> Self {
        Self {
            total_checked: 0,
            passed: 0,
            rejected_low_score: 0,
            rejected_high_risk: 0,
            rejected_lp_not_locked: 0,
            rejected_mint_authority: 0,
            rejected_freeze_authority: 0,
            rejected_no_cache: 0,
        }
    }
}

/// Log security filtering statistics for debugging
fn log_security_filtering_stats(stats: &SecurityFilteringStats) {
    log(
        LogTag::Filtering,
        "SECURITY_STATS",
        &format!(
            "Security checked: {}, Passed: {}, Rejected: {}",
            stats.total_checked,
            stats.passed,
            stats.total_checked - stats.passed
        )
    );

    log(
        LogTag::Filtering,
        "SECURITY_REJECTS",
        &format!(
            "Low score: {}, High risk: {}, LP not locked: {}, Mint auth: {}, Freeze auth: {}, No cache: {}",
            stats.rejected_low_score,
            stats.rejected_high_risk,
            stats.rejected_lp_not_locked,
            stats.rejected_mint_authority,
            stats.rejected_freeze_authority,
            stats.rejected_no_cache
        )
    );
}
