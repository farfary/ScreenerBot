/// Debug tool for detailed transaction analysis and detection verification
///
/// This tool provides comprehensive transaction analysis by:
/// - Fetching transaction details from the blockchain
/// - Running full transaction detection and classification
/// - Displaying balance changes, instruction analysis, and log parsing
/// - Showing detailed swap detection results and token information
/// - Analyzing ATA operations and rent calculations
/// - Verifying transaction types and router detection
///
/// All analysis uses the same transaction libs from src/ that the main bot uses.
/// This ensures debugging uses identical logic for accurate problem reproduction.
///
/// Features:
/// - Transaction fetching from RPC
/// - Complete transaction analysis pipeline
/// - Balance change analysis (SOL and tokens)
/// - Instruction breakdown with program IDs
/// - Log message analysis and pattern matching
/// - Swap detection for all supported DEXes
/// - Token mint extraction and validation
/// - ATA operation detection and rent calculation
/// - Router classification and verification
/// - Error analysis for failed transactions
///
/// Usage Examples:
/// cargo run --bin debug_transactions -- --signature <TX_SIGNATURE>
/// cargo run --bin debug_transactions -- --signature <TX_SIGNATURE> --verbose
/// cargo run --bin debug_transactions -- --signature <TX_SIGNATURE> --raw-data
use clap::Parser;
use screenerbot::arguments::set_cmd_args;
use screenerbot::rpc::get_rpc_client;
use screenerbot::tokens::TokenDatabase;
use screenerbot::transactions::TransactionsManager;
use screenerbot::transactions_types::*;
use screenerbot::utils::get_wallet_address;
use serde_json;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tokio;

#[derive(Parser, Debug)]
#[command(
    name = "debug_transactions",
    about = "Debug and analyze individual transactions with detailed breakdown"
)]
struct Args {
    /// Transaction signature to analyze
    #[arg(short, long)]
    signature: String,

    /// Show verbose output with detailed analysis steps
    #[arg(short, long)]
    verbose: bool,

    /// Show raw transaction data (JSON)
    #[arg(short, long)]
    raw_data: bool,

    /// Enable all debug logging
    #[arg(long)]
    debug: bool,

    /// Show balance changes in detail
    #[arg(long)]
    show_balances: bool,

    /// Show instruction breakdown
    #[arg(long)]
    show_instructions: bool,

    /// Show log message analysis
    #[arg(long)]
    show_logs: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Set debug flags if requested
    if args.debug {
        set_cmd_args(vec![
            "debug_transactions".to_string(),
            "--debug-transactions".to_string(),
        ]);
    }

    // Initialize logger - use basic print for now since init_logger might not exist
    println!("🔍 Starting Transaction Debug Tool...");

    println!("🔍 Transaction Debug Tool");
    println!("========================");
    println!("📝 Signature: {}", args.signature);
    println!();

    // Validate signature format
    if args.signature.len() < 44 || args.signature.len() > 88 {
        eprintln!("❌ Error: Invalid transaction signature format");
        eprintln!("   Expected: Base58 string (44-88 characters)");
        return Ok(());
    }

    // Initialize RPC client
    let rpc_client = get_rpc_client();

    // Initialize wallet for transaction manager
    let wallet_str = get_wallet_address()?;
    let wallet_pubkey = Pubkey::from_str(&wallet_str)?;
    println!("👛 Wallet: {}", wallet_str);
    println!();

    // Initialize tokens database (for future use)
    let _token_db = match TokenDatabase::new() {
        Ok(db) => db,
        Err(e) => {
            eprintln!("⚠️ Warning: Failed to initialize token database: {}", e);
            eprintln!("   Continuing without token database...");
            TokenDatabase::new()? // Use default
        }
    };

    // Create transaction manager (use real library analysis)
    let tx_manager = match TransactionsManager::new(wallet_pubkey).await {
        Ok(manager) => manager,
        Err(e) => {
            eprintln!("❌ Error creating transaction manager: {}", e);
            return Ok(());
        }
    };

    // Step 1: Fetch transaction from blockchain
    println!("🌐 Step 1: Fetching transaction from blockchain...");

    let tx_result = rpc_client.get_transaction_details(&args.signature).await;

