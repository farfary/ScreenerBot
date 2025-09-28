use clap::{Arg, Command};
use screenerbot::{
    arguments::set_cmd_args,
    logger::{log, LogTag},
    positions::Position,
    positions::{get_db_closed_positions, get_db_open_positions},
    tokens::get_token_decimals,
    transactions::{get_global_transaction_manager, get_transaction},
    utils::safe_truncate,
};
use std::collections::HashMap;
use tokio;

/// Debug tool to verify position token amounts match transaction analysis
/// This tool checks if positions are storing correct token amounts from transaction verification
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = Command::new("Debug Position Token Amounts")
        .about("Verify that positions store correct token amounts from transactions")
        .arg(
            Arg::new("signature")
                .long("signature")
                .value_name("SIGNATURE")
                .help("Check specific transaction signature")
                .required(false),
        )
        .arg(
            Arg::new("mint")
                .long("mint")
                .value_name("MINT")
                .help("Check positions for specific token mint")
                .required(false),
        )
        .arg(
            Arg::new("all-positions")
                .long("all-positions")
                .help("Check all positions for token amount accuracy")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("verbose")
                .short('v')
                .long("verbose")
                .help("Enable verbose output")
                .action(clap::ArgAction::SetTrue),
        )
        .get_matches();

    let args = std::env::args().collect::<Vec<String>>();
    set_cmd_args(args);

    let verbose = matches.get_flag("verbose");

    log(
        LogTag::System,
        "INFO",
        "🔍 Starting position token amount verification",
    );

    if let Some(signature) = matches.get_one::<String>("signature") {
        verify_single_transaction(signature, verbose).await?;
    } else if let Some(mint) = matches.get_one::<String>("mint") {
        verify_positions_for_mint(mint, verbose).await?;
    } else if matches.get_flag("all-positions") {
        verify_all_positions(verbose).await?;
    } else {
        println!("Please specify --signature, --mint, or --all-positions");
        return Ok(());
    }

    Ok(())
}

/// Verify token amounts for a specific transaction
async fn verify_single_transaction(
    signature: &str,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    log(
        LogTag::System,
        "INFO",
        &format!("🔍 Verifying transaction: {}", signature),
    );

    // Get transaction from system
    let transaction = match get_transaction(signature).await {
        Ok(Some(tx)) => tx,
        Ok(None) => {
            println!("❌ Transaction not found: {}", signature);
            return Ok(());
        }
        Err(e) => {
            println!("❌ Error getting transaction: {}", e);
            return Ok(());
        }
    };

    println!("\n📊 TRANSACTION ANALYSIS");
    println!("=======================");
    println!("Signature: {}", transaction.signature);
    println!("Success: {}", transaction.success);
    println!("Type: {:?}", transaction.transaction_type);
    println!(
        "SOL Balance Change: {:.9} SOL",
        transaction.sol_balance_change
    );

    if !transaction.token_balance_changes.is_empty() {
        println!("\n🪙 TOKEN BALANCE CHANGES:");
        for (i, change) in transaction.token_balance_changes.iter().enumerate() {
            println!("  {}. Mint: {}", i + 1, safe_truncate(&change.mint, 12));
            println!(
                "     Change: {:.9} tokens (decimals: {})",
                change.change, change.decimals
            );
            if let Some(pre) = change.pre_balance {
                println!("     Pre: {:.9}", pre);
            }
            if let Some(post) = change.post_balance {
                println!("     Post: {:.9}", post);
            }
        }
    }

    if !transaction.token_transfers.is_empty() {
        println!("\n💸 TOKEN TRANSFERS:");
        for (i, transfer) in transaction.token_transfers.iter().enumerate() {
            println!("  {}. Mint: {}", i + 1, safe_truncate(&transfer.mint, 12));
            println!("     Amount: {:.9}", transfer.amount);
            println!(
                "     From: {} To: {}",
                safe_truncate(&transfer.from, 8),
                safe_truncate(&transfer.to, 8)
            );
        }
    }

    // Create SwapPnLInfo to see how transaction is analyzed
    if let Some(global_manager) = get_global_transaction_manager().await {
        let manager = global_manager.lock().await;
        let empty_cache = HashMap::new();
        if let Some(swap_info) = manager.convert_to_swap_pnl_info(&transaction, &empty_cache, false)
        {
            println!("\n📈 SWAP ANALYSIS (SwapPnLInfo):");
            println!("  Type: {}", swap_info.swap_type);
            println!("  Token Mint: {}", safe_truncate(&swap_info.token_mint, 12));
            println!("  Token Amount: {:.9}", swap_info.token_amount);
            println!("  SOL Amount: {:.9}", swap_info.sol_amount);
            println!("  Price: {:.12} SOL/token", swap_info.calculated_price_sol);
            println!("  Router: {}", swap_info.router);
            println!("  Fee: {:.9} SOL", swap_info.fee_sol);
            println!("  ATA Rents: {:.9} SOL", swap_info.ata_rents);

            // Check decimal conversion
            if let Some(decimals) = get_token_decimals(&swap_info.token_mint).await {
                let token_amount_units =
                    (swap_info.token_amount.abs() * (10_f64).powi(decimals as i32)) as u64;
                println!(
                    "  Token Amount (units): {} (with {} decimals)",
                    token_amount_units, decimals
                );

                // Verify the reverse conversion
                let converted_back = (token_amount_units as f64) / (10_f64).powi(decimals as i32);
                let precision_error = (converted_back - swap_info.token_amount.abs()).abs();

                if precision_error > 1e-9 {
                    println!(
                        "  ⚠️  PRECISION WARNING: Conversion error = {:.12}",
                        precision_error
                    );
                } else {
                    println!("  ✅ Decimal conversion accurate");
                }
            } else {
                println!("  ❌ Token decimals not found");
            }
        } else {
            println!("\n❌ Could not convert transaction to SwapPnLInfo");
        }
    }

    // Check if this transaction is associated with any positions
    println!("\n🎯 POSITION ASSOCIATIONS:");
    let open_positions = get_db_open_positions().await.unwrap_or_default();
    let closed_positions = get_db_closed_positions().await.unwrap_or_default();

    let mut found_positions = false;

    // Check open positions
    for position in &open_positions {
        if position.entry_transaction_signature.as_deref() == Some(signature)
            || position.exit_transaction_signature.as_deref() == Some(signature)
        {
            found_positions = true;
            print_position_token_info(&position, verbose).await;
        }
    }

    // Check closed positions
    for position in &closed_positions {
        if position.entry_transaction_signature.as_deref() == Some(signature)
            || position.exit_transaction_signature.as_deref() == Some(signature)
        {
            found_positions = true;
            print_position_token_info(&position, verbose).await;
        }
    }

    if !found_positions {
        println!("  No positions found for this transaction");
    }

    Ok(())
}

