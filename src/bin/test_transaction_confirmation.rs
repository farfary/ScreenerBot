/// Test transaction confirmation functionality
/// This tests the new wait_for_transaction_confirmation function

use screenerbot::{
    logger::{log, LogTag, init_file_logging},
    rpc::{get_rpc_client},
    global::read_configs,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_file_logging();
    
    log(LogTag::System, "TEST_START", "🧪 Testing transaction confirmation functionality");
    
    // Load configs
    let _configs = read_configs()?;
    
    // Get RPC client
    let rpc_client = get_rpc_client();
    
    // Test with a known non-existent transaction (should timeout)
    let fake_signature = "1111111111111111111111111111111111111111111111111111111111111111111111111111111111111111";
    
    log(LogTag::System, "TEST_FAKE_TX", "🔍 Testing with fake transaction signature (should timeout)");
    
    match rpc_client.wait_for_transaction_confirmation(fake_signature, 3, 1000).await {
        Ok(true) => {
            log(LogTag::System, "TEST_ERROR", "❌ Unexpected: fake transaction confirmed");
        }
        Ok(false) => {
            log(LogTag::System, "TEST_SUCCESS", "✅ Correctly timed out for fake transaction");
        }
        Err(e) => {
            log(LogTag::System, "TEST_ERROR", &format!("❌ Error for fake transaction: {}", e));
        }
    }
    
    // Test with a known confirmed transaction from Solana mainnet
    // This is a random confirmed transaction signature from Solana Explorer
    let real_signature = "2ZE7R58vVFGNqTEGBMSkwWaG1q6gKsAvNZmrpFnBzFD4CCXCUqEZUGUY4LhCB2CTdYMWjgj1EHjXYZUKfNqV8Nzt";
    
    log(LogTag::System, "TEST_REAL_TX", "🔍 Testing with real transaction signature (should confirm quickly)");
    
    match rpc_client.wait_for_transaction_confirmation(real_signature, 3, 1000).await {
        Ok(true) => {
            log(LogTag::System, "TEST_SUCCESS", "✅ Correctly confirmed real transaction");
        }
        Ok(false) => {
            log(LogTag::System, "TEST_WARNING", "⚠️ Real transaction timed out (may be too old or not found)");
        }
        Err(e) => {
            log(LogTag::System, "TEST_INFO", &format!("ℹ️ Error for real transaction: {}", e));
        }
    }
    
    log(LogTag::System, "TEST_COMPLETE", "🏁 Transaction confirmation test completed");
    
    Ok(())
}
