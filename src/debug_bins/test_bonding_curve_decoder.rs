/// Test PumpFun Legacy bonding curve decoder with real pool
use screenerbot::constants::{PUMP_FUN_LEGACY_PROGRAM_ID, SOL_MINT};
use screenerbot::pools::decoders::pumpfun_legacy::PumpFunLegacyDecoder;
use screenerbot::pools::decoders::PoolDecoder;
use screenerbot::pools::fetcher::AccountData;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    // Initialize config and logger
    screenerbot::config::load_config().expect("Failed to load config");
    screenerbot::logger::init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <POOL_ADDRESS> <TOKEN_MINT>", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} Dm8vW6XQYxEbF4hjkLkeh1T23pohGB9Sae4p3G8QZwRP Wxndxj9rG8Y3KsbTjcEnJqrKa91FTcpmjPHnfw3pump", args[0]);
        std::process::exit(1);
    }

    let pool_address = &args[1];
    let token_mint = &args[2];

    println!("═══════════════════════════════════════════════════════════════════════════");
    println!("Testing PumpFun Legacy Bonding Curve Decoder");
    println!("═══════════════════════════════════════════════════════════════════════════");
    println!();
    println!("Pool:  {}", pool_address);
    println!("Token: {}", token_mint);
    println!("Quote: {} (SOL)", SOL_MINT);
    println!();

    // Parse addresses
    let pool_pubkey = Pubkey::from_str(pool_address).expect("Invalid pool address");

    // Get RPC client
    let rpc_url = screenerbot::config::with_config(|cfg| cfg.rpc.urls[0].clone());
    let rpc_client = RpcClient::new_with_commitment(rpc_url, CommitmentConfig::confirmed());

    // Fetch pool account
    println!("Fetching pool account...");
    let pool_account = match rpc_client.get_account(&pool_pubkey) {
        Ok(acc) => acc,
        Err(e) => {
            eprintln!("❌ Failed to fetch pool: {}", e);
            std::process::exit(1);
        }
    };

    println!("✅ Pool fetched:");
    println!("   Owner: {}", pool_account.owner);
    println!("   Size:  {} bytes", pool_account.data.len());
    println!();

    // Pre-load token decimals into cache
    println!("Loading token decimals...");
    let token_pubkey = Pubkey::from_str(token_mint).expect("Invalid token mint");
    match rpc_client.get_account(&token_pubkey) {
        Ok(mint_acc) => {
            if mint_acc.data.len() >= 45 {
                let decimals = mint_acc.data[44];
                println!("✅ Token decimals: {}", decimals);
                // Store in cache
                screenerbot::tokens::decimals::cache(token_mint, decimals);
            }
        }
        Err(e) => {
            eprintln!("⚠️  Could not fetch token mint: {}", e);
        }
    }
    println!();

    // Verify owner
    if pool_account.owner.to_string() != PUMP_FUN_LEGACY_PROGRAM_ID {
        eprintln!("❌ Pool owner mismatch!");
        eprintln!("   Expected: {}", PUMP_FUN_LEGACY_PROGRAM_ID);
        eprintln!("   Got:      {}", pool_account.owner);
        std::process::exit(1);
    }

    // Prepare accounts map for decoder
    let mut accounts = HashMap::new();
    accounts.insert(
        pool_address.to_string(),
        AccountData {
            pubkey: pool_pubkey,
            data: pool_account.data.clone(),
            slot: 0,
            fetched_at: std::time::Instant::now(),
            lamports: pool_account.lamports,
            owner: pool_account.owner,
        },
    );

    println!("─────────────────────────────────────────────────────────────────────────────");
    println!("Testing decode_and_calculate with TOKEN/SOL orientation");
    println!("─────────────────────────────────────────────────────────────────────────────");
    println!();

    // Test TOKEN/SOL
    let result1 = PumpFunLegacyDecoder::decode_and_calculate(&accounts, token_mint, SOL_MINT);

    match result1 {
        Some(price) => {
            println!("✅ SUCCESS!");
            println!();
            println!("Price Result:");
            println!("  Mint:          {}", price.mint);
            println!("  Price (SOL):   {:.15}", price.price_sol);
            println!("  Price (USD):   {:.6}", price.price_usd);
            println!("  Confidence:    {}", price.confidence);
            println!("  Source:        {:?}", price.source_pool);
            println!("  Pool:          {}", price.pool_address);
            println!("  SOL Reserves:  {:.9}", price.sol_reserves);
            println!("  Token Reserves: {:.6}", price.token_reserves);
            println!();
            println!("═══════════════════════════════════════════════════════════════════════════");
            println!(
                "Price in scientific notation: {:.6e} SOL/token",
                price.price_sol
            );
            println!("═══════════════════════════════════════════════════════════════════════════");
        }
        None => {
            println!("❌ FAILED - decode_and_calculate returned None");
            println!();
            println!("Check debug logs above for details.");
            std::process::exit(1);
        }
    }

    println!();
    println!("─────────────────────────────────────────────────────────────────────────────");
    println!("Testing decode_and_calculate with SOL/TOKEN orientation");
    println!("─────────────────────────────────────────────────────────────────────────────");
    println!();

    // Test SOL/TOKEN
    let result2 = PumpFunLegacyDecoder::decode_and_calculate(&accounts, SOL_MINT, token_mint);

    match result2 {
        Some(price) => {
            println!("✅ SUCCESS!");
            println!();
            println!("Price Result:");
            println!("  Price (SOL):   {:.15}", price.price_sol);
            println!("  Price (sci):   {:.6e} SOL/token", price.price_sol);
        }
        None => {
            println!("❌ FAILED - decode_and_calculate returned None");
        }
    }

    println!();
    println!("═══════════════════════════════════════════════════════════════════════════");
    println!("Test Complete");
    println!("═══════════════════════════════════════════════════════════════════════════");
}