/// Verify token amounts for all positions for a specific mint
async fn verify_positions_for_mint(
    mint: &str,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    log(
        LogTag::System,
        "INFO",
        &format!(
            "🔍 Verifying positions for mint: {}",
            safe_truncate(mint, 12)
        ),
    );

    let open_positions = get_db_open_positions().await.unwrap_or_default();
    let closed_positions = get_db_closed_positions().await.unwrap_or_default();

    let mut found_positions = false;

    // Check open positions
    for position in &open_positions {
        if position.mint == mint {
            found_positions = true;
            println!("\n🟢 OPEN POSITION:");
            verify_position_token_amounts(&position, verbose).await?;
        }
    }

    // Check closed positions
    for position in &closed_positions {
        if position.mint == mint {
            found_positions = true;
            println!("\n🔴 CLOSED POSITION:");
            verify_position_token_amounts(&position, verbose).await?;
        }
    }

    if !found_positions {
        println!(
            "❌ No positions found for mint: {}",
            safe_truncate(mint, 12)
        );
    }

    Ok(())
}

/// Verify token amounts for all positions
async fn verify_all_positions(verbose: bool) -> Result<(), Box<dyn std::error::Error>> {
    log(
        LogTag::System,
        "INFO",
        "🔍 Verifying all positions for token amount accuracy",
    );

    let open_positions = get_db_open_positions().await.unwrap_or_default();
    let closed_positions = get_db_closed_positions().await.unwrap_or_default();

    let mut total_positions = 0;
    let mut verified_positions = 0;
    let mut problematic_positions = 0;

    println!("\n🟢 OPEN POSITIONS:");
    println!("==================");
    for position in &open_positions {
        total_positions += 1;
        match verify_position_token_amounts(&position, verbose).await {
            Ok(true) => {
                verified_positions += 1;
            }
            Ok(false) => {
                problematic_positions += 1;
            }
            Err(e) => {
                println!("❌ Error verifying position {}: {}", position.symbol, e);
                problematic_positions += 1;
            }
        }
    }

    println!("\n🔴 CLOSED POSITIONS:");
    println!("===================");
    for position in &closed_positions {
        total_positions += 1;
        match verify_position_token_amounts(&position, verbose).await {
            Ok(true) => {
                verified_positions += 1;
            }
            Ok(false) => {
                problematic_positions += 1;
            }
            Err(e) => {
                println!("❌ Error verifying position {}: {}", position.symbol, e);
                problematic_positions += 1;
            }
        }
    }

    println!("\n📊 VERIFICATION SUMMARY:");
    println!("========================");
    println!("Total positions: {}", total_positions);
    println!("Verified OK: {}", verified_positions);
    println!("Problematic: {}", problematic_positions);
    println!(
        "Success rate: {:.1}%",
        if total_positions > 0 {
            ((verified_positions as f64) / (total_positions as f64)) * 100.0
        } else {
            0.0
        }
    );

    if problematic_positions > 0 {
        println!(
            "\n⚠️  Found {} positions with potential token amount issues",
            problematic_positions
        );
    } else {
        println!("\n✅ All positions have correct token amounts");
    }

    Ok(())
}

