// Test for token discovery API calls and database caching
use screenerbot::global::*;
use screenerbot::discovery::*;
use screenerbot::logger::{ log, LogTag };
use screenerbot::token_cache::TokenDatabaseStats;
use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "Starting Token Discovery & Caching Test");

    // Initialize the token database
    initialize_token_database()?;

    // Get initial database stats
    let initial_stats = get_db_stats().await?;
    log(
        LogTag::System,
        "INFO",
        &format!(
            "Initial DB stats - Total tokens: {}, Unique sources: {}, Accesses: {}",
            initial_stats.total_tokens,
            initial_stats.unique_sources,
            initial_stats.total_accesses
        )
    );

    // Test 1: API Discovery Functions
    log(LogTag::System, "TEST", "Testing API discovery functions...");

    let start_time = std::time::Instant::now();

    // Test DexScreener token profiles discovery
    log(LogTag::System, "API", "Calling discovery_dexscreener_fetch_token_profiles()...");
    match discovery_dexscreener_fetch_token_profiles().await {
        Ok(()) => log(LogTag::System, "SUCCESS", "Token profiles discovery completed"),
        Err(e) => log(LogTag::System, "ERROR", &format!("Token profiles discovery failed: {}", e)),
    }

    // Test DexScreener token boosts discovery
    log(LogTag::System, "API", "Calling discovery_dexscreener_fetch_token_boosts()...");
    match discovery_dexscreener_fetch_token_boosts().await {
        Ok(()) => log(LogTag::System, "SUCCESS", "Token boosts discovery completed"),
        Err(e) => log(LogTag::System, "ERROR", &format!("Token boosts discovery failed: {}", e)),
    }

    // Test DexScreener top token boosts discovery
    log(LogTag::System, "API", "Calling discovery_dexscreener_fetch_token_boosts_top()...");
    match discovery_dexscreener_fetch_token_boosts_top().await {
        Ok(()) => log(LogTag::System, "SUCCESS", "Top token boosts discovery completed"),
        Err(e) =>
            log(LogTag::System, "ERROR", &format!("Top token boosts discovery failed: {}", e)),
    }

    let discovery_duration = start_time.elapsed();
    log(
        LogTag::System,
        "PERF",
        &format!("API discovery completed in {:.2}s", discovery_duration.as_secs_f64())
    );

    // Test 2: Check LIST_MINTS population
    let mints_count = {
        let mints = LIST_MINTS.read().unwrap();
        mints.len()
    };
    log(LogTag::System, "TEST", &format!("LIST_MINTS contains {} mint addresses", mints_count));

    if mints_count == 0 {
        log(
            LogTag::System,
            "WARN",
            "No mints discovered - API might have failed or returned empty results"
        );
        return Ok(());
    }

    // Test 3: Token info update from mints (this triggers caching)
    log(LogTag::System, "TEST", "Testing token info update from mints (triggers caching)...");
    let shutdown = Arc::new(Notify::new());

    let update_start = std::time::Instant::now();
    match update_tokens_from_mints(shutdown).await {
        Ok(()) => {
            let update_duration = update_start.elapsed();
            log(
                LogTag::System,
                "SUCCESS",
                &format!("Token update completed in {:.2}s", update_duration.as_secs_f64())
            );
        }
        Err(e) => log(LogTag::System, "ERROR", &format!("Token update failed: {}", e)),
    }

    // Test 4: Verify tokens are in LIST_TOKENS
    let tokens_count = {
        let tokens = LIST_TOKENS.read().unwrap();
        tokens.len()
    };
    log(
        LogTag::System,
        "TEST",
        &format!("LIST_TOKENS contains {} tokens after update", tokens_count)
    );

    // Test 5: Verify database caching
    let final_stats = get_db_stats().await?;
    log(
        LogTag::System,
        "TEST",
        &format!(
            "Final DB stats - Total tokens: {}, Unique sources: {}, Accesses: {}",
            final_stats.total_tokens,
            final_stats.unique_sources,
            final_stats.total_accesses
        )
    );

    let tokens_cached = final_stats.total_tokens - initial_stats.total_tokens;
    log(
        LogTag::System,
        "SUCCESS",
        &format!("✅ {} new tokens were cached to database", tokens_cached)
    );

    // Test 6: Database search functionality
    log(LogTag::System, "TEST", "Testing database search functionality...");
    test_database_search().await?;

    // Test 7: Token retrieval functionality
    log(LogTag::System, "TEST", "Testing token retrieval functionality...");
    test_token_retrieval().await?;

    // Test 8: Performance metrics
    log(LogTag::System, "PERF", "=== PERFORMANCE SUMMARY ===");
    log(
        LogTag::System,
        "PERF",
        &format!("Discovery Duration: {:.2}s", discovery_duration.as_secs_f64())
    );
    log(LogTag::System, "PERF", &format!("Mints Discovered: {}", mints_count));
    log(LogTag::System, "PERF", &format!("Tokens Processed: {}", tokens_count));
    log(LogTag::System, "PERF", &format!("Tokens Cached: {}", tokens_cached));

    if tokens_count > 0 {
        let tokens_per_second = (tokens_count as f64) / discovery_duration.as_secs_f64();
        log(
            LogTag::System,
            "PERF",
            &format!("Processing Rate: {:.1} tokens/second", tokens_per_second)
        );
    }

    log(LogTag::System, "SUCCESS", "🎉 Token Discovery & Caching Test Completed Successfully!");

    Ok(())
}

