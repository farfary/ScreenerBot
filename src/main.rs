use anyhow::Result;
use screenerbot::{ Config, Database, Discovery, Logger, PricingManager };
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

    // Pricing manager
    let mut pricing_manager = PricingManager::new(
        Arc::clone(&database),
        Arc::new(Logger::new()),
        config.pricing
            .as_ref()
            .map(|p| p.update_interval_secs)
            .unwrap_or(300), // 5 minutes default
        config.pricing
            .as_ref()
            .map(|p| p.top_tokens_count)
            .unwrap_or(100) // Top 100 tokens default
    );

    // Enable the sophisticated tiered pricing system
    if let Err(e) = pricing_manager.enable_tiered_pricing().await {
        Logger::warn(&format!("Failed to enable tiered pricing: {}", e));
        Logger::info("Continuing with basic pricing system...");
    }

    let pricing_manager = Arc::new(pricing_manager);
    Logger::pricing("Pricing manager initialized");

    // Discovery module
    let discovery = Arc::new(Discovery::new(config.discovery.clone(), Arc::clone(&database)));
    Logger::discovery("Discovery module initialized");

    // Start modules
    Logger::info("Starting modules...");

    // Start pricing manager first
    pricing_manager.start().await;
    Logger::pricing("Pricing manager started");

    // Start discovery module
    let _ = discovery.start().await;
    Logger::discovery("Discovery module started");

    Logger::success("All modules started SUCCESSFULLY!");
    Logger::info("Press Ctrl+C to stop the bot");
    Logger::separator();

    // Start status display loop
    let status_discovery = Arc::clone(&discovery);
    let status_pricing = Arc::clone(&pricing_manager);

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(5000)); // Update every 5 seconds

        loop {
            interval.tick().await;

            // Display status
            display_status(&status_discovery, &status_pricing).await;
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
    // Note: pricing_manager doesn't need explicit stop - background tasks will be dropped

    Logger::success("ScreenerBot shutdown complete");

    Ok(())
}

async fn display_status(discovery: &Arc<Discovery>, pricing_manager: &Arc<PricingManager>) {
    // Discovery status
    if discovery.is_running().await {
        let stats = discovery.get_stats().await;
        let cached_tokens = discovery.get_cached_tokens().await;

        Logger::discovery(
            &format!(
                "Active | {} tokens | {:.1}/hr discovery rate",
                cached_tokens.len(),
                stats.discovery_rate_per_hour
            )
        );
    }

    // Pricing status
    let cache_stats = pricing_manager.get_cache_stats().await;
    Logger::info(
        &format!(
            "Pricing: {} tokens | {} pools | {:.0}% hit rate",
            cache_stats.valid_tokens,
            cache_stats.valid_pools,
            cache_stats.hit_rate_tokens() * 100.0
        )
    );
}
