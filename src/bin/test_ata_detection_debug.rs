use screenerbot::transactions::*;
use screenerbot::wallet::detect_and_separate_ata_rent;
use screenerbot::global::read_configs;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing ATA Detection on Specific Problem Transactions\n");

    let configs = read_configs("configs.json")?;
    let wallet_address = screenerbot::wallet::get_wallet_address()?;

    // Initialize transaction fetcher
    let fetcher = TransactionFetcher::new(configs, None)?;

    // Test cases from positions.json
    let test_cases = vec![
        (
            "PHI - WRONG (includes ATA)",
            "4xLQDEdQQaiWpXp5cS35xNvtCBKMNmNdLeUUumQnn6fgMYThHGK6iYz44yoXZhs1aEAdSYEZLmhPWEEtrHxgZ5Ph",
            "C19J3fcXX9otmTjPuGNdZMQdfRG6SRhbnJv8EJnRpump", // PHI mint
            0.002554144, // reported sol_received (looks wrong - likely includes ATA)
            true // should detect ATA
        ),
        (
            "AIRFRY - CORRECT (no ATA)",
            "5xPSJ8wrTi3jg4y1bR9GLKrZXJmqkbyGZWMQAH7VGvKuDxa3spcun7J1NuJjFL3waKxtQKmeSKpWNRZfDVpQZ4He",
            "4qTJV18HH5YUz9KSAdGEnVQuxPkR9c4gDwV7TaMxbonk", // AIRFRY mint
            0.00059585, // reported sol_received (looks correct)
            false // should not detect ATA
        ),
    ];

    for (name, signature, _token_mint, expected_sol, should_detect_ata) in test_cases {
        println!("{}", "=".repeat(80));
        println!("📊 Analyzing: {}", name);
        println!("🔗 Transaction: {}", signature);
        println!("💰 Expected SOL: {:.9}", expected_sol);
        println!("");

        match fetcher.get_transaction_details_with_fallback(signature).await {
            Ok(Some(transaction_result)) => {
                println!("✅ Transaction fetched successfully");

                // Convert to TransactionDetails format that our detection expects
                let details = screenerbot::wallet::TransactionDetails {
                    slot: transaction_result.slot,
                    transaction: screenerbot::wallet::TransactionData {
                        message: serde_json::Value::Null, // We don't need this for ATA detection
                        signatures: vec![signature.to_string()],
                    },
                    meta: transaction_result.meta.clone(),
                };

                // Print transaction metadata for debugging
                if let Some(meta) = &details.meta {
                    println!("📈 Pre-balances: {:?}", meta.pre_balances);
                    println!("📉 Post-balances: {:?}", meta.post_balances);
                    
                    if let Some(log_messages) = &meta.log_messages {
                        println!("📋 Log Messages ({} total):", log_messages.len());
                        for (i, log) in log_messages.iter().enumerate() {
                            println!("  [{}] {}", i, log);
                            // Look for ATA-related logs
                            if log.contains("CloseAccount") || log.contains("close") || log.contains("ATA") {
                                println!("    🎯 ATA-RELATED LOG DETECTED!");
                            }
                        }
                    }

                    // Calculate actual balance changes
                    let balance_changes: Vec<i64> = meta.post_balances.iter()
                        .zip(meta.pre_balances.iter())
                        .map(|(post, pre)| *post as i64 - *pre as i64)
                        .collect();
                    
                    println!("💱 Balance Changes: {:?}", balance_changes);

                    // Find wallet's balance change (usually first account)
                    if !balance_changes.is_empty() {
                        let wallet_sol_change = balance_changes[0];
                        println!("💰 Wallet SOL Change: {} lamports ({:.9} SOL)", 
                            wallet_sol_change, wallet_sol_change as f64 / 1_000_000_000.0);
                        
                        // Check if this matches our expected amount
                        let expected_lamports = (expected_sol * 1_000_000_000.0) as i64;
                        if wallet_sol_change == expected_lamports {
                            println!("  ✅ Matches expected SOL amount");
                        } else {
                            println!("  ⚠️  Different from expected: {} vs {} lamports", 
                                wallet_sol_change, expected_lamports);
                        }
                    }
                }

                // Test our ATA detection
                println!("\n🔍 Running ATA Detection:");
                let expected_lamports = (expected_sol * 1_000_000_000.0) as u64;
                let (ata_detected, ata_rent_amount, sol_from_trade_only) = 
                    detect_and_separate_ata_rent(&details, &wallet_address, expected_lamports, true);

                println!("🎯 ATA Detection Results:");
                println!("  • ATA Close Detected: {}", ata_detected);
                println!("  • ATA Rent Amount: {} lamports ({:.9} SOL)", 
                    ata_rent_amount, ata_rent_amount as f64 / 1_000_000_000.0);
                println!("  • SOL from Trade Only: {} lamports ({:.9} SOL)", 
                    sol_from_trade_only, sol_from_trade_only as f64 / 1_000_000_000.0);

                // Validation
                println!("\n✅ Validation:");
                if should_detect_ata && ata_detected {
                    println!("  ✅ CORRECT: ATA was detected as expected");
                    let ata_sol = ata_rent_amount as f64 / 1_000_000_000.0;
                    println!("  💡 ATA rent reclaimed: {:.9} SOL (~0.002 expected)", ata_sol);
                } else if !should_detect_ata && !ata_detected {
                    println!("  ✅ CORRECT: No ATA detected as expected");
                } else if should_detect_ata && !ata_detected {
                    println!("  ❌ ERROR: ATA should have been detected but wasn't!");
                    println!("  💡 This transaction might include ATA rent that we're missing");
                } else {
                    println!("  ❌ ERROR: ATA was detected but shouldn't have been!");
                    println!("  💡 False positive - clean transaction flagged as having ATA");
                }

                // Check if the amounts make sense
                if ata_detected {
                    let total_calculated = sol_from_trade_only + ata_rent_amount;
                    if total_calculated == expected_lamports {
                        println!("  ✅ CORRECT: ATA separation math adds up");
                        let clean_sol = sol_from_trade_only as f64 / 1_000_000_000.0;
                        println!("  💡 Clean trading proceeds: {:.9} SOL", clean_sol);
                    } else {
                        println!("  ❌ ERROR: ATA separation math doesn't add up!");
                        println!("    Expected: {} lamports", expected_lamports);
                        println!("    Got: {} + {} = {} lamports", 
                            sol_from_trade_only, ata_rent_amount, total_calculated);
                    }
                } else {
                    if sol_from_trade_only == expected_lamports {
                        println!("  ✅ CORRECT: No ATA separation, amounts match");
                    } else {
                        println!("  ❌ ERROR: No ATA detected but amounts don't match");
                        println!("    Expected: {} lamports", expected_lamports);
                        println!("    Got: {} lamports", sol_from_trade_only);
                    }
                }

            }
            Ok(None) => {
                println!("❌ Transaction not found in cache or RPC");
            }
            Err(e) => {
                println!("❌ Failed to fetch transaction: {:?}", e);
            }
        }

        println!("");
    }

    println!("🏁 ATA Detection Debug Complete");
    
    // Summary
    println!("\n📋 SUMMARY:");
    println!("• PHI transaction shows suspiciously high SOL (0.002554144) - likely includes ATA rent");
    println!("• AIRFRY transaction shows reasonable SOL (0.00059585) - likely clean");
    println!("• ATA rent is typically ~0.002 SOL (2,039,280 lamports)");
    println!("• When ATA is closed after swap, both amounts get summed incorrectly");
    
    Ok(())
}
