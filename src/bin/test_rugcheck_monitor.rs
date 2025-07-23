use screenerbot::discovery::*;
use screenerbot::global::*;
use screenerbot::logger::{ log, LogTag };
use screenerbot::monitor::monitor;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::Monitor, "INFO", "Testing RugCheck integration in monitor loop...");

    let shutdown = Arc::new(Notify::new());

    // Start the monitor task
    let monitor_shutdown = shutdown.clone();
    let monitor_handle = tokio::spawn(async move {
        monitor(monitor_shutdown).await;
    });

    // Let it run for 15 seconds to see the RugCheck APIs being called
    tokio::time::sleep(Duration::from_secs(15)).await;

    // Trigger shutdown
    shutdown.notify_one();

    // Wait for monitor to finish
    if let Err(e) = monitor_handle.await {
        log(LogTag::Monitor, "ERROR", &format!("Monitor task error: {}", e));
    }

    // Check final mint count
    let mint_count = match LIST_MINTS.read() {
        Ok(set) => set.len(),
        Err(_) => 0,
    };

    log(
        LogTag::Monitor,
        "SUCCESS",
        &format!("Monitor test completed. Final mint count: {}", mint_count)
    );

    Ok(())
}
