use screenerbot::trader::database::TraderDatabase;
use screenerbot::pairs::client::PairsClient;
use screenerbot::config::Config;
use anyhow::Result;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🔍 ScreenerBot Position Token Debug Tool");
    println!("═══════════════════════════════════════════");

    // Load config
    let config = Config::load("configs.json")?;

    // Initialize database
    let trader_db = Arc::new(TraderDatabase::new("trader.db")?);

    // Initialize pairs client
    let pairs_client = Arc::new(PairsClient::new()?);

    // Get all positions
    let positions = trader_db.get_active_positions()?;

    if positions.is_empty() {
        println!("📭 No active positions found in database");
        return Ok(());
    }

    println!("📊 Found {} active positions in database\n", positions.len());

    for (position_id, position) in positions.iter() {
        let mint = &position.token_address;
        println!("🧪 Testing position token: {}", mint);
        println!("   Position ID: {}", position_id);
        println!("   Symbol: {}", position.token_symbol);
        println!("   Amount: {} tokens", position.total_tokens);
        println!("   SOL Invested: {}", position.total_invested_sol);
        println!("   Entry Price: {}", position.average_buy_price);

        // Check if mint address looks valid (Solana address is typically 32-44 chars)
        if mint.len() < 32 || mint.len() > 44 {
            println!("   ❌ INVALID: Token address length {} (should be 32-44)", mint.len());
        } else if !mint.chars().all(|c| c.is_alphanumeric()) {
            println!("   ❌ INVALID: Token address contains non-alphanumeric characters");
        } else {
            println!("   ✅ Token address format looks valid");
        }

        // Test with DexScreener API
        println!("   📡 Testing DexScreener API...");
        match pairs_client.get_solana_token_pairs(mint).await {
            Ok(pairs) => {
                if pairs.is_empty() {
                    println!("   ⚠️  No trading pairs found on DexScreener");
                } else {
                    println!("   ✅ Found {} trading pairs", pairs.len());
                    for (i, pair) in pairs.iter().take(3).enumerate() {
                        println!("      Pair {}: {} on {}", i + 1, pair.pair_address, pair.dex_id);
                    }
                }
            }
            Err(e) => {
                println!("   ❌ API Error: {}", e);
            }
        }

        // Check if we can get price
        match pairs_client.get_best_price(mint).await {
            Ok(Some(price)) => {
                println!("   💰 Current price: ${:.8}", price);
            }
            Ok(None) => {
                println!("   ❌ No price available");
            }
            Err(_) => {
                println!("   ❌ Price fetch error");
            }
        }

        println!("   ────────────────────────────────────────");
    }

    // Summary
    let total_positions = positions.len();
    let mut valid_addresses = 0;
    let mut has_pairs = 0;
    let mut has_prices = 0;

    for (position_id, position) in positions.iter() {
        // Check address format
        if
            position.token_address.len() >= 32 &&
            position.token_address.len() <= 44 &&
            position.token_address.chars().all(|c| c.is_alphanumeric())
        {
            valid_addresses += 1;
        }

        // Check if has pairs (re-test for summary)
        if let Ok(pairs) = pairs_client.get_solana_token_pairs(&position.token_address).await {
            if !pairs.is_empty() {
                has_pairs += 1;

                // Check if has price
                if let Ok(Some(_)) = pairs_client.get_best_price(&position.token_address).await {
                    has_prices += 1;
                }
            }
        }

        // Rate limit between requests
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    println!("\n📈 Summary:");
    println!("   Total positions: {}", total_positions);
    println!("   Valid address format: {}/{}", valid_addresses, total_positions);
    println!("   Have trading pairs: {}/{}", has_pairs, total_positions);
    println!("   Have current prices: {}/{}", has_prices, total_positions);

    if has_pairs == 0 {
        println!("\n⚠️  ISSUE IDENTIFIED:");
        println!("   All positions contain tokens with no trading pairs on DexScreener.");
        println!("   This explains why price updates are failing.");
        println!("   These might be:");
        println!("   - Test tokens");
        println!("   - Very new tokens not yet indexed");
        println!("   - Inactive/delisted tokens");
        println!("   - Tokens with corrupted/invalid addresses");
    }

    Ok(())
}
