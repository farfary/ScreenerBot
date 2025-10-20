use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::Instant as StdInstant;

use chrono::Utc;
use futures::stream::{self, StreamExt};

use crate::config::schemas::{DexScreenerFilters, RugCheckFilters};
use crate::config::FilteringConfig;
use crate::global::is_debug_filtering_enabled;
use crate::logger::{log, LogTag};
use crate::positions;
use crate::tokens::{get_cached_decimals, get_full_token_async, list_tokens_async};
use crate::tokens::types::Token;

use super::types::{
    FilteringSnapshot, PassedToken, RejectedToken, TokenEntry, MAX_DECISION_HISTORY,
};

const TOKEN_FETCH_CONCURRENCY: usize = 24;

pub async fn compute_snapshot(config: FilteringConfig) -> Result<FilteringSnapshot, String> {
    let debug_enabled = is_debug_filtering_enabled();
    let start = StdInstant::now();

    let max_candidates = config.max_tokens_to_process.max(100);
    let fetch_limit = if config.target_filtered_tokens > 0 {
        max_candidates.max(config.target_filtered_tokens)
    } else {
        max_candidates
    };

    let metadata = list_tokens_async(fetch_limit)
        .await
        .map_err(|e| format!("Failed to list tokens: {}", e))?;

    if metadata.is_empty() {
        if debug_enabled {
            log(
                LogTag::Filtering,
                "SNAPSHOT_EMPTY",
                "Token store empty - snapshot remains empty",
            );
        }
        return Ok(FilteringSnapshot::empty());
    }

    let total_candidates = metadata.len();

    let tokens_with_index: Vec<(usize, Token)> = stream::iter(metadata.into_iter().enumerate().map(
        |(index, meta)| {
            let mint = meta.mint.clone();
            async move {
                match get_full_token_async(&mint).await {
                    Ok(Some(token)) => Some((index, token)),
                    Ok(None) => None,
                    Err(err) => {
                        log(
                            LogTag::Filtering,
                            "TOKEN_LOAD_ERROR",
                            &format!("mint={} error={}", mint, err),
                        );
                        None
                    }
                }
            }
        },
    ))
    .buffer_unordered(TOKEN_FETCH_CONCURRENCY)
    .filter_map(|entry| async move { entry })
    .collect()
    .await;

    if tokens_with_index.is_empty() {
        if debug_enabled {
            log(
                LogTag::Filtering,
                "SNAPSHOT_NO_TOKENS",
                &format!(
                    "Unable to load full tokens for any candidates (total_candidates={})",
                    total_candidates
                ),
            );
        }
        return Ok(FilteringSnapshot::empty());
    }

    let mut tokens_sorted = tokens_with_index;
    tokens_sorted.sort_by_key(|(index, _)| *index);
    let tokens: Vec<Token> = tokens_sorted.into_iter().map(|(_, token)| token).collect();

    let candidate_mints: Vec<String> = tokens.iter().map(|t| t.mint.clone()).collect();

    let priced_set: HashSet<String> = crate::pools::get_available_tokens().into_iter().collect();
    let open_position_set: HashSet<String> = positions::get_open_mints()
        .await
        .into_iter()
        .collect();

    let ohlcv_set: HashSet<String> = match crate::ohlcvs::get_mints_with_data(&candidate_mints).await
    {
        Ok(set) => set,
        Err(err) => {
            log(
                LogTag::Filtering,
                "OHLCV_LOOKUP_FAILED",
                &format!("error={}", err),
            );
            HashSet::new()
        }
    };

    let mut filtered_mints: Vec<String> = Vec::new();
    let mut rejected_mints: Vec<String> = Vec::new();
    let mut passed_tokens: VecDeque<PassedToken> = VecDeque::new();
    let mut rejected_tokens: VecDeque<RejectedToken> = VecDeque::new();
    let mut token_entries: HashMap<String, TokenEntry> = HashMap::with_capacity(tokens.len());
    let mut stats = FilteringStats::default();

    for token in tokens.iter().take(config.max_tokens_to_process) {
        stats.total_processed += 1;

        let has_pool_price = priced_set.contains(&token.mint);
        let has_open_position = open_position_set.contains(&token.mint);
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

        match apply_all_filters(token, &config).await {
            Ok(()) => {
                filtered_mints.push(token.mint.clone());
                stats.passed += 1;

                if passed_tokens.len() >= MAX_DECISION_HISTORY {
                    passed_tokens.pop_front();
                }
                passed_tokens.push_back(PassedToken {
                    mint: token.mint.clone(),
                    symbol: token.symbol.clone(),
                    name: Some(token.name.clone()),
                    passed_time: Utc::now().timestamp(),
                });
            }
            Err(reason) => {
                stats.record_rejection(reason);
                rejected_mints.push(token.mint.clone());
                if rejected_tokens.len() >= MAX_DECISION_HISTORY {
                    rejected_tokens.pop_front();
                }
                rejected_tokens.push_back(RejectedToken {
                    mint: token.mint.clone(),
                    symbol: token.symbol.clone(),
                    name: Some(token.name.clone()),
                    reason: reason.label().to_string(),
                    rejection_time: Utc::now().timestamp(),
                });
            }
        }

        if config.target_filtered_tokens > 0
            && filtered_mints.len() >= config.target_filtered_tokens
        {
            stats.target_reached = true;
            break;
        }
    }

    let elapsed_ms = start.elapsed().as_millis();

    if debug_enabled {
        let rejection_summary = stats.rejection_summary();
        log(
            LogTag::Filtering,
            "REFRESH_COMPLETE",
            &format!(
                "processed={} passed={} rejected={} target_reached={} duration_ms={} rejection_summary={}",
                stats.total_processed,
                stats.passed,
                stats.rejected,
                stats.target_reached,
                elapsed_ms,
                rejection_summary
            ),
        );
    }

    Ok(FilteringSnapshot {
        updated_at: Utc::now(),
        filtered_mints,
        passed_tokens: passed_tokens.into_iter().collect(),
        rejected_mints,
        rejected_tokens: rejected_tokens.into_iter().collect(),
        tokens: token_entries,
    })
}

