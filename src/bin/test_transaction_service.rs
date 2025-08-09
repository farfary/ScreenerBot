use screenerbot::logger::{log, LogTag, init_file_logging};
use screenerbot::swaps::transaction::{TransactionMonitoringService, Transaction};
use screenerbot::tokens;
use screenerbot::wallet_tracker;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tokio::sync::Notify;
use std::collections::HashMap;
use std::path::Path;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();
    println!("[TEST] Starting minimal transaction service test...");

    // Initialize tokens system
    println!("[TEST] Initializing tokens system...");
    match tokens::initialize_tokens_system().await {
        Ok(_) => println!("[TEST] ✅ Tokens system initialized"),
        Err(e) => {
            eprintln!("[TEST] ❌ Failed to initialize tokens system: {}", e);
            return Ok(());
        }
    }

    // Initialize price service
    println!("[TEST] Initializing price service...");
    match tokens::initialize_price_service().await {
        Ok(_) => println!("[TEST] ✅ Price service initialized"),
        Err(e) => {
            eprintln!("[TEST] ❌ Failed to initialize price service: {}", e);
            return Ok(());
        }
    }

    // Initialize pool service
    println!("[TEST] Initializing pool service...");
    let pool_service = tokens::pool::init_pool_service();
    pool_service.start_monitoring().await;
    println!("[TEST] ✅ Pool service initialized");

    // Initialize wallet tracker
    println!("[TEST] Initializing wallet tracker...");
    match wallet_tracker::init_wallet_tracker().await {
        Ok(_) => println!("[TEST] ✅ Wallet tracker initialized"),
        Err(e) => {
            eprintln!("[TEST] ❌ Failed to initialize wallet tracker: {}", e);
            return Ok(());
        }
    }

    // Initialize transaction monitoring service
    println!("[TEST] Initializing transaction monitoring service...");
    match TransactionMonitoringService::init_global_service().await {
        Ok(_) => println!("[TEST] ✅ Transaction monitoring service initialized"),
        Err(e) => {
            eprintln!("[TEST] ❌ Failed to initialize transaction service: {}", e);
            return Ok(());
        }
    }

    // Check for pending transactions file
    let pending_file = "data/pending_transactions.json";
    println!("[TEST] Checking for pending transactions file: {}", pending_file);
    
    if !Path::new(pending_file).exists() {
        println!("[TEST] ⏭️ No pending transactions file found");
        println!("[TEST] 💡 Run the bot to generate some transactions, then test again");
        return Ok(());
    }

    // Read and parse the file manually to see what's there
    match std::fs::read_to_string(pending_file) {
        Ok(content) => {
            println!("[TEST] 📄 Pending transactions file content (first 200 chars):");
            println!("[TEST] {}", &content[0..content.len().min(200)]);
            
            match serde_json::from_str::<Vec<Transaction>>(&content) {
                Ok(transactions) => {
                    println!("[TEST] 📊 Found {} pending transactions", transactions.len());
                    
                    if transactions.is_empty() {
                        println!("[TEST] ⏭️ No pending transactions to test");
                        return Ok(());
                    }

                    // Display information about the first transaction
                    let tx = &transactions[0];
                    let age = chrono::Utc::now().timestamp() - tx.created_at.timestamp();
                    println!("[TEST] 📝 First transaction: {} ({}...)", 
                            &tx.signature[0..8], &tx.signature[tx.signature.len()-8..]);
                    println!("[TEST]   - Token mint: {}", tx.mint);
                    println!("[TEST]   - Direction: {}", tx.direction);
                    println!("[TEST]   - Input mint: {}", tx.input_mint);
                    println!("[TEST]   - Output mint: {}", tx.output_mint);
                    println!("[TEST]   - State: {:?}", tx.state);
                    println!("[TEST]   - Position related: {}", tx.position_related);
                    println!("[TEST]   - Age: {} seconds", age);
                    
                    // Test transaction status check
                    println!("[TEST] 🔄 Testing transaction status check...");
                    match TransactionMonitoringService::get_transaction_status(&tx.signature).await {
                        Some(status) => {
                            println!("[TEST] ✅ Transaction status: {:?}", status);
                        }
                        None => {
                            println!("[TEST] ❌ No status found for transaction");
                        }
                    }
                    
                    // Test if transaction is complete
                    println!("[TEST] 🔄 Testing completion check...");
                    let is_complete = TransactionMonitoringService::is_transaction_complete(&tx.signature).await;
                    println!("[TEST] ✅ Transaction complete: {}", is_complete);
                    
                    // Test verification function directly (need wallet address, try to get it)
                    println!("[TEST] 🔄 Testing direct verification...");
                    if let Ok(wallet_address) = screenerbot::swaps::transaction::get_wallet_address() {
                        // Try with a reasonable expected SOL amount (0.01 SOL)
                        match screenerbot::swaps::transaction::verify_position_entry_transaction(
                            &tx.signature, 
                            &tx.mint, 
                            0.01 // Expected SOL spent - just a test value
                        ).await {
                            Ok(result) => {
                                println!("[TEST] ✅ Entry verification result:");
                                println!("[TEST]   - Success: {}", result.success);
                                println!("[TEST]   - SOL spent: {} lamports", result.sol_spent);
                                println!("[TEST]   - Tokens received: {}", result.token_amount_received);
                                println!("[TEST]   - Entry price: {}", result.effective_entry_price);
                                println!("[TEST]   - ATA created: {}", result.ata_created);
                                println!("[TEST]   - Total cost: {} SOL", result.total_cost_sol);
                                if let Some(error) = &result.error {
                                    println!("[TEST]   - Error: {}", error);
                                }
                            }
                            Err(e) => {
                                println!("[TEST] ❌ Entry verification failed: {}", e);
                            }
                        }
                    } else {
                        println!("[TEST] ❌ Could not get wallet address for verification");
                    }
                }
                Err(e) => {
                    eprintln!("[TEST] ❌ Failed to parse transactions: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("[TEST] ❌ Failed to read file: {}", e);
        }
    }

    // Test monitoring service for a short time
    println!("[TEST] 🚀 Testing background monitoring for 15 seconds...");
    
    let shutdown_notify = Arc::new(Notify::new());
    let shutdown_clone = shutdown_notify.clone();
    
    let monitoring_task = tokio::spawn(async move {
        if let Err(e) = TransactionMonitoringService::start_monitoring_service(shutdown_clone).await {
            eprintln!("[TEST] ❌ Monitoring service error: {}", e);
        }
    });

    // Let it run for 15 seconds
    sleep(Duration::from_secs(15)).await;
    
    // Signal shutdown and wait for clean exit
    shutdown_notify.notify_one();
    println!("[TEST] � Signaling shutdown...");
    
    // Give it a moment to shut down gracefully
    sleep(Duration::from_secs(2)).await;
    monitoring_task.abort();

    println!("[TEST] ✅ Transaction service test completed!");
    Ok(())
}
