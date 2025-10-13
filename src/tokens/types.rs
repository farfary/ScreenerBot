use chrono::{DateTime, Utc};
/// Data types for the centralized pricing system
use serde::{Deserialize, Serialize};

// ApiToken deleted - Token is now the only type (Phase 10: ApiToken elimination complete)

/// Transaction data for different time periods
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TxnPeriod {
    pub buys: Option<i64>,
    pub sells: Option<i64>,
}

/// Transaction statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TxnStats {
    pub m5: Option<TxnPeriod>,
    pub h1: Option<TxnPeriod>,
    pub h6: Option<TxnPeriod>,
    pub h24: Option<TxnPeriod>,
}

/// Liquidity information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LiquidityInfo {
    pub usd: Option<f64>,
    pub base: Option<f64>,
    pub quote: Option<f64>,
}

/// Volume statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VolumeStats {
    pub h24: Option<f64>,
    pub h6: Option<f64>,
    pub h1: Option<f64>,
    pub m5: Option<f64>,
}

/// Transaction detail
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TxnDetail {
    pub buys: Option<i64>,
    pub sells: Option<i64>,
}

/// Price change statistics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PriceChangeStats {
    pub h24: Option<f64>,
    pub h6: Option<f64>,
    pub h1: Option<f64>,
    pub m5: Option<f64>,
}

/// Boost information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoostInfo {
    pub active: Option<i64>,
}

/// Token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub address: String,
    pub name: String,
    pub symbol: String,
    pub image_url: Option<String>,
    pub websites: Option<Vec<WebsiteInfo>>,
    pub socials: Option<Vec<SocialInfo>>,
}

/// Website information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteInfo {
    pub url: String,
}

/// Social media information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialInfo {
    pub platform: String,
    pub handle: String,
}

/// Token struct with cached decimal data for fast filtering
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Token {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub chain: String,

    // Cached data for fast filtering (no API calls needed)
    pub decimals: Option<u8>,

    // Existing fields we need to keep
    pub logo_url: Option<String>,
    pub coingecko_id: Option<String>,
    pub website: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub is_verified: bool,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_updated: chrono::DateTime<chrono::Utc>,

    // Price data from various sources
    pub price_dexscreener_sol: Option<f64>,
    pub price_dexscreener_usd: Option<f64>,
    pub price_pool_sol: Option<f64>,
    pub price_pool_usd: Option<f64>,

    // New fields from DexScreener API
    pub dex_id: Option<String>,
    pub pair_address: Option<String>,
    pub pair_url: Option<String>,
    pub labels: Vec<String>,
    pub fdv: Option<f64>, // Fully Diluted Valuation
    pub market_cap: Option<f64>,
    pub txns: Option<TxnStats>,
    pub volume: Option<VolumeStats>,
    pub price_change: Option<PriceChangeStats>,
    pub liquidity: Option<LiquidityInfo>,
    pub info: Option<TokenInfoCompat>,
    pub boosts: Option<BoostInfo>,
}

/// Compatible token info struct for backward compatibility
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TokenInfoCompat {
    pub image_url: Option<String>,
    pub header: Option<String>,
    pub open_graph: Option<String>,
    pub websites: Vec<WebsiteLink>,
    pub socials: Vec<SocialLink>,
}

/// Social media links
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SocialLink {
    pub link_type: String, // "twitter", "telegram", etc.
    pub url: String,
}

/// Website links
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebsiteLink {
    pub label: Option<String>,
    pub url: String,
}

// =============================================================================
// TOKEN DATABASE TYPES
// =============================================================================

/// Database schema for tokens
#[derive(Debug, Clone)]
pub struct TokenRecord {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub chain_id: String,
    pub liquidity_usd: Option<f64>,
    pub price_usd: f64,
    pub price_sol: Option<f64>,
    pub last_updated: DateTime<Utc>,
}

// Conversion implementations deleted - Token is the only type, no conversions needed

