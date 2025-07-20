use screenerbot::{
    global::{ read_configs, Token },
    trader::Position,
    wallet::{
        get_wallet_address,
        execute_swap,
        sell_token,
        lamports_to_sol,
        get_token_balance,
        get_sol_balance,
    },
    transactions::{
        SwapTransaction,
        TransactionCache,
        SignatureInfo,
        TransactionResult,
        detect_swaps_in_transaction,
        get_all_dex_program_ids,
    },
    utils::{ load_positions_from_file },
};
use serde_json;
use std::collections::HashMap;
use reqwest;

const TEST_AMOUNT_SOL: f64 = 0.0001; // Small test amount

/// Test token for debugging - use a well-known token with good liquidity
const TEST_TOKEN_MINT: &str = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"; // BONK token
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Starting Effective Price Debug Test");
    println!("======================================");

    // Load configurations
    let configs = read_configs("configs.json")?;
    let wallet_address = get_wallet_address()?;

    println!("💼 Wallet Address: {}", wallet_address);

    // Check initial balances
    let initial_sol_balance = get_sol_balance(&wallet_address).await?;
    println!("💰 Initial SOL Balance: {:.6} SOL", initial_sol_balance);

    // Load current positions to analyze
    let positions = load_positions_from_file();
    println!("📍 Found {} positions in positions.json", positions.len());

    // Analyze closed positions and their effective prices
    analyze_closed_positions(&positions).await?;

    // Perform test swaps to debug effective price calculation
    println!("\n🧪 Starting Test Swaps for Price Debugging");
    println!("==========================================");

    // Create test token
    let test_token = Token {
        mint: TEST_TOKEN_MINT.to_string(),
        symbol: "BONK".to_string(),
        name: "Bonk".to_string(),
        decimals: 5, // BONK has 5 decimals
        chain: "solana".to_string(),
        logo_url: None,
        coingecko_id: None,
        website: None,
        description: None,
        tags: vec![],
        is_verified: true,
        created_at: None,
        price_dexscreener_sol: None,
        price_dexscreener_usd: None,
        price_geckoterminal_sol: None,
        price_geckoterminal_usd: None,
        price_raydium_sol: None,
        price_raydium_usd: None,
        price_pool_sol: None,
        price_pool_usd: None,
        pools: vec![],
        dex_id: None,
        pair_address: None,
        pair_url: None,
        labels: vec![],
        fdv: None,
        market_cap: None,
        txns: None,
        volume: None,
        price_change: None,
        liquidity: None,
        info: None,
        boosts: None,
    };

    // Test 1: Buy small amount
    println!("\n🛒 Test 1: Buying {} SOL worth of {}", TEST_AMOUNT_SOL, test_token.symbol);

    let buy_result = execute_swap(
        &test_token,
        SOL_MINT,
        &test_token.mint,
        TEST_AMOUNT_SOL,
        None
    ).await;

    match buy_result {
        Ok(result) => {
            println!("✅ Buy successful!");
            println!(
                "  Transaction: {}",
                result.transaction_signature.as_ref().unwrap_or(&"None".to_string())
            );
            println!("  Input amount: {} lamports", result.input_amount);
            println!("  Output amount: {} raw tokens", result.output_amount);

            if let Some(effective_price) = result.effective_price {
                println!("  🎯 Effective Price: {:.12} SOL per token", effective_price);
            }

            // Check token balance after buy
            let token_balance = get_token_balance(&wallet_address, &test_token.mint).await?;
            println!("  💎 Token balance after buy: {} tokens", token_balance);

            // Test 2: Sell the tokens back
            if token_balance > 0 {
                println!("\n💸 Test 2: Selling {} tokens back to SOL", token_balance);

                let sell_result = sell_token(&test_token, token_balance, None).await;

                match sell_result {
                    Ok(sell_result) => {
                        println!("✅ Sell successful!");
                        println!(
                            "  Transaction: {}",
                            sell_result.transaction_signature
                                .as_ref()
                                .unwrap_or(&"None".to_string())
                        );
                        println!("  Input amount: {} raw tokens", sell_result.input_amount);
                        println!("  Output amount: {} lamports", sell_result.output_amount);

                        if let Some(effective_price) = sell_result.effective_price {
                            println!("  🎯 Effective Price: {:.12} SOL per token", effective_price);
                        }

                        // Calculate round-trip efficiency
                        let sol_received = lamports_to_sol(
                            sell_result.output_amount.parse().unwrap_or(0)
                        );
                        let round_trip_efficiency = (sol_received / TEST_AMOUNT_SOL) * 100.0;
                        println!(
                            "  🔄 Round-trip efficiency: {:.2}% ({:.6} SOL → {:.6} SOL)",
                            round_trip_efficiency,
                            TEST_AMOUNT_SOL,
                            sol_received
                        );
                    }
                    Err(e) => {
                        println!("❌ Sell failed: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ Buy failed: {}", e);
        }
    }

    // Final balance check
    let final_sol_balance = get_sol_balance(&wallet_address).await?;
    println!("\n💰 Final SOL Balance: {:.6} SOL", final_sol_balance);
    println!("💸 Total cost of test: {:.6} SOL", initial_sol_balance - final_sol_balance);

    println!("\n🎯 Debug Summary");
    println!("================");
    println!("The effective price calculation issues identified:");
    println!("1. Check if decimal handling is correct");
    println!("2. Verify token balance changes are properly calculated");
    println!("3. Ensure price calculation uses correct numerator/denominator");
    println!("4. Validate transaction parsing for balance changes");

    Ok(())
}

/// Analyze closed positions and their effective prices
async fn analyze_closed_positions(
    positions: &[Position]
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📈 Analyzing Closed Positions");
    println!("=============================");

    let closed_positions: Vec<_> = positions
        .iter()
        .filter(|p| p.exit_price.is_some())
        .collect();

    println!("📊 Found {} closed positions", closed_positions.len());

    for (i, position) in closed_positions.iter().enumerate() {
        println!("\n🔍 Position {} - {}", i + 1, position.symbol);
        println!("  Mint: {}", position.mint);
        println!("  Entry Price (DexScreener): {:.12} SOL", position.entry_price);
        println!("  Exit Price (DexScreener): {:.12} SOL", position.exit_price.unwrap_or(0.0));

        if let Some(effective_entry) = position.effective_entry_price {
            println!("  🎯 Effective Entry Price: {:.12} SOL", effective_entry);

            // Check if effective price looks wrong (too small)
            if effective_entry < 1e-10 {
                println!("  ⚠️  WARNING: Effective entry price seems too small!");

                // Suggest potential issue
                if let Some(token_amount) = position.token_amount {
                    println!("    🔧 Debug: Token amount = {}", token_amount);
                    println!("    🔧 Debug: Entry SOL = {:.6}", position.entry_size_sol);

                    // Try manual calculation with different decimal assumptions
                    let manual_price_5_decimals =
                        position.entry_size_sol / ((token_amount as f64) / (10_f64).powi(5));
                    let manual_price_6_decimals =
                        position.entry_size_sol / ((token_amount as f64) / (10_f64).powi(6));
                    let manual_price_9_decimals =
                        position.entry_size_sol / ((token_amount as f64) / (10_f64).powi(9));

                    println!(
                        "    🧮 Manual calc (5 decimals): {:.12} SOL",
                        manual_price_5_decimals
                    );
                    println!(
                        "    🧮 Manual calc (6 decimals): {:.12} SOL",
                        manual_price_6_decimals
                    );
                    println!(
                        "    🧮 Manual calc (9 decimals): {:.12} SOL",
                        manual_price_9_decimals
                    );

                    println!("    💡 Expected price range: {:.12} SOL", position.entry_price);
                }
            }
        }

        if let Some(effective_exit) = position.effective_exit_price {
            println!("  🎯 Effective Exit Price: {:.12} SOL", effective_exit);

            if effective_exit < 1e-10 {
                println!("  ⚠️  WARNING: Effective exit price seems too small!");
            }
        }

        if let Some(token_amount) = position.token_amount {
            println!("  💎 Token Amount: {} raw units", token_amount);
        }

        println!("  💰 Entry Size: {:.6} SOL", position.entry_size_sol);
        println!(
            "  📈 P&L: {:.6} SOL ({:.2}%)",
            position.pnl_sol.unwrap_or(0.0),
            position.pnl_percent.unwrap_or(0.0)
        );

        // Analyze transaction signatures if available
        if let Some(entry_tx) = &position.entry_transaction_signature {
            println!("  🟢 Entry TX: {}", entry_tx);
            analyze_transaction_for_effective_price(entry_tx, "ENTRY", &position.mint).await;
        }

        if let Some(exit_tx) = &position.exit_transaction_signature {
            println!("  🔴 Exit TX: {}", exit_tx);
            analyze_transaction_for_effective_price(exit_tx, "EXIT", &position.mint).await;
        }
    }

    Ok(())
}

/// Analyze a specific transaction to debug effective price calculation
async fn analyze_transaction_for_effective_price(signature: &str, tx_type: &str, token_mint: &str) {
    println!("    📋 {} Transaction Analysis:", tx_type);
    println!("      Signature: {}", signature);
    println!("      Token Mint: {}", token_mint);

    // Get transaction details from RPC
    let configs = match read_configs("configs.json") {
        Ok(c) => c,
        Err(e) => {
            println!("      ❌ Failed to read config: {}", e);
            return;
        }
    };

    // Fetch transaction details
    match fetch_and_analyze_transaction(signature, &configs.rpc_url, token_mint).await {
        Ok(_) => {
            println!("      ✅ Transaction analysis completed");
        }
        Err(e) => {
            println!("      ❌ Failed to analyze transaction: {}", e);
        }
    }
}

/// Fetch and analyze transaction details to debug price calculation
async fn fetch_and_analyze_transaction(
    signature: &str,
    rpc_url: &str,
    token_mint: &str
) -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let rpc_payload =
        serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTransaction",
        "params": [
            signature,
            {
                "encoding": "json",
                "maxSupportedTransactionVersion": 0
            }
        ]
    });

    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .json(&rpc_payload)
        .send().await?;

    let rpc_response: serde_json::Value = response.json().await?;

    if let Some(result) = rpc_response.get("result") {
        if !result.is_null() {
            // Extract and analyze balance changes
            if let Some(meta) = result.get("meta") {
                analyze_balance_changes_for_price(meta, token_mint)?;
            }
        } else {
            println!("      ❌ Transaction not found");
        }
    }

    Ok(())
}

