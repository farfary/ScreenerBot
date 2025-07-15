use crate::config::TraderConfig;
use crate::logger::Logger;
use crate::types::{ TradingPosition, PositionStatus };
use crate::trading::position_manager::PositionManager;
use anyhow::Result;
use std::sync::Arc;

/// Manages trading risks and enforces safety limits
pub struct RiskManager {
    config: TraderConfig,
    position_manager: Arc<PositionManager>,
}

impl RiskManager {
    pub fn new(config: TraderConfig, position_manager: Arc<PositionManager>) -> Self {
        Self {
            config,
            position_manager,
        }
    }

    /// Check if a new position can be opened based on risk limits
    pub async fn can_open_position(&self, amount_sol: f64) -> Result<bool> {
        // Check maximum number of open positions
        let open_positions = self.position_manager.get_open_positions().await?;
        if open_positions.len() >= (self.config.max_open_positions as usize) {
            Logger::trader(
                &format!(
                    "⚠️  Cannot open position: Maximum positions limit reached ({}/{})",
                    open_positions.len(),
                    self.config.max_open_positions
                )
            );
            return Ok(false);
        }

        // Check trade size matches configuration
        if amount_sol != self.config.trade_size_sol {
            Logger::trader(
                &format!(
                    "⚠️  Cannot open position: Trade size {:.6} SOL does not match config {:.6} SOL",
                    amount_sol,
                    self.config.trade_size_sol
                )
            );
            return Ok(false);
        }

        // Additional risk checks can be added here
        // - Portfolio concentration limits
        // - Daily loss limits
        // - Volatility checks
        // - Market conditions

        Ok(true)
    }

    /// Check if a position should be force-closed due to risk limits
    pub async fn should_force_close(&self, position: &TradingPosition) -> Result<bool> {
        // Never force close due to losses (HODL strategy)
        // But we can add other risk-based closure reasons:

        // Check for extreme concentration (placeholder)
        if self.is_position_too_concentrated(position).await? {
            Logger::trader(
                &format!("⚠️  Position {} flagged for concentration risk", position.token_mint)
            );
            return Ok(false); // Still don't force close, just warn
        }

        // Check for stale positions (very old with minimal movement)
        if self.is_position_stale(position) {
            Logger::trader(
                &format!(
                    "⚠️  Position {} is stale (old with minimal movement)",
                    position.token_mint
                )
            );
            return Ok(false); // Log but don't force close
        }

        Ok(false) // Never force close due to losses
    }

    /// Calculate overall portfolio risk metrics
    pub async fn calculate_portfolio_risk(&self) -> Result<PortfolioRisk> {
        let positions = self.position_manager.get_open_positions().await?;
        let metrics = self.position_manager.get_portfolio_metrics().await?;

        let mut max_position_exposure = 0.0;
        let mut positions_at_loss = 0;
        let mut avg_position_age_hours = 0.0;

        for position in &positions {
            // Calculate exposure
            let exposure = (position.current_value_sol / metrics.total_value_sol) * 100.0;
            if exposure > max_position_exposure {
                max_position_exposure = exposure;
            }

            // Count positions at loss
            if position.pnl_sol < 0.0 {
                positions_at_loss += 1;
            }

            // Calculate average age
            let age_hours = chrono::Utc
                ::now()
                .signed_duration_since(position.opened_at)
                .num_hours() as f64;
            avg_position_age_hours += age_hours;
        }

        if !positions.is_empty() {
            avg_position_age_hours /= positions.len() as f64;
        }

        let risk_level = self.assess_risk_level(&metrics, max_position_exposure, positions_at_loss);

        Ok(PortfolioRisk {
            risk_level,
            total_exposure_sol: metrics.total_value_sol,
            max_position_exposure_pct: max_position_exposure,
            positions_at_loss,
            avg_position_age_hours,
            concentration_risk: max_position_exposure > 20.0, // More than 20% in one position
            win_rate: metrics.win_rate,
            total_pnl_pct: metrics.total_pnl_percentage,
        })
    }

    /// Log current risk status
    pub async fn log_risk_status(&self) -> Result<()> {
        let risk = self.calculate_portfolio_risk().await?;

        Logger::trader(
            &format!(
                "🛡️  Risk Status: {} | Exposure: {:.2} SOL | Max Position: {:.1}% | Losses: {} | Win Rate: {:.1}%",
                match risk.risk_level {
                    RiskLevel::Low => "🟢 LOW",
                    RiskLevel::Medium => "🟡 MEDIUM",
                    RiskLevel::High => "🔴 HIGH",
                },
                risk.total_exposure_sol,
                risk.max_position_exposure_pct,
                risk.positions_at_loss,
                risk.win_rate
            )
        );

        if risk.concentration_risk {
            Logger::trader("⚠️  Concentration risk detected: Position size > 20%");
        }

        Ok(())
    }

    /// Check if a position represents too much concentration
    async fn is_position_too_concentrated(&self, position: &TradingPosition) -> Result<bool> {
        let metrics = self.position_manager.get_portfolio_metrics().await?;

        if metrics.total_value_sol == 0.0 {
            return Ok(false);
        }

        let concentration_pct = (position.current_value_sol / metrics.total_value_sol) * 100.0;
        Ok(concentration_pct > 25.0) // Flag if more than 25% of portfolio
    }

    /// Check if a position is stale (old with minimal movement)
    fn is_position_stale(&self, position: &TradingPosition) -> bool {
        let age = chrono::Utc::now().signed_duration_since(position.opened_at);
        let is_old = age.num_hours() > 168; // More than 1 week
        let minimal_movement = position.pnl_percentage.abs() < 5.0; // Less than 5% movement

        is_old && minimal_movement
    }

    /// Assess overall portfolio risk level
    fn assess_risk_level(
        &self,
        metrics: &crate::types::PortfolioMetrics,
        max_exposure: f64,
        losses: u32
    ) -> RiskLevel {
        let mut risk_score = 0;

        // Factor in concentration
        if max_exposure > 30.0 {
            risk_score += 2;
        } else if max_exposure > 20.0 {
            risk_score += 1;
        }

        // Factor in losses
        if losses > 5 {
            risk_score += 2;
        } else if losses > 2 {
            risk_score += 1;
        }

        // Factor in overall PnL
        if metrics.total_pnl_percentage < -20.0 {
            risk_score += 2;
        } else if metrics.total_pnl_percentage < -10.0 {
            risk_score += 1;
        }

        // Factor in win rate
        if metrics.win_rate < 40.0 {
            risk_score += 1;
        }

        match risk_score {
            0..=1 => RiskLevel::Low,
            2..=3 => RiskLevel::Medium,
            _ => RiskLevel::High,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PortfolioRisk {
    pub risk_level: RiskLevel,
    pub total_exposure_sol: f64,
    pub max_position_exposure_pct: f64,
    pub positions_at_loss: u32,
    pub avg_position_age_hours: f64,
    pub concentration_risk: bool,
    pub win_rate: f64,
    pub total_pnl_pct: f64,
}

#[derive(Debug, Clone)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}
