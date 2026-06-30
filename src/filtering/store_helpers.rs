//! Free helper functions for the filtering store — entry collection, filtering,
//! sorting, pool-price overlay, stats building, and staleness checks.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration as ChronoDuration, Utc};

use crate::logger::{self, LogTag};
use crate::pools;
use crate::tokens::types::Token;

use super::types::{
    FilteringQuery, FilteringSnapshot, FilteringStatsSnapshot, FilteringView, SortDirection,
    TokenEntry, TokenSortKey,
};

// Snapshot is considered stale after FILTER_CACHE_STALE_SECS (180s = 3 min)
const FILTER_CACHE_STALE_SECS: u64 = 180;

pub(super) fn collect_entries<'a>(
    snapshot: &'a FilteringSnapshot,
    view: FilteringView,
    recent_cutoff: Option<DateTime<Utc>>,
) -> Vec<&'a TokenEntry> {
    match view {
        FilteringView::Pool => snapshot
            .tokens
            .values()
            .filter(|entry| entry.has_pool_price)
            .collect(),
        FilteringView::All => snapshot.tokens.values().collect(),
        FilteringView::Passed => {
            let mut seen = HashSet::new();
            snapshot
                .filtered_mints
                .iter()
                .filter_map(|mint| {
                    if seen.insert(mint.clone()) {
                        snapshot.tokens.get(mint)
                    } else {
                        None
                    }
                })
                .collect()
        }
        FilteringView::Rejected => snapshot
            .rejected_mints
            .iter()
            .filter_map(|mint| snapshot.tokens.get(mint))
            .filter(|entry| !entry.token.is_blacklisted)
            .collect(),
        FilteringView::Blacklisted => snapshot
            .tokens
            .values()
            .filter(|entry| entry.token.is_blacklisted)
            .collect(),
        FilteringView::Positions => snapshot
            .tokens
            .values()
            .filter(|entry| entry.has_open_position)
            .collect(),
        FilteringView::Recent => {
            let cutoff = recent_cutoff.unwrap_or_else(Utc::now);
            snapshot
                .tokens
                .values()
                .filter(|entry| {
                    entry
                        .pair_created_at
                        .and_then(|ts| DateTime::<Utc>::from_timestamp(ts, 0))
                        .map(|created| created > cutoff)
                        .unwrap_or_default()
                })
                .collect()
        }
        FilteringView::NoMarketData => snapshot
            .tokens
            .values()
            .filter(|entry| !entry.has_pool_price)
            .collect(),
    }
}

