/// Debug timestamp analysis tool
/// Analyzes the timestamp parsing issue in token data

use screenerbot::{ tokens::{ get_all_tokens_by_liquidity } };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ScreenerBot Timestamp Debug Tool ===");

    // Get all tokens from database
    let api_tokens = get_all_tokens_by_liquidity().await?;

    println!("Found {} tokens in database", api_tokens.len());

    // Analyze timestamps
    for (i, token) in api_tokens.iter().take(5).enumerate() {
        println!("\n--- Token {}: {} ---", i + 1, token.symbol);
        println!("  Raw pair_created_at: {:?}", token.pair_created_at);

        if let Some(ts) = token.pair_created_at {
            println!("  Raw timestamp: {}", ts);

            // Try as seconds
            if let Some(dt_seconds) = chrono::DateTime::from_timestamp(ts, 0) {
                println!("  As seconds: {}", dt_seconds);
            } else {
                println!("  As seconds: INVALID");
            }

            // Try as milliseconds
            if let Some(dt_millis) = chrono::DateTime::from_timestamp_millis(ts) {
                println!("  As milliseconds: {}", dt_millis);
            } else {
                println!("  As milliseconds: INVALID");
            }

            // Try as microseconds
            if let Some(dt_micros) = chrono::DateTime::from_timestamp_micros(ts) {
                println!("  As microseconds: {}", dt_micros);
            } else {
                println!("  As microseconds: INVALID");
            }

            // Try as nanoseconds (don't use from_timestamp_nanos for large values)
            if ts < 10_000_000_000 {
                // reasonable seconds timestamp
                println!("  As nanoseconds: Would be a reasonable seconds timestamp");
            } else {
                println!("  As nanoseconds: Value too large for reasonable timestamp");
            }

            // Show current time for comparison
            let now = chrono::Utc::now();
            println!("  Current time: {}", now);
            println!("  Current timestamp: {}", now.timestamp());
        }
    }

    println!("\n=== ANALYSIS COMPLETE ===");

    Ok(())
}
