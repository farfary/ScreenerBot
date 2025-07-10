// Test script to verify the new full watchlist display in print_summary
use screenerbot::helpers::print_summary;

#[tokio::main]
async fn main() {
    println!("🧪 Testing enhanced print_summary with full watchlist display...\n");
    
    // Call the enhanced print_summary function
    print_summary().await;
    
    println!("\n✅ Test completed!");
}
