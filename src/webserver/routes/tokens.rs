use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use crate::{
    filtering::{
        self, BlacklistReasonInfo, FilteringQuery, FilteringQueryResult, FilteringView,
        SortDirection, TokenSortKey,
    },
    logger::{self, LogTag},
    pools, positions,
    tokens::cleanup,
    tokens::database::get_global_database,
    tokens::SecurityRisk,
    webserver::{
        state::AppState,
        utils::{error_response, success_response},
    },
};

const MAX_PAGE_SIZE: usize = 200;

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

/// Token detail response with enriched data
#[derive(Debug, Serialize)]
pub struct TokenDetailResponse {
    // Identity
    pub mint: String,
    pub symbol: String,
    pub name: Option<String>,
    pub tagline: Option<String>,
    pub description: Option<String>,
    pub decimals: Option<u8>,

    // Visuals
    pub logo_url: Option<String>,
    pub website: Option<String>,

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
    pub last_updated: Option<i64>,
    pub pair_created_at: Option<i64>,
    pub pair_url: Option<String>,
    pub boosts_active: Option<i64>,

    // Price data
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub price_confidence: Option<String>,
    pub price_updated_at: Option<i64>,
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
    pub risk_score: Option<i32>,
    pub rugged: Option<bool>,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub total_holders: Option<i64>,
    pub top_10_concentration: Option<f64>,
    pub security_risks: Vec<SecurityRisk>,
    pub security_summary: Option<String>,

    // Social/Links
    pub websites: Vec<TokenWebsiteLink>,
    pub socials: Vec<TokenSocialLink>,

    // Pools
    pub pools: Vec<TokenPoolInfo>,

