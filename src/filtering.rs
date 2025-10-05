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
    RiskLevel,
    SecurityAnalysis,
    SecurityAnalyzer,
};
use crate::tokens::types::{ ApiToken, Token };
use chrono::{ Duration as ChronoDuration, Utc };
use std::collections::HashMap;
use std::sync::{ Arc, OnceLock };
use std::time::{ Duration as StdDuration, Instant as StdInstant };
use tokio::sync::RwLock;

// =============================================================================
// CACHING CONFIGURATION (to avoid blocking discovery every 5s)
// =============================================================================
/// TTL for cached filtered tokens; discovery reads from cache and we refresh in background
const FILTER_CACHE_TTL_SECS: u64 = 15;

struct FilterCache {
    tokens: Vec<String>,
    updated_at: StdInstant,
}

static FILTER_CACHE: OnceLock<Arc<RwLock<FilterCache>>> = OnceLock::new();

fn get_filter_cache() -> &'static Arc<RwLock<FilterCache>> {
    FILTER_CACHE.get_or_init(|| {
        Arc::new(
            RwLock::new(FilterCache {
                tokens: Vec::new(),
                // Initialize as very old so first call performs a synchronous compute
                updated_at: StdInstant::now() - StdDuration::from_secs(3600),
            })
        )
    })
}

// =============================================================================
// FILTERING CONFIGURATION - MINIMAL SAFETY FOR $1-$100M TRADING RANGE
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

// ===== TOKEN AGE REQUIREMENTS =====
/// Minimum token age in minutes before allowing monitoring
const MIN_TOKEN_AGE_MINUTES: i64 = 1 * 60;

// ===== TRANSACTION ACTIVITY REQUIREMENTS =====
/// Minimum transactions in 5 minutes (very low for small tokens)
const MIN_TRANSACTIONS_5MIN: i64 = 1;
/// Minimum transactions in 1 hour (low threshold for new tokens)
const MIN_TRANSACTIONS_1H: i64 = 5;

// ===== LIQUIDITY REQUIREMENTS =====
/// Minimum liquidity in USD ($1 = 0.001 SOL equivalent)
const MIN_LIQUIDITY_USD: f64 = 1.0;
/// Maximum liquidity in USD ($100M cap for trading focus)
const MAX_LIQUIDITY_USD: f64 = 100_000_000.0;

// ===== MARKET CAP REQUIREMENTS =====
/// Minimum market cap in USD ($1000 minimum)
const MIN_MARKET_CAP_USD: f64 = 1000.0;
/// Maximum market cap in USD ($100M maximum)
const MAX_MARKET_CAP_USD: f64 = 100_000_000.0;

