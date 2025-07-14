use crate::core::{
    BotResult,
    BotError,
    PortfolioConfig,
    Portfolio,
    Position,
    PerformanceMetrics,
    TokenBalance,
    WalletTransaction,
    PortfolioHealth,
    RebalanceRecommendation,
    RebalanceAction,
};
use crate::wallet::WalletManager;
use crate::cache::CacheManager;
use solana_sdk::pubkey::Pubkey;
use chrono::{ Utc, DateTime };
use std::collections::HashMap;
use std::str::FromStr;

mod tracker;
mod analyzer;
mod display;

use tracker::PositionTracker;
use analyzer::PortfolioAnalyzer;
use display::PortfolioDisplay;

pub use tracker::*;
pub use analyzer::*;
pub use display::*;

/// Main portfolio manager for tracking and analyzing positions
#[derive(Debug)]
pub struct PortfolioManager {
    config: PortfolioConfig,
    pub current_portfolio: Portfolio,
    tracker: PositionTracker,
    analyzer: PortfolioAnalyzer,
    display: PortfolioDisplay,
}

impl PortfolioManager {
    /// Create a new portfolio manager
    pub fn new(bot_config: &crate::core::BotConfig) -> BotResult<Self> {
        let config = &bot_config.portfolio_config;
        let current_portfolio = Portfolio {
            positions: Vec::new(),
            total_value: 0.0,
            sol_balance: 0.0,
            total_pnl: 0.0,
            performance_metrics: PerformanceMetrics {
                total_return: 0.0,
                win_rate: 0.0,
                avg_gain: 0.0,
                avg_loss: 0.0,
                max_drawdown: 0.0,
                sharpe_ratio: 0.0,
                total_trades: 0,
                winning_trades: 0,
                losing_trades: 0,
            },
            diversification_score: 0.0,
            risk_score: 0.0,
            last_updated: chrono::Utc::now(),
        };

        Ok(Self {
            config: config.clone(),
            current_portfolio,
            tracker: PositionTracker::new(config.clone()),
            analyzer: PortfolioAnalyzer::new(config.clone()),
            display: PortfolioDisplay::new(),
        })
    }

    /// Get current portfolio snapshot
    pub fn get_portfolio(&self) -> &Portfolio {
        &self.current_portfolio
    }

    /// Update portfolio with latest data
    pub async fn update_portfolio(
        &mut self,
        wallet: &WalletManager,
        cache: &CacheManager
    ) -> BotResult<()> {
        // Get current balances from wallet
        let balances = wallet.get_all_balances().await?;

        // Update positions from current balances
        self.tracker.update_positions(&balances, &mut self.current_portfolio).await?;

        // Get recent transactions for analysis
        let transactions = cache.get_recent_transactions(100).await?;

        // Calculate performance metrics
        self.current_portfolio.performance_metrics = self.analyzer.calculate_performance_metrics(
            &self.current_portfolio.positions,
            &transactions
        );

        // Update diversification and risk scores
        self.current_portfolio.diversification_score =
            self.analyzer.calculate_diversification_score(&self.current_portfolio.positions);
        self.current_portfolio.risk_score = self.calculate_risk_score().await?;

        // Update timestamp
        self.current_portfolio.last_updated = chrono::Utc::now();

        Ok(())
    }

    /// Calculate overall risk score for the portfolio
    async fn calculate_risk_score(&self) -> BotResult<f64> {
        // Simple risk calculation for now
        let mut total_risk = 0.0;
        let mut total_value = 0.0;

        for position in &self.current_portfolio.positions {
            let position_value = position.current_value;
            let position_risk = self.get_position_risk_score(position).await.unwrap_or(0.5);

            total_risk += position_value * position_risk;
            total_value += position_value;
        }

        if total_value > 0.0 {
            Ok(total_risk / total_value)
        } else {
            Ok(0.0)
        }
    }

    /// Get risk score for a specific position
    async fn get_position_risk_score(&self, _position: &Position) -> BotResult<f64> {
        // Simplified risk scoring - would analyze volatility, liquidity, etc.
        Ok(0.3) // Default medium risk
    }

