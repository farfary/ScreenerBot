/// Debug Raydium LP Extraction Issues
/// Investigates why Raydium pool decoding is failing
/// Usage: cargo run --bin debug_raydium_lp_extraction

use screenerbot::tokens::dexscreener::init_dexscreener_api;
use screenerbot::rpc::get_rpc_client;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    println!("🔍 Debug Raydium LP Extraction Issues");
    println!("{}", "=".repeat(60));

    // Initialize APIs
    if let Err(e) = init_dexscreener_api().await {
        println!("❌ Failed to initialize DexScreener API: {}", e);
        return;
    }

    // Test cases that failed in comprehensive test
    let test_cases = vec![
        ("SOL", "So11111111111111111111111111111111111111112", "G8LqPHYA"),
        ("USDC", "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "HnhpJPJg"),
        ("BONK", "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263", "GtKKKs3y")
    ];

    let client = get_rpc_client();

    for (name, token, pool_short) in test_cases {
        println!("\n🧪 Testing {}", name);
        println!("Token: {}", token);
        println!("Pool (partial): {}", pool_short);

        // First, let's manually inspect the pool account
        // We need to find the full pool address - let's check what DexScreener returns

        // For now, let's check what we can find about these tokens
        match screenerbot::tokens::lp_lock::check_lp_lock_status(token).await {
            Ok(analysis) => {
                println!("✅ Analysis completed");
                if let Some(pool_addr) = &analysis.pool_address {
                    println!("📍 Pool address: {}", pool_addr);

                    // Get the actual pool account data
                    if let Ok(pool_pubkey) = Pubkey::from_str(pool_addr) {
                        match client.get_account(&pool_pubkey).await {
                            Ok(account) => {
                                println!("✅ Retrieved pool account data");
                                println!("  Owner: {}", account.owner);
                                println!("  Data length: {} bytes", account.data.len());
                                println!("  Lamports: {}", account.lamports);

                                // Identify the program type
                                let owner_str = account.owner.to_string();
                                let program_type = match owner_str.as_str() {
                                    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" =>
                                        "Raydium Legacy AMM",
                                    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" =>
                                        "Raydium CPMM",
                                    "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM" =>
                                        "Raydium CLMM",
                                    _ => "Unknown/Other",
                                };

                                println!("  Program type: {}", program_type);

                                // Show first 100 bytes of data for analysis
                                println!("  First 100 bytes of data:");
                                let preview_len = std::cmp::min(100, account.data.len());
                                for (i, byte) in account.data[..preview_len].iter().enumerate() {
                                    if i % 32 == 0 {
                                        print!("\n  {:3}: ", i);
                                    }
                                    print!("{:02x} ", byte);
                                }
                                println!();

                                // Try to extract pubkeys at common offsets
                                println!("  Potential pubkeys at common offsets:");
                                let offsets = vec![8, 40, 72, 104, 136, 168, 200];
                                for offset in offsets {
                                    if offset + 32 <= account.data.len() {
                                        let pubkey_bytes = &account.data[offset..offset + 32];
                                        if let Ok(pubkey) = Pubkey::try_from(pubkey_bytes) {
                                            println!("    Offset {}: {}", offset, pubkey);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                println!("❌ Failed to get pool account: {}", e);
                            }
                        }
                    } else {
                        println!("❌ Invalid pool address format");
                    }
                } else {
                    println!("❌ No pool address in analysis");
                }

                println!("Analysis details:");
                for detail in &analysis.details {
                    println!("  - {}", detail);
                }
            }
            Err(e) => {
                println!("❌ Analysis failed: {}", e);
            }
        }

        println!("{}", "-".repeat(60));
    }

    println!("\n🔧 Debugging Recommendations:");
    println!("1. Check if Raydium program IDs are up to date");
    println!("2. Verify pool account data structure matches expectations");
    println!("3. Update LP mint extraction offsets if needed");
    println!("4. Consider using existing Raydium decoders from pools module");
}
