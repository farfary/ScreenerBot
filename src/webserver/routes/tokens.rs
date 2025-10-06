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
        .route("/tokens/stats", get(get_tokens_stats))
        .route("/tokens/filter", post(filter_tokens))
        .route("/tokens/:mint", get(get_token_detail))
        .route("/tokens/:mint/ohlcv", get(get_token_ohlcv))
}

// =============================================================================
// HANDLERS
// =============================================================================

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
// HELPER FUNCTIONS
// =============================================================================

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
