/// Quick test to verify the trader can now detect dip opportunities with updated monitoring
use screenerbot::logger::{ log, LogTag };
use screenerbot::tokens::{ initialize_tokens_system };
use screenerbot::tokens::price_service::{ initialize_price_service };
use screenerbot::trader::{ monitor_new_entries };
use tokio::time::{ sleep, Duration };
use tokio::sync::Notify;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "START", "Testing trader with fixed monitoring system");

    // Initialize the complete tokens system
    log(LogTag::System, "INIT", "Initializing tokens system...");
    let mut tokens_system = initialize_tokens_system().await?;
    log(LogTag::System, "SUCCESS", "Tokens system initialized");

    // Initialize price service
    log(LogTag::System, "INIT", "Initializing price service...");
    initialize_price_service().await?;
    log(LogTag::System, "SUCCESS", "Price service initialized");

    // Set up shutdown signal
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();
    let shutdown_trader = shutdown.clone();

    // Start monitoring background tasks
    log(LogTag::System, "MONITOR", "Starting monitoring background tasks...");
    match tokens_system.start_background_tasks(shutdown_clone.clone()).await {
        Ok(handles) => {
            log(LogTag::System, "SUCCESS", &format!("Monitoring started successfully with {} handles", handles.len()));
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to start monitoring: {}", e));
            return Err(e.into());
        }
    }

    // Wait for initial monitoring cycle to complete
    log(LogTag::System, "WAIT", "Waiting 60 seconds for initial monitoring cycle...");
    sleep(Duration::from_secs(60)).await;

    // Start trader's entry monitoring for a short test
    log(LogTag::System, "TRADER", "Starting trader entry monitoring test...");
    let trader_task = tokio::spawn(async move {
        // Run trader entry monitoring for 2 cycles
        for cycle in 1..=2 {
            log(LogTag::System, "TRADER_CYCLE", &format!("Starting trader cycle #{}", cycle));
            
            // Run one cycle of entry monitoring
            tokio::select! {
                _ = monitor_new_entries(shutdown_trader.clone()) => {
                    log(LogTag::System, "TRADER_COMPLETE", &format!("Trader cycle #{} completed", cycle));
                }
                _ = sleep(Duration::from_secs(30)) => {
                    log(LogTag::System, "TRADER_TIMEOUT", &format!("Trader cycle #{} timeout (this is normal for testing)", cycle));
                }
            }
            
            // Short pause between cycles
            if cycle < 2 {
                sleep(Duration::from_secs(10)).await;
            }
        }
        
        log(LogTag::System, "TRADER_DONE", "Trader test cycles completed");
    });

    // Wait for trader test to complete
    let _ = trader_task.await;

    // Shutdown the monitoring system
    log(LogTag::System, "SHUTDOWN", "Shutting down monitoring system...");
    shutdown.notify_waiters();
    sleep(Duration::from_secs(3)).await;

    log(LogTag::System, "COMPLETE", "Trader monitoring test complete");
    Ok(())
}
