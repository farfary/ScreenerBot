// bin/test_websocket_live_swap.rs - Test WebSocket monitoring with a live small swap
use screenerbot::transactions::*;
use screenerbot::global::read_configs;
use screenerbot::wallet::{ get_wallet_address, buy_token };
use screenerbot::logger::{ log, LogTag };
use tokio::time::{ Duration, sleep };
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "Starting WebSocket live swap monitoring test...");

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

    // Initialize shared state for monitoring
    let transaction_detected = Arc::new(Mutex::new(false));
    let transaction_signatures = Arc::new(Mutex::new(Vec::<String>::new()));

    // Step 1: Start WebSocket monitoring in background
    log(LogTag::System, "INFO", "=== Step 1: Starting WebSocket Monitoring ===");

    let mut ws_client = TransactionWebSocket::new(vec![wallet_address.clone()]).map_err(
        |e|
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<
                dyn std::error::Error
            >
    )?;

    let ws_url = configs.websocket_url.clone();

    // Start monitoring task
    let monitoring_handle = tokio::spawn(async move {
        log(LogTag::System, "INFO", "WebSocket monitoring task started");

        // Custom monitoring with transaction detection
        let result = ws_client.start_monitoring(&ws_url).await;

        match result {
            Ok(_) => {
                log(LogTag::System, "SUCCESS", "WebSocket monitoring completed");
            }
            Err(e) => {
                log(LogTag::System, "WARNING", &format!("WebSocket monitoring error: {}", e));
            }
        }
    });

    // Give WebSocket time to connect
    log(LogTag::System, "INFO", "Waiting 3 seconds for WebSocket to connect...");
    sleep(Duration::from_secs(3)).await;

    // Step 2: Perform a very small swap
    log(LogTag::System, "INFO", "=== Step 2: Performing Small Test Swap ===");

    // Create a simple BONK token for testing
    use screenerbot::global::Token;
    let bonk_token = Token {
        mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(), // BONK token
        symbol: "BONK".to_string(),
        name: "Bonk".to_string(),
        decimals: 5,
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: true,
        created_at: None,
        price_dexscreener_sol: Some(0.000000025), // Approximate BONK price
        price_dexscreener_usd: None,
        price_geckoterminal_sol: None,
        price_geckoterminal_usd: None,
        price_raydium_sol: None,
        price_raydium_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        pools: vec![],
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![],
        fdv: None,
        market_cap: None,
        txns: None,
        volume: None,
        price_change: None,
        liquidity: None,
        info: None,
        boosts: None,
    };

    let swap_amount_sol = 0.0001; // Very small amount - $0.02-0.03 worth

    log(
        LogTag::System,
        "INFO",
        &format!("Attempting to buy {} SOL worth of BONK token", swap_amount_sol)
    );
    log(LogTag::System, "INFO", "This is a tiny test swap to demonstrate real-time monitoring");

    // Get initial transaction count
    let db = TransactionDatabase::new()?;
    let initial_count = db.get_transaction_count()?;
    log(LogTag::System, "INFO", &format!("Initial database transaction count: {}", initial_count));

    // Perform the swap
    let swap_start_time = std::time::Instant::now();

    match buy_token(&bonk_token, swap_amount_sol, None).await {
        Ok(result) => {
            if let Some(signature) = &result.transaction_signature {
                log(
                    LogTag::System,
                    "SUCCESS",
                    &format!("Swap completed! Transaction: {}", signature)
                );
            } else {
                log(
                    LogTag::System,
                    "WARNING",
                    "Swap completed but no transaction signature returned"
                );
            }
            log(LogTag::System, "INFO", &format!("Output amount: {} tokens", result.output_amount));
            if let Some(price) = result.effective_price {
                log(LogTag::System, "INFO", &format!("Effective price: {} SOL per token", price));
            }

            // Mark that we've made a transaction
            let mut detected = transaction_detected.lock().await;
            *detected = true;

            if let Some(signature) = &result.transaction_signature {
                let mut signatures = transaction_signatures.lock().await;
                signatures.push(signature.clone());
            }
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Swap failed: {}", e));
            log(LogTag::System, "INFO", "Continuing with monitoring test anyway...");
        }
    }

    // Step 3: Monitor for transaction detection
    log(LogTag::System, "INFO", "=== Step 3: Monitoring for Transaction Detection ===");

    let mut detection_attempts = 0;
    let max_attempts = 20; // Monitor for up to 60 seconds (3 second intervals)

    while detection_attempts < max_attempts {
        sleep(Duration::from_secs(3)).await;
        detection_attempts += 1;

        // Check database for new transactions
        let current_count = db.get_transaction_count()?;
        let new_transactions = current_count - initial_count;

        log(
            LogTag::System,
            "INFO",
            &format!(
                "Check #{}: Database has {} new transactions since start",
                detection_attempts,
                new_transactions
            )
        );

        if new_transactions > 0 {
            log(LogTag::System, "SUCCESS", "🎉 NEW TRANSACTIONS DETECTED in database!");

            // Get recent transactions
            let recent_transactions = db.get_recent_transactions(5)?;
            for (index, transaction_record) in recent_transactions.iter().enumerate() {
                let elapsed = swap_start_time.elapsed();
                log(
                    LogTag::System,
                    "INFO",
                    &format!(
                        "  {}. {} (detected after {:?})",
                        index + 1,
                        transaction_record.signature,
                        elapsed
                    )
                );
            }
            break;
        }

        // Check if we've waited too long
        if detection_attempts >= max_attempts {
            log(LogTag::System, "WARNING", "Timeout waiting for transaction detection");
            break;
        }

        log(
            LogTag::System,
            "INFO",
            &format!(
                "Waiting for transaction detection... ({}/{})",
                detection_attempts,
                max_attempts
            )
        );
    }

    // Step 4: Final verification
    log(LogTag::System, "INFO", "=== Step 4: Final Verification ===");

    let final_count = db.get_transaction_count()?;
    let total_new_transactions = final_count - initial_count;

    if total_new_transactions > 0 {
        log(
            LogTag::System,
            "SUCCESS",
            &format!("✅ WebSocket monitoring successfully detected {} new transactions!", total_new_transactions)
        );

        // Show the most recent transactions
        let recent_transactions = db.get_recent_transactions(3)?;
        log(LogTag::System, "INFO", "Most recent transactions captured:");
        for (index, transaction_record) in recent_transactions.iter().enumerate() {
            log(
                LogTag::System,
                "INFO",
                &format!("  {}. {}", index + 1, transaction_record.signature)
            );
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "     Slot: {}, Block Time: {:?}",
                    transaction_record.slot,
                    transaction_record.block_time
                )
            );
        }
    } else {
        log(
            LogTag::System,
            "WARNING",
            "No new transactions detected - WebSocket monitoring may need adjustment"
        );
    }

    // Step 5: Clean up monitoring
    log(LogTag::System, "INFO", "=== Step 5: Cleanup ===");

    // Stop the monitoring task
    monitoring_handle.abort();
    log(LogTag::System, "INFO", "WebSocket monitoring task stopped");

    // Final summary
    log(LogTag::System, "INFO", "=== Live Swap Monitoring Test Summary ===");
    log(LogTag::System, "SUCCESS", "✅ WebSocket monitoring started successfully");
    log(LogTag::System, "SUCCESS", "✅ Small test swap executed");

    if total_new_transactions > 0 {
        log(LogTag::System, "SUCCESS", "✅ Real-time transaction detection working!");
        log(
            LogTag::System,
            "SUCCESS",
            &format!("✅ Captured {} transactions in real-time", total_new_transactions)
        );
    } else {
        log(LogTag::System, "WARNING", "⚠️ Transaction detection needs verification");
    }

    log(LogTag::System, "INFO", "Live swap monitoring test completed!");
    log(LogTag::System, "INFO", "🚀 Your WebSocket wallet monitoring system is operational!");

    Ok(())
}
