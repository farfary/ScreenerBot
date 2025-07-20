use screenerbot::{
    global::read_configs,
    wallet::{ get_wallet_address, get_token_balance, close_token_account },
    logger::{ log, LogTag },
};
use serde_json::Value;
use std::fs;

/// Find token accounts from our trading history and test closing empty ones
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing ATA Closing with Historical Positions");
    println!("==============================================");

    // Load configurations
    let _configs = read_configs("configs.json")?;
    let wallet_address = get_wallet_address()?;
    println!("✅ Wallet address: {}", wallet_address);

    // Read positions.json to get mints we've traded
    let positions_content = fs::read_to_string("positions.json")?;
    let positions: Value = serde_json::from_str(&positions_content)?;

    if let Some(positions_array) = positions.as_array() {
        println!("📊 Found {} historical positions", positions_array.len());

        // Get unique mints from our trading history
        let mut unique_mints = std::collections::HashSet::new();
        for position in positions_array {
            if let Some(mint) = position.get("mint").and_then(|m| m.as_str()) {
                unique_mints.insert(mint.to_string());
            }
        }

        println!("🎯 Found {} unique tokens traded", unique_mints.len());

        // Check current balance for each mint to find empty accounts
        let mut empty_mints = Vec::new();
        let mut mints_with_balance = Vec::new();

        println!("\n🔍 Checking current balances...");
        for mint in unique_mints.iter() {
            print!("   Checking {}... ", &mint[..8]);

            match get_token_balance(&wallet_address, mint).await {
                Ok(balance) => {
                    if balance == 0 {
                        println!("EMPTY ✅");
                        empty_mints.push(mint.clone());
                    } else {
                        println!("HAS {} TOKENS ❌", balance);
                        mints_with_balance.push((mint.clone(), balance));
                    }
                }
                Err(e) => {
                    println!("NO ACCOUNT OR ERROR: {} ⚠️", e);
                    // Still try to close in case account exists but is empty
                    empty_mints.push(mint.clone());
                }
            }

            // Small delay to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        println!("\n📋 Summary");
        println!("=========");
        println!("Empty/No account: {}", empty_mints.len());
        println!("With balance: {}", mints_with_balance.len());

        if !empty_mints.is_empty() {
            println!("\n🧪 Testing ATA Closing on First Empty Account");
            println!("==============================================");

            let test_mint = &empty_mints[0];
            println!("🎯 Test target: {} ({})", &test_mint[..16], &test_mint[16..32]);

            // Try to close this ATA
            println!("\n🗑️  Attempting to close ATA...");
            match close_token_account(test_mint, &wallet_address).await {
                Ok(signature) => {
                    println!("✅ SUCCESS! ATA closed successfully");
                    println!("   Transaction signature: {}", signature);
                    println!("   💰 Rent reclaimed: ~0.002 SOL");
                }
                Err(e) => {
                    println!("❌ FAILED: {}", e);
                    println!("   This is expected with current placeholder implementation");
                }
            }
        } else {
            println!("\n⚠️  No empty token accounts found to test closing");
            if !mints_with_balance.is_empty() {
                println!("💡 Consider selling some tokens first to create empty accounts");
                println!("   Accounts with balance:");
                for (mint, balance) in mints_with_balance.iter().take(3) {
                    println!("   - {}: {} tokens", &mint[..16], balance);
                }
            }
        }

        // Show some sample mints for manual testing
        println!("\n📋 Sample Mints for Testing (first 5):");
        for (i, mint) in unique_mints.iter().take(5).enumerate() {
            println!("{}. {}", i + 1, mint);
        }
    } else {
        println!("❌ Could not parse positions.json");
    }

    Ok(())
}