// ===== MINIMAL SECURITY REQUIREMENTS (VERY PERMISSIVE) =====
/// Minimum security score (0-100) - very low threshold for inclusion
const MIN_SECURITY_SCORE: i32 = 10;
/// Maximum acceptable top holder percentage (very permissive)
const MAX_TOP_HOLDER_PCT: f64 = 15.0;
/// Maximum acceptable top 3 holders percentage (very permissive)
const MAX_TOP_3_HOLDERS_PCT: f64 = 5.0;
/// Minimum LP lock percentage for pump.fun tokens (relaxed)
const MIN_PUMPFUN_LP_LOCK_PCT: f64 = 50.0;
/// Minimum LP lock percentage for regular tokens (very relaxed)
const MIN_REGULAR_LP_LOCK_PCT: f64 = 50.0;
/// Minimum unique holders required (based on Rugcheck cached DB)
const MIN_UNIQUE_HOLDERS: u32 = 500;

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
/// 4. Enforce minimum token age to avoid fresh launches
/// 5. Security checks (authorities safe, risk != Danger, minimum holders)
/// 6. Check basic token info completeness (name, symbol, logo)
/// 7. Check minimum transaction activity
/// 8. Check minimum liquidity
/// 9. Check market cap range
///
/// Returns: Vec<String> - List of token mint addresses ready for monitoring
pub async fn get_filtered_tokens() -> Result<Vec<String>, String> {
    let debug_enabled = is_debug_filtering_enabled();

    // Fast path: serve from cache if fresh
    {
        let cache = get_filter_cache();
        let guard = cache.read().await;
        let age = StdInstant::now().saturating_duration_since(guard.updated_at);
        if age < StdDuration::from_secs(FILTER_CACHE_TTL_SECS) {
            if debug_enabled {
                log(
                    LogTag::Filtering,
                    "CACHE_HIT",
                    &format!(
                        "Returning cached filtered tokens (age={}ms, count={})",
                        age.as_millis(),
                        guard.tokens.len()
                    )
                );
            }
            return Ok(guard.tokens.clone());
        }
    }

    // If cache exists (non-empty), trigger background refresh and return stale data immediately
    {
        let cache = get_filter_cache();
        let maybe_tokens = {
            let guard = cache.read().await;
            (guard.tokens.clone(), guard.updated_at)
        };

        if !maybe_tokens.0.is_empty() {
            if debug_enabled {
                log(
                    LogTag::Filtering,
                    "CACHE_EXPIRED",
                    &format!(
                        "Cache expired (age={}ms). Spawning background refresh and returning stale list ({} tokens).",
                        StdInstant::now().saturating_duration_since(maybe_tokens.1).as_millis(),
                        maybe_tokens.0.len()
                    )
                );
            }

            // Spawn background refresh; ignore join handle
            tokio::spawn(async move {
                if let Ok(tokens) = compute_filtered_tokens().await {
                    let cache = get_filter_cache();
                    let mut guard = cache.write().await;
                    guard.tokens = tokens;
                    guard.updated_at = StdInstant::now();
                }
            });

            return Ok(maybe_tokens.0);
        }
    }

    // Cold start (no cache): compute synchronously, update cache, and return
    let tokens = compute_filtered_tokens().await?;
    {
        let cache = get_filter_cache();
        let mut guard = cache.write().await;
        guard.tokens = tokens.clone();
        guard.updated_at = StdInstant::now();
    }
    Ok(tokens)
}

