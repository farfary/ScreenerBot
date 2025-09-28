/// Quick test binary to verify security summary functionality
use screenerbot::{
    logger::{init_file_logging, log, LogTag},
    tokens::security::start_security_monitoring,
};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();

    log(
        LogTag::System,
        "START",
        "Testing security summary functionality",
    );

    // Create shutdown signal
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    // Start security monitoring (it will print summaries every 30 seconds)
    log(
        LogTag::System,
        "INFO",
        "Starting security monitoring service for testing...",
    );

    match start_security_monitoring(shutdown_clone).await {
        Ok(handle) => {
            log(
                LogTag::System,
                "INFO",
                "Security monitoring started successfully",
            );

            // Let it run for 2 minutes to see a few summary cycles
            log(
                LogTag::System,
                "INFO",
                "Running for 2 minutes to observe security summaries...",
            );
            tokio::time::sleep(Duration::from_secs(120)).await;

            // Signal shutdown
            log(LogTag::System, "INFO", "Signaling shutdown...");
            shutdown.notify_waiters();

            // Wait for clean shutdown
            match tokio::time::timeout(Duration::from_secs(5), handle).await {
                Ok(result) => {
                    if let Err(e) = result {
                        log(
                            LogTag::System,
                            "ERROR",
                            &format!("Security monitoring task error: {:?}", e),
                        );
                    } else {
                        log(
                            LogTag::System,
                            "INFO",
                            "Security monitoring stopped cleanly",
                        );
                    }
                }
                Err(_) => {
                    log(
                        LogTag::System,
                        "WARNING",
                        "Security monitoring task did not stop within timeout",
                    );
                }
            }
        }
        Err(e) => {
            log(
                LogTag::System,
                "ERROR",
                &format!("Failed to start security monitoring: {}", e),
            );
            return Err(e.into());
        }
    }

    log(
        LogTag::System,
        "COMPLETE",
        "Security summary test completed",
    );
    Ok(())
}
