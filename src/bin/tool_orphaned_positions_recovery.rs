/// Orphaned Positions Recovery Tool
/// Finds successful swap transactions that didn't create positions due to confirmation timeouts
/// Provides option to create positions retroactively

use screenerbot::{
    configs::{read_configs, validate_configs, load_wallet_from_config},
    rpc::get_rpc_client,
    logger::{log, LogTag},
    transactions::{TransactionsManager},
    positions::{Position, SAVED_POSITIONS},
    utils::save_positions_to_file,
    tokens::{TokenDatabase, initialize_price_service, price::get_token_price_blocking_safe},
};
use solana_sdk::signature::Signer;
use std::collections::HashSet;

#[tokio::main]
async fn main() {
    // Initialize file logging
    screenerbot::logger::init_file_logging();

    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        print_usage();
        return;
    }

    let analyze_mode = args.len() > 1 && args[1] == "--analyze";
    let recovery_mode = args.len() > 1 && args[1] == "--recover";

    if !analyze_mode && !recovery_mode {
        print_usage();
        return;
    }

    log(LogTag::System, "INFO", "Starting Orphaned Positions Recovery Tool");

    // Load configuration
    let configs = match read_configs() {
        Ok(configs) => configs,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to read configs: {}", e));
            return;
        }
    };

    if let Err(e) = validate_configs(&configs) {
        log(LogTag::System, "ERROR", &format!("Config validation failed: {}", e));
        return;
    }

    let wallet = match load_wallet_from_config(&configs) {
        Ok(wallet) => wallet,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to load wallet: {}", e));
            return;
        }
    };

    let wallet_pubkey = wallet.pubkey();
    log(LogTag::System, "INFO", &format!("Loaded wallet: {}", wallet_pubkey));

    // Initialize dependencies
    let _rpc_client = get_rpc_client();
    log(LogTag::System, "SUCCESS", "Global RPC client initialized from configuration");

    initialize_price_service().await;
    log(LogTag::System, "INIT", "Price service initialized successfully");

    let token_db = match TokenDatabase::new() {
        Ok(db) => db,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to create token database: {}", e));
            return;
        }
    };
    log(LogTag::System, "DATABASE", "Token database initialized");

    // Create transaction manager
    let mut tx_manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to create transaction manager: {}", e));
            return;
        }
    };

    if analyze_mode {
        analyze_orphaned_transactions(&mut tx_manager).await;
    } else if recovery_mode {
        recover_orphaned_positions(&mut tx_manager).await;
    }

    log(LogTag::System, "INFO", "Orphaned Positions Recovery Tool completed");
}

fn print_usage() {
    println!("Orphaned Positions Recovery Tool");
    println!();
    println!("USAGE:");
    println!("  cargo run --bin tool_orphaned_positions_recovery -- [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("  --analyze     Analyze recent transactions for orphaned positions");
    println!("  --recover     Attempt to recover orphaned positions");
    println!("  --help, -h    Show this help message");
    println!();
    println!("DESCRIPTION:");
    println!("  This tool finds successful swap transactions that didn't create positions");
    println!("  due to confirmation timeouts, and provides recovery options.");
}

async fn analyze_orphaned_transactions(tx_manager: &mut TransactionsManager) {
    log(LogTag::Transactions, "ANALYZE", "🔍 Analyzing recent transactions for orphaned positions...");

    // Load existing positions to compare against
    let existing_positions = match SAVED_POSITIONS.lock() {
        Ok(positions) => positions.clone(),
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load positions: {}", e));
            return;
        }
    };

    let existing_signatures: HashSet<String> = existing_positions.iter()
        .filter_map(|p| p.entry_transaction_signature.clone())
        .collect();

    log(LogTag::Transactions, "INFO", &format!("Found {} existing positions with signatures", existing_signatures.len()));

    // Get recent transactions from the manager
    let recent_transactions = tx_manager.get_recent_transactions(50).await.unwrap_or_default();
    log(LogTag::Transactions, "INFO", &format!("Analyzing {} recent transactions", recent_transactions.len()));

    let mut orphaned_count = 0;

    for transaction in recent_transactions {
        // Skip if this transaction already has a position
        if existing_signatures.contains(&transaction.signature) {
            continue;
        }

        // Check if this is a successful SOL-to-token swap based on transaction type
        if transaction.success {
            match &transaction.transaction_type {
                screenerbot::transactions::TransactionType::SwapSolToToken { 
                    token_mint, sol_amount, token_amount, router 
                } => {
                    orphaned_count += 1;
                    
                    log(LogTag::Transactions, "ORPHANED", &format!(
                        "🔍 ORPHANED SWAP FOUND: {} | Token: {} | SOL: {:.6} | Tokens: {:.2} | Router: {} | Time: {}",
                        &transaction.signature[..12],
                        &token_mint[..8],
                        sol_amount,
                        token_amount,
                        router,
                        transaction.timestamp.format("%Y-%m-%d %H:%M:%S")
                    ));
                }
                _ => continue,
            }
        }
    }

    if orphaned_count == 0 {
        log(LogTag::Transactions, "SUCCESS", "✅ No orphaned transactions found - all swaps have corresponding positions");
    } else {
        log(LogTag::Transactions, "WARNING", &format!(
            "⚠️ Found {} orphaned successful swaps without positions", orphaned_count
        ));
        log(LogTag::Transactions, "INFO", "Run with --recover to attempt automatic position creation");
    }
}

