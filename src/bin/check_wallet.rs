//! Check main wallet balance and basic info

use screenerbot::config::Config;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::{ Keypair, Signer };
use std::str::FromStr;
use spl_associated_token_account::get_associated_token_address;
use solana_sdk::pubkey::Pubkey;

const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    println!("🔍 MAIN WALLET BALANCE CHECK");
    println!("============================");

    // Load configuration
    let config = Config::load("configs.json")?;

    // Create RPC client
    let rpc_client = RpcClient::new(&config.rpc_url);

    // Create wallet keypair from config
    let wallet_keypair = Keypair::from_base58_string(&config.main_wallet_private);
    let wallet_pubkey = wallet_keypair.pubkey();

    println!("📍 Main Wallet Address: {}", wallet_pubkey);
    println!("🔗 RPC Endpoint: {}", config.rpc_url);
    println!();

    // Check SOL balance
    println!("💰 Checking SOL balance...");
    match rpc_client.get_balance(&wallet_pubkey) {
        Ok(balance_lamports) => {
            let balance_sol = (balance_lamports as f64) / 1_000_000_000.0;
            println!("✅ SOL Balance: {:.9} SOL ({} lamports)", balance_sol, balance_lamports);

            if balance_sol >= 0.01 {
                println!("✅ Sufficient balance for testing (≥0.01 SOL)");
            } else if balance_sol >= 0.001 {
                println!("⚠️  Minimal balance for testing (≥0.001 SOL)");
            } else {
                println!("❌ Insufficient balance for testing (<0.001 SOL)");
            }
        }
        Err(e) => {
            println!("❌ Failed to get SOL balance: {}", e);
        }
    }

    // Check USDC balance
    println!();
    println!("💵 Checking USDC balance...");
    let usdc_mint = Pubkey::from_str(USDC_MINT)?;
    let usdc_ata = get_associated_token_address(&wallet_pubkey, &usdc_mint);

    match rpc_client.get_token_account_balance(&usdc_ata) {
        Ok(balance) => {
            let amount = balance.ui_amount.unwrap_or(0.0);
            println!("✅ USDC Balance: {:.6} USDC", amount);
            println!("📍 USDC Token Account: {}", usdc_ata);
        }
        Err(_) => {
            println!("ℹ️  No USDC token account found (balance: 0)");
            println!("📍 USDC ATA would be: {}", usdc_ata);
        }
    }

    // Check recent transaction count
    println!();
    println!("📊 Checking recent activity...");
    match rpc_client.get_signatures_for_address(&wallet_pubkey) {
        Ok(signatures) => {
            println!("✅ Found {} recent transactions", signatures.len());
            if signatures.len() > 0 {
                println!("🕐 Most recent: {}", signatures[0].signature);
            }
        }
        Err(e) => {
            println!("⚠️  Could not fetch recent transactions: {}", e);
        }
    }

    // Test RPC connectivity to fallbacks
    println!();
    println!("🌐 Testing RPC connectivity...");

    for (i, fallback_url) in config.rpc_fallbacks.iter().enumerate() {
        let fallback_client = RpcClient::new(fallback_url);
        match fallback_client.get_health() {
            Ok(_) => {
                println!("✅ Fallback RPC {} is healthy: {}", i + 1, fallback_url);
            }
            Err(e) => {
                println!("❌ Fallback RPC {} failed: {} ({})", i + 1, fallback_url, e);
            }
        }
    }

    println!();
    println!("📋 Configuration Summary:");
    println!("   🔄 Swap enabled: {}", config.swap.enabled);
    println!("   🎯 Default DEX: {}", config.swap.default_dex);
    println!("   📈 Max slippage: {}%", config.swap.max_slippage * 100.0);
    println!("   🚀 Trade size: {} SOL", config.trader.trade_size_sol);
    println!("   🔒 Anti-MEV: {}", config.swap.is_anti_mev);

    println!();
    println!("✅ Main wallet check completed!");

    Ok(())
}