    let raw_transaction = match tx_result {
        Ok(tx) => tx,
        Err(e) => {
            eprintln!("❌ Error fetching transaction: {}", e);
            eprintln!("   Possible causes:");
            eprintln!("   - Transaction signature not found");
            eprintln!("   - RPC endpoint issues");
            eprintln!("   - Transaction too old (pruned)");
            return Ok(());
        }
    };

    if args.raw_data {
        println!("📄 Raw Transaction Data:");
        println!("{}", serde_json::to_string_pretty(&raw_transaction)?);
        println!();
    }

    // Step 2: Process transaction using transactions lib
    println!("⚙️  Step 2: Processing transaction with transactions lib...");

    // Create a transaction object manually since we can't use the internal processing
    let mut transaction = Transaction {
        signature: args.signature.clone(),
        timestamp: chrono::Utc::now(),
        slot: None,
        block_time: None,
        status: TransactionStatus::Confirmed,
        transaction_type: TransactionType::Unknown,
        direction: TransactionDirection::Internal,
        success: false,
        fee_sol: 0.0,
        sol_balance_change: 0.0,
        sol_balance_changes: Vec::new(),
        token_transfers: Vec::new(),
        token_balance_changes: Vec::new(),
        instructions: Vec::new(),
        log_messages: Vec::new(),
        position_impact: None,
        profit_calculation: None,
        ata_analysis: None,
        token_info: None,
        token_symbol: None,
        token_decimals: None,
        calculated_token_price_sol: None,
        error_message: None,
        raw_transaction_data: Some(
            serde_json::to_value(&raw_transaction).unwrap_or(serde_json::Value::Null),
        ),
        last_updated: chrono::Utc::now(),
        cached_analysis: None,
    };

    println!("✅ Transaction object created");
    println!();

    // Step 4: Extract basic transaction information using the real library method
    println!("🔧 Step 4: Extracting basic transaction information...");
    if let Err(e) = tx_manager
        .extract_basic_transaction_info(&mut transaction)
        .await
    {
        eprintln!("❌ Error extracting basic info: {}", e);
        return Ok(());
    }

    print_basic_info(&transaction, args.verbose);

    // Step 5: Analyze transaction type using the real library method
    println!("🔍 Step 5: Analyzing transaction type...");
    if let Err(e) = tx_manager.analyze_transaction_type(&mut transaction).await {
        eprintln!("❌ Error analyzing transaction type: {}", e);
        return Ok(());
    }

    print_transaction_type(&transaction, args.verbose);

    // Step 6: Show balance changes if requested
    if args.show_balances || args.verbose {
        print_balance_changes(&transaction);
    }

    // Step 7: Show instructions if requested
    if args.show_instructions || args.verbose {
        print_instructions(&transaction);
    }

    // Step 8: Show log analysis if requested
    if args.show_logs || args.verbose {
        print_log_analysis(&transaction);
    }

    // Step 8.5: Show ATA analysis if verbose
    if args.verbose {
        print_ata_analysis(&transaction, true);
    }

    // Step 9: Final summary
    print_final_summary(&transaction);

    Ok(())
}

// Helper function to extract basic info from raw transaction data
fn extract_basic_info_from_raw(transaction: &mut Transaction) {
    if let Some(raw_data) = &transaction.raw_transaction_data {
        // Extract slot
        if let Some(slot) = raw_data.get("slot").and_then(|v| v.as_u64()) {
            transaction.slot = Some(slot);
        }

        // Extract block time
        if let Some(block_time) = raw_data.get("blockTime").and_then(|v| v.as_i64()) {
            transaction.block_time = Some(block_time);
        }

        // Extract meta information
        if let Some(meta) = raw_data.get("meta") {
            // Extract fee
            if let Some(fee) = meta.get("fee").and_then(|v| v.as_u64()) {
                transaction.fee_sol = (fee as f64) / 1_000_000_000.0;
            }

            // Calculate SOL balance change
            if let (Some(pre_balances), Some(post_balances)) = (
                meta.get("preBalances").and_then(|v| v.as_array()),
                meta.get("postBalances").and_then(|v| v.as_array()),
            ) {
                if !pre_balances.is_empty() && !post_balances.is_empty() {
                    let pre_balance = pre_balances[0].as_i64().unwrap_or(0);
                    let post_balance = post_balances[0].as_i64().unwrap_or(0);
                    transaction.sol_balance_change =
                        ((post_balance - pre_balance) as f64) / 1_000_000_000.0;
                }
            }

            // Check success
            transaction.success = meta.get("err").map_or(true, |v| v.is_null());

            // Extract error message
            if let Some(err) = meta.get("err") {
                if !err.is_null() {
                    transaction.error_message = Some(format!("{:?}", err));
                }
            }

            // Extract log messages
            if let Some(logs) = meta.get("logMessages").and_then(|v| v.as_array()) {
                transaction.log_messages = logs
                    .iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect();
            }
        }

        // Extract instructions
        if let Some(transaction_data) = raw_data.get("transaction") {
            if let Some(message) = transaction_data.get("message") {
                if let Some(instructions) = message.get("instructions").and_then(|v| v.as_array()) {
                    for (index, instruction) in instructions.iter().enumerate() {
                        let program_id = instruction
                            .get("programId")
                            .and_then(|v| v.as_str())
                            .unwrap_or("unknown")
                            .to_string();

                        let instruction_type = if let Some(parsed) = instruction.get("parsed") {
                            parsed
                                .get("type")
                                .and_then(|v| v.as_str())
                                .unwrap_or("parsed")
                                .to_string()
                        } else {
                            format!("instruction_{}", index)
                        };

                        transaction.instructions.push(InstructionInfo {
                            program_id,
                            instruction_type,
                            accounts: Vec::new(), // Simplified for now
                            data: instruction
                                .get("data")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string()),
                        });
                    }
                }
            }
        }
    }
}