pub(super) fn apply_filters(
    items: &mut Vec<&Token>,
    query: &FilteringQuery,
    snapshot: &FilteringSnapshot,
) {
    // quick maps for derived flags
    let flags: HashMap<&str, (&TokenEntry, bool, bool, bool)> = snapshot
        .tokens
        .iter()
        .map(|(mint, entry)| {
            (
                mint.as_str(),
                (
                    entry,
                    entry.has_pool_price,
                    entry.has_open_position,
                    entry.has_ohlcv,
                ),
            )
        })
        .collect();

    if let Some(search) = query.search.as_ref().map(|s| s.trim().to_lowercase()) {
        if !search.is_empty() {
            items.retain(|token| {
                token.symbol.to_lowercase().contains(&search)
                    || token.mint.to_lowercase().contains(&search)
                    || token.name.to_lowercase().contains(&search)
            });
        }
    }

    if let Some(min) = query.min_liquidity {
        items.retain(|t| t.liquidity_usd.unwrap_or_default() >= min);
    }

    if let Some(max) = query.max_liquidity {
        items.retain(|t| t.liquidity_usd.unwrap_or(f64::MAX) <= max);
    }

    if let Some(min) = query.min_volume_24h {
        items.retain(|t| t.volume_h24.unwrap_or_default() >= min);
    }

    if let Some(max) = query.max_volume_24h {
        items.retain(|t| t.volume_h24.unwrap_or(f64::MAX) <= max);
    }

    if let Some(max) = query.max_risk_score {
        items.retain(|t| t.security_score.unwrap_or(i32::MAX) <= max);
    }

    if let Some(min) = query.min_unique_holders {
        items.retain(|t| t.total_holders.unwrap_or_default() >= (min as i64));
    }

    if let Some(flag) = query.has_pool_price {
        items.retain(|t| {
            flags
                .get(t.mint.as_str())
                .map(|(_, hp, _, _)| *hp == flag)
                .unwrap_or_default()
        });
    }

    if let Some(flag) = query.has_open_position {
        items.retain(|t| {
            flags
                .get(t.mint.as_str())
                .map(|(_, _, op, _)| *op == flag)
                .unwrap_or_default()
        });
    }

    if let Some(flag) = query.blacklisted {
        items.retain(|t| t.is_blacklisted == flag);
    }

    if let Some(flag) = query.has_ohlcv {
        items.retain(|t| {
            flags
                .get(t.mint.as_str())
                .map(|(_, _, _, oh)| *oh == flag)
                .unwrap_or_default()
        });
    }

    // Filter by rejection reason using token's persisted last_rejection_reason
    if let Some(target_reason) = query.rejection_reason.as_ref() {
        items.retain(|t| {
            t.last_rejection_reason
                .as_ref()
                .map(|reason| reason.eq_ignore_ascii_case(target_reason))
                .unwrap_or_default()
        });
    }
}

pub(super) fn sort_tokens(
    items: &mut Vec<&Token>,
    sort_key: TokenSortKey,
    direction: SortDirection,
) {
    let ascending = matches!(direction, SortDirection::Asc);
    items.sort_by(|a, b| {
        let ordering = match sort_key {
            TokenSortKey::Symbol => a.symbol.cmp(&b.symbol),
            TokenSortKey::PriceSol => {
                // Use real-time pool price for sorting if available
                let price_a = pools::get_pool_price(&a.mint)
                    .map(|p| p.price_sol)
                    .unwrap_or(a.price_sol);
                let price_b = pools::get_pool_price(&b.mint)
                    .map(|p| p.price_sol)
                    .unwrap_or(b.price_sol);
                cmp_f64(Some(price_a), Some(price_b))
            }
            TokenSortKey::LiquidityUsd => cmp_f64(a.liquidity_usd, b.liquidity_usd),
            TokenSortKey::Volume24h => cmp_f64(a.volume_h24, b.volume_h24),
            TokenSortKey::Fdv => cmp_f64(a.fdv, b.fdv),
            TokenSortKey::MarketCap => cmp_f64(a.market_cap, b.market_cap),
            TokenSortKey::PriceChangeH1 => cmp_f64(a.price_change_h1, b.price_change_h1),
            TokenSortKey::PriceChangeH24 => cmp_f64(a.price_change_h24, b.price_change_h24),
            TokenSortKey::RiskScore => a
                .security_score
                .unwrap_or(i32::MAX)
                .cmp(&b.security_score.unwrap_or(i32::MAX)),
            TokenSortKey::MarketDataLastFetchedAt => a
                .market_data_last_fetched_at
                .cmp(&b.market_data_last_fetched_at),
            TokenSortKey::FirstDiscoveredAt => a.first_discovered_at.cmp(&b.first_discovered_at),
            TokenSortKey::MetadataLastFetchedAt => {
                let lhs = a.metadata_last_fetched_at;
                let rhs = b.metadata_last_fetched_at;
                lhs.cmp(&rhs)
            }
            TokenSortKey::BlockchainCreatedAt => {
                let lhs = a.blockchain_created_at.unwrap_or(a.first_discovered_at);
                let rhs = b.blockchain_created_at.unwrap_or(b.first_discovered_at);
                lhs.cmp(&rhs)
            }
            TokenSortKey::PoolPriceLastCalculatedAt => a
                .pool_price_last_calculated_at
                .cmp(&b.pool_price_last_calculated_at),
            TokenSortKey::Mint => a.mint.cmp(&b.mint),
            TokenSortKey::Txns5m => {
                let a_total = a.txns_5m_total();
                let b_total = b.txns_5m_total();
                a_total.cmp(&b_total)
            }
            TokenSortKey::Txns1h => {
                let a_total = a.txns_1h_total();
                let b_total = b.txns_1h_total();
                a_total.cmp(&b_total)
            }
            TokenSortKey::Txns6h => {
                let a_total = a.txns_6h_total();
                let b_total = b.txns_6h_total();
                a_total.cmp(&b_total)
            }
            TokenSortKey::Txns24h => {
                let a_total = a.txns_24h_total();
                let b_total = b.txns_24h_total();
                a_total.cmp(&b_total)
            }
        };

        if ascending {
            ordering
        } else {
            ordering.reverse()
        }
    });
}

