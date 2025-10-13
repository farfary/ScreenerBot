//! Centralized in-memory token store for ScreenerBot tokens module.
//!
//! This module consolidates all runtime token data into a single, queryable
//! cache that will eventually replace the ad-hoc caches (summary cache,
//! realtime broadcaster, etc.). It exposes a rich token snapshot structure,
//! advanced filtering and sorting helpers, and a background service that keeps
//! the store synchronized with the database and upstream monitors.

use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::RwLock;

use chrono::Utc;
use dashmap::DashMap;
use once_cell::sync::{Lazy, OnceCell};
use tokio::sync::broadcast;

use crate::logger::{log, LogTag};
use crate::pools;
use crate::pools::types::PriceResult;
use crate::positions;
use crate::tokens::blacklist;
use crate::tokens::database::TokenDatabase;
use crate::tokens::decimals;
use crate::tokens::security::RiskLevel;
use crate::tokens::summary::TokenSummaryContext;
use crate::tokens::types::{Token, TokenInfo};

/// Broadcast channel capacity for token updates.
const TOKEN_STORE_EVENT_CHANNEL_CAPACITY: usize = 2048;

/// Global token store instance accessible across the crate.
static TOKEN_STORE: Lazy<TokenStore> = Lazy::new(TokenStore::new);

/// Obtain a reference to the global [`TokenStore`].
pub fn get_global_token_store() -> &'static TokenStore {
    &TOKEN_STORE
}

/// Events emitted by the [`TokenStore`] whenever data changes.
#[derive(Debug, Clone)]
pub enum TokenStoreEvent {
    /// Token was inserted or updated.
    Upserted {
        snapshot: TokenSnapshot,
        inserted: bool,
        source: TokenUpdateSource,
    },
    /// Token was removed.
    Removed {
        mint: String,
        previous: Option<TokenSnapshot>,
        source: TokenUpdateSource,
    },
    /// Bulk refresh (e.g. full database snapshot) completed.
    BulkRefresh {
        stats: TokenStoreRefreshStats,
        source: TokenUpdateSource,
    },
}

/// Origin of a token update applied to the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenUpdateSource {
    Database,
    Monitor,
    Api,
    Manual,
}

/// Aggregated metadata describing a cached token entry.
#[derive(Debug, Clone)]
pub struct TokenStoreMeta {
    pub source: TokenUpdateSource,
    pub version: u64,
    pub inserted_at_unix: i64,
    pub updated_at_unix: i64,
}

impl TokenStoreMeta {
    fn new(source: TokenUpdateSource, version: u64, timestamp_unix: i64) -> Self {
        Self {
            source,
            version,
            inserted_at_unix: timestamp_unix,
            updated_at_unix: timestamp_unix,
        }
    }
}

/// Consolidated view of price-related information for a token.
#[derive(Debug, Clone, Default)]
pub struct TokenPriceState {
    pub last_result: Option<PriceResult>,
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub price_native: Option<f64>,
    pub confidence: Option<f32>,
    pub slot: Option<u64>,
    pub price_updated_at_unix: Option<i64>,
}

/// Consolidated security information for a token.
#[derive(Debug, Clone, Default)]
pub struct TokenSecurityState {
    pub score: Option<i32>,
    pub rugged: Option<bool>,
    pub total_holders: Option<i32>,
    pub risk_level: Option<RiskLevel>,
}

/// Activity flags derived from downstream systems (positions, OHLCV, etc.).
#[derive(Debug, Clone, Default)]
pub struct TokenActivityState {
    pub has_open_position: bool,
    pub open_position_count: usize,
    pub has_ohlcv: bool,
}

/// Token status flags used by trading and filtering logic.
#[derive(Debug, Clone, Default)]
pub struct TokenStatusFlags {
    pub blacklisted: bool,
    pub is_system_or_stable: bool,
    pub trade_eligible: bool,
}

/// Full runtime representation of a token.
#[derive(Debug, Clone)]
pub struct TokenSnapshot {
    pub data: Token,
    pub price: TokenPriceState,
    pub security: TokenSecurityState,
    pub activity: TokenActivityState,
    pub status: TokenStatusFlags,
    pub meta: TokenStoreMeta,
}

impl TokenSnapshot {
    /// Convenience getter for mint.
    #[inline]
    pub fn mint(&self) -> &str {
        &self.data.mint
    }

    /// Convenience getter for liquidity in USD.
    #[inline]
    pub fn liquidity_usd(&self) -> Option<f64> {
        self.data.liquidity.as_ref().and_then(|l| l.usd)
    }

