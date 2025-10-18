use std::collections::HashMap;
use std::time::Instant as StdInstant;

use chrono::Utc;

use crate::config::FilteringConfig;
use crate::global::is_debug_filtering_enabled;
use crate::logger::{log, LogTag};
use crate::positions;
use crate::tokens::get_cached_decimals;
use crate::tokens::types::Token;
use crate::tokens::store::all_tokens;

use super::types::{
    FilteringSnapshot, PassedToken, RejectedToken, TokenEntry, MAX_DECISION_HISTORY,
};

pub async fn compute_snapshot(config: FilteringConfig) -> Result<FilteringSnapshot, String> {
    let debug_enabled = is_debug_filtering_enabled();
    let start = StdInstant::now();

    // Use token store cache (unified Token) instead of legacy snapshot
    let tokens = all_tokens();

    if tokens.is_empty() {
        if debug_enabled {
            log(
                LogTag::Filtering,
                "EMPTY",
                "Token store empty - snapshot will be empty (cache may still be warming up)",
            );
        }
        return Ok(FilteringSnapshot::empty());
    }

    // All tokens for filtering
    let all_tokens: Vec<Token> = tokens;

    let mut filtered_mints: Vec<String> = Vec::new();
    let mut passed_tokens: Vec<PassedToken> = Vec::new();
    let mut rejected_mints: Vec<String> = Vec::new();
    let mut rejected_tokens: Vec<RejectedToken> = Vec::new();
    let mut stats = FilteringStats::new();

    let max_process = config.max_tokens_to_process.min(all_tokens.len());
    let decision_time = || Utc::now().timestamp();

    for token_api in all_tokens.iter().take(max_process) {
        stats.total_processed += 1;
        let token_obj: Token = token_api.clone().into();

        match apply_all_filters(&token_obj, &config).await {
            Ok(()) => {
                filtered_mints.push(token_api.mint.clone());
                stats.passed += 1;

                if passed_tokens.len() >= MAX_DECISION_HISTORY {
                    passed_tokens.remove(0);
                }
                passed_tokens.push(PassedToken {
                    mint: token_api.mint.clone(),
                    symbol: token_obj.symbol.clone(),
                    name: Some(token_obj.name.clone()),
                    passed_time: decision_time(),
                });

                if config.target_filtered_tokens > 0
                    && filtered_mints.len() >= config.target_filtered_tokens
                {
                    break;
                }
            }
            Err(reason) => {
                stats.record_rejection(&reason);

                rejected_mints.push(token_api.mint.clone());
                if rejected_tokens.len() >= MAX_DECISION_HISTORY {
                    rejected_tokens.remove(0);
                }
                rejected_tokens.push(RejectedToken {
                    mint: token_api.mint.clone(),
                    symbol: token_obj.symbol.clone(),
                    name: Some(token_obj.name.clone()),
                    reason: reason.label().to_string(),
                    rejection_time: decision_time(),
                });
            }
        }
    }

    // TODO: Replace placeholders with bulk helpers from pools/positions/ohlcvs
    // Build derived flags maps for one-pass assignment
    let priced_set: std::collections::HashSet<String> = {
        // Preferred: pools::get_available_tokens() returns mints with price
        crate::pools::get_available_tokens().into_iter().collect()
    };
    let open_pos_set: std::collections::HashSet<String> = {
        // Preferred: positions::get_open_mints()
        crate::positions::get_open_mints().await.into_iter().collect()
    };
    let ohlcv_set: std::collections::HashSet<String> = {
        // Preferred: ohlcvs::get_mints_with_data(&mints)
        let mints: Vec<String> = all_tokens.iter().map(|t| t.mint.clone()).collect();
        match crate::ohlcvs::get_mints_with_data(&mints).await {
            Ok(set) => set,
            Err(_) => Default::default(),
        }
    };

    let mut token_entries: HashMap<String, TokenEntry> = HashMap::with_capacity(all_tokens.len());
    for token in &all_tokens {
        let has_pool_price = priced_set.contains(&token.mint);
        let has_open_position = open_pos_set.contains(&token.mint);
        let has_ohlcv = ohlcv_set.contains(&token.mint);
        token_entries.insert(
            token.mint.clone(),
            TokenEntry {
                token: token.clone(),
                has_pool_price,
                has_open_position,
                has_ohlcv,
                pair_created_at: Some(token.first_seen_at.timestamp()),
                last_updated: token.updated_at,
            },
        );
    }

    let elapsed_ms = start.elapsed().as_millis();
    if debug_enabled {
        log(
            LogTag::Filtering,
            "REFRESH_COMPLETE",
            &format!(
                "filtered={} rejected={} processed={} duration_ms={}",
                filtered_mints.len(),
                rejected_tokens.len(),
                stats.total_processed,
                elapsed_ms
            ),
        );
    }

    Ok(FilteringSnapshot {
        updated_at: Utc::now(),
        filtered_mints,
        passed_tokens,
        rejected_mints,
        rejected_tokens,
        tokens: token_entries,
    })
}

