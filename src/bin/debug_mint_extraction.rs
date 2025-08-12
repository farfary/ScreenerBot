/// Debug tool to test token mint extraction from specific transactions
use clap::{Arg, Command};
use screenerbot::logger::{log, LogTag};
use screenerbot::transactions_manager::TransactionsManager;
use screenerbot::utils::get_wallet_address;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    
    let matches = Command::new("debug_mint_extraction")
        .about("Debug token mint extraction from transaction analysis")
        .arg(Arg::new("signature")
            .short('s')
            .long("signature")
            .value_name("SIGNATURE")
            .help("Transaction signature to analyze")
            .required(true))
        .get_matches();

    let signature = matches.get_one::<String>("signature").unwrap();
    
    log(LogTag::Transactions, "INFO", &format!("🔍 Debugging mint extraction for transaction: {}", &signature[..12]));
    
    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to get wallet address: {}", e));
            return;
        }
    };
    
    let wallet_pubkey = match Pubkey::from_str(&wallet_address) {
        Ok(pubkey) => pubkey,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Invalid wallet address: {}", e));
            return;
        }
    };
    
    // Initialize TransactionsManager
    let mut manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to initialize TransactionsManager: {}", e));
            return;
        }
    };
    
    // Check if TokenDatabase is initialized
    log(LogTag::Transactions, "INFO", &format!("TokenDatabase initialized: {}", manager.token_database.is_some()));
    
    // Process the transaction
    match manager.process_transaction(signature).await {
        Ok(transaction) => {
            log(LogTag::Transactions, "SUCCESS", &format!("✅ Transaction processed successfully"));
            
            // Debug transaction type
            log(LogTag::Transactions, "DEBUG", &format!("Transaction type: {:?}", transaction.transaction_type));
            
            // Debug token symbol
            log(LogTag::Transactions, "DEBUG", &format!("Token symbol: {:?}", transaction.token_symbol));
            
            // Debug token mint extraction
            if let Some(mint) = manager.extract_token_mint_from_transaction(&transaction) {
                log(LogTag::Transactions, "DEBUG", &format!("Extracted mint: {}", mint));
                
                // Test direct database lookup
                if let Some(ref db) = manager.token_database {
                    match db.get_token_by_mint(&mint) {
                        Ok(Some(token_info)) => {
                            log(LogTag::Transactions, "DEBUG", &format!("Database lookup successful: {}", token_info.symbol));
                        }
                        Ok(None) => {
                            log(LogTag::Transactions, "DEBUG", "Token not found in database");
                        }
                        Err(e) => {
                            log(LogTag::Transactions, "ERROR", &format!("Database lookup error: {}", e));
                        }
                    }
                } else {
                    log(LogTag::Transactions, "ERROR", "TokenDatabase is None!");
                }
            } else {
                log(LogTag::Transactions, "ERROR", "Failed to extract token mint from transaction");
            }
        }
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("❌ Failed to process transaction: {}", e));
        }
    }
}
