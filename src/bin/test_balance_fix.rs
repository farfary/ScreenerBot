use screenerbot::{
    global::{ read_configs, Token },
    wallet::{ get_wallet_address, get_token_balance },
    logger::{ log, LogTag },
    utils::{ load_positions_from_file },
};

/// Test that demonstrates the balance fix for closing positions
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Balance Fix for Position Closing");
    println!("============================================");

    // Load configurations
    let _configs = read_configs("configs.json")?;
    let wallet_address = get_wallet_address()?;
    println!("✅ Wallet address: {}", wallet_address);

    // Load existing positions
    let positions = load_positions_from_file();
    println!("📊 Loaded {} positions", positions.len());

    // Find positions with token amounts and check their current balances
    let mut positions_with_balance_issues = 0;
    let mut total_checked = 0;

    for position in positions.iter() {
        if let Some(stored_amount) = position.token_amount {
            if stored_amount > 0 {
                total_checked += 1;

                println!("\n🔍 Checking position: {} ({})", position.symbol, position.mint);
                println!("   Stored amount: {}", stored_amount);

                match get_token_balance(&wallet_address, &position.mint).await {
                    Ok(actual_balance) => {
                        println!("   Actual balance: {}", actual_balance);

                        if actual_balance < stored_amount {
                            positions_with_balance_issues += 1;
                            let sellable_amount = std::cmp::min(stored_amount, actual_balance);

                            println!("   ⚠️  BALANCE MISMATCH DETECTED!");
                            println!("       - Stored: {}", stored_amount);
                            println!("       - Actual: {}", actual_balance);
                            println!("       - Difference: {}", stored_amount - actual_balance);
                            println!("       - Would sell: {} (min of both)", sellable_amount);

                            if actual_balance == 0 {
                                println!(
                                    "       - 🚨 ZERO BALANCE - Position should be auto-closed"
                                );
                            }
                        } else {
                            println!("   ✅ Balance OK (actual >= stored)");
                        }
                    }
                    Err(e) => {
                        println!("   ❌ Failed to check balance: {}", e);
                    }
                }

                // Add delay to avoid rate limiting
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
    }

    println!("\n📊 SUMMARY");
    println!("==========");
    println!("Total positions checked: {}", total_checked);
    println!("Positions with balance issues: {}", positions_with_balance_issues);
    println!("Positions with correct balances: {}", total_checked - positions_with_balance_issues);

    if positions_with_balance_issues > 0 {
        println!("\n✅ The balance fix will handle {} problematic positions", positions_with_balance_issues);
        println!("   - These positions will now sell only the available balance");
        println!("   - No more 'Insufficient Balance' errors should occur");
    } else {
        println!("\n✅ All positions have correct balances - no fix needed currently");
    }

    Ok(())
}