    /// Convenience getter for 24h volume.
    #[inline]
    pub fn volume_24h(&self) -> Option<f64> {
        self.data.volume.as_ref().and_then(|v| v.h24)
    }

    /// Convenience getter for 1h price change.
    #[inline]
    pub fn price_change_h1(&self) -> Option<f64> {
        self.data.price_change.as_ref().and_then(|p| p.h1)
    }

    /// Convenience getter for 24h price change.
    #[inline]
    pub fn price_change_h24(&self) -> Option<f64> {
        self.data.price_change.as_ref().and_then(|p| p.h24)
    }
}

/// Query filter used to select tokens from the store.
#[derive(Debug, Clone)]
pub struct TokenFilter {
    pub mints: HashSet<String>,
    pub symbols: HashSet<String>,
    pub dex_ids: HashSet<String>,
    pub tags: HashSet<String>,
    pub search: Option<String>,
    pub include_blacklisted: bool,
    pub include_system_tokens: bool,
    pub require_open_position: bool,
    pub require_ohlcv: bool,
    pub min_liquidity_usd: Option<f64>,
    pub min_volume_24h: Option<f64>,
    pub min_security_score: Option<i32>,
    pub risk_levels: Vec<RiskLevel>,
    pub min_confidence: Option<f32>,
    pub min_price_sol: Option<f64>,
    pub max_price_sol: Option<f64>,
    pub updated_within_secs: Option<u64>,
}

impl Default for TokenFilter {
    fn default() -> Self {
        Self {
            mints: HashSet::new(),
            symbols: HashSet::new(),
            dex_ids: HashSet::new(),
            tags: HashSet::new(),
            search: None,
            include_blacklisted: false,
            include_system_tokens: false,
            require_open_position: false,
            require_ohlcv: false,
            min_liquidity_usd: None,
            min_volume_24h: None,
            min_security_score: None,
            risk_levels: Vec::new(),
            min_confidence: None,
            min_price_sol: None,
            max_price_sol: None,
            updated_within_secs: None,
        }
    }
}

/// Sorting field options used by [`TokenSortKey`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSortField {
    LiquidityUsd,
    Volume24h,
    PriceSol,
    PriceUsd,
    PriceChangeH1,
    PriceChangeH24,
    SecurityScore,
    LastUpdated,
    MarketCap,
    Fdv,
    Symbol,
    Confidence,
}

/// Sorting key (field + direction).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenSortKey {
    pub field: TokenSortField,
    pub descending: bool,
}

/// Sorting configuration for store queries.
#[derive(Debug, Clone)]
pub struct TokenSort {
    pub keys: Vec<TokenSortKey>,
}

impl TokenSort {
    pub fn by(field: TokenSortField, descending: bool) -> Self {
        Self {
            keys: vec![TokenSortKey { field, descending }],
        }
    }

    pub fn then(mut self, field: TokenSortField, descending: bool) -> Self {
        self.keys.push(TokenSortKey { field, descending });
        self
    }
}

impl Default for TokenSort {
    fn default() -> Self {
        Self::by(TokenSortField::LiquidityUsd, true)
    }
}

/// Pagination options for query results.
#[derive(Debug, Clone, Copy, Default)]
pub struct PaginationOptions {
    pub offset: usize,
    pub limit: usize,
}

/// Query result containing filtered and sorted tokens.
#[derive(Debug, Clone)]
pub struct TokenQueryResult {
    pub total_matching: usize,
    pub items: Vec<TokenSnapshot>,
    pub pagination: Option<PaginationOptions>,
}

impl Default for TokenQueryResult {
    fn default() -> Self {
        Self {
            total_matching: 0,
            items: Vec::new(),
            pagination: None,
        }
    }
}

/// Atomic metrics recorded by the [`TokenStore`].
#[derive(Debug, Default)]
struct TokenStoreMetrics {
    total_inserts: AtomicU64,
    total_updates: AtomicU64,
    total_removals: AtomicU64,
    last_full_refresh_unix: AtomicU64,
    last_delta_refresh_unix: AtomicU64,
}

impl TokenStoreMetrics {
    #[inline]
    fn record_insert(&self) {
        self.total_inserts.fetch_add(1, AtomicOrdering::Relaxed);
    }

    #[inline]
    fn record_update(&self) {
        self.total_updates.fetch_add(1, AtomicOrdering::Relaxed);
    }

    #[inline]
    fn record_remove(&self) {
        self.total_removals.fetch_add(1, AtomicOrdering::Relaxed);
    }

