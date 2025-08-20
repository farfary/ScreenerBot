use screenerbot::*;
use screenerbot::transactions::TransactionsManager;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🧪 Testing convert_to_swap_pnl_info conversion");
    
    // Initialize the system
    let config = configs::read_configs()?;
    let rpc_client = rpc::init_rpc_client()?;
    
    // Initialize transactions manager
    let transactions_manager = TransactionsManager::new(rpc_client, config).await;
    
    // Load the specific transaction
    let signature = "eQNWDCLwg9AnR4Wnc75kHBAcMFE9ZV76qXcj4SkzFLzyh4NdgBkiseam6xEcNrDUmqbW8BhcgGdRziCgk4hiooQ";
    
    println!("🔍 Loading transaction: {}", &signature[..8]);
    if let Ok(Some(transaction)) = transactions_manager.load_transaction(signature).await {
        println!("✅ Transaction loaded successfully");
        println!("   Type: {:?}", transaction.transaction_type);
        println!("   Success: {}", transaction.success);
        println!("   SOL change: {}", transaction.sol_balance_change);
        
        // Test convert_to_swap_pnl_info
        let empty_cache = HashMap::new();
        println!("\n🔬 Testing convert_to_swap_pnl_info...");
        
        match transactions_manager.convert_to_swap_pnl_info(&transaction, &empty_cache, false) {
            Some(swap_pnl_info) => {
                println!("✅ Conversion successful!");
                println!("   Type: {}", swap_pnl_info.swap_type);
                println!("   SOL amount: {}", swap_pnl_info.sol_amount);
                println!("   Token amount: {}", swap_pnl_info.token_amount);
                println!("   Token mint: {}", swap_pnl_info.token_mint);
            }
            None => {
                println!("❌ Conversion returned None");
            }
        }
    } else {
        println!("❌ Failed to load transaction");
    }
    
    Ok(())
}