    // Metadata
    pub timestamp: String,
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

fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

/// Token statistics response
#[derive(Debug, Serialize)]
pub struct TokenStatsResponse {
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

fn default_view() -> String {
    "pool".to_string()
}
fn default_sort_by() -> String {
    "liquidity_usd".to_string()
}
fn default_sort_dir() -> String {
    "desc".to_string()
}
fn default_page() -> usize {
    1
}
fn default_page_size() -> usize {
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

fn default_ohlcv_limit() -> u32 {
    100
}

fn default_ohlcv_timeframe() -> String {
    "1m".to_string()
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

fn normalize_search(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn normalize_choice(value: Option<String>) -> Option<String> {
    value.and_then(|raw| {
        let trimmed = raw.trim().to_string();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("all") {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn resolve_page_and_size(
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

fn build_token_list_response(
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
    let items = if matches!(view, FilteringView::Pool) {
        result
            .items
            .into_iter()
            .map(|mut token| {
                if let Some(price_result) = pools::get_pool_price(&token.mint) {
                    let old_price = token.price_sol;
                    let new_price = price_result.price_sol;
                    // Overlay pool price (real-time chain data) over database price
                    token.price_sol = new_price;

                    // Update timestamp to reflect pool price freshness
                    let age = price_result.timestamp.elapsed();
                    if let Ok(duration) = chrono::Duration::from_std(age) {
                        token.pool_price_last_calculated_at = chrono::Utc::now() - duration;
                    }

                    logger::debug(
                        LogTag::Webserver,
                        &format!(
                            "Pool price overlay: mint={} symbol={} old_price={:.12} new_price={:.12} diff={:.12} age={:.1}s",
                            token.mint,
                            token.symbol,
                            old_price,
                            new_price,
                            (new_price - old_price).abs(),
                            age.as_secs_f64()
                        ),
                    );
                }
                token
            })
            .collect()
    } else {
        // For other views, use database prices as-is
        result.items
    };

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
    fn into_filtering_query(self, max_page_size: usize) -> FilteringQuery {
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
    fn into_filtering_query(self, max_page_size: usize) -> FilteringQuery {
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

// =============================================================================
// ROUTE REGISTRATION
// =============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tokens/list", get(get_tokens_list))
        .route("/tokens/stats", get(get_tokens_stats))
        .route("/tokens/filter", post(filter_tokens))
        .route("/tokens/:mint", get(get_token_detail))
        .route("/tokens/:mint/refresh", post(refresh_token_data))
        .route("/tokens/:mint/ohlcv", get(get_token_ohlcv))
        .route("/tokens/:mint/ohlcv/refresh", post(refresh_token_ohlcv))
        .route("/tokens/:mint/dexscreener", get(get_token_dexscreener))
}

// =============================================================================
// HANDLERS
// =============================================================================

/// GET /api/tokens/list
///
/// Query: view, search, sort_by, sort_dir, cursor, limit, page, page_size,
///        has_pool_price, has_open_position
pub(crate) async fn get_tokens_list(
    Query(query): Query<TokenListQuery>,
) -> Json<TokenListResponse> {
    let max_page_size = MAX_PAGE_SIZE;
    let request_view = query.view.clone();
    let filtering_query = query.into_filtering_query(max_page_size);
    let view = FilteringView::from_str(&request_view);

    match filtering::query_tokens(filtering_query).await {
        Ok(result) => {
            logger::debug(
                LogTag::Webserver,
                &format!(
                    "view={} page={}/{} items={}/{}",
                    request_view,
                    result.page,
                    result.total_pages,
                    result.items.len(),
                    result.total
                ),
            );

            Json(build_token_list_response(result, view))
        }
        Err(err) => {
            logger::info(
                LogTag::Webserver,
                &format!("Failed to load tokens list via filtering service: {}", err),
            );

            Json(TokenListResponse {
                items: vec![],
                page: 1,
                page_size: max_page_size,
                total: 0,
                total_pages: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
                cursor: Some(0),
                next_cursor: None,
                prev_cursor: None,
                priced_total: 0,
                positions_total: 0,
                blacklisted_total: 0,
                rejection_reasons: HashMap::new(),
                available_rejection_reasons: Vec::new(),
                blacklist_reasons: HashMap::new(),
            })
        }
    }
}

/// Force refresh token data (immediate update outside scheduled loops)
async fn refresh_token_data(
    Path(mint): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    logger::debug(
        LogTag::Webserver,
        &format!("Force refresh requested for mint={}", mint),
    );

    match crate::tokens::request_immediate_update(&mint).await {
        Ok(result) => {
            if result.is_success() {
                logger::info(
                    LogTag::Webserver,
                    &format!(
                        "mint={} refresh_success sources={:?}",
                        mint, result.successes
                    ),
                );
                Ok(Json(serde_json::json!({
                    "success": true,
                    "mint": mint,
                    "sources_updated": result.successes,
                    "partial_failures": result.failures,
                })))
            } else {
                logger::info(
                    LogTag::Webserver,
                    &format!(
                        "mint={} refresh_failed failures={:?}",
                        mint, result.failures
                    ),
                );
                Err((
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({
                        "success": false,
                        "mint": mint,
                        "error": "All data sources failed",
                        "failures": result.failures,
                    })),
                ))
            }
        }
        Err(e) => {
            logger::warning(
                LogTag::Webserver,
                &format!("mint={} refresh_error error={}", mint, e),
            );
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({
                    "success": false,
                    "mint": mint,
                    "error": format!("Failed to refresh token: {}", e),
                })),
            ))
        }
    }
}

/// Get token detail
async fn get_token_detail(Path(mint): Path<String>) -> Json<TokenDetailResponse> {
    let request_start = std::time::Instant::now();

    logger::debug(LogTag::Webserver, &format!("mint={}", mint));

    // Fetch token from database (with market data)
    let lookup_start = std::time::Instant::now();
    let snapshot = match crate::tokens::get_full_token_async(&mint).await {
        Ok(Some(snap)) => {
            logger::info(
                LogTag::Webserver,
                &format!(
                    "mint={} elapsed={}μs",
                    mint,
                    lookup_start.elapsed().as_micros()
                ),
            );
            snap
        }
        Ok(None) | Err(_) => {
            logger::info(
                LogTag::Webserver,
                &format!(
                    "mint={} elapsed={}μs",
                    mint,
                    lookup_start.elapsed().as_micros()
                ),
            );
            return Json(TokenDetailResponse {
                mint: mint.clone(),
                symbol: "NOT_FOUND".to_string(),
                name: Some("Token not in cache".to_string()),
                tagline: None,
                description: None,
                logo_url: None,
                website: None,
                verified: false,
                tags: vec![],
                pair_labels: vec![],
                decimals: None,
                created_at: None,
                last_updated: None,
                pair_created_at: None,
                pair_url: None,
                boosts_active: None,
                price_sol: None,
                price_usd: None,
                price_confidence: None,
                price_updated_at: None,
                price_change_h1: None,
                price_change_h24: None,
                price_change_periods: PeriodStats::empty(),
                liquidity_usd: None,
                liquidity_base: None,
                liquidity_quote: None,
                volume_24h: None,
                volume_periods: PeriodStats::empty(),
                fdv: None,
                market_cap: None,
                pool_address: None,
                pool_dex: None,
                pool_reserves_sol: None,
                pool_reserves_token: None,
                txn_periods: PeriodStats::empty(),
                buys_24h: None,
                sells_24h: None,
                net_flow_24h: None,
                buy_sell_ratio_24h: None,
                risk_score: None,
                rugged: None,
                mint_authority: None,
                freeze_authority: None,
                total_holders: None,
                top_10_concentration: None,
                security_risks: vec![],
                security_summary: None,
                websites: vec![],
                socials: vec![],
                pools: vec![],
                has_ohlcv: false,
                has_pool_price: false,
                has_open_position: false,
                blacklisted: false,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };

    // Extract token from snapshot for processing
    let token = &snapshot;

    let pool_descriptors = pools::get_token_pools(&mint);
    let canonical_pool_id = pool_descriptors.first().map(|pool| pool.pool_id);
    let mint_pubkey = Pubkey::from_str(&mint).ok();
    let now_unix_opt = match std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => Some(duration.as_secs() as i64),
        Err(_) => None,
    };
    let pool_infos: Vec<TokenPoolInfo> = pool_descriptors
        .into_iter()
        .map(|pool| {
            let token_role = if let Some(mint_key) = mint_pubkey {
                if pool.base_mint == mint_key {
                    "base"
                } else if pool.quote_mint == mint_key {
                    "quote"
                } else {
                    "unknown"
                }
            } else {
                "unknown"
            };

            let paired_mint = if token_role == "base" {
                pool.quote_mint.to_string()
            } else {
                pool.base_mint.to_string()
            };

            let age_secs = pool.last_updated.elapsed().as_secs();
            let age_i64 = if age_secs > i64::MAX as u64 {
                i64::MAX
            } else {
                age_secs as i64
            };

            let last_updated_unix = now_unix_opt.map(|now| now.saturating_sub(age_i64));

            TokenPoolInfo {
                pool_id: pool.pool_id.to_string(),
                program: pool.program_kind.display_name().to_string(),
                base_mint: pool.base_mint.to_string(),
                quote_mint: pool.quote_mint.to_string(),
                token_role: token_role.to_string(),
                paired_mint,
                liquidity_usd: if pool.liquidity_usd.is_finite() {
                    Some(pool.liquidity_usd)
                } else {
                    None
                },
                volume_h24_usd: if pool.volume_h24_usd.is_finite() {
                    Some(pool.volume_h24_usd)
                } else {
                    None
                },
                reserve_accounts: pool
                    .reserve_accounts
                    .iter()
                    .map(|account| account.to_string())
                    .collect(),
                is_canonical: canonical_pool_id
                    .map(|canonical| canonical == pool.pool_id)
                    .unwrap_or(false),
                last_updated_unix,
            }
        })
        .collect();

    // Get enrichment data (all sync or from cache)
    let pool_start = std::time::Instant::now();
    let (
        price_sol,
        price_confidence,
        price_updated_at,
        pool_address,
        pool_dex,
        pool_reserves_sol,
        pool_reserves_token,
    ) = if let Some(price_result) = pools::get_pool_price(&mint) {
        let age_secs = price_result.timestamp.elapsed().as_secs();
        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        (
            Some(price_result.price_sol),
            Some(price_result.confidence.to_string()),
            Some(now_unix - (age_secs as i64)),
            Some(price_result.pool_address),
            price_result.source_pool,
            Some(price_result.sol_reserves),
            Some(price_result.token_reserves),
        )
    } else {
        (None, None, None, None, None, None, None)
    };

    logger::info(
        LogTag::Webserver,
        &format!(
            "mint={} elapsed={}ms has_price={}",
            mint,
            pool_start.elapsed().as_millis(),
            price_sol.is_some()
        ),
    );

    // Get security info using async wrapper (prevents blocking async runtime)
    let security_start = std::time::Instant::now();
    let (
        security_score,
        rugged,
        mint_authority,
        freeze_authority,
        total_holders,
        top_10_concentration,
        security_risks,
    ) = (
        None,
        None,
        None,
        None,
        None,
        None,
        Vec::<SecurityRisk>::new(),
    );

    logger::info(
        LogTag::Webserver,
        &format!(
            "mint={} elapsed={}ms has_score={}",
            mint,
            security_start.elapsed().as_millis(),
            security_score.is_some()
        ),
    );

    // Get status flags (mix of sync and cache checks)
    let ohlcv_start = std::time::Instant::now();
    let has_ohlcv = match crate::ohlcvs::has_data(&mint).await {
        Ok(flag) => flag,
        Err(e) => {
            logger::info(
                LogTag::Webserver,
                &format!("Failed to determine OHLCV availability for {}: {}", mint, e),
            );
            false
        }
    };

    logger::info(
        LogTag::Webserver,
        &format!(
            "mint={} elapsed={}ms has_data={}",
            mint,
            ohlcv_start.elapsed().as_millis(),
            has_ohlcv
        ),
    );

    let has_pool_price = price_sol.is_some();
    let blacklisted = if let Some(db) = get_global_database() {
        let mint_clone = mint.clone();
        let db_clone = db.clone();
        match tokio::task::spawn_blocking(move || db_clone.is_blacklisted(&mint_clone)).await {
            Ok(Ok(flag)) => flag,
            Ok(Err(err)) => {
                logger::info(
                    LogTag::Webserver,
                    &format!("Failed to check blacklist for {}: {}", mint, err),
                );
                false
            }
            Err(join_err) => {
                logger::info(
                    LogTag::Webserver,
                    &format!("Join error checking blacklist for {}: {}", mint, join_err),
                );
                false
            }
        }
    } else {
        false
    };

    // Position check - this is the ONLY additional async, keep it last and simple
    let position_start = std::time::Instant::now();
    let has_open_position = positions::is_open_position(&mint).await;

    logger::info(
        LogTag::Webserver,
        &format!(
            "mint={} elapsed={}ms has_position={}",
            mint,
            position_start.elapsed().as_millis(),
            has_open_position
        ),
    );

    // Add token to OHLCV monitoring with appropriate priority
    // This ensures chart data will be available when users view this token again
    let monitoring_start = std::time::Instant::now();
    let priority = if has_open_position {
        crate::ohlcvs::Priority::Critical
    } else {
        crate::ohlcvs::Priority::Medium // User is viewing, so medium priority
    };

    if let Err(e) = crate::ohlcvs::add_token_monitoring(&mint, priority).await {
        logger::info(
            LogTag::Webserver,
            &format!("Failed to add {} to OHLCV monitoring: {}", mint, e),
        );
    }

    // Record view activity
    if let Err(e) =
        crate::ohlcvs::record_activity(&mint, crate::ohlcvs::ActivityType::TokenViewed).await
    {
        logger::info(
            LogTag::Webserver,
            &format!("Failed to record token view for {}: {}", mint, e),
        );
    }

    logger::info(
        LogTag::Webserver,
        &format!(
            "mint={} elapsed={}ms",
            mint,
            monitoring_start.elapsed().as_millis()
        ),
    );

    logger::info(
        LogTag::Webserver,
        &format!(
            "mint={} total_elapsed={}ms",
            mint,
            request_start.elapsed().as_millis()
        ),
    );

    let created_at_ts = Some(token.first_discovered_at.timestamp());
    let token_birth_ts = token.blockchain_created_at.map(|dt| dt.timestamp());
    let last_updated_ts = Some(token.market_data_last_fetched_at.timestamp());
    let pair_created_at = token_birth_ts.or(created_at_ts);

    // Prefer pool price (real-time on-chain) over token cached price
    let price_usd = price_sol.map(|p| p * 150.0); // Rough SOL/USD conversion; ideally fetch SOL price

    // Build price change periods from flat fields
    let price_change_periods = PeriodStats {
        m5: token.price_change_m5,
        h1: token.price_change_h1,
        h6: token.price_change_h6,
        h24: token.price_change_h24,
    };

    // Build volume periods from flat fields
    let volume_periods = PeriodStats {
        m5: token.volume_m5,
        h1: token.volume_h1,
        h6: token.volume_h6,
        h24: token.volume_h24,
    };

    // Build transaction periods from flat fields
    let txn_periods = PeriodStats {
        m5: Some(TxnPeriodSummary {
            buys: token.txns_m5_buys,
            sells: token.txns_m5_sells,
        }),
        h1: Some(TxnPeriodSummary {
            buys: token.txns_h1_buys,
            sells: token.txns_h1_sells,
        }),
        h6: Some(TxnPeriodSummary {
            buys: token.txns_h6_buys,
            sells: token.txns_h6_sells,
        }),
        h24: Some(TxnPeriodSummary {
            buys: token.txns_h24_buys,
            sells: token.txns_h24_sells,
        }),
    };

    let buys_24h = token.txns_h24_buys;
    let sells_24h = token.txns_h24_sells;

    let net_flow_24h = match (buys_24h, sells_24h) {
        (Some(buys), Some(sells)) => Some(buys - sells),
        _ => None,
    };

    let buy_sell_ratio_24h = match (buys_24h, sells_24h) {
        (Some(buys), Some(sells)) if sells != 0 => Some(buys as f64 / sells as f64),
        _ => None,
    };

    // Liquidity base/quote not available in new unified Token - use None
    let liquidity_base = None;
    let liquidity_quote = None;

    // Build websites from token.websites vec
    let mut websites: Vec<TokenWebsiteLink> = token
        .websites
        .iter()
        .filter(|w| !w.url.trim().is_empty())
        .map(|w| TokenWebsiteLink {
            label: w.label.clone(),
            url: w.url.clone(),
        })
        .collect();

    // Build socials from token.socials vec
    let socials: Vec<TokenSocialLink> = token
        .socials
        .iter()
        .filter(|s| !s.url.trim().is_empty())
        .map(|s| TokenSocialLink {
            platform: s.link_type.clone(),
            url: s.url.clone(),
        })
        .collect();

    // Tags - unified token doesn't have separate tags/labels, use empty vec
    let combined_tags: Vec<String> = Vec::new();

    let logo_url = token.image_url.clone();
    let primary_website = websites.first().map(|link| link.url.clone());

    let security_summary = match (rugged, security_score) {
        (Some(true), Some(score)) => Some(format!(
            "⚠️ Token flagged as rugged (score {}). Investigate before trading.",
            score
        )),
        (Some(true), None) => {
            Some("⚠️ Token flagged as rugged. Investigate before trading.".to_string())
        }
        (_, Some(score)) if score >= 700 => {
            Some(format!("✅ Strong security posture (score {}).", score))
        }
        (_, Some(score)) if score >= 500 => Some(format!(
            "🟢 Moderate security score ({}). Monitor for changes.",
            score
        )),
        (_, Some(score)) if score >= 300 => Some(format!(
            "⚠️ Security score {} indicates elevated risk.",
            score
        )),
        (_, Some(score)) => Some(format!(
            "🚨 Security score {} indicates critical risk.",
            score
        )),
        _ => None,
    };

    // Use header image as tagline if available
    let tagline = token.header_image_url.clone();
    let description = normalize_optional_text(token.description.clone());

    Json(TokenDetailResponse {
        mint: token.mint.clone(),
        symbol: token.symbol.clone(),
        name: Some(token.name.clone()),
        tagline,
        description,
        logo_url,
        website: primary_website,
        verified: token.security_score.map(|s| s >= 500).unwrap_or(false),
        tags: combined_tags,
        pair_labels: Vec::new(), // Not available in unified Token
        decimals: Some(token.decimals),
        created_at: created_at_ts,
        last_updated: last_updated_ts,
        pair_created_at,
        pair_url: None,      // Not available in unified Token
        boosts_active: None, // Not available in unified Token
        price_sol,
        price_usd,
        price_confidence,
        price_updated_at,
        price_change_h1: token.price_change_h1,
        price_change_h24: token.price_change_h24,
        price_change_periods,
        liquidity_usd: token.liquidity_usd,
        liquidity_base,
        liquidity_quote,
        volume_24h: token.volume_h24,
        volume_periods,
        fdv: token.fdv,
        market_cap: token.market_cap,
        pool_address,
        pool_dex,
        pool_reserves_sol,
        pool_reserves_token,
        txn_periods,
        buys_24h,
        sells_24h,
        net_flow_24h,
        buy_sell_ratio_24h,
        risk_score: security_score,
        rugged,
        mint_authority,
        freeze_authority,
        total_holders,
        top_10_concentration,
        security_risks,
        security_summary,
        websites,
        socials,
        pools: pool_infos,
        has_ohlcv,
        has_pool_price,
        has_open_position,
        blacklisted,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Get token OHLCV data
async fn get_token_ohlcv(
    Path(mint): Path<String>,
    Query(query): Query<OhlcvQuery>,
) -> Result<Json<Vec<OhlcvPoint>>, StatusCode> {
    let normalized_tf = query.timeframe.trim().to_ascii_lowercase();
    let timeframe = match crate::ohlcvs::Timeframe::from_str(normalized_tf.as_str()) {
        Some(tf) => tf,
        None => {
            logger::info(
                LogTag::Webserver,
                &format!("mint={} timeframe={} (fallback=1m)", mint, query.timeframe),
            );
            crate::ohlcvs::Timeframe::Minute1
        }
    };

    logger::info(
        LogTag::Webserver,
        &format!(
            "mint={} limit={} timeframe={}",
            mint, query.limit, timeframe
        ),
    );

    // Add token to OHLCV monitoring with appropriate priority
    // This ensures data collection starts when a user views the chart
    let is_open_position = positions::is_open_position(&mint).await;
    let priority = if is_open_position {
        crate::ohlcvs::Priority::Critical
    } else {
        crate::ohlcvs::Priority::High // User is viewing chart, high interest
    };

    if let Err(e) = crate::ohlcvs::add_token_monitoring(&mint, priority).await {
        logger::info(
            LogTag::Webserver,
            &format!("Failed to add {} to OHLCV monitoring: {}", mint, e),
        );
    }

    // Record chart view activity (stronger signal than just viewing token)
    if let Err(e) =
        crate::ohlcvs::record_activity(&mint, crate::ohlcvs::ActivityType::ChartViewed).await
    {
        logger::info(
            LogTag::Webserver,
            &format!("Failed to record chart view for {}: {}", mint, e),
        );
    }

    // Fetch OHLCV data using new API - return empty array if no data available
    let data = match crate::ohlcvs::get_ohlcv_data(
        &mint,
        timeframe,
        None,
        query.limit as usize,
        None,
        None,
    )
    .await
    {
        Ok(data) => data,
        Err(e) => {
            logger::debug(
                LogTag::Webserver,
                &format!("mint={} timeframe={} no_data error={}", mint, timeframe, e),
            );
            // Return empty array for tokens without OHLCV data yet
            Vec::new()
        }
    };

    let points: Vec<OhlcvPoint> = data
        .iter()
        .map(|d| OhlcvPoint {
            timestamp: d.timestamp,
            open: d.open,
            high: d.high,
            low: d.low,
            close: d.close,
            volume: d.volume,
        })
        .collect();

    Ok(Json(points))
}

/// Force refresh OHLCV data (immediate fetch outside scheduled monitoring)
async fn refresh_token_ohlcv(
    Path(mint): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    logger::debug(
        LogTag::Webserver,
        &format!("OHLCV refresh requested for mint={}", mint),
    );

    // First, ensure token is being monitored (add if not already)
    let is_open_position = positions::is_open_position(&mint).await;
    let priority = if is_open_position {
        crate::ohlcvs::Priority::Critical
    } else {
        crate::ohlcvs::Priority::High
    };

    // Add to monitoring (idempotent - no-op if already monitored)
    let _ = crate::ohlcvs::add_token_monitoring(&mint, priority).await;

    // Record activity
    let _ = crate::ohlcvs::record_activity(&mint, crate::ohlcvs::ActivityType::DataRequested).await;

    // Try to refresh - but don't fail if no pools available yet
    match crate::ohlcvs::request_refresh(&mint).await {
        Ok(_) => {
            logger::info(
                LogTag::Webserver,
                &format!("mint={} ohlcv_refresh_success", mint),
            );
            Ok(Json(serde_json::json!({
                "success": true,
                "mint": mint,
                "message": "OHLCV refresh triggered",
            })))
        }
        Err(e) => {
            // Log as debug, not warning - this is normal for new tokens without pools
            logger::debug(
                LogTag::Webserver,
                &format!("mint={} ohlcv_refresh_deferred error={}", mint, e),
            );
            // Return success anyway - monitoring is active, data will come when pools are available
            Ok(Json(serde_json::json!({
                "success": true,
                "mint": mint,
                "message": "OHLCV monitoring active, data pending pool availability",
            })))
        }
    }
}

/// Get DexScreener data for a token
async fn get_token_dexscreener(
    Path(mint): Path<String>,
) -> Result<Json<crate::tokens::DexScreenerData>, StatusCode> {
    logger::debug(LogTag::Webserver, &format!("mint={}", mint));

    // Get DexScreener data from token database
    let mint_clone = mint.clone();
    let data = tokio::task::spawn_blocking(move || {
        let db = crate::tokens::get_global_database()
            .ok_or_else(|| "Token database not initialized".to_string())?;
        db.get_dexscreener_data(&mint_clone)
            .map_err(|e| format!("Database error: {}", e))
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match data {
        Some(dexscreener_data) => {
            logger::info(
                LogTag::Webserver,
                &format!(
                    "mint={} price_sol={:.9} liquidity_usd={} fetched={}",
                    mint,
                    dexscreener_data.price_sol,
                    dexscreener_data
                        .liquidity_usd
                        .map_or("N/A".to_string(), |v| format!("{:.2}", v)),
                    dexscreener_data
                        .market_data_last_fetched_at
                        .format("%Y-%m-%d %H:%M:%S")
                ),
            );
            Ok(Json(dexscreener_data))
        }
        None => {
            logger::info(LogTag::Webserver, &format!("mint={} not found", mint));
            Err(StatusCode::NOT_FOUND)
        }
    }
}

/// Get token statistics
async fn get_tokens_stats() -> Result<Json<TokenStatsResponse>, StatusCode> {
    match filtering::fetch_stats().await {
        Ok(snapshot) => {
            logger::info(
                LogTag::Webserver,
                &format!(
                    "total={} pool={} open={} blacklist={}",
                    snapshot.total_tokens,
                    snapshot.with_pool_price,
                    snapshot.open_positions,
                    snapshot.blacklisted
                ),
            );

            Ok(Json(TokenStatsResponse {
                total_tokens: snapshot.total_tokens,
                with_pool_price: snapshot.with_pool_price,
                open_positions: snapshot.open_positions,
                blacklisted: snapshot.blacklisted,
                with_ohlcv: snapshot.with_ohlcv,
                timestamp: snapshot.updated_at.to_rfc3339(),
            }))
        }
        Err(err) => {
            logger::info(
                LogTag::Webserver,
                &format!("Failed to load token stats via filtering service: {}", err),
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// Filter tokens with advanced criteria
async fn filter_tokens(
    Json(filter): Json<FilterRequest>,
) -> Result<Json<TokenListResponse>, StatusCode> {
    logger::info(
        LogTag::Webserver,
        &format!("view={} search='{}'", filter.view, filter.search),
    );

    let max_page_size = MAX_PAGE_SIZE;
    let view = FilteringView::from_str(&filter.view);
    let filtering_query = filter.into_filtering_query(max_page_size);

    match filtering::query_tokens(filtering_query).await {
        Ok(result) => Ok(Json(build_token_list_response(result, view))),
        Err(err) => {
            logger::info(
                LogTag::Webserver,
                &format!("Filtering query failed: {}", err),
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