fn cmp_f64(lhs: Option<f64>, rhs: Option<f64>) -> Ordering {
    let left = lhs.unwrap_or_default();
    let right = rhs.unwrap_or_default();
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

pub(super) fn overlay_pool_price_data(tokens: &mut [Token]) {
    for token in tokens.iter_mut() {
        if let Some(price_result) = pools::get_pool_price(&token.mint) {
            let old_price = token.price_sol;
            let new_price = price_result.price_sol;
            token.price_sol = new_price;

            let age_duration = price_result.timestamp.elapsed();
            let updated_at = ChronoDuration::from_std(age_duration)
                .map(|duration| Utc::now() - duration)
                .unwrap_or_else(|_| Utc::now());
            token.pool_price_last_calculated_at = updated_at;

            logger::debug(
                LogTag::Filtering,
                &format!(
                    "pool_overlay mint={} symbol={} old_price={:.12} new_price={:.12} diff={:.12} age={:.1}s",
                    token.mint,
                    token.symbol,
                    old_price,
                    new_price,
                    (new_price - old_price).abs(),
                    age_duration.as_secs_f64(),
                ),
            );
        }
    }
}

pub(super) async fn build_stats(snapshot: &FilteringSnapshot) -> FilteringStatsSnapshot {
    let mut with_pool_price = 0usize;
    let mut open_positions = 0usize;
    let mut blacklisted = 0usize;

    for entry in snapshot.tokens.values() {
        if entry.has_pool_price {
            with_pool_price += 1;
        }
        if entry.has_open_position {
            open_positions += 1;
        }
        if entry.token.is_blacklisted {
            blacklisted += 1;
        }
    }

    let with_ohlcv = snapshot
        .tokens
        .values()
        .filter(|entry| entry.has_ohlcv)
        .count();

    // Query actual database count (includes tokens without market data)
    let total_in_database = match crate::tokens::count_tokens_async().await {
        Ok(count) => count,
        Err(e) => {
            logger::warning(
                LogTag::Filtering,
                &format!(
                    "Failed to get database token count: {}, using snapshot count",
                    e
                ),
            );
            snapshot.tokens.len() // Fallback to snapshot count
        }
    };

    FilteringStatsSnapshot {
        total_tokens_in_database: total_in_database,
        total_tokens: snapshot.tokens.len(),
        with_pool_price,
        open_positions,
        blacklisted,
        with_ohlcv,
        passed_filtering: snapshot.passed_tokens.len(),
        updated_at: snapshot.updated_at,
    }
}

fn snapshot_age_secs(snapshot: &FilteringSnapshot) -> u64 {
    Utc::now()
        .signed_duration_since(snapshot.updated_at)
        .num_seconds()
        .max(0) as u64
}

pub(super) fn is_snapshot_stale(snapshot: &FilteringSnapshot) -> bool {
    snapshot_age_secs(snapshot) > FILTER_CACHE_STALE_SECS
}
