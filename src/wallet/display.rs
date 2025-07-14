use crate::core::{ TokenBalance, WalletTransaction, Position, PortfolioHealth };
use crate::portfolio::PositionAnalysis;
use solana_sdk::pubkey::Pubkey;
use chrono::{ DateTime, Utc };

/// Wallet status display formatter for comprehensive wallet overview
#[derive(Debug)]
pub struct WalletStatusDisplay {
    show_colors: bool,
    compact_mode: bool,
    show_transaction_summary: bool,
}

impl WalletStatusDisplay {
    pub fn new() -> Self {
        Self {
            show_colors: true,
            compact_mode: false,
            show_transaction_summary: true,
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

    pub fn with_transaction_summary(mut self, enabled: bool) -> Self {
        self.show_transaction_summary = enabled;
        self
    }

    /// Display comprehensive wallet status after startup and warmup
    pub fn display_wallet_status(
        &self,
        wallet_address: &Pubkey,
        balances: &[TokenBalance],
        positions: &[Position],
        portfolio_health: &PortfolioHealth,
        recent_transactions: &[WalletTransaction]
    ) {
        self.print_startup_header();

        // Wallet overview
        self.print_wallet_overview(wallet_address, balances, portfolio_health);

        // Token holdings table
        if !balances.is_empty() {
            println!();
            self.print_header("💰 WALLET TOKENS & BALANCES");
            self.print_tokens_table(balances);
        }

        // Position analysis
        if !positions.is_empty() {
            println!();
            self.print_header("📊 POSITION ANALYSIS");
            self.print_positions_analysis(positions);
        }

        // Transaction summary
        if self.show_transaction_summary && !recent_transactions.is_empty() {
            println!();
            self.print_header("📈 RECENT ACTIVITY SUMMARY");
            self.print_transaction_summary(recent_transactions);
        }

        // Portfolio health and recommendations
        println!();
        self.print_header("🎯 PORTFOLIO HEALTH");
        self.print_portfolio_health_summary(portfolio_health);

        self.print_footer();
    }

    /// Print startup header with branding
    fn print_startup_header(&self) {
        println!("{}", "═".repeat(100));
        println!("🤖 SCREENERBOT - WALLET STATUS REPORT");
        println!("📅 Generated: {}", Utc::now().format("%Y-%m-%d %H:%M:%S UTC"));
        println!("{}", "═".repeat(100));
    }

    /// Print wallet overview section
    fn print_wallet_overview(
        &self,
        wallet_address: &Pubkey,
        balances: &[TokenBalance],
        portfolio_health: &PortfolioHealth
    ) {
        let sol_balance = balances
            .iter()
            .find(|b| b.symbol.as_ref().map_or(false, |s| s == "SOL"))
            .map(|b| b.ui_amount)
            .unwrap_or(0.0);

        let total_tokens = balances.len().saturating_sub(1); // Exclude SOL
        let non_zero_tokens = balances
            .iter()
            .filter(|b| { b.amount > 0 && !b.symbol.as_ref().map_or(false, |s| s == "SOL") })
            .count();

        println!(
            "┌─ Wallet Overview ───────────────────────────────────────────────────────────────────────┐"
        );
        println!(
            "│ Address:         {:<71} │",
            self.truncate_string(&wallet_address.to_string(), 71)
        );
        println!(
            "│ SOL Balance:     {:<15.6} SOL                                           │",
            sol_balance
        );
        println!(
            "│ Total Value:     {:<15.4} SOL                                           │",
            portfolio_health.total_value_sol
        );
        println!(
            "│ Token Holdings:  {} tokens ({} with balance)                                    │",
            self.format_number_with_padding(total_tokens, 3),
            self.format_number_with_padding(non_zero_tokens, 3)
        );
        println!(
            "│ Positions:       {} active positions                                               │",
            self.format_number_with_padding(portfolio_health.positions_count, 3)
        );

        let status_emoji = if sol_balance < 0.01 {
            "🔴"
        } else if sol_balance < 0.1 {
            "🟡"
        } else {
            "🟢"
        };
        let status_text = if sol_balance < 0.01 {
            "LOW SOL - LIMITED TRADING"
        } else if sol_balance < 0.1 {
            "MODERATE SOL BALANCE"
        } else {
            "SUFFICIENT SOL FOR TRADING"
        };

        println!("│ Status:          {} {:<62} │", status_emoji, status_text);
        println!(
            "└─────────────────────────────────────────────────────────────────────────────────────────┘"
        );
    }

    /// Print comprehensive tokens table
    fn print_tokens_table(&self, balances: &[TokenBalance]) {
        if self.compact_mode {
            self.print_compact_tokens(balances);
            return;
        }

        // Sort balances: SOL first, then by value (largest first)
        let mut sorted_balances = balances.to_vec();
        sorted_balances.sort_by(|a, b| {
            if a.symbol.as_ref().map_or(false, |s| s == "SOL") {
                std::cmp::Ordering::Less
            } else if b.symbol.as_ref().map_or(false, |s| s == "SOL") {
                std::cmp::Ordering::Greater
            } else {
                let a_value = a.value_usd.unwrap_or(0.0);
                let b_value = b.value_usd.unwrap_or(0.0);
                b_value.partial_cmp(&a_value).unwrap_or(std::cmp::Ordering::Equal)
            }
        });

        println!(
            "┌──────────────┬─────────────────┬─────────────────┬─────────────────┬─────────────────┬─────────────┐"
        );
        println!(
            "│ Token        │ Symbol          │ Amount          │ USD Value       │ SOL Value       │ Status      │"
        );
        println!(
            "├──────────────┼─────────────────┼─────────────────┼─────────────────┼─────────────────┼─────────────┤"
        );

        for balance in &sorted_balances {
            let symbol = balance.symbol.as_deref().unwrap_or("UNKNOWN");
            let name = balance.name.as_deref().unwrap_or("Unknown Token");
            let amount_str = self.format_token_amount(balance.amount, balance.decimals);
            let usd_value = balance.value_usd.map_or("N/A".to_string(), |v| format!("${:.2}", v));
            let sol_value = if let Some(usd_val) = balance.value_usd {
                format!("{:.4} SOL", usd_val / 100.0) // Rough SOL conversion
            } else {
                "N/A".to_string()
            };

            let status = if balance.amount == 0 {
                "🔘 Empty"
            } else if symbol == "SOL" {
                "💎 Native"
            } else if balance.value_usd.unwrap_or(0.0) > 100.0 {
                "🚀 Large"
            } else if balance.value_usd.unwrap_or(0.0) > 10.0 {
                "✅ Medium"
            } else {
                "🔍 Small"
            };

            println!(
                "│ {:<12} │ {:<15} │ {:<15} │ {:<15} │ {:<15} │ {:<11} │",
                self.truncate_string(name, 12),
                self.truncate_string(symbol, 15),
                self.truncate_string(&amount_str, 15),
                self.truncate_string(&usd_value, 15),
                self.truncate_string(&sol_value, 15),
                status
            );
        }

        println!(
            "└──────────────┴─────────────────┴─────────────────┴─────────────────┴─────────────────┴─────────────┘"
        );
    }

    /// Print compact tokens display
    fn print_compact_tokens(&self, balances: &[TokenBalance]) {
        for (i, balance) in balances.iter().enumerate() {
            let symbol = balance.symbol.as_deref().unwrap_or("UNKNOWN");
            let amount_str = self.format_token_amount(balance.amount, balance.decimals);
            let value_str = balance.value_usd.map_or("N/A".to_string(), |v| format!("${:.2}", v));

            let status_emoji = if balance.amount == 0 {
                "⚪"
            } else if symbol == "SOL" {
                "💎"
            } else {
                "🪙"
            };

            println!("{:2}. {} {} - {} ({})", i + 1, status_emoji, symbol, amount_str, value_str);
        }
    }

    /// Print positions analysis with P&L
    fn print_positions_analysis(&self, positions: &[Position]) {
        let mut sorted_positions = positions.to_vec();
        sorted_positions.sort_by(|a, b|
            b.current_value_sol.partial_cmp(&a.current_value_sol).unwrap()
        );

        println!(
            "┌──────────────┬─────────────┬─────────────┬─────────────┬─────────────┬─────────────┬──────────────┐"
        );
        println!(
            "│ Token        │ Holdings    │ Value (SOL) │ Invested    │ P&L (SOL)   │ P&L (%)     │ Performance  │"
        );
        println!(
            "├──────────────┼─────────────┼─────────────┼─────────────┼─────────────┼─────────────┼──────────────┤"
        );

        for position in &sorted_positions {
            let pnl_indicator = if position.unrealized_pnl >= 0.0 { "🟢" } else { "🔴" };
            let performance = if position.unrealized_pnl_percentage >= 50.0 {
                "🚀 Excellent"
            } else if position.unrealized_pnl_percentage >= 20.0 {
                "✅ Good"
            } else if position.unrealized_pnl_percentage >= 0.0 {
                "📈 Positive"
            } else if position.unrealized_pnl_percentage >= -20.0 {
                "📉 Negative"
            } else {
                "❌ Poor"
            };

            let amount_str = self.format_token_amount(position.total_amount, 6); // Assume 6 decimals default

            println!(
                "│ {:<12} │ {:>11} │ {:>11.4} │ {:>11.4} │ {} {:>8.4} │ {:>10.2}% │ {:<12} │",
                self.truncate_string(&position.symbol, 12),
                amount_str,
                position.current_value_sol,
                position.total_invested_sol,
                pnl_indicator,
                position.unrealized_pnl.abs(),
                position.unrealized_pnl_percentage,
                performance
            );
        }

        println!(
            "└──────────────┴─────────────┴─────────────┴─────────────┴─────────────┴─────────────┴──────────────┘"
        );
    }

    /// Print transaction summary
    fn print_transaction_summary(&self, transactions: &[WalletTransaction]) {
        let recent_txs = transactions.iter().take(10); // Show last 10 transactions

        // Count transaction types
        let mut buy_count = 0;
        let mut sell_count = 0;
        let mut swap_count = 0;
        let mut other_count = 0;

        for tx in transactions {
            match tx.transaction_type {
                crate::core::TransactionType::Buy => {
                    buy_count += 1;
                }
                crate::core::TransactionType::Sell => {
                    sell_count += 1;
                }
                crate::core::TransactionType::Swap => {
                    swap_count += 1;
                }
                _ => {
                    other_count += 1;
                }
            }
        }

        println!(
            "┌─ Recent Activity (Last 10 Transactions) ───────────────────────────────────────────────┐"
        );
        println!(
            "│ Summary: {} buys, {} sells, {} swaps, {} other                               │",
            buy_count,
            sell_count,
            swap_count,
            other_count
        );
        println!(
            "├─────────────────┬─────────────────┬─────────────────┬─────────────────────────────────┤"
        );
        println!(
            "│ Type            │ Time            │ Status          │ Details                         │"
        );
        println!(
            "├─────────────────┼─────────────────┼─────────────────┼─────────────────────────────────┤"
        );

        for tx in recent_txs {
            let tx_type = format!("{:?}", tx.transaction_type);
            let time_str = if let Some(timestamp) = tx.block_time {
                DateTime::<Utc>
                    ::from_timestamp(timestamp, 0)
                    .map(|dt| dt.format("%m-%d %H:%M").to_string())
                    .unwrap_or_else(|| "Unknown".to_string())
            } else {
                "Pending".to_string()
            };

            let status = format!("{:?}", tx.status);
            let status_emoji = match tx.status {
                crate::core::TransactionStatus::Success => "✅",
                crate::core::TransactionStatus::Failed => "❌",
                crate::core::TransactionStatus::Pending => "⏳",
            };

            let details = if tx.tokens_involved.len() > 0 {
                format!("{} tokens", tx.tokens_involved.len())
            } else {
                "SOL transaction".to_string()
            };

            println!(
                "│ {:<15} │ {:<15} │ {} {:<12} │ {:<31} │",
                self.truncate_string(&tx_type, 15),
                self.truncate_string(&time_str, 15),
                status_emoji,
                self.truncate_string(&status, 12),
                self.truncate_string(&details, 31)
            );
        }

        println!(
            "└─────────────────┴─────────────────┴─────────────────┴─────────────────────────────────┘"
        );
    }

    /// Print portfolio health summary
    fn print_portfolio_health_summary(&self, health: &PortfolioHealth) {
        let health_emoji = match health.health_score {
            90..=100 => "🟢 Excellent",
            75..=89 => "🟡 Good",
            50..=74 => "🟠 Fair",
            25..=49 => "🔴 Poor",
            _ => "💀 Critical",
        };

        let pnl_emoji = if health.total_unrealized_pnl >= 0.0 { "📈" } else { "📉" };

        println!(
            "┌─ Portfolio Health Score ───────────────────────────────────────────────────────────────┐"
        );
        println!(
            "│ Overall Health:  {} ({}/100)                                              │",
            health_emoji,
            health.health_score
        );
        println!(
            "│ Total P&L:       {} {:.4} SOL ({:+.2}%)                                     │",
            pnl_emoji,
            health.total_unrealized_pnl,
            health.total_pnl_percentage
        );
        println!(
            "│ Win Rate:        {}/{} positions profitable                                       │",
            health.profitable_positions,
            health.positions_count
        );
        println!(
            "│ Risk Level:      {} (largest position: {:.1}%)                              │",
            health.portfolio_concentration_risk,
            health.largest_position_percentage
        );

        if !health.recommendations.is_empty() {
            println!(
                "├─ Recommendations ──────────────────────────────────────────────────────────────────────┤"
            );
            for (i, rec) in health.recommendations.iter().take(3).enumerate() {
                println!("│ {}. {:<84} │", i + 1, self.truncate_string(rec, 84));
            }
        }

        println!(
            "└─────────────────────────────────────────────────────────────────────────────────────────┘"
        );
    }

    /// Print section header
    fn print_header(&self, title: &str) {
        println!("┌─ {} {}", title, "─".repeat(88 - title.len()));
        println!("│");
    }

    /// Print footer
    fn print_footer(&self) {
        println!("{}", "═".repeat(100));
        println!("🤖 ScreenerBot Wallet Analysis Complete - Ready for Trading Operations");
        println!("{}", "═".repeat(100));
    }

    /// Helper function to format token amounts
    fn format_token_amount(&self, amount: u64, decimals: u8) -> String {
        let divisor = (10_f64).powi(decimals as i32);
        let ui_amount = (amount as f64) / divisor;

        if ui_amount >= 1_000_000.0 {
            format!("{:.2}M", ui_amount / 1_000_000.0)
        } else if ui_amount >= 1_000.0 {
            format!("{:.2}K", ui_amount / 1_000.0)
        } else if ui_amount >= 1.0 {
            format!("{:.4}", ui_amount)
        } else {
            format!("{:.8}", ui_amount)
        }
    }

    /// Helper function to truncate strings
    fn truncate_string(&self, s: &str, max_len: usize) -> String {
        if s.len() <= max_len {
            s.to_string()
        } else {
            format!("{}...", &s[..max_len.saturating_sub(3)])
        }
    }

    /// Helper function to format numbers with padding
    fn format_number_with_padding(&self, num: usize, width: usize) -> String {
        format!("{:width$}", num, width = width)
    }

    /// Quick wallet status for use in regular cycles
    pub fn display_quick_status(
        &self,
        balances: &[TokenBalance],
        portfolio_health: &PortfolioHealth
    ) {
        let sol_balance = balances
            .iter()
            .find(|b| b.symbol.as_ref().map_or(false, |s| s == "SOL"))
            .map(|b| b.ui_amount)
            .unwrap_or(0.0);

        let health_emoji = match portfolio_health.health_score {
            80..=100 => "🟢",
            60..=79 => "🟡",
            _ => "🔴",
        };

        let pnl_emoji = if portfolio_health.total_unrealized_pnl >= 0.0 { "📈" } else { "📉" };

        println!(
            "💰 Wallet: {:.4} SOL | Portfolio: {:.4} SOL {} | P&L: {} {:.2}% | Health: {} {}/100",
            sol_balance,
            portfolio_health.total_value_sol,
            if portfolio_health.total_value_sol > sol_balance {
                "📈"
            } else {
                "📊"
            },
            pnl_emoji,
            portfolio_health.total_pnl_percentage,
            health_emoji,
            portfolio_health.health_score
        );
    }
}

/// Quick display functions for common use cases
impl WalletStatusDisplay {
    /// Display wallet status during system startup
    pub fn display_startup_status(wallet_address: &Pubkey, sol_balance: f64, token_count: usize) {
        println!("🔑 Wallet Initialized: {}", wallet_address);
        println!("💰 SOL Balance: {:.6} SOL", sol_balance);
        println!("🪙 Token Holdings: {} different tokens", token_count);

        if sol_balance < 0.01 {
            println!("⚠️  WARNING: Low SOL balance - limited trading capability");
        } else if sol_balance < 0.1 {
            println!("⚡ Moderate SOL balance - ready for small trades");
        } else {
            println!("✅ Sufficient SOL balance - ready for trading");
        }
    }

    /// Display quick balance summary
    pub fn display_balance_summary(balances: &[TokenBalance]) -> String {
        let total_tokens = balances.len();
        let non_zero_tokens = balances
            .iter()
            .filter(|b| b.amount > 0)
            .count();
        let total_value: f64 = balances
            .iter()
            .filter_map(|b| b.value_usd)
            .sum();

        format!(
            "💰 {} tokens ({} with balance) | Total: ${:.2}",
            total_tokens,
            non_zero_tokens,
            total_value
        )
    }
}
