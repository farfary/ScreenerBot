use screenerbot::config::Config;
use screenerbot::rpc_manager::RpcManager;
use anyhow::Result;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("🔍 Testing RPC connectivity...\n");

    // Load configuration
    let config = Config::load("configs.json")?;

    // Test RPC connections
    println!("Primary RPC: {}", config.rpc_url);
    for (i, fallback) in config.rpc_fallbacks.iter().enumerate() {
        println!("Fallback {}: {}", i + 1, fallback);
    }
    println!();

    // Create RPC manager
    let rpc_manager = Arc::new(
        RpcManager::new(
            vec![config.rpc_url.clone()].into_iter().chain(config.rpc_fallbacks.clone()).collect()
        )?
    );

    // Test a simple RPC call
    println!("🔄 Testing RPC connectivity with block height query...");

    let result = rpc_manager.execute_with_fallback(|client| {
        client.get_block_height().map_err(|e| anyhow::anyhow!("RPC error: {}", e))
    });

    match result {
        Ok(block_height) => {
            println!("✅ RPC connection successful!");
            println!("📦 Current block height: {}", block_height);
        }
        Err(e) => {
            println!("❌ RPC connection failed: {}", e);

            // Test individual endpoints
            println!("\n🔍 Testing individual endpoints...");

            // Test primary
            println!("Testing primary: {}", config.rpc_url);
            let primary_client = solana_client::rpc_client::RpcClient::new(config.rpc_url.clone());
            match primary_client.get_block_height() {
                Ok(height) => println!("  ✅ Primary OK: height {}", height),
                Err(e) => println!("  ❌ Primary failed: {}", e),
            }

            // Test fallbacks
            for (i, fallback_url) in config.rpc_fallbacks.iter().enumerate() {
                println!("Testing fallback {}: {}", i + 1, fallback_url);
                let fallback_client = solana_client::rpc_client::RpcClient::new(
                    fallback_url.clone()
                );
                match fallback_client.get_block_height() {
                    Ok(height) => println!("  ✅ Fallback {} OK: height {}", i + 1, height),
                    Err(e) => println!("  ❌ Fallback {} failed: {}", i + 1, e),
                }
            }
        }
    }

    println!("\n✅ RPC connectivity test completed!");
    Ok(())
}
