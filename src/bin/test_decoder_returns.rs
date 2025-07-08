use anyhow::Result;
use screenerbot::pools::decoder::decode_any_pool;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn main() -> Result<()> {
    let rpc = RpcClient::new("https://api.mainnet-beta.solana.com");

    // Example pool addresses for testing (these are real addresses but might not exist)
    let test_pools = [
        // You can add real pool addresses here for testing
        "11111111111111111111111111111111", // placeholder
    ];

    for pool_str in &test_pools {
        match Pubkey::from_str(pool_str) {
            Ok(pool_pk) => {
                println!("Testing pool: {}", pool_pk);
                match decode_any_pool(&rpc, &pool_pk) {
                    Ok((base, quote, base_mint, quote_mint)) => {
                        println!("✅ Pool decoded successfully:");
                        println!("   Base amount:  {}", base);
                        println!("   Quote amount: {}", quote);
                        println!("   Base mint:    {}", base_mint);
                        println!("   Quote mint:   {}", quote_mint);
                        println!();
                    }
                    Err(e) => {
                        println!("❌ Failed to decode pool: {}", e);
                        println!();
                    }
                }
            }
            Err(e) => {
                println!("❌ Invalid pubkey {}: {}", pool_str, e);
            }
        }
    }

    println!("All decoders now return (u64, u64) representing base and quote amounts in the pool.");
    Ok(())
}
