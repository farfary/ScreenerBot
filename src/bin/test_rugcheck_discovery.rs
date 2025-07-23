use screenerbot::discovery::*;
use screenerbot::global::*;
use screenerbot::logger::{ log, LogTag };
use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let shutdown = Arc::new(Notify::new());

    log(LogTag::Monitor, "INFO", "Testing RugCheck API integration...");

    // Test all four RugCheck endpoints
    log(LogTag::Monitor, "INFO", "Testing RugCheck verified tokens...");
    if let Err(e) = discovery_rugcheck_fetch_verified().await {
        log(LogTag::Monitor, "ERROR", &format!("Failed to fetch verified: {}", e));
    } else {
        log(LogTag::Monitor, "SUCCESS", "RugCheck verified tokens fetched successfully");
    }

    log(LogTag::Monitor, "INFO", "Testing RugCheck trending tokens...");
    if let Err(e) = discovery_rugcheck_fetch_trending().await {
        log(LogTag::Monitor, "ERROR", &format!("Failed to fetch trending: {}", e));
    } else {
        log(LogTag::Monitor, "SUCCESS", "RugCheck trending tokens fetched successfully");
    }

    log(LogTag::Monitor, "INFO", "Testing RugCheck recent tokens...");
    if let Err(e) = discovery_rugcheck_fetch_recent().await {
        log(LogTag::Monitor, "ERROR", &format!("Failed to fetch recent: {}", e));
    } else {
        log(LogTag::Monitor, "SUCCESS", "RugCheck recent tokens fetched successfully");
    }

    log(LogTag::Monitor, "INFO", "Testing RugCheck new tokens...");
    if let Err(e) = discovery_rugcheck_fetch_new_tokens().await {
        log(LogTag::Monitor, "ERROR", &format!("Failed to fetch new tokens: {}", e));
    } else {
        log(LogTag::Monitor, "SUCCESS", "RugCheck new tokens fetched successfully");
    }

    // Check how many mints were discovered
    let mint_count = match LIST_MINTS.read() {
        Ok(set) => set.len(),
        Err(_) => 0,
    };

    log(LogTag::Monitor, "INFO", &format!("Total mints discovered: {}", mint_count));

    // Show some sample mints for verification
    if let Ok(set) = LIST_MINTS.read() {
        let sample_mints: Vec<String> = set.iter().take(10).cloned().collect();
        if !sample_mints.is_empty() {
            log(LogTag::Monitor, "DEBUG", &format!("Sample mints: {:?}", sample_mints));
        }
    }

    log(LogTag::Monitor, "SUCCESS", "RugCheck API integration test completed");
    Ok(())
}