// Helper function to analyze transaction type (simplified)
fn analyze_transaction_type_simple(transaction: &mut Transaction) {
    let log_text = transaction.log_messages.join(" ");

    // Simple pattern matching for common DEX patterns
    if log_text.contains("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P")
        || log_text.contains("pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA")
    {
        // Pump.fun
        if transaction.sol_balance_change < 0.0 {
            transaction.transaction_type = TransactionType::SwapSolToToken {
                router: "Pump.fun".to_string(),
                token_mint: "Unknown".to_string(),
                sol_amount: transaction.sol_balance_change.abs(),
                token_amount: 0.0,
            };
        } else if transaction.sol_balance_change > 0.0 {
            transaction.transaction_type = TransactionType::SwapTokenToSol {
                router: "Pump.fun".to_string(),
                token_mint: "Unknown".to_string(),
                token_amount: 0.0,
                sol_amount: transaction.sol_balance_change,
            };
        }
    } else if log_text.contains("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4") {
        // Jupiter
        if transaction.sol_balance_change < 0.0 {
            transaction.transaction_type = TransactionType::SwapSolToToken {
                router: "Jupiter".to_string(),
                token_mint: "Unknown".to_string(),
                sol_amount: transaction.sol_balance_change.abs(),
                token_amount: 0.0,
            };
        } else if transaction.sol_balance_change > 0.0 {
            transaction.transaction_type = TransactionType::SwapTokenToSol {
                router: "Jupiter".to_string(),
                token_mint: "Unknown".to_string(),
                token_amount: 0.0,
                sol_amount: transaction.sol_balance_change,
            };
        }
    } else if log_text.contains("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8")
        || log_text.contains("CPMMoo8L3")
    {
        // Raydium
        if transaction.sol_balance_change < 0.0 {
            transaction.transaction_type = TransactionType::SwapSolToToken {
                router: "Raydium".to_string(),
                token_mint: "Unknown".to_string(),
                sol_amount: transaction.sol_balance_change.abs(),
                token_amount: 0.0,
            };
        } else if transaction.sol_balance_change > 0.0 {
            transaction.transaction_type = TransactionType::SwapTokenToSol {
                router: "Raydium".to_string(),
                token_mint: "Unknown".to_string(),
                token_amount: 0.0,
                sol_amount: transaction.sol_balance_change,
            };
        }
    } else if log_text.contains("GMGN")
        || (transaction.sol_balance_change.abs() > 0.001
            && (log_text.contains("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL")
                || log_text.contains("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA")))
    {
        // GMGN or similar
        if transaction.sol_balance_change < 0.0 {
            transaction.transaction_type = TransactionType::SwapSolToToken {
                router: "GMGN".to_string(),
                token_mint: "Unknown".to_string(),
                sol_amount: transaction.sol_balance_change.abs(),
                token_amount: 0.0,
            };
        } else if transaction.sol_balance_change > 0.0 {
            transaction.transaction_type = TransactionType::SwapTokenToSol {
                router: "GMGN".to_string(),
                token_mint: "Unknown".to_string(),
                token_amount: 0.0,
                sol_amount: transaction.sol_balance_change,
            };
        }
    } else if transaction.sol_balance_change.abs() > 0.001
        && transaction
            .instructions
            .iter()
            .any(|i| i.program_id == "11111111111111111111111111111111")
    {
        // SOL transfer
        transaction.transaction_type = TransactionType::SolTransfer {
            amount: transaction.sol_balance_change.abs(),
            from: "unknown".to_string(),
            to: "unknown".to_string(),
        };
    }

    // Set direction based on transaction type
    transaction.direction = match &transaction.transaction_type {
        TransactionType::SwapSolToToken { .. } => TransactionDirection::Outgoing, // Buy (SOL out)
        TransactionType::SwapTokenToSol { .. } => TransactionDirection::Incoming, // Sell (SOL in)
        TransactionType::SolTransfer { .. } => {
            if transaction.sol_balance_change < 0.0 {
                TransactionDirection::Outgoing // Send
            } else {
                TransactionDirection::Incoming // Receive
            }
        }
        _ => TransactionDirection::Internal,
    };
}