/// Verify token amounts for a specific position
async fn verify_position_token_amounts(
    position: &Position,
    verbose: bool,
) -> Result<bool, Box<dyn std::error::Error>> {
    if verbose {
        print_position_token_info(position, verbose).await;
    }

    let mut is_valid = true;

    // Check if entry transaction has matching token amounts
    if let Some(ref entry_sig) = position.entry_transaction_signature {
        if let Ok(Some(entry_tx)) = get_transaction(entry_sig).await {
            if let Some(global_manager) = get_global_transaction_manager().await {
                let manager = global_manager.lock().await;
                let empty_cache = HashMap::new();
                if let Some(swap_info) =
                    manager.convert_to_swap_pnl_info(&entry_tx, &empty_cache, true)
                {
                    // Compare stored token amount with calculated amount
                    if let Some(stored_amount) = position.token_amount {
                        if let Some(decimals) = get_token_decimals(&position.mint).await {
                            let calculated_units = (swap_info.token_amount.abs()
                                * (10_f64).powi(decimals as i32))
                                as u64;
                            let difference =
                                ((stored_amount as i64) - (calculated_units as i64)).abs();
                            let tolerance = ((calculated_units as f64) * 0.001) as i64; // 0.1% tolerance

                            if difference > tolerance.max(1) {
                                println!("⚠️  TOKEN AMOUNT MISMATCH for {}:", position.symbol);
                                println!("  Stored: {} units", stored_amount);
                                println!("  Calculated: {} units", calculated_units);
                                println!("  Difference: {} units", difference);
                                println!("  Tolerance: {} units", tolerance);
                                is_valid = false;
                            } else if verbose {
                                println!("✅ Token amount matches for {}", position.symbol);
                            }
                        }
                    } else if verbose {
                        println!("⚠️  No token amount stored for {}", position.symbol);
                    }
                }
            }
        }
    }

    Ok(is_valid)
}

/// Print position token information
async fn print_position_token_info(position: &Position, verbose: bool) {
    println!("  Symbol: {}", position.symbol);
    println!("  Mint: {}", safe_truncate(&position.mint, 12));
    println!("  Entry Price: {:.12} SOL", position.entry_price);
    println!("  Entry Size: {:.9} SOL", position.entry_size_sol);

    if let Some(token_amount) = position.token_amount {
        println!("  Token Amount (stored): {} units", token_amount);

        if let Some(decimals) = get_token_decimals(&position.mint).await {
            let ui_amount = (token_amount as f64) / (10_f64).powi(decimals as i32);
            println!(
                "  Token Amount (UI): {:.9} tokens (decimals: {})",
                ui_amount, decimals
            );

            if ui_amount > 0.0 {
                let calculated_price = position.entry_size_sol / ui_amount;
                println!("  Calculated Price: {:.12} SOL/token", calculated_price);

                if let Some(effective_price) = position.effective_entry_price {
                    let price_diff = (calculated_price - effective_price).abs();
                    if price_diff > effective_price * 0.01 {
                        println!(
                            "  ⚠️  Price mismatch: stored={:.12}, calculated={:.12}",
                            effective_price, calculated_price
                        );
                    }
                }
            }
        } else {
            println!("  ❌ Decimals not found for token");
        }
    } else {
        println!("  ❌ No token amount stored");
    }

    if let Some(ref entry_sig) = position.entry_transaction_signature {
        println!("  Entry TX: {}", safe_truncate(entry_sig, 16));
    }

    if let Some(ref exit_sig) = position.exit_transaction_signature {
        println!("  Exit TX: {}", safe_truncate(exit_sig, 16));
    }

    if verbose {
        println!(
            "  Verified: entry={}, exit={}",
            position.transaction_entry_verified, position.transaction_exit_verified
        );
    }

    println!();
}
