use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::config::with_config;
use crate::logger::{log, LogTag};
use crate::tokens::summary::TokenSummary;

use super::engine::compute_snapshot;
use super::types::{
    FilteringQuery, FilteringQueryResult, FilteringSnapshot, FilteringStatsSnapshot, FilteringView,
    PassedToken, RejectedToken, SortDirection, TokenEntry, TokenSortKey,
};

static GLOBAL_STORE: Lazy<Arc<FilteringStore>> = Lazy::new(|| Arc::new(FilteringStore::new()));

const STALE_MULTIPLIER: u64 = 3;

pub struct FilteringStore {
    snapshot: RwLock<Option<Arc<FilteringSnapshot>>>,
}

impl FilteringStore {
    fn new() -> Self {
        Self {
            snapshot: RwLock::new(None),
        }
    }

    async fn ensure_snapshot(&self) -> Result<Arc<FilteringSnapshot>, String> {
        let max_age = with_config(|cfg| cfg.filtering.filter_cache_ttl_secs.max(5));
        let stale_snapshot = self.snapshot.read().await.clone();

        if let Some(existing) = stale_snapshot.as_ref() {
            if !is_snapshot_stale(existing, max_age) {
                return Ok(existing.clone());
            }
        }

        match self.try_refresh().await {
            Ok(snapshot) => Ok(snapshot),
            Err(err) => {
                if let Some(existing) = stale_snapshot {
                    let age_secs = snapshot_age_secs(existing.as_ref());
                    log(
                        LogTag::Filtering,
                        "SNAPSHOT_FALLBACK",
                        &format!(
                            "refresh_failed={} using_stale_snapshot age_secs={}",
                            err, age_secs
                        ),
                    );
                    Ok(existing)
                } else {
                    Err(err)
                }
            }
        }
    }

    async fn try_refresh(&self) -> Result<Arc<FilteringSnapshot>, String> {
        let config = with_config(|cfg| cfg.filtering.clone());
        let snapshot = Arc::new(compute_snapshot(config).await?);
        let mut guard = self.snapshot.write().await;
        *guard = Some(snapshot.clone());
        Ok(snapshot)
    }

    pub async fn refresh(&self) -> Result<(), String> {
        self.try_refresh().await.map(|_| ())
    }

    pub async fn get_filtered_mints(&self) -> Result<Vec<String>, String> {
        let snapshot = self.ensure_snapshot().await?;
        Ok(snapshot.filtered_mints.clone())
    }

    pub async fn get_passed_tokens(&self) -> Result<Vec<PassedToken>, String> {
        let snapshot = self.ensure_snapshot().await?;
        Ok(snapshot.passed_tokens.clone())
    }

    pub async fn get_rejected_tokens(&self) -> Result<Vec<RejectedToken>, String> {
        let snapshot = self.ensure_snapshot().await?;
        Ok(snapshot.rejected_tokens.clone())
    }

    pub async fn execute_query(
        &self,
        mut query: FilteringQuery,
    ) -> Result<FilteringQueryResult, String> {
        let (max_page_size, secure_threshold, recent_hours) = with_config(|cfg| {
            (
                cfg.webserver.tokens_tab.max_page_size,
                cfg.webserver.tokens_tab.secure_token_score_threshold,
                cfg.webserver.tokens_tab.recent_token_hours,
            )
        });

        query.clamp_page_size(max_page_size);
        if query.page == 0 {
            query.page = 1;
        }

        let snapshot = self.ensure_snapshot().await?;
        let recent_cutoff = if matches!(query.view, FilteringView::Recent) {
            Some(Utc::now() - ChronoDuration::hours(recent_hours.max(0)))
        } else {
            None
        };

        let entries = collect_entries(
            snapshot.as_ref(),
            query.view,
            secure_threshold,
            recent_cutoff,
        );
        let mut summaries: Vec<_> = entries
            .into_iter()
            .map(|entry| entry.summary.clone())
            .collect();

        apply_filters(&mut summaries, &query);
        sort_summaries(&mut summaries, query.sort_key, query.sort_direction);

        let total = summaries.len();
        let priced_total = summaries
            .iter()
            .filter(|summary| summary.has_pool_price)
            .count();
        let positions_total = summaries
            .iter()
            .filter(|summary| summary.has_open_position)
            .count();
        let blacklisted_total = summaries
            .iter()
            .filter(|summary| summary.blacklisted)
            .count();
        let total_pages = if total == 0 {
            0
        } else {
            (total + query.page_size - 1) / query.page_size
        };

        let normalized_page = if total_pages == 0 {
            1
        } else {
            query.page.min(total_pages)
        };

        let start_idx = normalized_page
            .saturating_sub(1)
            .saturating_mul(query.page_size);
        let end_idx = start_idx.saturating_add(query.page_size).min(total);
        let items = if start_idx < total {
            summaries[start_idx..end_idx].to_vec()
        } else {
            Vec::new()
        };

        Ok(FilteringQueryResult {
            items,
            page: normalized_page,
            page_size: query.page_size,
            total,
            total_pages,
            timestamp: snapshot.updated_at,
            priced_total,
            positions_total,
            blacklisted_total,
        })
    }

