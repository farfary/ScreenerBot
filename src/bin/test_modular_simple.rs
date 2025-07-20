// bin/test_modular_simple.rs - Simple test for the new modular transaction system
use screenerbot::transactions::*;
use screenerbot::global::read_configs;
use screenerbot::wallet::get_wallet_address;
use screenerbot::logger::{ log, LogTag };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "Testing new modular transaction system (simple version)...");

    // Get wallet address
    let wallet_address = get_wallet_address().map_err(|e|
        format!("Failed to get wallet address: {}", e)
    )?;
    log(LogTag::System, "INFO", &format!("Using wallet address: {}", wallet_address));

    // Load configuration
    let configs = read_configs("configs.json")?;

    // Test 1: DEX recognition
    log(LogTag::System, "INFO", "=== Testing DEX Recognition ===");
    for (program_id, dex_name) in DEX_PROGRAM_IDS.iter().take(5) {
        let recognized = is_known_dex(program_id);
        let name = get_dex_name(program_id);
        log(
            LogTag::System,
            "INFO",
            &format!(
                "Program {}: {} -> {}",
                &program_id[..8],
                recognized,
                name.unwrap_or("Unknown")
            )
        );
    }

    // Test 2: Legacy compatibility functions
    log(LogTag::System, "INFO", "=== Testing Legacy Functions ===");
    let client = reqwest::Client::new();

    log(LogTag::System, "INFO", "Testing signature fetching...");
    match get_recent_signatures_with_fallback(&client, &wallet_address, &configs, 5).await {
        Ok(signatures) => {
            log(
                LogTag::System,
                "SUCCESS",
                &format!("Legacy fetcher returned {} signatures", signatures.len())
            );

            if !signatures.is_empty() {
                log(LogTag::System, "INFO", "Testing batch transaction fetching...");
                let transactions = get_transactions_with_cache_and_fallback(
                    &client,
                    &signatures[..(3).min(signatures.len())],
                    &configs,
                    None
                ).await;
                log(
                    LogTag::System,
                    "SUCCESS",
                    &format!("Legacy batch fetcher returned {} transactions", transactions.len())
                );

                // Test 3: Transaction analysis
                log(LogTag::System, "INFO", "=== Testing Transaction Analysis ===");
                for (sig_info, transaction) in &transactions {
                    let is_swap = is_swap_transaction(&transaction);
                    log(
                        LogTag::System,
                        "INFO",
                        &format!("Transaction {}: is_swap={}", &sig_info.signature[..8], is_swap)
                    );
                }
            }
        }
        Err(e) => {
            log(LogTag::System, "WARNING", &format!("Signature fetching failed: {}", e));
        }
    }

    // Test 4: Database initialization (just check if it can be created)
    log(LogTag::System, "INFO", "=== Testing Database Initialization ===");
    match TransactionDatabase::new() {
        Ok(db) => {
            match db.get_transaction_count() {
                Ok(count) => {
                    log(
                        LogTag::System,
                        "SUCCESS",
                        &format!("Database initialized with {} transactions", count)
                    );
                }
                Err(e) => {
                    log(LogTag::System, "WARNING", &format!("Database count failed: {}", e));
                }
            }
        }
        Err(e) => {
            log(LogTag::System, "WARNING", &format!("Database initialization failed: {}", e));
        }
    }

    log(LogTag::System, "SUCCESS", "Simple modular transaction system test completed!");
    Ok(())
}
