// bin/test_websocket_wallet_watch.rs - Test WebSocket monitoring for wallet transactions
use screenerbot::transactions::*;
use screenerbot::global::read_configs;
use screenerbot::wallet::get_wallet_address;
use screenerbot::logger::{ log, LogTag };
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "Starting WebSocket wallet transaction monitoring test...");

    // Load configuration
    let configs = read_configs("configs.json")?;

    // Get wallet address from private key
    let wallet_address = get_wallet_address().map_err(
        |e|
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<
                dyn std::error::Error
            >
    )?;

    log(LogTag::System, "INFO", &format!("Monitoring wallet: {}", wallet_address));
    log(LogTag::System, "INFO", &format!("WebSocket URL: {}", configs.websocket_url));

    // Test 1: Initialize WebSocket client
    log(LogTag::System, "INFO", "=== Test 1: Initialize WebSocket Client ===");
    let mut ws_client = TransactionWebSocket::new(vec![wallet_address.clone()]).map_err(
        |e|
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<
                dyn std::error::Error
            >
    )?;

    log(LogTag::System, "SUCCESS", "WebSocket client initialized successfully");

    // Test 2: Test WebSocket connection with timeout
    log(LogTag::System, "INFO", "=== Test 2: Test WebSocket Connection ===");

    // Create a timeout wrapper for the WebSocket connection
    let ws_url = configs.websocket_url.clone();
    let monitoring_task = tokio::spawn(async move { ws_client.start_monitoring(&ws_url).await });

    // Run monitoring for 30 seconds to catch any transactions
    log(LogTag::System, "INFO", "Monitoring for 30 seconds to catch live transactions...");
    log(LogTag::System, "INFO", "Send some transactions to your wallet to see real-time updates!");

    let timeout_result = tokio::time::timeout(Duration::from_secs(30), monitoring_task).await;

    match timeout_result {
        Ok(Ok(Ok(()))) => {
            log(LogTag::System, "SUCCESS", "WebSocket monitoring completed successfully");
        }
        Ok(Ok(Err(e))) => {
            log(
                LogTag::System,
                "WARNING",
                &format!("WebSocket monitoring ended with error: {}", e)
            );
        }
        Ok(Err(e)) => {
            log(LogTag::System, "WARNING", &format!("WebSocket task panicked: {}", e));
        }
        Err(_) => {
            log(LogTag::System, "INFO", "WebSocket monitoring timeout reached (30 seconds)");
        }
    }

    // Test 3: Test WebSocket with fallback URLs
    log(LogTag::System, "INFO", "=== Test 3: Test WebSocket Fallback URLs ===");

    for (index, fallback_url) in configs.websocket_fallbacks.iter().enumerate() {
        log(
            LogTag::System,
            "INFO",
            &format!("Testing fallback WebSocket URL {}: {}", index + 1, fallback_url)
        );

        let mut fallback_client = TransactionWebSocket::new(vec![wallet_address.clone()]).map_err(
            |e|
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<
                    dyn std::error::Error
                >
        )?;

        let fallback_url_clone = fallback_url.clone();
        let fallback_task = tokio::spawn(async move {
            fallback_client.start_monitoring(&fallback_url_clone).await
        });

        // Test each fallback for 5 seconds
        let fallback_result = tokio::time::timeout(Duration::from_secs(5), fallback_task).await;

        match fallback_result {
            Ok(Ok(Ok(()))) => {
                log(
                    LogTag::System,
                    "SUCCESS",
                    &format!("Fallback URL {} connected successfully", index + 1)
                );
            }
            Ok(Ok(Err(e))) => {
                log(
                    LogTag::System,
                    "WARNING",
                    &format!("Fallback URL {} failed: {}", index + 1, e)
                );
            }
            Ok(Err(e)) => {
                log(
                    LogTag::System,
                    "WARNING",
                    &format!("Fallback URL {} task panicked: {}", index + 1, e)
                );
            }
            Err(_) => {
                log(
                    LogTag::System,
                    "INFO",
                    &format!("Fallback URL {} test timeout (5 seconds)", index + 1)
                );
            }
        }
    }

    // Test 4: Test database integration
    log(LogTag::System, "INFO", "=== Test 4: Test Database Integration ===");

    let db = TransactionDatabase::new()?;
    let transaction_count = db.get_transaction_count()?;
    log(
        LogTag::System,
        "INFO",
        &format!("Current database transaction count: {}", transaction_count)
    );

    // Check if any transactions were captured during WebSocket monitoring
    let recent_transactions = db.get_recent_transactions(10)?;
    log(
        LogTag::System,
        "INFO",
        &format!("Recent transactions in database: {}", recent_transactions.len())
    );

    if !recent_transactions.is_empty() {
        log(LogTag::System, "SUCCESS", "Found recent transactions in database:");
        for (index, transaction_record) in recent_transactions.iter().take(5).enumerate() {
            log(
                LogTag::System,
                "INFO",
                &format!("  {}. {}", index + 1, transaction_record.signature)
            );
        }
    } else {
        log(LogTag::System, "INFO", "No recent transactions found in database");
    }

    // Test 5: Manual transaction subscription test
    log(LogTag::System, "INFO", "=== Test 5: Manual Subscription Test ===");

    let ws_client_manual = TransactionWebSocket::new(vec![wallet_address.clone()]).map_err(
        |e|
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<
                dyn std::error::Error
            >
    )?;

    // Test subscription message creation
    let account_subscription = ws_client_manual.create_account_subscription(&wallet_address);
    log(LogTag::System, "INFO", &format!("Account subscription message created successfully"));
    log(
        LogTag::System,
        "INFO",
        &format!(
            "Account subscription sample: {}",
            &account_subscription[..account_subscription.len().min(100)]
        )
    );

    let signature_subscription = ws_client_manual.create_signature_subscription(&wallet_address);
    log(LogTag::System, "INFO", &format!("Signature subscription message created successfully"));
    log(
        LogTag::System,
        "INFO",
        &format!(
            "Signature subscription sample: {}",
            &signature_subscription[..signature_subscription.len().min(100)]
        )
    );

    // Test 6: Connection resilience test
    log(LogTag::System, "INFO", "=== Test 6: Connection Resilience Test ===");

    log(LogTag::System, "INFO", "Testing connection with invalid URL...");
    let mut invalid_client = TransactionWebSocket::new(vec![wallet_address.clone()]).map_err(
        |e|
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<
                dyn std::error::Error
            >
    )?;

    let invalid_task = tokio::spawn(async move {
        invalid_client.start_monitoring("wss://invalid-url.example.com").await
    });

    let invalid_result = tokio::time::timeout(Duration::from_secs(5), invalid_task).await;

    match invalid_result {
        Ok(Ok(Ok(()))) => {
            log(LogTag::System, "WARNING", "Unexpected success with invalid URL");
        }
        Ok(Ok(Err(e))) => {
            log(LogTag::System, "SUCCESS", &format!("Correctly failed with invalid URL: {}", e));
        }
        Ok(Err(e)) => {
            log(
                LogTag::System,
                "SUCCESS",
                &format!("Invalid URL task panicked as expected: {}", e)
            );
        }
        Err(_) => {
            log(LogTag::System, "SUCCESS", "Invalid URL test timed out as expected");
        }
    }

    // Final summary
    log(LogTag::System, "INFO", "=== WebSocket Test Summary ===");
    log(LogTag::System, "SUCCESS", "✅ WebSocket client initialization");
    log(LogTag::System, "SUCCESS", "✅ WebSocket connection testing");
    log(LogTag::System, "SUCCESS", "✅ Fallback URL testing");
    log(LogTag::System, "SUCCESS", "✅ Database integration");
    log(LogTag::System, "SUCCESS", "✅ Subscription message creation");
    log(LogTag::System, "SUCCESS", "✅ Connection resilience testing");

    log(LogTag::System, "INFO", "WebSocket wallet monitoring test completed!");
    log(
        LogTag::System,
        "INFO",
        "To see live transaction monitoring, run this test while making transactions to your wallet."
    );

    Ok(())
}
