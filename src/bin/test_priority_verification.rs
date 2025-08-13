/// Test tool to verify the priority verification timeouts
/// This tests that priority transactions use faster verification

use screenerbot::rpc::get_rpc_client;
use screenerbot::logger::{log, LogTag};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Testing Priority Verification Timeouts");
    println!("==========================================");
    
    // Initialize RPC client
    let rpc_client = get_rpc_client();
    
    // Test with a real signature format but non-existent transaction to trigger "not found" retry logic
    let fake_signature = "3SyJYuccEKzFdvobesVFZ6LYdSFfVZTQJrGm7ZPYhP5wPoDeEi1bEvSKrQxFTrge5N3wSihFYHknmrqKTW8m4bws";
    
    println!("\n📋 Test 1: Priority verification timeout (should be ~5 seconds)");
    println!("Signature: {}", &fake_signature[..16]);
    println!("Expected result: Should timeout in approximately 5 seconds");
    
    let start_time = Instant::now();
    
    // Test the priority confirmation logic
    match rpc_client.wait_for_priority_transaction_confirmation(fake_signature).await {
        Ok(true) => {
            println!("❌ UNEXPECTED: Priority verification returned success for fake transaction!");
        }
        Ok(false) => {
            let elapsed = start_time.elapsed();
            println!("✅ SUCCESS: Priority verification timed out as expected!");
            println!("⏱️  Elapsed time: {:.2} seconds", elapsed.as_secs_f64());
            
            if elapsed.as_secs() <= 6 { // Allow 1 second tolerance
                println!("✅ TIMING: Priority timeout is working correctly (≤6 seconds)");
            } else {
                println!("⚠️  TIMING: Priority timeout took longer than expected (>6 seconds)");
            }
        }
        Err(e) => {
            let elapsed = start_time.elapsed();
            println!("✅ SUCCESS: Priority verification failed as expected for fake transaction!");
            println!("⏱️  Elapsed time: {:.2} seconds", elapsed.as_secs_f64());
            println!("Error: {}", e);
        }
    }
    
    println!("\n📋 Test 2: Regular verification timeout (should be longer)");
    println!("Signature: {}", &fake_signature[..16]);
    println!("Expected result: Should timeout in approximately 20+ seconds (max 10 attempts × 2s)");
    
    let start_time = Instant::now();
    
    // Test the regular confirmation logic with limited attempts
    match rpc_client.wait_for_transaction_confirmation(fake_signature, 5, 2000).await {
        Ok(true) => {
            println!("❌ UNEXPECTED: Regular verification returned success for fake transaction!");
        }
        Ok(false) => {
            let elapsed = start_time.elapsed();
            println!("✅ SUCCESS: Regular verification timed out as expected!");
            println!("⏱️  Elapsed time: {:.2} seconds", elapsed.as_secs_f64());
            
            if elapsed.as_secs() >= 8 { // Should be at least 4-5 attempts × 2s with backoff
                println!("✅ TIMING: Regular timeout is working correctly (≥8 seconds)");
            } else {
                println!("⚠️  TIMING: Regular timeout was faster than expected (<8 seconds)");
            }
        }
        Err(e) => {
            let elapsed = start_time.elapsed();
            println!("✅ SUCCESS: Regular verification failed as expected for fake transaction!");
            println!("⏱️  Elapsed time: {:.2} seconds", elapsed.as_secs_f64());
            println!("Error: {}", e);
        }
    }
    
    println!("\n📊 Test completed. Priority verification should be significantly faster than regular verification.");
    Ok(())
}