    pub async fn get_stats(&self) -> Result<FilteringStatsSnapshot, String> {
        let secure_threshold =
            with_config(|cfg| cfg.webserver.tokens_tab.secure_token_score_threshold);

        let snapshot = self.ensure_snapshot().await?;
        Ok(build_stats(snapshot.as_ref(), secure_threshold))
    }

    pub async fn snapshot_age(&self) -> Option<Duration> {
        let snapshot = self.snapshot.read().await.clone()?;
        let age = Utc::now()
            .signed_duration_since(snapshot.updated_at)
            .to_std()
            .ok();
        age
    }
}

pub fn global_store() -> Arc<FilteringStore> {
    GLOBAL_STORE.clone()
}

pub async fn refresh_snapshot() -> Result<(), String> {
    global_store().refresh().await
}

pub async fn get_filtered_mints() -> Result<Vec<String>, String> {
    global_store().get_filtered_mints().await
}

pub async fn get_passed_tokens() -> Result<Vec<PassedToken>, String> {
    global_store().get_passed_tokens().await
}

pub async fn get_rejected_tokens() -> Result<Vec<RejectedToken>, String> {
    global_store().get_rejected_tokens().await
}

pub async fn execute_query(query: FilteringQuery) -> Result<FilteringQueryResult, String> {
    global_store().execute_query(query).await
}

pub async fn get_stats() -> Result<FilteringStatsSnapshot, String> {
    global_store().get_stats().await
}

fn collect_entries<'a>(
    snapshot: &'a FilteringSnapshot,
    view: FilteringView,
    secure_threshold: i32,
    recent_cutoff: Option<DateTime<Utc>>,
) -> Vec<&'a TokenEntry> {
    match view {
        FilteringView::Pool => snapshot
            .tokens
            .values()
            .filter(|entry| entry.summary.has_pool_price)
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
            .collect(),
        FilteringView::Blacklisted => snapshot
            .tokens
            .values()
            .filter(|entry| entry.summary.blacklisted)
            .collect(),
        FilteringView::Positions => snapshot
            .tokens
            .values()
            .filter(|entry| entry.summary.has_open_position)
            .collect(),
        FilteringView::Secure => snapshot
            .tokens
            .values()
            .filter(|entry| {
                entry
                    .summary
                    .security_score
                    .map(|score| score >= secure_threshold)
                    .unwrap_or(false)
                    && !entry.summary.rugged.unwrap_or(false)
            })
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
                        .unwrap_or(false)
                })
                .collect()
        }
    }
}

