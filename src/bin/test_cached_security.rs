// Test cached security info retrieval
use screenerbot::logger::init_file_logging;
use screenerbot::tokens::security::get_security_analyzer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();

    println!("🔍 Testing Cached Security Info Retrieval");

    // Test tokens that we know have security data
    let test_tokens = vec![
        "UmTt1cGStKwAQFXbQWELJzyjNFrdwV2JrFW6pcEpump".to_string(),
        "9vyLnN3TYexVA2fZ1fScHWfXdkye4XnEsGX5gfW7pump".to_string(),
        "8rWQDhktodDA2WP5b1o241taN2sujxCUGrrJ3JnZpump".to_string()
    ];

    let analyzer = get_security_analyzer();

    for mint in test_tokens {
        println!("\n--- Testing mint: {} ---", mint);

        // First check if it's in the database directly
        match analyzer.database.get_security_info(&mint) {
            Ok(Some(info)) => {
                println!("✅ Found in database:");
                println!("  - Mint Authority Disabled: {}", info.mint_authority_disabled);
                println!("  - Freeze Authority Disabled: {}", info.freeze_authority_disabled);
                println!("  - LP Safe: {}", info.lp_is_safe);
                println!("  - Is Safe: {}", info.is_safe);
                println!("  - Analyzed At: {}", info.analyzed_at);
                println!("  - Risk Level: {:?}", info.risk_level);
                println!("  - Security Score: {}", info.security_score);

                // Calculate age
                let now = chrono::Utc::now();
                let age = now.signed_duration_since(info.timestamps.last_updated);
                println!("  - Age: {} hours", age.num_hours());
                println!("  - Less than 1 day old: {}", age < chrono::Duration::days(1));
            }
            Ok(None) => println!("❌ Not found in database"),
            Err(e) => println!("⚠️ Database error: {}", e),
        }

        // Test in-memory cache
        if let Some(cached_info) = analyzer.cache.get(&mint) {
            println!("✅ Found in memory cache");
        } else {
            println!("❌ Not found in memory cache");
        }
    }

    println!("\n🏁 Test complete");
    Ok(())
}
