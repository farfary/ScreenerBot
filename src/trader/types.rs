use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignal {
    pub token_address: String,
    pub signal_type: TradeSignalType,
    pub current_price: f64,
    pub trigger_price: f64,
    pub timestamp: DateTime<Utc>,
    pub volume_24h: f64,
    pub liquidity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeSignalType {
    Buy,
    Sell,
    DCA,
    StopLoss,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeResult {
    pub success: bool,
    pub transaction_hash: Option<String>,
    pub amount_sol: f64,
    pub amount_tokens: f64,
    pub price_per_token: f64,
    pub fees: f64,
    pub slippage: f64,
    pub timestamp: DateTime<Utc>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSummary {
    pub token_address: String,
    pub token_symbol: String,
    pub total_invested_sol: f64,
    pub average_buy_price: f64,
    pub current_price: f64,
    pub total_tokens: f64,
    pub unrealized_pnl_sol: f64,
    pub unrealized_pnl_percent: f64,
    pub realized_pnl_sol: f64,
    pub total_trades: u32,
    pub dca_level: u32,
    pub status: PositionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionStatus {
    Active,
    Closed,
    StopLoss,
    TakeProfit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DCALevel {
    pub level: u32,
    pub trigger_percent: f64,
    pub amount_sol: f64,
    pub executed: bool,
    pub executed_at: Option<DateTime<Utc>>,
    pub price: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraderStats {
    pub total_trades: u32,
    pub successful_trades: u32,
    pub failed_trades: u32,
    pub total_invested_sol: f64,
    pub total_realized_pnl_sol: f64,
    pub total_unrealized_pnl_sol: f64,
    pub win_rate: f64,
    pub average_trade_size_sol: f64,
    pub largest_win_sol: f64,
    pub largest_loss_sol: f64,
    pub active_positions: u32,
    pub closed_positions: u32,
}

impl Default for TraderStats {
    fn default() -> Self {
        Self {
            total_trades: 0,
            successful_trades: 0,
            failed_trades: 0,
            total_invested_sol: 0.0,
            total_realized_pnl_sol: 0.0,
            total_unrealized_pnl_sol: 0.0,
            win_rate: 0.0,
            average_trade_size_sol: 0.0,
            largest_win_sol: 0.0,
            largest_loss_sol: 0.0,
            active_positions: 0,
            closed_positions: 0,
        }
    }
}
