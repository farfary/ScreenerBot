/// Tool to analyze wallet swap transactions using the efficient transaction manager
/// This will give us real on-chain data for swaps to verify P&L calculations

use screenerbot::{
    wallet_transactions::{
        get_wallet_transaction_manager, 
        analyze_recent_swaps_global,
        WalletTransactionManager,
    },
    logger::{init_file_logging, log, LogTag},
};
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize file logging system
    init_file_logging();
    
    log(LogTag::System, "INFO", "Starting wallet swap analysis tool with transaction manager");
    
    // Clean previous logs first
    log(LogTag::System, "INFO", "Logs cleaned for fresh analysis session");
    
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();
    let limit = if args.len() > 1 {
        match args[1].parse::<usize>() {
            Ok(n) => {
                log(LogTag::System, "ARGS", &format!("Analysis limit set to {} swaps", n));
                n
            },
            Err(e) => {
                log(LogTag::System, "ERROR", &format!("Invalid limit argument '{}': {}", args[1], e));
                println!("Usage: {} [limit]", args[0]);
                println!("  limit: Number of recent swaps to analyze (default: 20)");
                return Ok(());
            }
        }
    } else {
        20 // Default limit
    };
    
    log(LogTag::System, "INFO", &format!("Will analyze up to {} recent swap transactions", limit));
    
    // Check if the global wallet transaction manager is available
    match get_wallet_transaction_manager() {
        Ok(manager_lock) => {
            let manager_guard = manager_lock.read().unwrap();
            if let Some(ref manager) = *manager_guard {
                log(LogTag::System, "SUCCESS", "Using existing wallet transaction manager from global state");
                
                // Get sync stats
                let (cached_count, total_fetched, last_sync) = manager.get_sync_stats();
                log(LogTag::System, "CACHE_STATS", &format!("Cache status: {} transactions cached, {} total fetched, last sync: {}", 
                    cached_count, total_fetched, last_sync));
                
                println!("\n=== WALLET TRANSACTION CACHE STATUS ===");
                println!("📊 Cached transactions: {}", cached_count);
                println!("🔄 Total fetched: {}", total_fetched);
                println!("⏰ Last sync: {}", last_sync);
                
                // Analyze recent swaps
                let analysis = manager.analyze_recent_swaps(limit);
                
                // Display results
                WalletTransactionManager::display_analysis(&analysis);
                
                drop(manager_guard); // Release the lock explicitly
            } else {
                log(LogTag::System, "WARN", "Global wallet transaction manager not initialized, creating standalone instance");
                
                // Fall back to creating a standalone manager
                let analysis = analyze_recent_swaps_global(limit).await?;
                WalletTransactionManager::display_analysis(&analysis);
            }
        },
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to access wallet transaction manager: {}", e));
            return Err(e);
        }
    }
    
    log(LogTag::System, "SUCCESS", "Wallet swap analysis completed successfully");
    
    Ok(())
}
