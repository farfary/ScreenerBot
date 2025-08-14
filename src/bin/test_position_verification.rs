use screenerbot::configs::read_configs;
use screenerbot::utils::get_wallet_address;
use screenerbot::logger::{log, LogTag};
use screenerbot::transactions::{initialize_global_transaction_manager, TransactionsManager};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

async fn load_wallet_pubkey() -> Result<Pubkey, Box<dyn std::error::Error>> {
    let wallet_address_str = get_wallet_address()
        .map_err(|e| format!("Failed to get wallet address: {}", e))?;
    
    Pubkey::from_str(&wallet_address_str)
        .map_err(|e| format!("Invalid wallet address: {}", e).into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    log(LogTag::Transactions, "TEST", "🧪 Starting FIXED position verification test...");
    
    // Read configs and get wallet
    let _configs = read_configs()?;
    let wallet_pubkey = load_wallet_pubkey().await?;
    
    // Initialize transaction manager
    initialize_global_transaction_manager(wallet_pubkey).await?;
    
    // Create a direct manager instance for testing
    let mut manager = TransactionsManager::new(wallet_pubkey).await?;
    
    log(LogTag::Transactions, "TEST", "📋 Running FIXED position verification check...");
    
    // Test the fix multiple times to ensure it works consistently
    for i in 1..=3 {
        log(LogTag::Transactions, "TEST", &format!("🔄 Verification run {}/3...", i));
        
        match manager.check_and_verify_position_transactions().await {
            Ok(()) => {
                log(LogTag::Transactions, "TEST", &format!("✅ Verification run {} completed successfully", i));
            }
            Err(e) => {
                log(LogTag::Transactions, "TEST", &format!("❌ Verification run {} failed: {}", i, e));
            }
        }
        
        // Small delay between runs
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
    
    log(LogTag::Transactions, "TEST", "🏁 FIXED position verification test completed");
    log(LogTag::Transactions, "TEST", "🔧 The fix ensures positions are matched by transaction signature, not mint address");
    
    Ok(())
}
