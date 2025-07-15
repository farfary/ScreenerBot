//! Test real on-chain swap functionality with main wallet

use screenerbot::swap::{ SwapManager, types::* };
use screenerbot::config::Config;
use screenerbot::rpc_manager::RpcManager;
use screenerbot::trading::transaction_manager::TransactionManager;
use screenerbot::database::Database;
use screenerbot::wallet::WalletTracker;
use std::sync::Arc;
use solana_sdk::signature::{ Keypair, Signer };
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("🚀 REAL ON-CHAIN SWAP TEST");
    println!("==========================");
    println!("⚠️  WARNING: This will execute a real swap with SOL!");
    println!("⚠️  Make sure you have sufficient SOL balance.");
    println!();

    // Load configuration
    let config = Config::load("configs.json")?;

    // Initialize components
    let database = Arc::new(Database::new("test_swap.db")?);
    let rpc_manager = Arc::new(
        RpcManager::new(config.rpc_url.clone(), config.rpc_fallbacks.clone())
    );

    // Create wallet keypair
    let wallet_keypair = Keypair::from_base58_string(&config.main_wallet_private);
    let wallet_pubkey = wallet_keypair.pubkey();

    println!("📍 Wallet Address: {}", wallet_pubkey);

    // Check SOL balance using RPC manager
    let balance_lamports = rpc_manager.get_balance(&wallet_pubkey).await?;
    let balance_sol = (balance_lamports as f64) / 1_000_000_000.0;

    println!("💰 SOL Balance: {:.9} SOL ({} lamports)", balance_sol, balance_lamports);

    if balance_sol < 0.01 {
        println!("❌ Insufficient SOL balance for testing. Need at least 0.01 SOL.");
        return Err("Insufficient balance".into());
    }

    let wallet_tracker = Arc::new(WalletTracker::new(config.clone(), database.clone())?);

    let transaction_manager = Arc::new(
        TransactionManager::new(
            config.trading.transaction_manager.clone(),
            database.clone(),
            wallet_tracker
        )
    );

    // Create swap manager with legacy transaction enabled
    let mut swap_config = config.swap.clone();
    swap_config.jupiter.as_legacy_transaction = true; // Force legacy transactions to avoid versioned tx errors

    let swap_manager = SwapManager::new(swap_config, rpc_manager.clone(), transaction_manager);

    println!();
    println!("🔄 Testing Small Real Swap: 0.001 SOL → USDC");
    println!("═══════════════════════════════════════════════");

    let swap_amount = 1_000_000; // 0.001 SOL in lamports
    let swap_amount_sol = (swap_amount as f64) / 1_000_000_000.0;

    println!("📊 Swap Details:");
    println!("   💱 From: SOL");
    println!("   💱 To: USDC");
    println!("   💰 Amount: {:.9} SOL ({} lamports)", swap_amount_sol, swap_amount);
    println!("   🎯 Max Slippage: {}%", config.swap.max_slippage * 100.0);
    println!();

    // Create swap request
    let swap_request = SwapRequest {
        input_mint: SOL_MINT.to_string(),
        output_mint: USDC_MINT.to_string(),
        amount: swap_amount,
        slippage_bps: (config.swap.max_slippage * 10000.0) as u32,
        user_public_key: wallet_pubkey.to_string(),
        dex_preference: Some(DexType::Jupiter),
        is_anti_mev: config.swap.is_anti_mev,
    };

    // Step 1: Get quotes
    println!("📡 Step 1: Getting quotes from all DEXes...");
    match swap_manager.get_best_quote(&swap_request).await {
        Ok(route) => {
            println!("✅ Best quote found:");
            println!("   🔄 DEX: {}", route.dex);
            println!("   📈 Input: {} lamports ({:.9} SOL)", route.in_amount, swap_amount_sol);
            println!("   📉 Expected Output: {} micro-USDC", route.out_amount);
            println!("   💥 Price Impact: {}%", route.price_impact_pct);
            println!("   🛣️  Route Steps: {}", route.route_plan.len());

            for (i, step) in route.route_plan.iter().enumerate() {
                println!(
                    "      {}. {} via {}",
                    i + 1,
                    step.swap_info.label,
                    &step.swap_info.amm_key[..8]
                );
            }

            println!();
            print!("⚠️  Continue with REAL swap execution? (y/N): ");
            use std::io::{ self, Write };
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if input.trim().to_lowercase() != "y" {
                println!("❌ Swap cancelled by user.");
                return Ok(());
            }

            // Step 2: Execute the swap
            println!();
            println!("🚀 Step 2: Executing REAL swap...");
            println!("⏳ This may take 30-60 seconds...");

            let start_time = std::time::Instant::now();

            match swap_manager.execute_swap(swap_request, &wallet_keypair).await {
                Ok(result) => {
                    let execution_time = start_time.elapsed();

                    println!();
                    println!("🎉 SWAP EXECUTED SUCCESSFULLY!");
                    println!("════════════════════════════════");
                    println!(
                        "✅ Transaction Signature: {}",
                        result.signature.unwrap_or("N/A".to_string())
                    );
                    println!("🔄 DEX Used: {}", result.dex_used);
                    println!("📈 Input Amount: {} lamports", result.input_amount);
                    println!("📉 Output Amount: {} micro-USDC", result.output_amount);
                    println!("💥 Actual Price Impact: {:.4}%", result.price_impact);
                    println!(
                        "💸 Transaction Fee: {} lamports ({:.9} SOL)",
                        result.fee_lamports,
                        (result.fee_lamports as f64) / 1_000_000_000.0
                    );
                    println!("⏱️  Execution Time: {:.2}s", execution_time.as_secs_f64());

                    if let Some(block_height) = result.block_height {
                        println!("🧱 Block Height: {}", block_height);
                    }

                    // Verify new balance
                    println!();
                    println!("🔍 Verifying post-swap balance...");

                    // Wait a bit for transaction to settle
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

                    let new_balance_lamports = rpc_manager.get_balance(&wallet_pubkey).await?;
                    let new_balance_sol = (new_balance_lamports as f64) / 1_000_000_000.0;
                    let balance_diff = balance_sol - new_balance_sol;

                    println!("💰 New SOL Balance: {:.9} SOL", new_balance_sol);
                    println!("📉 SOL Spent: {:.9} SOL", balance_diff);

                    // Check USDC balance (if we have an SPL token account)
                    match get_usdc_balance(&rpc_manager, &wallet_pubkey).await {
                        Ok(usdc_balance) => {
                            println!("💵 USDC Balance: {:.6} USDC", usdc_balance);
                        }
                        Err(e) => {
                            println!("⚠️  Could not check USDC balance: {}", e);
                        }
                    }

                    println!();
                    println!("✅ Real swap test completed successfully!");
                }
                Err(e) => {
                    let execution_time = start_time.elapsed();
                    println!();
                    println!("❌ SWAP FAILED!");
                    println!("═══════════════");
                    println!("Error: {}", e);
                    println!("Execution time: {:.2}s", execution_time.as_secs_f64());

                    // Check if balance changed (transaction might have partially succeeded)
                    let new_balance_lamports = rpc_manager.get_balance(&wallet_pubkey).await?;
                    let new_balance_sol = (new_balance_lamports as f64) / 1_000_000_000.0;

                    if (balance_sol - new_balance_sol).abs() > 0.000001 {
                        println!(
                            "⚠️  Balance changed: {:.9} → {:.9} SOL",
                            balance_sol,
                            new_balance_sol
                        );
                        println!("⚠️  Transaction may have been partially processed");
                    }

                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            println!("❌ Failed to get quote: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

async fn get_usdc_balance(
    rpc_manager: &Arc<RpcManager>,
    wallet_pubkey: &Pubkey
) -> Result<f64, Box<dyn std::error::Error>> {
    use spl_associated_token_account::get_associated_token_address;

    let usdc_mint = Pubkey::from_str(USDC_MINT)?;
    let usdc_ata = get_associated_token_address(wallet_pubkey, &usdc_mint);

    match rpc_manager.get_token_account_balance(&usdc_ata).await {
        Ok(balance) => {
            let amount = balance.ui_amount.unwrap_or(0.0);
            Ok(amount)
        }
        Err(_) => {
            // Account doesn't exist or other error
            Ok(0.0)
        }
    }
}
