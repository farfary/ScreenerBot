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
    pub value_sol: Option<f64>,
    pub entry_price_sol: Option<f64>,
    pub current_price_sol: Option<f64>,
    pub pnl_sol: Option<f64>,
    pub pnl_percentage: Option<f64>,
    pub realized_pnl_sol: Option<f64>,
    pub unrealized_pnl_sol: Option<f64>,
    pub total_invested_sol: Option<f64>,
    pub average_entry_price_sol: Option<f64>,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryStats {
    pub total_tokens_discovered: u64,
    pub active_tokens: u64,
    pub last_discovery_run: chrono::DateTime<chrono::Utc>,
    pub discovery_rate_per_hour: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletTransaction {
    pub signature: String,
    pub mint: String,
    pub transaction_type: TransactionType,
    pub amount: u64,
    pub price_sol: Option<f64>,
    pub value_sol: Option<f64>,
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
    pub average_buy_price_sol: f64,
    pub average_sell_price_sol: f64,
    pub total_invested_sol: f64,
    pub total_received_sol: f64,
    pub realized_pnl_sol: f64,
    pub unrealized_pnl_sol: f64,
    pub total_pnl_sol: f64,
    pub roi_percentage: f64,
    pub current_value_sol: f64,
}
