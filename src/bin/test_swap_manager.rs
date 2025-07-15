//! Test Jupiter swap integration with main wallet
//!
//! This test creates a real swap manager and tests the complete quote flow.

use screenerbot::swap::SwapManager;
use screenerbot::swap::types::*;
use screenerbot::config::Config;
use screenerbot::rpc_manager::RpcManager;
use screenerbot::trading::transaction_manager::TransactionManager;
use screenerbot::database::Database;
use screenerbot::wallet::WalletTracker;
use std::sync::Arc;
use solana_sdk::signature::{ Keypair, Signer };
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("🚀 Testing Complete Jupiter Swap Integration");
    println!("============================================");

    // Load configuration
    let config = Config::load("configs.json").expect("Failed to load config");

    println!("📋 Configuration:");
    println!("   Swap enabled: {}", config.swap.enabled);
    println!("   Jupiter enabled: {}", config.swap.jupiter.enabled);
    println!("   Jupiter URL: {}", config.swap.jupiter.base_url);
    println!("   Max slippage: {}%", config.swap.max_slippage * 100.0);
    println!();

    // Create RPC manager
    let rpc_manager = Arc::new(
        RpcManager::new(config.rpc_url.clone(), config.rpc_fallbacks.clone())
    );

    // Create database
    let database = Arc::new(Database::new(&config.database.path)?);

    // Create wallet tracker
    let wallet_tracker = Arc::new(WalletTracker::new(config.clone(), database.clone())?);

    // Create transaction manager
    let transaction_manager = Arc::new(
        TransactionManager::new(
            config.trading.transaction_manager.clone(),
            database.clone(),
            wallet_tracker
        )
    );

    // Create swap manager
    let swap_manager = SwapManager::new(config.swap.clone(), rpc_manager, transaction_manager);

    // Get wallet keypair from config
    let wallet_keypair = Keypair::from_base58_string(&config.main_wallet_private);
    let wallet_pubkey = wallet_keypair.pubkey();

    println!("📍 Using wallet: {}", wallet_pubkey);
    println!();

    // Create test request - smaller amount for safety
    let request = SwapRequest {
        input_mint: SOL_MINT.to_string(), // SOL
        output_mint: USDC_MINT.to_string(), // USDC
        amount: 1_000_000, // 0.001 SOL (1M lamports) - very small amount for testing
        slippage_bps: 100, // 1%
        user_public_key: wallet_pubkey.to_string(),
        dex_preference: Some(DexType::Jupiter),
        is_anti_mev: true,
    };

    println!("💱 Test Parameters:");
    println!("   Input:  {} SOL", (request.amount as f64) / 1_000_000_000.0);
    println!("   Output: USDC");
    println!("   Slippage: {}%", (request.slippage_bps as f64) / 100.0);
    println!("   Anti-MEV: {}", request.is_anti_mev);
    println!("   DEX preference: {:?}", request.dex_preference);
    println!();

    // Test quote through SwapManager
    println!("🔍 Getting quote through SwapManager...");
    match swap_manager.get_best_quote(&request).await {
        Ok(route) => {
            println!("✅ SwapManager quote successful!");
            println!();
            println!("📊 Best Route Details:");
            println!("   DEX:           {}", route.dex);
            println!(
                "   Input Amount:  {} SOL",
                (route.in_amount.parse::<u64>().unwrap_or(0) as f64) / 1_000_000_000.0
            );
            println!(
                "   Output Amount: {} USDC",
                (route.out_amount.parse::<u64>().unwrap_or(0) as f64) / 1_000_000.0
            );
            println!("   Price Impact:  {}%", route.price_impact_pct);
            println!("   Slippage:      {}%", (route.slippage_bps as f64) / 100.0);
            println!("   Routes:        {} hop(s)", route.route_plan.len());

            if let Some(context_slot) = route.context_slot {
                println!("   Context Slot:  {}", context_slot);
            }

            if let Some(time_taken) = route.time_taken {
                println!("   Quote Time:    {:.3}s", time_taken);
            }

            // Display route plan
            if !route.route_plan.is_empty() {
                println!();
                println!("🛤️  Route Details:");
                for (i, plan) in route.route_plan.iter().enumerate() {
                    println!("   {}. {} ({}%)", i + 1, plan.swap_info.label, plan.percent);
                    if !plan.swap_info.amm_key.is_empty() {
                        println!("      AMM: {}", plan.swap_info.amm_key);
                    }
                    if !plan.swap_info.fee_amount.is_empty() && plan.swap_info.fee_amount != "0" {
                        println!("      Fee: {} {}", plan.swap_info.fee_amount, if
                            plan.swap_info.fee_mint == SOL_MINT
                        {
                            "SOL"
                        } else {
                            "tokens"
                        });
                    }
                }
            }

            // Calculate effective price
            let input_sol = (route.in_amount.parse::<u64>().unwrap_or(0) as f64) / 1_000_000_000.0;
            let output_usdc = (route.out_amount.parse::<u64>().unwrap_or(0) as f64) / 1_000_000.0;
            if input_sol > 0.0 {
                let price_per_sol = output_usdc / input_sol;
                println!();
                println!("💰 Effective Price: {:.2} USDC per SOL", price_per_sol);

                // Compare with amount threshold
                let threshold_usdc =
                    (route.other_amount_threshold.parse::<u64>().unwrap_or(0) as f64) / 1_000_000.0;
                let slippage_amount = output_usdc - threshold_usdc;
                let slippage_percent = if output_usdc > 0.0 {
                    (slippage_amount / output_usdc) * 100.0
                } else {
                    0.0
                };
                println!(
                    "🎯 Slippage Protection: {:.6} USDC ({:.3}%)",
                    slippage_amount,
                    slippage_percent
                );
            }

            println!();
            println!("✅ Jupiter SwapManager integration test PASSED!");
            println!();
            println!("⚠️  NOTE: This was only a quote test, no actual swap was executed.");
            println!(
                "   To execute a real swap, ensure you have sufficient SOL balance and call execute_swap()"
            );
        }
        Err(e) => {
            println!("❌ SwapManager quote failed: {}", e);
            println!("❌ Jupiter SwapManager integration test FAILED!");
            return Err(e.into());
        }
    }

    Ok(())
}
