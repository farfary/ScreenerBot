use screenerbot::helpers::*;
use std::env;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: {} <token_mint>", args[0]);
        println!("Example: {} So11111111111111111111111111111111111111112", args[0]);
        return Ok(());
    }

    let token_mint = &args[1];

    println!("🔍 Fetching pools for token: {}", token_mint);
    println!("{}", "═".repeat(80));

    match fetch_combined_pools(token_mint) {
        Ok(pools) => {
            println!(
                "✅ Found {} unique pools from both DexScreener and GeckoTerminal",
                pools.len()
            );
            println!("\nTop 10 pools sorted by liquidity:");
            println!("{}", "─".repeat(80));

            for (i, pool) in pools.iter().take(10).enumerate() {
                println!("{}. Address: {}", i + 1, pool.address);
                if let Some(name) = &pool.name {
                    println!("   Name: {}", name);
                }
                println!("   Source: {}", pool.source);
                if let Some(liq) = pool.liquidity_usd {
                    println!("   Liquidity: ${:.2}", liq);
                }
                if let Some(vol) = pool.volume_24h_usd {
                    println!("   Volume 24h: ${:.2}", vol);
                }
                if let Some(txs) = pool.tx_count_24h {
                    println!("   Transactions 24h: {}", txs);
                }
                println!();
            }

            // Show source breakdown
            let dex_count = pools
                .iter()
                .filter(|p| p.source == "dexscreener")
                .count();
            let gecko_count = pools
                .iter()
                .filter(|p| p.source == "geckoterminal")
                .count();

            println!("📊 Source breakdown:");
            println!("   DexScreener: {} pools", dex_count);
            println!("   GeckoTerminal: {} pools", gecko_count);
        }
        Err(e) => {
            println!("❌ Error fetching pools: {}", e);
        }
    }

    Ok(())
}
