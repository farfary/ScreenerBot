use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use once_cell::sync::Lazy;
use serde_json::json;
use tokio::sync::{Mutex, RwLock};

use crate::events::{record_filtering_event, Severity};
use crate::logger::{self, LogTag};
use crate::pools;
use crate::tokens::types::Token;

use super::engine::compute_snapshot;
use super::types::{
    BlacklistReasonInfo, FilteringQuery, FilteringQueryResult, FilteringSnapshot,
    FilteringStatsSnapshot, FilteringView, PassedToken, RejectedToken, SortDirection, TokenEntry,
    TokenSortKey,
};

static GLOBAL_STORE: Lazy<Arc<FilteringStore>> = Lazy::new(|| Arc::new(FilteringStore::new()));

// Timing constants
// Snapshot is considered stale after FILTER_CACHE_STALE_SECS (180s = 3 min)
// Background refresh is triggered when snapshot age exceeds this threshold
const FILTER_CACHE_STALE_SECS: u64 = 180;
const TOKENS_TAB_MAX_PAGE_SIZE: usize = 200;
const TOKENS_TAB_RECENT_TOKEN_HOURS: i64 = 24;

pub struct FilteringStore {
    snapshot: RwLock<Option<Arc<FilteringSnapshot>>>,
    /// Prevents multiple concurrent refresh operations
    refresh_in_progress: AtomicBool,
    /// Mutex to serialize refresh attempts
    refresh_lock: Mutex<()>,
}

impl FilteringStore {
    fn new() -> Self {
        Self {
            snapshot: RwLock::new(None),
            refresh_in_progress: AtomicBool::new(false),
            refresh_lock: Mutex::new(()),
        }
    }

    /// Non-blocking snapshot access - returns cached snapshot immediately if available,
    /// or triggers background refresh if stale. Never blocks waiting for refresh.
    async fn ensure_snapshot(&self) -> Result<Arc<FilteringSnapshot>, String> {
        let stale_snapshot = self.snapshot.read().await.clone();

        // If we have any snapshot (even stale), return it immediately
        if let Some(existing) = stale_snapshot.as_ref() {
            let is_stale = is_snapshot_stale(existing);

            // Trigger background refresh if stale and not already refreshing
            if is_stale && !self.refresh_in_progress.load(AtomicOrdering::Relaxed) {
                let store = global_store();
                tokio::spawn(async move {
                    let _ = store.try_refresh_background().await;
                });
            }

            return Ok(existing.clone());
        }

        // No snapshot exists - must wait for first refresh
        // But use a timeout to avoid blocking indefinitely
        match tokio::time::timeout(Duration::from_secs(30), self.try_refresh()).await {
            Ok(Ok(snapshot)) => Ok(snapshot),
            Ok(Err(err)) => Err(err),
            Err(_) => Err("Snapshot refresh timed out after 30 seconds".to_string()),
        }
    }

    /// Background refresh - doesn't block, logs errors instead of returning them
    async fn try_refresh_background(&self) -> Result<(), String> {
        // Check if refresh is already in progress
        if self.refresh_in_progress.swap(true, AtomicOrdering::SeqCst) {
            logger::debug(LogTag::Filtering, "Skipping refresh - already in progress");
            return Ok(());
        }

        let result = self.try_refresh_inner().await;
        self.refresh_in_progress
            .store(false, AtomicOrdering::SeqCst);

        if let Err(ref err) = result {
            logger::warning(
                LogTag::Filtering,
                &format!("Background refresh failed: {}", err),
            );
        }

        result.map(|_| ())
    }

    async fn try_refresh(&self) -> Result<Arc<FilteringSnapshot>, String> {
        // Acquire refresh lock to prevent concurrent refreshes
        let _guard = self.refresh_lock.lock().await;

        // Check again if snapshot is still stale (another refresh might have completed)
        let existing = self.snapshot.read().await.clone();
        if let Some(ref snapshot) = existing {
            if !is_snapshot_stale(snapshot) {
                return Ok(snapshot.clone());
            }
        }

        self.try_refresh_inner().await
    }

