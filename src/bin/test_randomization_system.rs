use screenerbot::{
    utils::{get_random_user_agent, get_random_partner_id, create_randomized_http_client},
    logger::{init_file_logging},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();

    println!("🎲 Testing Universal Randomization System\n");

    // Test user agent randomization
    println!("🌐 Testing Random User Agents:");
    for i in 1..=5 {
        let user_agent = get_random_user_agent();
        println!("  {}. {}", i, user_agent);
    }

    println!();

    // Test partner ID randomization
    println!("🔗 Testing Random Partner IDs:");
    for i in 1..=10 {
        let partner_id = get_random_partner_id();
        println!("  {}. {}", i, partner_id);
    }

    println!();

    // Test HTTP client creation
    println!("📡 Testing Randomized HTTP Client Creation:");
    for i in 1..=3 {
        match create_randomized_http_client() {
            Ok(_client) => {
                println!("  {}. ✅ HTTP client created successfully", i);
            }
            Err(e) => {
                println!("  {}. ❌ HTTP client creation failed: {}", i, e);
            }
        }
    }

    println!("\n✅ Randomization system test completed!");
    
    Ok(())
}