async fn recover_orphaned_positions(tx_manager: &mut TransactionsManager) {
    log(LogTag::Transactions, "RECOVER", "🔧 Starting orphaned position recovery...");

    // Load existing positions
    let mut existing_positions = match SAVED_POSITIONS.lock() {
        Ok(positions) => positions.clone(),
        Err(e) => {
            log(LogTag::Transactions, "ERROR", &format!("Failed to load positions: {}", e));
            return;
        }
    };

    let existing_signatures: HashSet<String> = existing_positions.iter()
        .filter_map(|p| p.entry_transaction_signature.clone())
        .collect();

    // Get recent transactions
    let recent_transactions = tx_manager.get_recent_transactions(50).await.unwrap_or_default();
    let mut recovered_count = 0;

    for transaction in recent_transactions {
        // Skip if position already exists
        if existing_signatures.contains(&transaction.signature) {
            continue;
        }

        // Check if this is a successful SOL-to-token swap
        if transaction.success {
            if let screenerbot::transactions::TransactionType::SwapSolToToken { 
                token_mint, sol_amount, token_amount, router: _ 
            } = &transaction.transaction_type {
                log(LogTag::Transactions, "RECOVERING", &format!(
                    "🔧 Recovering position for transaction: {}", &transaction.signature[..12]
                ));

                // Create position from transaction data
                if let Some(position) = create_position_from_transaction(&transaction, token_mint, *sol_amount, *token_amount).await {
                    existing_positions.push(position.clone());
                    recovered_count += 1;

                    log(LogTag::Transactions, "RECOVERED", &format!(
                        "✅ Position recovered: {} | TX: {} | SOL: {:.6} | Tokens: {:.2}",
                        position.symbol,
                        &transaction.signature[..12],
                        sol_amount,
                        token_amount
                    ));
                } else {
                    log(LogTag::Transactions, "ERROR", &format!(
                        "❌ Failed to create position for transaction: {}", &transaction.signature[..12]
                    ));
                }
            }
        }
    }

    if recovered_count > 0 {
        // Save updated positions
        save_positions_to_file(&existing_positions);
        log(LogTag::Transactions, "SUCCESS", &format!(
            "✅ Successfully recovered {} orphaned positions and saved to disk", recovered_count
        ));
    } else {
        log(LogTag::Transactions, "INFO", "No orphaned positions found to recover");
    }
}

async fn create_position_from_transaction(
    transaction: &screenerbot::transactions::Transaction,
    token_mint: &str,
    sol_amount: f64,
    token_amount: f64
) -> Option<Position> {
    // Calculate effective entry price
    let effective_entry_price = if token_amount > 0.0 {
        sol_amount / token_amount
    } else {
        0.0
    };

    // Get current price for profit calculation
    let current_price = get_token_price_blocking_safe(token_mint).await.unwrap_or(effective_entry_price);

    // Create position structure using minimal token information
    Some(Position {
        mint: token_mint.to_string(),
        symbol: format!("TOKEN_{}", &token_mint[..8]),
        name: format!("Unknown Token {}", &token_mint[..8]),
        entry_price: current_price, // Use current market price as signal price
        entry_time: transaction.timestamp,
        exit_price: None,
        exit_time: None,
        position_type: "buy".to_string(),
        entry_size_sol: 0.005, // Standard trade size
        total_size_sol: sol_amount,
        price_highest: current_price,
        price_lowest: current_price,
        entry_transaction_signature: Some(transaction.signature.clone()),
        exit_transaction_signature: None,
        token_amount: Some(token_amount as u64),
        effective_entry_price: Some(effective_entry_price),
        effective_exit_price: None,
        sol_received: None,
        profit_target_min: Some(20.0), // Default profit targets
        profit_target_max: Some(50.0),
        liquidity_tier: Some("unknown".to_string()), // Default tier since we don't have token data
        transaction_entry_verified: true, // Mark as verified since we're creating from verified transaction
        transaction_exit_verified: false,
        entry_fee_lamports: Some((transaction.fee_sol * 1_000_000_000.0) as u64),
        exit_fee_lamports: None,
    })
}
