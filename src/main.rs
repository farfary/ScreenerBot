use anyhow::Result;
use screenerbot::{ Config, Discovery, MarketData, SwapManager, TraderManager };
use screenerbot::api::init_dexscreener_rate_limiter;
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

    // Initialize DexScreener rate limiter
    init_dexscreener_rate_limiter(config.dexscreener.clone()).await?;
    println!("⏱️  DexScreener rate limiter initialized");

    // Discovery module
    let discovery = Arc::new(Discovery::new(config.discovery.clone())?);
    println!("🔎 Discovery module ready");

    // Market data module
    let market_data = Arc::new(MarketData::new(discovery.get_database())?);
    println!("💹 Market data module ready");

    // RPC manager
    let rpc_manager = Arc::new(
        screenerbot::RpcManager::new(
            config.rpc_url.clone(),
            config.rpc_fallbacks.clone(),
            config.rpc.clone()
        )?
    );
    println!("🌐 RPC manager ready");

    // Swap manager
    let swap_manager = Arc::new(SwapManager::new(config.swap.clone(), Arc::clone(&rpc_manager)));
    println!("💱 Swap manager ready");

    // Trader module
    let trader = if config.trader.enabled {
        let trader_manager = Arc::new(
            TraderManager::new(
                config.trader.clone(),
                Arc::clone(&swap_manager),
                Arc::clone(&market_data),
                Arc::clone(&discovery)
            )?
        );
        println!("🎯 Trader module ready");
        Some(trader_manager)
    } else {
        println!("⚠️  Trader module disabled");
        None
    };

    // Rug detection monitor module
    let rug_monitor = if config.trader.rug_detection.enabled {
        use screenerbot::rug_detection::{ RugDetectionEngine, RugDetectionMonitor };

        let rug_engine = Arc::new(
            RugDetectionEngine::new(market_data.get_database(), config.trader.rug_detection.clone())
        );

        let monitor = Arc::new(
            RugDetectionMonitor::new(
                market_data.get_database(),
                rug_engine,
                config.trader.rug_detection.clone()
            )
        );

        println!("🚨 Rug detection monitor ready");
        Some(monitor)
    } else {
        println!("⚠️  Rug detection monitor disabled");
        None
    };

    // Start modules
    println!("\nStarting modules...");

    // Start discovery module
    let _ = discovery.start().await;
    println!("🔎 Discovery module running");

    // Start market data module
    let _ = market_data.start().await;
    println!("💹 Market data module running");

    // Start trader module
    if let Some(ref trader_manager) = trader {
        let _ = trader_manager.start().await;
        println!("🎯 Trader module running");
    }

    // Start rug detection monitor
    if let Some(ref monitor) = rug_monitor {
        let _ = monitor.start().await;
        println!("🚨 Rug detection monitor running");
    }

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

    if let Some(trader_manager) = trader {
        trader_manager.stop().await;
    }

    if let Some(monitor) = rug_monitor {
        monitor.stop().await;
    }

    println!("✅ ScreenerBot shutdown complete\n");

    Ok(())
}