    fn record_full_refresh(&self, timestamp: u64) {
        self.last_full_refresh_unix
            .store(timestamp, AtomicOrdering::Relaxed);
    }

    fn record_delta_refresh(&self, timestamp: u64) {
        self.last_delta_refresh_unix
            .store(timestamp, AtomicOrdering::Relaxed);
    }

    fn snapshot(&self) -> TokenStoreMetricsSnapshot {
        TokenStoreMetricsSnapshot {
            total_inserts: self.total_inserts.load(AtomicOrdering::Relaxed),
            total_updates: self.total_updates.load(AtomicOrdering::Relaxed),
            total_removals: self.total_removals.load(AtomicOrdering::Relaxed),
            last_full_refresh_unix: self.last_full_refresh_unix.load(AtomicOrdering::Relaxed),
            last_delta_refresh_unix: self.last_delta_refresh_unix.load(AtomicOrdering::Relaxed),
        }
    }
}

/// Snapshot of token store metrics for reporting.
#[derive(Debug, Clone, Default)]
pub struct TokenStoreMetricsSnapshot {
    pub total_inserts: u64,
    pub total_updates: u64,
    pub total_removals: u64,
    pub last_full_refresh_unix: u64,
    pub last_delta_refresh_unix: u64,
}

#[derive(Debug, Clone)]
pub struct TokenStoreRefreshStats {
    pub source: TokenUpdateSource,
    pub total_processed: usize,
    pub inserted: usize,
    pub updated: usize,
    pub removed: usize,
}

impl TokenStoreRefreshStats {
    fn new(source: TokenUpdateSource) -> Self {
        Self {
            source,
            total_processed: 0,
            inserted: 0,
            updated: 0,
            removed: 0,
        }
    }
}

struct TokenIndexes {
    symbols: HashMap<String, HashSet<String>>, // lowercase symbol -> mints
    tags: HashMap<String, HashSet<String>>,    // lowercase tag -> mints
    dex_ids: HashMap<String, HashSet<String>>, // lowercase dex_id -> mints
}

impl Default for TokenIndexes {
    fn default() -> Self {
        Self {
            symbols: HashMap::new(),
            tags: HashMap::new(),
            dex_ids: HashMap::new(),
        }
    }
}

impl TokenIndexes {
    fn add(&mut self, snapshot: &TokenSnapshot) {
        let mint = snapshot.data.mint.clone();
        if !snapshot.data.symbol.is_empty() {
            let key = snapshot.data.symbol.to_ascii_lowercase();
            self.symbols.entry(key).or_default().insert(mint.clone());
        }

        if let Some(ref dex_id) = snapshot.data.dex_id {
            if !dex_id.is_empty() {
                let key = dex_id.to_ascii_lowercase();
                self.dex_ids.entry(key).or_default().insert(mint.clone());
            }
        }

        for label in &snapshot.data.labels {
            let key = label.to_ascii_lowercase();
            self.tags.entry(key).or_default().insert(mint.clone());
        }

        // Token.info is TokenInfoCompat which doesn't have extractable tags field
        // Tags are now directly on Token.labels and Token.tags fields
    }

    fn remove(&mut self, snapshot: &TokenSnapshot) {
        let mint = &snapshot.data.mint;

        if !snapshot.data.symbol.is_empty() {
            let key = snapshot.data.symbol.to_ascii_lowercase();
            remove_from_index(&mut self.symbols, &key, mint);
        }

        if let Some(ref dex_id) = snapshot.data.dex_id {
            if !dex_id.is_empty() {
                let key = dex_id.to_ascii_lowercase();
                remove_from_index(&mut self.dex_ids, &key, mint);
            }
        }

        for label in &snapshot.data.labels {
            let key = label.to_ascii_lowercase();
            remove_from_index(&mut self.tags, &key, mint);
        }
    }

    fn lookup_symbols(&self, symbols: &HashSet<String>) -> HashSet<String> {
        lookup_index(&self.symbols, symbols)
    }

    fn lookup_dex_ids(&self, dex_ids: &HashSet<String>) -> HashSet<String> {
        lookup_index(&self.dex_ids, dex_ids)
    }

    fn lookup_tags(&self, tags: &HashSet<String>) -> HashSet<String> {
        lookup_index(&self.tags, tags)
    }
}

// TokenInfo extraction no longer needed - Token has tags and labels fields directly

fn remove_from_index(index: &mut HashMap<String, HashSet<String>>, key: &str, mint: &str) {
    if let Some(set) = index.get_mut(key) {
        set.remove(mint);
        if set.is_empty() {
            index.remove(key);
        }
    }
}

