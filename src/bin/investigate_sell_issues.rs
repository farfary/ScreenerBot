use screenerbot::wallet::*;
use screenerbot::global::*;
use screenerbot::logger::{ log, LogTag };
use screenerbot::trader::{ Position, SAVED_POSITIONS };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Investigating Sell Transaction Issues\n");

    // Load positions to analyze sell failures
    let positions = SAVED_POSITIONS.lock().unwrap().clone();
    drop(positions);

    println!("📊 Issue Analysis:");
    println!("═══════════════════");

    // Get wallet address for balance checking
    let wallet_address = match get_wallet_address() {
        Ok(addr) => {
            println!("✅ Wallet Address: {}", addr);
            addr
        }
        Err(e) => {
            println!("❌ Failed to get wallet address: {}", e);
            return Ok(());
        }
    };

    // Check SOL balance
    match get_sol_balance(&wallet_address).await {
        Ok(balance) => println!("💰 SOL Balance: {:.6} SOL", balance),
        Err(e) => println!("❌ Failed to get SOL balance: {}", e),
    }

    println!("\n🔍 Analyzing Sell Transaction Failures:");
    println!("════════════════════════════════════════");

    // Load positions again for detailed analysis
    let positions = SAVED_POSITIONS.lock().unwrap().clone();
    let failed_sells: Vec<&Position> = positions
        .iter()
        .filter(|p| {
            if let Some(ref exit_sig) = p.exit_transaction_signature {
                exit_sig == "NO_TOKENS_TO_SELL"
            } else {
                false
            }
        })
        .collect();

    println!("Found {} positions with failed sell transactions", failed_sells.len());

    for (i, position) in failed_sells.iter().enumerate() {
        println!("\n📍 Failed Sell #{}: {} ({})", i + 1, position.symbol, position.mint);

        // Check current token balance
        match get_token_balance(&wallet_address, &position.mint).await {
            Ok(current_balance) => {
                println!("   Current Token Balance: {}", current_balance);

                if let Some(expected_amount) = position.token_amount {
                    println!("   Expected Token Amount: {}", expected_amount);

                    if current_balance == 0 {
                        println!("   🟡 Balance is 0 - tokens may have been sold successfully");
                        println!("       but sell transaction wasn't recorded properly");
                    } else if current_balance == expected_amount {
                        println!("   🔴 Full balance still present - sell definitely failed");
                    } else {
                        println!("   🟠 Partial balance - possible partial sell or other activity");
                    }
                }
            }
            Err(e) => {
                println!("   ❌ Failed to check token balance: {}", e);
            }
        }

        // Analyze the entry transaction for clues
        if let Some(ref entry_sig) = position.entry_transaction_signature {
            println!("   Entry TX: {}...{}", &entry_sig[..8], &entry_sig[entry_sig.len() - 8..]);

            // The buy was successful, so the issue is likely in the sell logic
            if position.token_amount.is_some() && position.effective_entry_price.is_some() {
                println!("   ✅ Entry transaction has proper data");
            } else {
                println!("   ⚠️  Entry transaction missing token amount or effective price");
            }
        }

        // Check if this token still exists in the discovery system
        let tokens = LIST_TOKENS.read().unwrap();
        let token_found = tokens.iter().any(|t| t.mint == position.mint);
        if token_found {
            println!("   ✅ Token still exists in discovery system");
        } else {
            println!("   ⚠️  Token no longer in discovery system (may be delisted)");
        }
        drop(tokens);

        // Display the exact error scenario
        println!("   📋 Sell Scenario Analysis:");
        if let Some(exit_price) = position.exit_price {
            println!("      • Exit price was set: {:.8} SOL", exit_price);
            println!("      • But exit signature shows: NO_TOKENS_TO_SELL");
            println!("      • This suggests the sell logic thought there were no tokens");
            println!(
                "      • But the position shows {} tokens should exist",
                position.token_amount.unwrap_or(0)
            );
        }
    }

    println!("\n🔧 Potential Root Causes:");
    println!("═════════════════════════");
    println!("1. ❌ Token balance check fails before sell");
    println!("2. ❌ ATA (Associated Token Account) doesn't exist or is closed");
    println!("3. ❌ Token account has 0 balance but position wasn't updated");
    println!("4. ❌ Sell transaction fails but error isn't handled properly");
    println!("5. ❌ Token decimals mismatch causing balance calculation errors");
    println!("6. ❌ RPC connection issues during sell transaction");

    println!("\n🧪 Testing Current Sell Function:");
    println!("═════════════════════════════════");

    // Test with one of the failed positions if we have any
    if let Some(test_position) = failed_sells.first() {
        println!("Testing sell logic with {} ({})", test_position.symbol, test_position.mint);

        // Check if we can get current balance
        match get_token_balance(&wallet_address, &test_position.mint).await {
            Ok(balance) => {
                println!("✅ Successfully checked token balance: {}", balance);

                if balance > 0 {
                    println!("🔍 Tokens are present - investigating why sell failed...");

                    // Test quote generation (without actually selling)
                    let test_amount = balance;
                    println!("   Testing quote for {} tokens...", test_amount);

                    // This would test the quote logic without executing
                    // We can't actually call sell here as it would execute a real transaction
                    println!("   💡 Suggestion: The sell function should be tested in isolation");
                    println!("      to identify where exactly it's failing");
                } else {
                    println!("ℹ️  No tokens to sell - this explains the NO_TOKENS_TO_SELL");
                    println!("   The position should have been marked as auto-closed");
                }
            }
            Err(e) => {
                println!("❌ Failed to check balance: {}", e);
                println!("   This could be the root cause - balance check failure");
            }
        }
    }

    println!("\n💡 Recommended Fixes:");
    println!("═══════════════════════");
    println!("1. 🔧 Improve error handling in sell_token() function");
    println!("2. 🔧 Add comprehensive balance validation before sell attempts");
    println!("3. 🔧 Implement transaction verification after sell attempts");
    println!("4. 🔧 Add retry logic for failed RPC calls");
    println!("5. 🔧 Enhance logging to capture exact failure points");
    println!("6. 🔧 Add ATA existence check before sell attempts");
    println!("7. 🔧 Implement proper position cleanup for auto-detected sells");

    println!("\n🔍 Next Steps:");
    println!("══════════════");
    println!("1. Run debug_profit_calculation to see detailed position analysis");
    println!("2. Run test_improved_profit_system to test the new calculation system");
    println!("3. Monitor the trader logs for sell transaction failures");
    println!("4. Consider implementing a position reconciliation system");

    Ok(())
}
