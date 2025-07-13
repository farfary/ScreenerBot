use anyhow::Result;
use env_logger;
use log::{ info, error };
use tokio::signal;

mod core;
mod wallet;
mod screener;
mod trader;
mod portfolio;
mod cache;
mod pools;

use core::BotRuntime;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("🤖 Starting ScreenerBot v0.1.0");

    // Load configuration
    let config_path = "configs.json";

    // Initialize and start the bot
    let mut bot = match BotRuntime::new(config_path).await {
        Ok(bot) => {
            info!("✅ Bot initialized successfully");
            bot
        }
        Err(e) => {
            error!("❌ Failed to initialize bot: {}", e);
            return Err(e);
        }
    };

    // Setup graceful shutdown
    let shutdown_signal = async {
        signal::ctrl_c().await.expect("Failed to install CTRL+C signal handler");
        info!("🛑 Shutdown signal received");
    };

    // Run the bot
    tokio::select! {
        result = bot.start() => {
            match result {
                Ok(_) => info!("🏁 Bot finished successfully"),
                Err(e) => error!("💥 Bot error: {}", e),
            }
        }
        _ = shutdown_signal => {
            info!("🛑 Gracefully shutting down...");
            bot.stop();
        }
    }

    Ok(())
}