/// Analyze balance changes to debug effective price calculation
fn analyze_balance_changes_for_price(
    meta: &serde_json::Value,
    token_mint: &str
) -> Result<(), Box<dyn std::error::Error>> {
    println!("      💰 Balance Change Analysis:");

    // Get wallet address for filtering
    let wallet_address = get_wallet_address()?;

    // SOL balance changes
    if
        let (Some(pre_balances), Some(post_balances)) = (
            meta.get("preBalances"),
            meta.get("postBalances"),
        )
    {
        if
            let (Ok(pre), Ok(post)) = (
                serde_json::from_value::<Vec<u64>>(pre_balances.clone()),
                serde_json::from_value::<Vec<u64>>(post_balances.clone()),
            )
        {
            if !pre.is_empty() && !post.is_empty() {
                let sol_change_lamports = (post[0] as i64) - (pre[0] as i64);
                let sol_change = lamports_to_sol(sol_change_lamports.abs() as u64);
                println!(
                    "        SOL Change: {} lamports ({:.9} SOL)",
                    sol_change_lamports,
                    sol_change
                );
            }
        }
    }

    // Token balance changes
    if
        let (Some(pre_token_balances), Some(post_token_balances)) = (
            meta.get("preTokenBalances"),
            meta.get("postTokenBalances"),
        )
    {
        if
            let (Ok(pre_balances), Ok(post_balances)) = (
                serde_json::from_value::<Vec<serde_json::Value>>(pre_token_balances.clone()),
                serde_json::from_value::<Vec<serde_json::Value>>(post_token_balances.clone()),
            )
        {
            // Find token changes for our specific mint
            let token_change = find_token_balance_change(
                &pre_balances,
                &post_balances,
                token_mint,
                &wallet_address
            );

            if let Some((change, decimals)) = token_change {
                println!("        Token Change: {} raw units", change);
                println!("        Token Decimals: {}", decimals);

                let ui_change = (change as f64) / (10_f64).powi(decimals as i32);
                println!("        Token UI Change: {} tokens", ui_change);

                // Try to calculate effective price
                if change != 0 {
                    // This is where the issue might be - let's debug this calculation
                    println!("        🔧 Price Calculation Debug:");
                    println!("          Raw token change: {}", change);
                    println!("          Decimals: {}", decimals);
                    println!("          UI token change: {}", ui_change);

                    // The current calculation might be wrong
                    let wrong_price = 0.0001 / (change as f64); // This is likely the bug!
                    println!(
                        "          WRONG calculation (0.0001 SOL / raw units): {:.12}",
                        wrong_price
                    );

                    // Correct calculation should be:
                    let correct_price = 0.0001 / ui_change;
                    println!(
                        "          CORRECT calculation (0.0001 SOL / UI tokens): {:.12}",
                        correct_price
                    );

                    println!(
                        "        🎯 The issue is likely using raw token units instead of UI token amounts!"
                    );
                }
            } else {
                println!("        ❌ No token balance change found for mint: {}", token_mint);
            }
        }
    }

    Ok(())
}

