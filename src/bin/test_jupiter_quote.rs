//! Test Jupiter quote functionality with main wallet
//!
//! This test verifies that the Jupiter quote API integration is working correctly.

use screenerbot::swap::dex::JupiterSwap;
use screenerbot::swap::types::*;
use screenerbot::config::Config;
use solana_sdk::signature::{ Keypair, Signer };

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("🚀 Testing Jupiter Quote API Integration");
    println!("=========================================");

    // Load configuration to get wallet details
    let config = Config::load("configs.json")?;
    let wallet_keypair = Keypair::from_base58_string(&config.main_wallet_private);
    let wallet_pubkey = wallet_keypair.pubkey();

    println!("📍 Using wallet: {}", wallet_pubkey);
    println!();
    let config = Config::load("configs.json")?;
    let wallet_keypair = Keypair::from_base58_string(&config.main_wallet_private);
    let wallet_pubkey = wallet_keypair.pubkey();

    println!("📍 Using wallet: {}", wallet_pubkey);
    println!();

    // Create Jupiter swap instance from config
    let jupiter = JupiterSwap::new(config.swap.jupiter.clone());

    // Create test request - 0.001 SOL to USDC
    let request = SwapRequest {
        input_mint: SOL_MINT.to_string(),
        output_mint: USDC_MINT.to_string(),
        amount: 1_000_000, // 0.001 SOL
        slippage_bps: 100, // 1%
        user_public_key: wallet_pubkey.to_string(),
        dex_preference: Some(DexType::Jupiter),
        is_anti_mev: false,
    };

    println!("� Test Parameters:");
    println!("   Input:  {} SOL", (request.amount as f64) / 1_000_000_000.0);
    println!("   Output: USDC");
    println!("   Slippage: {}%", (request.slippage_bps as f64) / 100.0);
    println!("   Anti-MEV: {}", request.is_anti_mev);
    println!();
    println!();

    // Test quote
    println!("💱 Requesting quote from Jupiter...");
    match jupiter.get_quote(&request).await {
        Ok(route) => {
            println!("✅ Jupiter quote successful!");
            println!();
            println!("📊 Quote Details:");
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
                println!("🛤️  Route Plan:");
                for (i, plan) in route.route_plan.iter().enumerate() {
                    println!("   {}. {} ({}%)", i + 1, plan.swap_info.label, plan.percent);
                    println!("      AMM: {}", plan.swap_info.amm_key);
                    println!(
                        "      In:  {} -> Out: {}",
                        plan.swap_info.in_amount,
                        plan.swap_info.out_amount
                    );
                }
            }

            // Calculate effective price
            let input_sol = (route.in_amount.parse::<u64>().unwrap_or(0) as f64) / 1_000_000_000.0;
            let output_usdc = (route.out_amount.parse::<u64>().unwrap_or(0) as f64) / 1_000_000.0;
            if input_sol > 0.0 {
                let price_per_sol = output_usdc / input_sol;
                println!();
                println!("💰 Effective Price: {:.2} USDC per SOL", price_per_sol);
            }

            println!();
            println!("✅ Jupiter integration test PASSED!");
        }
        Err(e) => {
            println!("❌ Jupiter quote failed: {}", e);
            println!("❌ Jupiter integration test FAILED!");
            return Err(e.into());
        }
    }

    Ok(())
}
