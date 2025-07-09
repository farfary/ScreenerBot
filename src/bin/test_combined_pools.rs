use screenerbot::helpers::*;

fn main() -> anyhow::Result<()> {
    // Test token (the one from your example)
    let token_mint = "42orNZHxsH1SNUZX87btNs6LiAoXdqj1RRUgRxgppump";

    println!("🧪 Testing pool fetching for token: {}", token_mint);
    println!("{}", "─".repeat(60));

    // Test DexScreener source
    println!("\n📊 Testing DexScreener only:");
    match fetch_dexscreener_pools(token_mint) {
        Ok(pools) => {
            println!("✅ Found {} pools", pools.len());
            for pool in pools.iter().take(3) {
                println!("  - {} [{}]", pool.address, pool.source);
            }
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    println!("\n📊 Testing combined approach (DexScreener only):");
    match fetch_combined_pools(token_mint) {
        Ok(pools) => {
            println!("✅ Total pools found: {}", pools.len());
            println!("\nTop 5 pools by liquidity:");
            for (i, pool) in pools.iter().take(5).enumerate() {
                println!("  {}. {} [{}]", i + 1, pool.address, pool.source);
                if let Some(name) = &pool.name {
                    println!("     Name: {}", name);
                }
                if let Some(liq) = pool.liquidity_usd {
                    println!("     Liquidity: ${:.2}", liq);
                }
                if let Some(vol) = pool.volume_24h_usd {
                    println!("     Volume 24h: ${:.2}", vol);
                }
                println!();
            }
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    println!("\n🔄 Testing updated fetch_solana_pairs function:");
    match fetch_solana_pairs(token_mint) {
        Ok(pubkeys) => {
            println!("✅ Found {} valid Pubkeys", pubkeys.len());
            for (i, pubkey) in pubkeys.iter().take(3).enumerate() {
                println!("  {}. {}", i + 1, pubkey);
            }
        }
        Err(e) => println!("❌ Error: {}", e),
    }

    Ok(())
}
