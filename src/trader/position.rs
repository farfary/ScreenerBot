use anyhow::Result;
use chrono::{ DateTime, Utc };

use crate::trader::types::*;
use crate::trader::database::TraderDatabase;

#[derive(Debug, Clone)]
pub struct Position {
    pub id: Option<i64>,
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

impl Position {
    pub fn new(token_address: String, token_symbol: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            token_address,
            token_symbol,
            total_invested_sol: 0.0,
            original_entry_price: 0.0,
            average_buy_price: 0.0,
            current_price: 0.0,
            total_tokens: 0.0,
            unrealized_pnl_sol: 0.0,
            unrealized_pnl_percent: 0.0,
            realized_pnl_sol: 0.0,
            dca_count: 0,
            status: PositionStatus::Active,
            created_at: now,
            updated_at: now,
            peak_price: 0.0,
            lowest_price: 0.0,
        }
    }

    pub fn from_summary(id: i64, summary: PositionSummary) -> Self {
        Self {
            id: Some(id),
            token_address: summary.token_address,
            token_symbol: summary.token_symbol,
            total_invested_sol: summary.total_invested_sol,
            original_entry_price: summary.original_entry_price,
            average_buy_price: summary.average_buy_price,
            current_price: summary.current_price,
            total_tokens: summary.total_tokens,
            unrealized_pnl_sol: summary.unrealized_pnl_sol,
            unrealized_pnl_percent: summary.unrealized_pnl_percent,
            realized_pnl_sol: summary.realized_pnl_sol,
            dca_count: summary.dca_count,
            status: summary.status,
            created_at: summary.created_at,
            updated_at: summary.updated_at,
            peak_price: summary.peak_price,
            lowest_price: summary.lowest_price,
        }
    }

    pub fn to_summary(&self) -> PositionSummary {
        PositionSummary {
            token_address: self.token_address.clone(),
            token_symbol: self.token_symbol.clone(),
            total_invested_sol: self.total_invested_sol,
            original_entry_price: self.original_entry_price,
            average_buy_price: self.average_buy_price,
            current_price: self.current_price,
            total_tokens: self.total_tokens,
            unrealized_pnl_sol: self.unrealized_pnl_sol,
            unrealized_pnl_percent: self.unrealized_pnl_percent,
            realized_pnl_sol: self.realized_pnl_sol,
            dca_count: self.dca_count,
            status: self.status.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            peak_price: self.peak_price,
            lowest_price: self.lowest_price,
        }
    }

    pub fn add_buy_trade(&mut self, amount_sol: f64, amount_tokens: f64, price_per_token: f64) {
        // Set original entry price on first trade
        if self.original_entry_price == 0.0 {
            self.original_entry_price = price_per_token;
            self.peak_price = price_per_token;
            self.lowest_price = price_per_token;
        }

        // Update average buy price using weighted average
        let new_total_invested = self.total_invested_sol + amount_sol;
        let new_total_tokens = self.total_tokens + amount_tokens;

        if new_total_tokens > 0.0 {
            self.average_buy_price = new_total_invested / new_total_tokens;
        }

        self.total_invested_sol = new_total_invested;
        self.total_tokens = new_total_tokens;
        self.updated_at = Utc::now();

        // Update unrealized PnL
        self.update_unrealized_pnl(self.current_price);
    }

    pub fn add_dca_trade(&mut self, amount_sol: f64, amount_tokens: f64, price_per_token: f64) {
        // First do the regular buy trade logic
        self.add_buy_trade(amount_sol, amount_tokens, price_per_token);
        
        // Then increment the DCA counter
        self.dca_count += 1;
    }

