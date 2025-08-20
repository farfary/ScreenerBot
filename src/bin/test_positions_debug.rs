use screenerbot::arguments::set_cmd_args;
use screenerbot::logger::{log, LogTag, init_file_logging};
use screenerbot::positions::{start_positions_manager_service, get_open_positions_count, get_open_positions};
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up arguments to enable debug mode
    set_cmd_args(vec![
        "test_positions_debug".to_string(),
        "--debug-positions".to_string(),
        "--dry-run".to_string(),
    ]);

    // Initialize logging
    init_file_logging();
    
    log(LogTag::System, "INFO", "🧪 Starting positions debug test");
    
    // Create shutdown notification
    let shutdown = Arc::new(Notify::new());
    
    // Start positions manager service
    start_positions_manager_service(shutdown.clone()).await;
    
    // Wait a moment for service to start
    sleep(Duration::from_millis(500)).await;
    
    // Test getting position counts
    log(LogTag::System, "INFO", "📊 Testing position queries...");
    
    let open_count = get_open_positions_count().await;
    let open_positions = get_open_positions().await;
    
    log(LogTag::System, "INFO", &format!(
        "📈 Found {} open positions", open_count
    ));
    
    for position in &open_positions {
        log(LogTag::System, "INFO", &format!(
            "  - {} ({}) entry: {:.8} SOL", 
            position.symbol, 
            &position.mint[..8],
            position.entry_price
        ));
    }
    
    // Test periodic ticks by waiting
    log(LogTag::System, "INFO", "⏱️ Waiting 12 seconds to observe verification tick...");
    sleep(Duration::from_secs(12)).await;
    
    // Shutdown gracefully
    log(LogTag::System, "INFO", "🛑 Shutting down test");
    shutdown.notify_waiters();
    
    // Wait for clean shutdown
    sleep(Duration::from_millis(100)).await;
    
    log(LogTag::System, "INFO", "✅ Positions debug test completed");
    
    Ok(())
}
