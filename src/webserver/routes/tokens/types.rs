//! Type definitions for the tokens API routes

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{
    filtering::{
        BlacklistReasonInfo, FilteringQuery, FilteringQueryResult, FilteringView, SortDirection,
        TokenSortKey,
    },
    logger::{self, LogTag},
    tokens::SecurityRisk,
};

// =============================================================================
// RESPONSE TYPES
// =============================================================================

/// Token list response
#[derive(Debug, Serialize)]
pub struct TokenListResponse {
    pub items: Vec<crate::tokens::types::Token>,
    pub page: usize,
    pub page_size: usize,
    pub total: usize,
    pub total_pages: usize,
    pub timestamp: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_cursor: Option<usize>,
    pub priced_total: usize,
    pub positions_total: usize,
    pub blacklisted_total: usize,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub rejection_reasons: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub available_rejection_reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub blacklist_reasons: HashMap<String, Vec<BlacklistReasonInfo>>,
}

/// Period-based numeric metrics helper
#[derive(Debug, Serialize, Clone)]
pub struct PeriodStats<T> {
    pub m5: Option<T>,
    pub h1: Option<T>,
    pub h6: Option<T>,
    pub h24: Option<T>,
}

impl<T> PeriodStats<T> {
    pub fn empty() -> Self {
        Self {
            m5: None,
            h1: None,
            h6: None,
            h24: None,
        }
    }
}

/// Buy/sell counts for a specific timeframe
#[derive(Debug, Serialize, Clone)]
pub struct TxnPeriodSummary {
    pub buys: Option<i64>,
    pub sells: Option<i64>,
}

/// Website link metadata for presentation
#[derive(Debug, Serialize, Clone)]
pub struct TokenWebsiteLink {
    pub label: Option<String>,
    pub url: String,
}

/// Social link metadata for presentation
#[derive(Debug, Serialize, Clone)]
pub struct TokenSocialLink {
    pub platform: String,
    pub url: String,
}

/// Pool descriptor for token detail view
#[derive(Debug, Serialize, Clone)]
pub struct TokenPoolInfo {
    pub pool_id: String,
    pub program: String,
    pub base_mint: String,
    pub quote_mint: String,
    pub token_role: String,
    pub paired_mint: String,
    pub liquidity_usd: Option<f64>,
    pub volume_h24_usd: Option<f64>,
    pub reserve_accounts: Vec<String>,
    pub is_canonical: bool,
    pub last_updated_unix: Option<i64>,
}

/// Top holder info for security display
#[derive(Debug, Serialize, Clone)]
pub struct TopHolderInfo {
    pub address: String,
    pub percentage: f64,
    pub is_insider: bool,
    pub owner_type: Option<String>,
}

/// Token detail response with enriched data
#[derive(Debug, Serialize)]
pub struct TokenDetailResponse {
    // Identity
    pub mint: String,
    pub symbol: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub decimals: Option<u8>,

    // Visuals / Media
    pub logo_url: Option<String>,
    pub header_image_url: Option<String>,
    pub open_graph_image: Option<String>,

    // Primary website (convenience field)
    pub website: Option<String>,

    // Data source info
    pub data_source: Option<String>,

    // Status flags
    pub verified: bool,
    pub tags: Vec<String>,
    pub pair_labels: Vec<String>,
    pub blacklisted: bool,
    pub has_ohlcv: bool,
    pub has_pool_price: bool,
    pub has_open_position: bool,

    // Timestamps
    pub created_at: Option<i64>,
    pub market_data_last_fetched_at: Option<i64>,
    pub pool_price_last_calculated_at: Option<i64>,
    pub pair_created_at: Option<i64>,
    pub pair_url: Option<String>,
    pub boosts_active: Option<i64>,

    // Price data
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub price_confidence: Option<String>,
    pub price_change_h1: Option<f64>,
    pub price_change_h24: Option<f64>,
    pub price_change_periods: PeriodStats<f64>,

