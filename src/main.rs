#[tokio::main]
async fn main() {
    use screenerbot::logger::{ log, LogLevel };

    log("MAIN", LogLevel::Info, "ScreenerBot starting...");

    // Initialize all background tasks
    log("MAIN", LogLevel::Info, "Starting configuration manager...");
    screenerbot::configs::start_config_manager();

    log("MAIN", LogLevel::Info, "Starting logger manager...");
    screenerbot::logger::start_logger_manager();

    log("MAIN", LogLevel::Info, "Starting RPC manager...");
    screenerbot::rpc::start_rpc_manager();

    log("MAIN", LogLevel::Info, "Starting wallet manager...");
    screenerbot::wallet::start_wallet_manager();

    log("MAIN", LogLevel::Info, "Starting pools manager...");
    screenerbot::pools::start_pools_manager();

    log("MAIN", LogLevel::Info, "Starting trader manager...");
    screenerbot::trader::start_trader();

    log("MAIN", LogLevel::Info, "Starting monitor manager...");
    screenerbot::monitor::start_monitoring();

    log("MAIN", LogLevel::Info, "All background tasks started successfully");
    log("MAIN", LogLevel::Info, "ScreenerBot is now running. Press Ctrl+C to shutdown.");

    // Setup graceful shutdown
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            log("MAIN", LogLevel::Info, "Shutdown signal received");
            screenerbot::global::trigger_shutdown();
            
            // Give tasks time to shut down gracefully
            log("MAIN", LogLevel::Info, "Waiting for tasks to shutdown...");
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            
            log("MAIN", LogLevel::Info, "ScreenerBot shutdown complete");
        }
        _ = keep_alive() => {
            log("MAIN", LogLevel::Error, "Keep alive loop ended unexpectedly");
        }
    }
}

async fn keep_alive() {
    loop {
        if screenerbot::global::is_shutdown() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}
