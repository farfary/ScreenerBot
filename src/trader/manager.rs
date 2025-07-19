use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;
use colored::Colorize;
use tabled::{ Table, Tabled, settings::Style };

use crate::config::TraderConfig;
use crate::trader::database::TraderDatabase;
use crate::trader::position::Position;
use crate::trader::strategy::TradingStrategy;
use crate::trader::types::*;
use crate::types::TokenInfo;
use crate::marketdata::{ TokenData, MarketData };
use crate::swap::SwapManager;
use crate::discovery::Discovery;
use crate::pairs::PairsClient;
use crate::rug_detection::{ RugDetectionEngine, RugAction };

pub struct TraderManager {
    config: TraderConfig,
    database: Arc<TraderDatabase>,
    strategy: Arc<RwLock<TradingStrategy>>,
    swap_manager: Arc<SwapManager>,
    market_data: Arc<MarketData>,
    discovery: Arc<Discovery>,
    pairs_client: Arc<PairsClient>,
    rug_detection: Arc<RugDetectionEngine>,
    positions: Arc<RwLock<HashMap<String, Position>>>,
    running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<TraderStats>>,
}

impl TraderManager {
    pub fn new(
        config: TraderConfig,
        swap_manager: Arc<SwapManager>,
        market_data: Arc<MarketData>,
        discovery: Arc<Discovery>
    ) -> Result<Self> {
        let database = Arc::new(TraderDatabase::new(&config.database_path)?);
        let strategy = Arc::new(RwLock::new(TradingStrategy::new(config.clone())));
        let pairs_client = Arc::new(PairsClient::new()?); // PairsClient::new() already returns Result
        let positions = Arc::new(RwLock::new(HashMap::new()));
        let running = Arc::new(RwLock::new(false));
        let stats = Arc::new(
            RwLock::new(TraderStats {
                total_trades: 0,
                successful_trades: 0,
                failed_trades: 0,
                total_invested_sol: 0.0,
                total_realized_pnl_sol: 0.0,
                total_unrealized_pnl_sol: 0.0,
                win_rate: 0.0,
                average_trade_size_sol: 0.0,
                largest_win_sol: 0.0,
                largest_loss_sol: 0.0,
                active_positions: 0,
                closed_positions: 0,
            })
        );

        // Create rug detection engine with shared market database
        let rug_detection = Arc::new(
            RugDetectionEngine::new(market_data.get_database(), config.rug_detection.clone())
        );

        Ok(Self {
            config,
            database,
            strategy,
            swap_manager,
            market_data,
            discovery,
            pairs_client,
            positions,
            running,
            stats,
            rug_detection,
        })
    }

    /// Validate token safety before trading - checks blacklist and rug indicators
    async fn validate_token_safety(
        &self,
        token_address: &str,
        trade_amount_sol: f64
    ) -> Result<bool> {
        log::debug!(
            "Validating token safety for {} (trade: {} SOL)",
            token_address,
            trade_amount_sol
        );

        // Get current liquidity for analysis (try multiple sources)
        let current_liquidity = self.get_current_liquidity(token_address).await.unwrap_or(0.0);

        // Run comprehensive rug detection analysis
        match self.rug_detection.analyze_token(token_address, current_liquidity).await {
            Ok(result) => {
                match result.recommended_action {
                    RugAction::Blacklist | RugAction::SellImmediately => {
                        log::warn!(
                            "Token {} blocked for trading: {:?} (confidence: {:.1}%)",
                            token_address,
                            result.reasons,
                            result.confidence * 100.0
                        );
                        return Ok(false);
                    }
                    RugAction::Monitor => {
                        log::warn!(
                            "Token {} trading warning: {:?} (confidence: {:.1}%)",
                            token_address,
                            result.reasons,
                            result.confidence * 100.0
                        );
                        // For warnings, allow small trades but block large ones
                        if trade_amount_sol > 0.5 {
                            log::warn!(
                                "Large trade blocked due to monitoring status for {}",
                                token_address
                            );
                            return Ok(false);
                        }
                    }
                    RugAction::Continue => {
                        log::debug!("Token {} passed rug detection checks", token_address);
                    }
                }
            }
            Err(e) => {
                log::warn!(
                    "Rug detection failed for {}: {}. Proceeding with caution.",
                    token_address,
                    e
                );
                // On detection failure, be conservative - block large trades
                if trade_amount_sol > 0.1 {
                    log::warn!(
                        "Large trade blocked due to rug detection failure for {}",
                        token_address
                    );
                    return Ok(false);
                }
            }
        }

        // Additional safety check - validate pool quality
        match self.validate_trade_with_pool_quality(token_address, trade_amount_sol).await {
            Ok(false) => {
                log::warn!("Token {} failed pool quality validation", token_address);
                return Ok(false);
            }
            Err(e) => {
                log::warn!("Pool quality validation failed for {}: {}", token_address, e);
                // Be conservative on validation errors
                return Ok(false);
            }
            Ok(true) => {
                log::debug!("Token {} passed pool quality validation", token_address);
            }
        }

        log::info!("Token {} validated as safe for trading", token_address);
        Ok(true)
    }

    /// Get current liquidity for a token from various sources
    async fn get_current_liquidity(&self, token_address: &str) -> Result<f64> {
        // Try to get liquidity from pairs client first
        if let Ok(pairs) = self.pairs_client.get_solana_token_pairs(token_address).await {
            if let Some(best_pair) = self.pairs_client.get_best_pair(pairs) {
                if let Some(liquidity) = best_pair.liquidity {
                    return Ok(liquidity.usd);
                }
            }
        }

        // Fallback to market data if available
        if let Ok(Some(token_data)) = self.market_data.get_token_data(token_address).await {
            return Ok(token_data.liquidity_quote);
        }

        Ok(0.0)
    }

    /// Get the best price for a token using smart pool selection
    async fn get_best_token_price(&self, token_address: &str) -> Result<Option<f64>> {
        // Try to get price from pairs client (DEX pools) first for accuracy
        match self.pairs_client.get_best_price(token_address).await {
            Ok(Some(price)) => {
                log::debug!("Got price from DEX pools for {}: {:.10} SOL", token_address, price);
                return Ok(Some(price));
            }
            Ok(None) => {
                log::debug!("No price from DEX pools for {}", token_address);
            }
            Err(e) => {
                log::warn!("Failed to get price from DEX pools for {}: {}", token_address, e);
            }
        }

        // Fallback to market data API
        match self.market_data.get_token_data(token_address).await {
            Ok(Some(token_data)) => {
                let price = token_data.price_native;
                log::debug!(
                    "Got fallback price from market data for {}: {:.10} SOL",
                    token_address,
                    price
                );
                Ok(Some(price))
            }
            Ok(None) => {
                log::debug!("No price from market data for {}", token_address);
                Ok(None)
            }
            Err(e) => {
                log::warn!("Failed to get price from market data for {}: {}", token_address, e);
                Ok(None)
            }
        }
    }

