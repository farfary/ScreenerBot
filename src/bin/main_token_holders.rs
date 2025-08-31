/// Token Holder Counter Tool
///
/// This binary counts the number of holders for a specific token mint
/// by querying Solana RPC directly using getProgramAccounts.
///
/// Usage:
///   cargo run --bin main_token_holders -- --mint <MINT_ADDRESS>
///   cargo run --bin main_token_holders -- --mint HZjwdor9NdCBod1ka1AE5TWXzjSHezYsEfjoWom4pump
use screenerbot::{
    global::read_configs,
    logger::{init_file_logging, log, LogTag},
    tokens::holders::get_count_holders,
};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Token Holder Counter Tool");
    println!("============================");

    // Initialize logging
    init_file_logging();
    log(LogTag::System, "INFO", "Token holder counter starting...");

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let mint_address = if args.len() > 2 && args[1] == "--mint" {
        &args[2]
    } else {
        // Default to the specified token
        "HZjwdor9NdCBod1ka1AE5TWXzjSHezYsEfjoWom4pump"
    };

    println!("🎯 Target Token: {}", mint_address);
    println!("📡 Querying Solana RPC for token holders...\n");

    let configs = read_configs().map_err(|e| format!("Failed to read configs: {}", e))?;

    println!("🔗 RPC Endpoints:");
    println!("   Primary: {}", configs.rpc_url);
    println!("   Premium: {}", configs.rpc_url_premium);
    println!();

    // Count holders using the single function
    let start_time = Instant::now();

    println!("📊 Counting token holders...");
    match get_count_holders(mint_address).await {
        Ok(count) => println!("   ✅ Total holders: {}", count),
        Err(e) => println!("   ❌ Error: {}", e),
    }

    let elapsed = start_time.elapsed();
    println!("\n⏱️  Total time: {:?}", elapsed);
    println!("✅ Token holder counting completed!");

    Ok(())
}
