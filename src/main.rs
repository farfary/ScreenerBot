use anyhow::Result;
use screenerbot::{ Config, Discovery, MarketData };
use std::sync::Arc;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    // Print header
    println!("\n==============================");
    println!("      Solana DEX Trader Bot     ");
    println!("==============================\n");
    println!("ScreenerBot is starting up...\n");

    // Load configuration
    let config = match Config::load("configs.json") {
        Ok(config) => {
            println!("✅ Loaded configuration");
            config
        }
        Err(e) => {
            eprintln!("❌ Could not load config: {}", e);
            println!("Generating default configuration...");
            let config = Config::default();
            config.save("configs.json")?;
            println!(
                "✅ Default configuration created. Please update configs.json with your settings."
            );
            return Ok(());
        }
    };

    // Initialize modules
    println!("\nInitializing modules...");

    // Discovery module
    let discovery = Arc::new(Discovery::new(config.discovery.clone())?);
    println!("🔎 Discovery module ready");

    // Market data module
    let market_data = Arc::new(MarketData::new(discovery.get_database())?);
    println!("💹 Market data module ready");

    // Start modules
    println!("\nStarting modules...");

    // Start discovery module
    let _ = discovery.start().await;
    println!("🔎 Discovery module running");

    // Start market data module
    let _ = market_data.start().await;
    println!("💹 Market data module running");

    println!("\n✅ All modules started successfully");
    println!("Press Ctrl+C to exit");
    println!("--------------------------------");

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            println!("\n🛑 Shutdown signal received");
        }
        Err(err) => {
            eprintln!("❌ Failed to listen for shutdown signal: {}", err);
        }
    }

    // Shutdown modules
    println!("--------------------------------");
    println!("Shutting down modules...");

    discovery.stop().await;
    market_data.stop().await;

    println!("✅ ScreenerBot shutdown complete\n");

    Ok(())
}