    /// Update position prices using best available price sources
    async fn update_position_prices(&self) -> Result<()> {
        let mut positions = self.positions.write().await;

        for (token_address, position) in positions.iter_mut() {
            if let Ok(Some(current_price)) = self.get_best_token_price(token_address).await {
                position.update_price(current_price);
                log::debug!("Updated price for {} to ${}", token_address, current_price);
            }
        }

        Ok(())
    }

    /// Validate trade using pool quality metrics
    async fn validate_trade_with_pool_quality(
        &self,
        token_address: &str,
        trade_amount_sol: f64
    ) -> Result<bool> {
        // Get pairs for the token
        let pairs = self.pairs_client.get_solana_token_pairs(token_address).await?;

        if pairs.is_empty() {
            log::warn!("No pairs found for token {}", token_address);
            return Ok(false);
        }

        // Find the best pair for this trade
        if let Some(best_pair) = self.pairs_client.get_best_pair(pairs) {
            let quality_score = self.pairs_client.calculate_pool_quality_score(&best_pair);

            // Require higher quality for larger trades
            let min_quality_score = if trade_amount_sol > 1.0 { 70.0 } else { 50.0 };

            if quality_score < min_quality_score {
                log::warn!(
                    "Pool quality too low for {} (score: {:.1}, required: {:.1})",
                    token_address,
                    quality_score,
                    min_quality_score
                );
                return Ok(false);
            }

            // Check if liquidity is sufficient for the trade
            let trade_value_usd = trade_amount_sol * 180.0; // Approximate SOL price
            let min_liquidity = trade_value_usd * 10.0; // Require 10x liquidity vs trade size

            let liquidity_usd = best_pair.liquidity.as_ref().map_or(0.0, |l| l.usd);
            if liquidity_usd < min_liquidity {
                log::warn!(
                    "Insufficient liquidity for {} trade: ${:.2} < ${:.2}",
                    token_address,
                    liquidity_usd,
                    min_liquidity
                );
                return Ok(false);
            }

            log::info!(
                "Trade validation passed for {} - Quality: {:.1}, Liquidity: ${:.2}",
                token_address,
                quality_score,
                liquidity_usd
            );
            return Ok(true);
        }

        Ok(false)
    }

