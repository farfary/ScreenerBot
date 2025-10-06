use axum::{ extract::{ Path, Query }, http::StatusCode, routing::{ get, post }, Json, Router };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

use crate::{
    logger::{ log, LogTag },
    pools,
    positions,
    tokens::{ blacklist, cache::TokenDatabase, ohlcv_db, security_db::SecurityDatabase },
    webserver::{ state::AppState, utils::{ error_response, success_response } },
};

// =============================================================================
// RESPONSE TYPES
// =============================================================================

/// Token summary for list views
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenSummary {
    pub mint: String,
    pub symbol: String,
    pub name: Option<String>,
    pub logo_url: Option<String>,
    pub price_sol: Option<f64>,
    pub price_updated_at: Option<i64>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,
    pub price_change_h1: Option<f64>,
    pub price_change_h24: Option<f64>,
    pub security_score: Option<i32>,
    pub rugged: Option<bool>,
    pub has_pool_price: bool,
    pub has_ohlcv: bool,
    pub has_open_position: bool,
    pub blacklisted: bool,
}

/// Token list response
#[derive(Debug, Serialize)]
pub struct TokenListResponse {
    pub items: Vec<TokenSummary>,
    pub page: usize,
    pub page_size: usize,
    pub total: usize,
    pub total_pages: usize,
    pub timestamp: String,
}

