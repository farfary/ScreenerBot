use axum::{ extract::Path, routing::get, Json, Router };
use serde::{ Deserialize, Serialize };
use std::sync::Arc;

use crate::{
    pools,
    tokens::{ cache::TokenDatabase, security_db::SecurityDatabase },
    webserver::state::AppState,
};

/// Token with price information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenWithPrice {
    pub mint: String,
    pub symbol: String,
    pub price_sol: f64,
    pub pool_address: String,
    pub updated_at: i64,
}

/// Tokens list response
#[derive(Debug, Serialize)]
pub struct TokensResponse {
    pub tokens: Vec<TokenWithPrice>,
    pub count: usize,
    pub timestamp: String,
}

/// Create tokens routes
pub fn routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/tokens", get(get_tokens_with_prices))
        .route("/tokens/:mint/debug", get(get_token_debug_info))
}

/// Get all tokens with available prices
async fn get_tokens_with_prices() -> Json<TokensResponse> {
    // Get all tokens that have available prices from pool service
    let available_mints = pools::get_available_tokens();

    let mut tokens_with_prices = Vec::new();

    // Get token details from database
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
        // Get price from pool service
        if let Some(price_result) = pools::get_pool_price(mint) {
            // Try to get symbol from database, fallback to short mint
            let symbol = match db.get_token_by_mint(mint) {
                Ok(Some(token)) => token.symbol,
                _ => format!("{}...", &mint[..8]),
            };

            // Calculate Unix timestamp from Instant
            let age_seconds = price_result.timestamp.elapsed().as_secs();
            let now_unix = std::time::SystemTime
                ::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;
            let price_unix_time = now_unix - (age_seconds as i64);

            tokens_with_prices.push(TokenWithPrice {
                mint: mint.clone(),
                symbol,
                price_sol: price_result.price_sol,
                pool_address: price_result.pool_address,
                updated_at: price_unix_time,
            });
        }
    }

    // Sort by symbol
    tokens_with_prices.sort_by(|a, b| a.symbol.cmp(&b.symbol));

    let count = tokens_with_prices.len();

    Json(TokensResponse {
        tokens: tokens_with_prices,
        count,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

// =============================================================================
// DEBUG INFO ENDPOINT
// =============================================================================

#[derive(Debug, Serialize)]
pub struct TokenDebugResponse {
    pub mint: String,
    pub timestamp: String,
    pub token_info: Option<TokenInfo>,
    pub price_data: Option<PriceData>,
    pub market_data: Option<MarketData>,
    pub pools: Vec<PoolInfo>,
    pub security: Option<SecurityInfo>,
    pub social: Option<SocialInfo>,
    pub pool_debug: Option<PoolDebugInfo>,
    pub token_debug: Option<TokenDebugInfo>,
}

#[derive(Debug, Serialize)]
pub struct TokenInfo {
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
pub struct PriceData {
    pub pool_price_sol: f64,
    pub pool_price_usd: Option<f64>,
    pub confidence: f32,
    pub last_updated: i64,
}

#[derive(Debug, Serialize)]
pub struct MarketData {
    pub market_cap: Option<f64>,
    pub fdv: Option<f64>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h: Option<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PoolInfo {
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
pub struct SecurityInfo {
    pub score: i32,
    pub score_normalised: i32,
    pub rugged: bool,
    pub mint_authority: Option<String>,
    pub freeze_authority: Option<String>,
    pub creator: Option<String>,
    pub total_holders: i32,
    pub top_10_concentration: Option<f64>,
    pub risks: Vec<RiskInfo>,
    pub analyzed_at: String,
}

#[derive(Debug, Serialize)]
pub struct RiskInfo {
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
    pub all_pools: Vec<PoolInfo>,
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
pub struct TokenDebugInfo {
    pub blacklist_status: Option<BlacklistStatus>,
    pub ohlcv_availability: OhlcvAvailability,
    pub decimals_info: DecimalsInfo,
}

#[derive(Debug, Serialize)]
pub struct BlacklistStatus {
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

/// Get comprehensive debug information for a token
async fn get_token_debug_info(Path(mint): Path<String>) -> Json<TokenDebugResponse> {
    let timestamp = chrono::Utc::now().to_rfc3339();

    // Load decimals from cache
    let decimals = crate::tokens::get_token_decimals(&mint).await;

    // 1. Get token info from database
    let api_token = TokenDatabase::new()
        .ok()
        .and_then(|db| db.get_token_by_mint(&mint).ok().flatten());

    let token_info = api_token.as_ref().map(|token| TokenInfo {
        symbol: token.symbol.clone(),
        name: token.name.clone(),
        decimals,
        logo_url: token.info.as_ref().and_then(|i| i.image_url.clone()),
        website: token.info
            .as_ref()
            .and_then(|i| i.websites.as_ref())
            .and_then(|w| w.first())
            .map(|w| w.url.clone()),
        description: None, // Not available in ApiToken
        tags: token.labels.clone().unwrap_or_default(),
        is_verified: token.labels
            .as_ref()
            .map(|l| l.iter().any(|label| label.to_lowercase() == "verified"))
            .unwrap_or(false),
    });

    // 2. Get current price from pool service
    let price_data = pools::get_pool_price(&mint).map(|price_result| {
        let age_seconds = price_result.timestamp.elapsed().as_secs();
        let now_unix = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let price_unix_time = now_unix - (age_seconds as i64);

        PriceData {
            pool_price_sol: price_result.price_sol,
            pool_price_usd: None, // We don't calculate USD prices
            confidence: price_result.confidence,
            last_updated: price_unix_time,
        }
    });

    // 3. Get market data from token database
    let market_data = api_token.as_ref().map(|token| MarketData {
        market_cap: token.market_cap,
        fdv: token.fdv,
        liquidity_usd: token.liquidity.as_ref().and_then(|l| l.usd),
        volume_24h: token.volume.as_ref().and_then(|v| v.h24),
    });

    // 4. Get pool info (currently only have single pool from price service)
    let mut pools_vec = Vec::new();
    if let Some(price_result) = pools::get_pool_price(&mint) {
        let age_seconds = price_result.timestamp.elapsed().as_secs();
        let now_unix = std::time::SystemTime
            ::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        let price_unix_time = now_unix - (age_seconds as i64);

        pools_vec.push(PoolInfo {
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
            last_updated: price_unix_time,
        });
    }

    // 5. Get security info from security database
    let security = SecurityDatabase::new("data/security.db")
        .ok()
        .and_then(|db| db.get_security_info(&mint).ok().flatten())
        .map(|sec| {
            // Calculate top 10 concentration
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

            SecurityInfo {
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
                    .map(|r| RiskInfo {
                        name: r.name.clone(),
                        level: r.level.clone(),
                        description: r.description.clone(),
                        score: r.score,
                    })
                    .collect(),
                analyzed_at: sec.analyzed_at,
            }
        });

    // 6. Get social info from token database
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

    // 7. Get pool debug data
    let pool_debug = {
        // Get price history (last 100 points)
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
                let price_unix_time = now_unix - (age_seconds as i64);

                PricePoint {
                    timestamp: price_unix_time,
                    price_sol: p.price_sol,
                    confidence: p.confidence,
                }
            })
            .collect();

        // Get price statistics
        let price_stats = pools
            ::get_price_history_stats(&mint)
            .ok()
            .map(|stats| {
                let (min_price, max_price) = stats.price_range_sol;
                let time_span = (stats.age_oldest_seconds - stats.age_newest_seconds) as i64;
                PriceStats {
                    min_price,
                    max_price,
                    avg_price: (min_price + max_price) / 2.0, // Simple average
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

        // Get all discovered pools (not just highest liquidity)
        // Note: Skipping detailed pool enumeration to keep changes minimal
        // The pools_vec already contains the main pool info
        let all_pools: Vec<PoolInfo> = pools_vec.clone();

        // Get cache stats
        let cache_stats = pools::get_cache_stats();
        let cache_stats_info = CacheStatsInfo {
            total_tokens_cached: cache_stats.total_prices,
            fresh_prices: cache_stats.fresh_prices,
            history_entries: cache_stats.history_entries,
        };

        Some(PoolDebugInfo {
            price_history,
            price_stats,
            all_pools,
            cache_stats: cache_stats_info,
        })
    };

    // 8. Get token debug data
    let token_debug = {
        // Get blacklist status
        let blacklist_status = {
            use crate::tokens::blacklist;
            let mints = blacklist::get_blacklisted_mints();
            if mints.contains(&mint) {
                BlacklistStatus {
                    is_blacklisted: true,
                    reason: Some("Token is blacklisted".to_string()),
                    first_occurrence: None,
                    occurrence_count: 0,
                }
            } else {
                BlacklistStatus {
                    is_blacklisted: false,
                    reason: None,
                    first_occurrence: None,
                    occurrence_count: 0,
                }
            }
        };

        // Get OHLCV availability
        let ohlcv_availability = {
            use crate::tokens::ohlcv_db;
            let db = ohlcv_db::get_ohlcv_database().ok();

            if let Some(db) = db {
                // Get OHLCV data (database stores 1m data only currently)
                let all_data = db.get_ohlcv_data(&mint, Some(1000)).ok().unwrap_or_default();
                let has_data = !all_data.is_empty();
                let oldest = all_data.last().map(|d| d.timestamp);
                let newest = all_data.first().map(|d| d.timestamp);

                OhlcvAvailability {
                    has_1m_data: has_data,
                    has_5m_data: false, // Not stored separately
                    has_15m_data: false, // Not stored separately
                    has_1h_data: false, // Not stored separately
                    total_candles: all_data.len(),
                    oldest_timestamp: oldest,
                    newest_timestamp: newest,
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

        // Get decimals info
        let decimals_info = {
            use crate::tokens::decimals;
            let cached_decimals = decimals::get_cached_decimals(&mint);
            let source = if cached_decimals.is_some() {
                "cache"
            } else if decimals.is_some() {
                "rpc_fetch"
            } else {
                "failed"
            };

            DecimalsInfo {
                decimals,
                cached: cached_decimals.is_some(),
                source: source.to_string(),
            }
        };

        Some(TokenDebugInfo {
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
