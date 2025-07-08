use screenerbot::helpers::*;

fn main() -> anyhow::Result<()> {
    // Test token (the one from your example)
    let token_mint = "42orNZHxsH1SNUZX87btNs6LiAoXdqj1RRUgRxgppump";

    println!("🧪 Testing combined pool fetching for token: {}", token_mint);
    println!("{}", "─".repeat(60));

    // Test individual sources first
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

    println!("\n🦎 Testing GeckoTerminal with different sorts:");
    let sorts = ["h24_volume_usd_desc", "h24_tx_count_desc", "h24_volume_usd_liquidity_desc"];

    for sort in &sorts {
        println!("\n  Sort: {}", sort);
        match fetch_gecko_pools(token_mint, sort) {
            Ok(pools) => {
                println!("  ✅ Found {} pools", pools.len());
                for pool in pools.iter().take(2) {
                    println!(
                        "    - {} [{}] {}",
                        pool.address,
                        pool.source,
                        pool.name.as_ref().unwrap_or(&"Unknown".to_string())
                    );
                }
            }
            Err(e) => println!("  ❌ Error: {}", e),
        }
    }

    println!("\n🔗 Testing combined approach:");
    match fetch_combined_pools(token_mint) {
        Ok(pools) => {
            println!("✅ Total unique pools found: {}", pools.len());
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
