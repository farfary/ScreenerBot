use screenerbot::{
    global::{ read_configs },
    wallet::{ get_wallet_address, get_token_balance, close_token_account },
    logger::{ log, LogTag },
    trader::CLOSE_ATA_AFTER_SELL,
    utils::{ load_positions_from_file },
};

/// Test ATA closing functionality for both regular SPL tokens and Token-2022 tokens
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing ATA Closing Functionality");
    println!("=====================================");

    // Check configuration
    println!("📋 Configuration:");
    println!("   CLOSE_ATA_AFTER_SELL: {}", CLOSE_ATA_AFTER_SELL);

    // Load configurations and get wallet address
    let _configs = read_configs("configs.json")?;
    let wallet_address = get_wallet_address()?;
    println!("✅ Wallet address: {}", wallet_address);

    // Load existing positions to find tokens with zero balance (candidates for ATA closing)
    let positions = load_positions_from_file();
    println!("📊 Loaded {} positions", positions.len());

    let mut zero_balance_tokens = Vec::new();
    let mut checked_count = 0;

    println!("\n🔍 Checking for tokens with zero balance (ATA close candidates):");

    for position in positions.iter() {
        if position.exit_time.is_some() && position.token_amount.is_some() {
            checked_count += 1;

            let token_mint = &position.mint;

            match get_token_balance(&wallet_address, token_mint).await {
                Ok(balance) => {
                    if balance == 0 {
                        zero_balance_tokens.push((position.symbol.clone(), token_mint.clone()));
                        println!(
                            "   🔹 {} ({}) - Zero balance, ATA candidate",
                            position.symbol,
                            &token_mint[..8]
                        );
                    } else {
                        println!(
                            "   ⏹️  {} ({}) - Has {} tokens, cannot close ATA",
                            position.symbol,
                            &token_mint[..8],
                            balance
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "   ❌ {} ({}) - Error checking balance: {}",
                        position.symbol,
                        &token_mint[..8],
                        e
                    );
                }
            }

            // Add delay to avoid rate limiting
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

            // Limit to first 10 for testing
            if checked_count >= 10 {
                break;
            }
        }
    }

    println!("\n📊 SUMMARY");
    println!("==========");
    println!("Positions checked: {}", checked_count);
    println!("Zero balance tokens (ATA candidates): {}", zero_balance_tokens.len());

    if zero_balance_tokens.is_empty() {
        println!("ℹ️  No tokens with zero balance found - cannot test ATA closing");
        println!("   This is normal if you don't have any fully sold positions");
        return Ok(());
    }

    // Test ATA closing on the first zero-balance token
    if let Some((symbol, mint)) = zero_balance_tokens.first() {
        println!("\n🧪 TESTING ATA CLOSING");
        println!("======================");
        println!("Testing with: {} ({})", symbol, mint);

        if CLOSE_ATA_AFTER_SELL {
            println!("✅ CLOSE_ATA_AFTER_SELL is enabled - proceeding with test");

            log(LogTag::Trader, "TEST", &format!("Testing ATA close for {}", symbol));

            match close_token_account(mint, &wallet_address).await {
                Ok(tx_signature) => {
                    println!("✅ SUCCESS: ATA closed successfully!");
                    println!("   Transaction: {}", tx_signature);
                    println!("   Rent SOL reclaimed (~0.002 SOL)");
                }
                Err(e) => {
                    println!("❌ FAILED: {}", e);
                    println!("   This might be expected if:");
                    println!("   - ATA was already closed");
                    println!("   - Token account doesn't exist");
                    println!("   - Network/RPC issues");
                }
            }
        } else {
            println!("⚠️  CLOSE_ATA_AFTER_SELL is disabled - ATA closing would be skipped");
            println!("   Set CLOSE_ATA_AFTER_SELL = true in trader.rs to enable");
        }
    }

    println!("\n🔧 HOW TO CONFIGURE:");
    println!("====================");
    println!("In src/trader.rs, line ~14:");
    println!("pub const CLOSE_ATA_AFTER_SELL: bool = true;  // Enable ATA closing");
    println!("pub const CLOSE_ATA_AFTER_SELL: bool = false; // Disable ATA closing");

    println!("\n💡 BENEFITS OF ATA CLOSING:");
    println!("===========================");
    println!("- Reclaims ~0.002 SOL rent per token account");
    println!("- Cleans up wallet from empty token accounts");
    println!("- Supports both regular SPL tokens and Token-2022");
    println!("- Automatically detects token type and uses appropriate program");

    Ok(())
}
