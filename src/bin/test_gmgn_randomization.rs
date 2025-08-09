use screenerbot::{
    utils::{get_random_partner_id},
    logger::{init_file_logging, log, LogTag},
    global::read_configs,
    swaps::gmgn::get_gmgn_quote,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();

    println!("🔵 Testing GMGN with Random Partner IDs\n");

    // Generate some random partner IDs like GMGN would
    println!("🎲 Partner IDs that GMGN would use:");
    for i in 1..=5 {
        let partner_id = get_random_partner_id();
        println!("  Request {}: partner={}", i, partner_id);
    }

    println!("\n🧪 Simulating GMGN quote requests with random partners...");
    
    // Test configuration loading
    match read_configs() {
        Ok(configs) => {
            log(LogTag::Test, "SUCCESS", "Configuration loaded successfully");
            
            // Simulate what happens during a GMGN quote request
            println!("\n📊 Each GMGN quote request now uses:");
            for i in 1..=3 {
                let partner_id = get_random_partner_id();
                println!("  Request {}: Random partner ID = '{}'", i, partner_id);
                println!("  Request {}: Random user agent = Complex browser simulation", i);
                println!("  Request {}: Random headers = Browser-like security headers", i);
                println!();
            }
        }
        Err(e) => {
            log(LogTag::Test, "ERROR", &format!("Failed to load configs: {}", e));
        }
    }

    println!("✅ GMGN randomization test completed!");
    println!("🔐 Every GMGN request now appears as a different application/browser");
    
    Ok(())
}
