use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
    pub supply: u64,
    pub market_cap: Option<f64>,
    pub price: Option<f64>,
    pub volume_24h: Option<f64>,
    pub liquidity: Option<f64>,
    pub pool_address: Option<String>,
    pub discovered_at: chrono::DateTime<chrono::Utc>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletPosition {
    pub mint: String,
    pub balance: u64,
    pub decimals: u8,
    pub value_usd: Option<f64>,
    pub entry_price: Option<f64>,
    pub current_price: Option<f64>,
    pub pnl: Option<f64>,
    pub pnl_percentage: Option<f64>,
    pub realized_pnl: Option<f64>,
    pub unrealized_pnl: Option<f64>,
    pub total_invested: Option<f64>,
    pub average_entry_price: Option<f64>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryStats {
    pub total_tokens_discovered: u64,
    pub active_tokens: u64,
    pub last_discovery_run: chrono::DateTime<chrono::Utc>,
    pub discovery_rate_per_hour: f64,
}

#[derive(Debug, Clone)]
pub struct LogLevel {
    pub level: log::Level,
    pub color: &'static str,
    pub prefix: &'static str,
}

impl LogLevel {
    pub const INFO: LogLevel = LogLevel {
        level: log::Level::Info,
        color: "\x1b[32m", // Green
        prefix: "ℹ️ ",
    };

    pub const WARN: LogLevel = LogLevel {
        level: log::Level::Warn,
        color: "\x1b[33m", // Yellow
        prefix: "⚠️ ",
    };

    pub const ERROR: LogLevel = LogLevel {
        level: log::Level::Error,
        color: "\x1b[31m", // Red
        prefix: "❌",
    };

    pub const DEBUG: LogLevel = LogLevel {
        level: log::Level::Debug,
        color: "\x1b[36m", // Cyan
        prefix: "🔍",
    };

    pub const SUCCESS: LogLevel = LogLevel {
        level: log::Level::Info,
        color: "\x1b[92m", // Bright Green
        prefix: "✅",
    };
}

pub const RESET_COLOR: &str = "\x1b[0m";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    pub signature: String,
    pub mint: String,
    pub transaction_type: TransactionType,
    pub amount: u64,
    pub price_usd: Option<f64>,
    pub value_usd: Option<f64>,
    pub sol_amount: Option<u64>,
    pub fee: Option<u64>,
    pub block_time: i64,
    pub slot: u64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Buy,
    Sell,
    Transfer,
    Receive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfitLossCalculation {
    pub mint: String,
    pub total_bought: u64,
    pub total_sold: u64,
    pub current_balance: u64,
    pub average_buy_price: f64,
    pub average_sell_price: f64,
    pub total_invested: f64,
    pub total_received: f64,
    pub realized_pnl: f64,
    pub unrealized_pnl: f64,
    pub total_pnl: f64,
    pub roi_percentage: f64,
    pub current_value: f64,
}
