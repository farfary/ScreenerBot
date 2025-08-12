/// Test tool to verify the RPC confirmation fix
/// This tests that failed transactions are properly rejected during confirmation

use screenerbot::rpc::get_rpc_client;
use screenerbot::logger::{log, LogTag};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 Testing RPC Confirmation Fix");
    println!("================================");
    
    // Initialize RPC client
    let rpc_client = get_rpc_client();
    
    // Test with the known failed transaction
    let failed_signature = "2SyJYuccEKzFdvobesVFZ6LYdSFfVZTQJrGm7ZPYhP5wPoDeEi1bEvSKrQxFTrge5N3wSihFYHknmrqKTW8m4bws";
    
    println!("\n📋 Test 1: Failed transaction");
    println!("Signature: {}", &failed_signature[..16]);
    println!("Expected result: Should return error (transaction failed on-chain)");
    
    // Test the confirmation logic with failed transaction
    match rpc_client.wait_for_transaction_confirmation(failed_signature, 1, 1000).await {
        Ok(true) => {
            println!("❌ BUG: wait_for_transaction_confirmation returned success for failed transaction!");
            println!("This means the fix did not work correctly.");
        }
        Ok(false) => {
            println!("⚠️  Timeout: wait_for_transaction_confirmation timed out");
            println!("This is unexpected for an existing transaction.");
        }
        Err(e) => {
            println!("✅ SUCCESS: wait_for_transaction_confirmation properly failed!");
            println!("Error: {}", e);
            println!("This means the fix is working correctly - failed transactions are rejected.");
        }
    }
    
    // Test with a known successful transaction
    let success_signature = "1tcfh74AtZRqMTRi9Dvaij8aqkZYQtvCUnfnbm8j9oSVe8E4DpXu4UrKmD9cd91hdhCF9USfUWJsPLiR5gDwfEu";
    
    println!("\n📋 Test 2: Successful transaction");
    println!("Signature: {}", &success_signature[..16]);
    println!("Expected result: Should return success (transaction succeeded on-chain)");
    
    // Test the confirmation logic with successful transaction
    match rpc_client.wait_for_transaction_confirmation(success_signature, 1, 1000).await {
        Ok(true) => {
            println!("✅ SUCCESS: wait_for_transaction_confirmation properly succeeded!");
            println!("This means successful transactions are still accepted correctly.");
        }
        Ok(false) => {
            println!("⚠️  Timeout: wait_for_transaction_confirmation timed out");
            println!("This is unexpected for an existing transaction.");
        }
        Err(e) => {
            println!("❌ UNEXPECTED: wait_for_transaction_confirmation failed for successful transaction!");
            println!("Error: {}", e);
            println!("This suggests the fix might be too strict.");
        }
    }
    
    println!("\n📊 Test completed. Check the results above.");
    Ok(())
}
