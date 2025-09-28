// Test the fixed security filtering logic
use screenerbot::filtering::get_filtered_tokens;
use screenerbot::logger::init_file_logging;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();

    println!("🔍 Testing Fixed Security Filtering");

    println!("\n--- Before Fix Analysis ---");
    println!("Expected: Only ~72 safe tokens from last 24h would be available");
    println!("Problem: 2,407 safe tokens (97.1%) were being rejected due to age");

    println!("\n--- Testing Fixed Filtering ---");
    match get_filtered_tokens().await {
        Ok(filtered_tokens) => {
            println!("✅ Filtering successful!");
            println!(
                "📊 Total tokens passed filtering: {}",
                filtered_tokens.len()
            );

            if filtered_tokens.len() > 100 {
                println!("🎉 SUCCESS: Significantly more tokens now available!");
                println!("🔒 Security data age filter has been fixed");

                // Show a few sample tokens
                println!("\nSample filtered tokens:");
                for (i, token) in filtered_tokens.iter().take(10).enumerate() {
                    println!("  {}. {}", i + 1, token);
                }
                if filtered_tokens.len() > 10 {
                    println!("  ... and {} more", filtered_tokens.len() - 10);
                }
            } else {
                println!("⚠️  Still low number of tokens - may need further investigation");
            }
        }
        Err(e) => {
            println!("❌ Filtering failed: {}", e);
        }
    }

    println!("\n🏁 Test complete");
    Ok(())
}
