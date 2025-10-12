use std::collections::HashMap;
use std::time::Instant as StdInstant;

use chrono::Utc;

use crate::config::FilteringConfig;
use crate::global::is_debug_filtering_enabled;
use crate::logger::{log, LogTag};
use crate::positions;
use crate::tokens::decimals::get_cached_decimals;
use crate::tokens::security::{
    get_security_analyzer, initialize_security_analyzer, RiskLevel, SecurityAnalyzer,
};
use crate::tokens::store::get_global_token_store;
use crate::tokens::summary::{token_to_summary, TokenSummaryContext};
use crate::tokens::types::Token;

use super::types::{
    FilteringSnapshot, PassedToken, RejectedToken, TokenEntry, MAX_DECISION_HISTORY,
};

pub async fn compute_snapshot(config: FilteringConfig) -> Result<FilteringSnapshot, String> {
    let debug_enabled = is_debug_filtering_enabled();
    let start = StdInstant::now();

    // Use token store cache instead of direct database access
    let store = get_global_token_store();
    let snapshots = store.all();

    if snapshots.is_empty() {
        if debug_enabled {
            log(
                LogTag::Filtering,
                "EMPTY",
                "Token store empty - snapshot will be empty (cache may still be warming up)",
            );
        }
        return Ok(FilteringSnapshot::empty());
    }

    // Convert TokenSnapshots to ApiTokens for filtering
    let all_tokens: Vec<_> = snapshots.iter().map(|s| s.data.clone()).collect();

    ensure_security_analyzer_initialized(debug_enabled);

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

    let mints: Vec<String> = all_tokens.iter().map(|token| token.mint.clone()).collect();
    let summary_context = TokenSummaryContext::build(&mints).await;

    let mut token_entries: HashMap<String, TokenEntry> = HashMap::with_capacity(all_tokens.len());
    for token_api in &all_tokens {
        let summary = token_to_summary(token_api, &summary_context);
        token_entries.insert(
            token_api.mint.clone(),
            TokenEntry {
                summary,
                pair_created_at: token_api.pair_created_at,
                last_updated: token_api.last_updated,
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

fn ensure_security_analyzer_initialized(debug_enabled: bool) {
    if get_security_analyzer().is_some() {
        return;
    }

    if let Err(err) = initialize_security_analyzer() {
        log(
            LogTag::Filtering,
            "SECURITY_INIT_FAIL",
            &format!("Failed to initialize security analyzer: {}", err),
        );
    } else if debug_enabled {
        log(
            LogTag::Filtering,
            "SECURITY_INIT",
            "Security analyzer initialized lazily for filtering",
        );
    }
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

    if let Some(reason) = check_security_requirements(&token.mint, config).await {
        return Err(reason);
    }

    if check_cooldown_filter(&token.mint).await {
        return Err(FilterRejectionReason::CooldownFiltered);
    }

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

    Ok(())
}

fn check_minimum_age(token: &Token, config: &FilteringConfig) -> Option<FilterRejectionReason> {
    let created_at = match token.created_at {
        Some(value) => value,
        None => return Some(FilterRejectionReason::MissingCreationTimestamp),
    };
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
    mint: &str,
    config: &FilteringConfig,
) -> Option<FilterRejectionReason> {
    let analyzer = match get_security_analyzer() {
        Some(analyzer) => analyzer,
        None => return Some(FilterRejectionReason::SecurityNoData),
    };

    analyze_with_security_analyzer(&analyzer, mint, config).await
}

async fn analyze_with_security_analyzer(
    analyzer: &SecurityAnalyzer,
    mint: &str,
    config: &FilteringConfig,
) -> Option<FilterRejectionReason> {
    let analysis = match analyzer.analyze_token_any_cached(mint).await {
        Some(result) => result,
        None => return Some(FilterRejectionReason::SecurityNoData),
    };

    if config.min_security_score > 0 && analysis.score_normalized < config.min_security_score {
        return Some(FilterRejectionReason::SecurityScoreTooLow);
    }

    if !analysis.authorities_safe {
        return Some(FilterRejectionReason::SecurityHighRisk);
    }

    if matches!(analysis.risk_level, RiskLevel::Danger) {
        return Some(FilterRejectionReason::SecurityHighRisk);
    }

    if config.max_top_holder_pct > 0.0 {
        if let Some(top_holder_pct) = analysis.top_holder_pct {
            if top_holder_pct > config.max_top_holder_pct {
                return Some(FilterRejectionReason::TopHolderConcentration);
            }
        }
    }

    if config.max_top_3_holders_pct > 0.0 {
        if let Some(top_three_pct) = analysis.top_3_holder_pct {
            if top_three_pct > config.max_top_3_holders_pct {
                return Some(FilterRejectionReason::TopThreeHolderConcentration);
            }
        }
    }

    let required_lp_lock = if analysis.pump_fun_token {
        config.min_pumpfun_lp_lock_pct
    } else {
        config.min_regular_lp_lock_pct
    };

    if required_lp_lock > 0.0 {
        let actual_lp_lock = analysis.max_lp_locked_pct.unwrap_or(0.0);
        if actual_lp_lock < required_lp_lock {
            return Some(FilterRejectionReason::LpLockTooLow);
        }
    }

    match analyzer.get_cached_holder_count(mint).await {
        Some(count) => {
            if count < config.min_unique_holders {
                Some(FilterRejectionReason::InsufficientHolders)
            } else {
                None
            }
        }
        None => Some(FilterRejectionReason::NoHolderData),
    }
}

async fn check_cooldown_filter(mint: &str) -> bool {
    positions::is_token_in_cooldown(mint).await
}

fn check_basic_token_info(
    token: &Token,
    config: &FilteringConfig,
) -> Option<FilterRejectionReason> {
    if config.require_name_and_symbol {
        if token.name.trim().is_empty() {
            return Some(FilterRejectionReason::EmptyName);
        }

        if token.symbol.trim().is_empty() {
            return Some(FilterRejectionReason::EmptySymbol);
        }
    }

    if config.require_logo_url {
        if token
            .logo_url
            .as_ref()
            .map_or(true, |url| url.trim().is_empty())
        {
            return Some(FilterRejectionReason::EmptyLogoUrl);
        }
    }

    if config.require_website_url {
        if token
            .website
            .as_ref()
            .map_or(true, |url| url.trim().is_empty())
        {
            return Some(FilterRejectionReason::EmptyWebsiteUrl);
        }
    }

    None
}

fn check_transaction_activity(
    token: &Token,
    config: &FilteringConfig,
) -> Option<FilterRejectionReason> {
    let txns = token.txns.as_ref()?;

    if let Some(m5) = &txns.m5 {
        let total = m5.buys.unwrap_or(0) + m5.sells.unwrap_or(0);
        if total < config.min_transactions_5min {
            return Some(FilterRejectionReason::InsufficientTransactions5Min);
        }
    } else {
        return Some(FilterRejectionReason::NoTransactionData);
    }

    if let Some(h1) = &txns.h1 {
        let total = h1.buys.unwrap_or(0) + h1.sells.unwrap_or(0);
        if total < config.min_transactions_1h {
            return Some(FilterRejectionReason::InsufficientTransactions1H);
        }
    } else {
        return Some(FilterRejectionReason::NoTransactionData);
    }

    None
}

fn check_liquidity_requirements(
    token: &Token,
    config: &FilteringConfig,
) -> Option<FilterRejectionReason> {
    let liquidity = token.liquidity.as_ref()?;
    let liquidity_usd = liquidity.usd?;

    if liquidity_usd <= 0.0 {
        return Some(FilterRejectionReason::ZeroLiquidity);
    }

    if liquidity_usd < config.min_liquidity_usd {
        return Some(FilterRejectionReason::InsufficientLiquidity);
    }

    if liquidity_usd > config.max_liquidity_usd {
        return Some(FilterRejectionReason::LiquidityTooHigh);
    }

    None
}

fn check_market_cap_requirements(
    token: &Token,
    config: &FilteringConfig,
) -> Option<FilterRejectionReason> {
    let market_cap = token.market_cap?;

    if market_cap < config.min_market_cap_usd {
        return Some(FilterRejectionReason::MarketCapTooLow);
    }

    if market_cap > config.max_market_cap_usd {
        return Some(FilterRejectionReason::MarketCapTooHigh);
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
    NoDecimalsInDatabase,
    SecurityHighRisk,
    SecurityScoreTooLow,
    SecurityNoData,
    NoHolderData,
    InsufficientHolders,
    TopHolderConcentration,
    TopThreeHolderConcentration,
    LpLockTooLow,
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
    TokenTooNew,
    MissingCreationTimestamp,
    CooldownFiltered,
}

impl FilterRejectionReason {
    fn label(&self) -> &'static str {
        match self {
            FilterRejectionReason::NoDecimalsInDatabase => "no_decimals",
            FilterRejectionReason::SecurityHighRisk => "security_high_risk",
            FilterRejectionReason::SecurityScoreTooLow => "security_score_low",
            FilterRejectionReason::SecurityNoData => "security_no_data",
            FilterRejectionReason::NoHolderData => "holders_no_data",
            FilterRejectionReason::InsufficientHolders => "holders_insufficient",
            FilterRejectionReason::TopHolderConcentration => "top_holder_pct",
            FilterRejectionReason::TopThreeHolderConcentration => "top3_holder_pct",
            FilterRejectionReason::LpLockTooLow => "lp_lock_low",
            FilterRejectionReason::EmptyName => "missing_name",
            FilterRejectionReason::EmptySymbol => "missing_symbol",
            FilterRejectionReason::EmptyLogoUrl => "missing_logo",
            FilterRejectionReason::EmptyWebsiteUrl => "missing_website",
            FilterRejectionReason::NoTransactionData => "tx_missing",
            FilterRejectionReason::InsufficientTransactions5Min => "tx_5m_low",
            FilterRejectionReason::InsufficientTransactions1H => "tx_1h_low",
            FilterRejectionReason::ZeroLiquidity => "liquidity_zero",
            FilterRejectionReason::InsufficientLiquidity => "liquidity_low",
            FilterRejectionReason::LiquidityTooHigh => "liquidity_high",
            FilterRejectionReason::MarketCapTooLow => "market_cap_low",
            FilterRejectionReason::MarketCapTooHigh => "market_cap_high",
            FilterRejectionReason::TokenTooNew => "token_new",
            FilterRejectionReason::MissingCreationTimestamp => "missing_creation_ts",
            FilterRejectionReason::CooldownFiltered => "cooldown",
        }
    }
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