    /// Get portfolio health assessment
    pub fn get_portfolio_health(&self) -> PortfolioHealth {
        let positions_count = self.current_portfolio.positions.len();
        let total_value = self.current_portfolio.total_value;

        if total_value < 100.0 {
            PortfolioHealth::Poor
        } else if positions_count < 3 || self.current_portfolio.diversification_score < 0.3 {
            PortfolioHealth::Fair
        } else if self.current_portfolio.risk_score > 0.7 {
            PortfolioHealth::Good
        } else {
            PortfolioHealth::Excellent
        }
    }

    /// Get rebalancing recommendations
    pub async fn get_rebalancing_recommendations(&self) -> BotResult<Vec<RebalanceRecommendation>> {
        let mut recommendations = Vec::new();

        // Check for overconcentration
        for position in &self.current_portfolio.positions {
            let position_percentage = position.current_value / self.current_portfolio.total_value;

            if position_percentage > self.config.max_position_size {
                recommendations.push(RebalanceRecommendation {
                    token_mint: position.token_mint,
                    action: RebalanceAction::Reduce,
                    current_percentage: position_percentage,
                    target_percentage: self.config.max_position_size,
                    amount: position.current_value -
                    self.config.max_position_size * self.current_portfolio.total_value,
                    reason: format!(
                        "Position exceeds maximum allocation of {}%",
                        self.config.max_position_size * 100.0
                    ),
                    priority: if position_percentage > self.config.max_position_size * 1.5 {
                        9
                    } else {
                        6
                    },
                });
            }
        }

        // Check for underperforming positions
        for position in &self.current_portfolio.positions {
            if position.pnl_percentage < -0.2 {
                // Down more than 20%
                recommendations.push(RebalanceRecommendation {
                    token_mint: position.token_mint,
                    action: RebalanceAction::Review,
                    current_percentage: position.current_value / self.current_portfolio.total_value,
                    target_percentage: 0.0,
                    amount: 0.0,
                    reason: format!(
                        "Position down {:.1}% - review for potential exit",
                        position.pnl_percentage * 100.0
                    ),
                    priority: 7,
                });
            }
        }

        Ok(recommendations)
    }

    /// Display portfolio summary
    pub fn display_summary(&self) -> String {
        self.display.format_portfolio_summary(&self.current_portfolio)
    }

    /// Display detailed portfolio info
    pub fn display_detailed(&self) -> String {
        self.display.format_detailed_portfolio(&self.current_portfolio)
    }

    /// Track a new trade
    pub async fn track_trade(&mut self, transaction: &WalletTransaction) -> BotResult<()> {
        self.tracker.process_transaction(transaction, &mut self.current_portfolio).await
    }

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> &PerformanceMetrics {
        &self.current_portfolio.performance_metrics
    }

    /// Get top performing positions
    pub fn get_top_performers(&self, limit: usize) -> Vec<&Position> {
        let mut positions: Vec<&Position> = self.current_portfolio.positions.iter().collect();
        positions.sort_by(|a, b|
            b.pnl_percentage.partial_cmp(&a.pnl_percentage).unwrap_or(std::cmp::Ordering::Equal)
        );
        positions.into_iter().take(limit).collect()
    }

    /// Get worst performing positions
    pub fn get_worst_performers(&self, limit: usize) -> Vec<&Position> {
        let mut positions: Vec<&Position> = self.current_portfolio.positions.iter().collect();
        positions.sort_by(|a, b|
            a.pnl_percentage.partial_cmp(&b.pnl_percentage).unwrap_or(std::cmp::Ordering::Equal)
        );
        positions.into_iter().take(limit).collect()
    }

    /// Get positions by token symbol
    pub fn get_position_by_mint(&self, mint: &Pubkey) -> Option<&Position> {
        self.current_portfolio.positions.iter().find(|p| p.token_mint == *mint)
    }

    /// Calculate position allocation percentage
    pub fn get_position_allocation(&self, mint: &Pubkey) -> f64 {
        if let Some(position) = self.get_position_by_mint(mint) {
            if self.current_portfolio.total_value > 0.0 {
                position.current_value / self.current_portfolio.total_value
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Check if portfolio needs rebalancing
    pub fn needs_rebalancing(&self) -> bool {
        // Check if any position exceeds maximum allocation
        for position in &self.current_portfolio.positions {
            let allocation = position.current_value / self.current_portfolio.total_value;
            if allocation > self.config.max_position_size {
                return true;
            }
        }

        // Check diversification
        if self.current_portfolio.diversification_score < self.config.min_diversification_score {
            return true;
        }

        false
    }
}