async fn apply_all_filters(
    token: &Token,
    config: &FilteringConfig,
) -> Result<(), FilterRejectionReason> {
    if !has_decimals_in_database(&token.mint) {
        return Err(FilterRejectionReason::NoDecimalsInDatabase);
    }

    if let Some(reason) = check_minimum_age(token, config) {
        return Err(reason);
    }

    if config.check_cooldown && check_cooldown_filter(&token.mint).await {
        return Err(FilterRejectionReason::CooldownFiltered);
    }

    // RugCheck security filters (if enabled)
    if config.rugcheck.enabled {
        if let Some(reason) = check_security_requirements(&token.mint, config).await {
            return Err(reason);
        }
    }

    // DexScreener market data filters (if enabled)
    if config.dexscreener.enabled {
        if let Some(reason) = check_basic_token_info(token, config) {
            return Err(reason);
        }

        if let Some(reason) = check_transaction_activity(token, config) {
            return Err(reason);
        }

        if let Some(reason) = check_liquidity_requirements(token, config) {
            return Err(reason);
        }

        if let Some(reason) = check_market_cap_requirements(token, config) {
            return Err(reason);
        }
    }

    Ok(())
}

fn check_minimum_age(token: &Token, config: &FilteringConfig) -> Option<FilterRejectionReason> {
    let created_at = token.first_seen_at;
    let age_minutes = Utc::now()
        .signed_duration_since(created_at)
        .num_minutes()
        .max(0);

    if age_minutes < config.min_token_age_minutes {
        Some(FilterRejectionReason::TokenTooNew)
    } else {
        None
    }
}

async fn check_security_requirements(
    _mint: &str,
    config: &FilteringConfig,
) -> Option<FilterRejectionReason> {
    // Placeholder: Security checks will be implemented when tokens.security provider is wired.
    // For now, honor the enable flag but do not reject tokens here.
    if !config.rugcheck.enabled {
        None
    } else {
        None
    }
}

// Analyzer-based checks removed; rely on Token fields populated by tokens provider.

async fn check_cooldown_filter(mint: &str) -> bool {
    positions::is_token_in_cooldown(mint).await
}

fn check_basic_token_info(
    token: &Token,
    config: &FilteringConfig,
) -> Option<FilterRejectionReason> {
    // Skip if token info checks are disabled
    if !config.dexscreener.token_info_enabled {
        return None;
    }

    if config.dexscreener.require_name_and_symbol {
        if token.name.trim().is_empty() {
            return Some(FilterRejectionReason::DexScreener_EmptyName);
        }

        if token.symbol.trim().is_empty() {
            return Some(FilterRejectionReason::DexScreener_EmptySymbol);
        }
    }

    if config.dexscreener.require_logo_url {
        // Use image_url (from DexScreener info) when available
        if token
            .image_url
            .as_ref()
            .map_or(true, |url| url.trim().is_empty())
        {
            return Some(FilterRejectionReason::DexScreener_EmptyLogoUrl);
        }
    }

    if config.dexscreener.require_website_url {
        // Consider websites list (metadata) as presence of a website
        let has_website = token
            .websites
            .iter()
            .any(|w| !w.url.trim().is_empty());
        if !has_website {
            return Some(FilterRejectionReason::DexScreener_EmptyWebsiteUrl);
        }
    }

    None
}

fn check_transaction_activity(
    token: &Token,
    config: &FilteringConfig,
) -> Option<FilterRejectionReason> {
    // Skip if transaction checks are disabled
    if !config.dexscreener.transactions_enabled {
        return None;
    }

    // Check flat transaction counters on Token
    let m5_total = token
        .txns_m5_buys
        .unwrap_or(0)
        .saturating_add(token.txns_m5_sells.unwrap_or(0));
    if m5_total < config.dexscreener.min_transactions_5min {
        return Some(FilterRejectionReason::DexScreener_InsufficientTransactions5Min);
    }

    let h1_total = token
        .txns_h1_buys
        .unwrap_or(0)
        .saturating_add(token.txns_h1_sells.unwrap_or(0));
    if h1_total < config.dexscreener.min_transactions_1h {
        return Some(FilterRejectionReason::DexScreener_InsufficientTransactions1H);
    }

    None
}

