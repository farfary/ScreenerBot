/// Test script to display wallet transaction cache statistics

use screenerbot::{
    logger::{init_file_logging},
    transactions_manager::initialize_transactions_manager,
    summary::display_transactions_statistics,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();
    
    println!("🧪 Testing Wallet Transaction Cache Statistics\n");
    
    // Initialize the wallet transaction manager
    initialize_transactions_manager().await?;
    
    // Display the statistics with our fixed cache efficiency calculation
    display_transactions_statistics();
    
    println!("\n✅ Cache statistics test completed!");
    
    Ok(())
}
