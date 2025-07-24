// Test the new token monitoring system with database checks and blacklisting
use screenerbot::global::initialize_token_database;
use screenerbot::token_blacklist::get_blacklist_stats;
use screenerbot::token_monitor::start_token_monitoring;
use screenerbot::discovery_manager::start_discovery_task;
use screenerbot::logger::{ log, LogTag };
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{ Duration, sleep };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing new token monitoring system...");

    // Initialize token database
    if let Err(e) = initialize_token_database() {
        eprintln!("Failed to initialize token database: {}", e);
        return Err(e);
    }

    // Create shutdown notifier
    let shutdown = Arc::new(Notify::new());
    let shutdown_discovery = shutdown.clone();
    let shutdown_monitoring = shutdown.clone();

    // Start discovery task
    let discovery_handle = tokio::spawn(async move {
        log(LogTag::System, "TEST", "Starting discovery task...");
        start_discovery_task(shutdown_discovery).await;
    });

    // Start token monitoring task
    let monitoring_handle = tokio::spawn(async move {
        log(LogTag::System, "TEST", "Starting token monitoring task...");
        start_token_monitoring(shutdown_monitoring).await;
    });

    // Let it run for a few minutes to test
    log(LogTag::System, "TEST", "Running monitoring system for 5 minutes...");

    for minute in 1..=5 {
        sleep(Duration::from_secs(60)).await;

        let (blacklisted, tracking) = get_blacklist_stats();
        log(
            LogTag::System,
            "TEST",
            &format!(
                "Minute {}: {} tokens blacklisted, {} being tracked for low liquidity",
                minute,
                blacklisted,
                tracking
            )
        );
    }

    // Shutdown
    log(LogTag::System, "TEST", "Shutting down monitoring system...");
    shutdown.notify_waiters();

    // Wait a bit for graceful shutdown
    sleep(Duration::from_secs(2)).await;

    // Cancel tasks if they haven't finished
    discovery_handle.abort();
    monitoring_handle.abort();

    log(LogTag::System, "SUCCESS", "Token monitoring system test completed!");

    Ok(())
}