fn check_liquidity_requirements(
    token: &Token,
    config: &FilteringConfig,
) -> Option<FilterRejectionReason> {
    // Skip if liquidity checks are disabled
    if !config.dexscreener.liquidity_enabled {
        return None;
    }

    let liquidity_usd = token.liquidity_usd?;

    if liquidity_usd <= 0.0 {
        return Some(FilterRejectionReason::DexScreener_ZeroLiquidity);
    }

    if liquidity_usd < config.dexscreener.min_liquidity_usd {
        return Some(FilterRejectionReason::DexScreener_InsufficientLiquidity);
    }

    if liquidity_usd > config.dexscreener.max_liquidity_usd {
        return Some(FilterRejectionReason::DexScreener_LiquidityTooHigh);
    }

    None
}

fn check_market_cap_requirements(
    token: &Token,
    config: &FilteringConfig,
) -> Option<FilterRejectionReason> {
    // Skip if market cap checks are disabled
    if !config.dexscreener.market_cap_enabled {
        return None;
    }

    let market_cap = token.market_cap?;

    if market_cap < config.dexscreener.min_market_cap_usd {
        return Some(FilterRejectionReason::DexScreener_MarketCapTooLow);
    }

    if market_cap > config.dexscreener.max_market_cap_usd {
        return Some(FilterRejectionReason::DexScreener_MarketCapTooHigh);
    }

    None
}