async fn apply_all_filters(
    token: &Token,
    config: &FilteringConfig,
) -> Result<(), FilterRejectionReason> {
    if config.require_decimals_in_db && !has_decimals_in_database(&token.mint) {
        return Err(FilterRejectionReason::NoDecimalsInDatabase);
    }

    if let Some(reason) = check_minimum_age(token, config) {
        return Err(reason);
    }

    if config.check_cooldown && check_cooldown_filter(&token.mint).await {
        return Err(FilterRejectionReason::CooldownFiltered);
    }

    if let Some(reason) = check_security_requirements(token, &config.rugcheck) {
        return Err(reason);
    }

    if config.dexscreener.enabled {
        if let Some(reason) = check_basic_token_info(token, &config.dexscreener) {
            return Err(reason);
        }

        if let Some(reason) = check_transaction_activity(token, &config.dexscreener) {
            return Err(reason);
        }

        if let Some(reason) = check_liquidity_requirements(token, &config.dexscreener) {
            return Err(reason);
        }

        if let Some(reason) = check_market_cap_requirements(token, &config.dexscreener) {
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

async fn check_cooldown_filter(mint: &str) -> bool {
    positions::is_token_in_cooldown(mint).await
}

fn check_basic_token_info(
    token: &Token,
    config: &DexScreenerFilters,
) -> Option<FilterRejectionReason> {
    if !config.token_info_enabled {
        return None;
    }

    if config.require_name_and_symbol {
        if token.name.trim().is_empty() {
            return Some(FilterRejectionReason::DexScreenerEmptyName);
        }
        if token.symbol.trim().is_empty() {
            return Some(FilterRejectionReason::DexScreenerEmptySymbol);
        }
    }

    if config.require_logo_url {
        if token
            .image_url
            .as_ref()
            .map_or(true, |url| url.trim().is_empty())
        {
            return Some(FilterRejectionReason::DexScreenerEmptyLogoUrl);
        }
    }

    if config.require_website_url {
        let has_website = token
            .websites
            .iter()
            .any(|w| !w.url.trim().is_empty());
        if !has_website {
            return Some(FilterRejectionReason::DexScreenerEmptyWebsiteUrl);
        }
    }

    None
}

fn check_transaction_activity(
    token: &Token,
    config: &DexScreenerFilters,
) -> Option<FilterRejectionReason> {
    if !config.transactions_enabled {
        return None;
    }

    let m5_total = token
        .txns_m5_buys
        .unwrap_or(0)
        .saturating_add(token.txns_m5_sells.unwrap_or(0));
    if m5_total < config.min_transactions_5min {
        return Some(FilterRejectionReason::DexScreenerInsufficientTransactions5Min);
    }

    let h1_total = token
        .txns_h1_buys
        .unwrap_or(0)
        .saturating_add(token.txns_h1_sells.unwrap_or(0));
    if h1_total < config.min_transactions_1h {
        return Some(FilterRejectionReason::DexScreenerInsufficientTransactions1H);
    }

    None
}

fn check_liquidity_requirements(
    token: &Token,
    config: &DexScreenerFilters,
) -> Option<FilterRejectionReason> {
    if !config.liquidity_enabled {
        return None;
    }

    let liquidity_usd = token.liquidity_usd?;

    if liquidity_usd <= 0.0 {
        return Some(FilterRejectionReason::DexScreenerZeroLiquidity);
    }

    if liquidity_usd < config.min_liquidity_usd {
        return Some(FilterRejectionReason::DexScreenerInsufficientLiquidity);
    }

    if liquidity_usd > config.max_liquidity_usd {
        return Some(FilterRejectionReason::DexScreenerLiquidityTooHigh);
    }

    None
}

fn check_market_cap_requirements(
    token: &Token,
    config: &DexScreenerFilters,
) -> Option<FilterRejectionReason> {
    if !config.market_cap_enabled {
        return None;
    }

    let market_cap = token.market_cap?;

    if market_cap < config.min_market_cap_usd {
        return Some(FilterRejectionReason::DexScreenerMarketCapTooLow);
    }

    if market_cap > config.max_market_cap_usd {
        return Some(FilterRejectionReason::DexScreenerMarketCapTooHigh);
    }

    None
}

fn check_security_requirements(
    token: &Token,
    config: &RugCheckFilters,
) -> Option<FilterRejectionReason> {
    if !config.enabled {
        return None;
    }

    if config.block_rugged_tokens && token.is_rugged {
        return Some(FilterRejectionReason::RugcheckRuggedToken);
    }

    if config.risk_score_enabled {
        if let Some(score) = token.security_score {
            if score > config.max_risk_score {
                return Some(FilterRejectionReason::RugcheckRiskScoreTooHigh);
            }
        }
    }

    if config.authority_checks_enabled && config.require_authorities_safe {
        if !config.allow_mint_authority && token.mint_authority.is_some() {
            return Some(FilterRejectionReason::RugcheckMintAuthorityBlocked);
        }

        if !config.allow_freeze_authority && token.freeze_authority.is_some() {
            return Some(FilterRejectionReason::RugcheckFreezeAuthorityBlocked);
        }
    }

    if config.holder_distribution_enabled {
        if let Some(total) = token.total_holders {
            if total < config.min_unique_holders as i64 {
                return Some(FilterRejectionReason::RugcheckNotEnoughHolders);
            }
        }

        let mut top_holders = token.top_holders.clone();
        top_holders.sort_by(|a, b| match b.pct.partial_cmp(&a.pct) {
            Some(Ordering::Greater) | Some(Ordering::Equal) => Ordering::Greater,
            Some(Ordering::Less) => Ordering::Less,
            None => Ordering::Equal,
        });

        if let Some(first) = top_holders.first() {
            if first.pct > config.max_top_holder_pct {
                return Some(FilterRejectionReason::RugcheckTopHolderTooHigh);
            }
        }

        let top_three_sum: f64 = top_holders.iter().take(3).map(|holder| holder.pct).sum();
        if top_three_sum > config.max_top_3_holders_pct {
            return Some(FilterRejectionReason::RugcheckTop3HoldersTooHigh);
        }
    }

    if config.insider_holder_checks_enabled {
        let insider_count = token
            .top_holders
            .iter()
            .take(10)
            .filter(|holder| holder.insider)
            .count() as u32;
        if insider_count > config.max_insider_holders_in_top_10 {
            return Some(FilterRejectionReason::RugcheckInsiderHolderCount);
        }

        let insider_total_pct: f64 = token
            .top_holders
            .iter()
            .filter(|holder| holder.insider)
            .map(|holder| holder.pct)
            .sum();
        if insider_total_pct > config.max_insider_total_pct {
            return Some(FilterRejectionReason::RugcheckInsiderTotalPct);
        }
    }

    if config.max_creator_balance_pct > 0.0 {
        if let Some(creator_pct) = token.creator_balance_pct {
            if creator_pct > config.max_creator_balance_pct {
                return Some(FilterRejectionReason::RugcheckCreatorBalanceTooHigh);
            }
        }
    }

    if config.transfer_fee_enabled {
        if let Some(fee_pct) = token.transfer_fee_pct {
            if config.block_transfer_fee_tokens && fee_pct > 0.0 {
                return Some(FilterRejectionReason::RugcheckTransferFeePresent);
            }

            if fee_pct > config.max_transfer_fee_pct {
                return Some(FilterRejectionReason::RugcheckTransferFeeTooHigh);
            }
        } else if config.block_transfer_fee_tokens {
            // If fee data missing and we block any fee, treat as rejection to stay safe
            return Some(FilterRejectionReason::RugcheckTransferFeePresent);
        }
    }

    None
}

fn has_decimals_in_database(mint: &str) -> bool {
    if mint == crate::tokens::SOL_MINT || mint == crate::tokens::WSOL_MINT {
        return true;
    }

    get_cached_decimals(mint).is_some()
}

#[derive(Default)]
struct FilteringStats {
    total_processed: usize,
    passed: usize,
    rejected: usize,
    target_reached: bool,
    rejection_counts: HashMap<FilterRejectionReason, usize>,
}

impl FilteringStats {
    fn record_rejection(&mut self, reason: FilterRejectionReason) {
        self.rejected += 1;
        self.rejection_counts
            .entry(reason)
            .and_modify(|count| *count += 1)
            .or_insert(1);
    }

    fn rejection_summary(&self) -> String {
        if self.rejection_counts.is_empty() {
            return "-".to_string();
        }

        let mut parts: Vec<(FilterRejectionReason, usize)> =
            self.rejection_counts.iter().map(|(k, v)| (*k, *v)).collect();
        parts.sort_by(|a, b| b.1.cmp(&a.1));

        parts
            .iter()
            .take(5)
            .map(|(reason, count)| format!("{}:{}", reason.label(), count))
            .collect::<Vec<_>>()
            .join(",")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum FilterRejectionReason {
    NoDecimalsInDatabase,
    TokenTooNew,
    CooldownFiltered,
    DexScreenerEmptyName,
    DexScreenerEmptySymbol,
    DexScreenerEmptyLogoUrl,
    DexScreenerEmptyWebsiteUrl,
    DexScreenerInsufficientTransactions5Min,
    DexScreenerInsufficientTransactions1H,
    DexScreenerZeroLiquidity,
    DexScreenerInsufficientLiquidity,
    DexScreenerLiquidityTooHigh,
    DexScreenerMarketCapTooLow,
    DexScreenerMarketCapTooHigh,
    RugcheckRuggedToken,
    RugcheckRiskScoreTooHigh,
    RugcheckMintAuthorityBlocked,
    RugcheckFreezeAuthorityBlocked,
    RugcheckTopHolderTooHigh,
    RugcheckTop3HoldersTooHigh,
    RugcheckNotEnoughHolders,
    RugcheckInsiderHolderCount,
    RugcheckInsiderTotalPct,
    RugcheckCreatorBalanceTooHigh,
    RugcheckTransferFeePresent,
    RugcheckTransferFeeTooHigh,
}

impl FilterRejectionReason {
    fn label(&self) -> &'static str {
        match self {
            FilterRejectionReason::NoDecimalsInDatabase => "no_decimals",
            FilterRejectionReason::TokenTooNew => "token_too_new",
            FilterRejectionReason::CooldownFiltered => "cooldown_filtered",
            FilterRejectionReason::DexScreenerEmptyName => "dex_empty_name",
            FilterRejectionReason::DexScreenerEmptySymbol => "dex_empty_symbol",
            FilterRejectionReason::DexScreenerEmptyLogoUrl => "dex_empty_logo",
            FilterRejectionReason::DexScreenerEmptyWebsiteUrl => "dex_empty_website",
            FilterRejectionReason::DexScreenerInsufficientTransactions5Min => "dex_txn_5m",
            FilterRejectionReason::DexScreenerInsufficientTransactions1H => "dex_txn_1h",
            FilterRejectionReason::DexScreenerZeroLiquidity => "dex_zero_liq",
            FilterRejectionReason::DexScreenerInsufficientLiquidity => "dex_liq_low",
            FilterRejectionReason::DexScreenerLiquidityTooHigh => "dex_liq_high",
            FilterRejectionReason::DexScreenerMarketCapTooLow => "dex_mcap_low",
            FilterRejectionReason::DexScreenerMarketCapTooHigh => "dex_mcap_high",
            FilterRejectionReason::RugcheckRuggedToken => "rug_rugged",
            FilterRejectionReason::RugcheckRiskScoreTooHigh => "rug_score",
            FilterRejectionReason::RugcheckMintAuthorityBlocked => "rug_mint_authority",
            FilterRejectionReason::RugcheckFreezeAuthorityBlocked => "rug_freeze_authority",
            FilterRejectionReason::RugcheckTopHolderTooHigh => "rug_top_holder",
            FilterRejectionReason::RugcheckTop3HoldersTooHigh => "rug_top3_holders",
            FilterRejectionReason::RugcheckNotEnoughHolders => "rug_min_holders",
            FilterRejectionReason::RugcheckInsiderHolderCount => "rug_insider_count",
            FilterRejectionReason::RugcheckInsiderTotalPct => "rug_insider_pct",
            FilterRejectionReason::RugcheckCreatorBalanceTooHigh => "rug_creator_pct",
            FilterRejectionReason::RugcheckTransferFeePresent => "rug_transfer_fee_present",
            FilterRejectionReason::RugcheckTransferFeeTooHigh => "rug_transfer_fee_high",
        }
    }
}
