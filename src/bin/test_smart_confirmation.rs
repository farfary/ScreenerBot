use screenerbot::rpc::RpcClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Smart Transaction Confirmation Logic");
    println!("================================================");
    
    // Create RPC client
    let rpc_client = RpcClient::new();
    
    // Test with the original failed transaction that took 110 seconds
    let failed_signature = "4hy7TxdbdxbbXk9zaNuUGg74jofjSNgpLaYuP7eysoV5R3isvv3ejFih2YwPPnRqJ9NLUSreMpWBjiC7mHEPsgYz";
    
    println!("🔍 Testing failed transaction (should fail fast): {}", &failed_signature[..8]);
    
    let start_time = std::time::Instant::now();
    
    match rpc_client.wait_for_transaction_confirmation_smart(failed_signature, 10, 3000).await {
        Ok(true) => {
            println!("❌ ERROR: Failed transaction reported as successful!");
        }
        Ok(false) => {
            let duration = start_time.elapsed();
            println!("✅ SUCCESS: Transaction properly failed fast in {:?}", duration);
            
            if duration.as_secs() < 30 {
                println!("🚀 IMPROVEMENT: Failed in {} seconds (vs previous 110+ seconds)", duration.as_secs());
            } else {
                println!("⚠️  Still took {} seconds - may need further optimization", duration.as_secs());
            }
        }
        Err(e) => {
            let duration = start_time.elapsed();
            println!("✅ SUCCESS: Transaction properly errored in {:?} - Error: {}", duration, e);
        }
    }
    
    // Test with the successful transaction for comparison
    let success_signature = "3RCav6BWq4cHZyjeSWDxRvCDPAkQaXkfYLiqRC22cj7ci3XZ3b1edsk31RCZDxiqsUiZkXELZE4DRN9dBmkE3CR8";
    
    println!("\n🔍 Testing successful transaction (should succeed): {}", &success_signature[..8]);
    
    let start_time = std::time::Instant::now();
    
    match rpc_client.wait_for_transaction_confirmation_smart(success_signature, 10, 3000).await {
        Ok(true) => {
            let duration = start_time.elapsed();
            println!("✅ SUCCESS: Transaction properly succeeded in {:?}", duration);
        }
        Ok(false) => {
            println!("⚠️  Unexpected: Successful transaction reported as failed/timeout");
        }
        Err(e) => {
            println!("⚠️  Unexpected: Successful transaction errored: {}", e);
        }
    }
    
    println!("\n🎯 Smart confirmation test completed!");
    
    Ok(())
}
