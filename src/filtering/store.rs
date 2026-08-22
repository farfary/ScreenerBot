//! Token filter store — manages filtered token results with pagination and querying.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::sync::Arc;
use std::time::Duration;

use chrono::{Duration as ChronoDuration, Utc};
use serde_json::json;
use std::sync::LazyLock;
use tokio::sync::{Mutex, RwLock};

use crate::events::{record_filtering_event, Severity};
use crate::logger::{self, LogTag};
use crate::tokens::types::Token;

use super::engine::compute_snapshot;
use super::store_helpers::{
    apply_filters, build_stats, collect_entries, is_snapshot_stale, overlay_pool_price_data,
    sort_tokens,
};
use super::types::{
    BlacklistReasonInfo, FilteringQuery, FilteringQueryResult, FilteringSnapshot,
    FilteringStatsSnapshot, FilteringView, PassedToken, RejectedToken, SortDirection, TokenSortKey,
};

static GLOBAL_STORE: LazyLock<Arc<FilteringStore>> =
    LazyLock::new(|| Arc::new(FilteringStore::new()));

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

    /// Snapshot access for callers that need an answer: returns the cached snapshot
    /// immediately when one exists (refreshing in the background if stale), and otherwise
    /// WAITS for the first build.
    ///
    /// That wait is the right trade for a surface that is nothing but filtering results —
    /// the tokens tab has nothing to show without it. It is the wrong trade for a caller
    /// that only wants counts; use [`Self::snapshot_if_ready`] there.
    async fn ensure_snapshot(&self) -> Result<Arc<FilteringSnapshot>, String> {
        let stale_snapshot = self.snapshot.read().await.clone();

        // If we have any snapshot (even stale), return it immediately
        if let Some(existing) = stale_snapshot.as_ref() {
            if is_snapshot_stale(existing) {
                self.spawn_background_refresh();
            }

            return Ok(existing.clone());
        }

        // No snapshot exists - must wait for first refresh
        // But use a timeout to avoid blocking indefinitely
        match tokio::time::timeout(Duration::from_secs(30), self.try_refresh()).await {
            Ok(Ok(snapshot)) => Ok(snapshot),
            Ok(Err(err)) => Err(err),
            Err(_) => Err("Snapshot refresh timed out after 30 seconds".to_owned()),
        }
    }

    /// The snapshot only if one ALREADY exists — never builds one, never waits.
    ///
    /// Building the first snapshot loads every token in the database and runs the whole
    /// pipeline over it, which on a mature database is tens of seconds. `ensure_snapshot`
    /// blocks for up to 30s on that build, and the home dashboard's single fetch is what
    /// clears the first-paint skeleton — so routing a mere COUNT through it left a
    /// freshly-launched app sitting in its loading state for the entire timeout, every
    /// launch. A count that is briefly absent is worth far less than a dashboard that
    /// paints; the background build fills it in on the next poll.
    async fn snapshot_if_ready(&self) -> Option<Arc<FilteringSnapshot>> {
        let existing = self.snapshot.read().await.clone();

        match existing {
            Some(snapshot) => {
                if is_snapshot_stale(&snapshot) {
                    self.spawn_background_refresh();
                }
                Some(snapshot)
            }
            None => {
                // Nothing to serve yet. Kick the build off so the next caller has one,
                // and answer now rather than holding the caller for it.
                self.spawn_background_refresh();
                None
            }
        }
    }

    /// Start a refresh off to the side, unless one is already running.
    fn spawn_background_refresh(&self) {
        if self.refresh_in_progress.load(AtomicOrdering::Relaxed) {
            return;
        }

        let store = global_store();
        tokio::spawn(async move {
            let _ = store.try_refresh_background().await;
        });
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
                &format!("Background refresh failed: {err}"),
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
        // Arc<Token> derefs to Token, so we can get &Token from it
        let mut tokens: Vec<&Token> = entries
            .into_iter()
            .map(|entry| entry.token.as_ref())
            .collect();

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
        let _ohlcv_set: std::collections::HashSet<_> = ohlcv_mints.iter().cloned().collect();

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
                        &format!("Failed to get rejection stats from DB: {e}"),
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
            TokenSortKey::Symbol => Some("symbol".to_owned()),
            TokenSortKey::PriceSol => Some("price_sol".to_owned()),
            TokenSortKey::LiquidityUsd => Some("liquidity_usd".to_owned()),
            TokenSortKey::Volume24h => Some("volume_24h".to_owned()),
            TokenSortKey::Fdv => Some("fdv".to_owned()),
            TokenSortKey::MarketCap => Some("market_cap".to_owned()),
            TokenSortKey::PriceChangeH1 => Some("price_change_h1".to_owned()),
            TokenSortKey::PriceChangeH24 => Some("price_change_h24".to_owned()),
            TokenSortKey::RiskScore => Some("risk_score".to_owned()),
            TokenSortKey::MarketDataLastFetchedAt => Some("market_data_last_fetched_at".to_owned()),
            TokenSortKey::FirstDiscoveredAt => Some("first_discovered_at".to_owned()),
            TokenSortKey::MetadataLastFetchedAt => Some("metadata_last_fetched_at".to_owned()),
            TokenSortKey::BlockchainCreatedAt => Some("blockchain_created_at".to_owned()),
            TokenSortKey::PoolPriceLastCalculatedAt => {
                Some("pool_price_last_calculated_at".to_owned())
            }
            TokenSortKey::Mint => Some("mint".to_owned()),
            // Transaction sorts - mapped to SQL expressions in database.rs
            TokenSortKey::Txns5m => Some("txns_5m".to_owned()),
            TokenSortKey::Txns1h => Some("txns_1h".to_owned()),
            TokenSortKey::Txns6h => Some("txns_6h".to_owned()),
            TokenSortKey::Txns24h => Some("txns_24h".to_owned()),
        };

        let sort_direction = match query.sort_direction {
            SortDirection::Asc => Some("asc".to_owned()),
            SortDirection::Desc => Some("desc".to_owned()),
        };

        // Only load the tokens for THIS page with proper sorting
        let items =
            get_all_tokens_optional_market_async(query.page_size, offset, sort_by, sort_direction)
                .await
                .map_err(|e| format!("Failed to get tokens from database: {:?}", e))?;

        // Derived flags (pool price, open positions, ohlcv) come from the snapshot, but the
        // ROWS above came straight from the database — so this view has everything it needs
        // to render without one. Waiting here is what made "All tokens" unresponsive for up
        // to 30 seconds after launch: the page was already in hand and the request sat on a
        // snapshot build it only wanted decoration from. Flags light up on a later poll.
        let snapshot = self.snapshot_if_ready().await;

        // Build lookup sets for derived flags
        let mut priced_mints: Vec<String> = Vec::new();
        let mut open_position_mints: Vec<String> = Vec::new();
        let mut ohlcv_mints: Vec<String> = Vec::new();

        if let Some(snapshot) = snapshot.as_ref() {
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
        }

        // Count totals (approximations based on snapshot)
        let priced_total = priced_mints.len();
        let positions_total = open_position_mints.len();
        let blacklisted_total = items.iter().filter(|t| t.is_blacklisted).count();

        let mut blacklist_reasons: HashMap<String, Vec<BlacklistReasonInfo>> = HashMap::new();
        if let Some(snapshot) = snapshot.as_ref() {
            for token in &items {
                if let Some(sources) = snapshot.blacklist_reasons.get(token.mint.as_str()) {
                    blacklist_reasons.insert(token.mint.clone(), sources.clone());
                }
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
            timestamp: snapshot
                .as_ref()
                .map(|s| s.updated_at)
                .unwrap_or_else(Utc::now),
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
            TokenSortKey::Symbol => Some("symbol".to_owned()),
            TokenSortKey::RiskScore => Some("risk_score".to_owned()),
            TokenSortKey::MarketDataLastFetchedAt => Some("market_data_last_fetched_at".to_owned()),
            TokenSortKey::FirstDiscoveredAt => Some("first_discovered_at".to_owned()),
            TokenSortKey::MetadataLastFetchedAt => Some("metadata_last_fetched_at".to_owned()),
            TokenSortKey::BlockchainCreatedAt => Some("blockchain_created_at".to_owned()),
            TokenSortKey::PoolPriceLastCalculatedAt => {
                Some("pool_price_last_calculated_at".to_owned())
            }
            TokenSortKey::Mint => Some("mint".to_owned()),
            _ => Some("metadata_last_fetched_at".to_owned()),
        };
        let sort_direction = match query.sort_direction {
            SortDirection::Asc => Some("asc".to_owned()),
            SortDirection::Desc => Some("desc".to_owned()),
        };

        let items = get_tokens_no_market_async(query.page_size, offset, sort_by, sort_direction)
            .await
            .map_err(|e| format!("Failed to load no-market tokens: {:?}", e))?;

        // Snapshot for timestamp and derived counts — read only if one already exists, for
        // the same reason as the All view: the rows are database-backed and the snapshot
        // contributes decoration, never the page itself.
        let snapshot = self.snapshot_if_ready().await;

        let priced_total = 0; // by definition of this view (no market), keep 0 to avoid confusion
        let positions_total = match snapshot.as_ref() {
            Some(snapshot) => items
                .iter()
                .filter(|t| {
                    snapshot
                        .tokens
                        .get(&t.mint)
                        .map(|e| e.has_open_position)
                        .unwrap_or_default()
                })
                .count(),
            None => 0,
        };
        let blacklisted_total = items.iter().filter(|t| t.is_blacklisted).count();

        let mut blacklist_reasons: HashMap<String, Vec<BlacklistReasonInfo>> = HashMap::new();
        if let Some(snapshot) = snapshot.as_ref() {
            for token in &items {
                if let Some(sources) = snapshot.blacklist_reasons.get(token.mint.as_str()) {
                    blacklist_reasons.insert(token.mint.clone(), sources.clone());
                }
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
            timestamp: snapshot
                .as_ref()
                .map(|s| s.updated_at)
                .unwrap_or_else(Utc::now),
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
        Ok(build_stats(snapshot.as_ref()).await)
    }

    pub async fn stats_if_ready(&self) -> Option<FilteringStatsSnapshot> {
        let snapshot = self.snapshot_if_ready().await?;
        Some(build_stats(snapshot.as_ref()).await)
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

/// Get the global filtering store singleton.
pub fn global_store() -> Arc<FilteringStore> {
    GLOBAL_STORE.clone()
}

/// Refresh the filtering snapshot with current token data.
pub async fn refresh_snapshot() -> Result<(), String> {
    global_store().refresh().await
}

/// Get the list of mint addresses that passed all filters.
pub async fn get_filtered_mints() -> Result<Vec<String>, String> {
    global_store().get_filtered_mints().await
}

/// Get detailed info for tokens that passed all filters.
pub async fn get_passed_tokens() -> Result<Vec<PassedToken>, String> {
    global_store().get_passed_tokens().await
}

/// Get detailed info for tokens that were rejected by filters.
pub async fn get_rejected_tokens() -> Result<Vec<RejectedToken>, String> {
    global_store().get_rejected_tokens().await
}

/// Execute a filtering query with custom parameters.
pub async fn execute_query(query: FilteringQuery) -> Result<FilteringQueryResult, String> {
    global_store().execute_query(query).await
}

/// Get aggregated filtering statistics.
pub async fn get_stats() -> Result<FilteringStatsSnapshot, String> {
    global_store().get_stats().await
}

/// Aggregated filtering statistics, but only if a snapshot already exists.
pub async fn stats_if_ready() -> Option<FilteringStatsSnapshot> {
    global_store().stats_if_ready().await
}
