/// Test program for LP lock verification with specific token
/// Usage: cargo run --bin test_lp_lock_verification

use screenerbot::logger::{ log, LogTag };
use screenerbot::tokens::lp_lock::{ check_lp_lock_status, LpLockStatus };
use screenerbot::tokens::dexscreener::init_dexscreener_api;
use screenerbot::rpc::get_rpc_client;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    // Initialize DexScreener API first
    if let Err(e) = init_dexscreener_api().await {
        println!("⚠️  Failed to initialize DexScreener API: {}", e);
        println!("Continuing with test anyway...");
    } else {
        println!("✅ DexScreener API initialized successfully");
    }

    // Test token and pool provided by user
    let test_token = "4CjyetevoeK2u4aUjuVbccbYmx6VqV3Qgqw88VqRPGgk";
    let test_pool = "9e9P4SzyxZMruF8e7hwYx6Lc7JEdahiXAVrcxW1ZdVxc";

    println!("🧪 Testing LP Lock Verification System");
    println!("Token: {}", test_token);
    println!("Pool: {}", test_pool);
    println!("Expected: Not burned (according to user)");
    println!("{}", "=".repeat(70));

    // Test the LP lock verification
    match check_lp_lock_status(test_token).await {
        Ok(analysis) => {
            println!("✅ Analysis completed successfully!");
            println!();

            println!("📊 Results:");
            println!("  Status: {:?}", analysis.status);
            println!("  Description: {}", analysis.status.description());
            println!("  Risk Level: {}", analysis.status.risk_level());
            println!("  Is Safe: {}", analysis.status.is_safe());
            println!("  Lock Score: {}/100", analysis.lock_score);
            println!();

            if let Some(pool_address) = &analysis.pool_address {
                println!("🏊 Pool Info:");
                println!("  Address: {}", pool_address);
                if let Some(lp_mint) = &analysis.lp_mint {
                    println!("  LP Mint: {}", lp_mint);
                }
                println!();
            }

            if !analysis.details.is_empty() {
                println!("🔍 Detailed Analysis:");
                for (i, detail) in analysis.details.iter().enumerate() {
                    println!("  {}. {}", i + 1, detail);
                }
                println!();
            }

            // Verify against user expectation
            match analysis.status {
                LpLockStatus::Burned => {
                    println!("❌ MISMATCH: System detected BURNED but user says NOT BURNED");
                }
                LpLockStatus::ProgramLocked { ref program, amount } => {
                    println!(
                        "⚠️  System detected PROGRAM LOCKED in {}, amount: {}",
                        program,
                        amount
                    );
                    println!("   This conflicts with user expectation of NOT BURNED");
                }
                LpLockStatus::Locked { confidence, .. } => {
                    println!("⚠️  System detected LOCKED with confidence: {}%", confidence);
                    println!("   This may or may not conflict with user expectation");
                }
                LpLockStatus::NotLocked { confidence } => {
                    println!("✅ MATCH: System detected NOT LOCKED (confidence: {}%)", confidence);
                    println!("   This aligns with user expectation of NOT BURNED");
                }
                LpLockStatus::CreatorHeld => {
                    println!("⚠️  System detected CREATOR HELD");
                    println!("   This may align with user expectation of NOT BURNED");
                }
                LpLockStatus::Unknown => {
                    println!("❓ System could not determine lock status");
                }
                LpLockStatus::NoPool => {
                    println!("❌ System could not find pool - this may indicate an issue");
                }
                _ => {
                    println!("❓ Unexpected status: {:?}", analysis.status);
                }
            }
        }
        Err(e) => {
            println!("❌ Analysis failed: {}", e);
            log(
                LogTag::Security,
                "ERROR",
                &format!("LP lock analysis failed for token {}: {}", test_token, e)
            );
        }
    }

    println!("{}", "=".repeat(70));

    // Also test manual pool analysis if we have the pool address
    println!("🔧 Testing Manual Pool Analysis with provided pool address...");

    // Test RPC connection
    let client = get_rpc_client();
    println!("Testing RPC connection...");

    if let Ok(pool_pubkey) = Pubkey::from_str(test_pool) {
        match client.get_account(&pool_pubkey).await {
            Ok(account) => {
                println!("✅ Successfully retrieved pool account data");
                println!("  Owner: {}", account.owner);
                println!("  Data length: {} bytes", account.data.len());
                println!("  Lamports: {}", account.lamports);

                // Try to identify the program type
                let owner_str = account.owner.to_string();
                let program_type = match owner_str.as_str() {
                    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => "Raydium AMM",
                    "CAMMCzo5YL8w4VFF8KVHrK22GGUQpMAScomTSXB7gdR" => "Raydium CPMM",
                    "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM" => "Raydium CLMM",
                    "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1" => "Orca Whirlpool",
                    "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => "Meteora DLMM",
                    "Eo7WjKq67rjJQSZxS6z3YkapzY3eMj6Xy8X5EQVn5UaB" => "Meteora DAMM",
                    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => "PumpFun",
                    "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" =>
                        "Phoenix V1 AMM (Research needed)",
                    _ => "Unknown",
                };

                println!("  Program Type: {}", program_type);

                if owner_str == "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA" {
                    println!("  🔍 RESEARCH NOTE: This appears to be Phoenix V1 AMM program");
                    println!(
                        "     Pool data size: {} bytes (typical for AMM pools)",
                        account.data.len()
                    );
                    println!(
                        "     This program type is not yet supported in our LP lock verification"
                    );
                }
            }
            Err(e) => {
                println!("❌ Failed to retrieve pool account: {}", e);
            }
        }
    } else {
        println!("❌ Invalid pool address format");
    }

    println!("\n🏁 Test completed!");
}
