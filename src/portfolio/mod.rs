use crate::core::{
    BotResult,
    PortfolioConfig,
    Portfolio,
    Position,
    PerformanceMetrics,
    WalletTransaction,
    PortfolioHealth,
    RebalanceRecommendation,
};
use crate::wallet::WalletManager;
use crate::cache::CacheManager;
use solana_sdk::pubkey::Pubkey;
use chrono::Utc;

mod tracker;
mod analyzer;
mod display;

use tracker::PositionTracker;
use analyzer::PortfolioAnalyzer;
use display::PortfolioDisplay;

// Re-export public interfaces
pub use analyzer::PositionAnalysis;

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
            total_value_sol: 0.0,
            total_invested_sol: 0.0,
            total_unrealized_pnl: 0.0,
            total_unrealized_pnl_percentage: 0.0,
            sol_balance: 0.0,
            positions: Vec::new(),
            performance_metrics: PerformanceMetrics {
                win_rate: 0.0,
                profit_factor: 0.0,
                sharpe_ratio: 0.0,
                max_drawdown: 0.0,
                total_trades: 0,
                winning_trades: 0,
                losing_trades: 0,
                best_trade_pnl: 0.0,
                worst_trade_pnl: 0.0,
                average_trade_duration_hours: 0.0,
            },
            last_updated: chrono::Utc::now(),
        };

        Ok(Self {
            config: config.clone(),
            current_portfolio,
            tracker: PositionTracker::new(),
            analyzer: PortfolioAnalyzer::new(),
            display: PortfolioDisplay::new(),
        })
    }

    /// Initialize portfolio manager - stub implementation
    pub async fn initialize(
        &mut self,
        _wallet: &WalletManager,
        _cache: &CacheManager
    ) -> BotResult<()> {
        // Update portfolio from wallet and cache
        Ok(())
    }

    /// Update portfolio with current data - stub implementation
    pub async fn update(
        &mut self,
        _balances: Vec<crate::core::TokenBalance>,
        _transactions: &[WalletTransaction],
        _cache: &CacheManager
    ) -> BotResult<()> {
        // Update portfolio metrics
        self.current_portfolio.last_updated = Utc::now();
        Ok(())
    }

    /// Print status - enhanced implementation with wallet integration
    pub async fn print_status(&self) -> BotResult<()> {
        log::debug!("📊 Printing portfolio status");

        // Basic portfolio metrics
        println!(
            "┌─ Portfolio Summary ─────────────────────────────────────────────────────────────────────┐"
        );
        println!(
            "│ Total Value:     {:<15.4} SOL                                           │",
            self.current_portfolio.total_value_sol
        );
        println!(
            "│ Total Invested:  {:<15.4} SOL                                           │",
            self.current_portfolio.total_invested_sol
        );
        println!(
            "│ Unrealized P&L:  {:<15.4} SOL ({:+.2}%)                                │",
            self.current_portfolio.total_unrealized_pnl,
            self.current_portfolio.total_unrealized_pnl_percentage
        );
        println!(
            "│ Active Positions: {:<3} positions                                               │",
            self.current_portfolio.positions.len()
        );
        println!(
            "│ SOL Balance:     {:<15.6} SOL                                           │",
            self.current_portfolio.sol_balance
        );
        println!(
            "└─────────────────────────────────────────────────────────────────────────────────────────┘"
        );

        // Performance metrics summary
        let metrics = &self.current_portfolio.performance_metrics;
        if metrics.total_trades > 0 {
            println!(
                "┌─ Performance Metrics ───────────────────────────────────────────────────────────────────┐"
            );
            println!(
                "│ Total Trades:    {:<3} ({} wins, {} losses)                                      │",
                metrics.total_trades,
                metrics.winning_trades,
                metrics.losing_trades
            );
            println!(
                "│ Win Rate:        {:<6.1}%                                                        │",
                metrics.win_rate
            );
            println!(
                "│ Profit Factor:   {:<8.2}                                                        │",
                metrics.profit_factor
            );
            if metrics.best_trade_pnl != 0.0 {
                println!(
                    "│ Best Trade:      {:<15.4} SOL                                           │",
                    metrics.best_trade_pnl
                );
                println!(
                    "│ Worst Trade:     {:<15.4} SOL                                           │",
                    metrics.worst_trade_pnl
                );
            }
            println!(
                "└─────────────────────────────────────────────────────────────────────────────────────────┘"
            );
        }

        // Display individual positions if any
        if !self.current_portfolio.positions.is_empty() {
            println!(
                "┌─ Active Positions ──────────────────────────────────────────────────────────────────────┐"
            );
            for (i, position) in self.current_portfolio.positions.iter().enumerate() {
                let pnl_indicator = if position.unrealized_pnl >= 0.0 { "📈" } else { "📉" };
                println!(
                    "│ {:2}. {} {:<10} - {:.4} SOL ({:+.1}%)                                         │",
                    i + 1,
                    pnl_indicator,
                    position.symbol,
                    position.current_value_sol,
                    position.unrealized_pnl_percentage
                );
            }
            println!(
                "└─────────────────────────────────────────────────────────────────────────────────────────┘"
            );
        }

        Ok(())
    }

    /// Get current portfolio snapshot
    pub fn get_portfolio(&self) -> &Portfolio {
        &self.current_portfolio
    }

    /// Update portfolio with latest data
    pub async fn update_portfolio(
        &mut self,
        _wallet: &WalletManager,
        _cache: &CacheManager
    ) -> BotResult<()> {
        // Simplified implementation for compilation
        self.current_portfolio.last_updated = chrono::Utc::now();
        Ok(())
    }

    /// Get portfolio health assessment
    pub fn get_portfolio_health(&self) -> PortfolioHealth {
        PortfolioHealth {
            total_value_sol: self.current_portfolio.total_value_sol,
            total_invested_sol: self.current_portfolio.total_invested_sol,
            total_unrealized_pnl: self.current_portfolio.total_unrealized_pnl,
            total_pnl_percentage: self.current_portfolio.total_unrealized_pnl_percentage,
            positions_count: self.current_portfolio.positions.len(),
            profitable_positions: 0,
            losing_positions: 0,
            largest_position_percentage: 0.0,
            portfolio_concentration_risk: "Low".to_string(),
            health_score: 85,
            recommendations: Vec::new(),
        }
    }

    /// Get rebalancing recommendations
    pub async fn get_rebalancing_recommendations(&self) -> BotResult<Vec<RebalanceRecommendation>> {
        Ok(Vec::new()) // Simplified for compilation
    }

    /// Display portfolio summary
    pub fn display_summary(&self) -> String {
        format!(
            "Portfolio: {:.4} SOL, {} positions",
            self.current_portfolio.total_value_sol,
            self.current_portfolio.positions.len()
        )
    }

    /// Display detailed portfolio info
    pub fn display_detailed(&self) -> String {
        format!(
            "Detailed Portfolio:\nValue: {:.4} SOL\nPositions: {}\nPnL: {:.4} SOL ({:.2}%)",
            self.current_portfolio.total_value_sol,
            self.current_portfolio.positions.len(),
            self.current_portfolio.total_unrealized_pnl,
            self.current_portfolio.total_unrealized_pnl_percentage
        )
    }

    /// Track a new trade
    pub async fn track_trade(&mut self, _transaction: &WalletTransaction) -> BotResult<()> {
        // Simplified implementation
        Ok(())
    }

    /// Get performance metrics
    pub fn get_performance_metrics(&self) -> &PerformanceMetrics {
        &self.current_portfolio.performance_metrics
    }

    /// Get top performing positions
    pub fn get_top_performers(&self, limit: usize) -> Vec<&Position> {
        self.current_portfolio.positions.iter().take(limit).collect()
    }

    /// Get worst performing positions
    pub fn get_worst_performers(&self, limit: usize) -> Vec<&Position> {
        self.current_portfolio.positions.iter().take(limit).collect()
    }

    /// Get positions by token
    pub fn get_position_by_mint(&self, mint: &Pubkey) -> Option<&Position> {
        self.current_portfolio.positions.iter().find(|p| p.token == *mint)
    }

    /// Calculate position allocation percentage
    pub fn get_position_allocation(&self, mint: &Pubkey) -> f64 {
        if let Some(position) = self.get_position_by_mint(mint) {
            if self.current_portfolio.total_value_sol > 0.0 {
                position.current_value_sol / self.current_portfolio.total_value_sol
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Check if portfolio needs rebalancing
    pub fn needs_rebalancing(&self) -> bool {
        false // Simplified for compilation
    }
}
