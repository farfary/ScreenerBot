use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    config::with_config,
    filtering::{self, FilteringQuery, FilteringView, SortDirection, TokenSortKey},
    global::is_debug_webserver_enabled,
    logger::{log, LogTag},
    pools, positions,
    tokens::{blacklist, cache::TokenDatabase, summary::TokenSummary, SecurityDatabase},
    webserver::{
        state::AppState,
        utils::{error_response, success_response},
    },
};

// =============================================================================
// RESPONSE TYPES
// =============================================================================

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
    #[serde(default)]
    pub min_holders: Option<i32>,
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
    pub min_security_score: Option<i32>,
    pub max_security_score: Option<i32>,
    pub min_holders: Option<i32>,
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

fn normalize_search(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

impl TokenListQuery {
    fn into_filtering_query(self, max_page_size: usize) -> FilteringQuery {
        let mut query = FilteringQuery::default();
        query.view = FilteringView::from_str(&self.view);
        query.search = normalize_search(self.search);
        query.sort_key = TokenSortKey::from_str(&self.sort_by);
        query.sort_direction = SortDirection::from_str(&self.sort_dir);
        query.page = self.page.max(1);
        query.page_size = self.page_size.max(1);
        query.min_unique_holders = self.min_holders;
        query.clamp_page_size(max_page_size);
        query
    }
}

impl FilterRequest {
    fn into_filtering_query(self, max_page_size: usize) -> FilteringQuery {
        let mut query = FilteringQuery::default();
        query.view = FilteringView::from_str(&self.view);
        query.search = normalize_search(self.search);
        query.sort_key = TokenSortKey::from_str(&self.sort_by);
        query.sort_direction = SortDirection::from_str(&self.sort_dir);
        query.page = self.page.max(1);
        query.page_size = self.page_size.max(1);
        query.min_liquidity = self.min_liquidity;
        query.max_liquidity = self.max_liquidity;
        query.min_volume_24h = self.min_volume_24h;
        query.max_volume_24h = self.max_volume_24h;
        query.min_security_score = self.min_security_score;
        query.max_security_score = self.max_security_score;
        query.min_unique_holders = self.min_holders;
        query.has_pool_price = self.has_pool_price;
        query.has_open_position = self.has_open_position;
        query.blacklisted = self.blacklisted;
        query.has_ohlcv = self.has_ohlcv;
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
        .route("/tokens/:mint/ohlcv", get(get_token_ohlcv))
}

// =============================================================================
// HANDLERS
// =============================================================================

/// GET /api/tokens/list
///
/// Query: view, search, sort_by, sort_dir, page, page_size
pub(crate) async fn get_tokens_list(
    Query(query): Query<TokenListQuery>,
) -> Json<TokenListResponse> {
    let max_page_size = with_config(|cfg| cfg.webserver.tokens_tab.max_page_size);
    let request_view = query.view.clone();
    let filtering_query = query.into_filtering_query(max_page_size);

    match filtering::query_tokens(filtering_query).await {
        Ok(result) => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "TOKENS_LIST",
                    &format!(
                        "view={} page={}/{} items={}/{}",
                        request_view,
                        result.page,
                        result.total_pages,
                        result.items.len(),
                        result.total
                    ),
                );
            }

            Json(TokenListResponse {
                items: result.items,
                page: result.page,
                page_size: result.page_size,
                total: result.total,
                total_pages: result.total_pages,
                timestamp: result.timestamp.to_rfc3339(),
            })
        }
        Err(err) => {
            log(
                LogTag::Webserver,
                "WARN",
                &format!("Failed to load tokens list via filtering service: {}", err),
            );

            Json(TokenListResponse {
                items: vec![],
                page: 1,
                page_size: max_page_size,
                total: 0,
                total_pages: 0,
                timestamp: chrono::Utc::now().to_rfc3339(),
            })
        }
    }
}

