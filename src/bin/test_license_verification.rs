/// Test license verification with mainnet NFT
/// 
/// This binary tests the license verification system by reading the wallet from config.toml

use screenerbot::license::verify_license_for_wallet;
use screenerbot::logger;
use screenerbot::config;
use screenerbot::utils::get_wallet_address;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    // Initialize logger
    logger::init();
    
    // Load config
    if let Err(e) = config::load_config() {
        eprintln!("❌ Failed to load config: {}", e);
        eprintln!("💡 Make sure data/config.toml exists");
        return;
    }

    println!("🔐 ScreenerBot License Verification Test");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Get wallet from config
    let test_wallet = match get_wallet_address() {
        Ok(wallet) => wallet,
        Err(e) => {
            eprintln!("❌ Failed to get wallet from config: {}", e);
            return;
        }
    };
    
    println!("📋 Test Configuration:");
    println!("   Wallet: {}", test_wallet);
    println!();

    let wallet_pubkey = match Pubkey::from_str(&test_wallet) {
        Ok(pubkey) => pubkey,
        Err(e) => {
            eprintln!("❌ Invalid wallet address: {}", e);
            return;
        }
    };

    println!("🔍 Starting license verification...");
    println!();

    // First attempt - will hit RPC
    println!("📡 Attempt 1 (RPC call expected):");
    let start = std::time::Instant::now();
    match verify_license_for_wallet(&wallet_pubkey).await {
        Ok(status) => {
            let duration = start.elapsed();
            println!("   Duration: {:.2}s", duration.as_secs_f64());
            println!();
            print_license_status(&status);
        }
        Err(e) => {
            eprintln!("❌ Verification failed: {}", e);
            return;
        }
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Second attempt - should hit cache
    println!("📦 Attempt 2 (cache hit expected):");
    let start = std::time::Instant::now();
    match verify_license_for_wallet(&wallet_pubkey).await {
        Ok(status) => {
            let duration = start.elapsed();
            println!("   Duration: {:.3}s (should be < 0.001s)", duration.as_secs_f64());
            println!();
            if duration.as_millis() < 10 {
                println!("   ✅ Cache working correctly!");
            } else {
                println!("   ⚠️  Cache may not be working (took > 10ms)");
            }
            println!();
            print_license_status(&status);
        }
        Err(e) => {
            eprintln!("❌ Verification failed: {}", e);
            return;
        }
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!("✅ Test completed successfully!");
}

fn print_license_status(status: &screenerbot::license::LicenseStatus) {
    println!("📊 License Status:");
    println!("   Valid: {}", if status.valid { "✅ Yes" } else { "❌ No" });
    
    if let Some(tier) = &status.tier {
        println!("   Tier: {}", tier);
    }
    
    if let Some(mint) = &status.mint {
        println!("   NFT Mint: {}", mint);
        println!("   Solscan: https://solscan.io/token/{}", mint);
    }
    
    if let Some(start_ts) = status.start_ts {
        let start_time = chrono::DateTime::from_timestamp(start_ts as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| format!("{}", start_ts));
        println!("   Start: {}", start_time);
    }
    
    if let Some(expiry_ts) = status.expiry_ts {
        let expiry_time = chrono::DateTime::from_timestamp(expiry_ts as i64, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
            .unwrap_or_else(|| format!("{}", expiry_ts));
        println!("   Expiry: {}", expiry_time);
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let remaining = expiry_ts.saturating_sub(now);
        let days_remaining = remaining / 86400;
        println!("   Remaining: {} days", days_remaining);
    }
    
    if let Some(reason) = &status.reason {
        println!("   Reason: {}", reason);
    }
}