fn apply_filters(items: &mut Vec<TokenSummary>, query: &FilteringQuery) {
    if let Some(search) = query.search.as_ref().map(|s| s.trim().to_lowercase()) {
        if !search.is_empty() {
            items.retain(|summary| {
                summary.symbol.to_lowercase().contains(&search)
                    || summary.mint.to_lowercase().contains(&search)
                    || summary
                        .name
                        .as_ref()
                        .map(|name| name.to_lowercase().contains(&search))
                        .unwrap_or(false)
            });
        }
    }

    if let Some(min) = query.min_liquidity {
        items.retain(|summary| summary.liquidity_usd.unwrap_or(0.0) >= min);
    }

    if let Some(max) = query.max_liquidity {
        items.retain(|summary| summary.liquidity_usd.unwrap_or(f64::MAX) <= max);
    }

    if let Some(min) = query.min_volume_24h {
        items.retain(|summary| summary.volume_24h.unwrap_or(0.0) >= min);
    }

    if let Some(max) = query.max_volume_24h {
        items.retain(|summary| summary.volume_24h.unwrap_or(f64::MAX) <= max);
    }

    if let Some(min) = query.min_security_score {
        items.retain(|summary| summary.security_score.unwrap_or(0) >= min);
    }

    if let Some(max) = query.max_security_score {
        items.retain(|summary| summary.security_score.unwrap_or(i32::MAX) <= max);
    }

    if let Some(min) = query.min_unique_holders {
        items.retain(|summary| summary.total_holders.unwrap_or(0) >= min);
    }

    if let Some(flag) = query.has_pool_price {
        items.retain(|summary| summary.has_pool_price == flag);
    }

    if let Some(flag) = query.has_open_position {
        items.retain(|summary| summary.has_open_position == flag);
    }

    if let Some(flag) = query.blacklisted {
        items.retain(|summary| summary.blacklisted == flag);
    }

    if let Some(flag) = query.has_ohlcv {
        items.retain(|summary| summary.has_ohlcv == flag);
    }
}

fn sort_summaries(items: &mut [TokenSummary], sort_key: TokenSortKey, direction: SortDirection) {
    let ascending = matches!(direction, SortDirection::Asc);
    items.sort_by(|a, b| {
        let ordering = match sort_key {
            TokenSortKey::Symbol => a.symbol.cmp(&b.symbol),
            TokenSortKey::PriceSol => cmp_f64(a.price_sol, b.price_sol),
            TokenSortKey::LiquidityUsd => cmp_f64(a.liquidity_usd, b.liquidity_usd),
            TokenSortKey::Volume24h => cmp_f64(a.volume_24h, b.volume_24h),
            TokenSortKey::Fdv => cmp_f64(a.fdv, b.fdv),
            TokenSortKey::MarketCap => cmp_f64(a.market_cap, b.market_cap),
            TokenSortKey::PriceChangeH1 => cmp_f64(a.price_change_h1, b.price_change_h1),
            TokenSortKey::PriceChangeH24 => cmp_f64(a.price_change_h24, b.price_change_h24),
            TokenSortKey::SecurityScore => a
                .security_score
                .unwrap_or(0)
                .cmp(&b.security_score.unwrap_or(0)),
            TokenSortKey::UpdatedAt => a.updated_at.cmp(&b.updated_at),
            TokenSortKey::Mint => a.mint.cmp(&b.mint),
        };

        if ascending {
            ordering
        } else {
            ordering.reverse()
        }
    });
}

fn cmp_f64(lhs: Option<f64>, rhs: Option<f64>) -> Ordering {
    let left = lhs.unwrap_or(0.0);
    let right = rhs.unwrap_or(0.0);
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

fn build_stats(snapshot: &FilteringSnapshot, secure_threshold: i32) -> FilteringStatsSnapshot {
    let mut with_pool_price = 0usize;
    let mut open_positions = 0usize;
    let mut blacklisted = 0usize;
    let mut secure_tokens = 0usize;

    for entry in snapshot.tokens.values() {
        if entry.summary.has_pool_price {
            with_pool_price += 1;
        }
        if entry.summary.has_open_position {
            open_positions += 1;
        }
        if entry.summary.blacklisted {
            blacklisted += 1;
        }
        if entry
            .summary
            .security_score
            .map(|score| score >= secure_threshold)
            .unwrap_or(false)
            && !entry.summary.rugged.unwrap_or(false)
        {
            secure_tokens += 1;
        }
    }

    let with_ohlcv = snapshot
        .tokens
        .values()
        .filter(|entry| entry.summary.has_ohlcv)
        .count();

    FilteringStatsSnapshot {
        total_tokens: snapshot.tokens.len(),
        with_pool_price,
        open_positions,
        blacklisted,
        secure_tokens,
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

fn is_snapshot_stale(snapshot: &FilteringSnapshot, max_age_secs: u64) -> bool {
    snapshot_age_secs(snapshot) > max_age_secs.saturating_mul(STALE_MULTIPLIER)
}