// Heavy compute moved here so we can refresh in background without recursion
async fn compute_filtered_tokens() -> Result<Vec<String>, String> {
    let start_time = StdInstant::now();
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

    // Ensure security analyzer is initialized
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

    // Apply ALL filters directly to all tokens (no separate security filtering step)
    let mut filtered_tokens = Vec::new();
    let mut filtering_stats = FilteringStats::new();
    filtering_stats.total_processed = all_tokens.len();

    if debug_enabled {
        log(
            LogTag::Filtering,
            "INFO",
            &format!(
                "Processing {} tokens with integrated filtering pipeline (security + basic filters)",
                all_tokens.len()
            )
        );
    }

    for token_api in all_tokens.iter().take(MAX_TOKENS_TO_PROCESS) {
        // Convert ApiToken to Token for filtering
        let token_obj = Token::from(token_api.clone());

        // Apply ALL filtering criteria (including security)
        if let Some(reason) = apply_all_filters(&token_obj, &mut filtering_stats).await {
            filtering_stats.record_rejection(reason);
            continue;
        }

        // Token passed all filters
        filtered_tokens.push(token_api.mint.clone());
        filtering_stats.passed_basic_filters += 1;

        // Stop when we have enough tokens
        if filtered_tokens.len() >= TARGET_FILTERED_TOKENS {
            break;
        }
    }

    // Update final stats
    filtering_stats.final_passed = filtered_tokens.len();

    let elapsed = start_time.elapsed();

    // Log results
    log(
        LogTag::Filtering,
        "COMPLETE",
        &format!(
            "Integrated filtering complete: {} tokens passed from {} processed in {:.2}ms",
            filtered_tokens.len(),
            filtering_stats.total_processed,
            elapsed.as_millis()
        )
    );

    if debug_enabled {
        log_filtering_stats(&filtering_stats, all_tokens.len());
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

    // 2. Enforce minimum age requirement
    if let Some(reason) = check_minimum_age(token) {
        return Some(reason);
    }
    stats.record_stage_pass("age");

    // 3. Check security requirements (new integrated approach)
    if let Some(reason) = check_security_requirements(&token.mint).await {
        return Some(reason);
    }
    stats.record_stage_pass("security");

    // 4. Check cooldown period for recently closed positions
    if check_cooldown_filter(&token.mint).await {
        return Some(FilterRejectionReason::CooldownFiltered);
    }

    // 5. Check basic token info completeness
    if let Some(reason) = check_basic_token_info(token) {
        return Some(reason);
    }
    stats.record_stage_pass("basic_info");

    // 6. Check minimum transaction activity
    if let Some(reason) = check_transaction_activity(token) {
        return Some(reason);
    }
    stats.record_stage_pass("transactions");

    // 7. Check minimum liquidity
    if let Some(reason) = check_liquidity_requirements(token) {
        return Some(reason);
    }
    stats.record_stage_pass("liquidity");

    // 8. Check market cap range
    if let Some(reason) = check_market_cap_requirements(token) {
        return Some(reason);
    }
    stats.record_stage_pass("market_cap");

    None // Token passed all filters
}

/// Ensure token has existed long enough to be eligible
fn check_minimum_age(token: &Token) -> Option<FilterRejectionReason> {
    let created_at = match token.created_at {
        Some(value) => value,
        None => {
            return Some(FilterRejectionReason::MissingCreationTimestamp);
        }
    };

    let age_minutes = Utc::now().signed_duration_since(created_at).num_minutes();
    let normalized_age = age_minutes.max(0);

    if normalized_age < MIN_TOKEN_AGE_MINUTES {
        return Some(FilterRejectionReason::TokenTooNew);
    }

    None
}

/// Check if token is in cooldown period after recent position closure
async fn check_cooldown_filter(mint: &str) -> bool {
    crate::positions::is_token_in_cooldown(mint).await
}

/// Check security requirements - STRICT authority checking
async fn check_security_requirements(mint: &str) -> Option<FilterRejectionReason> {
    use crate::global::is_debug_filtering_enabled;

    // Get security analyzer
    let analyzer = match get_security_analyzer() {
        Some(analyzer) => analyzer,
        None => {
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "SECURITY_REJECT",
                    &format!("No security analyzer available for mint={}", mint)
                );
            }
            return Some(FilterRejectionReason::SecurityNoData);
        }
    };

    // Get cached security data (avoid API calls for performance)
    match analyzer.analyze_token_any_cached(mint).await {
        Some(analysis) => {
            // CRITICAL: Always reject tokens with unsafe authorities (mint/freeze)
            if !analysis.authorities_safe {
                if is_debug_filtering_enabled() {
                    log(
                        LogTag::Filtering,
                        "AUTHORITY_REJECT",
                        &format!("Unsafe authorities detected for mint={}", mint)
                    );
                }
                return Some(FilterRejectionReason::SecurityHighRisk);
            }

            // Apply risk level filtering
            match analysis.risk_level {
                crate::tokens::security::RiskLevel::Danger => {
                    if is_debug_filtering_enabled() {
                        log(
                            LogTag::Filtering,
                            "RISK_REJECT",
                            &format!("High risk level detected for mint={}", mint)
                        );
                    }
                    Some(FilterRejectionReason::SecurityHighRisk)
                }
                _ => {
                    // Enforce minimum unique holders using cached Rugcheck data (no API calls)
                    match analyzer.get_cached_holder_count(mint).await {
                        Some(count) => {
                            if count < MIN_UNIQUE_HOLDERS {
                                if is_debug_filtering_enabled() {
                                    log(
                                        LogTag::Filtering,
                                        "HOLDERS_REJECT",
                                        &format!(
                                            "Insufficient holders for mint={} ({} < required {})",
                                            mint,
                                            count,
                                            MIN_UNIQUE_HOLDERS
                                        )
                                    );
                                }
                                return Some(FilterRejectionReason::InsufficientHolders);
                            }
                        }
                        None => {
                            if is_debug_filtering_enabled() {
                                log(
                                    LogTag::Filtering,
                                    "HOLDERS_NO_DATA",
                                    &format!("No holder count data available for mint={}", mint)
                                );
                            }
                            return Some(FilterRejectionReason::NoHolderData);
                        }
                    }
                    if is_debug_filtering_enabled() {
                        log(
                            LogTag::Filtering,
                            "SECURITY_PASS",
                            &format!(
                                "Security check passed for mint={} risk={:?}",
                                mint,
                                analysis.risk_level
                            )
                        );
                    }
                    None // Allow Safe, Warning, Unknown if authorities are safe
                }
            }
        }
        None => {
            // No security data available - reject for safety
            if is_debug_filtering_enabled() {
                log(
                    LogTag::Filtering,
                    "NO_DATA_REJECT",
                    &format!("No security data available for mint={}", mint)
                );
            }
            Some(FilterRejectionReason::SecurityNoData)
        }
    }
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

    if liquidity_usd > MAX_LIQUIDITY_USD {
        return Some(FilterRejectionReason::LiquidityTooHigh);
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

// =============================================================================
// FILTERING STATISTICS
// =============================================================================

/// Filter rejection reasons for statistics
#[derive(Debug, Clone)]
enum FilterRejectionReason {
    NoDecimalsInDatabase,
    SecurityHighRisk,
    SecurityNoData,
    NoHolderData,
    InsufficientHolders,
    EmptyName,
    EmptySymbol,
    EmptyLogoUrl,
    EmptyWebsiteUrl,
    NoTransactionData,
    InsufficientTransactions5Min,
    InsufficientTransactions1H,
    ZeroLiquidity,
    InsufficientLiquidity,
    LiquidityTooHigh,
    MarketCapTooLow,
    MarketCapTooHigh,
    CooldownFiltered,
    TokenTooNew,
    MissingCreationTimestamp,
}

impl FilterRejectionReason {
    fn as_str(&self) -> &'static str {
        match self {
            Self::NoDecimalsInDatabase => "no_decimals",
            Self::SecurityHighRisk => "security_high_risk",
            Self::SecurityNoData => "security_no_data",
            Self::NoHolderData => "no_holder_data",
            Self::InsufficientHolders => "insufficient_holders",
            Self::EmptyName => "empty_name",
            Self::EmptySymbol => "empty_symbol",
            Self::EmptyLogoUrl => "empty_logo",
            Self::EmptyWebsiteUrl => "empty_website",
            Self::NoTransactionData => "no_txn_data",
            Self::InsufficientTransactions5Min => "low_txn_5m",
            Self::InsufficientTransactions1H => "low_txn_1h",
            Self::ZeroLiquidity => "zero_liquidity",
            Self::InsufficientLiquidity => "low_liquidity",
            Self::LiquidityTooHigh => "liquidity_too_high",
            Self::MarketCapTooLow => "mcap_too_low",
            Self::MarketCapTooHigh => "mcap_too_high",
            Self::CooldownFiltered => "cooldown_filtered",
            Self::TokenTooNew => "token_too_new",
            Self::MissingCreationTimestamp => "missing_creation_timestamp",
        }
    }
}