fn lookup_index(
    index: &HashMap<String, HashSet<String>>,
    keys: &HashSet<String>,
) -> HashSet<String> {
    if keys.is_empty() {
        return HashSet::new();
    }

    let mut out: HashSet<String> = HashSet::new();
    for key in keys {
        let lookup_key = key.to_ascii_lowercase();
        if let Some(set) = index.get(&lookup_key) {
            if out.is_empty() {
                out.extend(set.iter().cloned());
            } else {
                for mint in set {
                    out.insert(mint.clone());
                }
            }
        }
    }
    out
}

/// Central token store with indexes and broadcast notifications.
pub struct TokenStore {
    tokens: DashMap<String, TokenSnapshot>,
    indexes: RwLock<TokenIndexes>,
    metrics: TokenStoreMetrics,
    broadcaster: broadcast::Sender<TokenStoreEvent>,
    version_counter: AtomicU64,
    database: OnceCell<TokenDatabase>,
}

impl TokenStore {
    fn new() -> Self {
        let (tx, _rx) = broadcast::channel(TOKEN_STORE_EVENT_CHANNEL_CAPACITY);
        Self {
            tokens: DashMap::new(),
            indexes: RwLock::new(TokenIndexes::default()),
            metrics: TokenStoreMetrics::default(),
            broadcaster: tx,
            version_counter: AtomicU64::new(0),
            database: OnceCell::new(),
        }
    }

    /// Configure the store with a shared database handle.
    pub fn configure_database(&self, db: TokenDatabase) {
        if self.database.set(db).is_err() {
            log(
                LogTag::Cache,
                "WARN",
                "Token store database already configured; keeping existing handle",
            );
        }
    }

    /// Returns true if a database handle has been configured.
    pub fn is_database_configured(&self) -> bool {
        self.database.get().is_some()
    }

    fn database(&self) -> Option<TokenDatabase> {
        self.database.get().cloned()
    }

    #[inline]
    fn now_unix() -> i64 {
        Utc::now().timestamp()
    }

    #[inline]
    fn next_version(&self) -> u64 {
        self.version_counter.fetch_add(1, AtomicOrdering::Relaxed) + 1
    }

    /// Total number of tokens in the cache.
    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    /// Retrieve metrics snapshot for observability surfaces.
    pub fn metrics_snapshot(&self) -> TokenStoreMetricsSnapshot {
        let mut snapshot = self.metrics.snapshot();
        if snapshot.last_full_refresh_unix == 0 {
            snapshot.last_full_refresh_unix = self
                .metrics
                .last_full_refresh_unix
                .load(AtomicOrdering::Relaxed);
        }
        if snapshot.last_delta_refresh_unix == 0 {
            snapshot.last_delta_refresh_unix = self
                .metrics
                .last_delta_refresh_unix
                .load(AtomicOrdering::Relaxed);
        }
        snapshot
    }

    /// Subscribe to token store events.
    pub fn subscribe(&self) -> broadcast::Receiver<TokenStoreEvent> {
        self.broadcaster.subscribe()
    }

    fn emit_event(&self, event: TokenStoreEvent) {
        // Only send events if there are active subscribers
        if self.broadcaster.receiver_count() > 0 {
            // Silently ignore send errors - lagged receivers are acceptable
            let _ = self.broadcaster.send(event);
        }
    }

    /// Get a token snapshot by mint.
    pub fn get(&self, mint: &str) -> Option<TokenSnapshot> {
        self.tokens.get(mint).map(|entry| entry.value().clone())
    }

    /// Get multiple snapshots by mint.
    pub fn get_many(&self, mints: &[String]) -> Vec<TokenSnapshot> {
        mints.iter().filter_map(|mint| self.get(mint)).collect()
    }

