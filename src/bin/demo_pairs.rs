use anyhow::Result;
use env_logger;
use log::info;
use screenerbot::pairs::{ PairsClient, PairsTrait };

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logger
    env_logger::init();

    info!("Starting Pairs Demo");

    // Create a new pairs client
    let client = PairsClient::new()?;

    // Jupiter token address on Solana
    let jupiter_token = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

    println!("🔍 Fetching all pairs for Jupiter (JUP) token...");
    println!("Token Address: {}", jupiter_token);
    println!();

    // Fetch all token pairs
    let pairs = client.get_token_pairs(jupiter_token).await?;

    if pairs.is_empty() {
        println!("❌ No pairs found for this token");
        return Ok(());
    }

    println!("✅ Found {} total pairs", pairs.len());
    println!();

    // Show basic statistics
    let total_liquidity: f64 = pairs
        .iter()
        .map(|p| p.liquidity.usd)
        .sum();
    let total_volume_24h: f64 = pairs
        .iter()
        .map(|p| p.volume.h24)
        .sum();
    let avg_price: f64 =
        pairs
            .iter()
            .filter_map(|p| p.price_usd_float().ok())
            .sum::<f64>() / (pairs.len() as f64);

    println!("📊 OVERALL STATISTICS");
    println!("Total Liquidity: ${:.2}", total_liquidity);
    println!("Total 24h Volume: ${:.2}", total_volume_24h);
    println!("Average Price: ${:.6}", avg_price);
    println!();

    // Filter for high liquidity pairs (>$50k)
    let high_liquidity_pairs = client.filter_by_liquidity(pairs.clone(), 50_000.0);
    println!("💰 HIGH LIQUIDITY PAIRS (>$50K)");
    println!("Found {} high liquidity pairs", high_liquidity_pairs.len());

    // Sort by liquidity and show top 5
    let top_liquidity = client.sort_by_liquidity(high_liquidity_pairs);
    for (i, pair) in top_liquidity.iter().take(5).enumerate() {
        println!(
            "{}. {} - {}/{} | Liquidity: ${:.2} | 24h Vol: ${:.2} | Price: ${}",
            i + 1,
            pair.dex_id.to_uppercase(),
            pair.base_token.symbol,
            pair.quote_token.symbol,
            pair.liquidity.usd,
            pair.volume.h24,
            pair.price_usd
        );
    }
    println!();

    // Filter for major pairs only (SOL, USDC, USDT)
    let major_pairs = client.filter_major_pairs(pairs.clone());
    println!("🏆 MAJOR TRADING PAIRS");
    println!("Found {} major pairs", major_pairs.len());

    // Sort by volume and show top 5
    let top_volume = client.sort_by_volume(major_pairs);
    for (i, pair) in top_volume.iter().take(5).enumerate() {
        let price_change_24h = pair.price_change.h24.unwrap_or(0.0);
        let change_emoji = if price_change_24h > 0.0 { "📈" } else { "📉" };

        println!(
            "{}. {} - {}/{} | 24h Vol: ${:.2} | Liquidity: ${:.2} | Change: {}{:.2}%",
            i + 1,
            pair.dex_id.to_uppercase(),
            pair.base_token.symbol,
            pair.quote_token.symbol,
            pair.volume.h24,
            pair.liquidity.usd,
            change_emoji,
            price_change_24h
        );
    }
    println!();

    // Show DEX distribution
    let mut dex_count = std::collections::HashMap::new();
    for pair in &pairs {
        *dex_count.entry(&pair.dex_id).or_insert(0) += 1;
    }

    println!("🏪 DEX DISTRIBUTION");
    let mut dex_vec: Vec<_> = dex_count.iter().collect();
    dex_vec.sort_by(|a, b| b.1.cmp(a.1));
    for (dex, count) in dex_vec {
        println!("{}: {} pairs", dex.to_uppercase(), count);
    }
    println!();

    // Find the best overall pair
    if let Some(best_pair) = client.get_best_pair(pairs.clone()) {
        println!("🥇 BEST OVERALL PAIR (by liquidity + volume)");
        println!("DEX: {}", best_pair.dex_id.to_uppercase());
        println!("Pair: {}/{}", best_pair.base_token.symbol, best_pair.quote_token.symbol);
        println!("Pair Address: {}", best_pair.pair_address);
        println!("Liquidity: ${:.2}", best_pair.liquidity.usd);
        println!("24h Volume: ${:.2}", best_pair.volume.h24);
        println!("Price: ${}", best_pair.price_usd);
        println!("URL: {}", best_pair.url);

        if let Some(info) = &best_pair.info {
            if let Some(websites) = &info.websites {
                if !websites.is_empty() {
                    println!("Website: {}", websites[0].url);
                }
            }
        }
    }
    println!();

    // Show recent activity
    let active_pairs: Vec<_> = pairs
        .iter()
        .filter(|p| p.has_recent_activity())
        .collect();

    println!("🔥 RECENT ACTIVITY (last 5 minutes)");
    println!("Found {} pairs with recent activity", active_pairs.len());
    for (i, pair) in active_pairs.iter().take(3).enumerate() {
        println!(
            "{}. {} - {}/{} | Buys: {} | Sells: {} | 5m Vol: ${:.2}",
            i + 1,
            pair.dex_id.to_uppercase(),
            pair.base_token.symbol,
            pair.quote_token.symbol,
            pair.txns.m5.buys,
            pair.txns.m5.sells,
            pair.volume.m5
        );
    }

    println!();
    println!("✅ Pairs Demo completed successfully!");

    Ok(())
}
