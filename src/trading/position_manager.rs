use crate::config::TraderConfig;
use crate::database::Database;
use crate::logger::Logger;
use crate::types::{ TradingPosition, PortfolioMetrics, PositionStatus, TimeCategory };
use crate::wallet::WalletTracker;
use anyhow::{ Context, Result };
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Manages all trading positions including tracking, updating, and metrics calculation
pub struct PositionManager {
    config: TraderConfig,
    database: Arc<Database>,
    wallet_tracker: Arc<WalletTracker>,
    positions: Arc<RwLock<HashMap<String, TradingPosition>>>,
    is_running: Arc<RwLock<bool>>,
}

impl PositionManager {
    pub fn new(
        config: TraderConfig,
        database: Arc<Database>,
        wallet_tracker: Arc<WalletTracker>
    ) -> Self {
        Self {
            config,
            database,
            wallet_tracker,
            positions: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> Result<()> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(());
        }
        *is_running = true;
        drop(is_running);

        Logger::success("Position Manager started");

        // Load existing positions from database
        self.load_positions().await?;

        Ok(())
    }

    pub async fn stop(&self) {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        Logger::info("Position Manager stopped");
    }

    pub async fn open_position(
        &self,
        token_mint: String,
        entry_price: f64,
        entry_amount_sol: f64,
        entry_amount_tokens: f64
    ) -> Result<String> {
        let position_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let position = TradingPosition {
            id: position_id.clone(),
            token_mint: token_mint.clone(),
            entry_price,
            entry_amount_sol,
            entry_amount_tokens,
            current_price: entry_price,
            current_value_sol: entry_amount_sol,
            pnl_sol: 0.0,
            pnl_percentage: 0.0,
            opened_at: now,
            last_updated: now,
            status: PositionStatus::Open,
            profit_target: self.config.min_profit_percentage,
            time_category: TimeCategory::Quick,
        };

        // Save to database
        self.save_position(&position).await?;

        // Add to memory
        self.positions.write().await.insert(position_id.clone(), position);

        Logger::trader(
            &format!(
                "📈 Position opened: {} | Size: {:.6} SOL | Price: ${:.8}",
                token_mint,
                entry_amount_sol,
                entry_price
            )
        );

        Ok(position_id)
    }

    pub async fn close_position(&self, position_id: &str) -> Result<()> {
        let mut positions = self.positions.write().await;

        if let Some(mut position) = positions.get_mut(position_id) {
            position.status = PositionStatus::Closed;
            position.last_updated = Utc::now();

            // Calculate final P&L
            self.update_position_pnl(&mut position).await?;

            // Save final state to database
            self.save_position(&position).await?;

            Logger::trader(
                &format!(
                    "📉 Position closed: {} | P&L: {:.2}% ({:.6} SOL)",
                    position.token_mint,
                    position.pnl_percentage,
                    position.pnl_sol
                )
            );

            // Remove from active positions
            positions.remove(position_id);
        }

        Ok(())
    }

    pub async fn update_positions(&self) -> Result<()> {
        let mut positions = self.positions.write().await;
        let mut updated_count = 0;

        for position in positions.values_mut() {
            if matches!(position.status, PositionStatus::Open) {
                if self.update_position_pnl(position).await.is_ok() {
                    self.update_time_category(position);
                    self.save_position(position).await?;
                    updated_count += 1;
                }
            }
        }

        if updated_count > 0 {
            Logger::trader(&format!("Updated {} positions", updated_count));
        }

        Ok(())
    }

    pub async fn get_open_positions(&self) -> Result<Vec<TradingPosition>> {
        let positions = self.positions.read().await;
        Ok(
            positions
                .values()
                .filter(|p| matches!(p.status, PositionStatus::Open))
                .cloned()
                .collect()
        )
    }

    pub async fn get_portfolio_metrics(&self) -> Result<PortfolioMetrics> {
        let positions = self.positions.read().await;
        let open_positions: Vec<_> = positions
            .values()
            .filter(|p| matches!(p.status, PositionStatus::Open))
            .collect();

        let total_value_sol = open_positions
            .iter()
            .map(|p| p.current_value_sol)
            .sum();
        let total_pnl_sol = open_positions
            .iter()
            .map(|p| p.pnl_sol)
            .sum();

        let total_pnl_percentage = if total_value_sol > 0.0 {
            (total_pnl_sol / (total_value_sol - total_pnl_sol)) * 100.0
        } else {
            0.0
        };

        let profitable_positions = open_positions
            .iter()
            .filter(|p| p.pnl_sol > 0.0)
            .count() as u32;
        let losing_positions = open_positions
            .iter()
            .filter(|p| p.pnl_sol < 0.0)
            .count() as u32;

        let win_rate = if open_positions.len() > 0 {
            ((profitable_positions as f64) / (open_positions.len() as f64)) * 100.0
        } else {
            0.0
        };

        let best_performer = open_positions
            .iter()
            .max_by(|a, b| a.pnl_percentage.partial_cmp(&b.pnl_percentage).unwrap())
            .map(|p| p.token_mint.clone());

        let worst_performer = open_positions
            .iter()
            .min_by(|a, b| a.pnl_percentage.partial_cmp(&b.pnl_percentage).unwrap())
            .map(|p| p.token_mint.clone());

        Ok(PortfolioMetrics {
            total_value_sol,
            total_pnl_sol,
            total_pnl_percentage,
            open_positions: open_positions.len() as u32,
            profitable_positions,
            losing_positions,
            best_performer,
            worst_performer,
            win_rate,
            last_updated: Utc::now(),
        })
    }

    async fn update_position_pnl(&self, position: &mut TradingPosition) -> Result<()> {
        // Get current token price (implement this based on your pricing system)
        let current_price = self.get_current_price(&position.token_mint).await?;

        position.current_price = current_price;
        position.current_value_sol = position.entry_amount_tokens * current_price;
        position.pnl_sol = position.current_value_sol - position.entry_amount_sol;
        position.pnl_percentage = (position.pnl_sol / position.entry_amount_sol) * 100.0;
        position.last_updated = Utc::now();

        Ok(())
    }

    fn update_time_category(&self, position: &mut TradingPosition) {
        let elapsed = Utc::now().signed_duration_since(position.opened_at);

        position.time_category = if elapsed.num_minutes() < 5 {
            TimeCategory::Quick
        } else if elapsed.num_hours() < 1 {
            TimeCategory::Medium
        } else if elapsed.num_hours() < 24 {
            TimeCategory::Long
        } else {
            TimeCategory::Extended
        };
    }

    async fn get_current_price(&self, token_mint: &str) -> Result<f64> {
        // This should integrate with your pricing system
        // For now, return a placeholder
        // TODO: Integrate with the pricing manager or external price source
        Ok(0.0)
    }

    async fn load_positions(&self) -> Result<()> {
        // Load positions from database
        // TODO: Implement database operations for TradingPosition
        Logger::trader("Loading positions from database...");
        Ok(())
    }

    async fn save_position(&self, position: &TradingPosition) -> Result<()> {
        // Save position to database
        // TODO: Implement database operations for TradingPosition
        Ok(())
    }
}