fn print_basic_info(transaction: &Transaction, verbose: bool) {
    println!("📋 Basic Transaction Information:");
    println!("   Status: {:?}", transaction.status);
    println!(
        "   Success: {}",
        if transaction.success {
            "✅ Yes"
        } else {
            "❌ No"
        }
    );
    println!("   Fee: {:.9} SOL", transaction.fee_sol);
    println!(
        "   SOL Balance Change: {:.9} SOL",
        transaction.sol_balance_change
    );

    if let Some(slot) = transaction.slot {
        println!("   Slot: {}", slot);
    }

    if let Some(block_time) = transaction.block_time {
        println!("   Block Time: {}", block_time);
    }

    if let Some(ref error) = transaction.error_message {
        println!("   Error: {}", error);
    }

    if verbose {
        println!("   Timestamp: {}", transaction.timestamp);
        println!("   Instructions Count: {}", transaction.instructions.len());
        println!("   Log Messages Count: {}", transaction.log_messages.len());
        println!(
            "   Token Transfers Count: {}",
            transaction.token_transfers.len()
        );
        println!(
            "   Token Balance Changes Count: {}",
            transaction.token_balance_changes.len()
        );
    }

    println!();
}

fn print_transaction_type(transaction: &Transaction, _verbose: bool) {
    println!("🏷️  Transaction Type Analysis:");

    match &transaction.transaction_type {
        TransactionType::SwapSolToToken {
            router,
            token_mint,
            sol_amount,
            token_amount,
        } => {
            println!("   Type: 🟢 SOL → Token (BUY)");
            println!("   Router: {}", router);
            println!("   Token Mint: {}", token_mint);
            println!("   SOL Amount: {:.9}", sol_amount);
            println!("   Token Amount: {:.6}", token_amount);
        }
        TransactionType::SwapTokenToSol {
            router,
            token_mint,
            token_amount,
            sol_amount,
        } => {
            println!("   Type: 🔴 Token → SOL (SELL)");
            println!("   Router: {}", router);
            println!("   Token Mint: {}", token_mint);
            println!("   Token Amount: {:.6}", token_amount);
            println!("   SOL Amount: {:.9}", sol_amount);
        }
        TransactionType::SwapTokenToToken {
            router,
            from_mint,
            to_mint,
            from_amount,
            to_amount,
        } => {
            println!("   Type: 🔄 Token → Token");
            println!("   Router: {}", router);
            println!("   From Mint: {}", from_mint);
            println!("   To Mint: {}", to_mint);
            println!("   From Amount: {:.6}", from_amount);
            println!("   To Amount: {:.6}", to_amount);
        }
        TransactionType::SolTransfer { amount, from, to } => {
            println!("   Type: 💸 SOL Transfer");
            println!("   Amount: {:.9} SOL", amount);
            println!("   From: {}", from);
            println!("   To: {}", to);
        }
        TransactionType::TokenTransfer {
            mint,
            amount,
            from,
            to,
        } => {
            println!("   Type: 🪙 Token Transfer");
            println!("   Mint: {}", mint);
            println!("   Amount: {:.6}", amount);
            println!("   From: {}", from);
            println!("   To: {}", to);
        }
        TransactionType::AtaClose {
            recovered_sol,
            token_mint,
        } => {
            println!("   Type: 🔒 ATA Close");
            println!("   Recovered SOL: {:.9}", recovered_sol);
            println!("   Token Mint: {}", token_mint);
        }
        TransactionType::Other {
            description,
            details,
        } => {
            println!("   Type: ❓ Other");
            println!("   Description: {}", description);
            println!("   Details: {}", details);
        }
        TransactionType::Unknown => {
            println!("   Type: ❓ Unknown");
            println!("   ⚠️  Could not classify transaction type");
        }
    }

    println!();
}

