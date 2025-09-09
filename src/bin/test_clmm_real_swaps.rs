/// Test real swaps on specific CLMM pool: WSOL-CANDY
/// Pool: HWek4aDnvgbBiDAGsJHN7JERv8sWbRnRa51KeoDff7xv
/// Token: 5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t (CANDY)
/// Program: CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK (Raydium CLMM)

use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::rpc::get_rpc_client;
use screenerbot::pools::decoders::raydium_clmm::RaydiumClmmDecoder;
use screenerbot::pools::swap::{ SwapBuilder, SwapDirection };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

const TARGET_POOL: &str = "HWek4aDnvgbBiDAGsJHN7JERv8sWbRnRa51KeoDff7xv";
const TARGET_TOKEN: &str = "5DhEM7PZrPVPfA4UK3tcNxxZ8UGwc6yFYwpAXB14uw2t"; // CANDY
const CLMM_PROGRAM: &str = "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK";
const SOL_MINT: &str = "So11111111111111111111111111111111111111112";

#[derive(Parser, Debug)]
#[command(name = "test_clmm_real_swaps", about = "Test real swaps on CLMM pool")]
struct Args {
    /// Amount of SOL to trade (default: 0.001)
    #[arg(short, long, default_value = "0.001")]
    amount: f64,

    /// Enable all debugging
    #[arg(long)]
    debug: bool,

    /// Dry run (no actual transactions)
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    if args.debug {
        set_cmd_args(
            vec![
                "test_clmm_real_swaps".to_string(),
                "--debug-pool-decoders".to_string(),
                "--debug-rpc".to_string()
            ]
        );
    }

    println!("🚀 Testing Real CLMM Swaps");
    println!("Pool: {}", TARGET_POOL);
    println!("Token: {} (CANDY)", TARGET_TOKEN);
    println!("Program: {} (Raydium CLMM)", CLMM_PROGRAM);
    println!("Amount: {} SOL", args.amount);
    println!("Dry Run: {}", args.dry_run);
    println!("{}", "=".repeat(80));

    // Step 1: Fetch and decode pool account
    println!("\n🔍 Step 1: Fetching Pool Account");
    let rpc_client = get_rpc_client();
    let pool_pubkey = Pubkey::from_str(TARGET_POOL)?;

    let pool_account = match rpc_client.get_account(&pool_pubkey).await {
        Ok(account) => account,
        Err(e) => {
            println!("❌ Failed to fetch pool account: {}", e);
            return Ok(());
        }
    };

    println!("✅ Pool account fetched: {} bytes", pool_account.data.len());

    // Step 2: Test CLMM decoder
    println!("\n🔬 Step 2: Testing CLMM Decoder");

    // Create accounts map for decoder
    let mut accounts = std::collections::HashMap::new();
    let account_data = screenerbot::pools::AccountData {
        pubkey: pool_pubkey,
        owner: Pubkey::from_str(CLMM_PROGRAM)?,
        data: pool_account.data.clone(),
        lamports: pool_account.lamports,
        slot: 0,
        fetched_at: std::time::Instant::now(),
    };
    accounts.insert(TARGET_POOL.to_string(), account_data);

    let pool_info = match RaydiumClmmDecoder::extract_pool_data(&accounts) {
        Some(info) => {
            println!("✅ CLMM decoder succeeded");
            println!("   Token Mint 0: {}", info.token_mint_0);
            println!("   Token Mint 1: {}", info.token_mint_1);
            println!("   Liquidity: {}", info.liquidity);
            println!("   Current Tick: {}", info.tick_current);
            println!("   Sqrt Price X64: {}", info.sqrt_price_x64);

            // Verify this is the correct pool
            if
                (info.token_mint_0 == SOL_MINT && info.token_mint_1 == TARGET_TOKEN) ||
                (info.token_mint_1 == SOL_MINT && info.token_mint_0 == TARGET_TOKEN)
            {
                println!("✅ Confirmed WSOL-CANDY pool");
            } else {
                println!("❌ Pool token mismatch!");
                println!("   Expected: {} and {}", SOL_MINT, TARGET_TOKEN);
                println!("   Found: {} and {}", info.token_mint_0, info.token_mint_1);
            }

            info
        }
        None => {
            println!("❌ CLMM decoder failed");
            return Ok(());
        }
    };

    // Step 3: Execute BUY swap (SOL -> CANDY)
    println!("\n💰 Step 3: Executing BUY Swap (SOL -> CANDY)");

    println!("� Executing buy swap...");
    let buy_result = SwapBuilder::new()
        .pool_address(TARGET_POOL)?
        .token_mint(TARGET_TOKEN)?
        .amount(args.amount)
        .direction(SwapDirection::Buy)
        .slippage_percent(5.0) // 5% slippage
        .dry_run(args.dry_run)
        .execute().await;

    match buy_result {
        Ok(result) => {
            if result.success {
                println!("✅ Buy swap completed!");
                println!(
                    "   Transaction: {}",
                    result.signature.map(|s| s.to_string()).unwrap_or("DRY_RUN".to_string())
                );
                println!("   Input: {:.6} SOL", result.params.input_amount);
                println!("   Output: {:.6} CANDY", result.params.expected_output);
                println!("   Minimum Output: {:.6} CANDY", result.params.minimum_output);
            } else {
                println!("❌ Buy swap failed: {:?}", result.error);
                if !args.dry_run {
                    return Ok(()); // Don't continue if real swap failed
                }
            }
        }
        Err(e) => {
            println!("❌ Buy swap error: {:?}", e);
            if !args.dry_run {
                return Ok(()); // Don't continue if real swap failed
            }
        }
    }

    // Step 4: Execute SELL swap (CANDY -> SOL)
    println!("\n💸 Step 4: Executing SELL Swap (CANDY -> SOL)");

    // For sell, we use a small amount of CANDY tokens
    let sell_amount = args.amount * 1000.0; // Approximate CANDY amount based on typical price

    println!("🔄 Executing sell swap...");
    let sell_result = SwapBuilder::new()
        .pool_address(TARGET_POOL)?
        .token_mint(TARGET_TOKEN)?
        .amount(sell_amount)
        .direction(SwapDirection::Sell)
        .slippage_percent(5.0) // 5% slippage
        .dry_run(args.dry_run)
        .execute().await;

    match sell_result {
        Ok(result) => {
            if result.success {
                println!("✅ Sell swap completed!");
                println!(
                    "   Transaction: {}",
                    result.signature.map(|s| s.to_string()).unwrap_or("DRY_RUN".to_string())
                );
                println!("   Input: {:.6} CANDY", result.params.input_amount);
                println!("   Output: {:.6} SOL", result.params.expected_output);
                println!("   Minimum Output: {:.6} SOL", result.params.minimum_output);
            } else {
                println!("❌ Sell swap failed: {:?}", result.error);
            }
        }
        Err(e) => {
            println!("❌ Sell swap error: {:?}", e);
        }
    }

    // Step 5: Summary
    println!("\n🎉 Test Summary");
    println!("   Pool: {} ✅", TARGET_POOL);
    println!("   CLMM Decoder: ✅");
    println!("   Buy Swap: Attempted");
    println!("   Sell Swap: Attempted");
    if args.dry_run {
        println!("   Mode: DRY RUN (no real transactions)");
    } else {
        println!("   Mode: LIVE TRADING");
    }

    Ok(())
}
