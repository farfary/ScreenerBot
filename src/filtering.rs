/// Clean and efficient token filtering system
///
/// This module provides a single, focused function to get filtered tokens ready for pool monitoring.
/// All filtering logic is consolidated here for clarity and efficiency.

use crate::global::is_debug_filtering_enabled;
use crate::logger::{ log, LogTag };
use crate::tokens::cache::TokenDatabase;
use crate::tokens::decimals::get_cached_decimals;
use crate::tokens::security::{
    get_security_analyzer,
    initialize_security_analyzer,
    SecurityAnalyzer,
    RiskLevel,
};
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

/// Security-first processing multiplier - process 3x more tokens to ensure secure tokens aren't excluded
const SECURITY_FIRST_MULTIPLIER: usize = 3;

// ===== BASIC TOKEN INFO REQUIREMENTS =====
/// Token must have name and symbol (always required)
const REQUIRE_NAME_AND_SYMBOL: bool = true;
/// Token must have logo URL
const REQUIRE_LOGO_URL: bool = false;
/// Token must have website URL
const REQUIRE_WEBSITE_URL: bool = false;

// ===== TRANSACTION ACTIVITY REQUIREMENTS =====
/// Minimum transactions in 5 minutes (only minimum, no maximum)
const MIN_TRANSACTIONS_5MIN: i64 = 1;
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
/// Filtering order (optimized for performance and security):
/// 1. Get tokens from database with security-aware ordering and increased limit
/// 2. Apply security filtering FIRST to prioritize secure tokens
/// 3. Check decimals availability in database
/// 4. Check basic token info completeness (name, symbol, logo)
/// 5. Check minimum transaction activity
/// 6. Check minimum liquidity
/// 7. Check market cap range
///
/// Returns: Vec<String> - List of token mint addresses ready for monitoring
pub async fn get_filtered_tokens() -> Result<Vec<String>, String> {
    let start_time = std::time::Instant::now();
    let debug_enabled = is_debug_filtering_enabled();

    if debug_enabled {
        log(LogTag::Filtering, "START", "Starting token filtering cycle");
    }

    // Step 1: Get tokens from database - INCREASED LIMIT to ensure secure tokens aren't excluded
    let db = TokenDatabase::new().map_err(|e| format!("Failed to create database: {}", e))?;

    let all_tokens = db
        .get_all_tokens().await
        .map_err(|e| format!("Failed to get tokens from database: {}", e))?;

    if all_tokens.is_empty() {
        log(LogTag::Filtering, "WARN", "No tokens found in database");
        return Ok(Vec::new());
    }

    // CRITICAL FIX: Increase processing limit and apply security filtering FIRST
    // This ensures secure tokens aren't excluded by the 5000 limit
    let extended_limit = std::cmp::min(
        all_tokens.len(),
        MAX_TOKENS_TO_PROCESS * SECURITY_FIRST_MULTIPLIER
    );
    let tokens_to_process = &all_tokens[..extended_limit];

    if debug_enabled {
        log(
            LogTag::Filtering,
            "INFO",
            &format!(
                "Processing {} tokens from database (total: {}) - SECURITY PRIORITY MODE",
                tokens_to_process.len(),
                all_tokens.len()
            )
        );
    }

    // Ensure security analyzer is initialized (debug binaries may not have run the main init)
    if get_security_analyzer().is_none() {
        if let Err(e) = initialize_security_analyzer() {
            log(
                LogTag::Filtering,
                "WARN",
                &format!("Security analyzer not initialized and failed to init: {}", e)
            );
        } else if debug_enabled {
            log(LogTag::Filtering, "INFO", "Security analyzer initialized lazily for filtering");
        }
    }

    // Step 2: Apply SECURITY filtering FIRST to prioritize secure tokens
    let token_mints: Vec<String> = tokens_to_process
        .iter()
        .map(|t| t.mint.clone())
        .collect();
    let (security_filtered, security_stats) =
        apply_cached_security_filtering_with_stats(token_mints).await?;

    if debug_enabled {
        log(
            LogTag::Filtering,
            "SECURITY_FIRST",
            &format!(
                "Security filtering FIRST: {} tokens passed from {} checked",
                security_filtered.len(),
                security_stats.total_checked
            )
        );
    }

    // Step 3: Apply remaining filters to security-approved tokens
    let mut filtered_tokens = Vec::new();
    let mut filtering_stats = FilteringStats::new();
    filtering_stats.total_processed = security_filtered.len(); // Track security-filtered count

    // Create lookup map for faster token retrieval
    let token_map: HashMap<String, &ApiToken> = tokens_to_process
        .iter()
        .map(|token| (token.mint.clone(), token))
        .collect();

    for mint in security_filtered {
        if let Some(token_api) = token_map.get(&mint) {
            // Convert ApiToken to Token for filtering
            let token_obj = Token::from((*token_api).clone());

            // Apply remaining filtering criteria (security already passed)
            if let Some(reason) = apply_remaining_filters(&token_obj, &mut filtering_stats).await {
                filtering_stats.record_rejection(reason);
                continue;
            }

            // Token passed all filters
            filtered_tokens.push(mint);
            filtering_stats.passed_basic_filters += 1;

            // Stop when we have enough tokens
            if filtered_tokens.len() >= TARGET_FILTERED_TOKENS {
                break;
            }
        }
    }

    // Update final stats
    filtering_stats.passed_security_filters = security_stats.passed;
    filtering_stats.final_passed = filtered_tokens.len();

    let elapsed = start_time.elapsed();

    // Log results
    log(
        LogTag::Filtering,
        "COMPLETE",
        &format!(
            "SECURITY-FIRST filtering complete: {} tokens passed from {} processed in {:.2}ms",
            filtered_tokens.len(),
            filtering_stats.total_processed,
            elapsed.as_millis()
        )
    );

    if debug_enabled {
        log_filtering_stats(&filtering_stats, &security_stats, all_tokens.len());
    }

    Ok(filtered_tokens)
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

/// Apply remaining filters (non-security) to tokens that already passed security check
/// This is used when security filtering is applied first
async fn apply_remaining_filters(
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

    None // Token passed all remaining filters
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
async fn apply_cached_security_filtering(token_mints: Vec<String>) -> Result<Vec<String>, String> {
    let (tokens, _) = apply_cached_security_filtering_with_stats(token_mints).await?;
    Ok(tokens)
}

/// Returns both filtered tokens and detailed statistics
async fn apply_cached_security_filtering_with_stats(
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

        // Check security using new analyzer (simplified for now)
        if let Some(is_safe) = get_security_info_for_filtering(&mint).await {
            if !is_safe {
                security_stats.rejected_high_risk += 1;
                continue;
            }
        } else {
            // No security info available - be conservative and reject
            security_stats.rejected_high_risk += 1;
            continue;
        }

        // If we get here, the token passed security filtering
        passed_tokens.push(mint);
        security_stats.passed += 1;
    }

    Ok((passed_tokens, security_stats))
}

/// Get security info for filtering (simplified for now)
async fn get_security_info_for_filtering(mint: &str) -> Option<bool> {
    // Use the analyzer cache/database only; avoid API calls in filtering
    if let Some(analyzer) = crate::tokens::security::get_security_analyzer() {
        return analyzer.analyze_token_cached_only(mint).await;
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
    let security_limit = MAX_TOKENS_TO_PROCESS * SECURITY_FIRST_MULTIPLIER;
    summary.push_str(
        &format!(
            "{} {} tokens in DB; security check limit: {}; post-security processed: {}\n",
            "💾 Database:".bright_white().bold(),
            format!("{}", total_in_db).bright_cyan().bold(),
            format!("{}", security_limit).bright_yellow().bold(),
            format!("{}", filtering_stats.total_processed).bright_yellow().bold()
        )
    );

    // Security stage first: show pass rate vs. total checked at security stage
    let security_pass_rate = if security_stats.total_checked > 0 {
        ((security_stats.passed as f64) / (security_stats.total_checked as f64)) * 100.0
    } else {
        0.0
    };
    let security_rejected = security_stats.total_checked.saturating_sub(security_stats.passed);
    summary.push_str(
        &format!(
            "{} checked={}, passed={} ({}%), rejected={}\n",
            "🛡️ Security:".bright_white().bold(),
            format!("{}", security_stats.total_checked).bright_cyan().bold(),
            format!("{}", security_stats.passed).bright_blue().bold(),
            format!("{:.1}", security_pass_rate).bright_blue().bold(),
            format!("{}", security_rejected).bright_red().bold()
        )
    );

    // Post-security pipeline: show pass rate vs. post-security processed
    let post_security_pass_rate = if filtering_stats.total_processed > 0 {
        ((filtering_stats.final_passed as f64) / (filtering_stats.total_processed as f64)) * 100.0
    } else {
        0.0
    };
    summary.push_str(
        &format!(
            "{} processed={}, final={} ({}%)\n",
            "📊 Pipeline (post-security):".bright_white().bold(),
            format!("{}", filtering_stats.total_processed).bright_yellow().bold(),
            format!("{}", filtering_stats.final_passed).bright_magenta().bold(),
            format!("{:.1}", post_security_pass_rate).bright_magenta().bold()
        )
    );

    // Detailed stage breakdown showing losses at each step
    // Stage details on multiple short lines to avoid wrap truncating numbers
    summary.push_str(&format!("{}\n", "📈 Stage Details:".bright_white().bold()));
    summary.push_str(
        &format!(
            "  • Decimals: {} → {} (lost {})\n",
            format!("{}", filtering_stats.total_processed).bright_yellow().bold(),
            format!("{}", filtering_stats.decimals_check_passed).bright_cyan().bold(),
            format!(
                "{}",
                filtering_stats.total_processed.saturating_sub(
                    filtering_stats.decimals_check_passed
                )
            )
                .bright_red()
                .bold()
        )
    );
    summary.push_str(
        &format!(
            "  • Info: {} → {} (lost {})\n",
            format!("{}", filtering_stats.decimals_check_passed).bright_cyan().bold(),
            format!("{}", filtering_stats.basic_info_check_passed).bright_green().bold(),
            format!(
                "{}",
                filtering_stats.decimals_check_passed.saturating_sub(
                    filtering_stats.basic_info_check_passed
                )
            )
                .bright_red()
                .bold()
        )
    );
    summary.push_str(
        &format!(
            "  • Transactions: {} → {} (lost {})\n",
            format!("{}", filtering_stats.basic_info_check_passed).bright_green().bold(),
            format!("{}", filtering_stats.transaction_check_passed).bright_blue().bold(),
            format!(
                "{}",
                filtering_stats.basic_info_check_passed.saturating_sub(
                    filtering_stats.transaction_check_passed
                )
            )
                .bright_red()
                .bold()
        )
    );
    summary.push_str(
        &format!(
            "  • Liquidity: {} → {} (lost {})\n",
            format!("{}", filtering_stats.transaction_check_passed).bright_blue().bold(),
            format!("{}", filtering_stats.liquidity_check_passed).bright_yellow().bold(),
            format!(
                "{}",
                filtering_stats.transaction_check_passed.saturating_sub(
                    filtering_stats.liquidity_check_passed
                )
            )
                .bright_red()
                .bold()
        )
    );
    summary.push_str(
        &format!(
            "  • MarketCap: {} → {} (lost {})\n",
            format!("{}", filtering_stats.liquidity_check_passed).bright_yellow().bold(),
            format!("{}", filtering_stats.market_cap_check_passed).bright_magenta().bold(),
            format!(
                "{}",
                filtering_stats.liquidity_check_passed.saturating_sub(
                    filtering_stats.market_cap_check_passed
                )
            )
                .bright_red()
                .bold()
        )
    );

    // Basic rejections summary with top causes highlighted
    let total_basic_rejections = filtering_stats.total_processed.saturating_sub(
        filtering_stats.final_passed
    );
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
                    "{}={} ({}%)",
                    reason.bright_white(),
                    format!("{}", count).bright_red().bold(),
                    format!("{:.1}", percentage).bright_red().bold()
                )
            })
            .collect();
        summary.push_str(&rejection_details.join(", "));
        summary.push('\n');
    }

    // Security summary with colorized numbers and pass rate (redundant with the first line but kept concise)
    let total_security_rejections = security_rejected;
    summary.push_str(
        &format!(
            "{} checked={}, passed={} ({}%), rejected={}\n",
            "🔒 Security Filtering:".bright_white().bold(),
            format!("{}", security_stats.total_checked).bright_cyan().bold(),
            format!("{}", security_stats.passed).bright_green().bold(),
            format!("{:.1}", security_pass_rate).bright_green().bold(),
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
