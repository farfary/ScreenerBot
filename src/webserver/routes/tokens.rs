use axum::{ extract::{ Path, Query }, http::StatusCode, routing::{ get, post }, Json, Router };
use futures::stream::{ self, StreamExt };
use serde::{ Deserialize, Serialize };
use std::{ collections::{ HashMap, HashSet }, sync::Arc };

use crate::{
    logger::{ log, LogTag },
    pools,
    positions,
    tokens::{ blacklist, cache::TokenDatabase, security_db::SecurityDatabase },
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
        .route("/tokens/list", get(get_tokens_list))
        .route("/tokens/stats", get(get_tokens_stats))
        .route("/tokens/filter", post(filter_tokens))
        .route("/tokens/:mint", get(get_token_detail))
        .route("/tokens/:mint/ohlcv", get(get_token_ohlcv))
}

// =============================================================================
// HANDLERS
// =============================================================================

/// List tokens with simple query params and view selection
/// GET /api/tokens/list?view=pool|all|blacklisted|positions|secure|recent&search=...&sort_by=...&sort_dir=...&page=1&page_size=50
async fn get_tokens_list(Query(query): Query<TokenListQuery>) -> Json<TokenListResponse> {
    use crate::config::with_config;

    // Config-driven limits
    let max_page_size = with_config(|cfg| cfg.webserver.tokens_tab.max_page_size);
    let page_size = query.page_size.min(max_page_size).max(1);
    let page = query.page.max(1);

    // Optimized path for view=all when sorting on inexpensive fields
    let cheap_sort = matches!(
        query.sort_by.as_str(),
        "symbol" |
            "liquidity_usd" |
            "volume_24h" |
            "fdv" |
            "market_cap" |
            "price_change_h1" |
            "price_change_h24"
    );

    if query.view == "all" && cheap_sort {
        // Load raw tokens
        let db = match TokenDatabase::new() {
            Ok(db) => db,
            Err(_) => {
                return Json(TokenListResponse {
                    items: vec![],
                    page,
                    page_size,
                    total: 0,
                    total_pages: 0,
                    timestamp: chrono::Utc::now().to_rfc3339(),
                });
            }
        };
        let mut raw = db.get_all_tokens().await.unwrap_or_default();

        // Search on raw fields
        if !query.search.is_empty() {
            let q = query.search.to_lowercase();
            raw.retain(|t| {
                t.symbol.to_lowercase().contains(&q) ||
                    t.mint.to_lowercase().contains(&q) ||
                    t.name.to_lowercase().contains(&q)
            });
        }

        // Sort raw by selected key
        let ascending = query.sort_dir == "asc";
        raw.sort_by(|a, b| {
            let cmp = match query.sort_by.as_str() {
                "symbol" => a.symbol.cmp(&b.symbol),
                "liquidity_usd" =>
                    optf(a.liquidity.as_ref().and_then(|l| l.usd))
                        .partial_cmp(&optf(b.liquidity.as_ref().and_then(|l| l.usd)))
                        .unwrap_or(std::cmp::Ordering::Equal),
                "volume_24h" =>
                    optf(a.volume.as_ref().and_then(|v| v.h24))
                        .partial_cmp(&optf(b.volume.as_ref().and_then(|v| v.h24)))
                        .unwrap_or(std::cmp::Ordering::Equal),
                "fdv" => optf(a.fdv).partial_cmp(&optf(b.fdv)).unwrap_or(std::cmp::Ordering::Equal),
                "market_cap" =>
                    optf(a.market_cap)
                        .partial_cmp(&optf(b.market_cap))
                        .unwrap_or(std::cmp::Ordering::Equal),
                "price_change_h1" =>
                    optf(a.price_change.as_ref().and_then(|p| p.h1))
                        .partial_cmp(&optf(b.price_change.as_ref().and_then(|p| p.h1)))
                        .unwrap_or(std::cmp::Ordering::Equal),
                "price_change_h24" =>
                    optf(a.price_change.as_ref().and_then(|p| p.h24))
                        .partial_cmp(&optf(b.price_change.as_ref().and_then(|p| p.h24)))
                        .unwrap_or(std::cmp::Ordering::Equal),
                _ => std::cmp::Ordering::Equal,
            };
            if ascending {
                cmp
            } else {
                cmp.reverse()
            }
        });

        // Paginate raw
        let total = raw.len();
        let total_pages = (total + page_size - 1) / page_size;
        let start_idx = (page - 1) * page_size;
        let end_idx = (start_idx + page_size).min(total);
        let page_slice: Vec<crate::tokens::types::ApiToken> = if start_idx < total {
            raw[start_idx..end_idx].to_vec()
        } else {
            vec![]
        };

        // Build caches only for the current page
        let mints: Vec<String> = page_slice
            .iter()
            .map(|t| t.mint.clone())
            .collect();
        let caches = TokenSummaryCaches::build(&mints).await;

        let items = page_slice
            .into_iter()
            .map(|t| token_to_summary(t, &caches))
            .collect();

        log(
            LogTag::Api,
            "TOKENS_LIST_OPTIMIZED",
            &format!(
                "view=all search='{}' sort_by={} sort_dir={} page={}/{} items={}/{}",
                query.search,
                query.sort_by,
                query.sort_dir,
                page,
                total_pages,
                page_size.min(total),
                total
            )
        );

        return Json(TokenListResponse {
            items,
            page,
            page_size,
            total,
            total_pages,
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
    }

    // Default generic path (handles pool/secure/positions/blacklisted/recent and expensive sorts)
    let tokens = get_tokens_for_view(&query.view).await.unwrap_or_default();

    // Search filter (sync)
    let filtered: Vec<TokenSummary> = if query.search.is_empty() {
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

    // Sort (sync)
    let mut sorted = filtered;
    sort_tokens(&mut sorted, &query.sort_by, &query.sort_dir);

    // Pagination (sync)
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

    Json(TokenListResponse {
        items,
        page,
        page_size,
        total,
        total_pages,
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
    let has_ohlcv = match crate::ohlcvs::has_data(&mint).await {
        Ok(flag) => flag,
        Err(e) => {
            log(
                LogTag::Api,
                "WARN",
                &format!("Failed to determine OHLCV availability for {}: {}", mint, e)
            );
            false
        }
    };

    let has_pool_price = price_sol.is_some();
    let blacklisted = blacklist::is_token_blacklisted_db(&mint);

    // Position check - this is the ONLY additional async, keep it last and simple
    let has_open_position = positions::is_open_position(&mint).await;

    // Add token to OHLCV monitoring with appropriate priority
    // This ensures chart data will be available when users view this token again
    let priority = if has_open_position {
        crate::ohlcvs::Priority::Critical
    } else {
        crate::ohlcvs::Priority::Medium // User is viewing, so medium priority
    };

    if let Err(e) = crate::ohlcvs::add_token_monitoring(&mint, priority).await {
        log(LogTag::Api, "WARN", &format!("Failed to add {} to OHLCV monitoring: {}", mint, e));
    }

    // Record view activity
    if
        let Err(e) = crate::ohlcvs::record_activity(
            &mint,
            crate::ohlcvs::ActivityType::TokenViewed
        ).await
    {
        log(LogTag::Api, "WARN", &format!("Failed to record token view for {}: {}", mint, e));
    }

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

    // Add token to OHLCV monitoring with appropriate priority
    // This ensures data collection starts when a user views the chart
    let is_open_position = positions::is_open_position(&mint).await;
    let priority = if is_open_position {
        crate::ohlcvs::Priority::Critical
    } else {
        crate::ohlcvs::Priority::High // User is viewing chart, high interest
    };

    if let Err(e) = crate::ohlcvs::add_token_monitoring(&mint, priority).await {
        log(LogTag::Api, "WARN", &format!("Failed to add {} to OHLCV monitoring: {}", mint, e));
    }

    // Record chart view activity (stronger signal than just viewing token)
    if
        let Err(e) = crate::ohlcvs::record_activity(
            &mint,
            crate::ohlcvs::ActivityType::ChartViewed
        ).await
    {
        log(LogTag::Api, "WARN", &format!("Failed to record chart view for {}: {}", mint, e));
    }

    // Fetch OHLCV data using new API
    let data = crate::ohlcvs
        ::get_ohlcv_data(
            &mint,
            crate::ohlcvs::Timeframe::Minute1,
            None,
            query.limit as usize,
            None,
            None
        ).await
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
    let ohlcv_metrics = crate::ohlcvs::get_metrics().await;
    let with_ohlcv = ohlcv_metrics.tokens_monitored;

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
    let all_tokens = db.get_all_tokens().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mints: Vec<String> = all_tokens
        .iter()
        .map(|token| token.mint.clone())
        .collect();
    let caches = TokenSummaryCaches::build(&mints).await;

    // Convert to TokenSummary and apply filters
    let mut tokens = Vec::new();
    for token in all_tokens {
        let mut summary = token_to_summary(token, &caches);

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
// HELPER FUNCTIONS
// =============================================================================

#[derive(Debug, Default)]
struct TokenSummaryCaches {
    security: HashMap<String, SecuritySnapshot>,
    ohlcv: HashSet<String>,
    open_positions: HashSet<String>,
    blacklisted: HashSet<String>,
}

#[derive(Debug, Clone, Copy)]
struct SecuritySnapshot {
    score: i32,
    rugged: bool,
}

impl TokenSummaryCaches {
    async fn build(mints: &[String]) -> Self {
        let unique_mints: HashSet<String> = mints.iter().cloned().collect();

        let open_positions = positions
            ::get_open_positions().await
            .into_iter()
            .map(|pos| pos.mint)
            .collect::<HashSet<_>>();

        let blacklisted = blacklist::get_blacklisted_mints().into_iter().collect::<HashSet<_>>();

        let security = load_security_snapshots(&unique_mints);
        let ohlcv = load_ohlcv_flags(&unique_mints).await;

        Self {
            security,
            ohlcv,
            open_positions,
            blacklisted,
        }
    }

    fn security_snapshot(&self, mint: &str) -> Option<&SecuritySnapshot> {
        self.security.get(mint)
    }

    fn has_ohlcv(&self, mint: &str) -> bool {
        self.ohlcv.contains(mint)
    }

    fn has_open_position(&self, mint: &str) -> bool {
        self.open_positions.contains(mint)
    }

    fn is_blacklisted(&self, mint: &str) -> bool {
        self.blacklisted.contains(mint)
    }
}

fn load_security_snapshots(mints: &HashSet<String>) -> HashMap<String, SecuritySnapshot> {
    let mut snapshots = HashMap::new();

    if mints.is_empty() {
        return snapshots;
    }

    if let Ok(db) = SecurityDatabase::new("data/security.db") {
        for mint in mints {
            if let Ok(Some(sec)) = db.get_security_info(mint) {
                snapshots.insert(mint.clone(), SecuritySnapshot {
                    score: sec.score,
                    rugged: sec.rugged,
                });
            }
        }
    }

    snapshots
}

async fn load_ohlcv_flags(mints: &HashSet<String>) -> HashSet<String> {
    if mints.is_empty() {
        return HashSet::new();
    }

    stream
        ::iter(mints.iter().cloned())
        .map(|mint| async move {
            let has_data = crate::ohlcvs::has_data(&mint).await.unwrap_or(false);
            (mint, has_data)
        })
        .buffer_unordered(8)
        .filter_map(|(mint, has_data)| async move {
            if has_data { Some(mint) } else { None }
        })
        .collect::<HashSet<_>>().await
}

#[inline]
fn optf(v: Option<f64>) -> f64 {
    v.unwrap_or(0.0)
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
    let mut raw_tokens = Vec::new();

    for mint in available_mints {
        if let Ok(Some(token)) = db.get_token_by_mint(&mint) {
            raw_tokens.push(token);
        }
    }

    let mints: Vec<String> = raw_tokens
        .iter()
        .map(|token| token.mint.clone())
        .collect();
    let caches = TokenSummaryCaches::build(&mints).await;

    Ok(
        raw_tokens
            .into_iter()
            .map(|token| token_to_summary(token, &caches))
            .collect()
    )
}

/// Get all tokens from database
async fn get_all_tokens_from_db() -> Result<Vec<TokenSummary>, String> {
    let db = TokenDatabase::new().map_err(|e| e.to_string())?;
    let tokens = db.get_all_tokens().await?;
    let mints: Vec<String> = tokens
        .iter()
        .map(|token| token.mint.clone())
        .collect();
    let caches = TokenSummaryCaches::build(&mints).await;

    Ok(
        tokens
            .into_iter()
            .map(|token| token_to_summary(token, &caches))
            .collect()
    )
}

/// Get blacklisted tokens
async fn get_blacklisted_tokens() -> Result<Vec<TokenSummary>, String> {
    // Fetch blacklist first, then fetch only those tokens from DB for speed
    let blacklisted_mints: Vec<String> = blacklist::get_blacklisted_mints();
    if blacklisted_mints.is_empty() {
        return Ok(Vec::new());
    }

    let db = TokenDatabase::new().map_err(|e| e.to_string())?;
    let tokens = db.get_tokens_by_mints(&blacklisted_mints).await.map_err(|e| e.to_string())?;

    // Build caches scoped to this page's mints only
    let caches = TokenSummaryCaches::build(&blacklisted_mints).await;

    Ok(
        tokens
            .into_iter()
            .map(|token| token_to_summary(token, &caches))
            .collect()
    )
}

/// Get tokens with open positions
async fn get_position_tokens() -> Result<Vec<TokenSummary>, String> {
    let db = TokenDatabase::new().map_err(|e| e.to_string())?;
    let open_positions = positions::get_open_positions().await;
    let mut raw_tokens = Vec::new();

    for pos in open_positions {
        if let Ok(Some(token)) = db.get_token_by_mint(&pos.mint) {
            raw_tokens.push(token);
        }
    }

    let mints: Vec<String> = raw_tokens
        .iter()
        .map(|token| token.mint.clone())
        .collect();
    let caches = TokenSummaryCaches::build(&mints).await;

    Ok(
        raw_tokens
            .into_iter()
            .map(|token| token_to_summary(token, &caches))
            .collect()
    )
}

/// Get secure tokens (high security score, not rugged)
async fn get_secure_tokens() -> Result<Vec<TokenSummary>, String> {
    use crate::config::with_config;

    let threshold = with_config(|cfg| cfg.webserver.tokens_tab.secure_token_score_threshold);
    let db = TokenDatabase::new().map_err(|e| e.to_string())?;
    let all_tokens = db.get_all_tokens().await?;
    let mints: Vec<String> = all_tokens
        .iter()
        .map(|token| token.mint.clone())
        .collect();
    let caches = TokenSummaryCaches::build(&mints).await;

    Ok(
        all_tokens
            .into_iter()
            .filter(|token| {
                caches
                    .security_snapshot(&token.mint)
                    .map(|snapshot| snapshot.score > threshold && !snapshot.rugged)
                    .unwrap_or(false)
            })
            .map(|token| token_to_summary(token, &caches))
            .collect()
    )
}

/// Get recently created tokens (configurable lookback period)
async fn get_recent_tokens() -> Result<Vec<TokenSummary>, String> {
    use crate::config::with_config;

    let hours = with_config(|cfg| cfg.webserver.tokens_tab.recent_token_hours);
    let db = TokenDatabase::new().map_err(|e| e.to_string())?;
    let all_tokens = db.get_all_tokens().await?;

    let cutoff = chrono::Utc::now() - chrono::Duration::hours(hours);
    let mut recent_tokens = Vec::new();

    for token in all_tokens {
        // Use pair_created_at if available
        if let Some(created_at) = token.pair_created_at {
            let created_time = chrono::DateTime::from_timestamp(created_at, 0);
            if let Some(ct) = created_time {
                if ct > cutoff {
                    recent_tokens.push(token);
                }
            }
        }
    }

    let mints: Vec<String> = recent_tokens
        .iter()
        .map(|token| token.mint.clone())
        .collect();
    let caches = TokenSummaryCaches::build(&mints).await;

    Ok(
        recent_tokens
            .into_iter()
            .map(|token| token_to_summary(token, &caches))
            .collect()
    )
}

/// Convert ApiToken to TokenSummary with enriched data
fn token_to_summary(
    token: crate::tokens::types::ApiToken,
    caches: &TokenSummaryCaches
) -> TokenSummary {
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

    // Check status flags
    let has_pool_price = price_sol.is_some();
    let has_ohlcv = caches.has_ohlcv(&token.mint);
    let has_open_position = caches.has_open_position(&token.mint);
    let blacklisted = caches.is_blacklisted(&token.mint);

    let (security_score, rugged) = caches
        .security_snapshot(&token.mint)
        .map(|snapshot| (Some(snapshot.score), Some(snapshot.rugged)))
        .unwrap_or((None, None));

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
