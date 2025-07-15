use screenerbot::screener::sources::{ raydium::RaydiumSource, TokenSource };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("🚀 Testing Raydium API integration...");

    // Create Raydium source
    let mut raydium = RaydiumSource::new();

    // Initialize the source
    println!("📡 Initializing Raydium source...");
    raydium.initialize().await?;

    // Health check
    println!("🏥 Performing health check...");
    let is_healthy = raydium.health_check().await?;
    println!("Health status: {}", if is_healthy { "✅ Healthy" } else { "❌ Unhealthy" });

    // Get new tokens
    println!("🔍 Fetching tokens from Raydium pools...");
    let tokens = raydium.get_new_tokens().await?;

    println!("✅ Found {} tokens from SOL pools", tokens.len());

    // Display first few tokens with details
    println!("\n📊 Sample tokens found:");
    println!(
        "{:<12} {:<8} {:<50} {:<15} {:<15} {:<15}",
        "Symbol",
        "Name",
        "Mint",
        "Price USD",
        "Liquidity",
        "Volume 24h"
    );
    println!("{}", "-".repeat(120));

    for (i, token) in tokens.iter().take(10).enumerate() {
        println!(
            "{:<12} {:<8} {:<50} ${:<14.6} ${:<14.2} ${:<14.2}",
            token.symbol,
            if token.name.len() > 8 {
                &token.name[..8]
            } else {
                &token.name
            },
            token.mint.to_string(),
            token.metrics.price_usd,
            token.metrics.liquidity_usd,
            token.metrics.volume_24h
        );

        if i == 0 {
            println!("\n🔍 Detailed info for first token:");
            println!("  Source: {:?}", token.source);
            println!("  Discovery time: {}", token.discovery_time);
            println!("  Risk score: {:.2}", token.risk_score);
            println!("  Confidence score: {:.2}", token.confidence_score);
            println!("  Liquidity provider: {:?}", token.liquidity_provider);
            println!("  Price change 24h: {:?}%", token.metrics.price_change_24h);
            println!("  Market cap: {:?}", token.metrics.market_cap);
            println!("  Age hours: {:.2}", token.metrics.age_hours);
            println!("  Verification status:");
            println!("    - Has profile: {}", token.verification_status.has_profile);
            println!("    - Is verified: {}", token.verification_status.is_verified);
            println!("    - Contract verified: {}", token.verification_status.contract_verified);
            println!("");
        }
    }

    if tokens.len() > 10 {
        println!("... and {} more tokens", tokens.len() - 10);
    }

    println!("\n🎯 Summary:");
    println!("  - Total tokens discovered: {}", tokens.len());
    println!("  - All tokens are from SOL liquidity pools");
    println!("  - Source: Raydium v3 API");
    println!("  - Data includes: price, liquidity, volume, and metadata");

    Ok(())
}
