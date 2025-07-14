use crate::core::{ Position, PortfolioHealth, RebalanceRecommendation, RebalanceAction };
use crate::portfolio::analyzer::{ PositionAnalysis, DiversificationAnalysis };
use chrono::{ DateTime, Utc };
use std::collections::HashMap;

/// Portfolio display formatter for console output
#[derive(Debug)]
pub struct PortfolioDisplay {
    show_colors: bool,
    compact_mode: bool,
}

impl PortfolioDisplay {
    pub fn new() -> Self {
        Self {
            show_colors: true,
            compact_mode: false,
        }
    }

    pub fn with_colors(mut self, enabled: bool) -> Self {
        self.show_colors = enabled;
        self
    }

    pub fn compact(mut self) -> Self {
        self.compact_mode = true;
        self
    }

    /// Display complete portfolio overview
    pub fn display_portfolio_overview(&self, health: &PortfolioHealth, positions: &[Position]) {
        self.print_header("🎯 PORTFOLIO OVERVIEW");

        // Portfolio summary
        self.print_portfolio_summary(health);

        // Position details
        if !positions.is_empty() {
            println!();
            self.print_header("📊 POSITIONS");
            self.print_positions_table(positions);
        }

        // Recommendations
        if !health.recommendations.is_empty() {
            println!();
            self.print_header("💡 RECOMMENDATIONS");
            for recommendation in &health.recommendations {
                println!("   {}", recommendation);
            }
        }

        println!("{}", "═".repeat(80));
    }

    /// Display portfolio summary metrics
    fn print_portfolio_summary(&self, health: &PortfolioHealth) {
        let pnl_color = if health.total_unrealized_pnl >= 0.0 { "🟢" } else { "🔴" };
        let health_color = match health.health_score {
            80..=100 => "🟢",
            60..=79 => "🟡",
            40..=59 => "🟠",
            _ => "🔴",
        };

        println!("┌─ Portfolio Metrics ─────────────────────────────────────────────┐");
        println!(
            "│ Total Value:     {:<15.4} SOL                              │",
            health.total_value_sol
        );
        println!(
            "│ Total Invested:  {:<15.4} SOL                              │",
            health.total_invested_sol
        );
        println!(
            "│ Unrealized P&L:  {} {:<10.4} SOL ({:>6.2}%)                     │",
            pnl_color,
            health.total_unrealized_pnl,
            health.total_pnl_percentage
        );
        println!("│                                                                 │");
        println!(
            "│ Positions:       {:<3} total ({} profitable, {} losing)          │",
            health.positions_count,
            health.profitable_positions,
            health.losing_positions
        );
        println!(
            "│ Concentration:   {:<15} (largest: {:.1}%)                  │",
            health.portfolio_concentration_risk,
            health.largest_position_percentage
        );
        println!(
            "│ Health Score:    {} {:<3}/100                                    │",
            health_color,
            health.health_score
        );
        println!("└─────────────────────────────────────────────────────────────────┘");
    }

    /// Display positions in a formatted table
    fn print_positions_table(&self, positions: &[Position]) {
        if self.compact_mode {
            self.print_compact_positions(positions);
            return;
        }

        // Sort positions by current value (largest first)
        let mut sorted_positions = positions.to_vec();
        sorted_positions.sort_by(|a, b|
            b.current_value_sol.partial_cmp(&a.current_value_sol).unwrap()
        );

        // Table header
        println!(
            "┌──────────────┬─────────────┬─────────────┬─────────────┬─────────────┬──────────────┐"
        );
        println!(
            "│ Token        │ Amount      │ Value (SOL) │ Avg Price   │ Current     │ P&L (%)      │"
        );
        println!(
            "├──────────────┼─────────────┼─────────────┼─────────────┼─────────────┼──────────────┤"
        );

        for position in &sorted_positions {
            let pnl_indicator = if position.unrealized_pnl >= 0.0 { "🟢" } else { "🔴" };
            let amount_str = self.format_token_amount(position.total_amount);

            println!(
                "│ {:<12} │ {:>11} │ {:>11.4} │ {:>11.6} │ {:>11.6} │ {} {:>7.2}% │",
                self.truncate_string(&position.symbol, 12),
                amount_str,
                position.current_value_sol,
                position.average_entry_price,
                position.current_price,
                pnl_indicator,
                position.unrealized_pnl_percentage
            );
        }

        println!(
            "└──────────────┴─────────────┴─────────────┴─────────────┴─────────────┴──────────────┘"
        );
    }