/// Find token balance change for a specific mint and wallet
fn find_token_balance_change(
    pre_balances: &[serde_json::Value],
    post_balances: &[serde_json::Value],
    target_mint: &str,
    wallet_address: &str
) -> Option<(i64, u8)> {
    // Create lookup maps
    let mut pre_by_mint: HashMap<String, &serde_json::Value> = HashMap::new();
    let mut post_by_mint: HashMap<String, &serde_json::Value> = HashMap::new();

    for balance in pre_balances {
        if
            let (Some(mint), Some(owner)) = (
                balance.get("mint").and_then(|m| m.as_str()),
                balance.get("owner").and_then(|o| o.as_str()),
            )
        {
            if owner == wallet_address {
                pre_by_mint.insert(mint.to_string(), balance);
            }
        }
    }

    for balance in post_balances {
        if
            let (Some(mint), Some(owner)) = (
                balance.get("mint").and_then(|m| m.as_str()),
                balance.get("owner").and_then(|o| o.as_str()),
            )
        {
            if owner == wallet_address {
                post_by_mint.insert(mint.to_string(), balance);
            }
        }
    }

    // Find the change for our target mint
    let pre_amount = pre_by_mint
        .get(target_mint)
        .and_then(|b| b.get("uiTokenAmount"))
        .and_then(|a| a.get("amount"))
        .and_then(|amt| amt.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let post_amount = post_by_mint
        .get(target_mint)
        .and_then(|b| b.get("uiTokenAmount"))
        .and_then(|a| a.get("amount"))
        .and_then(|amt| amt.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let decimals = post_by_mint
        .get(target_mint)
        .or_else(|| pre_by_mint.get(target_mint))
        .and_then(|b| b.get("uiTokenAmount"))
        .and_then(|a| a.get("decimals"))
        .and_then(|d| d.as_u64())
        .unwrap_or(9) as u8;

    let change = (post_amount as i64) - (pre_amount as i64);

    if change != 0 {
        Some((change, decimals))
    } else {
        None
    }
}

/// Debug token balance changes in detail
fn debug_token_balance_changes(
    pre_token_balances: &serde_json::Value,
    post_token_balances: &serde_json::Value,
    wallet_address: &str
) {
    println!("🔍 Debugging Token Balance Changes");

    // Convert to arrays
    if
        let (Ok(pre_balances), Ok(post_balances)) = (
            serde_json::from_value::<Vec<serde_json::Value>>(pre_token_balances.clone()),
            serde_json::from_value::<Vec<serde_json::Value>>(post_token_balances.clone()),
        )
    {
        println!("Pre-token balances count: {}", pre_balances.len());
        println!("Post-token balances count: {}", post_balances.len());

        // Create lookup maps by mint
        let mut pre_by_mint: HashMap<String, serde_json::Value> = HashMap::new();
        let mut post_by_mint: HashMap<String, serde_json::Value> = HashMap::new();

        for balance in pre_balances {
            if let Some(mint) = balance.get("mint").and_then(|m| m.as_str()) {
                pre_by_mint.insert(mint.to_string(), balance);
            }
        }

        for balance in post_balances {
            if let Some(mint) = balance.get("mint").and_then(|m| m.as_str()) {
                post_by_mint.insert(mint.to_string(), balance);
            }
        }

        // Find all mints involved
        let mut all_mints: std::collections::HashSet<String> = std::collections::HashSet::new();
        all_mints.extend(pre_by_mint.keys().cloned());
        all_mints.extend(post_by_mint.keys().cloned());

        for mint in all_mints {
            println!("\n🪙 Mint: {}", mint);

            let pre_amount = pre_by_mint
                .get(&mint)
                .and_then(|b| b.get("uiTokenAmount"))
                .and_then(|a| a.get("amount"))
                .and_then(|amt| amt.as_str())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);

            let post_amount = post_by_mint
                .get(&mint)
                .and_then(|b| b.get("uiTokenAmount"))
                .and_then(|a| a.get("amount"))
                .and_then(|amt| amt.as_str())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);

            let change = (post_amount as i64) - (pre_amount as i64);

            println!("  Pre-amount: {} raw units", pre_amount);
            println!("  Post-amount: {} raw units", post_amount);
            println!("  Change: {} raw units", change);

            // Get decimals for UI calculation
            let decimals = post_by_mint
                .get(&mint)
                .or_else(|| pre_by_mint.get(&mint))
                .and_then(|b| b.get("uiTokenAmount"))
                .and_then(|a| a.get("decimals"))
                .and_then(|d| d.as_u64())
                .unwrap_or(9) as u32;

            let ui_change = (change as f64) / (10_f64).powi(decimals as i32);
            println!("  UI Change: {} tokens (decimals: {})", ui_change, decimals);

            // Check if this balance belongs to our wallet
            let owner = post_by_mint
                .get(&mint)
                .or_else(|| pre_by_mint.get(&mint))
                .and_then(|b| b.get("owner"))
                .and_then(|o| o.as_str())
                .unwrap_or("unknown");

            println!("  Owner: {}", owner);

            if owner == wallet_address {
                println!("  ✅ This is OUR wallet's balance change!");
            } else {
                println!("  ❌ This is NOT our wallet (owner mismatch)");
            }
        }
    }
}