    pub async fn start(&self) -> Result<()> {
        {
            let mut running = self.running.write().await;
            if *running {
                return Ok(());
            }
            *running = true;
        }

        println!("🎯 Trader module starting...");

        // Load existing positions from database
        self.load_existing_positions().await?;

        // Update stats
        self.update_stats().await?;

        // Start background tasks
        self.start_price_monitoring().await;
        self.start_position_monitoring().await;
        self.start_discovery_monitoring().await;
        self.start_position_price_updates().await;

        // Start status display task
        {
            let positions = Arc::clone(&self.positions);
            let stats = Arc::clone(&self.stats);
            let running = Arc::clone(&self.running);
            let market_data = Arc::clone(&self.market_data);
            let database = Arc::clone(&self.database);

            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(15));

                loop {
                    interval.tick().await;

                    if !*running.read().await {
                        break;
                    }

                    // Update position prices with latest market data before display
                    {
                        let position_addresses: Vec<String> = {
                            let positions_read = positions.read().await;
                            positions_read.keys().cloned().collect()
                        };

                        for token_address in position_addresses {
                            if
                                let Ok(Some(token_data)) = market_data.get_token_data(
                                    &token_address
                                ).await
                            {
                                let current_price = token_data.price_native;

                                if current_price > 0.0 {
                                    let mut positions_write = positions.write().await;
                                    if let Some(position) = positions_write.get_mut(&token_address) {
                                        position.update_price(current_price);
                                    }
                                }
                            }
                        }
                    }

                    // Display status using table format
                    let positions_read = positions.read().await;
                    let stats_read = stats.read().await;

                    // Clear screen and display status
                    print!("\x1B[2J\x1B[1;1H");

                    let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
                    println!(
                        "🎯 {} {}",
                        "ScreenerBot Trading Dashboard".bold().bright_cyan(),
                        format!("({})", timestamp).dimmed()
                    );
                    println!();

                    // Calculate additional metrics
                    let total_pnl =
                        stats_read.total_realized_pnl_sol + stats_read.total_unrealized_pnl_sol;
                    let roi_percentage = if stats_read.total_invested_sol > 0.0 {
                        (total_pnl / stats_read.total_invested_sol) * 100.0
                    } else {
                        0.0
                    };

                    let avg_win = if stats_read.largest_win_sol > 0.0 {
                        stats_read.largest_win_sol
                    } else {
                        0.0
                    };
                    let avg_loss = if stats_read.largest_loss_sol < 0.0 {
                        stats_read.largest_loss_sol.abs()
                    } else {
                        0.0
                    };
                    let profit_factor = if avg_loss > 0.0 { avg_win / avg_loss } else { 0.0 };

                    let execution_success_rate = if stats_read.total_trades > 0 {
                        ((stats_read.successful_trades as f64) / (stats_read.total_trades as f64)) *
                            100.0
                    } else {
                        0.0
                    };

                    // Create comprehensive stats table
                    let stats_data = vec![
                        StatsRow {
                            metric: "Total Trades".to_string(),
                            value: format!("{}", stats_read.total_trades),
                        },
                        StatsRow {
                            metric: "Win Rate (P&L)".to_string(),
                            value: if stats_read.win_rate >= 50.0 {
                                format!("{:.1}%", stats_read.win_rate)
                            } else if stats_read.win_rate >= 30.0 {
                                format!("{:.1}%", stats_read.win_rate)
                            } else {
                                format!("{:.1}%", stats_read.win_rate)
                            },
                        },
                        StatsRow {
                            metric: "Execution Rate".to_string(),
                            value: format!("{:.1}%", execution_success_rate),
                        },
                        StatsRow {
                            metric: "Total Invested".to_string(),
                            value: format!("{:.5}", stats_read.total_invested_sol),
                        },
                        StatsRow {
                            metric: "Realized P&L".to_string(),
                            value: format!("{:.5}", stats_read.total_realized_pnl_sol),
                        },
                        StatsRow {
                            metric: "Unrealized P&L".to_string(),
                            value: format!("{:.5}", stats_read.total_unrealized_pnl_sol),
                        },
                        StatsRow {
                            metric: "Total P&L".to_string(),
                            value: format!("{:.5}", total_pnl),
                        },
                        StatsRow {
                            metric: "ROI".to_string(),
                            value: format!("{:.1}%", roi_percentage),
                        },
                        StatsRow {
                            metric: "Largest Win".to_string(),
                            value: format!("{:.5}", stats_read.largest_win_sol),
                        },
                        StatsRow {
                            metric: "Largest Loss".to_string(),
                            value: format!("{:.5}", stats_read.largest_loss_sol),
                        },
                        StatsRow {
                            metric: "Profit Factor".to_string(),
                            value: format!("{:.2}x", profit_factor),
                        },
                        StatsRow {
                            metric: "Active Positions".to_string(),
                            value: format!("{}", positions_read.len()),
                        },
                        StatsRow {
                            metric: "Closed Positions".to_string(),
                            value: format!("{}", stats_read.closed_positions),
                        },
                        StatsRow {
                            metric: "Avg Trade Size".to_string(),
                            value: format!("{:.5}", stats_read.average_trade_size_sol),
                        }
                    ];

                    let mut stats_table = Table::new(stats_data);
                    let styled_stats_table = stats_table.with(Style::modern());
                    println!("📊 {}", "Trading Performance Analytics".bold().bright_yellow());
                    println!("{}", styled_stats_table);
                    println!();

                    // Add performance summary
                    if stats_read.closed_positions > 0 {
                        let winning_positions = ((stats_read.win_rate / 100.0) *
                            (stats_read.closed_positions as f64)) as u32;
                        let losing_positions = stats_read.closed_positions - winning_positions;

                        println!("🏆 {}", "Performance Summary".bold().bright_green());
                        println!(
                            "   └─ {} winning trades • {} losing trades • {} active",
                            winning_positions,
                            losing_positions,
                            positions_read.len()
                        );
                        if roi_percentage >= 10.0 {
                            println!("   └─ 🚀 Strong performance with {:.1}% ROI", roi_percentage);
                        } else if roi_percentage >= 0.0 {
                            println!(
                                "   └─ 📈 Positive performance with {:.1}% ROI",
                                roi_percentage
                            );
                        } else {
                            println!("   └─ 📉 Needs improvement: {:.1}% ROI", roi_percentage);
                        }
                        println!();
                    }

                    if positions_read.is_empty() {
                        println!("📝 {}", "No active positions".italic().dimmed());
                    } else {
                        // Create positions table
                        let mut position_data: Vec<PositionRow> = positions_read
                            .values()
                            .map(|pos| PositionRow {
                                address: pos.token_address.clone(), // Full address instead of truncated
                                invested: format!("{:.5}", pos.total_invested_sol),
                                tokens: format!("{:.2}", pos.total_tokens),
                                current_price: format!("{:.8}", pos.current_price),
                                peak_price: if pos.peak_price > 0.0 {
                                    format!("{:.8}", pos.peak_price)
                                } else {
                                    "-".to_string()
                                },
                                low_price: if pos.lowest_price > 0.0 {
                                    format!("{:.8}", pos.lowest_price)
                                } else {
                                    "-".to_string()
                                },
                                pnl_sol: format!("{:.5}", pos.unrealized_pnl_sol),
                                pnl_percent: format!("{:.2}%", pos.unrealized_pnl_percent),
                                opens: "1".to_string(), // Always 1 since simplified
                                closes: "0".to_string(), // 0 for active positions
                                dca_count: pos.dca_count.to_string(),
                                status: format!("{:?}", pos.status),
                                age: {
                                    let duration = Utc::now().signed_duration_since(pos.created_at);
                                    if duration.num_days() > 0 {
                                        format!("{}d", duration.num_days())
                                    } else if duration.num_hours() > 0 {
                                        format!("{}h", duration.num_hours())
                                    } else {
                                        format!("{}m", duration.num_minutes())
                                    }
                                },
                            })
                            .collect();

                        // Sort by unrealized P&L descending
                        position_data.sort_by(|a, b| {
                            let a_pnl: f64 = a.pnl_sol.parse().unwrap_or(0.0);
                            let b_pnl: f64 = b.pnl_sol.parse().unwrap_or(0.0);
                            b_pnl.partial_cmp(&a_pnl).unwrap_or(std::cmp::Ordering::Equal)
                        });

                        let mut positions_table = Table::new(position_data);
                        let styled_positions_table = positions_table.with(Style::modern());
                        println!("🔥 {}", "Active Positions".bold().bright_green());
                        println!("{}", styled_positions_table);

                        // Show last 10 closed positions
                        match database.get_closed_positions(10) {
                            Ok(closed_positions) if !closed_positions.is_empty() => {
                                println!();
                                println!("📋 {}", "Recent Closed Positions".bold().bright_blue());

                                let closed_data: Vec<PositionRow> = closed_positions
                                    .iter()
                                    .map(|(_, pos)| PositionRow {
                                        address: pos.token_address.clone(),
                                        invested: format!("{:.5}", pos.total_invested_sol),
                                        tokens: format!("{:.2}", pos.total_tokens),
                                        current_price: format!("{:.8}", pos.current_price),
                                        peak_price: if pos.peak_price > 0.0 {
                                            format!("{:.8}", pos.peak_price)
                                        } else {
                                            "-".to_string()
                                        },
                                        low_price: if pos.lowest_price > 0.0 {
                                            format!("{:.8}", pos.lowest_price)
                                        } else {
                                            "-".to_string()
                                        },
                                        pnl_sol: format!("{:.5}", pos.realized_pnl_sol),
                                        pnl_percent: if pos.total_invested_sol > 0.0 {
                                            format!(
                                                "{:.2}%",
                                                (pos.realized_pnl_sol / pos.total_invested_sol) *
                                                    100.0
                                            )
                                        } else {
                                            "0.00%".to_string()
                                        },
                                        opens: "1".to_string(), // Always 1 since simplified
                                        closes: "1".to_string(), // 1 for closed positions
                                        dca_count: pos.dca_count.to_string(),
                                        status: format!("{:?}", pos.status),
                                        age: {
                                            let duration = Utc::now().signed_duration_since(
                                                pos.updated_at
                                            );
                                            if duration.num_days() > 0 {
                                                format!("{}d", duration.num_days())
                                            } else if duration.num_hours() > 0 {
                                                format!("{}h", duration.num_hours())
                                            } else {
                                                format!("{}m", duration.num_minutes())
                                            }
                                        },
                                    })
                                    .collect();

                                let mut closed_table = Table::new(closed_data);
                                let styled_closed_table = closed_table.with(Style::modern());
                                println!("{}", styled_closed_table);
                            }
                            _ => {}
                        }
                    }

                    println!();
                    println!("{}", "═".repeat(80).dimmed());
                }
            });
        }

        println!("🎯 Trader module started successfully");
        Ok(())
    }

    pub async fn stop(&self) {
        {
            let mut running = self.running.write().await;
            *running = false;
        }
        println!("🎯 Trader module stopped");
    }

    async fn load_existing_positions(&self) -> Result<()> {
        let active_positions = self.database.get_active_positions()?;
        let mut positions = self.positions.write().await;

        for (id, summary) in active_positions {
            let position = Position::from_summary(id, summary);
            positions.insert(position.token_address.clone(), position);
        }

        println!("📊 Loaded {} active positions", positions.len());
        Ok(())
    }

    async fn start_price_monitoring(&self) {
        let strategy = Arc::clone(&self.strategy);
        let positions = Arc::clone(&self.positions);
        let running = Arc::clone(&self.running);
        let market_data = Arc::clone(&self.market_data);
        let pairs_client = Arc::clone(&self.pairs_client);

        tokio::spawn(async move {
            // Check price every 5 seconds as requested
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            let mut last_top_tokens_update = std::time::Instant::now();
            let mut top_tokens: Vec<TokenData> = Vec::new();

            println!("🎯 Starting price monitoring for top 20 tokens (5 sec intervals)");

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                // Update top 20 tokens every 60 seconds to avoid overloading
                if
                    last_top_tokens_update.elapsed() > Duration::from_secs(20) ||
                    top_tokens.is_empty()
                {
                    match market_data.get_top_tokens_by_volume(120).await {
                        Ok(tokens) => {
                            top_tokens = tokens;
                            println!("📊 Updated top 20 tokens list ({} tokens)", top_tokens.len());
                            last_top_tokens_update = std::time::Instant::now();
                        }
                        Err(e) => {
                            println!("❌ Error getting top tokens: {}", e);
                            continue;
                        }
                    }
                }

                // Monitor prices for top tokens using smart price discovery
                for token in &top_tokens {
                    // Get previous price from strategy for comparison
                    let previous_price = strategy.read().await.get_current_price(&token.mint);

                    // Use smart price discovery (DEX pools first, then fallback to market data)
                    let market_price = match pairs_client.get_best_price(&token.mint).await {
                        Ok(Some(price)) => {
                            log::debug!("Got DEX pool price for {}: ${}", token.mint, price);
                            price
                        }
                        Ok(None) | Err(_) => {
                            // Fallback to market data price
                            log::debug!(
                                "Using market data price for {}: {:.10} SOL",
                                token.mint,
                                token.price_native
                            );
                            token.price_native
                        }
                    };

                    // Update strategy with new price
                    strategy.write().await.update_price(&token.mint, market_price);

                    // Calculate price change if we have a previous price
                    if let Some(prev_price) = previous_price {
                        if (market_price - prev_price).abs() > prev_price * 0.001 {
                            // Show changes > 0.1% with enhanced display
                            let change_percent = ((market_price - prev_price) / prev_price) * 100.0;
                            let change_indicator = if change_percent > 0.0 { "📈" } else { "📉" };

                            // Get token age from pool data
                            let token_age_str = match
                                market_data.get_database().get_token_pools(&token.mint)
                            {
                                Ok(pools) => {
                                    let token_age = pools
                                        .iter()
                                        .map(|p| p.created_at)
                                        .min();
                                    let now = chrono::Utc::now();
                                    if let Some(created_at) = token_age {
                                        let duration = now.signed_duration_since(created_at);
                                        let days = duration.num_days();
                                        let hours = duration.num_hours() % 24;
                                        let mins = duration.num_minutes() % 60;
                                        format!("{}d {}h {}m", days, hours, mins)
                                    } else {
                                        "unknown".to_string()
                                    }
                                }
                                Err(_) => "unknown".to_string(),
                            };
                        }
                    } else {
                        // First time getting price for this token - show basic info
                        println!(
                            "🎯 {} ({}): Initial Price={:.10} SOL | Volume={:.2}K",
                            token.symbol,
                            &token.mint[..8],
                            market_price,
                            token.volume_24h / 1000.0
                        );
                    }
                }

                // Print monitoring summary every 30 seconds
                if last_top_tokens_update.elapsed() > Duration::from_secs(30) {
                    println!(
                        "📊 Monitoring Summary: {} tokens using market data prices",
                        top_tokens.len()
                    );

                    // Display enhanced position summary every 30 seconds
                    let positions_read = positions.read().await;
                    if !positions_read.is_empty() {
                        println!("\n{}", "📊 ACTIVE POSITIONS".bright_cyan().bold());
                        for (mint, position) in positions_read.iter() {
                            println!(
                                "   🪙 {} | Invested: {:.10} SOL | Tokens: {:.4} | PnL: {:.10} SOL",
                                position.token_symbol,
                                position.total_invested_sol,
                                position.total_tokens,
                                position.unrealized_pnl_sol
                            );
                        }
                    } else {
                        println!("\n📊 No active positions");
                    }

                    last_top_tokens_update = std::time::Instant::now();
                }

                // Update prices for active positions using market data
                let positions_clone = {
                    let positions_read = positions.read().await;
                    positions_read.clone()
                };

                for (token_address, _position) in &positions_clone {
                    // Use market data for position price updates - prioritize native price
                    if let Ok(Some(token_data)) = market_data.get_token_data(token_address).await {
                        let current_price = token_data.price_native;

                        // Only update if we have a valid price
                        if current_price > 0.0 {
                            strategy.write().await.update_price(token_address, current_price);

                            let mut positions_write = positions.write().await;
                            if let Some(pos) = positions_write.get_mut(token_address) {
                                pos.update_price(current_price);
                                log::debug!(
                                    "Updated position price for {} from market data: {:.10} SOL (native)",
                                    token_address,
                                    current_price
                                );
                            }
                        } else {
                            log::warn!(
                                "Invalid price for {} from market data: {}",
                                token_address,
                                current_price
                            );
                        }
                    } else {
                        log::warn!("No market data found for position: {}", token_address);
                    }
                }

                if !positions_clone.is_empty() {
                    println!("📊 Updated prices for {} positions", positions_clone.len());
                }
            }
        });
    }

    async fn start_position_monitoring(&self) {
        let strategy = Arc::clone(&self.strategy);
        let positions = Arc::clone(&self.positions);
        let running = Arc::clone(&self.running);
        let database = Arc::clone(&self.database);
        let trader_manager = self.clone_for_async();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_secs(config.position_check_interval_seconds)
            );

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                let positions_clone = {
                    let positions_read = positions.read().await;
                    positions_read.clone()
                };

                for (_token_address, position) in positions_clone {
                    let current_price = position.current_price;

                    // Analyze position for signals
                    let signals = strategy.read().await.analyze_position(&position, current_price);

                    for signal in signals {
                        // Record signal in database
                        if let Err(e) = database.record_signal(&signal) {
                            eprintln!("❌ Failed to record signal: {}", e);
                        }

                        // Execute trade based on signal
                        if let Err(e) = trader_manager.execute_trade_signal(&signal).await {
                            eprintln!("❌ Failed to execute trade: {}", e);
                        }
                    }
                }
            }
        });
    }

    async fn start_discovery_monitoring(&self) {
        let strategy = Arc::clone(&self.strategy);
        let discovery = Arc::clone(&self.discovery);
        let market_data = Arc::clone(&self.market_data);
        let running = Arc::clone(&self.running);
        let trader_manager = self.clone_for_async();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                Duration::from_secs(config.price_check_interval_seconds)
            );

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                // Get recent discovered tokens from database
                let discovery_db = discovery.get_database();
                if let Ok(recent_tokens) = discovery_db.get_recent_tokens(1) {
                    for discovered_token in recent_tokens {
                        // Get token data from market data
                        if
                            let Ok(Some(token_data)) = market_data.get_token_data(
                                &discovered_token.mint
                            ).await
                        {
                            let current_price = token_data.price_native;

                            if current_price > 0.0 {
                                // Convert TokenData to TokenInfo
                                let token_info = Self::token_data_to_token_info(&token_data);

                                // Update strategy with new price
                                strategy
                                    .write().await
                                    .update_price(&discovered_token.mint, current_price);

                                // Check for buy signals
                                if
                                    let Some(signal) = strategy
                                        .read().await
                                        .analyze_token(&token_info, current_price)
                                {
                                    println!(
                                        "📡 New buy signal: {} at {:.10} SOL",
                                        token_info.symbol,
                                        current_price
                                    );

                                    // Execute buy trade
                                    if
                                        let Err(e) = trader_manager.execute_trade_signal(
                                            &signal
                                        ).await
                                    {
                                        eprintln!("❌ Failed to execute buy trade: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    async fn start_position_price_updates(&self) {
        let positions = Arc::clone(&self.positions);
        let running = Arc::clone(&self.running);
        let market_data = Arc::clone(&self.market_data);

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30)); // Update every 30 seconds

            loop {
                interval.tick().await;

                if !*running.read().await {
                    break;
                }

                let position_addresses: Vec<String> = {
                    let positions_read = positions.read().await;
                    positions_read.keys().cloned().collect()
                };

                // Update prices for all active positions using market data native price
                for token_address in position_addresses {
                    if let Ok(Some(token_data)) = market_data.get_token_data(&token_address).await {
                        let current_price = token_data.price_native;

                        if current_price > 0.0 {
                            let mut positions_write = positions.write().await;
                            if let Some(position) = positions_write.get_mut(&token_address) {
                                position.update_price(current_price);
                                log::debug!(
                                    "Updated position price for {} to {:.10} SOL (native price)",
                                    token_address,
                                    current_price
                                );
                            }
                        }
                    }
                }
            }
        });
    }

    pub async fn execute_trade_signal(&self, signal: &TradeSignal) -> Result<()> {
        let position = {
            let positions = self.positions.read().await;
            positions.get(&signal.token_address).cloned()
        };

        match signal.signal_type {
            TradeSignalType::Buy => {
                self.execute_buy_trade(signal, position).await?;
            }
            TradeSignalType::Sell => {
                if let Some(pos) = position {
                    self.execute_sell_trade(signal, pos).await?;
                }
            }
            TradeSignalType::DCA => {
                if let Some(pos) = position {
                    self.execute_dca_trade(signal, pos).await?;
                }
            }
            TradeSignalType::StopLoss => {
                if let Some(pos) = position {
                    self.execute_stop_loss_trade(signal, pos).await?;
                }
            }
        }

        Ok(())
    }

    async fn execute_buy_trade(
        &self,
        signal: &TradeSignal,
        existing_position: Option<Position>
    ) -> Result<()> {
        // Check if position exists in database if not in memory
        let existing_position = if existing_position.is_some() {
            existing_position
        } else {
            Position::load_from_database(&self.database, &signal.token_address)?
        };

        let trade_size = self.strategy
            .read().await
            .calculate_trade_size(signal, existing_position.as_ref());

        // First validate token safety (rug detection + blacklist)
        if !self.validate_token_safety(&signal.token_address, trade_size).await? {
            println!(
                "⚠️  Skipping BUY for {} - failed rug detection/safety validation",
                signal.token_address
            );
            return Ok(());
        }

        // Validate trade using pool quality metrics
        if !self.validate_trade_with_pool_quality(&signal.token_address, trade_size).await? {
            println!("⚠️  Skipping BUY for {} - pool quality insufficient", signal.token_address);
            return Ok(());
        }

        // DO NOT fetch updated price here!
        // let current_price = self
        //     .get_best_token_price(&signal.token_address).await?
        //     .unwrap_or(signal.current_price);

        let current_price = signal.current_price;

        // Check if price hasn't moved too much since signal generation
        let min_price_threshold = 1e-10;
        if signal.current_price > min_price_threshold && current_price > min_price_threshold {
            let price_deviation = (
                (current_price - signal.current_price) /
                signal.current_price
            ).abs();
            if price_deviation > 0.05 {
                println!(
                    "⚠️  Skipping BUY for {} - price moved too much: {:.2}% (signal: {:.10} SOL → current: {:.10} SOL)",
                    signal.token_address,
                    price_deviation * 100.0,
                    signal.current_price,
                    current_price
                );
                return Ok(());
            }
        } else {
            log::warn!(
                "Skipping price deviation check for {} due to very small prices (signal: {:.10} SOL, current: {:.10} SOL)",
                signal.token_address,
                signal.current_price,
                current_price
            );
        }

        println!(
            "🟢 Executing BUY: {} - {:.10} SOL (${:.4} SOL) - Pool validated",
            signal.token_address,
            current_price,
            trade_size
        );

        // Use signal.current_price for trade execution
        let updated_signal = TradeSignal {
            current_price,
            ..signal.clone()
        };

        let trade_result = if self.config.dry_run {
            self.simulate_buy_trade(&updated_signal, trade_size).await
        } else {
            self.execute_real_buy_trade(&updated_signal, trade_size).await
        };

        match trade_result {
            Ok(result) => {
                println!("✅ Buy trade executed successfully");
                self.process_buy_trade_result(&updated_signal, result, existing_position).await?;
            }
            Err(e) => {
                eprintln!("❌ Buy trade failed: {}", e);
                let failed_result = TradeResult {
                    success: false,
                    transaction_hash: None,
                    amount_sol: trade_size,
                    amount_tokens: 0.0,
                    price_per_token: current_price,
                    fees: 0.0,
                    slippage: 0.0,
                    timestamp: Utc::now(),
                    error: Some(e.to_string()),
                };
                self.process_buy_trade_result(signal, failed_result, existing_position).await?;
            }
        }

        Ok(())
    }

    async fn execute_sell_trade(&self, signal: &TradeSignal, position: Position) -> Result<()> {
        let trade_size = position.total_tokens;

        // Check for rug indicators (but don't block sell trades - we want to exit bad positions)
        let current_liquidity = self
            .get_current_liquidity(&signal.token_address).await
            .unwrap_or(0.0);
        match self.rug_detection.analyze_token(&signal.token_address, current_liquidity).await {
            Ok(result) => {
                match result.recommended_action {
                    RugAction::Blacklist | RugAction::SellImmediately => {
                        println!(
                            "🚨 URGENT SELL for rugged token {}: {:?}",
                            signal.token_address,
                            result.reasons
                        );
                    }
                    RugAction::Monitor => {
                        println!(
                            "⚠️  Selling potentially risky token {}: {:?}",
                            signal.token_address,
                            result.reasons
                        );
                    }
                    RugAction::Continue => {
                        log::debug!("Selling safe token {}", signal.token_address);
                    }
                }
            }
            Err(e) => {
                log::warn!("Rug detection failed for sell of {}: {}", signal.token_address, e);
            }
        }

        // Get updated price using smart price discovery
        let current_price = self
            .get_best_token_price(&signal.token_address).await?
            .unwrap_or(signal.current_price);

        println!(
            "🔴 Executing SELL: {} - {:.10} SOL ({:.4} tokens) - Using best pool price",
            signal.token_address,
            current_price,
            trade_size
        );

        // Create updated signal with best price
        let updated_signal = TradeSignal {
            current_price,
            ..signal.clone()
        };

        let trade_result = if self.config.dry_run {
            self.simulate_sell_trade(&updated_signal, trade_size).await
        } else {
            self.execute_real_sell_trade(&updated_signal, trade_size).await
        };

        match trade_result {
            Ok(result) => {
                println!("✅ Sell trade executed successfully");
                self.process_sell_trade_result(&updated_signal, result, position).await?;
            }
            Err(e) => {
                eprintln!("❌ Sell trade failed: {}", e);
                let failed_result = TradeResult {
                    success: false,
                    transaction_hash: None,
                    amount_sol: trade_size * current_price,
                    amount_tokens: trade_size,
                    price_per_token: current_price,
                    fees: 0.0,
                    slippage: 0.0,
                    timestamp: Utc::now(),
                    error: Some(e.to_string()),
                };
                self.process_sell_trade_result(&updated_signal, failed_result, position).await?;
            }
        }

        Ok(())
    }

    async fn execute_dca_trade(&self, signal: &TradeSignal, position: Position) -> Result<()> {
        let trade_size = self.strategy.read().await.calculate_trade_size(signal, Some(&position));

        println!(
            "🟡 Executing DCA: {} - {:.10} SOL (${:.4} SOL, DCA #{}/{})",
            signal.token_address,
            signal.current_price,
            trade_size,
            position.dca_count + 1,
            self.config.dca_levels
        );

        let trade_result = if self.config.dry_run {
            self.simulate_buy_trade(signal, trade_size).await
        } else {
            self.execute_real_buy_trade(signal, trade_size).await
        };

        match trade_result {
            Ok(result) => {
                println!("✅ DCA trade executed successfully");
                self.process_dca_trade_result(signal, result, position).await?;
            }
            Err(e) => {
                eprintln!("❌ DCA trade failed: {}", e);
            }
        }

        Ok(())
    }

    async fn execute_stop_loss_trade(
        &self,
        signal: &TradeSignal,
        position: Position
    ) -> Result<()> {
        let trade_size = position.total_tokens;

        println!(
            "🛑 Executing STOP LOSS: {} - ${:.6} ({:.4} tokens)",
            signal.token_address,
            signal.current_price,
            trade_size
        );

        let trade_result = if self.config.dry_run {
            self.simulate_sell_trade(signal, trade_size).await
        } else {
            self.execute_real_sell_trade(signal, trade_size).await
        };

        match trade_result {
            Ok(result) => {
                println!("✅ Stop loss executed successfully");
                self.process_stop_loss_trade_result(signal, result, position).await?;
            }
            Err(e) => {
                eprintln!("❌ Stop loss failed: {}", e);
            }
        }

        Ok(())
    }

    async fn simulate_buy_trade(
        &self,
        signal: &TradeSignal,
        amount_sol: f64
    ) -> Result<TradeResult> {
        // Simulate slippage and fees
        let slippage = 0.01; // 1% slippage
        let fees = amount_sol * 0.0025; // 0.25% fees
        let effective_amount = amount_sol - fees;
        let effective_price = signal.current_price * (1.0 + slippage);
        let tokens_received = effective_amount / effective_price;

        Ok(TradeResult {
            success: true,
            transaction_hash: Some(format!("SIMULATED_BUY_{}", Utc::now().timestamp())),
            amount_sol,
            amount_tokens: tokens_received,
            price_per_token: effective_price,
            fees,
            slippage,
            timestamp: Utc::now(),
            error: None,
        })
    }

    async fn simulate_sell_trade(
        &self,
        signal: &TradeSignal,
        amount_tokens: f64
    ) -> Result<TradeResult> {
        // Simulate slippage and fees
        let slippage = 0.01; // 1% slippage
        let effective_price = signal.current_price * (1.0 - slippage);
        let sol_received = amount_tokens * effective_price;
        let fees = sol_received * 0.0025; // 0.25% fees
        let net_sol = sol_received - fees;

        Ok(TradeResult {
            success: true,
            transaction_hash: Some(format!("SIMULATED_SELL_{}", Utc::now().timestamp())),
            amount_sol: net_sol,
            amount_tokens,
            price_per_token: effective_price,
            fees,
            slippage,
            timestamp: Utc::now(),
            error: None,
        })
    }

    async fn execute_real_buy_trade(
        &self,
        _signal: &TradeSignal,
        _amount_sol: f64
    ) -> Result<TradeResult> {
        // Use swap manager to execute real trade
        // This is a placeholder - implement actual swap logic
        Err(anyhow::anyhow!("Real trading not implemented yet"))
    }

    async fn execute_real_sell_trade(
        &self,
        _signal: &TradeSignal,
        _amount_tokens: f64
    ) -> Result<TradeResult> {
        // Use swap manager to execute real trade
        // This is a placeholder - implement actual swap logic
        Err(anyhow::anyhow!("Real trading not implemented yet"))
    }

    async fn process_buy_trade_result(
        &self,
        signal: &TradeSignal,
        result: TradeResult,
        existing_position: Option<Position>
    ) -> Result<()> {
        let mut position = existing_position.unwrap_or_else(|| {
            Position::new(signal.token_address.clone(), "Unknown".to_string())
        });

        if result.success {
            position.add_buy_trade(result.amount_sol, result.amount_tokens, result.price_per_token);
            // Update current price to signal price
            position.update_price(signal.current_price);
        }

        // Save position to database (this will update existing or create new)
        let position_id = position.save_to_database(&self.database)?;
        position.id = Some(position_id);

        // Record trade in database
        self.database.record_trade(position_id, &signal.token_address, "BUY", &result)?;

        // Update positions map (ensure synchronization)
        {
            let mut positions = self.positions.write().await;
            positions.insert(signal.token_address.clone(), position);
        }

        // Update stats
        self.update_stats().await?;

        // Update stats
        self.update_stats().await?;

        Ok(())
    }

    async fn process_sell_trade_result(
        &self,
        signal: &TradeSignal,
        result: TradeResult,
        mut position: Position
    ) -> Result<()> {
        if result.success {
            position.add_sell_trade(
                result.amount_sol,
                result.amount_tokens,
                result.price_per_token
            );
            position.status = PositionStatus::Closed;
        }

        // Save position to database
        let position_id = position.id.unwrap();
        self.database.update_position(position_id, &position.to_summary())?;

        // Record trade in database
        self.database.record_trade(position_id, &signal.token_address, "SELL", &result)?;

        // Remove from active positions if closed
        if matches!(position.status, PositionStatus::Closed) {
            let mut positions = self.positions.write().await;
            positions.remove(&signal.token_address);
        }

        // Update stats
        self.update_stats().await?;

        Ok(())
    }

    async fn process_dca_trade_result(
        &self,
        signal: &TradeSignal,
        result: TradeResult,
        mut position: Position
    ) -> Result<()> {
        if result.success {
            position.add_dca_trade(result.amount_sol, result.amount_tokens, result.price_per_token);
            // Update current price to signal price
            position.update_price(signal.current_price);
        }

        // Save position to database
        let position_id = position.id.unwrap();
        self.database.update_position(position_id, &position.to_summary())?;

        // Record trade in database
        self.database.record_trade(position_id, &signal.token_address, "DCA", &result)?;

        // Update positions map
        {
            let mut positions = self.positions.write().await;
            positions.insert(signal.token_address.clone(), position);
        }

        // Update stats
        self.update_stats().await?;

        Ok(())
    }

    async fn process_stop_loss_trade_result(
        &self,
        signal: &TradeSignal,
        result: TradeResult,
        mut position: Position
    ) -> Result<()> {
        if result.success {
            position.add_sell_trade(
                result.amount_sol,
                result.amount_tokens,
                result.price_per_token
            );
            position.status = PositionStatus::StopLoss;
        }

        // Save position to database
        let position_id = position.id.unwrap();
        self.database.update_position(position_id, &position.to_summary())?;

        // Record trade in database
        self.database.record_trade(position_id, &signal.token_address, "STOP_LOSS", &result)?;

        // Remove from active positions
        {
            let mut positions = self.positions.write().await;
            positions.remove(&signal.token_address);
        }

        // Update stats
        self.update_stats().await?;

        Ok(())
    }

    async fn update_stats(&self) -> Result<()> {
        let db_stats = self.database.get_trader_stats()?;
        let mut stats = self.stats.write().await;
        *stats = db_stats;
        Ok(())
    }

    pub async fn get_stats(&self) -> TraderStats {
        self.stats.read().await.clone()
    }

    pub async fn get_positions(&self) -> Vec<PositionSummary> {
        let positions = self.positions.read().await;
        positions
            .values()
            .map(|p| p.to_summary())
            .collect()
    }

    pub async fn get_position(&self, token_address: &str) -> Option<PositionSummary> {
        let positions = self.positions.read().await;
        positions.get(token_address).map(|p| p.to_summary())
    }

    // Helper function to convert TokenData to TokenInfo
    fn token_data_to_token_info(token_data: &TokenData) -> TokenInfo {
        TokenInfo {
            mint: token_data.mint.clone(),
            symbol: token_data.symbol.clone(),
            name: token_data.name.clone(),
            decimals: token_data.decimals,
            supply: 0, // We don't have supply in new TokenData, would need to fetch separately
            market_cap: token_data.market_cap,
            price: Some(token_data.price_native),
            volume_24h: Some(token_data.volume_24h),
            liquidity: Some(token_data.liquidity_quote),
            pool_address: token_data.top_pool_address.clone(),
            discovered_at: token_data.last_updated,
            last_updated: token_data.last_updated,
            is_active: true,
        }
    }

    fn clone_for_async(&self) -> TraderManagerAsync {
        TraderManagerAsync {
            config: self.config.clone(),
            database: Arc::clone(&self.database),
            strategy: Arc::clone(&self.strategy),
            swap_manager: Arc::clone(&self.swap_manager),
            market_data: Arc::clone(&self.market_data),
            discovery: Arc::clone(&self.discovery),
            pairs_client: Arc::clone(&self.pairs_client),
            positions: Arc::clone(&self.positions),
            running: Arc::clone(&self.running),
            stats: Arc::clone(&self.stats),
        }
    }

    /// Display comprehensive token information with market data
    pub async fn display_token_info(
        &self,
        token_data: &TokenData,
        show_price_change: bool
    ) -> Result<()> {
        use colored::*;

        let symbol = &token_data.symbol;
        let name = &token_data.name;

        // Header
        println!("\n{}", "━".repeat(80).bright_black());
        println!(
            "{} {} ({})",
            "🪙 TOKEN INFO:".bright_cyan().bold(),
            name.bright_white().bold(),
            symbol.bright_yellow().bold()
        );
        println!("{}", "━".repeat(80).bright_black());

        // Basic info
        println!("   📍 Mint: {}", token_data.mint.bright_blue());
        println!(
            "   💰 Price: {:.10} SOL (${:.10}) [source: {}]",
            token_data.price_native.to_string().bright_green().bold(),
            token_data.price_usd,
            token_data.source.bright_cyan()
        );

        if let Some(market_cap) = token_data.market_cap {
            println!("   🏦 Market Cap: ${:.2}", market_cap);
        }

        if let Some(fdv) = token_data.fdv {
            println!("   💎 FDV: ${:.2}", fdv);
        }

        println!(
            "   💧 Liquidity: ${:.2} USD ({:.2} SOL)",
            token_data.liquidity_usd,
            token_data.liquidity_quote
        );

        println!("   🔄 Volume 24h: ${:.2}", token_data.volume_24h);
        println!(
            "   🏪 DEX: {} | Pool: {}",
            token_data.dex_id.bright_magenta(),
            token_data.top_pool_address.as_ref().unwrap_or(&"N/A".to_string()).bright_blue()
        );

        // Price change information
        if show_price_change {
            self.display_price_change_info(token_data).await?;
        }

        println!("{}", "━".repeat(80).bright_black());
        Ok(())
    }

    /// Display price change information with detailed analytics
    async fn display_price_change_info(&self, token_data: &TokenData) -> Result<()> {
        use colored::*;

        // Get historical data for price change calculation
        if let Ok(Some(historical)) = self.market_data.get_database().get_token(&token_data.mint) {
            let old_price = historical.price_native;
            if old_price > 0.0 {
                let price_change = ((token_data.price_native - old_price) / old_price) * 100.0;

                let (change_color, change_emoji) = if price_change > 0.0 {
                    ("green", "📈")
                } else if price_change < 0.0 {
                    ("red", "📉")
                } else {
                    ("yellow", "➡️")
                };

                println!(
                    "   {} Price Change: {}{:.2}%",
                    change_emoji,
                    if price_change > 0.0 {
                        "+"
                    } else {
                        ""
                    },
                    price_change.to_string().color(change_color).bold()
                );

                println!(
                    "   📊 Previous: {:.10} SOL → Current: {:.10} SOL",
                    old_price.to_string().dimmed(),
                    token_data.price_native.to_string().bright_white().bold()
                );
            }
        }

        Ok(())
    }

    /// Display current position summary
    pub async fn display_position_summary(&self) -> Result<()> {
        use colored::*;

        // First update all position prices with latest market data
        {
            let position_addresses: Vec<String> = {
                let positions_read = self.positions.read().await;
                positions_read.keys().cloned().collect()
            };

            for token_address in position_addresses {
                if let Ok(Some(token_data)) = self.market_data.get_token_data(&token_address).await {
                    let current_price = token_data.price_native;

                    if current_price > 0.0 {
                        let mut positions_write = self.positions.write().await;
                        if let Some(position) = positions_write.get_mut(&token_address) {
                            position.update_price(current_price);
                        }
                    }
                }
            }
        }

        let positions = self.positions.read().await;

        if positions.is_empty() {
            println!("\n{} No active positions", "📊 POSITIONS:".bright_cyan().bold());
            return Ok(());
        }

        println!("\n{}", "━".repeat(80).bright_black());
        println!("{}", "📊 ACTIVE POSITIONS".bright_cyan().bold());
        println!("{}", "━".repeat(80).bright_black());

        let mut total_value_sol = 0.0;
        let mut total_pnl_sol = 0.0;

        for (mint, position) in positions.iter() {
            // Get current market data
            if let Ok(Some(token_data)) = self.market_data.get_database().get_token(mint) {
                let current_value = position.total_tokens * token_data.price_native;
                let pnl = position.unrealized_pnl_sol;
                let pnl_percent = position.unrealized_pnl_percent;

                total_value_sol += current_value;
                total_pnl_sol += pnl;

                let symbol = &position.token_symbol;

                let pnl_color = if pnl > 0.0 {
                    "green"
                } else if pnl < 0.0 {
                    "red"
                } else {
                    "yellow"
                };

                println!("   🪙 {} ({})", token_data.name.bright_white(), symbol.bright_yellow());
                println!(
                    "      💰 Tokens: {:.4} | Value: {:.4} SOL",
                    position.total_tokens,
                    current_value
                );
                println!(
                    "      📈 P&L: {}{:.4} SOL ({}{:.2}%)",
                    if pnl > 0.0 {
                        "+"
                    } else {
                        ""
                    },
                    pnl.to_string().color(pnl_color).bold(),
                    if pnl_percent > 0.0 {
                        "+"
                    } else {
                        ""
                    },
                    pnl_percent.to_string().color(pnl_color).bold()
                );
                println!(
                    "      🕒 Created: {} | Current: {:.8} SOL",
                    position.created_at.format("%H:%M:%S").to_string().dimmed(),
                    token_data.price_native.to_string().bright_green()
                );
                println!();
            }
        }

        // Summary
        let total_pnl_percent = if total_value_sol > 0.0 {
            (total_pnl_sol / (total_value_sol - total_pnl_sol)) * 100.0
        } else {
            0.0
        };

        let summary_color = if total_pnl_sol > 0.0 {
            "green"
        } else if total_pnl_sol < 0.0 {
            "red"
        } else {
            "yellow"
        };

        println!("{}", "─".repeat(80).bright_black());
        println!(
            "   💼 Portfolio Value: {:.4} SOL",
            total_value_sol.to_string().bright_white().bold()
        );
        println!(
            "   📊 Total P&L: {}{:.4} SOL ({}{:.2}%)",
            if total_pnl_sol > 0.0 {
                "+"
            } else {
                ""
            },
            total_pnl_sol.to_string().color(summary_color).bold(),
            if total_pnl_percent > 0.0 {
                "+"
            } else {
                ""
            },
            total_pnl_percent.to_string().color(summary_color).bold()
        );
        println!("{}", "━".repeat(80).bright_black());

        Ok(())
    }

    /// Display trading statistics
    pub async fn display_trading_stats(&self) -> Result<()> {
        use colored::*;

        let stats = self.stats.read().await;
        let success_rate = if stats.total_trades > 0 {
            ((stats.successful_trades as f64) / (stats.total_trades as f64)) * 100.0
        } else {
            0.0
        };

        println!("\n{}", "📈 TRADING STATISTICS".bright_cyan().bold());
        println!("   🎯 Total Trades: {}", stats.total_trades.to_string().bright_white().bold());
        println!(
            "   ✅ Successful: {} | ❌ Failed: {}",
            stats.successful_trades.to_string().bright_green(),
            stats.failed_trades.to_string().bright_red()
        );
        println!("   📊 Success Rate: {:.1}%", success_rate.to_string().bright_yellow().bold());
        println!(
            "   💰 Total Realized P&L: {:.4} SOL",
            stats.total_realized_pnl_sol.to_string().bright_white().bold()
        );

        Ok(())
    }
}

// Helper function to format large numbers
fn format_large_number(num: f64) -> String {
    if num >= 1_000_000_000.0 {
        format!("{:.1}B", num / 1_000_000_000.0)
    } else if num >= 1_000_000.0 {
        format!("{:.1}M", num / 1_000_000.0)
    } else if num >= 1_000.0 {
        format!("{:.1}K", num / 1_000.0)
    } else {
        format!("{:.0}", num)
    }
}

// Helper struct for async operations
#[derive(Clone)]
struct TraderManagerAsync {
    config: TraderConfig,
    database: Arc<TraderDatabase>,
    strategy: Arc<RwLock<TradingStrategy>>,
    swap_manager: Arc<SwapManager>,
    market_data: Arc<MarketData>,
    discovery: Arc<Discovery>,
    pairs_client: Arc<PairsClient>,
    positions: Arc<RwLock<HashMap<String, Position>>>,
    running: Arc<RwLock<bool>>,
    stats: Arc<RwLock<TraderStats>>,
}

impl TraderManagerAsync {
    async fn execute_trade_signal(&self, signal: &TradeSignal) -> Result<()> {
        // Create a temporary TraderManager with the same data
        let temp_manager = TraderManager {
            config: self.config.clone(),
            database: Arc::clone(&self.database),
            strategy: Arc::clone(&self.strategy),
            swap_manager: Arc::clone(&self.swap_manager),
            market_data: Arc::clone(&self.market_data),
            discovery: Arc::clone(&self.discovery),
            pairs_client: Arc::clone(&self.pairs_client),
            positions: Arc::clone(&self.positions),
            running: Arc::clone(&self.running),
            stats: Arc::clone(&self.stats),
            rug_detection: Arc::new(
                RugDetectionEngine::new(
                    self.market_data.get_database(),
                    self.config.rug_detection.clone()
                )
            ),
        };

        temp_manager.execute_trade_signal(signal).await
    }
}

// Structs for table display
#[derive(Tabled)]
struct StatsRow {
    #[tabled(rename = "Metric")]
    metric: String,
    #[tabled(rename = "Value")]
    value: String,
}

#[derive(Tabled)]
struct PositionRow {
    #[tabled(rename = "Address")]
    address: String,
    #[tabled(rename = "Invested")]
    invested: String,
    #[tabled(rename = "Tokens")]
    tokens: String,
    #[tabled(rename = "Price")]
    current_price: String,
    #[tabled(rename = "Peak")]
    peak_price: String,
    #[tabled(rename = "Low")]
    low_price: String,
    #[tabled(rename = "P&L")]
    pnl_sol: String,
    #[tabled(rename = "P&L (%)")]
    pnl_percent: String,
    #[tabled(rename = "Opens")]
    opens: String,
    #[tabled(rename = "Closes")]
    closes: String,
    #[tabled(rename = "DCA")]
    dca_count: String,
    #[tabled(rename = "Status")]
    status: String,
    #[tabled(rename = "Age")]
    age: String,
}
