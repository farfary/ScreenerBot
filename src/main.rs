use screenerbot::trader::trader;
use screenerbot::logger::{ log, LogTag, init_file_logging };

use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::main]
async fn main() {
    // Initialize file logging system first
    init_file_logging();

    log(LogTag::System, "INFO", "Starting ScreenerBot background tasks");

    // Initialize token database
    if let Err(e) = screenerbot::tokens::initialize_token_database() {
        log(LogTag::System, "ERROR", &format!("Failed to initialize token database: {}", e));
        std::process::exit(1);
    }

    // Initialize tokens system
    let mut tokens_system = match screenerbot::tokens::initialize_tokens_system().await {
        Ok(system) => system,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to initialize tokens system: {}", e));
            std::process::exit(1);
        }
    };

    let shutdown = Arc::new(Notify::new());
    let shutdown_trader = shutdown.clone();
    let shutdown_tokens = shutdown.clone();
    let shutdown_pricing = shutdown.clone();

    // Start tokens system background tasks
    let tokens_handles = match tokens_system.start_background_tasks(shutdown_tokens).await {
        Ok(handles) => handles,
        Err(e) => {
            log(
                LogTag::System,
                "WARN",
                &format!("Some tokens system tasks failed to start: {}", e)
            );
            Vec::new()
        }
    };

    // Start pricing background tasks
    let pricing_handles = match
        screenerbot::tokens::start_pricing_background_tasks(shutdown_pricing).await
    {
        Ok(handles) => handles,
        Err(e) => {
            log(
                LogTag::System,
                "WARN",
                &format!("Pricing background tasks failed to start: {}", e)
            );
            Vec::new()
        }
    };

    let trader_handle = tokio::spawn(async move {
        log(LogTag::Trader, "INFO", "Trader task started");
        trader(shutdown_trader).await;
        log(LogTag::Trader, "INFO", "Trader task ended");
    });

    log(LogTag::System, "INFO", "Waiting for Ctrl+C to shutdown");
    tokio::signal::ctrl_c().await.expect("failed to listen for event");
    log(LogTag::System, "INFO", "Shutdown signal received, notifying tasks");
    shutdown.notify_waiters();

    // Wait for background tasks to finish with timeout
    let shutdown_timeout = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        // Wait for trader task
        let _ = trader_handle.await;

        // Wait for tokens system tasks
        for handle in tokens_handles {
            let _ = handle.await;
        }

        // Wait for pricing tasks
        for handle in pricing_handles {
            let _ = handle.await;
        }
    });

    match shutdown_timeout.await {
        Ok(_) => {
            log(LogTag::System, "INFO", "All background tasks finished gracefully. Exiting.");
        }
        Err(_) => {
            log(LogTag::System, "WARN", "Tasks did not finish within timeout, forcing exit.");
        }
    }

    // Force exit to ensure clean shutdown
    std::process::exit(0);
}

// Access CMD_ARGS anywhere via CMD_ARGS.lock().unwrap()
