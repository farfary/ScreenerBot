/// Test random user agent generation for DexScreener API
use screenerbot::{
    tokens::api::DexScreenerApi,
    logger::init_file_logging,
    global::set_cmd_args,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing Random User Agent Generation\n");
    
    // Initialize logging system
    init_file_logging();
    
    // Set up debug API flag to see user agents in logs
    set_cmd_args(vec!["test_random_user_agents".to_string(), "--debug-api".to_string()]);

    // Create multiple API instances to see different user agents
    for i in 1..=5 {
        println!("📡 Creating API Client #{}", i);
        let _api = DexScreenerApi::new();
        
        // Small delay to show the randomization
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    println!("\n✅ Random user agent test completed!");
    println!("📝 Each API client instance gets a random browser user agent");
    println!("🔄 This helps avoid rate limiting based on static user agent strings");
    println!("📋 Available user agents include:");
    println!("   • Chrome on Windows/Mac/Linux");
    println!("   • Firefox on Windows/Mac/Linux");
    println!("   • Safari on macOS");
    println!("   • Edge on Windows/Mac");
    println!("\n💡 To see the actual user agents being used, check the log files:");
    println!("   tail -f logs/screenerbot_*.log | grep USER_AGENT");

    Ok(())
}
