use screenerbot::tokens::price_service::{
    initialize_price_service,
    get_token_price_safe,
    get_token_price_blocking_safe,
};
use screenerbot::logger::init_file_logging;
use screenerbot::global::set_cmd_args;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();
    set_cmd_args(vec!["--debug-price-service".to_string()]);

    println!("Testing price service for token: GkyPYa7NnCFbduLknCfBfP7p8564X1VZhwZYJ6CZpump");

    // Initialize price service
    initialize_price_service().await?;

    let token_mint = "GkyPYa7NnCFbduLknCfBfP7p8564X1VZhwZYJ6CZpump";

    // Test instant lookup (should return cached or None)
    println!("Testing get_token_price_safe (instant)...");
    let instant_price = get_token_price_safe(token_mint).await;
    println!("Instant price result: {:?}", instant_price);

    // Test blocking lookup (should wait for update)
    println!("Testing get_token_price_blocking_safe (waits for update)...");
    let blocking_price = get_token_price_blocking_safe(token_mint).await;
    println!("Blocking price result: {:?}", blocking_price);

    // Test instant lookup again (should be cached now)
    println!("Testing get_token_price_safe again (should be cached)...");
    let cached_price = get_token_price_safe(token_mint).await;
    println!("Cached price result: {:?}", cached_price);

    Ok(())
}
