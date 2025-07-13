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
    current_portfolio: Portfolio,
    tracker: PositionTracker,
    analyzer: PerformanceAnalyzer,
    display: PortfolioDisplay,
}

impl PortfolioManager {
    /// Create a new portfolio manager
    pub fn new(config: &PortfolioConfig) -> BotResult<Self> {
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
            last_updated: Utc::now(),
        };

        let tracker = PositionTracker::new();
        let analyzer = PerformanceAnalyzer::new();
        let display = PortfolioDisplay::new();

        Ok(Self {
            config: config.clone(),
            current_portfolio,
            tracker,
            analyzer,
            display,
        })
    }

    /// Initialize the portfolio manager
    pub async fn initialize(
        &mut self,
        wallet: &WalletManager,
        cache: &CacheManager
    ) -> BotResult<()> {
        log::info!("📊 Initializing portfolio manager...");

        // Load current balances and build initial portfolio
        let balances = wallet.get_all_balances().await?;
        let transactions = wallet.get_recent_transactions().await?;

        self.update_portfolio_from_data(balances, transactions, cache).await?;

        log::info!(
            "✅ Portfolio manager initialized with {} positions",
            self.current_portfolio.positions.len()
        );
        Ok(())
    }

    /// Update portfolio with new data
    pub async fn update(
        &mut self,
        balances: Vec<TokenBalance>,
        transactions: Vec<WalletTransaction>,
        cache: &CacheManager
    ) -> BotResult<()> {
        self.update_portfolio_from_data(balances, transactions, cache).await?;

        // Store portfolio snapshot in cache
        cache.store_balance_snapshot(
            &Pubkey::from_str(
                &balances
                    .first()
                    .map(|b| b.mint.to_string())
                    .unwrap_or_default()
            ).unwrap_or_default(),
            &balances
        ).await?;

        Ok(())
    }

    /// Update portfolio from balance and transaction data
    async fn update_portfolio_from_data(
        &mut self,
        balances: Vec<TokenBalance>,
        transactions: Vec<WalletTransaction>,
        cache: &CacheManager
    ) -> BotResult<()> {
        // Update SOL balance
        if
            let Some(sol_balance) = balances
                .iter()
                .find(|b| b.mint.to_string() == crate::core::WSOL_MINT)
        {
            self.current_portfolio.sol_balance = sol_balance.ui_amount;
        }

        // Track positions from balances
        let positions = self.tracker.build_positions_from_balances(&balances, &transactions).await?;

        // Update current positions
        self.current_portfolio.positions = positions;

        // Calculate portfolio totals
        self.calculate_portfolio_totals().await?;

        // Update performance metrics
        self.current_portfolio.performance_metrics = self.analyzer.calculate_performance_metrics(
            &self.current_portfolio,
            &transactions
        ).await?;

        // Update timestamp
        self.current_portfolio.last_updated = Utc::now();

        Ok(())
    }

    /// Calculate portfolio totals
    async fn calculate_portfolio_totals(&mut self) -> BotResult<()> {
        let mut total_value = self.current_portfolio.sol_balance;
        let mut total_invested = 0.0;
        let mut total_unrealized_pnl = 0.0;

        for position in &self.current_portfolio.positions {
            total_value += position.current_value_sol;
            total_invested += position.total_invested_sol;
            total_unrealized_pnl += position.unrealized_pnl;
        }

        self.current_portfolio.total_value_sol = total_value;
        self.current_portfolio.total_invested_sol = total_invested;
        self.current_portfolio.total_unrealized_pnl = total_unrealized_pnl;

        // Calculate total PnL percentage
        if total_invested > 0.0 {
            self.current_portfolio.total_unrealized_pnl_percentage =
                (total_unrealized_pnl / total_invested) * 100.0;
        } else {
            self.current_portfolio.total_unrealized_pnl_percentage = 0.0;
        }

        Ok(())
    }

    /// Print portfolio status to console
    pub async fn print_status(&self) -> BotResult<()> {
        let health = self.analyzer.analyze_portfolio_health(&self.current_positions);
        self.display.display_portfolio_overview(&health, &self.current_positions);

        if self.config.show_detailed_breakdown {
            // Show individual position details
            for position in &self.current_positions {
                let analysis = self.analyzer.analyze_position_performance(position);
                self.display.display_position_details(position, &analysis);
            }

            // Show rebalance recommendations
            let recommendations = self.analyzer.generate_rebalance_recommendations(
                &self.current_positions
            ).await?;
            self.display.display_rebalance_recommendations(&recommendations);
        }

        Ok(())
    }

    /// Get current portfolio
    pub fn get_portfolio(&self) -> &Portfolio {
        &self.current_portfolio
    }

    /// Get specific position by token
    pub fn get_position(&self, token: &Pubkey) -> Option<&Position> {
        self.current_portfolio.positions.iter().find(|p| p.token == *token)
    }

    /// Get top positions by value
    pub fn get_top_positions(&self, limit: usize) -> Vec<&Position> {
        let mut positions = self.current_portfolio.positions.iter().collect::<Vec<_>>();
        positions.sort_by(|a, b|
            b.current_value_sol
                .partial_cmp(&a.current_value_sol)
                .unwrap_or(std::cmp::Ordering::Equal)
        );
        positions.into_iter().take(limit).collect()
    }

    /// Get positions with profit/loss above threshold
    pub fn get_positions_with_pnl_above(&self, threshold_percentage: f64) -> Vec<&Position> {
        self.current_portfolio.positions
            .iter()
            .filter(|p| p.unrealized_pnl_percentage.abs() >= threshold_percentage)
            .collect()
    }

    /// Get portfolio diversification score
    pub fn get_diversification_score(&self) -> f64 {
        if self.current_portfolio.positions.is_empty() {
            return 1.0;
        }

        let total_value = self.current_portfolio.positions
            .iter()
            .map(|p| p.current_value_sol)
            .sum::<f64>();

        if total_value == 0.0 {
            return 1.0;
        }

        // Calculate Herfindahl-Hirschman Index (HHI) for concentration
        let hhi: f64 = self.current_portfolio.positions
            .iter()
            .map(|p| {
                let weight = p.current_value_sol / total_value;
                weight * weight
            })
            .sum();

        // Convert HHI to diversification score (1 - HHI)
        (1.0 - hhi).max(0.0)
    }

    /// Check for rebalancing opportunities
    pub async fn check_rebalancing_opportunities(&self) -> BotResult<Vec<RebalanceRecommendation>> {
        self.analyzer.generate_rebalance_recommendations(&self.current_positions).await
    }

    /// Get portfolio health score
    pub fn get_portfolio_health_score(&self) -> PortfolioHealth {
        self.analyzer.analyze_portfolio_health(&self.current_positions)
    }
}
