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
    pub dca_levels: Vec<DCALevel>,
}

impl Position {
    pub fn new(token_address: String, token_symbol: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            token_address,
            token_symbol,
            total_invested_sol: 0.0,
            average_buy_price: 0.0,
            current_price: 0.0,
            total_tokens: 0.0,
            unrealized_pnl_sol: 0.0,
            unrealized_pnl_percent: 0.0,
            realized_pnl_sol: 0.0,
            total_trades: 0,
            dca_level: 0,
            status: PositionStatus::Active,
            created_at: now,
            updated_at: now,
            dca_levels: Vec::new(),
        }
    }

    pub fn from_summary(id: i64, summary: PositionSummary) -> Self {
        Self {
            id: Some(id),
            token_address: summary.token_address,
            token_symbol: summary.token_symbol,
            total_invested_sol: summary.total_invested_sol,
            average_buy_price: summary.average_buy_price,
            current_price: summary.current_price,
            total_tokens: summary.total_tokens,
            unrealized_pnl_sol: summary.unrealized_pnl_sol,
            unrealized_pnl_percent: summary.unrealized_pnl_percent,
            realized_pnl_sol: summary.realized_pnl_sol,
            total_trades: summary.total_trades,
            dca_level: summary.dca_level,
            status: summary.status,
            created_at: summary.created_at,
            updated_at: summary.updated_at,
            dca_levels: Vec::new(),
        }
    }

    pub fn to_summary(&self) -> PositionSummary {
        PositionSummary {
            token_address: self.token_address.clone(),
            token_symbol: self.token_symbol.clone(),
            total_invested_sol: self.total_invested_sol,
            average_buy_price: self.average_buy_price,
            current_price: self.current_price,
            total_tokens: self.total_tokens,
            unrealized_pnl_sol: self.unrealized_pnl_sol,
            unrealized_pnl_percent: self.unrealized_pnl_percent,
            realized_pnl_sol: self.realized_pnl_sol,
            total_trades: self.total_trades,
            dca_level: self.dca_level,
            status: self.status.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }

    pub fn add_buy_trade(&mut self, amount_sol: f64, amount_tokens: f64, _price_per_token: f64) {
        // Update average buy price using weighted average
        let new_total_invested = self.total_invested_sol + amount_sol;
        let new_total_tokens = self.total_tokens + amount_tokens;

        if new_total_tokens > 0.0 {
            self.average_buy_price = new_total_invested / new_total_tokens;
        }

        self.total_invested_sol = new_total_invested;
        self.total_tokens = new_total_tokens;
        self.total_trades += 1;
        self.updated_at = Utc::now();

        // Update unrealized PnL
        self.update_unrealized_pnl(self.current_price);
    }

    pub fn add_sell_trade(&mut self, amount_sol: f64, amount_tokens: f64, _price_per_token: f64) {
        // Calculate realized PnL for this trade
        let cost_basis = amount_tokens * self.average_buy_price;
        let realized_pnl = amount_sol - cost_basis;

        self.realized_pnl_sol += realized_pnl;
        self.total_tokens -= amount_tokens;
        self.total_trades += 1;
        self.updated_at = Utc::now();

        // If we sold all tokens, close the position
        if self.total_tokens <= 0.0001 {
            self.status = PositionStatus::Closed;
            self.total_tokens = 0.0;
            self.unrealized_pnl_sol = 0.0;
            self.unrealized_pnl_percent = 0.0;
        } else {
            // Update unrealized PnL for remaining tokens
            self.update_unrealized_pnl(self.current_price);
        }
    }

    pub fn update_price(&mut self, new_price: f64) {
        self.current_price = new_price;
        self.updated_at = Utc::now();
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

    pub fn get_next_dca_level(&self) -> Option<&DCALevel> {
        self.dca_levels
            .iter()
            .find(|level| !level.executed && self.unrealized_pnl_percent <= level.trigger_percent)
    }

    pub fn initialize_dca_levels(&mut self, config: &crate::config::TraderConfig) {
        if !config.dca_enabled {
            return;
        }

        let mut levels = Vec::new();
        let step =
            (config.dca_max_loss_percent - config.dca_min_loss_percent) /
            (config.dca_levels as f64);

        for i in 0..config.dca_levels {
            let trigger_percent = config.dca_min_loss_percent + (i as f64) * step;
            levels.push(DCALevel {
                level: i + 1,
                trigger_percent,
                amount_sol: config.trade_size_sol * (1.0 + (i as f64) * 0.5), // Increase DCA size
                executed: false,
                executed_at: None,
                price: None,
            });
        }

        self.dca_levels = levels;
    }

    pub fn execute_dca_level(&mut self, level: u32, price: f64) -> Result<()> {
        if let Some(dca_level) = self.dca_levels.iter_mut().find(|l| l.level == level) {
            dca_level.executed = true;
            dca_level.executed_at = Some(Utc::now());
            dca_level.price = Some(price);
            self.dca_level = level;
        }
        Ok(())
    }

    pub fn save_to_database(&self, db: &TraderDatabase) -> Result<i64> {
        let position_id = if let Some(id) = self.id {
            db.update_position(id, &self.to_summary())?;
            id
        } else {
            let id = db.create_position(&self.token_address, &self.token_symbol)?;
            db.update_position(id, &self.to_summary())?;

            // Create DCA levels if they exist
            if !self.dca_levels.is_empty() {
                db.create_dca_levels(id, &self.dca_levels)?;
            }

            id
        };

        Ok(position_id)
    }

    pub fn load_from_database(db: &TraderDatabase, token_address: &str) -> Result<Option<Self>> {
        if let Some((id, summary)) = db.get_position(token_address)? {
            let mut position = Self::from_summary(id, summary);
            position.dca_levels = db.get_dca_levels(id)?;
            Ok(Some(position))
        } else {
            Ok(None)
        }
    }
}
