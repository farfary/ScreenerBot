// Test token deletion functionality with foreign key constraints
use screenerbot::logger::{init_file_logging, log, LogTag};
use screenerbot::tokens::cache::TokenDatabase;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <mint_address1> [mint_address2] ...", args[0]);
        println!(
            "Example: {} EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            args[0]
        );
        return Ok(());
    }

    let test_mints: Vec<String> = args[1..].to_vec();

    // Initialize token database
    let token_db = TokenDatabase::new()?;

    println!("Testing deletion of {} tokens:", test_mints.len());
    for mint in &test_mints {
        println!("  - {}", mint);
    }

    // Check if tokens exist before deletion
    println!("\nChecking if tokens exist in database...");
    for mint in &test_mints {
        match token_db.get_tokens_by_mints(&[mint.clone()]).await {
            Ok(tokens) => {
                if tokens.is_empty() {
                    println!("  ✗ {} not found in database", mint);
                } else {
                    println!("  ✓ {} exists in database", mint);
                }
            }
            Err(e) => println!("  ⚠ Error checking {}: {}", mint, e),
        }
    }

    // Check foreign key references
    println!("\nChecking foreign key references...");
    let connection = screenerbot::tokens::cache::create_configured_connection()?;

    for mint in &test_mints {
        let liquidity_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM liquidity_tracking WHERE mint = ?",
                &[mint],
                |row| row.get(0),
            )
            .unwrap_or(0);

        let route_failure_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM route_failure_tracking WHERE mint = ?",
                &[mint],
                |row| row.get(0),
            )
            .unwrap_or(0);

        if liquidity_count > 0 || route_failure_count > 0 {
            println!(
                "  {} has {} liquidity_tracking + {} route_failure_tracking records",
                mint, liquidity_count, route_failure_count
            );
        } else {
            println!("  {} has no foreign key references", mint);
        }
    }

    // Perform deletion
    println!("\nAttempting to delete tokens...");
    match token_db.delete_tokens(&test_mints).await {
        Ok(deleted_count) => {
            println!("✓ Successfully deleted {} tokens", deleted_count);

            // Verify deletion
            println!("\nVerifying deletion...");
            for mint in &test_mints {
                match token_db.get_tokens_by_mints(&[mint.clone()]).await {
                    Ok(tokens) => {
                        if tokens.is_empty() {
                            println!("  ✓ {} successfully deleted", mint);
                        } else {
                            println!("  ⚠ {} still exists in database", mint);
                        }
                    }
                    Err(e) => println!("  ⚠ Error checking {}: {}", mint, e),
                }
            }
        }
        Err(e) => {
            println!("✗ Failed to delete tokens: {}", e);
        }
    }

    Ok(())
}