    async fn try_refresh_inner(&self) -> Result<Arc<FilteringSnapshot>, String> {
        let config = crate::config::with_config(|cfg| cfg.filtering.clone());
        let previous_snapshot = {
            let guard = self.snapshot.read().await;
            guard.clone()
        };

        let snapshot = Arc::new(compute_snapshot(config, previous_snapshot.as_deref()).await?);
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
        // OPTIMIZATION: Use references to avoid cloning all tokens
        let mut tokens: Vec<&Token> = entries.into_iter().map(|entry| &entry.token).collect();

        apply_filters(&mut tokens, &query, snapshot.as_ref());

        // Sort references (using dynamic price lookup if needed)
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

        // Clone only the page we are returning
        let mut items: Vec<Token> = if start_idx < total {
            tokens[start_idx..end_idx]
                .iter()
                .map(|t| (*t).clone())
                .collect()
        } else {
            Vec::new()
        };

        // Apply pool price overlay only to the returned page
        if matches!(query.view, FilteringView::Pool) {
            overlay_pool_price_data(&mut items);
        }

        let mut rejection_reasons = HashMap::new();
        let mut available_rejection_reasons = Vec::new();
        if matches!(query.view, FilteringView::Rejected) {
            // Build rejection reasons from token's persisted last_rejection_reason (database)
            // This replaces the truncated snapshot.rejected_tokens lookup
            for token in &items {
                if let Some(ref reason) = token.last_rejection_reason {
                    let trimmed = reason.trim();
                    if !trimmed.is_empty() {
                        rejection_reasons.insert(token.mint.clone(), trimmed.to_string());
                    }
                }
            }

            // Collect unique reasons from database (not limited snapshot) for filter dropdown
            // Use get_rejection_stats_async() which queries all rejection reasons from update_tracking table
            match crate::tokens::get_rejection_stats_async().await {
                Ok(stats) => {
                    let mut unique_reasons: HashSet<String> = HashSet::new();
                    for (reason, _source, _count) in stats {
                        let trimmed = reason.trim();
                        if !trimmed.is_empty() {
                            unique_reasons.insert(trimmed.to_string());
                        }
                    }
                    let mut sorted_reasons: Vec<String> = unique_reasons.into_iter().collect();
                    sorted_reasons.sort_unstable_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
                    available_rejection_reasons = sorted_reasons;
                }
                Err(e) => {
                    logger::warning(
                        LogTag::Filtering,
                        &format!("Failed to get rejection stats from DB: {}", e),
                    );
                    // Fallback to snapshot if DB query fails
                    let mut unique_reasons: HashSet<String> = HashSet::new();
                    for entry in &snapshot.rejected_tokens {
                        let trimmed = entry.reason.trim();
                        if !trimmed.is_empty() {
                            unique_reasons.insert(trimmed.to_string());
                        }
                    }
                    let mut sorted_reasons: Vec<String> = unique_reasons.into_iter().collect();
                    sorted_reasons.sort_unstable_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
                    available_rejection_reasons = sorted_reasons;
                }
            }
        }

        let mut blacklist_reasons: HashMap<String, Vec<BlacklistReasonInfo>> = HashMap::new();
        for token in &items {
            if let Some(sources) = snapshot.blacklist_reasons.get(token.mint.as_str()) {
                blacklist_reasons.insert(token.mint.clone(), sources.clone());
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
            blacklist_reasons,
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
            TokenSortKey::MarketDataLastFetchedAt => {
                Some("market_data_last_fetched_at".to_string())
            }
            TokenSortKey::FirstDiscoveredAt => Some("first_discovered_at".to_string()),
            TokenSortKey::MetadataLastFetchedAt => Some("metadata_last_fetched_at".to_string()),
            TokenSortKey::BlockchainCreatedAt => Some("blockchain_created_at".to_string()),
            TokenSortKey::PoolPriceLastCalculatedAt => {
                Some("pool_price_last_calculated_at".to_string())
            }
            TokenSortKey::Mint => Some("mint".to_string()),
            // Transaction sorts - mapped to SQL expressions in database.rs
            TokenSortKey::Txns5m => Some("txns_5m".to_string()),
            TokenSortKey::Txns1h => Some("txns_1h".to_string()),
            TokenSortKey::Txns6h => Some("txns_6h".to_string()),
            TokenSortKey::Txns24h => Some("txns_24h".to_string()),
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

        let mut blacklist_reasons: HashMap<String, Vec<BlacklistReasonInfo>> = HashMap::new();
        for token in &items {
            if let Some(sources) = snapshot.blacklist_reasons.get(token.mint.as_str()) {
                blacklist_reasons.insert(token.mint.clone(), sources.clone());
            }
        }

        // Build rejection reasons from token's persisted last_rejection_reason (database)
        let mut rejection_reasons = HashMap::new();
        for token in &items {
            if let Some(ref reason) = token.last_rejection_reason {
                let trimmed = reason.trim();
                if !trimmed.is_empty() {
                    rejection_reasons.insert(token.mint.clone(), trimmed.to_string());
                }
            }
        }

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
            rejection_reasons,
            available_rejection_reasons: Vec::new(), // All view doesn't need filter dropdown
            blacklist_reasons,
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
            TokenSortKey::MarketDataLastFetchedAt => {
                Some("market_data_last_fetched_at".to_string())
            }
            TokenSortKey::FirstDiscoveredAt => Some("first_discovered_at".to_string()),
            TokenSortKey::MetadataLastFetchedAt => Some("metadata_last_fetched_at".to_string()),
            TokenSortKey::BlockchainCreatedAt => Some("blockchain_created_at".to_string()),
            TokenSortKey::PoolPriceLastCalculatedAt => {
                Some("pool_price_last_calculated_at".to_string())
            }
            TokenSortKey::Mint => Some("mint".to_string()),
            _ => Some("metadata_last_fetched_at".to_string()),
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

        let mut blacklist_reasons: HashMap<String, Vec<BlacklistReasonInfo>> = HashMap::new();
        for token in &items {
            if let Some(sources) = snapshot.blacklist_reasons.get(token.mint.as_str()) {
                blacklist_reasons.insert(token.mint.clone(), sources.clone());
            }
        }

        // Build rejection reasons from token's persisted last_rejection_reason (database)
        let mut rejection_reasons = HashMap::new();
        for token in &items {
            if let Some(ref reason) = token.last_rejection_reason {
                let trimmed = reason.trim();
                if !trimmed.is_empty() {
                    rejection_reasons.insert(token.mint.clone(), trimmed.to_string());
                }
            }
        }

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
            rejection_reasons,
            available_rejection_reasons: Vec::new(),
            blacklist_reasons,
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

fn apply_filters(items: &mut Vec<&Token>, query: &FilteringQuery, snapshot: &FilteringSnapshot) {
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

    // Filter by rejection reason using token's persisted last_rejection_reason
    if let Some(target_reason) = query.rejection_reason.as_ref() {
        items.retain(|t| {
            t.last_rejection_reason
                .as_ref()
                .map(|reason| reason.eq_ignore_ascii_case(target_reason))
                .unwrap_or(false)
        });
    }
}

fn sort_tokens(items: &mut Vec<&Token>, sort_key: TokenSortKey, direction: SortDirection) {
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
    let left = lhs.unwrap_or(0.0);
    let right = rhs.unwrap_or(0.0);
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

fn overlay_pool_price_data(tokens: &mut [Token]) {
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

fn is_snapshot_stale(snapshot: &FilteringSnapshot) -> bool {
    snapshot_age_secs(snapshot) > FILTER_CACHE_STALE_SECS
}
