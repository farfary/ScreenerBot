/// Test script to display wallet transaction cache statistics

use screenerbot::{
    logger::init_file_logging,
    wallet_transactions::initialize_wallet_transaction_manager,
    summary::display_wallet_transaction_statistics,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_file_logging();
    
    println!("🧪 Testing Wallet Transaction Cache Statistics\n");
    
    // Initialize the wallet transaction manager
    initialize_wallet_transaction_manager().await?;
    
    // Display the statistics with our fixed cache efficiency calculation
    display_wallet_transaction_statistics();
    
    println!("\n✅ Cache statistics test completed!");
    
    Ok(())
}
