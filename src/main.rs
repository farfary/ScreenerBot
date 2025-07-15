use screenerbot::{ Config, Database, Discovery, WalletTracker, Trader, Logger };
use anyhow::Result;
use std::sync::Arc;
use tokio::signal;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    Logger::header("SOLANA DEX TRADER BOT");
    Logger::info("Starting ScreenerBot...");

    // Load configuration
    let config = match Config::load("configs.json") {
        Ok(config) => {
            Logger::success("Configuration loaded successfully");
            config
        }
        Err(e) => {
            Logger::error(&format!("Failed to load config: {}", e));
            Logger::info("Creating default configuration...");
            let config = Config::default();
            config.save("configs.json")?;
            Logger::success(
                "Default configuration created. Please edit configs.json with your settings."
            );
            return Ok(());
        }
    };

    // Initialize database
    let database = match Database::new(&config.database.path) {
        Ok(db) => {
            Logger::success("Database initialized successfully");
            Arc::new(db)
        }
        Err(e) => {
            Logger::error(&format!("Failed to initialize database: {}", e));
            return Err(e);
        }
    };

    // Initialize modules
    Logger::separator();
    Logger::info("Initializing modules...");

    // Discovery module
    let discovery = Arc::new(Discovery::new(config.discovery.clone(), Arc::clone(&database)));

    // Wallet tracker
    let wallet_tracker = match WalletTracker::new(config.clone(), Arc::clone(&database)) {
        Ok(tracker) => {
            Logger::success("Wallet tracker initialized");
            Arc::new(tracker)
        }
        Err(e) => {
            Logger::error(&format!("Failed to initialize wallet tracker: {}", e));
            return Err(e);
        }
    };

    // Trader (only if enabled)
    let trader = Arc::new(
        Trader::new(
            config.trader.clone(),
            Arc::clone(&database),
            Arc::clone(&discovery),
            Arc::clone(&wallet_tracker)
        )
    );

    Logger::separator();
    Logger::info("Starting modules...");

    // Start discovery module
    if let Err(e) = discovery.start().await {
        Logger::error(&format!("Failed to start discovery: {}", e));
        return Err(e);
    }

    // Start wallet tracker
    if let Err(e) = wallet_tracker.start().await {
        Logger::error(&format!("Failed to start wallet tracker: {}", e));
        return Err(e);
    }

    // Start trader (if enabled)
    if config.trader.enabled {
        if let Err(e) = trader.start().await {
            Logger::error(&format!("Failed to start trader: {}", e));
            return Err(e);
        }
    } else {
        Logger::warn("Trading module is disabled in configuration");
    }

    Logger::separator();
    Logger::success("All modules started successfully!");
    Logger::info("Press Ctrl+C to stop the bot");
    Logger::separator();

    // Start status display loop
    let status_discovery = Arc::clone(&discovery);
    let status_wallet = Arc::clone(&wallet_tracker);
    let status_trader = Arc::clone(&trader);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(5000)); // Update every 5 seconds

        loop {
            interval.tick().await;

            // Display status
            display_status(&status_discovery, &status_wallet, &status_trader).await;
        }
    });

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            Logger::info("Shutdown signal received...");
        }
        Err(err) => {
            Logger::error(&format!("Failed to listen for shutdown signal: {}", err));
        }
    }

    // Shutdown modules
    Logger::separator();
    Logger::info("Shutting down modules...");

    discovery.stop().await;
    wallet_tracker.stop().await;
    trader.stop().await;

    Logger::success("ScreenerBot shutdown complete");

    Ok(())
}

async fn display_status(
    discovery: &Arc<Discovery>,
    wallet_tracker: &Arc<WalletTracker>,
    trader: &Arc<Trader>
) {
    // Clear screen and move cursor to top
    print!("\x1B[2J\x1B[1;1H");

    Logger::header("SCREENER BOT STATUS");

    // Discovery status
    if discovery.is_running().await {
        let stats = discovery.get_stats().await;
        let cached_tokens = discovery.get_cached_tokens().await;

        Logger::discovery(
            &format!(
                "ACTIVE - {} tokens cached, {:.1} discoveries/hour",
                cached_tokens.len(),
                stats.discovery_rate_per_hour
            )
        );

        // Show some recent discoveries
        let mut recent_tokens: Vec<_> = cached_tokens.values().collect();
        recent_tokens.sort_by(|a, b| b.discovered_at.cmp(&a.discovered_at));

        for (i, token) in recent_tokens.iter().take(3).enumerate() {
            Logger::discovery(
                &format!(
                    "  {}. {} - ${:.4} | Vol: ${:.0} | Liq: ${:.0}",
                    i + 1,
                    token.symbol,
                    token.price.unwrap_or(0.0),
                    token.volume_24h.unwrap_or(0.0),
                    token.liquidity.unwrap_or(0.0)
                )
            );
        }
    } else {
        Logger::discovery("STOPPED");
    }

    Logger::separator();

    // Wallet status
    if wallet_tracker.is_running().await {
        let positions = wallet_tracker.get_positions().await;

        if let Ok(sol_balance) = wallet_tracker.get_sol_balance().await {
            Logger::wallet(&format!("SOL Balance: {:.4} SOL", sol_balance));
        }

        Logger::wallet(&format!("Active Positions: {}", positions.len()));

        // Show top positions by value
        let mut sorted_positions: Vec<_> = positions.values().collect();
        sorted_positions.sort_by(|a, b| {
            b.value_usd.unwrap_or(0.0).partial_cmp(&a.value_usd.unwrap_or(0.0)).unwrap()
        });

        let mut total_value = 0.0;
        for position in &sorted_positions {
            total_value += position.value_usd.unwrap_or(0.0);
        }

        Logger::wallet(&format!("Total Portfolio Value: ${:.2}", total_value));

        for (i, position) in sorted_positions.iter().take(3).enumerate() {
            let balance = (position.balance as f64) / (10_f64).powi(position.decimals as i32);
            let pnl_color = if position.pnl_percentage.unwrap_or(0.0) >= 0.0 { "🟢" } else { "🔴" };

            Logger::wallet(
                &format!(
                    "  {}. {} - {:.4} | ${:.2} | {} {:.1}%",
                    i + 1,
                    &position.mint[..8],
                    balance,
                    position.value_usd.unwrap_or(0.0),
                    pnl_color,
                    position.pnl_percentage.unwrap_or(0.0)
                )
            );
        }
    } else {
        Logger::wallet("STOPPED");
    }

    Logger::separator();

    // Trader status
    if trader.is_running().await {
        let signals = trader.get_active_signals().await;
        Logger::trader(&format!("ACTIVE - {} signals generated", signals.len()));

        // Show active signals
        for (mint, signal) in signals.iter().take(3) {
            let signal_emoji = match signal.signal_type {
                screenerbot::types::SignalType::Buy => "🟢 BUY",
                screenerbot::types::SignalType::Sell => "🔴 SELL",
                screenerbot::types::SignalType::Hold => "🟡 HOLD",
            };

            Logger::trader(
                &format!(
                    "  {} {} - Confidence: {:.0}% | {}",
                    signal_emoji,
                    &mint[..8],
                    signal.confidence * 100.0,
                    signal.reason
                )
            );
        }
    } else {
        Logger::trader("DISABLED (for safety)");
    }

    Logger::separator();
    Logger::info(&format!("Last updated: {}", chrono::Utc::now().format("%H:%M:%S UTC")));
    Logger::info("Press Ctrl+C to exit");
}
