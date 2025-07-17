use crate::logger::Logger;
use crate::types::WalletPosition;
use super::portfolio::{ PortfolioSummary, PerformanceMetrics };
use anyhow::Result;
use colored::*;
use std::collections::HashMap;
use tabled::{
    Table,
    Tabled,
    settings::{ Style, Alignment, Modify, object::{ Rows, Columns }, Width },
};

#[derive(Clone)]
pub struct TableDisplay;

#[derive(Tabled)]
struct PositionRow {
    #[tabled(rename = "Token")]
    token: String,
    #[tabled(rename = "Balance")]
    balance: String,
    #[tabled(rename = "Value (SOL)")]
    value_sol: String,
    #[tabled(rename = "P&L (SOL)")]
    pnl_sol: String,
    #[tabled(rename = "ROI %")]
    roi_percent: String,
    #[tabled(rename = "Status")]
    status: String,
}

#[derive(Tabled)]
struct SummaryRow {
    #[tabled(rename = "Metric")]
    metric: String,
    #[tabled(rename = "Value")]
    value: String,
    #[tabled(rename = "Details")]
    details: String,
}

#[derive(Tabled)]
struct PerformanceRow {
    #[tabled(rename = "Category")]
    category: String,
    #[tabled(rename = "Count/Value")]
    count_value: String,
    #[tabled(rename = "Percentage/Rate")]
    percentage: String,
}

impl TableDisplay {
    pub fn new() -> Self {
        Self
    }

