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
    tokens::{blacklist, store::get_global_token_store, summary::TokenSummary, SecurityDatabase},
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
    pub pair_labels: Vec<String>,
    pub decimals: Option<u8>,
    pub created_at: Option<i64>,
    pub last_updated: Option<i64>,
    pub pair_created_at: Option<i64>,
    pub pair_url: Option<String>,
    pub boosts_active: Option<i64>,
    // Price info
    pub price_sol: Option<f64>,
    pub price_usd: Option<f64>,
    pub price_confidence: Option<f32>,
    pub price_updated_at: Option<i64>,
    pub price_change_h1: Option<f64>,
    pub price_change_h24: Option<f64>,
    pub price_change_periods: PeriodStats<f64>,
    // Market info
    pub liquidity_usd: Option<f64>,
    pub liquidity_base: Option<f64>,
    pub liquidity_quote: Option<f64>,
    pub volume_24h: Option<f64>,
    pub volume_periods: PeriodStats<f64>,
    pub fdv: Option<f64>,
    pub market_cap: Option<f64>,
    // Pool info
    pub pool_address: Option<String>,
    pub pool_dex: Option<String>,
    pub pool_reserves_sol: Option<f64>,
    pub pool_reserves_token: Option<f64>,
    // Activity info
    pub txn_periods: PeriodStats<TxnPeriodSummary>,
    pub buys_24h: Option<i64>,
    pub sells_24h: Option<i64>,
    pub net_flow_24h: Option<i64>,
    pub buy_sell_ratio_24h: Option<f64>,
    // Security info
    pub security_score: Option<i32>,
    pub security_score_normalized: Option<i32>,
    pub rugged: Option<bool>,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub total_holders: Option<i32>,
    pub top_10_concentration: Option<f64>,
    pub security_risks: Vec<SecurityRisk>,
    pub security_summary: Option<String>,
    // External references
    pub websites: Vec<TokenWebsiteLink>,
    pub socials: Vec<TokenSocialLink>,
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

    // Fetch token from in-memory store (instant lookup, no DB I/O)
    let lookup_start = std::time::Instant::now();
    let snapshot = match get_global_token_store().get(&mint) {
        Some(snap) => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "TOKEN_DETAIL_CACHE",
                    &format!(
                        "mint={} elapsed={}μs",
                        mint,
                        lookup_start.elapsed().as_micros()
                    ),
                );
            }
            snap
        }
        None => {
            if is_debug_webserver_enabled() {
                log(
                    LogTag::Webserver,
                    "TOKEN_DETAIL_NOT_FOUND",
                    &format!(
                        "mint={} elapsed={}μs",
                        mint,
                        lookup_start.elapsed().as_micros()
                    ),
                );
            }
            return Json(TokenDetailResponse {
                mint: mint.clone(),
                symbol: "NOT_FOUND".to_string(),
                name: Some("Token not in cache".to_string()),
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
                security_score: None,
                security_score_normalized: None,
                rugged: None,
                mint_authority: None,
                freeze_authority: None,
                total_holders: None,
                top_10_concentration: None,
                security_risks: vec![],
                security_summary: None,
                websites: vec![],
                socials: vec![],
                has_ohlcv: false,
                has_pool_price: false,
                has_open_position: false,
                blacklisted: false,
                timestamp: chrono::Utc::now().to_rfc3339(),
            });
        }
    };

    let token = &snapshot.data; // ApiToken from cache

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

    let created_at_ts = token.created_at.map(|dt| dt.timestamp());
    let last_updated_ts = Some(token.last_updated.timestamp());
    let pair_created_at = created_at_ts;
    let price_usd = token.price_pool_usd.or(token.price_dexscreener_usd);
    let price_change_periods = token
        .price_change
        .as_ref()
        .map(|changes| PeriodStats {
            m5: changes.m5,
            h1: changes.h1,
            h6: changes.h6,
            h24: changes.h24,
        })
        .unwrap_or_else(PeriodStats::empty);
    let volume_periods = token
        .volume
        .as_ref()
        .map(|vol| PeriodStats {
            m5: vol.m5,
            h1: vol.h1,
            h6: vol.h6,
            h24: vol.h24,
        })
        .unwrap_or_else(PeriodStats::empty);
    let txn_periods = token
        .txns
        .as_ref()
        .map(|stats| PeriodStats {
            m5: stats.m5.as_ref().map(|p| TxnPeriodSummary {
                buys: p.buys,
                sells: p.sells,
            }),
            h1: stats.h1.as_ref().map(|p| TxnPeriodSummary {
                buys: p.buys,
                sells: p.sells,
            }),
            h6: stats.h6.as_ref().map(|p| TxnPeriodSummary {
                buys: p.buys,
                sells: p.sells,
            }),
            h24: stats.h24.as_ref().map(|p| TxnPeriodSummary {
                buys: p.buys,
                sells: p.sells,
            }),
        })
        .unwrap_or_else(PeriodStats::empty);

    let (buys_24h, sells_24h) = if let Some(txn) = token.txns.as_ref() {
        (
            txn.h24.as_ref().and_then(|p| p.buys),
            txn.h24.as_ref().and_then(|p| p.sells),
        )
    } else {
        (None, None)
    };

    let net_flow_24h = match (buys_24h, sells_24h) {
        (Some(buys), Some(sells)) => Some(buys - sells),
        _ => None,
    };

    let buy_sell_ratio_24h = match (buys_24h, sells_24h) {
        (Some(buys), Some(sells)) if sells != 0 => Some(buys as f64 / sells as f64),
        _ => None,
    };

    let liquidity_base = token.liquidity.as_ref().and_then(|l| l.base);
    let liquidity_quote = token.liquidity.as_ref().and_then(|l| l.quote);

    let mut websites: Vec<TokenWebsiteLink> = Vec::new();
    if let Some(primary_site) = token.website.as_ref() {
        if !primary_site.trim().is_empty() {
            websites.push(TokenWebsiteLink {
                label: Some("Website".to_string()),
                url: primary_site.clone(),
            });
        }
    }

    let mut socials: Vec<TokenSocialLink> = Vec::new();
    if let Some(info) = token.info.as_ref() {
        for site in &info.websites {
            if site.url.trim().is_empty() {
                continue;
            }
            if websites.iter().any(|existing| existing.url == site.url) {
                continue;
            }
            websites.push(TokenWebsiteLink {
                label: site.label.clone(),
                url: site.url.clone(),
            });
        }

        for social in &info.socials {
            if social.url.trim().is_empty() {
                continue;
            }
            socials.push(TokenSocialLink {
                platform: social.link_type.clone(),
                url: social.url.clone(),
            });
        }
    }

    let mut combined_tags = token.tags.clone();
    for label in &token.labels {
        if !combined_tags.contains(label) {
            combined_tags.push(label.clone());
        }
    }

    let logo_url = token
        .logo_url
        .clone()
        .or_else(|| token.info.as_ref().and_then(|i| i.image_url.clone()));
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

    Json(TokenDetailResponse {
        mint: token.mint.clone(),
        symbol: token.symbol.clone(),
        name: Some(token.name.clone()),
        logo_url,
        website: primary_website,
        verified: token.is_verified,
        tags: combined_tags,
        pair_labels: token.labels.clone(),
        decimals: token.decimals,
        created_at: created_at_ts,
        last_updated: last_updated_ts,
        pair_created_at,
        pair_url: token.pair_url.clone(),
        boosts_active: token.boosts.as_ref().and_then(|b| b.active),
        price_sol,
        price_usd,
        price_confidence,
        price_updated_at,
        price_change_h1: token.price_change.as_ref().and_then(|p| p.h1),
        price_change_h24: token.price_change.as_ref().and_then(|p| p.h24),
        price_change_periods,
        liquidity_usd: token.liquidity.as_ref().and_then(|l| l.usd),
        liquidity_base,
        liquidity_quote,
        volume_24h: token.volume.as_ref().and_then(|v| v.h24),
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
        security_score,
        security_score_normalized,
        rugged,
        mint_authority,
        freeze_authority,
        total_holders,
        top_10_concentration,
        security_risks,
        security_summary,
        websites,
        socials,
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
