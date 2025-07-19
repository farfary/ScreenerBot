use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use std::collections::HashMap;

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
    FastProfit {
        profit_percentage: f64,
        sell_portion: f64,
        reason: String,
    },
    EmergencyStopLoss,
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
    // New fields for effective price tracking
    pub effective_price_per_token: f64, // Actual price after fees and slippage
    pub trading_fee: f64, // Fixed trading fee in SOL
    pub net_sol_received: f64, // For sells: actual SOL received after all fees
    pub net_tokens_received: f64, // For buys: actual tokens received
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenCooldown {
    pub token_address: String,
    pub sell_timestamp: DateTime<Utc>,
    pub cooldown_until: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct TokenCooldownManager {
    cooldowns: HashMap<String, TokenCooldown>,
    cooldown_duration_minutes: u64,
}

impl TokenCooldownManager {
    pub fn new(cooldown_duration_minutes: u64) -> Self {
        Self {
            cooldowns: HashMap::new(),
            cooldown_duration_minutes,
        }
    }

    pub fn add_cooldown(&mut self, token_address: &str) {
        let now = Utc::now();
        let cooldown_until = now + chrono::Duration::minutes(self.cooldown_duration_minutes as i64);

        let cooldown = TokenCooldown {
            token_address: token_address.to_string(),
            sell_timestamp: now,
            cooldown_until,
        };

        self.cooldowns.insert(token_address.to_string(), cooldown);
    }

    pub fn is_token_in_cooldown(&self, token_address: &str) -> bool {
        if let Some(cooldown) = self.cooldowns.get(token_address) {
            Utc::now() < cooldown.cooldown_until
        } else {
            false
        }
    }

    pub fn get_cooldown_remaining(&self, token_address: &str) -> Option<chrono::Duration> {
        if let Some(cooldown) = self.cooldowns.get(token_address) {
            let remaining = cooldown.cooldown_until - Utc::now();
            if remaining > chrono::Duration::zero() {
                Some(remaining)
            } else {
                None
            }
        } else {
            None
        }
    }

    pub fn cleanup_expired(&mut self) {
        let now = Utc::now();
        self.cooldowns.retain(|_, cooldown| now < cooldown.cooldown_until);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionSummary {
    pub token_address: String,
    pub token_symbol: String,
    pub total_invested_sol: f64,
    pub original_entry_price: f64,
    pub average_buy_price: f64,
    pub current_price: f64,
    pub total_tokens: f64,
    pub unrealized_pnl_sol: f64,
    pub unrealized_pnl_percent: f64,
    pub realized_pnl_sol: f64,
    pub dca_count: u32,
    pub status: PositionStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub peak_price: f64,
    pub lowest_price: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PositionStatus {
    Active,
    Closed,
    StopLoss,
    TakeProfit,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeExecution {
    pub trade_type: String,
    pub amount_sol: f64,
    pub amount_tokens: f64,
    pub price_per_token: f64,
    pub fees: f64,
    pub slippage: f64,
    pub transaction_hash: Option<String>,
    pub success: bool,
    pub error: Option<String>,
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