/// Filtering statistics tracker
struct FilteringStats {
    total_processed: usize,
    passed_basic_filters: usize,
    final_passed: usize,
    rejection_counts: HashMap<String, usize>,
    // Detailed breakdown
    decimals_check_passed: usize,
    security_check_passed: usize,
    age_check_passed: usize,
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
            final_passed: 0,
            rejection_counts: HashMap::new(),
            decimals_check_passed: 0,
            security_check_passed: 0,
            age_check_passed: 0,
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
            "age" => {
                self.age_check_passed += 1;
            }
            "security" => {
                self.security_check_passed += 1;
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
fn log_filtering_stats(filtering_stats: &FilteringStats, total_in_db: usize) {
    use colored::*;

    // Build comprehensive summary in a single message
    let mut summary = String::new();

    // Header with bright cyan color
    summary.push_str(&format!("{}\n", "🔍 INTEGRATED FILTERING RESULTS".bright_cyan().bold()));

    // Database overview
    summary.push_str(
        &format!(
            "{} {} tokens in DB; processed: {}\n",
            "💾 Database:".bright_white().bold(),
            format!("{}", total_in_db).bright_cyan().bold(),
            format!("{}", filtering_stats.total_processed).bright_yellow().bold()
        )
    );

    // Overall pipeline results
    let overall_pass_rate = if filtering_stats.total_processed > 0 {
        ((filtering_stats.final_passed as f64) / (filtering_stats.total_processed as f64)) * 100.0
    } else {
        0.0
    };
    summary.push_str(
        &format!(
            "{} processed={}, final={} ({}%)\n",
            "� Pipeline:".bright_white().bold(),
            format!("{}", filtering_stats.total_processed).bright_yellow().bold(),
            format!("{}", filtering_stats.final_passed).bright_magenta().bold(),
            format!("{:.1}", overall_pass_rate).bright_magenta().bold()
        )
    );

    // Detailed stage breakdown
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
            "  • Age: {} → {} (lost {})\n",
            format!("{}", filtering_stats.decimals_check_passed).bright_cyan().bold(),
            format!("{}", filtering_stats.age_check_passed).bright_blue().bold(),
            format!(
                "{}",
                filtering_stats.decimals_check_passed.saturating_sub(
                    filtering_stats.age_check_passed
                )
            )
                .bright_red()
                .bold()
        )
    );
    summary.push_str(
        &format!(
            "  • Security: {} → {} (lost {})\n",
            format!("{}", filtering_stats.age_check_passed).bright_blue().bold(),
            format!("{}", filtering_stats.security_check_passed).bright_blue().bold(),
            format!(
                "{}",
                filtering_stats.age_check_passed.saturating_sub(
                    filtering_stats.security_check_passed
                )
            )
                .bright_red()
                .bold()
        )
    );
    summary.push_str(
        &format!(
            "  • Basic Info: {} → {} (lost {})\n",
            format!("{}", filtering_stats.security_check_passed).bright_blue().bold(),
            format!("{}", filtering_stats.basic_info_check_passed).bright_green().bold(),
            format!(
                "{}",
                filtering_stats.security_check_passed.saturating_sub(
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
            format!("{}", filtering_stats.transaction_check_passed).bright_yellow().bold(),
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
            format!("{}", filtering_stats.transaction_check_passed).bright_yellow().bold(),
            format!("{}", filtering_stats.liquidity_check_passed).bright_cyan().bold(),
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
            "  • Market Cap: {} → {} (lost {})\n",
            format!("{}", filtering_stats.liquidity_check_passed).bright_cyan().bold(),
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

    // Rejection breakdown
    let total_rejections = filtering_stats.total_processed.saturating_sub(
        filtering_stats.final_passed
    );
    summary.push_str(
        &format!(
            "{} {} total ({:.1}% of processed)\n",
            "❌ Rejections:".bright_white().bold(),
            format!("{}", total_rejections).bright_red().bold(),
            if filtering_stats.total_processed > 0 {
                ((total_rejections as f64) / (filtering_stats.total_processed as f64)) * 100.0
            } else {
                0.0
            }
        )
    );

    // Top rejection reasons
    let mut rejection_vec: Vec<_> = filtering_stats.rejection_counts.iter().collect();
    rejection_vec.sort_by(|a, b| b.1.cmp(a.1));

    if !rejection_vec.is_empty() {
        summary.push_str(&format!("{} ", "📋 Top Reasons:".bright_white().bold()));
        let rejection_details: Vec<String> = rejection_vec
            .iter()
            .take(5)
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

    // Log the entire summary in one call
    log(LogTag::Filtering, "SUMMARY", &summary);
}
