use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use once_cell::sync::Lazy;
use serde_json::json;
use tokio::sync::RwLock;

use crate::events::{record_filtering_event, Severity};
use crate::logger::{self, LogTag};
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
const TOKENS_TAB_MAX_PAGE_SIZE: usize = 200;
const TOKENS_TAB_RECENT_TOKEN_HOURS: i64 = 24;

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
                // DEBUG: Record cache hit (sampled)
                if existing.filtered_mints.len() % 50 == 0 {
                    let age_secs = snapshot_age_secs(existing.as_ref());
                    let passed_count = existing.filtered_mints.len();
                    tokio::spawn(async move {
                        record_filtering_event(
                            "snapshot_cache_hit",
                            Severity::Debug,
                            None,
                            None,
                            json!({
                                "age_secs": age_secs,
                                "passed_count": passed_count,
                            }),
                        )
                        .await
                    });
                }
                return Ok(existing.clone());
            }
        }

        match self.try_refresh().await {
            Ok(snapshot) => Ok(snapshot),
            Err(err) => {
                if let Some(existing) = stale_snapshot {
                    let age_secs = snapshot_age_secs(existing.as_ref());
                    logger::info(
                        LogTag::Filtering,
                        &format!(
                            "refresh_failed={} using_stale_snapshot age_secs={}",
                            err, age_secs
                        ),
                    );

                    // WARN: Record using stale snapshot
                    let err_clone = err.clone();
                    tokio::spawn(async move {
                        record_filtering_event(
                            "snapshot_using_stale",
                            Severity::Warn,
                            None,
                            None,
                            json!({
                                "error": err_clone,
                                "age_secs": age_secs,
                            }),
                        )
                        .await
                    });

                    Ok(existing)
                } else {
                    // ERROR: Record no snapshot available
                    let err_clone = err.clone();
                    tokio::spawn(async move {
                        record_filtering_event(
                            "snapshot_unavailable",
                            Severity::Error,
                            None,
                            None,
                            json!({
                                "error": err_clone,
                            }),
                        )
                        .await
                    });

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

        // INFO: Record snapshot refresh
        let passed_count = snapshot.filtered_mints.len();
        let rejected_count = snapshot.rejected_mints.len();
        tokio::spawn(async move {
            record_filtering_event(
                "snapshot_refreshed",
                Severity::Info,
                None,
                None,
                json!({
                    "passed_count": passed_count,
                    "rejected_count": rejected_count,
                    "total_tokens": passed_count + rejected_count,
                }),
            )
            .await
        });

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
        let max_page_size = TOKENS_TAB_MAX_PAGE_SIZE;
        let recent_hours = TOKENS_TAB_RECENT_TOKEN_HOURS;

        query.clamp_page_size(max_page_size);
        if query.page == 0 {
            query.page = 1;
        }

        // Special handling for "All" view - query database directly to get ALL tokens
        if matches!(query.view, FilteringView::All) {
            return self.execute_all_view_query(query).await;
        }

        // Special handling for "NoMarketData" view - query database for tokens with no market API data
        if matches!(query.view, FilteringView::NoMarketData) {
            return self.execute_no_market_view_query(query).await;
        }

        let snapshot = self.ensure_snapshot().await?;
        let recent_cutoff = if matches!(query.view, FilteringView::Recent) {
            Some(Utc::now() - ChronoDuration::hours(recent_hours.max(0)))
        } else {
            None
        };

        let entries = collect_entries(snapshot.as_ref(), query.view, recent_cutoff);
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

            let mut unique_reasons: std::collections::HashSet<String> =
                std::collections::HashSet::new();
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

    /// Execute query for "All" view by querying database directly (bypasses snapshot)
    async fn execute_all_view_query(
        &self,
        query: FilteringQuery,
    ) -> Result<FilteringQueryResult, String> {
        use crate::tokens::{count_tokens_async, get_all_tokens_optional_market_async};

        // Fast count query (no data loading)
        let total_count = count_tokens_async()
            .await
            .map_err(|e| format!("Failed to count tokens: {:?}", e))?;

        // Calculate pagination FIRST, then only fetch what we need
        let total_pages = if total_count == 0 {
            0
        } else {
            (total_count + query.page_size - 1) / query.page_size
        };

        let normalized_page = if total_pages == 0 {
            1
        } else {
            query.page.min(total_pages)
        };

        let offset = (normalized_page - 1) * query.page_size;

        // Map TokenSortKey to SQL column name
        let sort_by = match query.sort_key {
            TokenSortKey::Symbol => Some("symbol".to_string()),
            TokenSortKey::PriceSol => Some("price_sol".to_string()),
            TokenSortKey::LiquidityUsd => Some("liquidity_usd".to_string()),
            TokenSortKey::Volume24h => Some("volume_24h".to_string()),
            TokenSortKey::Fdv => Some("fdv".to_string()),
            TokenSortKey::MarketCap => Some("market_cap".to_string()),
            TokenSortKey::PriceChangeH1 => Some("price_change_h1".to_string()),
            TokenSortKey::PriceChangeH24 => Some("price_change_h24".to_string()),
            TokenSortKey::RiskScore => Some("risk_score".to_string()),
            TokenSortKey::UpdatedAt => Some("updated_at".to_string()),
            TokenSortKey::FirstSeenAt => Some("first_seen_at".to_string()),
            TokenSortKey::MetadataUpdatedAt => Some("metadata_updated_at".to_string()),
            TokenSortKey::TokenBirthAt => Some("token_birth_at".to_string()),
            TokenSortKey::Mint => Some("mint".to_string()),
            // Transaction sorts require in-memory sorting (need sum of buys+sells)
            TokenSortKey::Txns5m
            | TokenSortKey::Txns1h
            | TokenSortKey::Txns6h
            | TokenSortKey::Txns24h => None,
        };

        let sort_direction = match query.sort_direction {
            SortDirection::Asc => Some("asc".to_string()),
            SortDirection::Desc => Some("desc".to_string()),
        };

        // Only load the tokens for THIS page with proper sorting
        let items =
            get_all_tokens_optional_market_async(query.page_size, offset, sort_by, sort_direction)
                .await
                .map_err(|e| format!("Failed to get tokens from database: {:?}", e))?;

        // Get snapshot for derived flags lookup (pool price, open positions, ohlcv)
        let snapshot = self.ensure_snapshot().await?;

        // Build lookup sets for derived flags
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

        // Count totals (approximations based on snapshot)
        let priced_total = snapshot
            .tokens
            .values()
            .filter(|e| e.has_pool_price)
            .count();
        let positions_total = snapshot
            .tokens
            .values()
            .filter(|e| e.has_open_position)
            .count();
        let blacklisted_total = items.iter().filter(|t| t.is_blacklisted).count();

        Ok(FilteringQueryResult {
            items,
            page: normalized_page,
            page_size: query.page_size,
            total: total_count,
            total_pages,
            timestamp: snapshot.updated_at,
            priced_total,
            positions_total,
            blacklisted_total,
            priced_mints,
            open_position_mints,
            ohlcv_mints,
            rejection_reasons: HashMap::new(), // Not applicable for "All" view
            available_rejection_reasons: Vec::new(), // Not applicable for "All" view
        })
    }

    /// Execute query for "No Market Data" view using DB (no Dex/Gecko rows)
    async fn execute_no_market_view_query(
        &self,
        query: FilteringQuery,
    ) -> Result<FilteringQueryResult, String> {
        use crate::tokens::{count_tokens_no_market_async, get_tokens_no_market_async};

        // Count
        let total_count = count_tokens_no_market_async()
            .await
            .map_err(|e| format!("Failed to count no-market tokens: {:?}", e))?;

        let total_pages = if total_count == 0 {
            0
        } else {
            (total_count + query.page_size - 1) / query.page_size
        };
        let normalized_page = if total_pages == 0 {
            1
        } else {
            query.page.min(total_pages)
        };
        let offset = (normalized_page - 1) * query.page_size;

        // Sort mapping (limit to metadata/security)
        let sort_by = match query.sort_key {
            TokenSortKey::Symbol => Some("symbol".to_string()),
            TokenSortKey::RiskScore => Some("risk_score".to_string()),
            TokenSortKey::UpdatedAt => Some("updated_at".to_string()),
            TokenSortKey::FirstSeenAt => Some("first_seen_at".to_string()),
            TokenSortKey::MetadataUpdatedAt => Some("metadata_updated_at".to_string()),
            TokenSortKey::TokenBirthAt => Some("token_birth_at".to_string()),
            TokenSortKey::Mint => Some("mint".to_string()),
            _ => Some("updated_at".to_string()),
        };
        let sort_direction = match query.sort_direction {
            SortDirection::Asc => Some("asc".to_string()),
            SortDirection::Desc => Some("desc".to_string()),
        };

        let items = get_tokens_no_market_async(query.page_size, offset, sort_by, sort_direction)
            .await
            .map_err(|e| format!("Failed to load no-market tokens: {:?}", e))?;

        // Snapshot for timestamp and derived counts
        let snapshot = self.ensure_snapshot().await?;

        let priced_total = 0; // by definition of this view (no market), keep 0 to avoid confusion
        let positions_total = items
            .iter()
            .filter(|t| {
                snapshot
                    .tokens
                    .get(&t.mint)
                    .map(|e| e.has_open_position)
                    .unwrap_or(false)
            })
            .count();
        let blacklisted_total = items.iter().filter(|t| t.is_blacklisted).count();

        Ok(FilteringQueryResult {
            items,
            page: normalized_page,
            page_size: query.page_size,
            total: total_count,
            total_pages,
            timestamp: snapshot.updated_at,
            priced_total,
            positions_total,
            blacklisted_total,
            priced_mints: Vec::new(),
            open_position_mints: Vec::new(),
            ohlcv_mints: Vec::new(),
            rejection_reasons: HashMap::new(),
            available_rejection_reasons: Vec::new(),
        })
    }
    pub async fn get_stats(&self) -> Result<FilteringStatsSnapshot, String> {
        let snapshot = self.ensure_snapshot().await?;
        Ok(build_stats(snapshot.as_ref()))
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
            TokenSortKey::FirstSeenAt => a.first_seen_at.cmp(&b.first_seen_at),
            TokenSortKey::MetadataUpdatedAt => {
                let lhs = a.metadata_updated_at.unwrap_or(a.created_at);
                let rhs = b.metadata_updated_at.unwrap_or(b.created_at);
                lhs.cmp(&rhs)
            }
            TokenSortKey::TokenBirthAt => {
                let lhs = a.token_birth_at.unwrap_or(a.created_at);
                let rhs = b.token_birth_at.unwrap_or(b.created_at);
                lhs.cmp(&rhs)
            }
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
    let left = lhs.unwrap_or(0.0);
    let right = rhs.unwrap_or(0.0);
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

fn build_stats(snapshot: &FilteringSnapshot) -> FilteringStatsSnapshot {
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

    FilteringStatsSnapshot {
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

fn is_snapshot_stale(snapshot: &FilteringSnapshot, max_age_secs: u64) -> bool {
    snapshot_age_secs(snapshot) > max_age_secs.saturating_mul(STALE_MULTIPLIER)
}
