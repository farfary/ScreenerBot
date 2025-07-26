/// Test to verify the monitoring system is actually making API calls and updating prices
use screenerbot::logger::{ log, LogTag };
use screenerbot::tokens::{ initialize_tokens_system, TokensSystem };
use screenerbot::tokens::price_service::{ initialize_price_service, get_price_cache_stats };
use tokio::time::{ sleep, Duration };
use tokio::sync::Notify;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "START", "Testing monitoring system functionality");

    // Initialize the complete tokens system
    log(LogTag::System, "INIT", "Initializing tokens system...");
    let mut tokens_system = initialize_tokens_system().await?;
    log(LogTag::System, "SUCCESS", "Tokens system initialized");

    // Initialize price service
    log(LogTag::System, "INIT", "Initializing price service...");
    initialize_price_service().await?;
    log(LogTag::System, "SUCCESS", "Price service initialized");

    // Get initial cache stats
    let initial_stats = get_price_cache_stats().await;
    log(LogTag::System, "CACHE_STATS_INITIAL", &initial_stats);

    // Set up shutdown signal
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    // Start monitoring background tasks with a timeout
    log(LogTag::System, "MONITOR", "Starting monitoring background tasks...");
    
    // Start the monitoring system
    match tokens_system.start_background_tasks(shutdown_clone.clone()).await {
        Ok(handles) => {
            log(LogTag::System, "SUCCESS", &format!("Monitoring started successfully with {} handles", handles.len()));
            
            // Wait for either timeout or shutdown
            tokio::select! {
                _ = sleep(Duration::from_secs(90)) => {
                    log(LogTag::System, "MONITOR_TIMEOUT", "Monitoring timeout reached, shutting down");
                    shutdown_clone.notify_waiters();
                    sleep(Duration::from_secs(2)).await; // Give tasks time to shutdown
                }
                _ = shutdown_clone.notified() => {
                    log(LogTag::System, "SHUTDOWN", "Shutdown signal received");
                }
            }
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to start monitoring: {}", e));
        }
    }

    // Monitor cache stats every 30 seconds
    let stats_task = tokio::spawn(async move {
        for i in 1..=3 {
            sleep(Duration::from_secs(30)).await;
            let stats = get_price_cache_stats().await;
            log(LogTag::System, &format!("CACHE_STATS_{}", i), &stats);
        }
    });

    // Wait for stats task to complete
    let _ = stats_task.await;

    // Shutdown the monitoring system
    shutdown.notify_waiters();
    sleep(Duration::from_secs(2)).await;

    // Get final cache stats
    let final_stats = get_price_cache_stats().await;
    log(LogTag::System, "CACHE_STATS_FINAL", &final_stats);

    log(LogTag::System, "COMPLETE", "Monitoring system test complete");
    Ok(())
}
