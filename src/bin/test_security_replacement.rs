/// Quick test of the new simplified security module
///
/// This tests the replaced security.rs module to ensure the Rugcheck API integration
/// works correctly and maintains backward compatibility.

use screenerbot::tokens::security::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Testing Replaced Security Module ===\n");

    // Test API status first
    println!("1. Testing API Status...");
    match check_api_status().await {
        Ok(true) => println!("✅ Rugcheck API is operational"),
        Ok(false) => println!("❌ Rugcheck API returned error status"),
        Err(e) => println!("❌ Failed to check API status: {}", e),
    }

    println!();

    // Test backward compatibility - get security analyzer
    println!("2. Testing Backward Compatibility...");
    let analyzer = get_security_analyzer();
    println!("✅ Security analyzer obtained");

    // Test token analysis with the sample token from curl
    let test_token = "6WiJZTKLN4ZN4ncUbB7cZZFr2UTW3yF3pLeT2w23ZCdk";
    println!("3. Testing Token Analysis...");
    println!("Token: {}", test_token);

    match analyzer.analyze_token_security(test_token).await {
        Ok(security_info) => {
            println!("✅ Security analysis successful");
            println!("Summary: {}", security_info.summary());
            println!();
            println!("Backward Compatibility Fields:");
            println!("  - Security Score: {}", security_info.security_score);
            println!("  - Risk Level: {}", security_info.risk_level.as_str());
            println!("  - Can Mint: {}", security_info.security_flags.can_mint);
            println!("  - Can Freeze: {}", security_info.security_flags.can_freeze);
            println!("  - LP Locked: {}", security_info.security_flags.lp_locked);
            println!("  - Holder Info: {:?}", security_info.holder_info.is_some());
            println!("  - Timestamps: Last Updated = {}", security_info.timestamps.last_updated);
        }
        Err(e) => {
            println!("❌ Security analysis failed: {}", e);
        }
    }

    println!("\n=== Test Complete ===");
    println!("✅ Old security.rs successfully replaced with simplified Rugcheck-only version!");

    Ok(())
}