/// Token discovery source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoverySource {
    pub source_type: DiscoverySourceType,
    pub discovered_at: DateTime<Utc>,
    pub url: String,
}

/// Types of discovery sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoverySourceType {
    DexScreenerBoosts,
    DexScreenerBoostsTop,
    DexScreenerProfiles,
}

/// Price source information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceSource {
    pub source_type: PriceSourceType,
    pub price: f64,
    pub timestamp: DateTime<Utc>,
    pub confidence: f64, // 0.0 to 1.0
}

/// Types of price sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PriceSourceType {
    DexScreenerApi,
    PoolCalculation,
    CachedPrice,
}

/// API response wrapper for DexScreener
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexScreenerResponse {
    pub schema_version: Option<String>,
    pub pairs: Option<Vec<DexScreenerPair>>,
}

/// DexScreener pair data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexScreenerPair {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    #[serde(rename = "dexId")]
    pub dex_id: String,
    pub url: Option<String>,
    #[serde(rename = "pairAddress")]
    pub pair_address: String,
    pub labels: Option<Vec<String>>,
    #[serde(rename = "baseToken")]
    pub base_token: BaseToken,
    #[serde(rename = "quoteToken")]
    pub quote_token: QuoteToken,
    #[serde(rename = "priceNative")]
    pub price_native: String,
    #[serde(rename = "priceUsd")]
    pub price_usd: Option<String>,
    pub txns: Option<serde_json::Value>,
    pub volume: Option<serde_json::Value>,
    #[serde(rename = "priceChange")]
    pub price_change: Option<serde_json::Value>,
    pub liquidity: Option<serde_json::Value>,
    pub fdv: Option<f64>,
    #[serde(rename = "marketCap")]
    pub market_cap: Option<f64>,
    #[serde(rename = "pairCreatedAt")]
    pub pair_created_at: Option<i64>,
    pub info: Option<serde_json::Value>,
    pub boosts: Option<serde_json::Value>,
}

/// Base token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseToken {
    pub address: String,
    pub name: String,
    pub symbol: String,
}

/// Quote token information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteToken {
    pub address: String,
    pub name: String,
    pub symbol: String,
}

/// Statistics for API calls
#[derive(Debug, Clone)]
pub struct ApiStats {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub last_request_time: Option<DateTime<Utc>>,
    pub average_response_time_ms: f64,
}

impl ApiStats {
    pub fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            cache_hits: 0,
            cache_misses: 0,
            last_request_time: None,
            average_response_time_ms: 0.0,
        }
    }

    pub fn record_request(&mut self, success: bool, response_time_ms: f64) {
        self.total_requests += 1;
        if success {
            self.successful_requests += 1;
        } else {
            self.failed_requests += 1;
        }

        // Update average response time
        let total_time = self.average_response_time_ms * ((self.total_requests - 1) as f64);
        self.average_response_time_ms =
            (total_time + response_time_ms) / (self.total_requests as f64);

        self.last_request_time = Some(Utc::now());
    }

    pub fn record_cache_hit(&mut self) {
        self.cache_hits += 1;
    }

    pub fn record_cache_miss(&mut self) {
        self.cache_misses += 1;
    }

    pub fn get_success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            ((self.successful_requests as f64) / (self.total_requests as f64)) * 100.0
        }
    }

    pub fn get_cache_hit_rate(&self) -> f64 {
        let total_cache_requests = self.cache_hits + self.cache_misses;
        if total_cache_requests == 0 {
            0.0
        } else {
            ((self.cache_hits as f64) / (total_cache_requests as f64)) * 100.0
        }
    }
}

impl std::fmt::Display for ApiStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Total: {}, Success: {:.1}%, Cache Hit: {:.1}%, Avg Response: {:.1}ms",
            self.total_requests,
            self.get_success_rate(),
            self.get_cache_hit_rate(),
            self.average_response_time_ms
        )
    }
}
