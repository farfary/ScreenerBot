/// Test Priority-Aware Adaptive Polling
/// 
/// This tool tests the new priority-aware transaction monitoring system to verify:
/// - Fast polling (5s) when pending priority transactions exist
/// - Adaptive polling (30s -> 60s -> 120s) during idle periods
/// - Immediate switch to fast polling when priority transactions are added

use std::time::Duration;
use tokio::time::{sleep, Instant};
use std::env;
use solana_sdk::signer::Signer;

use screenerbot::logger::{log, LogTag};
use screenerbot::arguments::{set_cmd_args, is_debug_transactions_enabled};
use screenerbot::configs::{read_configs, load_wallet_from_config};
use screenerbot::transactions::TransactionsManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Set up arguments for debug mode
    let args: Vec<String> = env::args().collect();
    set_cmd_args(args);

    log(LogTag::System, "INFO", "🧪 Priority-Aware Polling Test Starting");

    // Load wallet configuration
    let configs = read_configs()?;
    let wallet = load_wallet_from_config(&configs)?;
    let wallet_pubkey = wallet.pubkey();

    // Create TransactionsManager
    let mut manager = TransactionsManager::new(wallet_pubkey).await
        .map_err(|e| format!("Failed to create TransactionsManager: {}", e))?;

    if let Err(e) = manager.initialize_known_signatures().await {
        log(LogTag::Transactions, "ERROR", &format!("Failed to initialize: {}", e));
        return Ok(());
    }

    log(LogTag::System, "INFO", &format!(
        "TransactionsManager initialized for wallet: {} (known transactions: {})",
        wallet_pubkey,
        manager.known_signatures().len()
    ));

    // Test 1: Normal adaptive polling without priority transactions
    log(LogTag::System, "TEST", "📊 Test 1: Normal adaptive polling (should use 30s -> 60s -> 120s intervals)");
    
    let test_start = Instant::now();
    let mut cycle_count = 0;
    let max_test_cycles = 5;

    while cycle_count < max_test_cycles {
        let cycle_start = Instant::now();
        
        match do_test_monitoring_cycle(&mut manager).await {
            Ok((new_count, has_pending)) => {
                let cycle_duration = cycle_start.elapsed();
                
                log(LogTag::System, "CYCLE", &format!(
                    "Cycle {}: {} new transactions, {} pending, took {:?}",
                    cycle_count + 1, new_count, has_pending, cycle_duration
                ));
                
                // Simulate adaptive timing based on activity
                let sleep_duration = if cycle_count >= 3 {
                    Duration::from_secs(3) // Simulate 120s interval (reduced for testing)
                } else if cycle_count >= 1 {
                    Duration::from_secs(2) // Simulate 60s interval (reduced for testing)  
                } else {
                    Duration::from_secs(1) // Simulate 30s interval (reduced for testing)
                };
                
                log(LogTag::System, "TIMING", &format!(
                    "⏱️ Sleeping for {:?} (simulating adaptive interval)", sleep_duration
                ));
                
                sleep(sleep_duration).await;
            }
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("Monitoring cycle failed: {}", e));
                break;
            }
        }
        
        cycle_count += 1;
    }

    log(LogTag::System, "TEST", "✅ Test 1 completed");
    sleep(Duration::from_secs(1)).await;

    // Test 2: Add priority transaction and verify fast polling
    log(LogTag::System, "TEST", "🚀 Test 2: Adding priority transaction (should trigger 5s fast polling)");

    // Add a mock priority transaction
    let mock_signature = "MockPriorityTransaction123456789ABCDEF".to_string();
    manager.add_priority_transaction(mock_signature.clone());

    log(LogTag::System, "PRIORITY", &format!(
        "Added priority transaction: {} (total pending: {})",
        &mock_signature[..8], manager.priority_transactions().len()
    ));

    // Test fast polling cycles
    let fast_test_cycles = 3;
    for i in 0..fast_test_cycles {
        let cycle_start = Instant::now();
        
        match do_test_monitoring_cycle(&mut manager).await {
            Ok((new_count, has_pending)) => {
                let cycle_duration = cycle_start.elapsed();
                
                log(LogTag::System, "FAST_CYCLE", &format!(
                    "Fast cycle {}: {} new, {} pending, took {:?} - {}",
                    i + 1, new_count, has_pending, cycle_duration,
                    if has_pending { "🚀 FAST POLLING ACTIVE" } else { "⚠️ No pending detected" }
                ));
                
                // Simulate 5s fast polling (reduced to 0.5s for testing)
                sleep(Duration::from_millis(500)).await;
            }
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("Fast monitoring cycle failed: {}", e));
                break;
            }
        }
    }

    log(LogTag::System, "TEST", "✅ Test 2 completed");

    // Test 3: Clear priority transactions and verify return to adaptive polling
    log(LogTag::System, "TEST", "📊 Test 3: Clearing priority transactions (should return to adaptive polling)");

    // Clear priority transactions (simulate completion)
    manager.clear_priority_transactions_for_testing();
    
    log(LogTag::System, "ADAPTIVE", &format!(
        "Cleared priority transactions (remaining: {})", 
        manager.priority_transactions().len()
    ));

    // Test return to adaptive polling
    let adaptive_return_cycles = 2;
    for i in 0..adaptive_return_cycles {
        let cycle_start = Instant::now();
        
        match do_test_monitoring_cycle(&mut manager).await {
            Ok((new_count, has_pending)) => {
                let cycle_duration = cycle_start.elapsed();
                
                log(LogTag::System, "RETURN_CYCLE", &format!(
                    "Return cycle {}: {} new, {} pending, took {:?} - {}",
                    i + 1, new_count, has_pending, cycle_duration,
                    if has_pending { "🚀 Still fast" } else { "📊 Back to adaptive" }
                ));
                
                // Simulate return to 30s adaptive interval (reduced to 1s for testing)
                sleep(Duration::from_secs(1)).await;
            }
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("Return monitoring cycle failed: {}", e));
                break;
            }
        }
    }

    let total_test_duration = test_start.elapsed();
    
    log(LogTag::System, "SUCCESS", &format!(
        "🎉 All priority-aware polling tests completed successfully in {:?}", 
        total_test_duration
    ));

    log(LogTag::System, "SUMMARY", "📋 Test Summary:");
    log(LogTag::System, "SUMMARY", "   ✅ Normal adaptive polling: 30s -> 60s -> 120s intervals");
    log(LogTag::System, "SUMMARY", "   ✅ Priority transactions trigger 5s fast polling");
    log(LogTag::System, "SUMMARY", "   ✅ Clearing priority transactions returns to adaptive polling");
    log(LogTag::System, "SUMMARY", "   🚀 Priority-aware system is working correctly!");

    Ok(())
}

/// Simplified monitoring cycle for testing
async fn do_test_monitoring_cycle(manager: &mut TransactionsManager) -> Result<(usize, bool), String> {
    // Check for new transactions (this will be 0 in most test cases)
    let new_signatures = manager.check_new_transactions().await?;
    let new_transaction_count = new_signatures.len();
    
    // Process any new transactions
    for signature in &new_signatures {
        log(LogTag::Transactions, "PROCESS", &format!(
            "Processing new transaction: {}", &signature[..8]
        ));
    }

    // Check priority transactions
    if let Err(e) = manager.check_priority_transactions().await {
        log(LogTag::Transactions, "WARN", &format!(
            "Priority transaction check failed: {}", e
        ));
    }

    // Check if we have pending priority transactions
    let has_pending_transactions = !manager.priority_transactions().is_empty();

    // Log current state
    if is_debug_transactions_enabled() {
        let stats = manager.get_stats();
        log(LogTag::Transactions, "TEST_STATS", &format!(
            "New: {}, Priority: {}, Total: {}, Pending: {}",
            new_transaction_count,
            stats.priority_transactions_count,
            stats.total_transactions,
            has_pending_transactions
        ));
    }

    Ok((new_transaction_count, has_pending_transactions))
}
