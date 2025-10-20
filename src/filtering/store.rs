use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use once_cell::sync::Lazy;
use tokio::sync::RwLock;

use crate::logger::{log, LogTag};
use crate::tokens::types::Token;

use super::engine::compute_snapshot;
use super::types::{
    FilteringQuery, FilteringQueryResult, FilteringSnapshot, FilteringStatsSnapshot, FilteringView,
    PassedToken, RejectedToken, SortDirection, TokenEntry, TokenSortKey,
};

static GLOBAL_STORE: Lazy<Arc<FilteringStore>> = Lazy::new(|| Arc::new(FilteringStore::new()));

// Timing constants
const FILTER_CACHE_TTL_SECS: u64 = 30;
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
        let max_age = FILTER_CACHE_TTL_SECS;
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
        let config = crate::config::with_config(|cfg| cfg.filtering.clone());
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
        let (max_page_size, secure_threshold, recent_hours) = crate::config::with_config(|cfg| {
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
        // Collect raw tokens for filtering/sorting on Token fields
        let mut tokens: Vec<_> = entries
            .into_iter()
            .map(|entry| entry.token.clone())
            .collect();

        apply_filters(&mut tokens, &query, snapshot.as_ref());
        sort_tokens(&mut tokens, query.sort_key, query.sort_direction);

        let total = tokens.len();
        // Build a quick lookup for derived flags from snapshot entries
        let mut priced_mints: Vec<String> = Vec::new();
        let mut open_position_mints: Vec<String> = Vec::new();
        let mut ohlcv_mints: Vec<String> = Vec::new();
        for (mint, entry) in &snapshot.tokens {
            if entry.has_pool_price {
                priced_mints.push(mint.clone());
            }
            if entry.has_open_position {
                open_position_mints.push(mint.clone());
            }
            if entry.has_ohlcv {
                ohlcv_mints.push(mint.clone());
            }
        }
        let priced_set: std::collections::HashSet<_> = priced_mints.iter().cloned().collect();
        let open_set: std::collections::HashSet<_> = open_position_mints.iter().cloned().collect();
        let ohlcv_set: std::collections::HashSet<_> = ohlcv_mints.iter().cloned().collect();

        let priced_total = tokens
            .iter()
            .filter(|t| priced_set.contains(&t.mint))
            .count();
        let positions_total = tokens.iter().filter(|t| open_set.contains(&t.mint)).count();
        let blacklisted_total = tokens.iter().filter(|t| t.is_blacklisted).count();
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
            tokens[start_idx..end_idx].to_vec()
        } else {
            Vec::new()
        };

        let mut rejection_reasons = HashMap::new();
        let mut available_rejection_reasons = Vec::new();
        if matches!(query.view, FilteringView::Rejected) {
            let reason_lookup: HashMap<&str, &str> = snapshot
                .rejected_tokens
                .iter()
                .map(|entry| (entry.mint.as_str(), entry.reason.as_str()))
                .collect();

            let mut unique_reasons: std::collections::HashSet<String> = std::collections::HashSet::new();
            for entry in &snapshot.rejected_tokens {
                let trimmed = entry.reason.trim();
                if !trimmed.is_empty() {
                    unique_reasons.insert(trimmed.to_string());
                }
            }

            let mut sorted_reasons: Vec<String> = unique_reasons.into_iter().collect();
            sorted_reasons.sort_unstable_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
            available_rejection_reasons = sorted_reasons;

            for token in &items {
                if let Some(reason) = reason_lookup.get(token.mint.as_str()) {
                    rejection_reasons.insert(token.mint.clone(), (*reason).to_string());
                }
            }
        }

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
            priced_mints,
            open_position_mints,
            ohlcv_mints,
            rejection_reasons,
            available_rejection_reasons,
        })
    }

    pub async fn get_stats(&self) -> Result<FilteringStatsSnapshot, String> {
        let secure_threshold =
            crate::config::with_config(|cfg| cfg.webserver.tokens_tab.secure_token_score_threshold);

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
        FilteringView::Secure => snapshot
            .tokens
            .values()
            .filter(|entry| {
                entry
                    .token
                    .security_score
                    .map(|score| score <= secure_threshold)
                    .unwrap_or(false)
                    && !entry.token.is_rugged
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
        FilteringView::NoMarketData => snapshot
            .tokens
            .values()
            .filter(|entry| !entry.has_pool_price)
            .collect(),
    }
}

fn apply_filters(items: &mut Vec<Token>, query: &FilteringQuery, snapshot: &FilteringSnapshot) {
    // quick maps for derived flags
    let flags: std::collections::HashMap<&str, (&TokenEntry, bool, bool, bool)> = snapshot
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

    let rejection_reasons: std::collections::HashMap<&str, &str> = snapshot
        .rejected_tokens
        .iter()
        .map(|entry| (entry.mint.as_str(), entry.reason.as_str()))
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
        items.retain(|t| t.liquidity_usd.unwrap_or(0.0) >= min);
    }

    if let Some(max) = query.max_liquidity {
        items.retain(|t| t.liquidity_usd.unwrap_or(f64::MAX) <= max);
    }

    if let Some(min) = query.min_volume_24h {
        items.retain(|t| t.volume_h24.unwrap_or(0.0) >= min);
    }

    if let Some(max) = query.max_volume_24h {
        items.retain(|t| t.volume_h24.unwrap_or(f64::MAX) <= max);
    }

    if let Some(max) = query.max_risk_score {
        items.retain(|t| t.security_score.unwrap_or(i32::MAX) <= max);
    }

    if let Some(min) = query.min_unique_holders {
        items.retain(|t| t.total_holders.unwrap_or(0) >= (min as i64));
    }

    if let Some(flag) = query.has_pool_price {
        items.retain(|t| {
            flags
                .get(t.mint.as_str())
                .map(|(_, hp, _, _)| *hp == flag)
                .unwrap_or(false)
        });
    }

    if let Some(flag) = query.has_open_position {
        items.retain(|t| {
            flags
                .get(t.mint.as_str())
                .map(|(_, _, op, _)| *op == flag)
                .unwrap_or(false)
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
                .unwrap_or(false)
        });
    }

    if let Some(target_reason) = query.rejection_reason.as_ref() {
        items.retain(|t| {
            rejection_reasons
                .get(t.mint.as_str())
                .map(|reason| reason.eq_ignore_ascii_case(target_reason))
                .unwrap_or(false)
        });
    }
}

fn sort_tokens(items: &mut [Token], sort_key: TokenSortKey, direction: SortDirection) {
    let ascending = matches!(direction, SortDirection::Asc);
    items.sort_by(|a, b| {
        let ordering = match sort_key {
            TokenSortKey::Symbol => a.symbol.cmp(&b.symbol),
            TokenSortKey::PriceSol => cmp_f64(Some(a.price_sol), Some(b.price_sol)),
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
        if entry.has_pool_price {
            with_pool_price += 1;
        }
        if entry.has_open_position {
            open_positions += 1;
        }
        if entry.token.is_blacklisted {
            blacklisted += 1;
        }
        if entry
            .token
            .security_score
            .map(|score| score <= secure_threshold)
            .unwrap_or(false)
            && !entry.token.is_rugged
        {
            secure_tokens += 1;
        }
    }

    let with_ohlcv = snapshot
        .tokens
        .values()
        .filter(|entry| entry.has_ohlcv)
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
