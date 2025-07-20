// Test token database caching functionality
use screenerbot::global::*;
use screenerbot::logger::{ log, LogTag };
use screenerbot::discovery::update_tokens_from_mints;
use std::sync::Arc;
use tokio::sync::Notify;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "Testing token database caching");

    // Initialize token database
    initialize_token_database()?;

    // Add some test mints to LIST_MINTS
    {
        let mut mints = LIST_MINTS.write().unwrap();
        mints.insert("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v".to_string()); // USDC
        mints.insert("So11111111111111111111111111111111111111112".to_string()); // SOL
        mints.insert("Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB".to_string()); // USDT
    }

    log(LogTag::System, "INFO", "Added test mints, now discovering tokens...");

    // Create shutdown signal for the discovery process
    let shutdown = Arc::new(Notify::new());

    // Run token discovery which should cache tokens to database
    match update_tokens_from_mints(shutdown).await {
        Ok(_) => log(LogTag::System, "SUCCESS", "Token discovery completed"),
        Err(e) => log(LogTag::System, "ERROR", &format!("Token discovery failed: {}", e)),
    }

    // Check how many tokens are in LIST_TOKENS
    let live_tokens_count = { LIST_TOKENS.read().unwrap().len() };

    log(LogTag::System, "INFO", &format!("Live tokens in LIST_TOKENS: {}", live_tokens_count));

    // Check database stats
    if let Ok(token_db_guard) = TOKEN_DB.lock() {
        if let Some(ref db) = *token_db_guard {
            match db.get_stats() {
                Ok(stats) => {
                    log(LogTag::System, "STATS", &format!("{}", stats));

                    // Test searching for tokens
                    match db.search_tokens("USD") {
                        Ok(search_results) => {
                            log(
                                LogTag::System,
                                "SEARCH",
                                &format!("Found {} tokens matching 'USD'", search_results.len())
                            );
                            for token in search_results.iter().take(3) {
                                log(
                                    LogTag::System,
                                    "FOUND",
                                    &format!("  {} ({}) - {}", token.symbol, token.name, token.mint)
                                );
                            }
                        }
                        Err(e) => log(LogTag::System, "ERROR", &format!("Search failed: {}", e)),
                    }

                    // Test getting tokens since startup
                    let startup_time = *STARTUP_TIME;
                    match db.get_tokens_since(startup_time) {
                        Ok(new_tokens) => {
                            log(
                                LogTag::System,
                                "NEW_TOKENS",
                                &format!(
                                    "Found {} tokens discovered since startup",
                                    new_tokens.len()
                                )
                            );
                        }
                        Err(e) =>
                            log(
                                LogTag::System,
                                "ERROR",
                                &format!("Failed to get new tokens: {}", e)
                            ),
                    }
                }
                Err(e) => log(LogTag::System, "ERROR", &format!("Failed to get stats: {}", e)),
            }
        }
    }

    // Test direct token retrieval
    let test_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
    match get_token_from_db(test_mint) {
        Some(token) => {
            log(
                LogTag::System,
                "RETRIEVE",
                &format!("Successfully retrieved {} from database", token.symbol)
            );
        }
        None => {
            log(LogTag::System, "WARN", "Failed to retrieve test token from database");
        }
    }

    log(LogTag::System, "SUCCESS", "Token database caching test completed");
    Ok(())
}