    /// Display positions in compact format
    fn print_compact_positions(&self, positions: &[Position]) {
        for (i, position) in positions.iter().enumerate() {
            let pnl_indicator = if position.unrealized_pnl >= 0.0 { "📈" } else { "📉" };

            println!(
                "{:2}. {} {} - {:.4} SOL ({:+.1}%)",
                i + 1,
                pnl_indicator,
                position.symbol,
                position.current_value_sol,
                position.unrealized_pnl_percentage
            );
        }
    }

    /// Display detailed position analysis
    pub fn display_position_details(&self, position: &Position, analysis: &PositionAnalysis) {
        self.print_header(&format!("📈 {} POSITION DETAILS", position.symbol));

        let status_emoji = match analysis.status.as_str() {
            "Strong Winner" => "🚀",
            "Winner" => "✅",
            "Underperforming" => "⚠️",
            "Significant Loss" => "❌",
            _ => "📊",
        };

        println!("┌─ Position Overview ─────────────────────────────────────────────┐");
        println!("│ Token:           {:<47} │", position.symbol);
        println!("│ Status:          {} {:<43} │", status_emoji, analysis.status);
        println!("│ Amount:          {:<47} │", self.format_token_amount(position.total_amount));
        println!(
            "│ Current Value:   {:<15.4} SOL                        │",
            position.current_value_sol
        );
        println!(
            "│ Total Invested:  {:<15.4} SOL                        │",
            position.total_invested_sol
        );
        println!("│                                                                 │");
        println!("│ Average Entry:   ${:<44.6} │", position.average_entry_price);
        println!("│ Current Price:   ${:<44.6} │", position.current_price);
        println!(
            "│ Unrealized P&L:  {:<15.4} SOL ({:+.2}%)                │",
            position.unrealized_pnl,
            position.unrealized_pnl_percentage
        );
        println!("│                                                                 │");
        println!("│ Days Held:       {:<47.1} │", analysis.days_held);
        println!("│ Trade Count:     {:<47} │", position.trade_count);
        println!("│ Risk Level:      {:<47} │", analysis.risk_level);
        if analysis.annualized_return != 0.0 {
            println!("│ Annualized ROI:  {:<44.1}% │", analysis.annualized_return);
        }
        println!("└─────────────────────────────────────────────────────────────────┘");

        // Trading timeline
        println!("\n┌─ Trading Timeline ──────────────────────────────────────────────┐");
        println!("│ First Buy:       {:<47} │", self.format_datetime(&position.first_buy_time));
        println!("│ Last Buy:        {:<47} │", self.format_datetime(&position.last_buy_time));
        println!("│ DCA Opportunities: {:<45} │", position.dca_opportunities);
        println!("└─────────────────────────────────────────────────────────────────┘");

        // Action recommendations
        let mut actions = Vec::new();
        if analysis.should_dca {
            actions.push("🔄 Consider DCA (position down significantly)");
        }
        if analysis.should_take_profit {
            actions.push("💰 Consider taking profits (strong gains)");
        }

        if !actions.is_empty() {
            println!("\n💡 Recommendations:");
            for action in actions {
                println!("   {}", action);
            }
        }
    }

