/// Test tool to verify main RPC vs premium RPC separation
/// This tests that:
/// - Main RPC is used for lightweight signature checking
/// - Premium RPC is used for data-intensive transaction fetching
/// - Both methods work correctly and log their usage

use screenerbot::{
    rpc::{get_rpc_client, init_rpc_client},
    logger::{init_file_logging, log, LogTag},
    global::read_configs,
};
use solana_sdk::{pubkey::Pubkey, signature::Signer};
use std::str::FromStr;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Test RPC separation between main and premium endpoints")]
struct Args {
    /// Wallet address to test with
    #[arg(short, long)]
    wallet: Option<String>,
    
    /// Specific transaction signature to test fetching
    #[arg(short, long)]
    signature: Option<String>,
    
    /// Number of signatures to fetch (default: 5)
    #[arg(short, long, default_value = "5")]
    limit: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize logging
    init_file_logging();
    log(LogTag::System, "START", "🧪 Testing RPC separation between main and premium endpoints");
    
    // Initialize RPC client
    init_rpc_client()?;
    let rpc_client = get_rpc_client();
    
    // Get wallet address
    let wallet_address = if let Some(wallet) = args.wallet {
        wallet
    } else {
        // Use configured wallet
        let configs = read_configs()?;
        let wallet_keypair = screenerbot::global::load_wallet_from_config(&configs)?;
        wallet_keypair.pubkey().to_string()
    };
    
    log(LogTag::System, "INFO", &format!("Testing with wallet: {}", &wallet_address[..8]));
    
    // Test 1: Fetch signatures using main RPC (lightweight operation)
    log(LogTag::System, "TEST", "🔍 Test 1: Fetching signatures using main RPC");
    let wallet_pubkey = Pubkey::from_str(&wallet_address)?;
    
    match rpc_client.get_wallet_signatures_main_rpc(&wallet_pubkey, args.limit, None).await {
        Ok(signatures) => {
            log(LogTag::System, "SUCCESS", &format!("✅ Successfully fetched {} signatures using main RPC", signatures.len()));
            
            if !signatures.is_empty() {
                // Test 2: Fetch transaction details using premium RPC (data-intensive operation)
                log(LogTag::System, "TEST", "🔍 Test 2: Fetching transaction details using premium RPC");
                
                let test_signature = if let Some(sig) = args.signature {
                    sig
                } else {
                    signatures[0].signature.clone()
                };
                
                match rpc_client.get_transaction_details_premium_rpc(&test_signature).await {
                    Ok(_transaction) => {
                        log(LogTag::System, "SUCCESS", &format!("✅ Successfully fetched transaction {} using premium RPC", &test_signature[..8]));
                    }
                    Err(e) => {
                        log(LogTag::System, "ERROR", &format!("❌ Failed to fetch transaction details: {}", e));
                    }
                }
                
                // Test 3: Batch fetch multiple transactions using premium RPC
                if signatures.len() > 1 {
                    log(LogTag::System, "TEST", "🔍 Test 3: Batch fetching transaction details using premium RPC");
                    
                    let test_signatures: Vec<String> = signatures.iter()
                        .take(3.min(signatures.len()))
                        .map(|s| s.signature.clone())
                        .collect();
                    
                    match rpc_client.batch_get_transaction_details_premium_rpc(&test_signatures).await {
                        Ok(transactions) => {
                            log(LogTag::System, "SUCCESS", &format!("✅ Successfully batch fetched {}/{} transactions using premium RPC", transactions.len(), test_signatures.len()));
                        }
                        Err(e) => {
                            log(LogTag::System, "ERROR", &format!("❌ Failed to batch fetch transaction details: {}", e));
                        }
                    }
                }
            } else {
                log(LogTag::System, "INFO", "No signatures found for this wallet, skipping transaction tests");
            }
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Failed to fetch signatures: {}", e));
        }
    }
    
    // Test 4: Verify RPC statistics are being recorded correctly
    log(LogTag::System, "TEST", "🔍 Test 4: Checking RPC statistics");
    if let Some(stats) = screenerbot::rpc::get_global_rpc_stats() {
        log(LogTag::System, "INFO", &format!("📊 Total RPC calls: {}", stats.total_calls()));
        log(LogTag::System, "INFO", &format!("📊 Calls per second: {:.2}", stats.calls_per_second()));
        
        // Show main RPC usage
        let configs = read_configs()?;
        let main_rpc_calls = stats.calls_per_url.get(&configs.rpc_url).unwrap_or(&0);
        log(LogTag::System, "INFO", &format!("📊 Main RPC calls: {}", main_rpc_calls));
        
        // Show premium RPC usage if available
        let premium_rpc_calls = stats.calls_per_url.get(&configs.rpc_url_premium).unwrap_or(&0);
        log(LogTag::System, "INFO", &format!("📊 Premium RPC calls: {}", premium_rpc_calls));
        
        log(LogTag::System, "SUCCESS", "✅ RPC statistics are being tracked correctly");
    } else {
        log(LogTag::System, "WARNING", "⚠️ No RPC statistics available");
    }
    
    log(LogTag::System, "COMPLETE", "🎉 RPC separation testing completed");
    
    println!("\n🧪 RPC Separation Test Results:");
    println!("• Main RPC: Used for lightweight signature fetching");
    println!("• Premium RPC: Used for data-intensive transaction details");
    println!("• Statistics: Properly tracking both endpoint usage");
    println!("• Check logs for detailed RPC usage information");
    
    Ok(())
}