fn has_decimals_in_database(mint: &str) -> bool {
    if mint == "So11111111111111111111111111111111111111112" {
        return true;
    }

    get_cached_decimals(mint).is_some()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FilterRejectionReason {
    // Meta requirements (apply before source-specific checks)
    NoDecimalsInDatabase,
    TokenTooNew,
    MissingCreationTimestamp,
    CooldownFiltered,

    // DexScreener rejections
    DexScreener_EmptyName,
    DexScreener_EmptySymbol,
    DexScreener_EmptyLogoUrl,
    DexScreener_EmptyWebsiteUrl,
    DexScreener_NoTransactionData,
    DexScreener_InsufficientTransactions5Min,
    DexScreener_InsufficientTransactions1H,
    DexScreener_ZeroLiquidity,
    DexScreener_InsufficientLiquidity,
    DexScreener_LiquidityTooHigh,
    DexScreener_MarketCapTooLow,
    DexScreener_MarketCapTooHigh,

    // RugCheck rejections
    RugCheck_SecurityHighRisk,
    RugCheck_RiskScoreTooHigh,
    RugCheck_SecurityNoData,
    RugCheck_NoHolderData,
    RugCheck_InsufficientHolders,
    RugCheck_TopHolderConcentration,
    RugCheck_TopThreeHolderConcentration,
    RugCheck_LpLockTooLow,
    RugCheck_TokenRugged,
    RugCheck_TooManyInsiders,
    RugCheck_TooManyInsiderHolders,
    RugCheck_InsiderConcentration,
    RugCheck_CreatorBalanceTooHigh,
    RugCheck_InsufficientLpProviders,
    RugCheck_HasTransferFee,
    RugCheck_TransferFeeTooHigh,
}

impl FilterRejectionReason {
    fn label(&self) -> &'static str {
        match self {
            // Meta
            FilterRejectionReason::NoDecimalsInDatabase => "meta_no_decimals",
            FilterRejectionReason::TokenTooNew => "meta_token_new",
            FilterRejectionReason::MissingCreationTimestamp => "meta_missing_creation_ts",
            FilterRejectionReason::CooldownFiltered => "meta_cooldown",

            // DexScreener
            FilterRejectionReason::DexScreener_EmptyName => "dexscreener_missing_name",
            FilterRejectionReason::DexScreener_EmptySymbol => "dexscreener_missing_symbol",
            FilterRejectionReason::DexScreener_EmptyLogoUrl => "dexscreener_missing_logo",
            FilterRejectionReason::DexScreener_EmptyWebsiteUrl => "dexscreener_missing_website",
            FilterRejectionReason::DexScreener_NoTransactionData => "dexscreener_tx_missing",
            FilterRejectionReason::DexScreener_InsufficientTransactions5Min => {
                "dexscreener_tx_5m_low"
            }
            FilterRejectionReason::DexScreener_InsufficientTransactions1H => {
                "dexscreener_tx_1h_low"
            }
            FilterRejectionReason::DexScreener_ZeroLiquidity => "dexscreener_liquidity_zero",
            FilterRejectionReason::DexScreener_InsufficientLiquidity => "dexscreener_liquidity_low",
            FilterRejectionReason::DexScreener_LiquidityTooHigh => "dexscreener_liquidity_high",
            FilterRejectionReason::DexScreener_MarketCapTooLow => "dexscreener_market_cap_low",
            FilterRejectionReason::DexScreener_MarketCapTooHigh => "dexscreener_market_cap_high",

            // RugCheck
            FilterRejectionReason::RugCheck_SecurityHighRisk => "rugcheck_security_high_risk",
            FilterRejectionReason::RugCheck_RiskScoreTooHigh => "rugcheck_risk_score_high",
            FilterRejectionReason::RugCheck_SecurityNoData => "rugcheck_security_no_data",
            FilterRejectionReason::RugCheck_NoHolderData => "rugcheck_holders_no_data",
            FilterRejectionReason::RugCheck_InsufficientHolders => "rugcheck_holders_insufficient",
            FilterRejectionReason::RugCheck_TopHolderConcentration => "rugcheck_top_holder_pct",
            FilterRejectionReason::RugCheck_TopThreeHolderConcentration => {
                "rugcheck_top3_holder_pct"
            }
            FilterRejectionReason::RugCheck_LpLockTooLow => "rugcheck_lp_lock_low",
            FilterRejectionReason::RugCheck_TokenRugged => "rugcheck_token_rugged",
            FilterRejectionReason::RugCheck_TooManyInsiders => "rugcheck_too_many_insiders",
            FilterRejectionReason::RugCheck_TooManyInsiderHolders => {
                "rugcheck_too_many_insider_holders"
            }
            FilterRejectionReason::RugCheck_InsiderConcentration => {
                "rugcheck_insider_concentration"
            }
            FilterRejectionReason::RugCheck_CreatorBalanceTooHigh => {
                "rugcheck_creator_balance_high"
            }
            FilterRejectionReason::RugCheck_InsufficientLpProviders => {
                "rugcheck_lp_providers_insufficient"
            }
            FilterRejectionReason::RugCheck_HasTransferFee => "rugcheck_has_transfer_fee",
            FilterRejectionReason::RugCheck_TransferFeeTooHigh => "rugcheck_transfer_fee_high",
        }
    }

    /// Returns the source that caused the rejection
    fn source(&self) -> FilterSource {
        match self {
            FilterRejectionReason::NoDecimalsInDatabase
            | FilterRejectionReason::TokenTooNew
            | FilterRejectionReason::MissingCreationTimestamp
            | FilterRejectionReason::CooldownFiltered => FilterSource::Meta,

            FilterRejectionReason::DexScreener_EmptyName
            | FilterRejectionReason::DexScreener_EmptySymbol
            | FilterRejectionReason::DexScreener_EmptyLogoUrl
            | FilterRejectionReason::DexScreener_EmptyWebsiteUrl
            | FilterRejectionReason::DexScreener_NoTransactionData
            | FilterRejectionReason::DexScreener_InsufficientTransactions5Min
            | FilterRejectionReason::DexScreener_InsufficientTransactions1H
            | FilterRejectionReason::DexScreener_ZeroLiquidity
            | FilterRejectionReason::DexScreener_InsufficientLiquidity
            | FilterRejectionReason::DexScreener_LiquidityTooHigh
            | FilterRejectionReason::DexScreener_MarketCapTooLow
            | FilterRejectionReason::DexScreener_MarketCapTooHigh => FilterSource::DexScreener,

            FilterRejectionReason::RugCheck_SecurityHighRisk
            | FilterRejectionReason::RugCheck_RiskScoreTooHigh
            | FilterRejectionReason::RugCheck_SecurityNoData
            | FilterRejectionReason::RugCheck_NoHolderData
            | FilterRejectionReason::RugCheck_InsufficientHolders
            | FilterRejectionReason::RugCheck_TopHolderConcentration
            | FilterRejectionReason::RugCheck_TopThreeHolderConcentration
            | FilterRejectionReason::RugCheck_LpLockTooLow
            | FilterRejectionReason::RugCheck_TokenRugged
            | FilterRejectionReason::RugCheck_TooManyInsiders
            | FilterRejectionReason::RugCheck_TooManyInsiderHolders
            | FilterRejectionReason::RugCheck_InsiderConcentration
            | FilterRejectionReason::RugCheck_CreatorBalanceTooHigh
            | FilterRejectionReason::RugCheck_InsufficientLpProviders
            | FilterRejectionReason::RugCheck_HasTransferFee
            | FilterRejectionReason::RugCheck_TransferFeeTooHigh => FilterSource::RugCheck,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FilterSource {
    Meta,
    DexScreener,
    RugCheck,
}

struct FilteringStats {
    total_processed: usize,
    passed: usize,
    rejected: HashMap<FilterRejectionReason, usize>,
}

impl FilteringStats {
    fn new() -> Self {
        Self {
            total_processed: 0,
            passed: 0,
            rejected: HashMap::new(),
        }
    }

    fn record_rejection(&mut self, reason: &FilterRejectionReason) {
        *self.rejected.entry(*reason).or_insert(0) += 1;
    }
}
