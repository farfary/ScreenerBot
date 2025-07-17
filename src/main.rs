use anyhow::Result;
use screenerbot::{ Config, Discovery, MarketData, Logger };
use std::sync::Arc;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    Logger::header("Solana DEX Trader Bot");
    Logger::info("ScreenerBot is starting up");

    // Load configuration
    let config = match Config::load("configs.json") {
        Ok(config) => {
            Logger::success("Loaded configuration");
            config
        }
        Err(e) => {
            Logger::error(&format!("Could not load config: {}", e));
            Logger::info("Generating default configuration");
            let config = Config::default();
            config.save("configs.json")?;
            Logger::success(
                "Default configuration created. Please update configs.json with your settings."
            );
            return Ok(());
        }
    };

    // Initialize modules
    Logger::info("Initializing modules");

    // Discovery module
    let discovery = Arc::new(Discovery::new(config.discovery.clone())?);
    Logger::discovery("Discovery module ready");

    // Market data module
    let market_data = Arc::new(MarketData::new(discovery.get_database())?);
    Logger::info("Market data module ready");

    // Start modules
    Logger::info("Starting modules");

    // Start discovery module
    let _ = discovery.start().await;
    Logger::discovery("Discovery module running");

    // Start market data module
    let _ = market_data.start().await;
    Logger::info("Market data module running");

    Logger::success("All modules started successfully");
    Logger::info("Press Ctrl+C to exit");
    Logger::separator();

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            Logger::info("Shutdown signal received");
        }
        Err(err) => {
            Logger::error(&format!("Failed to listen for shutdown signal: {}", err));
        }
    }

    // Shutdown modules
    Logger::separator();
    Logger::info("Shutting down modules");

    discovery.stop().await;
    market_data.stop().await;

    Logger::success("ScreenerBot shutdown complete");

    Ok(())
}
