mod core;
mod swap;

use std::error::Error;
use crate::swap::{ SwapManager, SwapRequest, SwapType };

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🚀 SCREENER BOT - MODULAR SWAP DEMO");
    println!("==================================");

    // Initialize swap manager with default providers
    let swap_manager = SwapManager::with_defaults().await;

    // Print provider status
    swap_manager.print_provider_status().await;
    println!();

    // Example wallet and token
    let wallet_address = "YourWalletAddressHere";
    let token_address = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"; // Example: BONK token

    // Demo 1: Quick buy operation
    println!("📈 DEMO 1: Quick Buy Operation");
    println!("   Buying 0.1 SOL worth of tokens...");

    match
        swap_manager.quick_buy(
            token_address,
            0.002, // 0.002 SOL
            wallet_address,
            Some(500) // 5% slippage
        ).await
    {
        Ok(result) => {
            println!("   ✅ Buy operation initiated!");
            result.print_summary();
        }
        Err(e) => {
            println!("   ❌ Buy failed: {}", e);
        }
    }

    println!();

    // Demo 2: Manual swap request with quote comparison
    println!("📊 DEMO 2: Quote Comparison");
    println!("   Getting quotes from all providers...");

    let buy_request = SwapRequest::new_buy(
        token_address,
        100_000_000, // 0.1 SOL in lamports
        wallet_address,
        500, // 5% slippage
        200_000 // Priority fee
    );

    let quotes = swap_manager.get_all_quotes(&buy_request).await;
    for (provider_id, quote_result) in quotes {
        match quote_result {
            Ok(quote) => {
                println!(
                    "   • {}: {} tokens out (price impact: {:.2}%)",
                    provider_id,
                    quote.out_amount,
                    quote.route_info.price_impact_pct
                );
            }
            Err(e) => {
                println!("   • {}: Error - {}", provider_id, e);
            }
        }
    }

    println!();

    // Demo 3: Best quote selection
    println!("🎯 DEMO 3: Best Quote Selection");
    match swap_manager.get_best_quote(&buy_request).await {
        Ok((provider_id, quote)) => {
            println!("   Best provider: {}", provider_id);
            println!("   Output amount: {} tokens", quote.out_amount);
            println!("   Price impact: {:.2}%", quote.route_info.price_impact_pct);
            println!("   Route: {}", quote.route_info.liquidity_sources.join(" -> "));
        }
        Err(e) => {
            println!("   ❌ No quotes available: {}", e);
        }
    }

    println!();

    // Demo 4: Provider-specific operation
    println!("🔧 DEMO 4: Provider-Specific Operation");
    println!("   Using GMGN provider directly...");

    match swap_manager.execute_swap_with_provider(&buy_request, "gmgn").await {
        Ok(result) => {
            println!("   ✅ GMGN swap initiated!");
            result.print_summary();

            // Demo transaction monitoring
            if let Some(signature) = &result.transaction_signature {
                println!("   Monitoring transaction: {}", signature);

                match swap_manager.monitor_transaction(signature, "gmgn").await {
                    Ok(status) => {
                        println!("   Transaction status: {:?}", status);
                    }
                    Err(e) => {
                        println!("   Status check failed: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("   ❌ GMGN swap failed: {}", e);
        }
    }

    println!();

    // Demo 5: Sell operation
    println!("📉 DEMO 5: Quick Sell Operation");
    println!("   Selling 1000 tokens...");

    match
        swap_manager.quick_sell(
            token_address,
            1000_000, // 1000 tokens (assuming 6 decimals)
            wallet_address,
            Some(500) // 5% slippage
        ).await
    {
        Ok(result) => {
            println!("   ✅ Sell operation initiated!");
            result.print_summary();
        }
        Err(e) => {
            println!("   ❌ Sell failed: {}", e);
        }
    }

    println!();
    println!("🏁 DEMO COMPLETE");
    println!("   The modular swap system is ready for production use!");
    println!("   To integrate:");
    println!("   1. Add your wallet configuration");
    println!("   2. Configure API keys for providers");
    println!("   3. Add additional providers (Jupiter, etc.)");
    println!("   4. Implement proper error handling and retries");

    Ok(())
}
