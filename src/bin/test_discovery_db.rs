use screenerbot::discovery::database::DiscoveryDatabase;
use anyhow::Result;

fn main() -> Result<()> {
    println!("Testing Discovery Database...");

    // Create a new discovery database
    let db = DiscoveryDatabase::new()?;

    // Test saving tokens
    let test_tokens = vec![
        "So11111111111111111111111111111111111111112",
        "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R"
    ];

    for token in &test_tokens {
        let was_new = db.save_token(token)?;
        println!("Token {}: {}", token, if was_new { "NEW" } else { "EXISTS" });
    }

    // Test getting all tokens
    let all_tokens = db.get_all_tokens()?;
    println!("Total tokens in database: {}", all_tokens.len());

    // Test getting stats
    let stats = db.get_stats()?;
    println!("Discovery Stats:");
    println!("  Total tokens discovered: {}", stats.total_tokens_discovered);
    println!("  Active tokens: {}", stats.active_tokens);
    println!("  Discovery rate per hour: {:.2}", stats.discovery_rate_per_hour);

    // Test checking if token exists
    if db.token_exists("So11111111111111111111111111111111111111112")? {
        println!("SOL token exists in database");
    }

    println!("Discovery Database test completed successfully!");

    Ok(())
}
