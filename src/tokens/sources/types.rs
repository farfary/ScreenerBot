use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataSource {
    DexScreener,
    GeckoTerminal,
}

impl ToString for DataSource {
    fn to_string(&self) -> String {
        match self {
            DataSource::DexScreener => "dexscreener".to_string(),
            DataSource::GeckoTerminal => "geckoterminal".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourcedPrice {
    pub source: DataSource,
    pub price_sol: f64,
    pub price_usd: Option<f64>,
    pub pool_address: Option<String>,
    pub liquidity_usd: Option<f64>,
    pub fetched_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SourcedPool {
    pub source: DataSource,
    pub pool_address: String,
    pub dex_id: String,
    pub base_token: String,
    pub quote_token: String,
    pub liquidity_usd: Option<f64>,
    pub volume_24h_usd: Option<f64>,
    pub price_sol: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct UnifiedTokenInfo {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub prices: Vec<SourcedPrice>,
    pub consensus_price_sol: Option<f64>,
    pub price_confidence: f64,
    pub pools: Vec<SourcedPool>,
    pub primary_pool: Option<String>,
    pub liquidity_usd: Option<f64>,
    pub volume_24h_usd: Option<f64>,
    pub market_cap: Option<f64>,
    pub last_updated: DateTime<Utc>,
    pub sources: Vec<DataSource>,
    pub fetch_timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub enum ValidationIssue {
    NotEnoughSources { available: usize, required: usize },
    SourcesDisagree { max_deviation_pct: f64 },
    NoConsensusPrice,
    NoSourcesEnabled,
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub consensus_price: Option<f64>,
    pub used_sources: Vec<DataSource>,
    pub issues: Vec<ValidationIssue>,
}