    pub fn add_sell_trade(&mut self, amount_sol: f64, amount_tokens: f64, _price_per_token: f64) {
        // Calculate realized PnL for this trade
        let cost_basis = amount_tokens * self.average_buy_price;
        let realized_pnl = amount_sol - cost_basis;

        self.realized_pnl_sol += realized_pnl;
        self.total_tokens -= amount_tokens;
        self.updated_at = Utc::now();

        // If all tokens sold, mark position as closed
        if self.total_tokens <= 0.01 {
            // Using small threshold for floating point comparison
            self.total_tokens = 0.0;
            self.status = PositionStatus::Closed;
        } else {
            // Update unrealized PnL for remaining tokens
            self.update_unrealized_pnl(self.current_price);
        }
    }

    pub fn update_price(&mut self, new_price: f64) {
        self.current_price = new_price;
        self.updated_at = Utc::now();

        // Update peak and lowest prices after initial entry
        if self.original_entry_price > 0.0 {
            // Only track after first trade
            if self.peak_price == 0.0 || new_price > self.peak_price {
                self.peak_price = new_price;
            }
            if self.lowest_price == 0.0 || new_price < self.lowest_price {
                self.lowest_price = new_price;
            }
        }

        self.update_unrealized_pnl(new_price);
    }

    pub fn update_unrealized_pnl(&mut self, current_price: f64) {
        if self.total_tokens > 0.0 && self.average_buy_price > 0.0 {
            let current_value = self.total_tokens * current_price;
            let cost_basis = self.total_tokens * self.average_buy_price;

            self.unrealized_pnl_sol = current_value - cost_basis;
            self.unrealized_pnl_percent = (self.unrealized_pnl_sol / cost_basis) * 100.0;
        } else {
            self.unrealized_pnl_sol = 0.0;
            self.unrealized_pnl_percent = 0.0;
        }
    }

    pub fn get_total_pnl(&self) -> f64 {
        self.realized_pnl_sol + self.unrealized_pnl_sol
    }

    pub fn get_total_pnl_percent(&self) -> f64 {
        if self.total_invested_sol > 0.0 {
            (self.get_total_pnl() / self.total_invested_sol) * 100.0
        } else {
            0.0
        }
    }

    pub fn should_stop_loss(&self, stop_loss_percent: f64) -> bool {
        matches!(self.status, PositionStatus::Active) &&
            self.unrealized_pnl_percent <= stop_loss_percent
    }

    pub fn should_take_profit(&self, take_profit_percent: f64) -> bool {
        matches!(self.status, PositionStatus::Active) &&
            self.unrealized_pnl_percent >= take_profit_percent
    }

    pub fn should_dca(&self, config: &crate::config::TraderConfig) -> bool {
        if !config.dca_enabled {
            return false;
        }

        // Only DCA if we haven't exceeded max DCA count and price is below trigger
        let max_dca_count = config.dca_levels;
        let dca_trigger_percent = config.dca_min_loss_percent + 
            (self.dca_count as f64) * (config.dca_max_loss_percent - config.dca_min_loss_percent) / (max_dca_count as f64);

        self.dca_count < max_dca_count && 
        self.unrealized_pnl_percent <= dca_trigger_percent
    }

    pub fn get_dca_amount_sol(&self, config: &crate::config::TraderConfig) -> f64 {
        // Increase DCA size progressively: 1x, 1.5x, 2x, 2.5x...
        let multiplier = 1.0 + (self.dca_count as f64) * 0.5;
        config.trade_size_sol * multiplier
    }

    pub fn save_to_database(&self, db: &TraderDatabase) -> Result<i64> {
        let position_id = if let Some(id) = self.id {
            db.update_position(id, &self.to_summary())?;
            id
        } else {
            let id = db.create_position(&self.token_address, &self.token_symbol)?;
            db.update_position(id, &self.to_summary())?;
            id
        };

        Ok(position_id)
    }

    pub fn load_from_database(db: &TraderDatabase, token_address: &str) -> Result<Option<Self>> {
        if let Some((id, summary)) = db.get_position(token_address)? {
            let position = Self::from_summary(id, summary);
            Ok(Some(position))
        } else {
            Ok(None)
        }
    }
}
