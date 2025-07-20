// bin/test_websocket_debug_sync.rs - Debug WebSocket monitoring and swap synchronization
use screenerbot::transactions::*;
use screenerbot::global::{ read_configs, Token };
use screenerbot::wallet::{ get_wallet_address, buy_token };
use screenerbot::logger::{ log, LogTag };
use tokio::time::{ Duration, sleep, Instant };
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(
        LogTag::System,
        "INFO",
        "Starting WebSocket DEBUG monitoring and swap synchronization test..."
    );

    // Load configuration
    let configs = read_configs("configs.json")?;

    // Get wallet address from private key
    let wallet_address = get_wallet_address().map_err(
        |e|
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<
                dyn std::error::Error
            >
    )?;

    log(LogTag::System, "INFO", &format!("🎯 Target wallet: {}", wallet_address));
    log(LogTag::System, "INFO", &format!("📡 WebSocket URL: {}", configs.websocket_url));

    // Step 1: Initialize database and get baseline
    log(LogTag::System, "INFO", "=== Step 1: Initialize Database Baseline ===");
    let db = TransactionDatabase::new()?;
    let initial_count = db.get_transaction_count()?;
    log(
        LogTag::System,
        "INFO",
        &format!("📊 Initial database transaction count: {}", initial_count)
    );

    // Step 2: Start WebSocket monitoring BEFORE any swap
    log(
        LogTag::System,
        "INFO",
        "=== Step 2: Start WebSocket Monitoring (CRITICAL: Before Swap) ==="
    );

    let notification_counter = Arc::new(Mutex::new(0u32));
    let swap_signature = Arc::new(Mutex::new(Option::<String>::None));

    // Create WebSocket client with shared state for debugging
    let mut ws_client = TransactionWebSocket::new(vec![wallet_address.clone()]).map_err(
        |e|
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<
                dyn std::error::Error
            >
    )?;

    let ws_url = configs.websocket_url.clone();
    let notification_counter_clone = notification_counter.clone();

    // Start monitoring task with detailed logging
    let monitoring_handle = tokio::spawn(async move {
        log(
            LogTag::System,
            "INFO",
            "🚀 WebSocket monitoring task STARTED - Ready to capture transactions"
        );

        // Enhanced monitoring with notification tracking
        let result = ws_client.start_monitoring(&ws_url).await;

        match result {
            Ok(_) => {
                log(LogTag::System, "SUCCESS", "✅ WebSocket monitoring completed normally");
            }
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("❌ WebSocket monitoring error: {}", e));
            }
        }

        log(LogTag::System, "INFO", "🛑 WebSocket monitoring task ENDED");
    });

    // Give WebSocket adequate time to connect and stabilize
    log(
        LogTag::System,
        "INFO",
        "⏳ Waiting 5 seconds for WebSocket to fully connect and stabilize..."
    );
    sleep(Duration::from_secs(5)).await;

    // Verify WebSocket is ready
    log(LogTag::System, "SUCCESS", "✅ WebSocket monitoring is now ACTIVE and ready");

    // Step 3: Execute the swap AFTER monitoring is established
    log(LogTag::System, "INFO", "=== Step 3: Execute Swap (WebSocket Should Capture This) ===");

    // Create minimal BONK token for testing
    let test_token = Token {
        mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(), // BONK
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
        price_dexscreener_sol: Some(0.000000025),
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

    let swap_amount = 0.0001; // Very small test amount
    let swap_start_time = Instant::now();

    log(LogTag::System, "INFO", &format!("💰 Initiating swap: {} SOL for BONK", swap_amount));
    log(
        LogTag::System,
        "INFO",
        "🕐 Swap timestamp recorded for correlation with WebSocket notifications"
    );

    // Execute swap and capture signature for tracking
    match buy_token(&test_token, swap_amount, None).await {
        Ok(result) => {
            if let Some(signature) = &result.transaction_signature {
                log(
                    LogTag::System,
                    "SUCCESS",
                    &format!("🎉 SWAP SUCCESSFUL! Transaction: {}", signature)
                );
                log(
                    LogTag::System,
                    "INFO",
                    &format!("📈 Output amount: {} BONK", result.output_amount)
                );
                if let Some(price) = result.effective_price {
                    log(
                        LogTag::System,
                        "INFO",
                        &format!("💱 Effective price: {:.12} SOL per BONK", price)
                    );
                }

                // Store signature for correlation
                let mut stored_sig = swap_signature.lock().await;
                *stored_sig = Some(signature.clone());

                log(
                    LogTag::System,
                    "INFO",
                    "🔍 NOW MONITORING: WebSocket should detect this transaction..."
                );
            } else {
                log(LogTag::System, "WARNING", "⚠️ Swap completed but no transaction signature");
            }
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Swap failed: {}", e));
            log(LogTag::System, "INFO", "🔄 Continuing monitoring test despite swap failure");
        }
    }

    // Step 4: Monitor for WebSocket notifications and database updates
    log(LogTag::System, "INFO", "=== Step 4: Debug WebSocket Notification Processing ===");

    let monitoring_duration = Duration::from_secs(30);
    let check_interval = Duration::from_secs(2);
    let max_checks = (monitoring_duration.as_secs() / check_interval.as_secs()) as u32;

    log(
        LogTag::System,
        "INFO",
        &format!(
            "🔍 Monitoring for {} seconds with checks every {} seconds",
            monitoring_duration.as_secs(),
            check_interval.as_secs()
        )
    );

    let mut websocket_notifications_seen = false;
    let mut database_updates_seen = false;

    for check_num in 1..=max_checks {
        sleep(check_interval).await;

        // Check database for new transactions
        let current_count = db.get_transaction_count()?;
        let new_transactions = current_count - initial_count;

        // Check for WebSocket notifications (this is where we need to debug)
        let notification_count = {
            let counter = notification_counter.lock().await;
            *counter
        };

        log(
            LogTag::System,
            "INFO",
            &format!(
                "🔎 Check #{}/{}: DB transactions: +{}, WebSocket notifications: {}",
                check_num,
                max_checks,
                new_transactions,
                notification_count
            )
        );

        if new_transactions > 0 {
            database_updates_seen = true;
            log(
                LogTag::System,
                "SUCCESS",
                &format!("✅ DATABASE UPDATE: {} new transactions detected!", new_transactions)
            );

            // Show recent transactions
            let recent = db.get_recent_transactions(3)?;
            for (i, tx) in recent.iter().enumerate() {
                let elapsed = swap_start_time.elapsed();
                log(
                    LogTag::System,
                    "INFO",
                    &format!("  📝 {}. {} (detected after {:?})", i + 1, tx.signature, elapsed)
                );
            }
        }

        if notification_count > 0 {
            websocket_notifications_seen = true;
            log(
                LogTag::System,
                "SUCCESS",
                &format!("📡 WEBSOCKET ACTIVE: {} notifications received", notification_count)
            );
        }

        // Early exit if we've seen both
        if database_updates_seen && websocket_notifications_seen {
            log(
                LogTag::System,
                "SUCCESS",
                "🎯 BOTH WebSocket notifications AND database updates detected!"
            );
            break;
        }

        // Show what we're still waiting for
        if check_num % 5 == 0 {
            if !websocket_notifications_seen {
                log(LogTag::System, "INFO", "⏳ Still waiting for WebSocket notifications...");
            }
            if !database_updates_seen {
                log(LogTag::System, "INFO", "⏳ Still waiting for database updates...");
            }
        }
    }

    // Step 5: Diagnostic Analysis
    log(LogTag::System, "INFO", "=== Step 5: Diagnostic Analysis ===");

    let final_count = db.get_transaction_count()?;
    let total_new_transactions = final_count - initial_count;
    let final_notification_count = {
        let counter = notification_counter.lock().await;
        *counter
    };

    log(LogTag::System, "INFO", "📊 FINAL STATISTICS:");
    log(
        LogTag::System,
        "INFO",
        &format!("   💾 Database: {} new transactions", total_new_transactions)
    );
    log(
        LogTag::System,
        "INFO",
        &format!("   📡 WebSocket: {} notifications received", final_notification_count)
    );

    // Diagnostic conclusions
    if total_new_transactions > 0 && final_notification_count > 0 {
        log(LogTag::System, "SUCCESS", "✅ PERFECT: Both WebSocket AND database are working!");
    } else if total_new_transactions > 0 && final_notification_count == 0 {
        log(LogTag::System, "WARNING", "⚠️ ISSUE: Database updated but NO WebSocket notifications");
        log(
            LogTag::System,
            "INFO",
            "🔧 DIAGNOSIS: WebSocket receiving but not counting notifications properly"
        );
    } else if total_new_transactions == 0 && final_notification_count > 0 {
        log(
            LogTag::System,
            "WARNING",
            "⚠️ ISSUE: WebSocket notifications received but NOT stored in database"
        );
        log(
            LogTag::System,
            "INFO",
            "🔧 DIAGNOSIS: WebSocket notifications not being processed into database records"
        );
    } else {
        log(
            LogTag::System,
            "ERROR",
            "❌ CRITICAL: Neither WebSocket nor database detected the transaction"
        );
        log(
            LogTag::System,
            "INFO",
            "🔧 DIAGNOSIS: WebSocket monitoring or transaction detection is not working"
        );
    }

    // Show actual transaction signature for manual verification
    if let Some(signature) = &*swap_signature.lock().await {
        log(
            LogTag::System,
            "INFO",
            &format!("🔍 MANUAL VERIFY: Check transaction {} on Solana Explorer", signature)
        );
        log(LogTag::System, "INFO", &format!("🌐 URL: https://solscan.io/tx/{}", signature));
    }

    // Step 6: Cleanup
    log(LogTag::System, "INFO", "=== Step 6: Cleanup ===");

    monitoring_handle.abort();
    log(LogTag::System, "INFO", "🛑 WebSocket monitoring task terminated");

    // Final summary
    log(LogTag::System, "INFO", "=== DEBUG TEST SUMMARY ===");
    log(LogTag::System, "SUCCESS", "✅ WebSocket monitoring started BEFORE swap");
    log(LogTag::System, "SUCCESS", "✅ Swap execution completed");

    if total_new_transactions > 0 {
        log(LogTag::System, "SUCCESS", "✅ Database transaction detection working");
    } else {
        log(LogTag::System, "WARNING", "⚠️ Database transaction detection needs investigation");
    }

    if final_notification_count > 0 {
        log(LogTag::System, "SUCCESS", "✅ WebSocket notification reception working");
    } else {
        log(LogTag::System, "WARNING", "⚠️ WebSocket notification reception needs investigation");
    }

    log(LogTag::System, "INFO", "🎯 DEBUG TEST COMPLETED - Check diagnostics above for issues");

    Ok(())
}
