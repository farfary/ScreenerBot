//! Types for Trade Watcher module
//!
//! Defines types for monitoring external wallet trades and triggering actions.

use serde::{Deserialize, Serialize};

/// Watch type defines what action to take when a trade is detected
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum WatchType {
    /// Buy when external wallet sells (counter-trade)
    BuyOnSell,
    /// Sell when external wallet buys (follow-trade)
    SellOnBuy,
    /// Just send notification, no automatic action
    NotifyOnly,
}

impl WatchType {
    /// Parse watch type from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "buy_on_sell" => Self::BuyOnSell,
            "sell_on_buy" => Self::SellOnBuy,
            _ => Self::NotifyOnly,
        }
    }

    /// Convert to string representation
    pub fn to_str(&self) -> &'static str {
        match self {
            Self::BuyOnSell => "buy_on_sell",
            Self::SellOnBuy => "sell_on_buy",
            Self::NotifyOnly => "notify_only",
        }
    }

    /// Get human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::BuyOnSell => "Buy when target wallet sells",
            Self::SellOnBuy => "Sell when target wallet buys",
            Self::NotifyOnly => "Notify only, no action",
        }
    }
}

impl Default for WatchType {
    fn default() -> Self {
        Self::NotifyOnly
    }
}

/// Source of pool data
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum PoolSource {
    GeckoTerminal,
    DexScreener,
}

impl PoolSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GeckoTerminal => "geckoterminal",
            Self::DexScreener => "dexscreener",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "geckoterminal" => Self::GeckoTerminal,
            "dexscreener" => Self::DexScreener,
            _ => Self::GeckoTerminal,
        }
    }
}

impl Default for PoolSource {
    fn default() -> Self {
        Self::GeckoTerminal
    }
}

/// Pool information from API sources
#[derive(Clone, Debug, Serialize)]
pub struct PoolInfo {
    /// Pool address (pair address)
    pub address: String,
    /// Data source (GeckoTerminal or DexScreener)
    pub source: PoolSource,
    /// DEX identifier (raydium, orca, etc.)
    pub dex: String,
    /// Base token mint address
    pub base_token: String,
    /// Base token symbol
    pub base_symbol: String,
    /// Quote token mint address
    pub quote_token: String,
    /// Quote token symbol
    pub quote_symbol: String,
    /// Liquidity in USD
    pub liquidity_usd: f64,
    /// 24h trading volume in USD
    pub volume_24h: f64,
    /// Current price in USD
    pub price_usd: f64,
}

impl PoolInfo {
    /// Get the trading pair name (e.g., "BONK/SOL")
    pub fn pair_name(&self) -> String {
        format!("{}/{}", self.base_symbol, self.quote_symbol)
    }
}

/// Detected trade from pool monitoring
#[derive(Clone, Debug, Serialize)]
pub struct DetectedTrade {
    /// Transaction signature
    pub signature: String,
    /// Trade type ("buy" or "sell")
    pub trade_type: String,
    /// Wallet address that made the trade
    pub wallet: String,
    /// Amount of base token traded
    pub amount_base: f64,
    /// Amount of quote token traded
    pub amount_quote: f64,
    /// Trade price
    pub price: f64,
    /// Volume in USD
    pub volume_usd: f64,
    /// Block timestamp
    pub timestamp: i64,
}

/// Trade monitor status
#[derive(Clone, Debug, Serialize)]
pub struct TradeMonitorStatus {
    /// Whether the monitor is currently running
    pub is_running: bool,
    /// Number of tokens being watched
    pub watched_count: usize,
    /// Number of active watches
    pub active_count: usize,
    /// Total trades detected across all watches
    pub total_trades_detected: i32,
    /// Total actions triggered
    pub total_actions_triggered: i32,
}
