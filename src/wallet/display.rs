use crate::logger::Logger;
use crate::types::WalletPosition;
use super::portfolio::{ PortfolioSummary, PerformanceMetrics };
use anyhow::Result;
use std::collections::HashMap;

#[derive(Clone)]
pub struct ConsoleDisplay;

impl ConsoleDisplay {
    pub fn new() -> Self {
        Self
    }

    /// Display current token positions in a formatted table
    pub async fn show_current_positions(
        &self,
        positions: &HashMap<String, WalletPosition>
    ) -> Result<()> {
        if positions.is_empty() {
            Logger::wallet("📋 No token positions to display");
            return Ok(());
        }

        Logger::separator();
        Logger::wallet("🏦 CURRENT TOKEN POSITIONS");
        Logger::separator();

        // Sort positions by value (highest first)
        let mut sorted_positions: Vec<_> = positions.values().collect();
        sorted_positions.sort_by(|a, b| {
            b.value_sol.unwrap_or(0.0).partial_cmp(&a.value_sol.unwrap_or(0.0)).unwrap()
        });

        // Header
        Logger::wallet(
            "┌─────────────────┬──────────────┬──────────────┬──────────────┬──────────────┐"
        );
        Logger::wallet(
            "│ Token           │ Balance      │ Value (SOL)  │ P&L (SOL)    │ ROI %        │"
        );
        Logger::wallet(
            "├─────────────────┼──────────────┼──────────────┼──────────────┼──────────────┤"
        );

        let mut total_value = 0.0;
        let mut total_pnl = 0.0;

        for (i, position) in sorted_positions.iter().enumerate() {
            let actual_balance =
                (position.balance as f64) / (10_f64).powi(position.decimals as i32);
            let value_sol = position.value_sol.unwrap_or(0.0);
            let pnl_sol = position.pnl_sol.unwrap_or(0.0);
            let pnl_percentage = position.pnl_percentage.unwrap_or(0.0);

            total_value += value_sol;
            total_pnl += pnl_sol;

            // Color coding for P&L
            let pnl_color = if pnl_percentage >= 0.0 { "🟢" } else { "🔴" };
            let pnl_sign = if pnl_percentage >= 0.0 { "+" } else { "" };

            let token_display = if let Some(symbol) = &position.symbol {
                if let Some(name) = &position.name {
                    if name.len() > 12 {
                        format!("{} ({})", symbol, &name[..8])
                    } else {
                        format!("{} ({})", symbol, name)
                    }
                } else {
                    symbol.clone()
                }
            } else if position.mint.len() > 15 {
                format!("{}...", &position.mint[..12])
            } else {
                position.mint.clone()
            };

            Logger::wallet(
                &format!(
                    "│ {:<15} │ {:<12.6} │ {:<12.6} │ {}{:<11.6} │ {}{}{:<10.2} │",
                    token_display,
                    actual_balance,
                    value_sol,
                    if pnl_sol >= 0.0 {
                        " "
                    } else {
                        ""
                    },
                    pnl_sol,
                    pnl_color,
                    pnl_sign,
                    pnl_percentage
                )
            );

            // Add separator every 5 rows for readability
            if (i + 1) % 5 == 0 && i + 1 < sorted_positions.len() {
                Logger::wallet(
                    "├─────────────────┼──────────────┼──────────────┼──────────────┼──────────────┤"
                );
            }
        }

        Logger::wallet(
            "└─────────────────┴──────────────┴──────────────┴──────────────┴──────────────┘"
        );

        // Summary
        let avg_roi = if !sorted_positions.is_empty() {
            sorted_positions
                .iter()
                .map(|p| p.pnl_percentage.unwrap_or(0.0))
                .sum::<f64>() / (sorted_positions.len() as f64)
        } else {
            0.0
        };

        Logger::wallet(
            &format!(
                "📊 SUMMARY: {} positions | {:.6} SOL total | {}{:.6} SOL P&L | {}{}% avg ROI",
                sorted_positions.len(),
                total_value,
                if total_pnl >= 0.0 {
                    "+"
                } else {
                    ""
                },
                total_pnl,
                if avg_roi >= 0.0 {
                    "+"
                } else {
                    ""
                },
                avg_roi
            )
        );

        Logger::separator();
        Ok(())
    }

    /// Display portfolio summary with key metrics
    pub async fn show_portfolio_summary(&self, summary: &PortfolioSummary) -> Result<()> {
        Logger::separator();
        Logger::wallet("💰 PORTFOLIO SUMMARY");
        Logger::separator();

        // Main metrics
        Logger::wallet(&format!("💎 SOL Balance:      {:.6} SOL", summary.sol_balance));
        Logger::wallet(&format!("🏦 Total Value:      {:.6} SOL", summary.total_value_sol));
        Logger::wallet(&format!("💰 Total Invested:   {:.6} SOL", summary.total_invested_sol));

        // P&L metrics with color coding
        let pnl_emoji = if summary.total_pnl_sol >= 0.0 { "🟢" } else { "🔴" };
        let pnl_sign = if summary.total_pnl_sol >= 0.0 { "+" } else { "" };
        let roi_sign = if summary.roi_percentage >= 0.0 { "+" } else { "" };

        Logger::wallet(
            &format!("{} Total P&L:       {}{:.6} SOL", pnl_emoji, pnl_sign, summary.total_pnl_sol)
        );
        Logger::wallet(&format!("📈 ROI:              {}{}%", roi_sign, summary.roi_percentage));

        Logger::separator();

        // Detailed breakdown
        Logger::wallet(
            &format!(
                "💰 Realized P&L:     {}{:.6} SOL",
                if summary.realized_pnl_sol >= 0.0 {
                    "+"
                } else {
                    ""
                },
                summary.realized_pnl_sol
            )
        );
        Logger::wallet(
            &format!(
                "📊 Unrealized P&L:   {}{:.6} SOL",
                if summary.unrealized_pnl_sol >= 0.0 {
                    "+"
                } else {
                    ""
                },
                summary.unrealized_pnl_sol
            )
        );

        Logger::separator();

        // Position metrics
        Logger::wallet(&format!("🎯 Active Positions: {}", summary.active_positions));
        Logger::wallet(&format!("📋 Total Positions:  {}", summary.total_positions));
        Logger::wallet(&format!("🏆 Largest Position: {:.6} SOL", summary.largest_position_value));

        Logger::separator();
        Ok(())
    }

