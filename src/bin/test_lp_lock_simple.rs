/// Simple LP Lock Test - Test the enhanced LP lock detection without hanging
use screenerbot::{ logger::{ log, LogTag }, tokens::lp_lock::check_lp_lock_status };

#[tokio::main]
async fn main() {
    println!("Starting simple LP lock test");

    // Test tokens of different sizes
    let test_tokens = vec![
        ("So11111111111111111111111111111111111111112", "SOL - Should Skip"),
        ("4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R", "RAY - Should Skip"),
        ("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", "USDC - Should Skip")
    ];

    for (mint, description) in test_tokens {
        println!("\n{}", "=".repeat(60));
        println!("Testing: {} ({})", description, &mint[..8]);
        println!("{}", "=".repeat(60));

        match check_lp_lock_status(mint).await {
            Ok(analysis) => {
                println!("✅ LP Lock Analysis Result:");
                println!("   Status: {:?}", analysis.status);
                println!("   Pool Type: {:?}", analysis.details.pool_type);
                println!("   Score: {}", analysis.lock_score);
                println!("   Notes: {:?}", analysis.details.notes);
            }
            Err(e) => {
                println!("❌ Error: {}", e);
            }
        }
    }

    println!("\n🎉 Test completed without hanging!");
}
