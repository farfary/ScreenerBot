use screenerbot::tokens::decimals::{
    get_token_decimals_from_chain,
    get_cache_stats,
    clear_decimals_cache,
    save_decimal_cache,
};
use screenerbot::logger::{ log, LogTag };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Testing decimal cache functionality...\n");

    // Show initial cache stats
    let (size, capacity) = get_cache_stats();
    println!("Initial cache: {} entries (capacity: {})", size, capacity);

    // Test fetching some known token decimals
    let test_tokens = vec![
        "So11111111111111111111111111111111111111112", // WSOL - 9 decimals
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v" // USDC - 6 decimals
    ];

    println!("\nFetching decimals for test tokens...");
    for token in &test_tokens {
        match get_token_decimals_from_chain(token).await {
            Ok(decimals) => {
                log(LogTag::System, "TEST", &format!("Token {} has {} decimals", token, decimals));
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed to get decimals for {}: {}", token, e)
                );
            }
        }
    }

    // Show cache stats after fetching
    let (size, capacity) = get_cache_stats();
    println!("\nCache after fetching: {} entries (capacity: {})", size, capacity);

    // Force save cache
    save_decimal_cache();
    println!("\nForced cache save to disk completed");

    // Test fetching the same tokens again (should be from cache)
    println!("\nFetching same tokens again (should be from cache)...");
    for token in &test_tokens {
        match get_token_decimals_from_chain(token).await {
            Ok(decimals) => {
                log(
                    LogTag::System,
                    "TEST",
                    &format!("Token {} has {} decimals (cached)", token, decimals)
                );
            }
            Err(e) => {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed to get decimals for {}: {}", token, e)
                );
            }
        }
    }

    println!("\nTest completed. Check for 'decimal_cache.json' file in the project root.");

    Ok(())
}