fn print_ata_analysis(transaction: &Transaction, verbose: bool) {
    println!("🔍 ATA Closure Detection Analysis:");
    println!("===================================");

    // Check for closeAccount instructions in Token Programs
    let has_close_account_instruction = transaction.instructions.iter().any(|instruction| {
        (instruction.program_id == "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            || instruction.program_id == "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb")
            && instruction.instruction_type == "closeAccount"
    });

    // Check for closeAccount in logs
    let has_close_account_log = transaction
        .log_messages
        .iter()
        .any(|log| log.contains("Instruction: CloseAccount"));

    let has_close_account = has_close_account_instruction || has_close_account_log;

    println!("   📋 closeAccount Detection:");
    println!(
        "      ✅ In instructions: {}",
        has_close_account_instruction
    );
    println!("      ✅ In logs: {}", has_close_account_log);
    println!("      🎯 Overall result: {}", has_close_account);

    // Check for characteristic ATA rent recovery
    let rent_recovery = transaction.sol_balance_change;
    let is_rent_like = rent_recovery > 0.0 && rent_recovery >= 0.002 && rent_recovery <= 0.0025;

    println!("   💰 Rent Recovery Analysis:");
    println!("      💵 SOL change: {:.9}", rent_recovery);
    println!("      📊 Is positive: {}", rent_recovery > 0.0);
    println!(
        "      📏 In range [0.002, 0.0025]: {}",
        rent_recovery >= 0.002 && rent_recovery <= 0.0025
    );
    println!("      🎯 Rent-like: {}", is_rent_like);

    // Check for significant token trading
    let max_token_amount = transaction
        .token_transfers
        .iter()
        .map(|transfer| transfer.amount.abs())
        .fold(0.0, f64::max);

    let has_significant_token_trading = max_token_amount > 100.0;

    println!("   🪙 Token Trading Analysis:");
    println!(
        "      📊 Token transfers count: {}",
        transaction.token_transfers.len()
    );
    println!("      💰 Max token amount: {:.6}", max_token_amount);
    println!(
        "      🚫 Has significant trading (>100): {}",
        has_significant_token_trading
    );

    // Final ATA detection result
    let should_be_ata_close = has_close_account && is_rent_like && !has_significant_token_trading;

    println!("   🎯 Final ATA Detection Result:");
    println!("      ✅ closeAccount found: {}", has_close_account);
    println!("      ✅ Rent-like recovery: {}", is_rent_like);
    println!(
        "      ✅ No significant trading: {}",
        !has_significant_token_trading
    );
    println!("      🏆 SHOULD BE ATA CLOSE: {}", should_be_ata_close);

    // Show what the current classification is
    match &transaction.transaction_type {
        TransactionType::AtaClose { .. } => {
            println!("      🟢 Current classification: ATA Close (CORRECT)");
        }
        TransactionType::SwapTokenToSol { router, .. } => {
            println!(
                "      🔴 Current classification: {} Swap Token→SOL (INCORRECT!)",
                router
            );
        }
        TransactionType::SwapSolToToken { router, .. } => {
            println!(
                "      🔴 Current classification: {} Swap SOL→Token (INCORRECT!)",
                router
            );
        }
        _ => {
            println!(
                "      ⚪ Current classification: {:?}",
                transaction.transaction_type
            );
        }
    }

    if verbose {
        println!("\n   🔍 Detailed Instruction Analysis:");
        for (i, instruction) in transaction.instructions.iter().enumerate() {
            println!(
                "      Instruction {}: program_id='{}', type='{}'",
                i + 1,
                instruction.program_id,
                instruction.instruction_type
            );
        }

        println!("\n   📝 Relevant Log Messages:");
        for (i, log) in transaction.log_messages.iter().enumerate() {
            if log.contains("CloseAccount") || log.contains("TokenkegQ") {
                println!("      Log {}: {}", i + 1, log);
            }
        }
    }

    println!();
}

fn print_token_info(transaction: &Transaction, _verbose: bool) {
    println!("🪙 Token Information:");

    if let Some(ref token_info) = transaction.token_info {
        println!("   Mint: {}", token_info.mint);
        println!("   Symbol: {}", token_info.symbol);
        println!("   Decimals: {}", token_info.decimals);

        if let Some(price) = token_info.current_price_sol {
            println!("   Current Price: {:.12} SOL", price);
        } else {
            println!("   Current Price: Not available");
        }

        println!("   Is Verified: {}", token_info.is_verified);
    } else {
        println!("   No token information available");
    }

    if let Some(ref symbol) = transaction.token_symbol {
        println!("   Cached Symbol: {}", symbol);
    }

    if let Some(decimals) = transaction.token_decimals {
        println!("   Cached Decimals: {}", decimals);
    }

    if let Some(price) = transaction.calculated_token_price_sol {
        println!("   Calculated Price: {:.12} SOL", price);
    }

    println!();
}

fn print_balance_changes(transaction: &Transaction) {
    println!("💰 Balance Changes:");

    println!(
        "   SOL Balance Change: {:.9} SOL",
        transaction.sol_balance_change
    );

    if !transaction.sol_balance_changes.is_empty() {
        println!("   Detailed SOL Changes:");
        for (i, change) in transaction.sol_balance_changes.iter().enumerate() {
            println!(
                "     {}. Account: {} -> Change: {:.9} SOL",
                i + 1,
                &change.account[..8],
                change.change
            );
        }
    }

    if !transaction.token_balance_changes.is_empty() {
        println!("   Token Balance Changes:");
        for (i, change) in transaction.token_balance_changes.iter().enumerate() {
            println!(
                "     {}. Mint: {} -> Change: {:.6} (decimals: {})",
                i + 1,
                &change.mint[..8],
                change.change,
                change.decimals
            );
            if let Some(pre) = change.pre_balance {
                println!("        Pre: {:.6}", pre);
            }
            if let Some(post) = change.post_balance {
                println!("        Post: {:.6}", post);
            }
        }
    }

    if !transaction.token_transfers.is_empty() {
        println!("   Token Transfers:");
        for (i, transfer) in transaction.token_transfers.iter().enumerate() {
            println!(
                "     {}. Mint: {} -> Amount: {:.6}",
                i + 1,
                &transfer.mint[..8],
                transfer.amount
            );
            println!(
                "        From: {} To: {}",
                &transfer.from[..8],
                &transfer.to[..8]
            );
        }
    }

    println!();
}

fn print_instructions(transaction: &Transaction) {
    println!("🔧 Instructions Analysis:");

    if transaction.instructions.is_empty() {
        println!("   No instructions found");
        println!();
        return;
    }

    for (i, instruction) in transaction.instructions.iter().enumerate() {
        println!("   {}. Program ID: {}", i + 1, instruction.program_id);
        println!("      Type: {}", instruction.instruction_type);

        if !instruction.accounts.is_empty() {
            println!(
                "      Accounts ({}): {}",
                instruction.accounts.len(),
                instruction
                    .accounts
                    .iter()
                    .take(3)
                    .map(|acc| format!("{}...", &acc[..8]))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if instruction.accounts.len() > 3 {
                println!(
                    "                    ... and {} more",
                    instruction.accounts.len() - 3
                );
            }
        }

        if let Some(ref data) = instruction.data {
            println!("      Data: {}...", &data[..std::cmp::min(32, data.len())]);
        }
    }

    println!();
}

fn print_log_analysis(transaction: &Transaction) {
    println!("📝 Log Messages Analysis:");

    if transaction.log_messages.is_empty() {
        println!("   No log messages found");
        println!();
        return;
    }

    println!("   Total log messages: {}", transaction.log_messages.len());

    // Analyze log patterns
    let log_text = transaction.log_messages.join(" ");

    println!("   Pattern Analysis:");

    // DEX patterns
    let dex_patterns = [
        ("Jupiter", "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"),
        ("Pump.fun", "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"),
        (
            "Pump.fun (Legacy)",
            "pAMMBay6oceH9fJKBRHGP5D4bD4sWpmSwMn52FMfXEA",
        ),
        ("Raydium", "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"),
        ("Raydium CPMM", "CPMMoo8L3"),
        ("Orca", "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP"),
        ("Serum", "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin"),
        ("GMGN", "GMGNreQcJFufBiCTLDBgKhYEfEe9B454UjpDr5CaSLA1"),
    ];

    for (name, pattern) in &dex_patterns {
        if log_text.contains(pattern) {
            println!("     ✅ {} detected", name);
        }
    }

    // Instruction patterns
    let instruction_patterns = [
        ("Transfer", "Instruction: Transfer"),
        ("Swap", "Instruction: Swap"),
        ("Buy", "Instruction: Buy"),
        ("Sell", "Instruction: Sell"),
        ("Route", "Instruction: Route"),
        ("CloseAccount", "Instruction: CloseAccount"),
        ("CreateIdempotent", "CreateIdempotent"),
    ];

    for (name, pattern) in &instruction_patterns {
        if log_text.contains(pattern) {
            println!("     ✅ {} instruction detected", name);
        }
    }

    // Token patterns
    if log_text.contains("So11111111111111111111111111111111111111112") {
        println!("     ✅ WSOL operations detected");
    }

    if log_text.contains("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL") {
        println!("     ✅ ATA operations detected");
    }

    // Show first few log messages for context
    println!("   Sample Log Messages:");
    for (i, log) in transaction.log_messages.iter().take(5).enumerate() {
        let truncated = if log.len() > 100 {
            format!("{}...", &log[..100])
        } else {
            log.clone()
        };
        println!("     {}. {}", i + 1, truncated);
    }

    if transaction.log_messages.len() > 5 {
        println!(
            "     ... and {} more messages",
            transaction.log_messages.len() - 5
        );
    }

    println!();
}

fn print_final_summary(transaction: &Transaction) {
    println!("📊 Final Analysis Summary:");
    println!("=========================");

    // Classification success
    let is_classified = !matches!(transaction.transaction_type, TransactionType::Unknown);
    println!(
        "✅ Transaction Classification: {}",
        if is_classified {
            "Successfully classified"
        } else {
            "❌ Failed to classify"
        }
    );

    // Swap detection
    let is_swap = matches!(
        transaction.transaction_type,
        TransactionType::SwapSolToToken { .. }
            | TransactionType::SwapTokenToSol { .. }
            | TransactionType::SwapTokenToToken { .. }
    );

    if is_swap {
        println!("✅ Swap Detection: Detected as swap transaction");

        // Router detection
        let router = match &transaction.transaction_type {
            TransactionType::SwapSolToToken { router, .. }
            | TransactionType::SwapTokenToSol { router, .. }
            | TransactionType::SwapTokenToToken { router, .. } => router.clone(),
            _ => "Unknown".to_string(),
        };
        println!("✅ Router Detection: {}", router);

        // Token mint extraction
        let token_mint = match &transaction.transaction_type {
            TransactionType::SwapSolToToken { token_mint, .. }
            | TransactionType::SwapTokenToSol { token_mint, .. } => Some(token_mint.clone()),
            TransactionType::SwapTokenToToken { to_mint, .. } => Some(to_mint.clone()),
            _ => None,
        };

        if let Some(mint) = token_mint {
            println!("✅ Token Mint Extraction: {}", mint);
        } else {
            println!("❌ Token Mint Extraction: Failed");
        }
    } else {
        println!("ℹ️  Swap Detection: Not a swap transaction");
    }

    // Balance analysis
    println!("✅ Balance Analysis:");
    println!("   SOL Change: {:.9} SOL", transaction.sol_balance_change);
    println!("   Fee: {:.9} SOL", transaction.fee_sol);

    // Token information (simplified since we don't have full integration)
    println!("ℹ️  Token Information: Limited (would need full transaction lib integration)");

    if let Some(ref token_info) = transaction.token_info {
        println!("✅ Token Information:");
        println!("   Symbol: {}", token_info.symbol);
        println!("   Decimals: {}", token_info.decimals);
        if let Some(price) = token_info.current_price_sol {
            println!("   Current Price: {:.12} SOL", price);
        }
    }

    // Success/failure analysis
    if transaction.success {
        println!("✅ Transaction Status: Successful");
    } else {
        println!("❌ Transaction Status: Failed");
        if let Some(ref error) = transaction.error_message {
            println!("   Error: {}", error);
        }
    }

    println!();
    println!("🎯 Analysis Complete!");
}