    /// Display rebalance recommendations
    pub fn display_rebalance_recommendations(&self, recommendations: &[RebalanceRecommendation]) {
        if recommendations.is_empty() {
            println!("✅ Portfolio is well balanced - no immediate actions needed");
            return;
        }

        self.print_header("⚖️ REBALANCE RECOMMENDATIONS");

        let high_priority: Vec<_> = recommendations
            .iter()
            .filter(|r| r.priority == "High")
            .collect();
        let medium_priority: Vec<_> = recommendations
            .iter()
            .filter(|r| r.priority == "Medium")
            .collect();

        if !high_priority.is_empty() {
            println!("🚨 HIGH PRIORITY:");
            for rec in high_priority {
                self.print_recommendation(rec);
            }
            println!();
        }

        if !medium_priority.is_empty() {
            println!("⚠️  MEDIUM PRIORITY:");
            for rec in medium_priority {
                self.print_recommendation(rec);
            }
        }
    }

    /// Print individual rebalance recommendation
    fn print_recommendation(&self, rec: &RebalanceRecommendation) {
        let action_emoji = match rec.action {
            RebalanceAction::DCA => "🔄",
            RebalanceAction::TakeProfit => "💰",
            RebalanceAction::Reduce => "📉",
            RebalanceAction::Close => "❌",
            RebalanceAction::Increase => "📈",
        };

        println!(
            "   {} {} ({}): {}",
            action_emoji,
            rec.symbol,
            format!("{:?}", rec.action).to_uppercase(),
            rec.reason
        );

        match rec.action {
            RebalanceAction::DCA | RebalanceAction::Increase => {
                println!("      💡 Suggested amount: {:.4} SOL", rec.amount_sol);
            }
            RebalanceAction::TakeProfit | RebalanceAction::Reduce => {
                println!("      💡 Suggested reduction: {:.4} SOL", rec.amount_sol);
            }
            RebalanceAction::Close => {
                println!("      💡 Close entire position: {:.4} SOL", rec.amount_sol);
            }
        }
    }

    /// Display portfolio diversification analysis
    pub fn display_diversification_analysis(&self, analysis: &DiversificationAnalysis) {
        self.print_header("🎯 DIVERSIFICATION ANALYSIS");

        let score_emoji = match analysis.diversification_score {
            80..=100 => "🟢",
            60..=79 => "🟡",
            40..=59 => "🟠",
            _ => "🔴",
        };

        println!("┌─ Diversification Metrics ──────────────────────────────────────┐");
        println!("│ Unique Positions:    {:<43} │", analysis.unique_positions);
        println!(
            "│ Diversification:     {} {:<3}/100                              │",
            score_emoji,
            analysis.diversification_score
        );
        println!("│ Concentration Risk:  {:<43} │", analysis.concentration_risk);
        println!("│ HHI Index:           {:<43.3} │", analysis.herfindahl_index);
        println!("└─────────────────────────────────────────────────────────────────┘");

        // Interpretation
        match analysis.concentration_risk.as_str() {
            "High" => println!("\n⚠️  High concentration detected - consider diversifying"),
            "Medium" => println!("\n📊 Moderate concentration - monitor position sizes"),
            "Low" => println!("\n✅ Well diversified portfolio"),
            _ => {}
        }
    }

