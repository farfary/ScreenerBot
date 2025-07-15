//! Test Jupiter swap fixes with main wallet

use screenerbot::swap::{ SwapManager, types::* };
use screenerbot::config::Config;
use screenerbot::rpc_manager::RpcManager;
use screenerbot::trading::transaction_manager::TransactionManager;
use screenerbot::database::Database;
use screenerbot::wallet::WalletTracker;
use std::sync::Arc;
use solana_sdk::signature::{ Keypair, Signer };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("🧪 Testing Jupiter Swap Fixes");
    println!("===============================");

    // Load configuration
    let config = Config::load("configs.json")?;

    println!("📋 Configuration:");
    println!("   Jupiter URL: {}", config.swap.jupiter.base_url);
    println!("   Max slippage: {}%", config.swap.max_slippage * 100.0);
    println!();

    // Create components
    let rpc_manager = Arc::new(
        RpcManager::new(config.rpc_url.clone(), config.rpc_fallbacks.clone())
    );

    let database = Arc::new(Database::new("test_jupiter.db")?);
    let wallet_tracker = Arc::new(WalletTracker::new(config.clone(), database.clone())?);

    let transaction_manager = Arc::new(
        TransactionManager::new(
            config.trading.transaction_manager.clone(),
            database,
            wallet_tracker
        )
    );

    // Create swap manager
    let swap_manager = SwapManager::new(config.swap.clone(), rpc_manager, transaction_manager);

    // Test wallet from config
    let test_keypair = Keypair::from_base58_string(&config.main_wallet_private);

    println!("💱 Testing Small Swap Quote (0.001 SOL → USDC)");
    println!("────────────────────────────────────────────────");

    let request = SwapRequest {
        input_mint: SOL_MINT.to_string(),
        output_mint: USDC_MINT.to_string(),
        amount: 1_000_000, // 0.001 SOL
        slippage_bps: 100, // 1%
        user_public_key: test_keypair.pubkey().to_string(),
        dex_preference: Some(DexType::Jupiter),
        is_anti_mev: false,
    };

    println!("📡 Getting quote from Jupiter...");
    match swap_manager.get_best_quote(&request).await {
        Ok(route) => {
            println!("✅ Quote successful!");
            println!("   📊 DEX: {}", route.dex);
            println!("   📈 Input: {} lamports", route.in_amount);
            println!("   📉 Output: {} micro-USDC", route.out_amount);
            println!("   💥 Price Impact: {}%", route.price_impact_pct);
            println!("   🛤️  Route Hops: {}", route.route_plan.len());

            for (i, plan) in route.route_plan.iter().enumerate() {
                println!(
                    "      {}. {} ({})",
                    i + 1,
                    plan.swap_info.label,
                    plan.swap_info.amm_key[..8].to_string() + "..."
                );
            }
        }
        Err(e) => {
            println!("❌ Quote failed: {}", e);
            return Err(e.into());
        }
    }

    println!();
    println!("💱 Testing Micro Swap Quote (0.0001 SOL → USDC)");
    println!("──────────────────────────────────────────────────");

    let micro_request = SwapRequest {
        input_mint: SOL_MINT.to_string(),
        output_mint: USDC_MINT.to_string(),
        amount: 100_000, // 0.0001 SOL
        slippage_bps: 100, // 1%
        user_public_key: test_keypair.pubkey().to_string(),
        dex_preference: Some(DexType::Jupiter),
        is_anti_mev: false,
    };

    println!("📡 Getting micro quote from Jupiter...");
    match swap_manager.get_best_quote(&micro_request).await {
        Ok(route) => {
            println!("✅ Micro quote successful!");
            println!("   📊 DEX: {}", route.dex);
            println!("   📈 Input: {} lamports", route.in_amount);
            println!("   📉 Output: {} micro-USDC", route.out_amount);
            println!("   💥 Price Impact: {}%", route.price_impact_pct);
            println!("   🛤️  Route Hops: {}", route.route_plan.len());
        }
        Err(e) => {
            println!("❌ Micro quote failed: {}", e);
            // Don't return error for micro quote as it might legitimately fail
        }
    }

    println!();
    println!("🎉 Jupiter fix test completed!");

    Ok(())
}
