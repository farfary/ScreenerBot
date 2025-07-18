use serde::{ Deserialize, Serialize };
use chrono::{ DateTime, Utc };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    #[serde(rename = "chainId")]
    pub chain_id: String,
    #[serde(rename = "dexId")]
    pub dex_id: String,
    pub url: String,
    #[serde(rename = "pairAddress")]
    pub pair_address: String,
    pub labels: Option<Vec<String>>,
    #[serde(rename = "baseToken")]
    pub base_token: Token,
    #[serde(rename = "quoteToken")]
    pub quote_token: Token,
    #[serde(rename = "priceNative")]
    pub price_native: String,
    #[serde(rename = "priceUsd")]
    pub price_usd: String,
    pub txns: TransactionMetrics,
    pub volume: VolumeMetrics,
    #[serde(rename = "priceChange")]
    pub price_change: PriceChangeMetrics,
    #[serde(default)]
    pub liquidity: Option<LiquidityMetrics>,
    pub fdv: Option<f64>,
    #[serde(rename = "marketCap")]
    pub market_cap: Option<f64>,
    #[serde(rename = "pairCreatedAt")]
    pub pair_created_at: u64,
    pub info: Option<TokenInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub address: String,
    pub name: String,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionMetrics {
    pub m5: TransactionCount,
    pub h1: TransactionCount,
    pub h6: TransactionCount,
    pub h24: TransactionCount,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionCount {
    pub buys: u32,
    pub sells: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMetrics {
    pub m5: f64,
    pub h1: f64,
    pub h6: f64,
    pub h24: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceChangeMetrics {
    pub m5: Option<f64>,
    pub h1: Option<f64>,
    pub h6: Option<f64>,
    pub h24: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiquidityMetrics {
    pub usd: f64,
    pub base: f64,
    pub quote: f64,
}

impl Default for LiquidityMetrics {
    fn default() -> Self {
        Self {
            usd: 0.0,
            base: 0.0,
            quote: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    #[serde(rename = "imageUrl")]
    pub image_url: Option<String>,
    pub header: Option<String>,
    #[serde(rename = "openGraph")]
    pub open_graph: Option<String>,
    pub websites: Option<Vec<WebsiteInfo>>,
    pub socials: Option<Vec<SocialInfo>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteInfo {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialInfo {
    #[serde(rename = "type")]
    pub social_type: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairsError {
    pub message: String,
    pub code: Option<u16>,
}

impl std::fmt::Display for PairsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pairs error: {}", self.message)
    }
}

impl std::error::Error for PairsError {}

// Extension methods for analysis
impl TokenPair {
    /// Calculate total volume over 24 hours
    pub fn total_volume_24h(&self) -> f64 {
        self.volume.h24
    }

    /// Calculate total transactions over 24 hours
    pub fn total_transactions_24h(&self) -> u32 {
        self.txns.h24.buys + self.txns.h24.sells
    }

    /// Calculate buy/sell ratio for 24 hours
    pub fn buy_sell_ratio_24h(&self) -> Option<f64> {
        if self.txns.h24.sells > 0 {
            Some((self.txns.h24.buys as f64) / (self.txns.h24.sells as f64))
        } else {
            None
        }
    }

    /// Get price as float
    pub fn price_usd_float(&self) -> Result<f64, std::num::ParseFloatError> {
        self.price_usd.parse::<f64>()
    }

    /// Get price in native currency as float
    pub fn price_native_float(&self) -> Result<f64, std::num::ParseFloatError> {
        self.price_native.parse::<f64>()
    }

    /// Check if pair has high liquidity (>$100k USD)
    pub fn has_high_liquidity(&self) -> bool {
        self.liquidity.as_ref().map_or(false, |l| l.usd > 100_000.0)
    }

    /// Check if pair has recent activity (transactions in last 5 minutes)
    pub fn has_recent_activity(&self) -> bool {
        self.txns.m5.buys > 0 || self.txns.m5.sells > 0
    }

    /// Get the pair creation date
    pub fn created_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp((self.pair_created_at as i64) / 1000, 0).unwrap_or_else(||
            Utc::now()
        )
    }

    /// Check if this is a major trading pair (SOL or USDC quote)
    pub fn is_major_pair(&self) -> bool {
        matches!(self.quote_token.symbol.as_str(), "SOL" | "USDC" | "USDT" | "ETH" | "BTC")
    }

    /// Get price in SOL terms (approximation)
    pub fn price_sol_approx(&self) -> Result<f64, std::num::ParseFloatError> {
        let usd_price = self.price_usd.parse::<f64>()?;
        let sol_price_usd = 180.0; // Approximate SOL price
        Ok(usd_price / sol_price_usd)
    }

    /// Get liquidity in SOL terms (approximation)
    pub fn liquidity_sol_approx(&self) -> Option<f64> {
        self.liquidity.as_ref().map(|l| {
            let sol_price_usd = 180.0; // Approximate SOL price
            l.usd / sol_price_usd
        })
    }
}
