// bin/test_modular_transactions.rs - Test the new modular transaction system
use screenerbot::transactions::*;
use screenerbot::global::read_configs;
use screenerbot::logger::{ log, LogTag };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "Testing new modular transaction system...");

    // Load configuration
    let configs = read_configs("configs.json")?;
    let wallet_address = {
        use screenerbot::wallet::get_wallet_address;
        get_wallet_address().map_err(
            |e|
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<
                    dyn std::error::Error
                >
        )?
    };

    // Test 1: Database operations
    log(LogTag::System, "INFO", "=== Testing Database Operations ===");
    let db = TransactionDatabase::new()?;

    let transaction_count = db.get_transaction_count()?;
    log(LogTag::System, "INFO", &format!("Current transaction count: {}", transaction_count));

    // Test 2: Transaction fetcher
    log(LogTag::System, "INFO", "=== Testing Transaction Fetcher ===");
    let fetcher = TransactionFetcher::new(configs.clone(), None)?;

    log(LogTag::System, "INFO", "Fetching recent signatures...");
    let signatures = fetcher
        .get_recent_signatures(&wallet_address, 10).await
        .map_err(
            |e|
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<
                    dyn std::error::Error
                >
        )?;
    log(LogTag::System, "SUCCESS", &format!("Retrieved {} signatures", signatures.len()));

    if !signatures.is_empty() {
        log(LogTag::System, "INFO", "Testing batch fetch...");
        let transactions = fetcher.batch_fetch_transactions(
            &signatures[..(5).min(signatures.len())],
            None
        ).await?;
        log(
            LogTag::System,
            "SUCCESS",
            &format!("Batch fetched {} transactions", transactions.len())
        );

        // Test 3: Transaction analyzer
        log(LogTag::System, "INFO", "=== Testing Transaction Analyzer ===");
        let analyzer = TransactionAnalyzer::new();

        for (sig_info, transaction) in &transactions {
            let analysis = analyzer.analyze_transaction(&transaction);
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "Transaction {}: {:?}",
                    &sig_info.signature[..8],
                    analysis.transaction_type
                )
            );

            if analysis.is_swap {
                if let Some(swap_info) = &analysis.swap_info {
                    log(
                        LogTag::System,
                        "SUCCESS",
                        &format!(
                            "Swap detected: {} on {}",
                            &sig_info.signature[..8],
                            swap_info.dex_name
                        )
                    );
                }
            }
        }

        // Generate transaction statistics
        let stats = analyzer.get_transaction_stats(&transactions);
        log(LogTag::System, "INFO", &format!("Transaction Stats:"));
        log(LogTag::System, "INFO", &format!("  Total: {}", stats.total));
        log(
            LogTag::System,
            "INFO",
            &format!("  Swaps: {} ({:.1}%)", stats.swaps, stats.swap_percentage)
        );
        log(LogTag::System, "INFO", &format!("  Airdrops: {}", stats.airdrops));
        log(LogTag::System, "INFO", &format!("  Transfers: {}", stats.transfers));
        if let Some(most_used_dex) = &stats.most_used_dex {
            log(LogTag::System, "INFO", &format!("  Most used DEX: {}", most_used_dex));
        }
    }

    // Test 4: Sync status
    log(LogTag::System, "INFO", "=== Testing Sync Status ===");
    match db.get_sync_status(&wallet_address)? {
        Some(sync_status) => {
            log(
                LogTag::System,
                "INFO",
                &format!(
                    "Last sync: {} (slot: {})",
                    sync_status.last_sync_time.format("%Y-%m-%d %H:%M:%S"),
                    sync_status.last_sync_slot
                )
            );
            log(
                LogTag::System,
                "INFO",
                &format!("Total transactions: {}", sync_status.total_transactions)
            );
        }
        None => {
            log(LogTag::System, "INFO", "No sync status found - this is a fresh installation");
        }
    }

    // Test 5: Cache performance
    log(LogTag::System, "INFO", "=== Testing Cache Performance ===");
    if !signatures.is_empty() {
        let test_signature = &signatures[0].signature;

        // Time cache lookup
        let start = std::time::Instant::now();
        let cached_result = db.get_transaction(test_signature)?;
        let cache_time = start.elapsed();

        match cached_result {
            Some(_) => {
                log(LogTag::System, "SUCCESS", &format!("Cache hit in {:?}", cache_time));
            }
            None => {
                log(LogTag::System, "INFO", "Cache miss - transaction not in database");
            }
        }
    }

    // Test 6: Legacy compatibility
    log(LogTag::System, "INFO", "=== Testing Legacy Compatibility ===");
    let client = reqwest::Client::new();
    let legacy_signatures = get_recent_signatures_with_fallback(
        &client,
        &wallet_address,
        &configs,
        5
    ).await.map_err(
        |e|
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<
                dyn std::error::Error
            >
    )?;
    log(
        LogTag::System,
        "SUCCESS",
        &format!("Legacy fetcher returned {} signatures", legacy_signatures.len())
    );

    let legacy_transactions = get_transactions_with_cache_and_fallback(
        &client,
        &legacy_signatures[..(3).min(legacy_signatures.len())],
        &configs,
        None
    ).await;
    log(
        LogTag::System,
        "SUCCESS",
        &format!("Legacy batch fetcher returned {} transactions", legacy_transactions.len())
    );

    // Test 7: DEX recognition
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

    log(LogTag::System, "SUCCESS", "All modular transaction system tests completed!");
    Ok(())
}
