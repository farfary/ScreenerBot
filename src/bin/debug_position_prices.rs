/// Debug utility to analyze why open position prices show as N/A
///
/// This tool examines:
/// - Pool price cache status
/// - Global token list contents
/// - Price source availability
/// - Validation status for each open position token

use screenerbot::logger::{ log, LogTag };
use screenerbot::global::{ read_configs, LIST_TOKENS };
use screenerbot::positions::SAVED_POSITIONS;
use screenerbot::pool_price_manager::{
    debug_token_price_lookup,
    get_best_available_price,
    is_token_validated,
    get_cached_pool_price,
};
use screenerbot::trader::get_current_token_price;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Debugging Open Position Price Display Issues\n");

    // Load configurations
    let _configs = match read_configs("configs.json") {
        Ok(configs) => {
            println!("✅ Configuration loaded successfully");
            configs
        }
        Err(e) => {
            println!("❌ Failed to load configs: {}", e);
            return Ok(());
        }
    };

    // Get open positions
    let open_positions = {
        let all_positions = SAVED_POSITIONS.lock().unwrap();
        all_positions
            .iter()
            .filter(|p| p.exit_time.is_none())
            .cloned()
            .collect::<Vec<_>>()
    };

    println!("📊 Found {} open positions\n", open_positions.len());

    if open_positions.is_empty() {
        println!("No open positions found. Exiting.");
        return Ok(());
    }

    // Analyze each open position
    for (i, position) in open_positions.iter().enumerate() {
        println!("🔍 Position {} - {} ({})", i + 1, position.symbol, position.mint);
        println!("   Entry Price: {:.10} SOL", position.entry_price);

        // Get current price using the same method as the UI
        let current_price = get_current_token_price(&position.mint, true);
        println!("   Current Price: {:?}", current_price);

        if current_price.is_none() {
            println!("   ❌ NO PRICE FOUND - Analyzing why...\n");

            // Detailed analysis
            let debug_info = debug_token_price_lookup(&position.mint);
            for line in debug_info.lines() {
                println!("      {}", line);
            }

            // Try best available price
            let best_price = get_best_available_price(&position.mint);
            println!("      Best available price: {:?}", best_price);

            println!();
        } else {
            println!("   ✅ Price found: {:.10} SOL\n", current_price.unwrap());
        }
    }

    // Analyze global token list status
    println!("🌐 Global Token List Analysis:");
    match LIST_TOKENS.try_read() {
        Ok(tokens) => {
            println!("   ✅ Token list accessible with {} tokens", tokens.len());

            let mut found_count = 0;
            let mut with_dexscreener_price = 0;
            let mut with_pool_price = 0;

            for position in &open_positions {
                if let Some(token) = tokens.iter().find(|t| t.mint == position.mint) {
                    found_count += 1;
                    if token.price_dexscreener_sol.is_some() {
                        with_dexscreener_price += 1;
                    }
                    if token.price_pool_sol.is_some() {
                        with_pool_price += 1;
                    }
                }
            }

            println!(
                "   📈 Open positions found in token list: {}/{}",
                found_count,
                open_positions.len()
            );
            println!(
                "   💰 With DexScreener SOL price: {}/{}",
                with_dexscreener_price,
                found_count
            );
            println!("   🏊 With Pool SOL price: {}/{}", with_pool_price, found_count);
        }
        Err(_) => {
            println!("   ❌ Could not access token list");
        }
    }

    // Pool price cache analysis
    println!("\n🏊 Pool Price Cache Analysis:");
    let mut validated_count = 0;
    let mut cached_count = 0;

    for position in &open_positions {
        if is_token_validated(&position.mint) {
            validated_count += 1;
            if get_cached_pool_price(&position.mint).is_some() {
                cached_count += 1;
            }
        }
    }

    println!("   ✅ Validated tokens: {}/{}", validated_count, open_positions.len());
    println!("   💾 Cached pool prices: {}/{}", cached_count, validated_count);

    println!("\n🔧 Recommendations:");
    if validated_count == 0 {
        println!("   1. Pool price manager may not be running properly");
        println!("   2. Tokens may not be getting validated for pool decoding");
    }
    if cached_count < validated_count {
        println!("   3. Pool price cache may be expiring too quickly");
    }

    let tokens_not_in_list =
        open_positions.len() -
        (if let Ok(tokens) = LIST_TOKENS.try_read() {
            open_positions
                .iter()
                .filter(|p| tokens.iter().any(|t| t.mint == p.mint))
                .count()
        } else {
            0
        });

    if tokens_not_in_list > 0 {
        println!("   4. {} tokens not found in global token list - may need discovery refresh", tokens_not_in_list);
    }

    Ok(())
}
