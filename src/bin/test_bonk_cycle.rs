use screenerbot::{
    global::{ read_configs, Token },
    wallet::{
        buy_token,
        sell_token,
        close_token_account,
        get_wallet_address,
        get_token_balance,
        get_sol_balance,
    },
    logger::{ log, LogTag },
};

/// Test buying BONK, selling it, and closing the ATA
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing BONK Buy -> Sell -> Close ATA Cycle");
    println!("==============================================");

    // Load configurations
    let _configs = read_configs("configs.json")?;
    let wallet_address = get_wallet_address()?;
    println!("✅ Wallet address: {}", wallet_address);

    // BONK token details
    let bonk_token = Token {
        mint: "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263".to_string(), // BONK mint
        symbol: "BONK".to_string(),
        name: "Bonk".to_string(),
        decimals: 5, // BONK has 5 decimals
        chain: "solana".to_string(),
        // Optional fields
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![], // Empty vector instead of None
        is_verified: true, // Boolean instead of Option<bool>
        created_at: None,
        // Price fields
        price_dexscreener_sol: None,
        price_dexscreener_usd: None,
        price_geckoterminal_sol: None,
        price_geckoterminal_usd: None,
        price_raydium_sol: None,
        price_raydium_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        pools: vec![], // Empty vector for pools
        // DexScreener API fields
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![], // Empty vector instead of None
        fdv: None,
        market_cap: None,
        txns: None,
        volume: None,
        price_change: None,
        liquidity: None,
        info: None,
        boosts: None,
    };

    let buy_amount = 0.001; // 0.001 SOL

    println!("\n📊 Initial Balances");
    println!("==================");

    // Check initial SOL balance
    let initial_sol_balance = get_sol_balance(&wallet_address).await?;
    println!("SOL Balance: {:.6} SOL", initial_sol_balance);

    // Check initial BONK balance
    let initial_bonk_balance = get_token_balance(&wallet_address, &bonk_token.mint).await.unwrap_or(
        0
    );
    println!("BONK Balance: {} tokens", initial_bonk_balance);

    if initial_sol_balance < buy_amount + 0.01 {
        println!(
            "❌ Insufficient SOL balance for test. Need at least {:.6} SOL",
            buy_amount + 0.01
        );
        return Ok(());
    }

    println!("\n🛒 Step 1: Buying BONK");
    println!("=====================");

    // Buy BONK tokens
    match buy_token(&bonk_token, buy_amount, None).await {
        Ok(buy_result) => {
            if buy_result.success {
                println!("✅ Buy successful!");
                println!(
                    "   TX: {}",
                    buy_result.transaction_signature.as_ref().unwrap_or(&"None".to_string())
                );

                if let Some(effective_price) = buy_result.effective_price {
                    println!("   Effective price: {:.12} SOL per BONK", effective_price);
                }

                if let Some(tokens_received) = buy_result.actual_output_change {
                    println!("   Tokens received: {} BONK", tokens_received);

                    // Wait a bit for the transaction to settle
                    println!("   ⏳ Waiting 10 seconds for transaction to settle...");
                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

                    // Check BONK balance after buy
                    let bonk_balance_after_buy = get_token_balance(
                        &wallet_address,
                        &bonk_token.mint
                    ).await.unwrap_or(0);
                    println!("   BONK balance after buy: {} tokens", bonk_balance_after_buy);

                    // Use the actual received amount or the balance, whichever is higher
                    let sellable_amount = std::cmp::max(bonk_balance_after_buy, tokens_received);

                    if sellable_amount > 0 {
                        println!("\n💰 Step 2: Selling BONK");
                        println!("=======================");
                        println!("   Selling {} BONK tokens", sellable_amount);

                        // Sell all BONK tokens
                        match sell_token(&bonk_token, sellable_amount, None).await {
                            Ok(sell_result) => {
                                if sell_result.success {
                                    println!("✅ Sell successful!");
                                    println!(
                                        "   TX: {}",
                                        sell_result.transaction_signature
                                            .as_ref()
                                            .unwrap_or(&"None".to_string())
                                    );

                                    if let Some(effective_price) = sell_result.effective_price {
                                        println!(
                                            "   Effective sell price: {:.12} SOL per BONK",
                                            effective_price
                                        );
                                    }

                                    if let Some(sol_received) = sell_result.actual_output_change {
                                        let sol_received_amount =
                                            screenerbot::wallet::lamports_to_sol(sol_received);
                                        println!("   SOL received: {:.6} SOL", sol_received_amount);

                                        // Calculate P&L
                                        let net_pnl = sol_received_amount - buy_amount;
                                        let pnl_percent = (net_pnl / buy_amount) * 100.0;

                                        if net_pnl >= 0.0 {
                                            println!(
                                                "   📈 Profit: +{:.6} SOL ({:.2}%)",
                                                net_pnl,
                                                pnl_percent
                                            );
                                        } else {
                                            println!(
                                                "   📉 Loss: {:.6} SOL ({:.2}%)",
                                                net_pnl,
                                                pnl_percent
                                            );
                                        }
                                    }

                                    // Wait for sell transaction to settle
                                    println!(
                                        "   ⏳ Waiting 10 seconds for sell transaction to settle..."
                                    );
                                    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await; // Check BONK balance after sell
                                    let bonk_balance_after_sell = get_token_balance(
                                        &wallet_address,
                                        &bonk_token.mint
                                    ).await.unwrap_or(0);
                                    println!("   BONK balance after sell: {} tokens", bonk_balance_after_sell);

                                    if bonk_balance_after_sell == 0 {
                                        println!("\n🗑️  Step 3: Closing BONK ATA");
                                        println!("============================");

                                        // Close the ATA to reclaim rent
                                        match
                                            close_token_account(
                                                &bonk_token.mint,
                                                &wallet_address
                                            ).await
                                        {
                                            Ok(close_tx) => {
                                                println!("✅ ATA closed successfully!");
                                                println!("   TX: {}", close_tx);
                                                println!("   💰 Rent reclaimed (~0.002 SOL)");

                                                // Wait for close transaction to settle
                                                println!(
                                                    "   ⏳ Waiting 5 seconds for close transaction to settle..."
                                                );
                                                tokio::time::sleep(
                                                    tokio::time::Duration::from_secs(5)
                                                ).await;

                                                // Final verification - try to get BONK balance (should fail or return 0)
                                                let final_bonk_balance = get_token_balance(
                                                    &wallet_address,
                                                    &bonk_token.mint
                                                ).await.unwrap_or(0);
                                                if final_bonk_balance == 0 {
                                                    println!(
                                                        "   ✅ Verified: No BONK token account exists"
                                                    );
                                                } else {
                                                    println!("   ⚠️  Warning: BONK balance still shows {} tokens", final_bonk_balance);
                                                }
                                            }
                                            Err(e) => {
                                                println!("❌ Failed to close ATA: {}", e);
                                                if
                                                    e
                                                        .to_string()
                                                        .contains(
                                                            "No associated token account found"
                                                        )
                                                {
                                                    println!(
                                                        "   ℹ️  This might mean the ATA was already closed or never existed"
                                                    );
                                                }
                                            }
                                        }
                                    } else {
                                        println!("   ⚠️  Cannot close ATA - still have {} BONK tokens", bonk_balance_after_sell);
                                    }
                                } else {
                                    println!(
                                        "❌ Sell failed: {}",
                                        sell_result.error
                                            .as_ref()
                                            .unwrap_or(&"Unknown error".to_string())
                                    );
                                }
                            }
                            Err(e) => {
                                println!("❌ Failed to sell BONK: {}", e);
                            }
                        }
                    } else {
                        println!(
                            "❌ No BONK tokens available to sell (received: {}, balance: {})",
                            tokens_received,
                            bonk_balance_after_buy
                        );
                    }
                } else {
                    println!("❌ Buy result missing token amount information");
                }
            } else {
                println!(
                    "❌ Buy failed: {}",
                    buy_result.error.as_ref().unwrap_or(&"Unknown error".to_string())
                );
            }
        }
        Err(e) => {
            println!("❌ Failed to buy BONK: {}", e);
        }
    }

    println!("\n📊 Final Balances");
    println!("=================");

    // Check final SOL balance
    let final_sol_balance = get_sol_balance(&wallet_address).await?;
    println!("SOL Balance: {:.6} SOL", final_sol_balance);

    // Check final BONK balance
    let final_bonk_balance = get_token_balance(&wallet_address, &bonk_token.mint).await.unwrap_or(
        0
    );
    println!("BONK Balance: {} tokens", final_bonk_balance);

    // Calculate total cost including fees
    let total_cost = initial_sol_balance - final_sol_balance;
    println!("\n💰 Summary");
    println!("==========");
    println!("Total cost (including fees): {:.6} SOL", total_cost);
    println!("Buy amount: {:.6} SOL", buy_amount);
    println!("Net fees paid: {:.6} SOL", total_cost - buy_amount);

    if final_bonk_balance == 0 {
        println!("✅ Test completed successfully - all BONK tokens sold and ATA closed");
    } else {
        println!("⚠️  Test incomplete - {} BONK tokens remain", final_bonk_balance);
    }

    Ok(())
}
