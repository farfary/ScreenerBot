use anyhow::Result;
use screenerbot::{ Config, Database, Discovery, Logger };
use screenerbot::market_data::MarketDataManager;
use std::sync::Arc;
use tokio::signal;
use screenerbot::market_data;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    Logger::header("SOLANA DEX TRADER BOT");
    Logger::info("Starting ScreenerBot...");

    // Load configuration
    let config = match Config::load("configs.json") {
        Ok(config) => {
            Logger::success("Configuration loaded");
            config
        }
        Err(e) => {
            Logger::error(&format!("FAILED to load config: {}", e));
            Logger::info("Creating default configuration...");
            let config = Config::default();
            config.save("configs.json")?;
            Logger::success("Default configuration created. Edit configs.json with your settings.");
            return Ok(());
        }
    };

    // Initialize database
    let database = match Database::new(&config.database.path) {
        Ok(db) => {
            Logger::database("Database initialized");
            Arc::new(db)
        }
        Err(e) => {
            Logger::error(&format!("FAILED to initialize database: {}", e));
            return Err(e);
        }
    };

    // Initialize modules
    Logger::info("Initializing modules...");

    // Market data manager
    let pricing_config = market_data::PricingConfig {
        update_interval_secs: config.pricing
            .as_ref()
            .map(|p| p.update_interval_secs)
            .unwrap_or(60),
        top_tokens_count: config.pricing
            .as_ref()
            .map(|p| p.top_tokens_count)
            .unwrap_or(1000),
        cache_ttl_secs: 300,
        max_cache_size: 10000,
        enable_dynamic_pricing: config.pricing
            .as_ref()
            .map(|p| p.enable_dynamic_pricing)
            .unwrap_or(false),
    };

    let mut market_data_manager = MarketDataManager::new(
        Arc::clone(&database),
        Arc::new(Logger::new()),
        pricing_config
    );

    // Enable the sophisticated tiered pricing system
    if let Err(e) = market_data_manager.enable_tiered_pricing().await {
        Logger::warn(&format!("Failed to enable tiered pricing: {}", e));
        Logger::info("Continuing with basic pricing system...");
    }

    let market_data_manager = Arc::new(market_data_manager);
    Logger::pricing("Market data manager initialized");

    // Discovery module
    let discovery = Arc::new(Discovery::new(config.discovery.clone(), Arc::clone(&database)));
    Logger::discovery("Discovery module initialized");

    // Start modules
    Logger::info("Starting modules...");

    // Start pricing manager first
    market_data_manager.start().await;
    Logger::pricing("Market data manager started");

    // Start discovery module
    let _ = discovery.start().await;
    Logger::discovery("Discovery module started");

    Logger::success("All modules started SUCCESSFULLY!");
    Logger::info("Press Ctrl+C to stop the bot");
    Logger::separator();

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
    // Note: market_data_manager doesn't need explicit stop - background tasks will be dropped

    Logger::success("ScreenerBot shutdown complete");

    Ok(())
}
