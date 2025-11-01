use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::tokens::types::Token;

/// Maximum number of historical decisions to keep in memory per category
pub const MAX_DECISION_HISTORY: usize = 1000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PassedToken {
    pub mint: String,
    pub symbol: String,
    pub name: Option<String>,
    pub passed_time: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RejectedToken {
    pub mint: String,
    pub symbol: String,
    pub name: Option<String>,
    pub reason: String,
    pub rejection_time: i64,
}

/// Reason metadata describing why a token was blacklisted
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlacklistReasonInfo {
    pub category: String,
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilteringView {
    Pool,
    All,
    Passed,
    Rejected,
    Blacklisted,
    Positions,
    Recent,
    NoMarketData,
}

impl FilteringView {
    pub fn as_str(&self) -> &'static str {
        match self {
            FilteringView::Pool => "pool",
            FilteringView::All => "all",
            FilteringView::Passed => "passed",
            FilteringView::Rejected => "rejected",
            FilteringView::Blacklisted => "blacklisted",
            FilteringView::Positions => "positions",
            FilteringView::Recent => "recent",
            FilteringView::NoMarketData => "no_market",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "all" => FilteringView::All,
            "passed" => FilteringView::Passed,
            "rejected" => FilteringView::Rejected,
            "blacklisted" => FilteringView::Blacklisted,
            "positions" => FilteringView::Positions,
            "recent" => FilteringView::Recent,
            "no_market" => FilteringView::NoMarketData,
            _ => FilteringView::Pool,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            SortDirection::Asc => "asc",
            SortDirection::Desc => "desc",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "desc" => SortDirection::Desc,
            _ => SortDirection::Asc,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSortKey {
    Symbol,
    PriceSol,
    LiquidityUsd,
    Volume24h,
    Fdv,
    MarketCap,
    PriceChangeH1,
    PriceChangeH24,
    RiskScore,
    MarketDataLastFetchedAt,
    FirstDiscoveredAt,
    MetadataLastFetchedAt,
    BlockchainCreatedAt,
    PoolPriceLastCalculatedAt,
    Mint,
    Txns5m,
    Txns1h,
    Txns6h,
    Txns24h,
}

impl TokenSortKey {
    pub fn from_str(value: &str) -> Self {
        match value {
            "symbol" => TokenSortKey::Symbol,
            "price_sol" => TokenSortKey::PriceSol,
            "liquidity_usd" => TokenSortKey::LiquidityUsd,
            "volume_24h" => TokenSortKey::Volume24h,
            "fdv" => TokenSortKey::Fdv,
            "market_cap" => TokenSortKey::MarketCap,
            "price_change_h1" => TokenSortKey::PriceChangeH1,
            "price_change_h24" => TokenSortKey::PriceChangeH24,
            "risk_score" => TokenSortKey::RiskScore,
            "market_data_last_fetched_at" => TokenSortKey::MarketDataLastFetchedAt,
            "first_discovered_at" => TokenSortKey::FirstDiscoveredAt,
            "metadata_last_fetched_at" => TokenSortKey::MetadataLastFetchedAt,
            "blockchain_created_at" => TokenSortKey::BlockchainCreatedAt,
            "pool_price_last_calculated_at" => TokenSortKey::PoolPriceLastCalculatedAt,
            "txns_5m" => TokenSortKey::Txns5m,
            "txns_1h" => TokenSortKey::Txns1h,
            "txns_6h" => TokenSortKey::Txns6h,
            "txns_24h" => TokenSortKey::Txns24h,
            _ => TokenSortKey::Mint,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FilteringQuery {
    pub view: FilteringView,
    pub search: Option<String>,
    pub sort_key: TokenSortKey,
    pub sort_direction: SortDirection,
    pub page: usize,
    pub page_size: usize,
    pub min_liquidity: Option<f64>,
    pub max_liquidity: Option<f64>,
    pub min_volume_24h: Option<f64>,
    pub max_volume_24h: Option<f64>,
    pub max_risk_score: Option<i32>,
    pub min_unique_holders: Option<i32>,
    pub has_pool_price: Option<bool>,
    pub has_open_position: Option<bool>,
    pub blacklisted: Option<bool>,
    pub has_ohlcv: Option<bool>,
    pub rejection_reason: Option<String>,
}

impl Default for FilteringQuery {
    fn default() -> Self {
        Self {
            view: FilteringView::Pool,
            search: None,
            sort_key: TokenSortKey::LiquidityUsd,
            sort_direction: SortDirection::Desc,
            page: 1,
            page_size: 50,
            min_liquidity: None,
            max_liquidity: None,
            min_volume_24h: None,
            max_volume_24h: None,
            max_risk_score: None,
            min_unique_holders: None,
            has_pool_price: None,
            has_open_position: None,
            blacklisted: None,
            has_ohlcv: None,
            rejection_reason: None,
        }
    }
}

impl FilteringQuery {
    pub fn with_page_bounds(mut self) -> Self {
        if self.page == 0 {
            self.page = 1;
        }
        if self.page_size == 0 {
            self.page_size = 50;
        }
        self
    }

    pub fn clamp_page_size(&mut self, max_page_size: usize) {
        let max_size = max_page_size.max(1);
        self.page_size = self.page_size.max(1).min(max_size);
    }
}

#[derive(Debug, Clone)]
pub struct FilteringQueryResult {
    // Core items come directly from the unified Token type
    pub items: Vec<Token>,
    pub page: usize,
    pub page_size: usize,
    pub total: usize,
    pub total_pages: usize,
    pub timestamp: DateTime<Utc>,
    pub priced_total: usize,
    pub positions_total: usize,
    pub blacklisted_total: usize,
    // Derived flag sets for downstream consumers (UI/routes) to compute per-item flags
    // without duplicating summary structs. Kept as Vec<String> for easy JSON.
    pub priced_mints: Vec<String>,
    pub open_position_mints: Vec<String>,
    pub ohlcv_mints: Vec<String>,
    pub rejection_reasons: HashMap<String, String>,
    pub available_rejection_reasons: Vec<String>,
    pub blacklist_reasons: HashMap<String, Vec<BlacklistReasonInfo>>,
}

#[derive(Debug, Clone)]
pub struct TokenEntry {
    pub token: Token,
    pub has_pool_price: bool,
    pub has_open_position: bool,
    pub has_ohlcv: bool,
    pub pair_created_at: Option<i64>,
    pub last_updated: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct FilteringSnapshot {
    pub updated_at: DateTime<Utc>,
    pub filtered_mints: Vec<String>,
    pub passed_tokens: Vec<PassedToken>,
    pub rejected_mints: Vec<String>,
    pub rejected_tokens: Vec<RejectedToken>,
    pub tokens: HashMap<String, TokenEntry>,
    pub blacklist_reasons: HashMap<String, Vec<BlacklistReasonInfo>>,
}

impl FilteringSnapshot {
    pub fn empty() -> Self {
        Self {
            updated_at: Utc::now(),
            filtered_mints: Vec::new(),
            passed_tokens: Vec::new(),
            rejected_mints: Vec::new(),
            rejected_tokens: Vec::new(),
            tokens: HashMap::new(),
            blacklist_reasons: HashMap::new(),
        }
    }

    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }
}

impl Default for FilteringSnapshot {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Debug, Clone)]
pub struct FilteringStatsSnapshot {
    pub total_tokens: usize,
    pub with_pool_price: usize,
    pub open_positions: usize,
    pub blacklisted: usize,
    pub with_ohlcv: usize,
    pub passed_filtering: usize,
    pub updated_at: DateTime<Utc>,
}