    /// Display current positions using a professional table format
    pub async fn show_positions_table(
        &self,
        positions: &HashMap<String, WalletPosition>
    ) -> Result<()> {
        if positions.is_empty() {
            Logger::wallet("📋 No token positions to display");
            return Ok(());
        }

        // Sort positions by value (highest first)
        let mut sorted_positions: Vec<_> = positions.values().collect();
        sorted_positions.sort_by(|a, b| {
            b.value_sol.unwrap_or(0.0).partial_cmp(&a.value_sol.unwrap_or(0.0)).unwrap()
        });

        // Create table rows
        let mut rows = Vec::new();
        for position in sorted_positions.iter() {
            let actual_balance =
                (position.balance as f64) / (10_f64).powi(position.decimals as i32);
            let value_sol = position.value_sol.unwrap_or(0.0);
            let pnl_sol = position.pnl_sol.unwrap_or(0.0);
            let pnl_percentage = position.pnl_percentage.unwrap_or(0.0);

            let token_display = if position.mint.len() > 12 {
                format!("{}...", &position.mint[..12])
            } else {
                position.mint.clone()
            };

            let status = if pnl_sol >= 0.0 { "🟢 PROFIT" } else { "🔴 LOSS" };
            let pnl_sign = if pnl_sol >= 0.0 { "+" } else { "" };
            let roi_sign = if pnl_percentage >= 0.0 { "+" } else { "" };

            rows.push(PositionRow {
                token: token_display,
                balance: format!("{:.6}", actual_balance),
                value_sol: format!("{:.6}", value_sol),
                pnl_sol: format!("{}{:.6}", pnl_sign, pnl_sol),
                roi_percent: format!("{}{}%", roi_sign, pnl_percentage),
                status: status.to_string(),
            });
        }

        // Create and style the table
        let mut table = Table::new(rows);
        table
            .with(Style::rounded())
            .with(Modify::new(Rows::first()).with(Alignment::center()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()))
            .with(Modify::new(Columns::new(1..=4)).with(Alignment::right()))
            .with(Modify::new(Columns::single(5)).with(Alignment::center()))
            .with(Width::wrap(80).keep_words());

        println!();
        Logger::wallet("🏦 CURRENT TOKEN POSITIONS");
        println!();
        println!("{}", table);

        // Calculate and display summary
        let total_value: f64 = sorted_positions
            .iter()
            .map(|p| p.value_sol.unwrap_or(0.0))
            .sum();
        let total_pnl: f64 = sorted_positions
            .iter()
            .map(|p| p.pnl_sol.unwrap_or(0.0))
            .sum();
        let avg_roi = if !sorted_positions.is_empty() {
            sorted_positions
                .iter()
                .map(|p| p.pnl_percentage.unwrap_or(0.0))
                .sum::<f64>() / (sorted_positions.len() as f64)
        } else {
            0.0
        };

        println!();
        Logger::success(
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
        println!();

        Ok(())
    }

    /// Display portfolio summary in a table format
    pub async fn show_portfolio_summary_table(&self, summary: &PortfolioSummary) -> Result<()> {
        let rows = vec![
            SummaryRow {
                metric: "💎 SOL Balance".to_string(),
                value: format!("{:.6} SOL", summary.sol_balance),
                details: "Native SOL in wallet".to_string(),
            },
            SummaryRow {
                metric: "🏦 Total Portfolio Value".to_string(),
                value: format!("{:.6} SOL", summary.total_value_sol),
                details: "SOL + All token positions".to_string(),
            },
            SummaryRow {
                metric: "💰 Total Invested".to_string(),
                value: format!("{:.6} SOL", summary.total_invested_sol),
                details: "Historical investment amount".to_string(),
            },
            SummaryRow {
                metric: (
                    if summary.total_pnl_sol >= 0.0 {
                        "🟢 Total P&L"
                    } else {
                        "🔴 Total P&L"
                    }
                ).to_string(),
                value: format!(
                    "{}{:.6} SOL",
                    if summary.total_pnl_sol >= 0.0 {
                        "+"
                    } else {
                        ""
                    },
                    summary.total_pnl_sol
                ),
                details: format!(
                    "{}{}% ROI",
                    if summary.roi_percentage >= 0.0 {
                        "+"
                    } else {
                        ""
                    },
                    summary.roi_percentage
                ),
            },
            SummaryRow {
                metric: "💰 Realized P&L".to_string(),
                value: format!(
                    "{}{:.6} SOL",
                    if summary.realized_pnl_sol >= 0.0 {
                        "+"
                    } else {
                        ""
                    },
                    summary.realized_pnl_sol
                ),
                details: "Closed position profits".to_string(),
            },
            SummaryRow {
                metric: "📊 Unrealized P&L".to_string(),
                value: format!(
                    "{}{:.6} SOL",
                    if summary.unrealized_pnl_sol >= 0.0 {
                        "+"
                    } else {
                        ""
                    },
                    summary.unrealized_pnl_sol
                ),
                details: "Open position profits".to_string(),
            },
            SummaryRow {
                metric: "🎯 Active Positions".to_string(),
                value: summary.active_positions.to_string(),
                details: format!("of {} total positions", summary.total_positions),
            },
            SummaryRow {
                metric: "🏆 Largest Position".to_string(),
                value: format!("{:.6} SOL", summary.largest_position_value),
                details: "Biggest single holding".to_string(),
            }
        ];

        let mut table = Table::new(rows);
        table
            .with(Style::rounded())
            .with(Modify::new(Rows::first()).with(Alignment::center()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()))
            .with(Modify::new(Columns::single(1)).with(Alignment::right()))
            .with(Modify::new(Columns::single(2)).with(Alignment::left()))
            .with(Width::wrap(80).keep_words());

        println!();
        Logger::wallet("💰 PORTFOLIO SUMMARY");
        println!();
        println!("{}", table);
        println!();

        Ok(())
    }

    /// Display performance metrics in a table format
    pub async fn show_performance_metrics_table(&self, metrics: &PerformanceMetrics) -> Result<()> {
        let rows = vec![
            PerformanceRow {
                category: "📊 Total Positions".to_string(),
                count_value: metrics.total_tokens.to_string(),
                percentage: "100%".to_string(),
            },
            PerformanceRow {
                category: "🟢 Profitable Positions".to_string(),
                count_value: metrics.profitable_tokens.to_string(),
                percentage: if metrics.total_tokens > 0 {
                    format!(
                        "{:.1}%",
                        ((metrics.profitable_tokens as f64) / (metrics.total_tokens as f64)) * 100.0
                    )
                } else {
                    "0.0%".to_string()
                },
            },
            PerformanceRow {
                category: "🔴 Loss-Making Positions".to_string(),
                count_value: metrics.loss_making_tokens.to_string(),
                percentage: if metrics.total_tokens > 0 {
                    format!(
                        "{:.1}%",
                        ((metrics.loss_making_tokens as f64) / (metrics.total_tokens as f64)) *
                            100.0
                    )
                } else {
                    "0.0%".to_string()
                },
            },
            PerformanceRow {
                category: "🎯 Win Rate".to_string(),
                count_value: format!("{:.1}%", metrics.win_rate),
                percentage: "Success ratio".to_string(),
            },
            PerformanceRow {
                category: "📈 Average ROI".to_string(),
                count_value: format!(
                    "{}{}%",
                    if metrics.average_roi >= 0.0 {
                        "+"
                    } else {
                        ""
                    },
                    metrics.average_roi
                ),
                percentage: "Per position".to_string(),
            },
            PerformanceRow {
                category: "💰 Average Position Size".to_string(),
                count_value: format!("{:.6} SOL", metrics.average_position_size_sol),
                percentage: "Investment size".to_string(),
            }
        ];

        let mut table = Table::new(rows);
        table
            .with(Style::rounded())
            .with(Modify::new(Rows::first()).with(Alignment::center()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()))
            .with(Modify::new(Columns::single(1)).with(Alignment::right()))
            .with(Modify::new(Columns::single(2)).with(Alignment::left()))
            .with(Width::wrap(80).keep_words());

        println!();
        Logger::wallet("🎯 PERFORMANCE METRICS");
        println!();
        println!("{}", table);

        // Show best and worst performers
        if let Some((mint, roi)) = &metrics.best_performer {
            println!();
            Logger::success(&format!("🏆 Best Performer:  {}... (+{:.2}%)", &mint[..8], roi));
        }

        if let Some((mint, roi)) = &metrics.worst_performer {
            Logger::error(&format!("💀 Worst Performer: {}... ({:.2}%)", &mint[..8], roi));
        }

        println!();
        Ok(())
    }

    /// Display top performers in a compact table
    pub async fn show_top_performers_table(
        &self,
        positions: &[WalletPosition],
        title: &str
    ) -> Result<()> {
        if positions.is_empty() {
            return Ok(());
        }

        let rows: Vec<PositionRow> = positions
            .iter()
            .enumerate()
            .map(|(i, position)| {
                let actual_balance =
                    (position.balance as f64) / (10_f64).powi(position.decimals as i32);
                let value_sol = position.value_sol.unwrap_or(0.0);
                let pnl_sol = position.pnl_sol.unwrap_or(0.0);
                let pnl_percentage = position.pnl_percentage.unwrap_or(0.0);

                let token_display = format!("{}. {}...", i + 1, &position.mint[..8]);
                let status = if pnl_sol >= 0.0 { "🟢" } else { "🔴" };
                let pnl_sign = if pnl_sol >= 0.0 { "+" } else { "" };
                let roi_sign = if pnl_percentage >= 0.0 { "+" } else { "" };

                PositionRow {
                    token: token_display,
                    balance: format!("{:.4}", actual_balance),
                    value_sol: format!("{:.6}", value_sol),
                    pnl_sol: format!("{}{:.6}", pnl_sign, pnl_sol),
                    roi_percent: format!("{}{}%", roi_sign, pnl_percentage),
                    status: status.to_string(),
                }
            })
            .collect();

        let mut table = Table::new(rows);
        table
            .with(Style::rounded())
            .with(Modify::new(Rows::first()).with(Alignment::center()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()))
            .with(Modify::new(Columns::new(1..=4)).with(Alignment::right()))
            .with(Modify::new(Columns::single(5)).with(Alignment::center()))
            .with(Width::wrap(80).keep_words());

        println!();
        Logger::wallet(title);
        println!();
        println!("{}", table);
        println!();

        Ok(())
    }

    /// Display a compact dashboard overview
    pub async fn show_dashboard(
        &self,
        summary: &PortfolioSummary,
        metrics: &PerformanceMetrics,
        _positions: &HashMap<String, WalletPosition>
    ) -> Result<()> {
        println!();
        println!("{}", "═".repeat(80).bright_blue().bold());
        println!("{:^80}", "📊 SCREENERBOT WALLET DASHBOARD 📊".bright_blue().bold());
        println!("{}", "═".repeat(80).bright_blue().bold());

        // Quick stats row
        let stats = vec![
            SummaryRow {
                metric: "💎 SOL Balance".to_string(),
                value: format!("{:.6}", summary.sol_balance),
                details: "SOL".to_string(),
            },
            SummaryRow {
                metric: "🏦 Portfolio Value".to_string(),
                value: format!("{:.6}", summary.total_value_sol),
                details: "SOL".to_string(),
            },
            SummaryRow {
                metric: (
                    if summary.total_pnl_sol >= 0.0 {
                        "🟢 Total P&L"
                    } else {
                        "🔴 Total P&L"
                    }
                ).to_string(),
                value: format!(
                    "{}{:.6}",
                    if summary.total_pnl_sol >= 0.0 {
                        "+"
                    } else {
                        ""
                    },
                    summary.total_pnl_sol
                ),
                details: "SOL".to_string(),
            },
            SummaryRow {
                metric: "🎯 Win Rate".to_string(),
                value: format!("{:.1}", metrics.win_rate),
                details: "%".to_string(),
            }
        ];

        let mut table = Table::new(stats);
        table
            .with(Style::modern())
            .with(Modify::new(Rows::first()).with(Alignment::center()))
            .with(Modify::new(Columns::single(0)).with(Alignment::left()))
            .with(Modify::new(Columns::single(1)).with(Alignment::right()))
            .with(Modify::new(Columns::single(2)).with(Alignment::center()));

        println!("{}", table);

        // Active positions count
        println!();
        Logger::info(
            &format!(
                "Active Positions: {} | Total Positions: {} | Avg ROI: {}{}%",
                summary.active_positions,
                summary.total_positions,
                if metrics.average_roi >= 0.0 {
                    "+"
                } else {
                    ""
                },
                metrics.average_roi
            )
        );

        println!("{}", "═".repeat(80).bright_blue().bold());
        println!();

        Ok(())
    }

    /// Alternative compact table style using comfy-table
    pub async fn show_compact_positions(
        &self,
        positions: &HashMap<String, WalletPosition>
    ) -> Result<()> {
        use comfy_table::*;

        if positions.is_empty() {
            Logger::wallet("📋 No positions to display");
            return Ok(());
        }

        let mut table = comfy_table::Table::new();
        table
            .load_preset(presets::UTF8_FULL)
            .apply_modifier(modifiers::UTF8_ROUND_CORNERS)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_width(80);

        // Header
        table.set_header(
            vec![
                Cell::new("Token").add_attribute(Attribute::Bold),
                Cell::new("Balance").add_attribute(Attribute::Bold),
                Cell::new("Value (SOL)").add_attribute(Attribute::Bold),
                Cell::new("P&L").add_attribute(Attribute::Bold),
                Cell::new("ROI").add_attribute(Attribute::Bold)
            ]
        );

        // Sort positions by value
        let mut sorted_positions: Vec<_> = positions.values().collect();
        sorted_positions.sort_by(|a, b| {
            b.value_sol.unwrap_or(0.0).partial_cmp(&a.value_sol.unwrap_or(0.0)).unwrap()
        });

        // Add rows
        for position in sorted_positions.iter().take(10) {
            // Show top 10
            let actual_balance =
                (position.balance as f64) / (10_f64).powi(position.decimals as i32);
            let value_sol = position.value_sol.unwrap_or(0.0);
            let pnl_sol = position.pnl_sol.unwrap_or(0.0);
            let pnl_percentage = position.pnl_percentage.unwrap_or(0.0);

            let token_display = if position.mint.len() > 12 {
                format!("{}...", &position.mint[..12])
            } else {
                position.mint.clone()
            };

            let pnl_cell = if pnl_sol >= 0.0 {
                Cell::new(format!("+{:.6}", pnl_sol)).fg(comfy_table::Color::Green)
            } else {
                Cell::new(format!("{:.6}", pnl_sol)).fg(comfy_table::Color::Red)
            };

            let roi_cell = if pnl_percentage >= 0.0 {
                Cell::new(format!("+{:.2}%", pnl_percentage)).fg(comfy_table::Color::Green)
            } else {
                Cell::new(format!("{:.2}%", pnl_percentage)).fg(comfy_table::Color::Red)
            };

            table.add_row(
                vec![
                    Cell::new(&token_display),
                    Cell::new(format!("{:.6}", actual_balance)),
                    Cell::new(format!("{:.6}", value_sol)),
                    pnl_cell,
                    roi_cell
                ]
            );
        }

        println!();
        Logger::wallet("🏦 TOP POSITIONS (Compact View)");
        println!();
        println!("{}", table);
        println!();

        Ok(())
    }
}