    /// Display performance metrics
    pub async fn show_performance_metrics(&self, metrics: &PerformanceMetrics) -> Result<()> {
        Logger::separator();
        Logger::wallet("🎯 PERFORMANCE METRICS");
        Logger::separator();

        Logger::wallet(&format!("📊 Total Positions:    {}", metrics.total_tokens));
        Logger::wallet(
            &format!("🟢 Profitable:        {} ({:.1}%)", metrics.profitable_tokens, if
                metrics.total_tokens > 0
            {
                ((metrics.profitable_tokens as f64) / (metrics.total_tokens as f64)) * 100.0
            } else {
                0.0
            })
        );
        Logger::wallet(
            &format!("🔴 Loss Making:       {} ({:.1}%)", metrics.loss_making_tokens, if
                metrics.total_tokens > 0
            {
                ((metrics.loss_making_tokens as f64) / (metrics.total_tokens as f64)) * 100.0
            } else {
                0.0
            })
        );
        Logger::wallet(&format!("🎯 Win Rate:          {:.1}%", metrics.win_rate));
        Logger::wallet(
            &format!(
                "📈 Average ROI:       {}{}%",
                if metrics.average_roi >= 0.0 {
                    "+"
                } else {
                    ""
                },
                metrics.average_roi
            )
        );
        Logger::wallet(
            &format!("💰 Avg Position Size: {:.6} SOL", metrics.average_position_size_sol)
        );

        if let Some((mint, roi)) = &metrics.best_performer {
            Logger::wallet(&format!("🏆 Best Performer:    {}... (+{}%)", &mint[..8], roi));
        }

        if let Some((mint, roi)) = &metrics.worst_performer {
            Logger::wallet(&format!("💀 Worst Performer:   {}... ({}%)", &mint[..8], roi));
        }

        Logger::separator();
        Ok(())
    }

    /// Display top performing positions
    pub async fn show_top_positions(&self, positions: &[WalletPosition]) -> Result<()> {
        if positions.is_empty() {
            return Ok(());
        }

        Logger::separator();
        Logger::wallet("🏆 TOP PERFORMERS");
        Logger::separator();

        for (i, position) in positions.iter().enumerate() {
            let actual_balance =
                (position.balance as f64) / (10_f64).powi(position.decimals as i32);
            let value_sol = position.value_sol.unwrap_or(0.0);
            let pnl_percentage = position.pnl_percentage.unwrap_or(0.0);

            Logger::wallet(
                &format!(
                    "{}. {}... | {:.6} tokens | {:.6} SOL | +{}%",
                    i + 1,
                    &position.mint[..8],
                    actual_balance,
                    value_sol,
                    pnl_percentage
                )
            );
        }

        Logger::separator();
        Ok(())
    }

    /// Display worst performing positions
    pub async fn show_worst_positions(&self, positions: &[WalletPosition]) -> Result<()> {
        if positions.is_empty() {
            return Ok(());
        }

        Logger::separator();
        Logger::wallet("💀 WORST PERFORMERS");
        Logger::separator();

        for (i, position) in positions.iter().enumerate() {
            let actual_balance =
                (position.balance as f64) / (10_f64).powi(position.decimals as i32);
            let value_sol = position.value_sol.unwrap_or(0.0);
            let pnl_percentage = position.pnl_percentage.unwrap_or(0.0);

            Logger::wallet(
                &format!(
                    "{}. {}... | {:.6} tokens | {:.6} SOL | {}%",
                    i + 1,
                    &position.mint[..8],
                    actual_balance,
                    value_sol,
                    pnl_percentage
                )
            );
        }

        Logger::separator();
        Ok(())
    }

    /// Display real-time position updates (compact format)
    pub async fn show_position_update(
        &self,
        mint: &str,
        balance: f64,
        value_sol: f64,
        pnl_percentage: f64
    ) -> Result<()> {
        let pnl_emoji = if pnl_percentage >= 0.0 { "🟢" } else { "🔴" };
        let pnl_sign = if pnl_percentage >= 0.0 { "+" } else { "" };

        Logger::wallet(
            &format!(
                "{} {}... | {:.6} | {:.6} SOL | {}{}%",
                pnl_emoji,
                &mint[..8],
                balance,
                value_sol,
                pnl_sign,
                pnl_percentage
            )
        );

        Ok(())
    }

    /// Display loading/processing status
    pub async fn show_loading_status(&self, message: &str) -> Result<()> {
        Logger::wallet(&format!("⏳ {}", message));
        Ok(())
    }

    /// Display success status
    pub async fn show_success_status(&self, message: &str) -> Result<()> {
        Logger::success(&format!("✅ {}", message));
        Ok(())
    }

    /// Display error status
    pub async fn show_error_status(&self, message: &str) -> Result<()> {
        Logger::error(&format!("❌ {}", message));
        Ok(())
    }
}