    /// Display performance summary for specific time period
    pub fn display_performance_summary(&self, positions: &[Position], days: u32) {
        self.print_header(&format!("📈 PERFORMANCE SUMMARY (Last {} days)", days));

        // Filter positions that had activity in the time period
        let cutoff_time = Utc::now() - chrono::Duration::days(days as i64);
        let recent_positions: Vec<_> = positions
            .iter()
            .filter(|p| p.last_buy_time > cutoff_time)
            .collect();

        if recent_positions.is_empty() {
            println!("No trading activity in the specified period");
            return;
        }

        let total_recent_invested: f64 = recent_positions
            .iter()
            .map(|p| p.total_invested_sol)
            .sum();
        let total_recent_value: f64 = recent_positions
            .iter()
            .map(|p| p.current_value_sol)
            .sum();
        let recent_pnl = total_recent_value - total_recent_invested;
        let recent_pnl_percentage = if total_recent_invested > 0.0 {
            (recent_pnl / total_recent_invested) * 100.0
        } else {
            0.0
        };

        let winners = recent_positions
            .iter()
            .filter(|p| p.unrealized_pnl > 0.0)
            .count();
        let losers = recent_positions
            .iter()
            .filter(|p| p.unrealized_pnl < 0.0)
            .count();

        println!("Recent Activity:");
        println!(
            "• Positions: {} ({} winners, {} losers)",
            recent_positions.len(),
            winners,
            losers
        );
        println!("• Total Invested: {:.4} SOL", total_recent_invested);
        println!("• Current Value: {:.4} SOL", total_recent_value);
        println!("• P&L: {:.4} SOL ({:+.2}%)", recent_pnl, recent_pnl_percentage);

        if !recent_positions.is_empty() {
            println!("\nTop Performers:");
            let mut sorted = recent_positions.clone();
            sorted.sort_by(|a, b|
                b.unrealized_pnl_percentage.partial_cmp(&a.unrealized_pnl_percentage).unwrap()
            );

            for (i, position) in sorted.iter().take(3).enumerate() {
                let emoji = if position.unrealized_pnl >= 0.0 { "🟢" } else { "🔴" };
                println!(
                    "  {}. {} {} ({:+.1}%)",
                    i + 1,
                    emoji,
                    position.symbol,
                    position.unrealized_pnl_percentage
                );
            }
        }
    }

    // Helper methods

    fn print_header(&self, title: &str) {
        let line = "═".repeat(80);
        println!("{}", line);
        println!("{:^80}", title);
        println!("{}", line);
    }

    fn format_token_amount(&self, amount: u64) -> String {
        if amount >= 1_000_000_000 {
            format!("{:.1}B", (amount as f64) / 1_000_000_000.0)
        } else if amount >= 1_000_000 {
            format!("{:.1}M", (amount as f64) / 1_000_000.0)
        } else if amount >= 1_000 {
            format!("{:.1}K", (amount as f64) / 1_000.0)
        } else {
            amount.to_string()
        }
    }

    fn truncate_string(&self, s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }

    fn format_datetime(&self, dt: &DateTime<Utc>) -> String {
        dt.format("%Y-%m-%d %H:%M UTC").to_string()
    }
}

/// Quick display functions for common use cases
impl PortfolioDisplay {
    /// Quick portfolio summary (one-liner)
    pub fn quick_summary(&self, health: &PortfolioHealth) {
        let pnl_emoji = if health.total_unrealized_pnl >= 0.0 { "📈" } else { "📉" };
        println!(
            "{} Portfolio: {:.4} SOL ({:+.2}%) | {} positions | Health: {}/100",
            pnl_emoji,
            health.total_value_sol,
            health.total_pnl_percentage,
            health.positions_count,
            health.health_score
        );
    }

    /// Quick position list (symbols and P&L only)
    pub fn quick_positions(&self, positions: &[Position]) {
        for position in positions {
            let emoji = if position.unrealized_pnl >= 0.0 { "🟢" } else { "🔴" };
            println!(
                "{} {} {:.4} SOL ({:+.1}%)",
                emoji,
                position.symbol,
                position.current_value_sol,
                position.unrealized_pnl_percentage
            );
        }
    }

    /// Alert for urgent actions needed
    pub fn display_alerts(&self, recommendations: &[RebalanceRecommendation]) {
        let urgent: Vec<_> = recommendations
            .iter()
            .filter(|r| r.priority == "High")
            .collect();

        if !urgent.is_empty() {
            println!("🚨 URGENT ACTIONS NEEDED:");
            for rec in urgent {
                println!(
                    "   {} {}: {}",
                    match rec.action {
                        RebalanceAction::Close => "❌",
                        RebalanceAction::DCA => "🔄",
                        RebalanceAction::TakeProfit => "💰",
                        _ => "⚠️",
                    },
                    rec.symbol,
                    rec.reason
                );
            }
            println!();
        }
    }
}