/// Get token detail
async fn get_token_detail(Path(mint): Path<String>) -> Json<TokenDetailResponse> {
    let request_start = std::time::Instant::now();

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "TOKEN_DETAIL_START",
            &format!("mint={}", mint),
        );
    }

    // Fetch token from database using async wrapper (prevents blocking async runtime)
    let db_start = std::time::Instant::now();
    let token = match TokenDatabase::get_token_by_mint_async(&mint).await {
        Ok(Some(t)) => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "TOKEN_DETAIL_DB",
                    &format!("mint={} elapsed={}ms", mint, db_start.elapsed().as_millis()),
                );
            }
            t
        }
        Ok(None) => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "TOKEN_DETAIL_NOT_FOUND",
                    &format!("mint={} elapsed={}ms", mint, db_start.elapsed().as_millis()),
                );
            }
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

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "TOKEN_DETAIL_POOL",
            &format!(
                "mint={} elapsed={}ms has_price={}",
                mint,
                pool_start.elapsed().as_millis(),
                price_sol.is_some()
            ),
        );
    }

    // Get security info using async wrapper (prevents blocking async runtime)
    let security_start = std::time::Instant::now();
    let (
        security_score,
        security_score_normalized,
        rugged,
        mint_authority,
        freeze_authority,
        total_holders,
        top_10_concentration,
        security_risks,
    ) = match SecurityDatabase::get_security_info_async(&mint).await {
        Ok(Some(sec)) => {
            let top_10_conc = if sec.top_holders.len() >= 10 {
                Some(sec.top_holders.iter().take(10).map(|h| h.pct).sum::<f64>())
            } else {
                None
            };
            let risks = sec
                .risks
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
        }
        Ok(None) | Err(_) => (None, None, None, None, None, None, None, vec![]),
    };

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "TOKEN_DETAIL_SECURITY",
            &format!(
                "mint={} elapsed={}ms has_score={}",
                mint,
                security_start.elapsed().as_millis(),
                security_score.is_some()
            ),
        );
    }

    // Get status flags (mix of sync and cache checks)
    let ohlcv_start = std::time::Instant::now();
    let has_ohlcv = match crate::ohlcvs::has_data(&mint).await {
        Ok(flag) => flag,
        Err(e) => {
            log(
                LogTag::Webserver,
                "WARN",
                &format!("Failed to determine OHLCV availability for {}: {}", mint, e),
            );
            false
        }
    };

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "TOKEN_DETAIL_OHLCV",
            &format!(
                "mint={} elapsed={}ms has_data={}",
                mint,
                ohlcv_start.elapsed().as_millis(),
                has_ohlcv
            ),
        );
    }

    let has_pool_price = price_sol.is_some();
    let blacklisted = blacklist::is_token_blacklisted_db(&mint);

    // Position check - this is the ONLY additional async, keep it last and simple
    let position_start = std::time::Instant::now();
    let has_open_position = positions::is_open_position(&mint).await;

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "TOKEN_DETAIL_POSITION",
            &format!(
                "mint={} elapsed={}ms has_position={}",
                mint,
                position_start.elapsed().as_millis(),
                has_open_position
            ),
        );
    }

    // Add token to OHLCV monitoring with appropriate priority
    // This ensures chart data will be available when users view this token again
    let monitoring_start = std::time::Instant::now();
    let priority = if has_open_position {
        crate::ohlcvs::Priority::Critical
    } else {
        crate::ohlcvs::Priority::Medium // User is viewing, so medium priority
    };

    if let Err(e) = crate::ohlcvs::add_token_monitoring(&mint, priority).await {
        log(
            LogTag::Webserver,
            "WARN",
            &format!("Failed to add {} to OHLCV monitoring: {}", mint, e),
        );
    }

    // Record view activity
    if let Err(e) =
        crate::ohlcvs::record_activity(&mint, crate::ohlcvs::ActivityType::TokenViewed).await
    {
        log(
            LogTag::Webserver,
            "WARN",
            &format!("Failed to record token view for {}: {}", mint, e),
        );
    }

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "TOKEN_DETAIL_MONITORING",
            &format!(
                "mint={} elapsed={}ms",
                mint,
                monitoring_start.elapsed().as_millis()
            ),
        );
    }

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "TOKEN_DETAIL_COMPLETE",
            &format!(
                "mint={} total_elapsed={}ms",
                mint,
                request_start.elapsed().as_millis()
            ),
        );
    }

    Json(TokenDetailResponse {
        mint: token.mint,
        symbol: token.symbol,
        name: Some(token.name),
        logo_url: token.info.as_ref().and_then(|i| i.image_url.clone()),
        website: token
            .info
            .as_ref()
            .and_then(|i| i.websites.as_ref())
            .and_then(|w| w.first())
            .map(|w| w.url.clone()),
        verified: token
            .labels
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
    Query(query): Query<OhlcvQuery>,
) -> Result<Json<Vec<OhlcvPoint>>, StatusCode> {
    let normalized_tf = query.timeframe.trim().to_ascii_lowercase();
    let timeframe = match crate::ohlcvs::Timeframe::from_str(normalized_tf.as_str()) {
        Some(tf) => tf,
        None => {
            log(
                LogTag::Webserver,
                "TOKEN_OHLCV_INVALID_TIMEFRAME",
                &format!("mint={} timeframe={} (fallback=1m)", mint, query.timeframe),
            );
            crate::ohlcvs::Timeframe::Minute1
        }
    };

    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "TOKEN_OHLCV",
            &format!(
                "mint={} limit={} timeframe={}",
                mint, query.limit, timeframe
            ),
        );
    }

    // Add token to OHLCV monitoring with appropriate priority
    // This ensures data collection starts when a user views the chart
    let is_open_position = positions::is_open_position(&mint).await;
    let priority = if is_open_position {
        crate::ohlcvs::Priority::Critical
    } else {
        crate::ohlcvs::Priority::High // User is viewing chart, high interest
    };

    if let Err(e) = crate::ohlcvs::add_token_monitoring(&mint, priority).await {
        log(
            LogTag::Webserver,
            "WARN",
            &format!("Failed to add {} to OHLCV monitoring: {}", mint, e),
        );
    }

    // Record chart view activity (stronger signal than just viewing token)
    if let Err(e) =
        crate::ohlcvs::record_activity(&mint, crate::ohlcvs::ActivityType::ChartViewed).await
    {
        log(
            LogTag::Webserver,
            "WARN",
            &format!("Failed to record chart view for {}: {}", mint, e),
        );
    }

    // Fetch OHLCV data using new API
    let data =
        crate::ohlcvs::get_ohlcv_data(&mint, timeframe, None, query.limit as usize, None, None)
            .await
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
    match filtering::fetch_stats().await {
        Ok(snapshot) => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "TOKENS_STATS",
                    &format!(
                        "total={} pool={} open={} blacklist={} secure={}",
                        snapshot.total_tokens,
                        snapshot.with_pool_price,
                        snapshot.open_positions,
                        snapshot.blacklisted,
                        snapshot.secure_tokens
                    ),
                );
            }

            Ok(Json(TokenStatsResponse {
                total_tokens: snapshot.total_tokens,
                with_pool_price: snapshot.with_pool_price,
                open_positions: snapshot.open_positions,
                blacklisted: snapshot.blacklisted,
                secure_tokens: snapshot.secure_tokens,
                with_ohlcv: snapshot.with_ohlcv,
                timestamp: snapshot.updated_at.to_rfc3339(),
            }))
        }
        Err(err) => {
            log(
                LogTag::Webserver,
                "WARN",
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
    if is_debug_webserver_enabled() {
        log(
            LogTag::Webserver,
            "TOKENS_FILTER",
            &format!("view={} search='{}'", filter.view, filter.search),
        );
    }

    let max_page_size = with_config(|cfg| cfg.webserver.tokens_tab.max_page_size);
    let filtering_query = filter.into_filtering_query(max_page_size);

    match filtering::query_tokens(filtering_query).await {
        Ok(result) => Ok(Json(TokenListResponse {
            items: result.items,
            page: result.page,
            page_size: result.page_size,
            total: result.total,
            total_pages: result.total_pages,
            timestamp: result.timestamp.to_rfc3339(),
        })),
        Err(err) => {
            log(
                LogTag::Webserver,
                "WARN",
                &format!("Filtering query failed: {}", err),
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
