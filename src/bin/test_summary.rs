use screenerbot::summary::display_positions_table;
use screenerbot::tokens::price_service::initialize_price_service;
use screenerbot::logger::{ log, LogTag };

#[tokio::main]
async fn main() {
    println!("🧪 Testing summary display functionality...");
    
    // Initialize the price service
    if let Err(e) = initialize_price_service().await {
        println!("❌ Failed to initialize price service: {}", e);
        return;
    }
    
    // Display the positions table
    display_positions_table().await;
    
    println!("✅ Summary test completed");
}
