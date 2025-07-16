use crate::database::Database;
use crate::types::WalletPosition;
use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PortfolioSummary {
    pub sol_balance: f64,
    pub total_value_sol: f64,
    pub total_invested_sol: f64,
    pub total_pnl_sol: f64,
    pub unrealized_pnl_sol: f64,
    pub realized_pnl_sol: f64,
    pub roi_percentage: f64,
    pub active_positions: usize,
    pub total_positions: usize,
    pub largest_position_value: f64,
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub total_tokens: usize,
    pub profitable_tokens: usize,
    pub loss_making_tokens: usize,
    pub win_rate: f64,
    pub average_roi: f64,
    pub best_performer: Option<(String, f64)>,
    pub worst_performer: Option<(String, f64)>,
    pub total_trades: usize,
    pub average_position_size_sol: f64,
}

#[derive(Clone)]
pub struct PortfolioAnalyzer {
    #[allow(dead_code)]
    database: Arc<Database>,
}

impl PortfolioAnalyzer {
    pub fn new(database: Arc<Database>) -> Self {
        Self { database }
    }

    /// Calculate comprehensive portfolio summary
    pub async fn calculate_portfolio_summary(
        &self,
        positions: &HashMap<String, WalletPosition>,
        sol_balance: f64
    ) -> Result<PortfolioSummary> {
        let mut total_value_sol = sol_balance;
        let mut total_invested_sol = 0.0;
        let mut total_pnl_sol = 0.0;
        let mut total_unrealized_pnl = 0.0;
        let mut total_realized_pnl = 0.0;
        let mut active_positions = 0;
        let mut largest_position_value: f64 = 0.0;

        for position in positions.values() {
            if let Some(value) = position.value_sol {
                total_value_sol += value;
                largest_position_value = largest_position_value.max(value);

                if value > 0.001 {
                    // Consider positions > 0.001 SOL as active
                    active_positions += 1;
                }
            }

            if let Some(invested) = position.total_invested_sol {
                total_invested_sol += invested;
            }

            if let Some(pnl) = position.pnl_sol {
                total_pnl_sol += pnl;
            }

            if let Some(unrealized) = position.unrealized_pnl_sol {
                total_unrealized_pnl += unrealized;
            }

            if let Some(realized) = position.realized_pnl_sol {
                total_realized_pnl += realized;
            }
        }

        let roi_percentage = if total_invested_sol > 0.0 {
            (total_pnl_sol / total_invested_sol) * 100.0
        } else {
            0.0
        };

        Ok(PortfolioSummary {
            sol_balance,
            total_value_sol,
            total_invested_sol,
            total_pnl_sol,
            unrealized_pnl_sol: total_unrealized_pnl,
            realized_pnl_sol: total_realized_pnl,
            roi_percentage,
            active_positions,
            total_positions: positions.len(),
            largest_position_value,
        })
    }

    /// Get detailed performance metrics
    pub async fn get_performance_metrics(
        &self,
        positions: &HashMap<String, WalletPosition>
    ) -> Result<PerformanceMetrics> {
        let mut profitable_tokens = 0;
        let mut loss_making_tokens = 0;
        let mut total_roi = 0.0;
        let mut best_performer: Option<(String, f64)> = None;
        let mut worst_performer: Option<(String, f64)> = None;
        let mut total_value = 0.0;

        for (mint, position) in positions {
            if let Some(roi) = position.pnl_percentage {
                total_roi += roi;

                if roi > 0.0 {
                    profitable_tokens += 1;
                } else if roi < 0.0 {
                    loss_making_tokens += 1;
                }

                // Track best performer
                if let Some((_, best_roi)) = &best_performer {
                    if roi > *best_roi {
                        best_performer = Some((mint.clone(), roi));
                    }
                } else {
                    best_performer = Some((mint.clone(), roi));
                }

                // Track worst performer
                if let Some((_, worst_roi)) = &worst_performer {
                    if roi < *worst_roi {
                        worst_performer = Some((mint.clone(), roi));
                    }
                } else {
                    worst_performer = Some((mint.clone(), roi));
                }
            }

            if let Some(value) = position.value_sol {
                total_value += value;
            }
        }

        let total_tokens = positions.len();
        let win_rate = if total_tokens > 0 {
            ((profitable_tokens as f64) / (total_tokens as f64)) * 100.0
        } else {
            0.0
        };

        let average_roi = if total_tokens > 0 { total_roi / (total_tokens as f64) } else { 0.0 };

        let average_position_size_sol = if total_tokens > 0 {
            total_value / (total_tokens as f64)
        } else {
            0.0
        };

        Ok(PerformanceMetrics {
            total_tokens,
            profitable_tokens,
            loss_making_tokens,
            win_rate,
            average_roi,
            best_performer,
            worst_performer,
            total_trades: total_tokens, // Simplified - could be more detailed
            average_position_size_sol,
        })
    }

    /// Get top performing positions
    pub async fn get_top_positions(
        &self,
        positions: &HashMap<String, WalletPosition>,
        limit: usize
    ) -> Result<Vec<WalletPosition>> {
        let mut sorted_positions: Vec<_> = positions.values().cloned().collect();
        sorted_positions.sort_by(|a, b| {
            b.pnl_percentage.unwrap_or(0.0).partial_cmp(&a.pnl_percentage.unwrap_or(0.0)).unwrap()
        });
        Ok(sorted_positions.into_iter().take(limit).collect())
    }

    /// Get worst performing positions
    pub async fn get_worst_positions(
        &self,
        positions: &HashMap<String, WalletPosition>,
        limit: usize
    ) -> Result<Vec<WalletPosition>> {
        let mut sorted_positions: Vec<_> = positions.values().cloned().collect();
        sorted_positions.sort_by(|a, b| {
            a.pnl_percentage.unwrap_or(0.0).partial_cmp(&b.pnl_percentage.unwrap_or(0.0)).unwrap()
        });
        Ok(sorted_positions.into_iter().take(limit).collect())
    }

    /// Get positions by value (largest first)
    pub async fn get_positions_by_value(
        &self,
        positions: &HashMap<String, WalletPosition>,
        limit: usize
    ) -> Result<Vec<WalletPosition>> {
        let mut sorted_positions: Vec<_> = positions.values().cloned().collect();
        sorted_positions.sort_by(|a, b| {
            b.value_sol.unwrap_or(0.0).partial_cmp(&a.value_sol.unwrap_or(0.0)).unwrap()
        });
        Ok(sorted_positions.into_iter().take(limit).collect())
    }
}