    // Liquidity
    pub liquidity_usd: Option<f64>,
    pub liquidity_base: Option<f64>,
    pub liquidity_quote: Option<f64>,

    // Volume
    pub volume_24h: Option<f64>,
    pub volume_periods: PeriodStats<f64>,

    // Market metrics
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,

    // Pool info
    pub pool_address: Option<String>,
    pub pool_dex: Option<String>,
    pub pool_reserves_sol: Option<f64>,
    pub pool_reserves_token: Option<f64>,

    // Transactions
    pub txn_periods: PeriodStats<TxnPeriodSummary>,
    pub buys_24h: Option<i64>,
    pub sells_24h: Option<i64>,
    pub net_flow_24h: Option<i64>,
    pub buy_sell_ratio_24h: Option<f64>,

    // Security
    /// Raw risk score from Rugcheck (0-150000+, HIGHER = MORE RISKY)
    pub risk_score: Option<i32>,
    /// Computed safety score (0-100, HIGHER = SAFER) - inverted from Rugcheck normalized score
    pub safety_score: Option<i32>,
    pub rugged: Option<bool>,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub total_holders: Option<i64>,
    pub top_10_concentration: Option<f64>,
    pub security_risks: Vec<SecurityRisk>,
    pub security_summary: Option<String>,
    // Additional security fields
    pub token_type: Option<String>,
    pub creator_balance_pct: Option<f64>,
    pub lp_provider_count: Option<i64>,
    pub graph_insiders_detected: Option<i64>,
    pub transfer_fee_pct: Option<f64>,
    pub transfer_fee_max_amount: Option<i64>,
    pub transfer_fee_authority: Option<String>,
    pub top_holders: Vec<TopHolderInfo>,
    pub security_last_updated: Option<i64>,

    // Social/Links
    pub websites: Vec<TokenWebsiteLink>,
    pub socials: Vec<TokenSocialLink>,

    // Pools
    pub pools: Vec<TokenPoolInfo>,

    // Per-source data status (which providers supplied data, which have none,
    // which are currently unavailable). Drives the dialog's "no data" row so the
    // user sees exactly why a token shows blanks instead of a silent empty view.
    pub source_status: Vec<SourceStatus>,

    // Metadata
    pub timestamp: String,
}

/// State of a single upstream data source for a token, surfaced in the
/// token-details dialog. `ok` = we have data; `no_data` = the source simply
/// does not list this token; `unavailable` = the source is currently
/// unreachable/rate-limited so data may appear later.
#[derive(Debug, Serialize, Clone)]
pub struct SourceStatus {
    /// Stable id: "dexscreener" | "geckoterminal" | "rugcheck" | "ohlcv".
    pub source: String,
    /// Display label, e.g. "DexScreener".
    pub label: String,
    /// "ok" | "no_data" | "unavailable".
    pub state: String,
    /// Short human message, e.g. "Not listed on DexScreener".
    pub message: String,
}

