// debug_token_monitoring.rs - Debug token monitoring database and LIST_TOKENS issues
use screenerbot::*;
use std::sync::Arc;
use tokio::sync::Notify;
use tokio::time::{ sleep, Duration };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Debugging Token Monitoring System");
    println!("====================================");

    // Initialize systems
    global::initialize_token_database()?;

    // Check initial database state
    println!("\n1. Checking initial database state...");
    if let Ok(token_db_guard) = global::TOKEN_DB.lock() {
        if let Some(ref db) = *token_db_guard {
            let tokens = db.get_all_tokens()?;
            println!("   📊 Database contains {} tokens", tokens.len());

            // Show first few tokens if any
            for (i, token) in tokens.iter().take(3).enumerate() {
                let liquidity = token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);
                println!(
                    "   {}. {} ({}): ${:.0} liquidity",
                    i + 1,
                    token.symbol,
                    token.mint,
                    liquidity
                );
            }
        } else {
            println!("   ❌ Database not initialized");
        }
    }

    // Check initial LIST_TOKENS state
    println!("\n2. Checking initial LIST_TOKENS state...");
    if let Ok(tokens) = global::LIST_TOKENS.read() {
        println!("   📈 LIST_TOKENS contains {} tokens", tokens.len());
    } else {
        println!("   ❌ Could not read LIST_TOKENS");
    }

    // Check LIST_MINTS state
    println!("\n3. Checking LIST_MINTS state...");
    if let Ok(mints) = global::LIST_MINTS.read() {
        println!("   🔗 LIST_MINTS contains {} mints", mints.len());
        for (i, mint) in mints.iter().take(3).enumerate() {
            println!("   {}. {}", i + 1, mint);
        }
    } else {
        println!("   ❌ Could not read LIST_MINTS");
    }

    // Run discovery for a short time to populate data
    println!("\n4. Running discovery to populate data...");
    let shutdown = Arc::new(Notify::new());

    let discovery_shutdown = shutdown.clone();
    let discovery_handle = tokio::spawn(async move {
        discovery_manager::start_discovery_task(discovery_shutdown).await;
    });

    // Let discovery run for 10 seconds
    sleep(Duration::from_secs(10)).await;

    // Check database state after discovery
    println!("\n5. Checking database state after discovery...");
    if let Ok(token_db_guard) = global::TOKEN_DB.lock() {
        if let Some(ref db) = *token_db_guard {
            let tokens = db.get_all_tokens()?;
            println!("   📊 Database now contains {} tokens", tokens.len());

            // Show liquidity breakdown
            let with_liquidity = tokens
                .iter()
                .filter(|token| {
                    token.liquidity
                        .as_ref()
                        .and_then(|l| l.usd)
                        .unwrap_or(0.0) > 0.0
                })
                .count();
            println!("   💰 {} tokens have liquidity data", with_liquidity);

            // Show top liquidity tokens
            let mut sorted_tokens = tokens.clone();
            sorted_tokens.sort_by(|a, b| {
                let liquidity_a = a.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);
                let liquidity_b = b.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);
                liquidity_b.partial_cmp(&liquidity_a).unwrap_or(std::cmp::Ordering::Equal)
            });

            println!("   🏆 Top 3 tokens by liquidity:");
            for (i, token) in sorted_tokens.iter().take(3).enumerate() {
                let liquidity = token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);
                println!(
                    "   {}. {} ({}): ${:.0} liquidity",
                    i + 1,
                    token.symbol,
                    token.mint,
                    liquidity
                );
            }
        }
    }

    // Check LIST_TOKENS state after discovery
    println!("\n6. Checking LIST_TOKENS state after discovery...");
    if let Ok(tokens) = global::LIST_TOKENS.read() {
        println!("   📈 LIST_TOKENS now contains {} tokens", tokens.len());

        if tokens.len() > 0 {
            // Show some examples
            for (i, token) in tokens.iter().take(3).enumerate() {
                let liquidity = token.liquidity
                    .as_ref()
                    .and_then(|l| l.usd)
                    .unwrap_or(0.0);
                println!(
                    "   {}. {} ({}): ${:.0} liquidity",
                    i + 1,
                    token.symbol,
                    token.mint,
                    liquidity
                );
            }
        }
    }

    // Test token monitor loading from database
    println!("\n7. Testing token monitor database loading...");
    let token_monitor = token_monitor::TokenMonitor::new();
    // This is a private method, so we'll test the get_tokens_for_monitoring equivalent
    if let Ok(token_db_guard) = global::TOKEN_DB.lock() {
        if let Some(ref db) = *token_db_guard {
            let tokens = db.get_all_tokens().map_err(|e| format!("Database error: {}", e))?;
            println!("   📊 Token monitor can load {} tokens from database", tokens.len());

            // Test position exclusion
            let open_position_mints = position_monitor::get_open_position_mints();
            println!("   🔒 Found {} open position mints", open_position_mints.len());

            let filtered_tokens: Vec<_> = tokens
                .into_iter()
                .filter(|token| !open_position_mints.contains(&token.mint))
                .collect();

            println!(
                "   🧹 After position exclusion: {} tokens remain for monitoring",
                filtered_tokens.len()
            );
        }
    }

    // Shutdown discovery
    shutdown.notify_waiters();
    discovery_handle.abort();

    println!("\n✅ Debug completed!");
    Ok(())
}