async fn get_db_stats() -> Result<TokenDatabaseStats, Box<dyn std::error::Error>> {
    if let Ok(token_db_guard) = TOKEN_DB.lock() {
        if let Some(ref db) = *token_db_guard {
            return db.get_stats();
        }
    }
    Err("Token database not initialized".into())
}

async fn test_database_search() -> Result<(), Box<dyn std::error::Error>> {
    if let Ok(token_db_guard) = TOKEN_DB.lock() {
        if let Some(ref db) = *token_db_guard {
            // Test search for common terms
            let search_terms = vec!["USD", "SOL", "Token", "Coin"];

            for term in search_terms {
                let results = db.search_tokens(term)?;
                log(
                    LogTag::System,
                    "SEARCH",
                    &format!("Search '{}' found {} results", term, results.len())
                );

                // Show first few results
                for (i, token) in results.iter().take(3).enumerate() {
                    log(
                        LogTag::System,
                        "RESULT",
                        &format!("  {}. {} ({}) - {}", i + 1, token.symbol, token.name, token.mint)
                    );
                }
            }
        }
    }
    Ok(())
}

async fn test_token_retrieval() -> Result<(), Box<dyn std::error::Error>> {
    // Get some tokens from LIST_TOKENS to test retrieval
    let test_mints = {
        let tokens = LIST_TOKENS.read().unwrap();
        tokens
            .iter()
            .take(5)
            .map(|t| t.mint.clone())
            .collect::<Vec<String>>()
    };

    log(
        LogTag::System,
        "TEST",
        &format!("Testing retrieval of {} tokens from database", test_mints.len())
    );

    for mint in test_mints {
        match get_token_from_db(&mint) {
            Some(token) => {
                log(
                    LogTag::System,
                    "RETRIEVE",
                    &format!("✅ Retrieved {} ({}) from DB", token.symbol, token.name)
                );

                // Test token data integrity
                if token.mint == mint {
                    log(LogTag::System, "VERIFY", &format!("✅ Mint address matches: {}", mint));
                } else {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("❌ Mint mismatch! Expected: {}, Got: {}", mint, token.mint)
                    );
                }
            }
            None => {
                log(LogTag::System, "ERROR", &format!("❌ Failed to retrieve token: {}", mint));
            }
        }
    }

    Ok(())
}