    /// Returns all snapshots (cloned).
    pub fn all(&self) -> Vec<TokenSnapshot> {
        self.tokens
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    /// Primary ingestion path for tokens.
    pub async fn ingest_tokens(
        &self,
        tokens: Vec<Token>,
        source: TokenUpdateSource,
    ) -> Result<TokenStoreRefreshStats, String> {
        if tokens.is_empty() {
            return Ok(TokenStoreRefreshStats::new(source));
        }

        if let Some(db) = self.database() {
            let mut new_tokens = Vec::new();
            let mut existing_tokens = Vec::new();

            for token in &tokens {
                if self.tokens.contains_key(&token.mint) {
                    existing_tokens.push(token.clone());
                } else {
                    new_tokens.push(token.clone());
                }
            }

            if !new_tokens.is_empty() {
                if let Err(err) = db.add_tokens(&new_tokens).await {
                    let message =
                        format!("Failed to persist {} new tokens: {}", new_tokens.len(), err);
                    log(LogTag::Tokens, "DB_WRITE_FAILED", &message);
                    return Err(message);
                }
            }

            if !existing_tokens.is_empty() {
                if let Err(err) = db.update_tokens(&existing_tokens).await {
                    let message =
                        format!("Failed to update {} tokens: {}", existing_tokens.len(), err);
                    log(LogTag::Tokens, "DB_WRITE_FAILED", &message);
                    return Err(message);
                }
            }
        }

        self.apply_snapshot(tokens, source, false).await
    }

    /// Refresh the store from the database.
    pub async fn refresh_all_from_database(&self) -> Result<TokenStoreRefreshStats, String> {
        let db = self
            .database()
            .ok_or_else(|| "Token store database not configured".to_string())?;
        let tokens = db.get_all_tokens().await?;
        let stats = self
            .apply_snapshot(tokens, TokenUpdateSource::Database, true)
            .await?;
        self.metrics.record_full_refresh(Self::now_unix() as u64);
        Ok(stats)
    }

    async fn apply_snapshot(
        &self,
        tokens: Vec<Token>,
        source: TokenUpdateSource,
        remove_missing: bool,
    ) -> Result<TokenStoreRefreshStats, String> {
        if tokens.is_empty() {
            return Ok(TokenStoreRefreshStats::new(source));
        }

        let mints: Vec<String> = tokens.iter().map(|t| t.mint.clone()).collect();
        let context = TokenEnrichmentContext::build(&mints).await;
        let mut stats = TokenStoreRefreshStats::new(source);

        for token in tokens {
            let snapshot = self.build_snapshot(token.clone(), &context, source);
            let inserted = self.upsert_snapshot(snapshot, source);
            stats.total_processed += 1;
            if inserted {
                stats.inserted += 1;
            } else {
                stats.updated += 1;
            }
        }

        if remove_missing {
            let fresh: HashSet<String> = mints.into_iter().collect();
            let mut to_remove = Vec::new();
            for entry in self.tokens.iter() {
                if !fresh.contains(entry.key()) {
                    to_remove.push(entry.key().clone());
                }
            }
            for mint in to_remove {
                if self.remove_internal(&mint, source).is_some() {
                    stats.removed += 1;
                }
            }
        }

        self.metrics.record_delta_refresh(Self::now_unix() as u64);
        self.emit_event(TokenStoreEvent::BulkRefresh {
            stats: stats.clone(),
            source,
        });

        Ok(stats)
    }

    fn upsert_snapshot(&self, mut snapshot: TokenSnapshot, source: TokenUpdateSource) -> bool {
        let now = Self::now_unix();
        snapshot.meta.source = source;
        snapshot.meta.version = self.next_version();
        snapshot.meta.updated_at_unix = now;

        let mint = snapshot.data.mint.clone();
        let mut inserted = false;
        let mut previous: Option<TokenSnapshot> = None;

        if let Some(mut entry) = self.tokens.get_mut(&mint) {
            snapshot.meta.inserted_at_unix = entry.meta.inserted_at_unix;
            previous = Some(entry.clone());
            *entry = snapshot.clone();
            self.metrics.record_update();
        } else {
            snapshot.meta.inserted_at_unix = now;
            self.tokens.insert(mint.clone(), snapshot.clone());
            self.metrics.record_insert();
            inserted = true;
        }

        {
            let mut indexes = self.indexes.write().expect("Token indexes poisoned");
            if let Some(old_snapshot) = previous.as_ref() {
                indexes.remove(old_snapshot);
            }
            indexes.add(&snapshot);
        }

        self.emit_event(TokenStoreEvent::Upserted {
            snapshot,
            inserted,
            source,
        });

        inserted
    }

    fn remove_internal(&self, mint: &str, source: TokenUpdateSource) -> Option<TokenSnapshot> {
        if let Some((_, snapshot)) = self.tokens.remove(mint) {
            {
                let mut indexes = self.indexes.write().expect("Token indexes poisoned");
                indexes.remove(&snapshot);
            }
            self.metrics.record_remove();
            self.emit_event(TokenStoreEvent::Removed {
                mint: mint.to_string(),
                previous: Some(snapshot.clone()),
                source,
            });
            Some(snapshot)
        } else {
            None
        }
    }

    /// Remove a token from the store and database atomically.
    pub async fn remove_token(
        &self,
        mint: &str,
        source: TokenUpdateSource,
    ) -> Result<Option<TokenSnapshot>, String> {
        let removed = self.remove_internal(mint, source);

        if removed.is_none() {
            return Ok(None);
        }

        if let Some(db) = self.database() {
            if let Err(err) = db.delete_tokens(&[mint.to_string()]).await {
                if let Some(snapshot) = removed.clone() {
                    self.upsert_snapshot(snapshot, source);
                }
                let message = format!("Failed to delete token {} from database: {}", mint, err);
                log(LogTag::Tokens, "DB_DELETE_FAILED", &message);
                return Err(message);
            }
        }

        Ok(removed)
    }

    fn build_snapshot(
        &self,
        mut token: Token,
        context: &TokenEnrichmentContext,
        source: TokenUpdateSource,
    ) -> TokenSnapshot {
        let mint = token.mint.clone();

        // Enrich token with decimals if not already set
        if token.decimals.is_none() {
            token.decimals = context.decimals.get(&mint).copied();
        }

        let price_state = TokenPriceState::from_token(&token, context.price.get(&mint));
        let security_state = context.security_state(&mint);
        let activity_state = context.activity_state(&mint);
        let mut status_flags = TokenStatusFlags {
            blacklisted: context.is_blacklisted(&mint),
            is_system_or_stable: blacklist::is_system_or_stable_token(&mint),
            trade_eligible: false,
        };
        status_flags.trade_eligible =
            !status_flags.blacklisted && !status_flags.is_system_or_stable;

        let meta = TokenStoreMeta::new(source, self.next_version(), Self::now_unix());

        TokenSnapshot {
            data: token,
            price: price_state,
            security: security_state,
            activity: activity_state,
            status: status_flags,
            meta,
        }
    }

    /// Execute a filtered query with optional sorting and pagination.
    pub fn query(
        &self,
        filter: &TokenFilter,
        sort: &TokenSort,
        pagination: Option<PaginationOptions>,
    ) -> TokenQueryResult {
        let mut candidates = self.collect_candidates(filter);
        let total_matching = candidates.len();

        if total_matching > 1 {
            candidates.sort_by(|a, b| compare_snapshots(a, b, sort));
        }

        let items = if let Some(pager) = pagination {
            let start = pager.offset.min(candidates.len());
            let end = (start + pager.limit).min(candidates.len());
            candidates[start..end].to_vec()
        } else {
            candidates
        };

        TokenQueryResult {
            total_matching,
            items,
            pagination,
        }
    }

    fn collect_candidates(&self, filter: &TokenFilter) -> Vec<TokenSnapshot> {
        let mut candidate_mints: Option<HashSet<String>> = None;
        let indexes = self.indexes.read().expect("Token indexes poisoned");

        if !filter.symbols.is_empty() {
            let set = indexes.lookup_symbols(&filter.symbols);
            candidate_mints = Some(set);
        }

        if !filter.dex_ids.is_empty() {
            let set = indexes.lookup_dex_ids(&filter.dex_ids);
            candidate_mints = intersect_candidates(candidate_mints, set);
        }

        if !filter.tags.is_empty() {
            let set = indexes.lookup_tags(&filter.tags);
            candidate_mints = intersect_candidates(candidate_mints, set);
        }

        if !filter.mints.is_empty() {
            candidate_mints = intersect_candidates(candidate_mints, filter.mints.clone());
        }
        drop(indexes);

        let mut snapshots: Vec<TokenSnapshot> = if let Some(mints) = candidate_mints {
            self.get_many(&mints.into_iter().collect::<Vec<_>>())
        } else {
            self.all()
        };

        let needle = filter.search.as_ref().map(|s| s.to_ascii_lowercase());
        let now = Self::now_unix();

        snapshots.retain(|snapshot| matches_filter(snapshot, filter, needle.as_deref(), now));
        snapshots
    }
}

fn intersect_candidates(
    current: Option<HashSet<String>>,
    candidates: HashSet<String>,
) -> Option<HashSet<String>> {
    if let Some(mut existing) = current {
        existing.retain(|mint| candidates.contains(mint));
        Some(existing)
    } else {
        Some(candidates)
    }
}

fn matches_filter(
    snapshot: &TokenSnapshot,
    filter: &TokenFilter,
    search_lower: Option<&str>,
    now_unix: i64,
) -> bool {
    if !filter.include_blacklisted && snapshot.status.blacklisted {
        return false;
    }

    if !filter.include_system_tokens && snapshot.status.is_system_or_stable {
        return false;
    }

    if filter.require_open_position && !snapshot.activity.has_open_position {
        return false;
    }

    if filter.require_ohlcv && !snapshot.activity.has_ohlcv {
        return false;
    }

    if let Some(min_lq) = filter.min_liquidity_usd {
        if snapshot.liquidity_usd().map_or(true, |v| v < min_lq) {
            return false;
        }
    }

    if let Some(min_vol) = filter.min_volume_24h {
        if snapshot.volume_24h().map_or(true, |v| v < min_vol) {
            return false;
        }
    }

    if let Some(min_score) = filter.min_security_score {
        match snapshot.security.score {
            Some(score) if score >= min_score => {}
            _ => return false,
        }
    }

    if !filter.risk_levels.is_empty() {
        if let Some(risk) = snapshot.security.risk_level.clone() {
            if !filter.risk_levels.contains(&risk) {
                return false;
            }
        } else {
            return false;
        }
    }

    if let Some(min_confidence) = filter.min_confidence {
        if snapshot
            .price
            .confidence
            .map_or(true, |conf| conf < min_confidence)
        {
            return false;
        }
    }

    if let Some(min_price) = filter.min_price_sol {
        if snapshot
            .price
            .price_sol
            .map_or(true, |price| price < min_price)
        {
            return false;
        }
    }

    if let Some(max_price) = filter.max_price_sol {
        if snapshot
            .price
            .price_sol
            .map_or(true, |price| price > max_price)
        {
            return false;
        }
    }

    if let Some(updated_within) = filter.updated_within_secs {
        if snapshot
            .meta
            .updated_at_unix
            .saturating_sub(now_unix)
            .unsigned_abs()
            > updated_within
        {
            return false;
        }
    }

    if let Some(search) = search_lower {
        if !matches_search(snapshot, search) {
            return false;
        }
    }

    if !filter.dex_ids.is_empty() {
        if let Some(ref dex_id) = snapshot.data.dex_id {
            let needle = dex_id.to_ascii_lowercase();
            if !filter.dex_ids.contains(&needle) {
                return false;
            }
        } else {
            return false;
        }
    }

    if !filter.tags.is_empty() {
        let mut tag_pool: HashSet<String> = HashSet::new();
        for label in &snapshot.data.labels {
            tag_pool.insert(label.to_ascii_lowercase());
        }
        for tag in &snapshot.data.tags {
            tag_pool.insert(tag.to_ascii_lowercase());
        }
        if !filter
            .tags
            .iter()
            .all(|tag| tag_pool.contains(&tag.to_ascii_lowercase()))
        {
            return false;
        }
    }

    true
}

fn matches_search(snapshot: &TokenSnapshot, needle: &str) -> bool {
    if snapshot.data.mint.to_ascii_lowercase().contains(needle) {
        return true;
    }
    if snapshot.data.symbol.to_ascii_lowercase().contains(needle) {
        return true;
    }
    if snapshot.data.name.to_ascii_lowercase().contains(needle) {
        return true;
    }
    if let Some(ref pair_address) = snapshot.data.pair_address {
        if pair_address.to_ascii_lowercase().contains(needle) {
            return true;
        }
    }
    if snapshot
        .data
        .labels
        .iter()
        .any(|label| label.to_ascii_lowercase().contains(needle))
    {
        return true;
    }
    // Token name/symbol already checked above via snapshot.data.name and snapshot.data.symbol
    false
}

fn compare_snapshots(a: &TokenSnapshot, b: &TokenSnapshot, sort: &TokenSort) -> Ordering {
    for key in &sort.keys {
        let ordering = match key.field {
            TokenSortField::LiquidityUsd => {
                compare_option_f64(a.liquidity_usd(), b.liquidity_usd())
            }
            TokenSortField::Volume24h => compare_option_f64(a.volume_24h(), b.volume_24h()),
            TokenSortField::PriceSol => compare_option_f64(a.price.price_sol, b.price.price_sol),
            TokenSortField::PriceUsd => compare_option_f64(a.price.price_usd, b.price.price_usd),
            TokenSortField::PriceChangeH1 => {
                compare_option_f64(a.price_change_h1(), b.price_change_h1())
            }
            TokenSortField::PriceChangeH24 => {
                compare_option_f64(a.price_change_h24(), b.price_change_h24())
            }
            TokenSortField::SecurityScore => compare_option_i32(a.security.score, b.security.score),
            TokenSortField::LastUpdated => a.meta.updated_at_unix.cmp(&b.meta.updated_at_unix),
            TokenSortField::MarketCap => compare_option_f64(a.data.market_cap, b.data.market_cap),
            TokenSortField::Fdv => compare_option_f64(a.data.fdv, b.data.fdv),
            TokenSortField::Symbol => a.data.symbol.cmp(&b.data.symbol),
            TokenSortField::Confidence => {
                compare_option_f32(a.price.confidence, b.price.confidence)
            }
        };

        if ordering != Ordering::Equal {
            return if key.descending {
                ordering.reverse()
            } else {
                ordering
            };
        }
    }

    Ordering::Equal
}

fn compare_option_f64(a: Option<f64>, b: Option<f64>) -> Ordering {
    match (a, b) {
        (Some(a_val), Some(b_val)) => a_val.partial_cmp(&b_val).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

fn compare_option_f32(a: Option<f32>, b: Option<f32>) -> Ordering {
    match (a, b) {
        (Some(a_val), Some(b_val)) => a_val.partial_cmp(&b_val).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

fn compare_option_i32(a: Option<i32>, b: Option<i32>) -> Ordering {
    match (a, b) {
        (Some(a_val), Some(b_val)) => a_val.cmp(&b_val),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

struct TokenEnrichmentContext {
    price: HashMap<String, PriceResult>,
    decimals: HashMap<String, u8>,
    summary: TokenSummaryContext,
    open_position_counts: HashMap<String, usize>,
    blacklisted: HashSet<String>,
}

impl TokenEnrichmentContext {
    async fn build(mints: &[String]) -> Self {
        let summary = TokenSummaryContext::build(mints).await;
        let open_positions = positions::get_open_positions().await;
        let mut open_position_counts: HashMap<String, usize> = HashMap::new();
        for position in open_positions {
            *open_position_counts
                .entry(position.mint.clone())
                .or_insert(0) += 1;
        }

        let mut price = HashMap::new();
        for mint in mints {
            if let Some(result) = pools::get_pool_price(mint) {
                price.insert(mint.clone(), result);
            }
        }

        let mut decimals_map = HashMap::new();
        for mint in mints {
            if let Some(value) = decimals::get_cached_decimals(mint) {
                decimals_map.insert(mint.clone(), value);
            }
        }

        let mut blacklisted = HashSet::new();
        for mint in mints {
            if summary.is_blacklisted(mint) {
                blacklisted.insert(mint.clone());
            }
        }

        Self {
            price,
            decimals: decimals_map,
            summary,
            open_position_counts,
            blacklisted,
        }
    }

    fn security_state(&self, mint: &str) -> TokenSecurityState {
        if let Some(snapshot) = self.summary.security_snapshot(mint) {
            let risk_level = Some(risk_level_from_score(snapshot.score));
            TokenSecurityState {
                score: Some(snapshot.score),
                rugged: Some(snapshot.rugged),
                total_holders: snapshot.total_holders,
                risk_level,
            }
        } else {
            TokenSecurityState::default()
        }
    }

    fn activity_state(&self, mint: &str) -> TokenActivityState {
        TokenActivityState {
            has_open_position: self.summary.has_open_position(mint),
            open_position_count: self.open_position_counts.get(mint).copied().unwrap_or(0),
            has_ohlcv: self.summary.has_ohlcv(mint),
        }
    }

    fn is_blacklisted(&self, mint: &str) -> bool {
        self.blacklisted.contains(mint)
    }
}

fn risk_level_from_score(score: i32) -> RiskLevel {
    if score >= 70 {
        RiskLevel::Safe
    } else if score >= 40 {
        RiskLevel::Warning
    } else {
        RiskLevel::Danger
    }
}

impl TokenPriceState {
    fn from_token(token: &Token, price_result: Option<&PriceResult>) -> Self {
        if let Some(result) = price_result {
            let price_updated_at_unix = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|dur| dur.as_secs() as i64)
                .ok();
            Self {
                last_result: Some(result.clone()),
                price_sol: Some(result.price_sol),
                price_usd: Some(result.price_usd),
                price_native: token.price_dexscreener_sol,
                confidence: Some(result.confidence),
                slot: Some(result.slot),
                price_updated_at_unix,
            }
        } else {
            Self {
                last_result: None,
                price_sol: token.price_dexscreener_sol,
                price_usd: token.price_dexscreener_usd,
                price_native: token.price_dexscreener_sol,
                confidence: None,
                slot: None,
                price_updated_at_unix: Some(token.last_updated.timestamp()),
            }
        }
    }
}
