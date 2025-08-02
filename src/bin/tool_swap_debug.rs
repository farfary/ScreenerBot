use screenerbot::{
    global::{ read_configs, load_wallet_from_config },
    tokens::{
        api::get_global_dexscreener_api,
        price_service::get_token_price_safe,
        decimals::get_token_decimals_from_chain,
        Token,
    },
    rpc::init_rpc_client,
    wallet::{ buy_token, sell_token, get_wallet_address, SwapRequest },
};
use chrono::Utc;
use colored::*;
use std::thread;
use std::time::Duration;

async fn debug_swap_operations_with_amount(
    mint: &str,
    test_amount_sol: f64
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🔧 REAL SWAP DEBUG TOOL - STARTING INITIALIZATION".bright_blue().bold());

    // Load configuration
    println!("📄 Loading configuration...");
    let configs = read_configs("configs.json")?;
    let wallet = load_wallet_from_config(&configs)?;
    let wallet_address = get_wallet_address()?;
    println!("✅ Configuration loaded successfully");
    println!("💼 Wallet address: {}", wallet_address);

    // Initialize systems
    println!("🔗 Initializing RPC client...");
    init_rpc_client()?;

    println!("🌐 Initializing DexScreener API...");
    screenerbot::tokens::api::init_dexscreener_api().await?;

    println!("✅ All systems initialized");

    println!("\n{}", format!("💰 TEST AMOUNT: {} SOL", test_amount_sol).bright_green().bold());

    // Get token information first
    println!("\n{}", "📊 STEP 1: GATHERING TOKEN INFORMATION".bright_yellow().bold());

    // Get token decimals
    let token_decimals = match get_token_decimals_from_chain(mint).await {
        Ok(decimals) => {
            println!("✅ Token decimals: {}", decimals);
            decimals
        }
        Err(e) => {
            println!("❌ Failed to get decimals: {}", e);
            println!("⚠️  Using default decimals: 9");
            9
        }
    };

    // Get token price from API or alternative sources
    println!("💲 Getting token price...");
    let token_price = match get_token_price_safe(mint).await {
        Some(price) => {
            println!("✅ API Price: {} SOL", price);
            println!("🔍 DEBUG: API price source used for slippage calculation");
            price
        }
        None => {
            println!("⚠️  API price not available, trying pool price...");
            // Try to get pool price as backup
            match screenerbot::tokens::pool::get_pool_service().get_pool_price(mint, None).await {
                Some(pool_result) if pool_result.price_sol.is_some() => {
                    let price = pool_result.price_sol.unwrap();
                    println!("✅ Pool Price: {} SOL", price);
                    println!("🔍 DEBUG: Pool price source used for slippage calculation");
                    price
                }
                _ => {
                    println!("⚠️  Pool price not available, using fallback price for testing...");
                    // Use a small price for testing purposes only
                    let fallback_price = 0.000001;
                    println!("⚠️  Using fallback price: {} SOL for testing", fallback_price);
                    println!(
                        "🔍 DEBUG: FALLBACK price source used - this will cause HIGH slippage!"
                    );
                    fallback_price
                }
            }
        }
    };

    // Get DexScreener data to create Token struct
    println!("📈 Getting token metadata from DexScreener...");
    let api = get_global_dexscreener_api().await?;
    let mints = vec![mint.to_string()];
    let api_tokens = {
        let mut api_instance = api.lock().await;
        api_instance.get_tokens_info(&mints).await?
    };

    let api_token = api_tokens.first().ok_or("Token not found in DexScreener")?;

    // Create Token struct for swap operations
    let token = Token {
        mint: mint.to_string(),
        symbol: api_token.symbol.clone(),
        name: api_token.name.clone(),
        chain: "solana".to_string(),
        logo_url: api_token.info.as_ref().and_then(|i| i.image_url.clone()),
        coingecko_id: None,
        website: api_token.info
            .as_ref()
            .and_then(|i| i.websites.as_ref())
            .and_then(|w| w.first())
            .map(|w| w.url.clone()),
        description: None,
        tags: vec![],
        is_verified: false,
        created_at: Some(Utc::now()),
        price_dexscreener_sol: Some(token_price),
        price_dexscreener_usd: Some(api_token.price_usd),
        price_pool_sol: None,
        price_pool_usd: None,
        dex_id: Some(api_token.dex_id.clone()),
        pair_address: Some(api_token.pair_address.clone()),
        pair_url: api_token.pair_url.clone(),
        labels: api_token.labels.clone().unwrap_or_default(),
        fdv: api_token.fdv,
        market_cap: api_token.market_cap,
        txns: api_token.txns.clone(),
        volume: api_token.volume.clone(),
        price_change: api_token.price_change.clone(),
        liquidity: api_token.liquidity.clone(),
        info: None,
        boosts: api_token.boosts.clone(),
    };

    println!("✅ Token information gathered:");
    println!("   Symbol: {}", token.symbol);
    println!("   Name: {}", token.name);
    println!("   Price: {} SOL", token_price);
    println!("   Decimals: {}", token_decimals);

    // STEP 2: PERFORM BUY SWAP
    println!("\n{}", "🛒 STEP 2: PERFORMING BUY SWAP (SOL → TOKEN)".bright_blue().bold());
    println!("💸 Buying {} SOL worth of {} tokens...", test_amount_sol, token.symbol);

    // Create a swap request for buying tokens with SOL
    let swap_request = SwapRequest {
        input_mint: "So11111111111111111111111111111111111111112".to_string(), // SOL mint
        output_mint: token.mint.clone(),
        amount_sol: test_amount_sol,
        from_address: wallet_address.to_string(),
        slippage: 10.0, // 10% slippage tolerance for testing
        fee: 0.0,
        is_anti_mev: false,
        expected_price: Some(token_price),
    };

    let buy_result = match buy_token(&token, test_amount_sol, Some(token_price)).await {
        Ok(result) => {
            if result.success {
                println!("✅ {} BUY SWAP SUCCESSFUL!", "SUCCESS".bright_green().bold());
                println!(
                    "   Transaction: {}",
                    result.transaction_signature.as_ref().unwrap_or(&"No signature".to_string())
                );
                println!("   Input amount: {} lamports", result.input_amount);
                println!("   Output amount: {} tokens", result.output_amount);
                if let Some(price) = result.effective_price {
                    println!("   Effective price: {:.12} SOL", price);
                    println!(
                        "   Price difference: {:.2}%",
                        ((price - token_price) / token_price) * 100.0
                    );
                } else {
                    println!("   Effective price: Not available");
                }
                result
            } else {
                println!("❌ {} BUY SWAP FAILED!", "FAILED".bright_red().bold());
                println!("   Error: {}", result.error.unwrap_or("Unknown error".to_string()));
                return Err("Buy swap failed".into());
            }
        }
        Err(e) => {
            println!("❌ {} BUY SWAP ERROR: {}", "ERROR".bright_red().bold(), e);
            return Err(format!("Buy swap error: {}", e).into());
        }
    };

    println!("\n⏳ Waiting 10 seconds before sell swap to allow price updates...");
    thread::sleep(Duration::from_secs(10));

    // STEP 3: GET UPDATED PRICE FOR SELL
    println!("\n{}", "📊 STEP 3: GETTING UPDATED PRICE FOR SELL".bright_yellow().bold());
    let updated_price = match get_token_price_safe(mint).await {
        Some(price) => {
            println!("✅ Updated price: {} SOL", price);
            price
        }
        None => {
            println!("⚠️  Using original price: {} SOL", token_price);
            token_price
        }
    };

    // STEP 4: PERFORM SELL SWAP
    println!("\n{}", "💰 STEP 4: PERFORMING SELL SWAP (TOKEN → SOL)".bright_blue().bold());

    // Get token amount from buy output_amount
    let token_amount_to_sell = match buy_result.output_amount.parse::<u64>() {
        Ok(amount) => amount,
        Err(_) => {
            println!("❌ Failed to parse token amount from buy result");
            return Err("Cannot parse token amount".into());
        }
    };

    println!("💸 Selling {} tokens back to SOL...", token_amount_to_sell);

    let expected_sol_output =
        ((token_amount_to_sell as f64) / (10_f64).powi(token_decimals as i32)) * updated_price;
    println!("📈 Expected SOL output: {:.8} SOL", expected_sol_output);

    let sell_result = match
        sell_token(&token, token_amount_to_sell, Some(expected_sol_output)).await
    {
        Ok(result) => {
            if result.success {
                println!("✅ {} SELL SWAP SUCCESSFUL!", "SUCCESS".bright_green().bold());
                println!(
                    "   Transaction: {}",
                    result.transaction_signature.as_ref().unwrap_or(&"No signature".to_string())
                );
                println!("   Input amount: {} tokens", result.input_amount);
                println!("   Output amount: {} lamports", result.output_amount);
                if let Some(price) = result.effective_price {
                    println!("   Effective price: {:.12} SOL", price);
                    println!(
                        "   Price difference: {:.2}%",
                        ((price - updated_price) / updated_price) * 100.0
                    );
                } else {
                    println!("   Effective price: Not available");
                }
                result
            } else {
                println!("❌ {} SELL SWAP FAILED!", "FAILED".bright_red().bold());
                println!("   Error: {}", result.error.unwrap_or("Unknown error".to_string()));
                return Err("Sell swap failed".into());
            }
        }
        Err(e) => {
            println!("❌ {} SELL SWAP ERROR: {}", "ERROR".bright_red().bold(), e);
            return Err(format!("Sell swap error: {}", e).into());
        }
    };

    // STEP 5: CALCULATE ROUND-TRIP RESULTS
    println!("\n{}", "📊 STEP 5: ROUND-TRIP ANALYSIS".bright_green().bold());

    let sol_invested = test_amount_sol;
    let sol_received_lamports = match sell_result.output_amount.parse::<u64>() {
        Ok(amount) => amount,
        Err(_) => {
            println!("⚠️  Failed to parse SOL amount from sell result");
            0
        }
    };
    let sol_received = (sol_received_lamports as f64) / 1_000_000_000.0; // Convert lamports to SOL
    let net_sol_change = sol_received - sol_invested;
    let net_percentage = (net_sol_change / sol_invested) * 100.0;

    println!("💰 ROUND-TRIP SUMMARY:");
    println!("   SOL invested: {:.8} SOL", sol_invested);
    println!("   SOL received: {:.8} SOL", sol_received);
    println!("   Net change: {:.8} SOL ({:.3}%)", net_sol_change, net_percentage);

    if net_percentage > 0.0 {
        println!("   {} Round-trip was profitable!", "📈".bright_green());
    } else if net_percentage > -5.0 {
        println!("   {} Small loss (likely fees)", "⚠️".bright_yellow());
    } else {
        println!("   {} Significant loss detected", "📉".bright_red());
    }

    // STEP 6: DETAILED PRICE ANALYSIS
    println!("\n{}", "🔍 STEP 6: DETAILED PRICE ANALYSIS".bright_cyan().bold());

    println!("🏷️  PRICE EVOLUTION:");
    println!("   Initial API price: {:.12} SOL", token_price);
    if let Some(buy_price) = buy_result.effective_price {
        println!("   Buy effective price: {:.12} SOL", buy_price);
    }
    println!("   Updated API price: {:.12} SOL", updated_price);
    if let Some(sell_price) = sell_result.effective_price {
        println!("   Sell effective price: {:.12} SOL", sell_price);
    }

    println!("📊 SLIPPAGE ANALYSIS:");

    // Add detailed debug logging for slippage calculation
    println!("🔍 DEBUG - SLIPPAGE CALCULATION DETAILS:");
    println!("   📊 Initial token price (for comparison): {:.15} SOL", token_price);
    if let Some(buy_price) = buy_result.effective_price {
        println!("   📊 Buy effective price: {:.15} SOL", buy_price);
        println!("   📊 Price difference: {:.15} SOL", buy_price - token_price);
        println!(
            "   📊 Calculation: (({:.15} - {:.15}) / {:.15}) * 100",
            buy_price,
            token_price,
            token_price
        );
    }
    if let Some(sell_price) = sell_result.effective_price {
        println!("   📊 Sell effective price: {:.15} SOL", sell_price);
        println!("   📊 Updated price (for comparison): {:.15} SOL", updated_price);
        println!("   📊 Price difference: {:.15} SOL", sell_price - updated_price);
    }

    let buy_slippage = if let Some(buy_price) = buy_result.effective_price {
        let slippage = ((buy_price - token_price) / token_price) * 100.0;
        println!("   📊 Calculated buy slippage: {:.6}%", slippage);
        slippage
    } else {
        println!("   📊 No buy effective price available");
        0.0
    };
    let sell_slippage = if let Some(sell_price) = sell_result.effective_price {
        let slippage = ((sell_price - updated_price) / updated_price) * 100.0;
        println!("   📊 Calculated sell slippage: {:.6}%", slippage);
        slippage
    } else {
        println!("   📊 No sell effective price available");
        0.0
    };

    println!("   Buy slippage: {:.3}%", buy_slippage);
    println!("   Sell slippage: {:.3}%", sell_slippage);
    println!("   Combined slippage impact: {:.3}%", buy_slippage + sell_slippage);

    // STEP 7: TRANSACTION DETAILS
    println!("\n{}", "🔗 STEP 7: TRANSACTION DETAILS".bright_magenta().bold());
    println!("📝 TRANSACTION HASHES:");
    println!(
        "   Buy transaction: {}",
        buy_result.transaction_signature.as_ref().unwrap_or(&"No signature".to_string())
    );
    println!(
        "   Sell transaction: {}",
        sell_result.transaction_signature.as_ref().unwrap_or(&"No signature".to_string())
    );

    println!("🔍 AMOUNTS:");
    println!("   Input SOL: {} lamports", buy_result.input_amount);
    println!("   Tokens received: {} tokens", buy_result.output_amount);
    println!("   Tokens sold: {} tokens", sell_result.input_amount);
    println!("   SOL received: {} lamports", sell_result.output_amount);

    println!("\n{}", "🎯 DEBUG SUMMARY".bright_blue().bold());
    println!("Token: {} ({})", token.symbol, mint);
    println!("Round-trip completed: {} → {} → SOL", "SOL", token.symbol);
    println!("Net result: {:.8} SOL ({:.3}%)", net_sol_change, net_percentage);
    println!(
        "Buy TX: {}",
        buy_result.transaction_signature.as_ref().unwrap_or(&"No signature".to_string())
    );
    println!(
        "Sell TX: {}",
        sell_result.transaction_signature.as_ref().unwrap_or(&"No signature".to_string())
    );

    println!("\n✅ Real swap debug analysis completed!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();

    let mut mint: Option<String> = None;
    let mut test_amount = 0.002; // Default test amount

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--token" => {
                if i + 1 < args.len() {
                    mint = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("❌ Error: --token requires a mint address");
                    std::process::exit(1);
                }
            }
            "--amount" => {
                if i + 1 < args.len() {
                    match args[i + 1].parse::<f64>() {
                        Ok(amount) => {
                            // Allow any positive amount
                            if amount > 0.0 {
                                test_amount = amount;
                                i += 2;
                            } else {
                                eprintln!("❌ Error: --amount must be greater than 0");
                                std::process::exit(1);
                            }
                        }
                        Err(_) => {
                            eprintln!("❌ Error: --amount requires a valid positive number");
                            std::process::exit(1);
                        }
                    }
                } else {
                    eprintln!("❌ Error: --amount requires a value");
                    std::process::exit(1);
                }
            }
            _ => {
                // Check if this is a mint address without --token flag
                if mint.is_none() && args[i].len() > 40 {
                    mint = Some(args[i].clone());
                }
                i += 1;
            }
        }
    }

    let mint = match mint {
        Some(m) => m,
        None => {
            eprintln!("❌ Error: No mint address provided");
            eprintln!("Usage: {} --token <MINT_ADDRESS> [--amount <AMOUNT>]", args[0]);
            eprintln!("   or: {} <MINT_ADDRESS> [--amount <AMOUNT>]", args[0]);
            eprintln!("   Amount: Any positive SOL amount (default: 0.002)");
            std::process::exit(1);
        }
    };

    println!("🚀 Starting REAL swap debug test for mint: {}", mint);
    println!("💰 Test amount: {} SOL (as requested)", test_amount);

    // Update the test amount in the debug function
    if let Err(e) = debug_swap_operations_with_amount(&mint, test_amount).await {
        eprintln!("❌ Debug test failed: {}", e);
        std::process::exit(1);
    }

    println!("✅ Real swap debug test completed successfully");
    Ok(())
}
