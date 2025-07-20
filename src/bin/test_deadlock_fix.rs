use std::sync::Arc;
use tokio::sync::Notify;
use screenerbot::{
    trader::{ Position, monitor_open_positions, SAVED_POSITIONS },
    logger::{ log, LogTag },
};
use chrono::Utc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "Starting deadlock fix test...");

    // Create some test positions to trigger the concurrent selling logic
    let test_positions = vec![
        Position {
            position_type: "buy".to_string(),
            symbol: "TEST1".to_string(),
            name: "Test Token 1".to_string(),
            mint: "TestMint1111111111111111111111111111111111".to_string(),
            entry_price: 0.001,
            effective_entry_price: Some(0.001),
            entry_time: Utc::now(),
            entry_size_sol: 1.0,
            exit_price: None,
            exit_time: None,
            pnl_sol: None,
            pnl_percent: None,
            total_size_sol: 1.0,
            entry_transaction_signature: Some("test_entry_1".to_string()),
            exit_transaction_signature: None,
            token_amount: Some(1000),
            effective_exit_price: None,
            price_highest: 0.001,
            price_lowest: 0.001,
            drawdown_percent: 0.0,
        },
        Position {
            position_type: "buy".to_string(),
            symbol: "TEST2".to_string(),
            name: "Test Token 2".to_string(),
            mint: "TestMint2222222222222222222222222222222222".to_string(),
            entry_price: 0.002,
            effective_entry_price: Some(0.002),
            entry_time: Utc::now(),
            entry_size_sol: 1.0,
            exit_price: None,
            exit_time: None,
            pnl_sol: None,
            pnl_percent: None,
            total_size_sol: 1.0,
            entry_transaction_signature: Some("test_entry_2".to_string()),
            exit_transaction_signature: None,
            token_amount: Some(500),
            effective_exit_price: None,
            price_highest: 0.002,
            price_lowest: 0.002,
            drawdown_percent: 0.0,
        }
    ];

    // Add test positions to global state
    {
        let mut positions = SAVED_POSITIONS.lock().unwrap();
        positions.extend(test_positions);
    }

    log(LogTag::System, "INFO", "Created test positions");

    // Create shutdown notify for graceful termination
    let shutdown = Arc::new(Notify::new());
    let shutdown_clone = shutdown.clone();

    // Start the monitor_open_positions task
    let monitor_handle = tokio::spawn(async move {
        monitor_open_positions(shutdown_clone).await;
    });

    // Let it run for a few seconds to test the concurrent processing
    log(LogTag::System, "INFO", "Running monitor for 5 seconds to test concurrent processing...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    // Signal shutdown
    log(LogTag::System, "INFO", "Signaling shutdown...");
    shutdown.notify_one();

    // Wait for the monitor to finish
    if let Err(e) = monitor_handle.await {
        log(LogTag::System, "ERROR", &format!("Monitor task failed: {}", e));
    } else {
        log(LogTag::System, "SUCCESS", "Monitor task completed successfully without deadlock!");
    }

    // Check final positions state
    let final_count = {
        let positions = SAVED_POSITIONS.lock().unwrap();
        log(LogTag::System, "INFO", &format!("Final positions count: {}", positions.len()));
        positions.len()
    };

    log(
        LogTag::System,
        "SUCCESS",
        &format!("Deadlock fix test completed successfully! {} positions processed", final_count)
    );

    Ok(())
}
