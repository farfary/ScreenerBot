use screenerbot::{ global::{ LIST_TOKENS } };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing Current Price Display");

    // Check if LIST_TOKENS has any data
    {
        let tokens = LIST_TOKENS.read().unwrap();
        println!("📊 Found {} tokens in LIST_TOKENS", tokens.len());

        if tokens.is_empty() {
            println!(
                "⚠️  LIST_TOKENS is empty. You need to run the discovery first to populate tokens."
            );
            println!("💡 Current price will show 'N/A' until tokens are discovered.");
        } else {
            // Show some sample tokens with prices
            println!("📈 Sample tokens with prices:");
            for (i, token) in tokens.iter().take(5).enumerate() {
                println!(
                    "  {}. {} ({}) - DexScreener SOL: {:?}",
                    i + 1,
                    token.symbol,
                    &token.mint[..8],
                    token.price_dexscreener_sol
                );
            }
        }
    }

    println!(
        "\n🔄 Testing position display (should show current prices if tokens are discovered):"
    );
    // This would call the display function, but we need to be sure the bot is set up properly

    Ok(())
}