/// OHLCV data point for charting
#[derive(Debug, Serialize, Clone)]
pub struct OhlcvPoint {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

// =============================================================================
// TOKEN ANALYSIS RESPONSE TYPES
// =============================================================================

/// Comprehensive token analysis response for the Token Analyzer feature
#[derive(Debug, Serialize)]
pub struct TokenAnalysisResponse {
    pub success: bool,
    pub overview: TokenOverview,
    pub security: Option<SecurityAnalysis>,
    pub market: Option<MarketAnalysis>,
    pub liquidity: Option<LiquidityAnalysis>,
    pub fetched_at: String,
}

/// Token overview information
#[derive(Debug, Serialize)]
pub struct TokenOverview {
    pub mint: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub logo_url: Option<String>,
    pub decimals: u8,
    pub supply: Option<String>,
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub total_holders: Option<i64>,
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
}

/// Security analysis data
#[derive(Debug, Serialize)]
pub struct SecurityAnalysis {
    /// Raw risk score from Rugcheck (0-150000+, HIGHER = MORE RISKY)
    pub score: Option<i32>,
    /// Rugcheck normalized score (0-100, HIGHER = MORE RISKY) - use for filtering
    pub normalized_score: Option<i32>,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub has_transfer_fee: bool,
    pub is_mutable: bool,
    pub top_holders_pct: Option<f64>,
    pub risks: Vec<AnalysisSecurityRisk>,
}

/// Security risk item for analysis
#[derive(Debug, Serialize)]
pub struct AnalysisSecurityRisk {
    pub name: String,
    pub level: String,
    pub description: String,
}

/// Market analysis data
#[derive(Debug, Serialize)]
pub struct MarketAnalysis {
    pub price_sol: f64,
    pub price_usd: Option<f64>,
    pub volume_h24: Option<f64>,
    pub volume_h6: Option<f64>,
    pub volume_h1: Option<f64>,
    pub price_change_h24: Option<f64>,
    pub price_change_h6: Option<f64>,
    pub price_change_h1: Option<f64>,
    pub txns_buys_h24: Option<i64>,
    pub txns_sells_h24: Option<i64>,
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,
}

/// Liquidity analysis data
#[derive(Debug, Serialize)]
pub struct LiquidityAnalysis {
    pub total_liquidity_sol: f64,
    pub total_liquidity_usd: Option<f64>,
    pub pool_count: i32,
    pub pools: Vec<AnalysisPoolInfo>,
}

/// Pool info for liquidity analysis
#[derive(Debug, Serialize)]
pub struct AnalysisPoolInfo {
    pub address: String,
    pub dex: String,
    pub liquidity_sol: f64,
    pub is_canonical: bool,
}

/// Token statistics response
#[derive(Debug, Serialize)]
pub struct TokenStatsResponse {
    /// Total tokens in database (all tokens, including those without market data)
    pub total_tokens_in_database: usize,
    /// Tokens with market data loaded in filtering snapshot
    pub total_tokens: usize,
    pub with_pool_price: usize,
    pub open_positions: usize,
    pub blacklisted: usize,
    pub with_ohlcv: usize,
    pub timestamp: String,
}

// =============================================================================
// QUERY TYPES
// =============================================================================

/// Token list query parameters
#[derive(Debug, Deserialize)]
pub struct TokenListQuery {
    #[serde(default = "default_view")]
    pub view: String,
    #[serde(default)]
    pub search: String,
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    #[serde(default = "default_sort_dir")]
    pub sort_dir: String,
    #[serde(default)]
    pub cursor: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
    #[serde(default)]
    pub min_holders: Option<i32>,
    #[serde(default)]
    pub has_pool_price: Option<bool>,
    #[serde(default)]
    pub has_open_position: Option<bool>,
    #[serde(default)]
    pub rejection_reason: Option<String>,
}

pub(super) fn default_view() -> String {
    "pool".to_owned()
}
pub(super) fn default_sort_by() -> String {
    "liquidity_usd".to_owned()
}
pub(super) fn default_sort_dir() -> String {
    "desc".to_owned()
}
pub(super) fn default_page() -> usize {
    1
}
pub(super) fn default_page_size() -> usize {
    50
}

/// OHLCV query parameters
#[derive(Debug, Deserialize)]
pub struct OhlcvQuery {
    #[serde(default = "default_ohlcv_limit")]
    pub limit: u32,
    #[serde(default = "default_ohlcv_timeframe")]
    pub timeframe: String,
}

pub(super) fn default_ohlcv_limit() -> u32 {
    100
}

pub(super) fn default_ohlcv_timeframe() -> String {
    "1m".to_owned()
}

/// Filter request body
#[derive(Debug, Deserialize)]
pub struct FilterRequest {
    #[serde(default = "default_view")]
    pub view: String,
    #[serde(default)]
    pub search: String,
    pub min_liquidity: Option<f64>,
    pub max_liquidity: Option<f64>,
    pub min_volume_24h: Option<f64>,
    pub max_volume_24h: Option<f64>,
    pub max_risk_score: Option<i32>,
    pub min_holders: Option<i32>,
    pub has_pool_price: Option<bool>,
    pub has_open_position: Option<bool>,
    pub blacklisted: Option<bool>,
    pub has_ohlcv: Option<bool>,
    #[serde(default)]
    pub rejection_reason: Option<String>,
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    #[serde(default = "default_sort_dir")]
    pub sort_dir: String,
    #[serde(default)]
    pub cursor: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

// =============================================================================
// SEARCH TYPES
// =============================================================================

/// Query parameters for token search
#[derive(Debug, Deserialize)]
pub struct TokenSearchQuery {
    pub q: String,
    pub limit: Option<usize>,
}

/// Token search response
#[derive(Debug, Serialize)]
pub struct TokenSearchResponse {
    pub results: Vec<crate::tokens::TokenSearchResult>,
    pub query: String,
    pub total: usize,
}

// =============================================================================
// FAVORITES TYPES
// =============================================================================

/// Response for favorites list.
///
/// Each entry is the full assembled `Token` (same shape as `/api/tokens/list`
/// items, so the dashboard can render the identical column set) merged with the
/// favorite's own fields (`notes`, `favorite_created_at`, `is_favorite`) and the
/// trading-state flags (`has_open_position`, `blacklisted`). Built as
/// `serde_json::Value` so the token JSON and the favorite extras flatten into one
/// flat row object.
#[derive(Debug, Serialize)]
pub struct FavoritesListResponse {
    pub favorites: Vec<serde_json::Value>,
    pub total: usize,
}

/// Response for single favorite operations
#[derive(Debug, Serialize)]
pub struct FavoriteResponse {
    pub success: bool,
    pub favorite: Option<crate::tokens::favorites::FavoriteToken>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// =============================================================================
// BLACKLIST TYPES
// =============================================================================

/// Request to add a token to blacklist
#[derive(Debug, Clone, Deserialize)]
pub struct AddBlacklistRequest {
    pub mint: String,
    #[serde(default = "default_blacklist_reason")]
    pub reason: String,
}

pub(super) fn default_blacklist_reason() -> String {
    "Manual blacklist via UI".to_owned()
}

/// Response for blacklist operations
#[derive(Debug, Serialize)]
pub struct BlacklistResponse {
    pub success: bool,
    pub mint: String,
    pub is_blacklisted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// =============================================================================
// FOCUS TYPES - Dashboard priority boost
// =============================================================================

/// Response for token focus/unfocus operations
/// Used when user opens/closes token details dialog to boost data fetching priority
#[derive(Debug, Serialize)]
pub struct FocusResponse {
    pub success: bool,
    pub mint: String,
    pub focused: bool,
    pub ohlcv_priority_updated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

pub(super) fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

pub(super) fn normalize_search(value: String) -> Option<String> {
    let trimmed = value.trim().to_owned();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

pub(super) fn normalize_choice(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim().to_owned();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("all") {
            None
        } else {
            Some(trimmed)
        }
    })
}

pub(super) fn resolve_page_and_size(
    cursor: Option<usize>,
    limit: Option<usize>,
    page: usize,
    page_size: usize,
    max_page_size: usize,
) -> (usize, usize) {
    let mut effective_limit = limit.unwrap_or(page_size).max(1);
    let max_page_size = max_page_size.max(1);
    if effective_limit > max_page_size {
        effective_limit = max_page_size;
    }

    let base_cursor = cursor.unwrap_or_else(|| {
        let safe_page = page.max(1);
        safe_page.saturating_sub(1).saturating_mul(effective_limit)
    });

    let normalized_cursor = (base_cursor / effective_limit).saturating_mul(effective_limit);
    let computed_page = (normalized_cursor / effective_limit).saturating_add(1);

    (computed_page.max(1), effective_limit)
}

pub(super) fn build_token_list_response(
    result: FilteringQueryResult,
    view: FilteringView,
) -> TokenListResponse {
    let start_index = result
        .page
        .saturating_sub(1)
        .saturating_mul(result.page_size);
    let current_len = result.items.len();

    let next_cursor = if start_index + current_len < result.total {
        Some(start_index + current_len)
    } else {
        None
    };

    let prev_cursor = if start_index == 0 || result.page_size == 0 {
        None
    } else {
        Some(start_index.saturating_sub(result.page_size))
    };

    // For Pool Service view, overlay real-time pool prices from pools module
    let items = result.items;

    TokenListResponse {
        items,
        page: result.page,
        page_size: result.page_size,
        total: result.total,
        total_pages: result.total_pages,
        timestamp: result.timestamp.to_rfc3339(),
        cursor: Some(start_index),
        next_cursor,
        prev_cursor,
        priced_total: result.priced_total,
        positions_total: result.positions_total,
        blacklisted_total: result.blacklisted_total,
        rejection_reasons: result.rejection_reasons,
        available_rejection_reasons: result.available_rejection_reasons,
        blacklist_reasons: result.blacklist_reasons,
    }
}

impl TokenListQuery {
    pub fn into_filtering_query(self, max_page_size: usize) -> FilteringQuery {
        let (page, page_size) = resolve_page_and_size(
            self.cursor,
            self.limit,
            self.page,
            self.page_size,
            max_page_size,
        );
        let mut query = FilteringQuery::default();
        query.view = FilteringView::from_str(&self.view);
        query.search = normalize_search(self.search);
        query.sort_key = TokenSortKey::from_str(&self.sort_by);
        query.sort_direction = SortDirection::from_str(&self.sort_dir);
        query.page = page.max(1);
        query.page_size = page_size.max(1);
        query.min_unique_holders = self.min_holders;
        query.has_pool_price = self.has_pool_price;
        query.has_open_position = self.has_open_position;
        query.rejection_reason = normalize_choice(self.rejection_reason);
        query.clamp_page_size(max_page_size);
        query
    }
}

impl FilterRequest {
    pub fn into_filtering_query(self, max_page_size: usize) -> FilteringQuery {
        let (page, page_size) = resolve_page_and_size(
            self.cursor,
            self.limit,
            self.page,
            self.page_size,
            max_page_size,
        );
        let mut query = FilteringQuery::default();
        query.view = FilteringView::from_str(&self.view);
        query.search = normalize_search(self.search);
        query.sort_key = TokenSortKey::from_str(&self.sort_by);
        query.sort_direction = SortDirection::from_str(&self.sort_dir);
        query.page = page.max(1);
        query.page_size = page_size.max(1);
        query.min_liquidity = self.min_liquidity;
        query.max_liquidity = self.max_liquidity;
        query.min_volume_24h = self.min_volume_24h;
        query.max_volume_24h = self.max_volume_24h;
        query.max_risk_score = self.max_risk_score;
        query.min_unique_holders = self.min_holders;
        query.has_pool_price = self.has_pool_price;
        query.has_open_position = self.has_open_position;
        query.blacklisted = self.blacklisted;
        query.has_ohlcv = self.has_ohlcv;
        query.rejection_reason = normalize_choice(self.rejection_reason);
        query.clamp_page_size(max_page_size);
        query
    }
}

/// Attempt to fetch token from external APIs (DexScreener, GeckoTerminal) and add to database
///
/// This is used when a token is requested but not found in the local database.
/// Returns the Token if found and successfully added, None otherwise.
/// Build the per-source status list for the token-details dialog.
///
/// `has_*` reflect whether we hold data for each source. When we don't, we
/// distinguish "the source genuinely doesn't list this token" (`no_data`) from
/// "the source is currently unreachable/rate-limited" (`unavailable`) using the
/// connectivity health monitor, so the UI can say "retrying" instead of a flat
/// "no data" when a provider is merely throttled.
pub(super) async fn build_source_status(
    has_dexscreener: bool,
    has_geckoterminal: bool,
    has_rugcheck: bool,
    has_ohlcv: bool,
) -> Vec<SourceStatus> {
    async fn market_state(
        endpoint: &str,
        label: &str,
        has_data: bool,
    ) -> SourceStatus {
        let (state, message) = if has_data {
            ("ok", "Live market data".to_owned())
        } else if crate::connectivity::get_endpoint_health(endpoint)
            .await
            .map(|h| h.is_unhealthy())
            .unwrap_or(false)
        {
            ("unavailable", format!("{label} unavailable — retrying"))
        } else {
            ("no_data", format!("Not listed on {label}"))
        };
        SourceStatus {
            source: endpoint.to_owned(),
            label: label.to_owned(),
            state: state.to_owned(),
            message,
        }
    }

    let (dex, gecko) = tokio::join!(
        market_state("dexscreener", "DexScreener", has_dexscreener),
        market_state("geckoterminal", "GeckoTerminal", has_geckoterminal),
    );

    let rug = {
        let (state, message) = if has_rugcheck {
            ("ok", "Security report available".to_owned())
        } else if crate::connectivity::get_endpoint_health("rugcheck")
            .await
            .map(|h| h.is_unhealthy())
            .unwrap_or(false)
        {
            ("unavailable", "Rugcheck unavailable — retrying".to_owned())
        } else {
            ("no_data", "No Rugcheck report".to_owned())
        };
        SourceStatus {
            source: "rugcheck".to_owned(),
            label: "Rugcheck".to_owned(),
            state: state.to_owned(),
            message,
        }
    };

    let ohlcv = SourceStatus {
        source: "ohlcv".to_owned(),
        label: "Chart".to_owned(),
        state: if has_ohlcv { "ok" } else { "no_data" }.to_owned(),
        message: if has_ohlcv {
            "Chart data available".to_owned()
        } else {
            "No chart data yet".to_owned()
        },
    };

    vec![dex, gecko, rug, ohlcv]
}

/// Per-provider deadline for the on-demand external token fetch. Keeps a
/// rate-limited provider (e.g. GeckoTerminal 429 + retry/backoff) from stalling
/// the `GET /tokens/:mint` request past the dashboard's client-side abort.
/// Kept at 3s because the two providers are tried sequentially (DexScreener then
/// GeckoTerminal), so the worst case is ~2x this — 3s keeps the total under the
/// 10s client abort even when both providers are slow/rate-limited.
const EXTERNAL_FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

pub(super) async fn fetch_and_add_token_from_external(mint: &str) -> Option<crate::tokens::Token> {
    use crate::apis::get_api_manager;
    use crate::tokens::database::get_global_database;

    logger::debug(
        LogTag::Webserver,
        &format!("Token not in DB, attempting external fetch: mint={mint}"),
    );

    let apis = get_api_manager();
    let db = get_global_database()?;

    // Try DexScreener first - most reliable for Solana tokens.
    //
    // Bound the provider call: a user opening a token we have never tracked hits
    // this path, and when a provider is rate-limited (GeckoTerminal returns 429
    // and enters its internal retry/backoff) the fetch can block for 10s+ — past
    // the dashboard's client-side abort — leaving the token-details dialog stuck
    // on a loading spinner. A timed-out provider is treated as a miss so we fall
    // through to the next source (or NOT_FOUND) promptly instead of hanging.
    if apis.dexscreener.is_enabled() {
        let dex_fetch = tokio::time::timeout(
            EXTERNAL_FETCH_TIMEOUT,
            apis.dexscreener.fetch_token_pools(mint, None),
        )
        .await
        .unwrap_or_else(|_| Err("provider fetch timed out".to_owned()));
        match dex_fetch {
            Ok(pools) => {
                if let Some(pool) = pools.first() {
                    let symbol = if !pool.base_token_symbol.is_empty() {
                        Some(pool.base_token_symbol.clone())
                    } else {
                        None
                    };
                    let name = if !pool.base_token_name.is_empty() {
                        Some(pool.base_token_name.clone())
                    } else {
                        None
                    };

                    // Clone values for the spawn_blocking closure
                    let db_clone = db.clone();
                    let mint_owned = mint.to_string();
                    let symbol_clone = symbol.clone();
                    let name_clone = name.clone();

                    // Wrap blocking DB call in spawn_blocking
                    let upsert_result = tokio::task::spawn_blocking(move || {
                        db_clone.upsert_token(
                            &mint_owned,
                            symbol_clone.as_deref(),
                            name_clone.as_deref(),
                            None,
                        )
                    })
                    .await;

                    match upsert_result {
                        Ok(Ok(())) => {
                            logger::info(
                                LogTag::Webserver,
                                &format!(
                                    "Token added from DexScreener: mint={} symbol={:?} name={:?}",
                                    mint, symbol, name
                                ),
                            );

                            // Now fetch the token from database
                            if let Ok(Some(token)) = crate::tokens::get_full_token_async(mint).await
                            {
                                return Some(token);
                            }
                        }
                        Ok(Err(e)) => {
                            logger::warning(
                                LogTag::Webserver,
                                &format!(
                                    "Failed to add token from DexScreener: mint={} error={}",
                                    mint, e
                                ),
                            );
                        }
                        Err(e) => {
                            logger::warning(
                                LogTag::Webserver,
                                &format!(
                  "spawn_blocking failed for DexScreener upsert: mint={} error={}",
                  mint, e
                ),
                            );
                        }
                    }
                }
            }
            Err(e) => {
                logger::debug(
                    LogTag::Webserver,
                    &format!("DexScreener fetch failed for {mint}: {e}"),
                );
            }
        }
    }

    // Try GeckoTerminal as fallback (same bounded-fetch rationale as above:
    // never let a rate-limited provider stall the token-details request).
    if apis.geckoterminal.is_enabled() {
        let gecko_fetch = tokio::time::timeout(
            EXTERNAL_FETCH_TIMEOUT,
            apis.geckoterminal.fetch_pools(mint),
        )
        .await
        .unwrap_or_else(|_| Err("provider fetch timed out".to_owned()));
        match gecko_fetch {
            Ok(pools) => {
                if let Some(pool) = pools.first() {
                    // Extract symbol from pool name (format: "SYMBOL/SOL")
                    let symbol = pool
                        .pool_name
                        .split('/')
                        .next()
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string());

                    // Clone values for the spawn_blocking closure
                    let db_clone = db.clone();
                    let mint_owned = mint.to_string();
                    let symbol_clone = symbol.clone();

                    // Wrap blocking DB call in spawn_blocking
                    let upsert_result = tokio::task::spawn_blocking(move || {
                        db_clone.upsert_token(&mint_owned, symbol_clone.as_deref(), None, None)
                    })
                    .await;

                    match upsert_result {
                        Ok(Ok(())) => {
                            logger::info(
                                LogTag::Webserver,
                                &format!(
                                    "Token added from GeckoTerminal: mint={} symbol={:?}",
                                    mint, symbol
                                ),
                            );

                            // Now fetch the token from database
                            if let Ok(Some(token)) = crate::tokens::get_full_token_async(mint).await
                            {
                                return Some(token);
                            }
                        }
                        Ok(Err(e)) => {
                            logger::warning(
                                LogTag::Webserver,
                                &format!(
                                    "Failed to add token from GeckoTerminal: mint={} error={}",
                                    mint, e
                                ),
                            );
                        }
                        Err(e) => {
                            logger::warning(
                                LogTag::Webserver,
                                &format!(
                  "spawn_blocking failed for GeckoTerminal upsert: mint={} error={}",
                  mint, e
                ),
                            );
                        }
                    }
                }
            }
            Err(e) => {
                logger::debug(
                    LogTag::Webserver,
                    &format!("GeckoTerminal fetch failed for {mint}: {e}"),
                );
            }
        }
    }

    None
}