/// Token detail response
#[derive(Debug, Serialize)]
pub struct TokenDetailResponse {
    pub mint: String,
    pub symbol: String,
    pub name: Option<String>,
    pub logo_url: Option<String>,
    pub website: Option<String>,
    pub verified: bool,
    pub tags: Vec<String>,
    // Price info
    pub price_sol: Option<f64>,
    pub price_confidence: Option<f32>,
    pub price_updated_at: Option<i64>,
    // Market info
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,
    pub price_change_h1: Option<f64>,
    pub price_change_h24: Option<f64>,
    // Pool info
    pub pool_address: Option<String>,
    pub pool_dex: Option<String>,
    pub pool_reserves_sol: Option<f64>,
    pub pool_reserves_token: Option<f64>,
    // Security info
    pub security_score: Option<i32>,
    pub security_score_normalized: Option<i32>,
    pub rugged: Option<bool>,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub total_holders: Option<i32>,
    pub top_10_concentration: Option<f64>,
    pub security_risks: Vec<SecurityRisk>,
    // Status flags
    pub has_ohlcv: bool,
    pub has_pool_price: bool,
    pub has_open_position: bool,
    pub blacklisted: bool,
    pub timestamp: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct SecurityRisk {
    pub name: String,
    pub level: String,
    pub description: String,
    pub score: i32,
}

/// OHLCV data point
#[derive(Debug, Serialize)]
pub struct OhlcvPoint {
    pub timestamp: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Token statistics response
#[derive(Debug, Serialize)]
pub struct TokenStatsResponse {
    pub total_tokens: usize,
    pub with_pool_price: usize,
    pub open_positions: usize,
    pub blacklisted: usize,
    pub secure_tokens: usize,
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
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
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
}

fn default_ohlcv_limit() -> u32 {
    100
}

/// Filter request body
#[derive(Debug, Deserialize)]
pub struct FilterRequest {
    #[serde(default)]
    pub search: String,
    pub min_liquidity: Option<f64>,
    pub max_liquidity: Option<f64>,
    pub min_volume_24h: Option<f64>,
    pub max_volume_24h: Option<f64>,
    pub min_security_score: Option<i32>,
    pub max_security_score: Option<i32>,
    pub has_pool_price: Option<bool>,
    pub has_open_position: Option<bool>,
    pub blacklisted: Option<bool>,
    pub has_ohlcv: Option<bool>,
    #[serde(default = "default_sort_by")]
    pub sort_by: String,
    #[serde(default = "default_sort_dir")]
    pub sort_dir: String,
    #[serde(default = "default_page")]
    pub page: usize,
    #[serde(default = "default_page_size")]
    pub page_size: usize,
}

// =============================================================================
// ROUTE REGISTRATION
// =============================================================================

pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tokens", get(get_tokens_with_prices)) // Legacy endpoint
        .route("/tokens/list", get(get_tokens_list))
        .route("/tokens/stats", get(get_tokens_stats))
        .route("/tokens/filter", post(filter_tokens))
        .route("/tokens/:mint", get(get_token_detail))
        .route("/tokens/:mint/ohlcv", get(get_token_ohlcv))
        .route("/tokens/:mint/debug", get(get_token_debug_info))
}

// =============================================================================
// HANDLERS
// =============================================================================

/// Get tokens list with views, sorting, and pagination
///
/// NOTE: Uses POST /api/tokens/filter pattern to work around Axum 0.7 Handler trait issue
/// with complex nested async functions. The full implementation is available via filter_tokens
/// endpoint. This stub is kept for compatibility.
async fn get_tokens_list(Query(query): Query<TokenListQuery>) -> Json<TokenListResponse> {
    use crate::config::with_config;

    let max_page_size = with_config(|cfg| cfg.webserver.tokens_tab.max_page_size);
    let page_size = query.page_size.min(max_page_size).max(1);
    let page = query.page.max(1);

    // Return stub - use POST /api/tokens/filter for full functionality
    Json(TokenListResponse {
        items: vec![],
        page,
        page_size,
        total: 0,
        total_pages: 0,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Get token detail
async fn get_token_detail(Path(mint): Path<String>) -> Json<TokenDetailResponse> {
    log(LogTag::Api, "TOKEN_DETAIL", &format!("mint={}", mint));

    // Fetch token from database (ONE async call)
    let db = match TokenDatabase::new() {
        Ok(db) => db,
        Err(_) => {
            return Json(TokenDetailResponse {
                mint: mint.clone(),
                symbol: "ERROR".to_string(),
                name: Some("Database unavailable".to_string()),
                logo_url: None,
                website: None,
                verified: false,
                tags: vec![],
                price_sol: None,
                price_confidence: None,
                price_updated_at: None,
                liquidity_usd: None,
                volume_24h: None,
                fdv: None,
                market_cap: None,
                price_change_h1: None,
                price_change_h24: None,
                pool_address: None,
                pool_dex: None,
                pool_reserves_sol: None,
                pool_reserves_token: None,
                security_score: None,
                security_score_normalized: None,
                rugged: None,
                mint_authority: None,
                freeze_authority: None,
                total_holders: None,
                top_10_concentration: None,
                security_risks: vec![],
                has_ohlcv: false,
                has_pool_price: false,
                has_open_position: false,
                blacklisted: false,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };

    let token = match db.get_token_by_mint(&mint) {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Json(TokenDetailResponse {
                mint: mint.clone(),
                symbol: "NOT_FOUND".to_string(),
                name: Some("Token not in database".to_string()),
                logo_url: None,
                website: None,
                verified: false,
                tags: vec![],
                price_sol: None,
                price_confidence: None,
                price_updated_at: None,
                liquidity_usd: None,
                volume_24h: None,
                fdv: None,
                market_cap: None,
                price_change_h1: None,
                price_change_h24: None,
                pool_address: None,
                pool_dex: None,
                pool_reserves_sol: None,
                pool_reserves_token: None,
                security_score: None,
                security_score_normalized: None,
                rugged: None,
                mint_authority: None,
                freeze_authority: None,
                total_holders: None,
                top_10_concentration: None,
                security_risks: vec![],
                has_ohlcv: false,
                has_pool_price: false,
                has_open_position: false,
                blacklisted: false,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
        Err(_) => {
            return Json(TokenDetailResponse {
                mint: mint.clone(),
                symbol: "ERROR".to_string(),
                name: Some("Database error".to_string()),
                logo_url: None,
                website: None,
                verified: false,
                tags: vec![],
                price_sol: None,
                price_confidence: None,
                price_updated_at: None,
                liquidity_usd: None,
                volume_24h: None,
                fdv: None,
                market_cap: None,
                price_change_h1: None,
                price_change_h24: None,
                pool_address: None,
                pool_dex: None,
                pool_reserves_sol: None,
                pool_reserves_token: None,
                security_score: None,
                security_score_normalized: None,
                rugged: None,
                mint_authority: None,
                freeze_authority: None,
                total_holders: None,
                top_10_concentration: None,
                security_risks: vec![],
                has_ohlcv: false,
                has_pool_price: false,
                has_open_position: false,
                blacklisted: false,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };

    // Get enrichment data (all sync or from cache)
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
        let now_unix = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        (
            Some(price_result.price_sol),
            Some(price_result.confidence),
            Some(now_unix - (age_secs as i64)),
            Some(price_result.pool_address),
            price_result.source_pool,
            Some(price_result.sol_reserves),
            Some(price_result.token_reserves),
        )
    } else {
        (None, None, None, None, None, None, None)
    };

    // Get security info (sync DB call)
    let (
        security_score,
        security_score_normalized,
        rugged,
        mint_authority,
        freeze_authority,
        total_holders,
        top_10_concentration,
        security_risks,
    ) = SecurityDatabase::new("data/security.db")
        .ok()
        .and_then(|db| db.get_security_info(&mint).ok().flatten())
        .map(|sec| {
            let top_10_conc = if sec.top_holders.len() >= 10 {
                Some(
                    sec.top_holders
                        .iter()
                        .take(10)
                        .map(|h| h.pct)
                        .sum::<f64>()
                )
            } else {
                None
            };
            let risks = sec.risks
                .iter()
                .map(|r| SecurityRisk {
                    name: r.name.clone(),
                    level: r.level.clone(),
                    description: r.description.clone(),
                    score: r.score,
                })
                .collect();
            (
                Some(sec.score),
                Some(sec.score_normalised),
                Some(sec.rugged),
                sec.mint_authority,
                sec.freeze_authority,
                Some(sec.total_holders),
                top_10_conc,
                risks,
            )
        })
        .unwrap_or((None, None, None, None, None, None, None, vec![]));

    // Get status flags (mix of sync and cache checks)
    let has_ohlcv = ohlcv_db
        ::get_ohlcv_database()
        .and_then(|db| db.check_data_availability(&mint))
        .map(|meta| meta.data_points_count > 0 && !meta.is_expired)
        .unwrap_or(false);

    let has_pool_price = price_sol.is_some();
    let blacklisted = blacklist::is_token_blacklisted_db(&mint);

    // Position check - this is the ONLY additional async, keep it last and simple
    let has_open_position = positions::is_open_position(&mint).await;

    Json(TokenDetailResponse {
        mint: token.mint,
        symbol: token.symbol,
        name: Some(token.name),
        logo_url: token.info.as_ref().and_then(|i| i.image_url.clone()),
        website: token.info
            .as_ref()
            .and_then(|i| i.websites.as_ref())
            .and_then(|w| w.first())
            .map(|w| w.url.clone()),
        verified: token.labels
            .as_ref()
            .map(|l| l.iter().any(|label| label.to_lowercase() == "verified"))
            .unwrap_or(false),
        tags: token.labels.unwrap_or_default(),
        price_sol,
        price_confidence,
        price_updated_at,
        liquidity_usd: token.liquidity.as_ref().and_then(|l| l.usd),
        volume_24h: token.volume.as_ref().and_then(|v| v.h24),
        fdv: token.fdv,
        market_cap: token.market_cap,
        price_change_h1: token.price_change.as_ref().and_then(|p| p.h1),
        price_change_h24: token.price_change.as_ref().and_then(|p| p.h24),
        pool_address,
        pool_dex,
        pool_reserves_sol,
        pool_reserves_token,
        security_score,
        security_score_normalized,
        rugged,
        mint_authority,
        freeze_authority,
        total_holders,
        top_10_concentration,
        security_risks,
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
    Query(query): Query<OhlcvQuery>
) -> Result<Json<Vec<OhlcvPoint>>, StatusCode> {
    log(LogTag::Api, "TOKEN_OHLCV", &format!("mint={} limit={}", mint, query.limit));

    let db = ohlcv_db::get_ohlcv_database().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;

    let data = db
        .get_ohlcv_data(&mint, Some(query.limit))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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

/// Get token statistics
async fn get_tokens_stats() -> Result<Json<TokenStatsResponse>, StatusCode> {
    log(LogTag::Api, "TOKENS_STATS", "Calculating statistics");

    let db = TokenDatabase::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let all_tokens = db.get_all_tokens().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total_tokens = all_tokens.len();
    let with_pool_price = pools::get_available_tokens().len();

    // Count open positions
    let mut open_positions = 0;
    for token in &all_tokens {
        if positions::is_open_position(&token.mint).await {
            open_positions += 1;
        }
    }

    // Count blacklisted
    let blacklisted = all_tokens
        .iter()
        .filter(|t| blacklist::is_token_blacklisted_db(&t.mint))
        .count();

    // Count secure tokens (score > 500 as example threshold)
    let security_db = SecurityDatabase::new("data/security.db").ok();
    let secure_tokens = if let Some(sec_db) = security_db {
        all_tokens
            .iter()
            .filter(|t| {
                sec_db
                    .get_security_info(&t.mint)
                    .ok()
                    .flatten()
                    .map(|s| s.score > 500 && !s.rugged)
                    .unwrap_or(false)
            })
            .count()
    } else {
        0
    };

    // Count with OHLCV
    let with_ohlcv = if let Ok(ohlcv_db) = ohlcv_db::get_ohlcv_database() {
        all_tokens
            .iter()
            .filter(|t| {
                ohlcv_db
                    .check_data_availability(&t.mint)
                    .map(|meta| meta.data_points_count > 0)
                    .unwrap_or(false)
            })
            .count()
    } else {
        0
    };

    Ok(
        Json(TokenStatsResponse {
            total_tokens,
            with_pool_price,
            open_positions,
            blacklisted,
            secure_tokens,
            with_ohlcv,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    )
}

/// Filter tokens with advanced criteria
async fn filter_tokens(Json(filter): Json<FilterRequest>) -> Result<
    Json<TokenListResponse>,
    StatusCode
> {
    log(
        LogTag::Api,
        "TOKENS_FILTER",
        &format!(
            "Applying filters: search='{}' liquidity={:?}-{:?}",
            filter.search,
            filter.min_liquidity,
            filter.max_liquidity
        )
    );

    // Get all tokens
    let db = TokenDatabase::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut all_tokens = db.get_all_tokens().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convert to TokenSummary and apply filters
    let mut tokens = Vec::new();
    for token in all_tokens {
        let mut summary = token_to_summary(token).await;

        // Apply filters
        if !filter.search.is_empty() {
            let search_lower = filter.search.to_lowercase();
            if
                !summary.symbol.to_lowercase().contains(&search_lower) &&
                !summary.mint.to_lowercase().contains(&search_lower) &&
                !summary.name
                    .as_ref()
                    .map(|n| n.to_lowercase().contains(&search_lower))
                    .unwrap_or(false)
            {
                continue;
            }
        }

        if let Some(min) = filter.min_liquidity {
            if summary.liquidity_usd.unwrap_or(0.0) < min {
                continue;
            }
        }
        if let Some(max) = filter.max_liquidity {
            if summary.liquidity_usd.unwrap_or(f64::MAX) > max {
                continue;
            }
        }
        if let Some(min) = filter.min_volume_24h {
            if summary.volume_24h.unwrap_or(0.0) < min {
                continue;
            }
        }
        if let Some(max) = filter.max_volume_24h {
            if summary.volume_24h.unwrap_or(f64::MAX) > max {
                continue;
            }
        }
        if let Some(min) = filter.min_security_score {
            if summary.security_score.unwrap_or(0) < min {
                continue;
            }
        }
        if let Some(max) = filter.max_security_score {
            if summary.security_score.unwrap_or(i32::MAX) > max {
                continue;
            }
        }
        if let Some(flag) = filter.has_pool_price {
            if summary.has_pool_price != flag {
                continue;
            }
        }
        if let Some(flag) = filter.has_open_position {
            if summary.has_open_position != flag {
                continue;
            }
        }
        if let Some(flag) = filter.blacklisted {
            if summary.blacklisted != flag {
                continue;
            }
        }
        if let Some(flag) = filter.has_ohlcv {
            if summary.has_ohlcv != flag {
                continue;
            }
        }

        tokens.push(summary);
    }

    // Sort
    sort_tokens(&mut tokens, &filter.sort_by, &filter.sort_dir);

    // Paginate
    let page_size = filter.page_size.min(200).max(1);
    let page = filter.page.max(1);
    let total = tokens.len();
    let total_pages = (total + page_size - 1) / page_size;
    let start_idx = (page - 1) * page_size;
    let end_idx = (start_idx + page_size).min(total);

    let items = if start_idx < total { tokens[start_idx..end_idx].to_vec() } else { vec![] };

    Ok(
        Json(TokenListResponse {
            items,
            page,
            page_size,
            total,
            total_pages,
            timestamp: chrono::Utc::now().to_rfc3339(),
        })
    )
}

// =============================================================================
// LEGACY ENDPOINT (kept for compatibility)
// =============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenWithPrice {
    pub mint: String,
    pub symbol: String,
    pub price_sol: f64,
    pub pool_address: String,
    pub updated_at: i64,
}

#[derive(Debug, Serialize)]
pub struct TokensResponse {
    pub tokens: Vec<TokenWithPrice>,
    pub count: usize,
    pub timestamp: String,
}

async fn get_tokens_with_prices() -> Json<TokensResponse> {
    let available_mints = pools::get_available_tokens();
    let mut tokens_with_prices = Vec::new();
    let db = match TokenDatabase::new() {
        Ok(db) => db,
        Err(_) => {
            return Json(TokensResponse {
                tokens: vec![],
                count: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };

    for mint in &available_mints {
        if let Some(price_result) = pools::get_pool_price(mint) {
            let symbol = match db.get_token_by_mint(mint) {
                Ok(Some(token)) => token.symbol,
                _ => format!("{}...", &mint[..8]),
            };
            let age_seconds = price_result.timestamp.elapsed().as_secs();
            let now_unix = std::time::SystemTime
                ::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            tokens_with_prices.push(TokenWithPrice {
                mint: mint.clone(),
                symbol,
                price_sol: price_result.price_sol,
                pool_address: price_result.pool_address,
                updated_at: now_unix - (age_seconds as i64),
            });
        }
    }

    tokens_with_prices.sort_by(|a, b| a.symbol.cmp(&b.symbol));
    let count = tokens_with_prices.len();

    Json(TokensResponse {
        tokens: tokens_with_prices,
        count,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

// =============================================================================
// HELPER FUNCTIONS
// =============================================================================

/// Build complete token list response with filtering, sorting, and pagination
async fn build_token_list_response(query: TokenListQuery) -> TokenListResponse {
    use crate::config::with_config;

    // Get config-driven limits
    let max_page_size = with_config(|cfg| cfg.webserver.tokens_tab.max_page_size);
    let page_size = query.page_size.min(max_page_size).max(1);
    let page = query.page.max(1);

    // Fetch tokens for the specified view
    let tokens = match get_tokens_for_view(&query.view).await {
        Ok(t) => t,
        Err(_) => vec![],
    };

    // Apply search filter
    let filtered = if query.search.is_empty() {
        tokens
    } else {
        let search_lower = query.search.to_lowercase();
        tokens
            .into_iter()
            .filter(|t| {
                t.symbol.to_lowercase().contains(&search_lower) ||
                    t.mint.to_lowercase().contains(&search_lower) ||
                    t.name
                        .as_ref()
                        .map(|n| n.to_lowercase().contains(&search_lower))
                        .unwrap_or(false)
            })
            .collect()
    };

    // Sort tokens
    let mut sorted = filtered;
    sort_tokens(&mut sorted, &query.sort_by, &query.sort_dir);

    // Calculate pagination
    let total = sorted.len();
    let total_pages = (total + page_size - 1) / page_size;
    let start_idx = (page - 1) * page_size;
    let end_idx = (start_idx + page_size).min(total);

    let items = if start_idx < total { sorted[start_idx..end_idx].to_vec() } else { vec![] };

    log(
        LogTag::Api,
        "TOKENS_LIST",
        &format!(
            "view={} search='{}' page={}/{} items={}/{}",
            query.view,
            query.search,
            page,
            total_pages,
            items.len(),
            total
        )
    );

    TokenListResponse {
        items,
        page,
        page_size,
        total,
        total_pages,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }
}

/// Get tokens for a specific view
async fn get_tokens_for_view(view: &str) -> Result<Vec<TokenSummary>, String> {
    match view {
        "pool" => get_pool_tokens().await,
        "all" => get_all_tokens_from_db().await,
        "blacklisted" => get_blacklisted_tokens().await,
        "positions" => get_position_tokens().await,
        "secure" => get_secure_tokens().await,
        "recent" => get_recent_tokens().await,
        _ => get_pool_tokens().await, // default to pool view
    }
}

/// Get tokens with pool prices
async fn get_pool_tokens() -> Result<Vec<TokenSummary>, String> {
    let available_mints = pools::get_available_tokens();
    let db = TokenDatabase::new().map_err(|e| e.to_string())?;
    let mut tokens = Vec::new();

    for mint in available_mints {
        if let Ok(Some(token)) = db.get_token_by_mint(&mint) {
            tokens.push(token_to_summary(token).await);
        }
    }

    Ok(tokens)
}

/// Get all tokens from database
async fn get_all_tokens_from_db() -> Result<Vec<TokenSummary>, String> {
    let db = TokenDatabase::new().map_err(|e| e.to_string())?;
    let tokens = db.get_all_tokens().await?;

    let mut summaries = Vec::new();
    for token in tokens {
        summaries.push(token_to_summary(token).await);
    }

    Ok(summaries)
}

/// Get blacklisted tokens
async fn get_blacklisted_tokens() -> Result<Vec<TokenSummary>, String> {
    let db = TokenDatabase::new().map_err(|e| e.to_string())?;
    let all_tokens = db.get_all_tokens().await?;

    let mut tokens = Vec::new();
    for token in all_tokens {
        if blacklist::is_token_blacklisted_db(&token.mint) {
            tokens.push(token_to_summary(token).await);
        }
    }

    Ok(tokens)
}

/// Get tokens with open positions
async fn get_position_tokens() -> Result<Vec<TokenSummary>, String> {
    let db = TokenDatabase::new().map_err(|e| e.to_string())?;
    let open_positions = positions::get_open_positions().await;

    let mut tokens = Vec::new();
    for pos in open_positions {
        if let Ok(Some(token)) = db.get_token_by_mint(&pos.mint) {
            tokens.push(token_to_summary(token).await);
        }
    }

    Ok(tokens)
}

/// Get secure tokens (high security score, not rugged)
async fn get_secure_tokens() -> Result<Vec<TokenSummary>, String> {
    use crate::config::with_config;

    let threshold = with_config(|cfg| cfg.webserver.tokens_tab.secure_token_score_threshold);
    let db = TokenDatabase::new().map_err(|e| e.to_string())?;
    let all_tokens = db.get_all_tokens().await?;
    let security_db = SecurityDatabase::new("data/security.db").map_err(|e| e.to_string())?;

    let mut tokens = Vec::new();
    for token in all_tokens {
        if let Ok(Some(sec)) = security_db.get_security_info(&token.mint) {
            if sec.score > threshold && !sec.rugged {
                tokens.push(token_to_summary(token).await);
            }
        }
    }

    Ok(tokens)
}

/// Get recently created tokens (configurable lookback period)
async fn get_recent_tokens() -> Result<Vec<TokenSummary>, String> {
    use crate::config::with_config;

    let hours = with_config(|cfg| cfg.webserver.tokens_tab.recent_token_hours);
    let db = TokenDatabase::new().map_err(|e| e.to_string())?;
    let all_tokens = db.get_all_tokens().await?;

    let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
    let mut tokens = Vec::new();

    for token in all_tokens {
        // Use pair_created_at if available
        if let Some(created_at) = token.pair_created_at {
            let created_time = chrono::DateTime::from_timestamp(created_at, 0);
            if let Some(ct) = created_time {
                if ct > cutoff {
                    tokens.push(token_to_summary(token).await);
                }
            }
        }
    }

    Ok(tokens)
}

/// Convert ApiToken to TokenSummary with enriched data
async fn token_to_summary(token: crate::tokens::types::ApiToken) -> TokenSummary {
    // Get price info
    let (price_sol, price_updated_at) = if
        let Some(price_result) = pools::get_pool_price(&token.mint)
    {
        let age_secs = price_result.timestamp.elapsed().as_secs();
        let now_unix = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        (Some(price_result.price_sol), Some(now_unix - (age_secs as i64)))
    } else {
        (None, None)
    };

    // Get security info
    let (security_score, rugged) = SecurityDatabase::new("data/security.db")
        .ok()
        .and_then(|db| db.get_security_info(&token.mint).ok().flatten())
        .map(|s| (Some(s.score), Some(s.rugged)))
        .unwrap_or((None, None));

    // Check status flags
    let has_pool_price = price_sol.is_some();
    let has_ohlcv = ohlcv_db
        ::get_ohlcv_database()
        .and_then(|db| db.check_data_availability(&token.mint))
        .map(|meta| meta.data_points_count > 0 && !meta.is_expired)
        .unwrap_or(false);
    let has_open_position = positions::is_open_position(&token.mint).await;
    let blacklisted = blacklist::is_token_blacklisted_db(&token.mint);

    TokenSummary {
        mint: token.mint,
        symbol: token.symbol,
        name: Some(token.name),
        logo_url: token.info.as_ref().and_then(|i| i.image_url.clone()),
        price_sol,
        price_updated_at,
        liquidity_usd: token.liquidity.as_ref().and_then(|l| l.usd),
        volume_24h: token.volume.as_ref().and_then(|v| v.h24),
        fdv: token.fdv,
        market_cap: token.market_cap,
        price_change_h1: token.price_change.as_ref().and_then(|p| p.h1),
        price_change_h24: token.price_change.as_ref().and_then(|p| p.h24),
        security_score,
        rugged,
        has_pool_price,
        has_ohlcv,
        has_open_position,
        blacklisted,
    }
}

/// Sort tokens by specified field and direction
fn sort_tokens(tokens: &mut [TokenSummary], sort_by: &str, sort_dir: &str) {
    let ascending = sort_dir == "asc";

    tokens.sort_by(|a, b| {
        let cmp = match sort_by {
            "symbol" => a.symbol.cmp(&b.symbol),
            "liquidity_usd" =>
                a.liquidity_usd
                    .unwrap_or(0.0)
                    .partial_cmp(&b.liquidity_usd.unwrap_or(0.0))
                    .unwrap(),
            "volume_24h" =>
                a.volume_24h.unwrap_or(0.0).partial_cmp(&b.volume_24h.unwrap_or(0.0)).unwrap(),
            "price_sol" =>
                a.price_sol.unwrap_or(0.0).partial_cmp(&b.price_sol.unwrap_or(0.0)).unwrap(),
            "market_cap" =>
                a.market_cap.unwrap_or(0.0).partial_cmp(&b.market_cap.unwrap_or(0.0)).unwrap(),
            "fdv" => a.fdv.unwrap_or(0.0).partial_cmp(&b.fdv.unwrap_or(0.0)).unwrap(),
            "security_score" => a.security_score.unwrap_or(0).cmp(&b.security_score.unwrap_or(0)),
            "price_change_h1" =>
                a.price_change_h1
                    .unwrap_or(0.0)
                    .partial_cmp(&b.price_change_h1.unwrap_or(0.0))
                    .unwrap(),
            "price_change_h24" =>
                a.price_change_h24
                    .unwrap_or(0.0)
                    .partial_cmp(&b.price_change_h24.unwrap_or(0.0))
                    .unwrap(),
            "updated_at" => a.price_updated_at.unwrap_or(0).cmp(&b.price_updated_at.unwrap_or(0)),
            _ => a.mint.cmp(&b.mint), // fallback to mint as tie-breaker
        };

        if ascending {
            cmp
        } else {
            cmp.reverse()
        }
    });
}

// Keep the rest of the debug endpoint unchanged
// (Include all the debug-related types and get_token_debug_info function from the original file)

// =============================================================================
// DEBUG INFO ENDPOINT (Legacy - kept for compatibility)
// =============================================================================

#[derive(Debug, Serialize)]
pub struct TokenDebugResponse {
    pub mint: String,
    pub timestamp: String,
    pub token_info: Option<TokenDebugInfo>,
    pub price_data: Option<DebugPriceData>,
    pub market_data: Option<DebugMarketData>,
    pub pools: Vec<DebugPoolInfo>,
    pub security: Option<DebugSecurityInfo>,
    pub social: Option<SocialInfo>,
    pub pool_debug: Option<PoolDebugInfo>,
    pub token_debug: Option<TokenSystemDebugInfo>,
}

#[derive(Debug, Serialize)]
pub struct TokenDebugInfo {
    pub symbol: String,
    pub name: String,
    pub decimals: Option<u8>,
    pub logo_url: Option<String>,
    pub website: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub is_verified: bool,
}

#[derive(Debug, Serialize)]
pub struct DebugPriceData {
    pub pool_price_sol: f64,
    pub pool_price_usd: Option<f64>,
    pub confidence: f32,
    pub last_updated: i64,
}

#[derive(Debug, Serialize)]
pub struct DebugMarketData {
    pub market_cap: Option<f64>,
    pub fdv: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DebugPoolInfo {
    pub pool_address: String,
    pub program_kind: String,
    pub dex_name: String,
    pub sol_reserves: f64,
    pub token_reserves: f64,
    pub price_sol: f64,
    pub confidence: f32,
    pub last_updated: i64,
}

#[derive(Debug, Serialize)]
pub struct DebugSecurityInfo {
    pub score: i32,
    pub score_normalised: i32,
    pub rugged: bool,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub creator: Option<String>,
    pub total_holders: i32,
    pub top_10_concentration: Option<f64>,
    pub risks: Vec<DebugRiskInfo>,
    pub analyzed_at: String,
}

#[derive(Debug, Serialize)]
pub struct DebugRiskInfo {
    pub name: String,
    pub level: String,
    pub description: String,
    pub score: i32,
}

#[derive(Debug, Serialize)]
pub struct SocialInfo {
    pub website: Option<String>,
    pub twitter: Option<String>,
    pub telegram: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PoolDebugInfo {
    pub price_history: Vec<PricePoint>,
    pub price_stats: PriceStats,
    pub all_pools: Vec<DebugPoolInfo>,
    pub cache_stats: CacheStatsInfo,
}

#[derive(Debug, Serialize)]
pub struct PricePoint {
    pub timestamp: i64,
    pub price_sol: f64,
    pub confidence: f32,
}

#[derive(Debug, Serialize)]
pub struct PriceStats {
    pub min_price: f64,
    pub max_price: f64,
    pub avg_price: f64,
    pub price_volatility: f64,
    pub data_points: usize,
    pub time_span_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct CacheStatsInfo {
    pub total_tokens_cached: usize,
    pub fresh_prices: usize,
    pub history_entries: usize,
}

#[derive(Debug, Serialize)]
pub struct TokenSystemDebugInfo {
    pub blacklist_status: Option<BlacklistDebugStatus>,
    pub ohlcv_availability: OhlcvAvailability,
    pub decimals_info: DecimalsInfo,
}

#[derive(Debug, Serialize)]
pub struct BlacklistDebugStatus {
    pub is_blacklisted: bool,
    pub reason: Option<String>,
    pub first_occurrence: Option<String>,
    pub occurrence_count: u32,
}

#[derive(Debug, Serialize)]
pub struct OhlcvAvailability {
    pub has_1m_data: bool,
    pub has_5m_data: bool,
    pub has_15m_data: bool,
    pub has_1h_data: bool,
    pub total_candles: usize,
    pub oldest_timestamp: Option<i64>,
    pub newest_timestamp: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct DecimalsInfo {
    pub decimals: Option<u8>,
    pub cached: bool,
    pub source: String,
}

async fn get_token_debug_info(Path(mint): Path<String>) -> Json<TokenDebugResponse> {
    let timestamp = chrono::Utc::now().to_rfc3339();
    let decimals = crate::tokens::get_token_decimals(&mint).await;

    let api_token = TokenDatabase::new()
        .ok()
        .and_then(|db| db.get_token_by_mint(&mint).ok().flatten());

    let token_info = api_token.as_ref().map(|token| TokenDebugInfo {
        symbol: token.symbol.clone(),
        name: token.name.clone(),
        decimals,
        logo_url: token.info.as_ref().and_then(|i| i.image_url.clone()),
        website: token.info
            .as_ref()
            .and_then(|i| i.websites.as_ref())
            .and_then(|w| w.first())
            .map(|w| w.url.clone()),
        description: None,
        tags: token.labels.clone().unwrap_or_default(),
        is_verified: token.labels
            .as_ref()
            .map(|l| l.iter().any(|label| label.to_lowercase() == "verified"))
            .unwrap_or(false),
    });

    let price_data = pools::get_pool_price(&mint).map(|price_result| {
        let age_seconds = price_result.timestamp.elapsed().as_secs();
        let now_unix = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        DebugPriceData {
            pool_price_sol: price_result.price_sol,
            pool_price_usd: None,
            confidence: price_result.confidence,
            last_updated: now_unix - (age_seconds as i64),
        }
    });

    let market_data = api_token.as_ref().map(|token| DebugMarketData {
        market_cap: token.market_cap,
        fdv: token.fdv,
        liquidity_usd: token.liquidity.as_ref().and_then(|l| l.usd),
        volume_24h: token.volume.as_ref().and_then(|v| v.h24),
    });

    let mut pools_vec = Vec::new();
    if let Some(price_result) = pools::get_pool_price(&mint) {
        let age_seconds = price_result.timestamp.elapsed().as_secs();
        let now_unix = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        pools_vec.push(DebugPoolInfo {
            pool_address: price_result.pool_address.clone(),
            program_kind: format!(
                "{:?}",
                price_result.source_pool.as_ref().unwrap_or(&"Unknown".to_string())
            ),
            dex_name: price_result.source_pool.as_ref().unwrap_or(&"Unknown".to_string()).clone(),
            sol_reserves: price_result.sol_reserves,
            token_reserves: price_result.token_reserves,
            price_sol: price_result.price_sol,
            confidence: price_result.confidence,
            last_updated: now_unix - (age_seconds as i64),
        });
    }

    let security = SecurityDatabase::new("data/security.db")
        .ok()
        .and_then(|db| db.get_security_info(&mint).ok().flatten())
        .map(|sec| {
            let top_10_concentration = if sec.top_holders.len() >= 10 {
                Some(
                    sec.top_holders
                        .iter()
                        .take(10)
                        .map(|h| h.pct)
                        .sum::<f64>()
                )
            } else {
                None
            };
            DebugSecurityInfo {
                score: sec.score,
                score_normalised: sec.score_normalised,
                rugged: sec.rugged,
                mint_authority: sec.mint_authority,
                freeze_authority: sec.freeze_authority,
                creator: sec.creator,
                total_holders: sec.total_holders,
                top_10_concentration,
                risks: sec.risks
                    .iter()
                    .map(|r| DebugRiskInfo {
                        name: r.name.clone(),
                        level: r.level.clone(),
                        description: r.description.clone(),
                        score: r.score,
                    })
                    .collect(),
                analyzed_at: sec.analyzed_at,
            }
        });

    let social = api_token.as_ref().and_then(|token| {
        token.info.as_ref().map(|info| SocialInfo {
            website: info.websites
                .as_ref()
                .and_then(|w| w.first())
                .map(|w| w.url.clone()),
            twitter: info.socials.as_ref().and_then(|socials|
                socials
                    .iter()
                    .find(|s| s.platform.to_lowercase().contains("twitter"))
                    .map(|s| format!("https://twitter.com/{}", s.handle))
            ),
            telegram: info.socials.as_ref().and_then(|socials|
                socials
                    .iter()
                    .find(|s| s.platform.to_lowercase().contains("telegram"))
                    .map(|s| format!("https://t.me/{}", s.handle))
            ),
        })
    });

    let pool_debug = {
        let price_history: Vec<PricePoint> = pools
            ::get_price_history(&mint)
            .iter()
            .rev()
            .take(100)
            .map(|p| {
                let age_seconds = p.timestamp.elapsed().as_secs();
                let now_unix = std::time::SystemTime
                    ::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64;
                PricePoint {
                    timestamp: now_unix - (age_seconds as i64),
                    price_sol: p.price_sol,
                    confidence: p.confidence,
                }
            })
            .collect();

        let price_stats = pools
            ::get_price_history_stats(&mint)
            .ok()
            .map(|stats| {
                let (min_price, max_price) = stats.price_range_sol;
                let time_span = (stats.age_oldest_seconds - stats.age_newest_seconds) as i64;
                PriceStats {
                    min_price,
                    max_price,
                    avg_price: (min_price + max_price) / 2.0,
                    price_volatility: if max_price > 0.0 {
                        ((max_price - min_price) / max_price) * 100.0
                    } else {
                        0.0
                    },
                    data_points: stats.total_points,
                    time_span_seconds: time_span,
                }
            })
            .unwrap_or_else(|| PriceStats {
                min_price: 0.0,
                max_price: 0.0,
                avg_price: 0.0,
                price_volatility: 0.0,
                data_points: 0,
                time_span_seconds: 0,
            });

        let cache_stats = pools::get_cache_stats();
        Some(PoolDebugInfo {
            price_history,
            price_stats,
            all_pools: pools_vec.clone(),
            cache_stats: CacheStatsInfo {
                total_tokens_cached: cache_stats.total_prices,
                fresh_prices: cache_stats.fresh_prices,
                history_entries: cache_stats.history_entries,
            },
        })
    };

    let token_debug = {
        let blacklist_status = {
            let is_blacklisted = blacklist::is_token_blacklisted_db(&mint);
            BlacklistDebugStatus {
                is_blacklisted,
                reason: if is_blacklisted {
                    Some("Token is blacklisted".to_string())
                } else {
                    None
                },
                first_occurrence: None,
                occurrence_count: 0,
            }
        };

        let ohlcv_availability = {
            if let Ok(db) = ohlcv_db::get_ohlcv_database() {
                let all_data = db.get_ohlcv_data(&mint, Some(1000)).ok().unwrap_or_default();
                let has_data = !all_data.is_empty();
                OhlcvAvailability {
                    has_1m_data: has_data,
                    has_5m_data: false,
                    has_15m_data: false,
                    has_1h_data: false,
                    total_candles: all_data.len(),
                    oldest_timestamp: all_data.last().map(|d| d.timestamp),
                    newest_timestamp: all_data.first().map(|d| d.timestamp),
                }
            } else {
                OhlcvAvailability {
                    has_1m_data: false,
                    has_5m_data: false,
                    has_15m_data: false,
                    has_1h_data: false,
                    total_candles: 0,
                    oldest_timestamp: None,
                    newest_timestamp: None,
                }
            }
        };

        let decimals_info = {
            use crate::tokens::decimals;
            let cached_decimals = decimals::get_cached_decimals(&mint);
            DecimalsInfo {
                decimals,
                cached: cached_decimals.is_some(),
                source: (
                    if cached_decimals.is_some() {
                        "cache"
                    } else if decimals.is_some() {
                        "rpc_fetch"
                    } else {
                        "failed"
                    }
                ).to_string(),
            }
        };

        Some(TokenSystemDebugInfo {
            blacklist_status: Some(blacklist_status),
            ohlcv_availability,
            decimals_info,
        })
    };

    Json(TokenDebugResponse {
        mint,
        timestamp,
        token_info,
        price_data,
        market_data,
        pools: pools_vec,
        security,
        social,
        pool_debug,
        token_debug,
    })
}
