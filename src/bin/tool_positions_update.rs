use screenerbot::logger::{ log, LogTag, init_file_logging };
use screenerbot::rpc::{ SwapError, get_rpc_client, init_rpc_client };
use screenerbot::wallet::{ get_wallet_address };
use screenerbot::utils::{ load_positions_from_file, save_positions_to_file };
use screenerbot::positions::Position;
use screenerbot::tokens::{ get_token_price_safe, initialize_price_service };

use chrono::Utc;
use colored::Colorize;

/// Print help menu for the Positions Update Tool
fn print_help() {
    println!("📊 Positions Update Tool");
    println!("=================================");
    println!(
        "Checks all open positions against actual wallet token balances and updates positions.json."
    );
    println!("Automatically closes positions where tokens have been sold (zero balance detected).");
    println!("");
    println!("USAGE:");
    println!("    cargo run --bin tool_positions_update [OPTIONS]");
    println!("");
    println!("OPTIONS:");
    println!("    --help, -h          Show this help message");
    println!("    --dry-run          Analyze positions without updating positions.json");
    println!("    --force            Execute actual position updates and save to file");
    println!("    --verbose          Enable detailed logging for each position");
    println!(
        "    --min-balance NUM  Minimum token balance to consider position open (default: 100)"
    );
    println!("");
    println!("EXAMPLES:");
    println!("    # Analyze all positions without making changes");
    println!("    cargo run --bin tool_positions_update -- --dry-run");
    println!("");
    println!("    # Update positions and save changes to positions.json");
    println!("    cargo run --bin tool_positions_update -- --force");
    println!("");
    println!("    # Detailed analysis with verbose output");
    println!("    cargo run --bin tool_positions_update -- --dry-run --verbose");
    println!("");
    println!("SAFETY FEATURES:");
    println!("    🟢 Low-Risk: Read-only operations, no wallet transactions");
    println!("    • Only checks token balances via RPC calls");
    println!("    • Updates positions.json based on actual wallet state");
    println!("    • Preserves all historical position data");
    println!("    • Automatic backup of positions.json before changes");
    println!("    • Multi-RPC fallback for reliable balance checking");
    println!("");
    println!("OUTPUT:");
    println!("    • Analysis of all open positions vs wallet balances");
    println!("    • Identification of positions with zero/low token balances");
    println!("    • Current market prices for position P&L calculation");
    println!("    • Summary of positions marked for closure");
    println!("");
    println!("POSITION CLOSURE CRITERIA:");
    println!("    • Token balance below minimum threshold (default: 100 tokens)");
    println!("    • No exit price/time already recorded");
    println!("    • Uses current market price as estimated exit price");
    println!("    • Sets exit time to current timestamp");
    println!("");
}

/// Backup positions file before making changes
fn backup_positions_file() -> Result<(), Box<dyn std::error::Error>> {
    let backup_filename = format!("positions_backup_{}.json", Utc::now().format("%Y%m%d_%H%M%S"));
    std::fs::copy("positions.json", &backup_filename)?;
    log(LogTag::System, "BACKUP", &format!("Created backup: {}", backup_filename));
    Ok(())
}

/// Check token balance for a specific mint address
async fn get_token_balance_for_position(
    wallet_address: &str,
    mint: &str,
    verbose: bool
) -> Result<u64, SwapError> {
    if verbose {
        log(LogTag::System, "BALANCE_CHECK", &format!("Checking balance for mint: {}", mint));
    }

    let rpc_client = get_rpc_client();
    let balance = rpc_client.get_token_balance(wallet_address, mint).await?;

    if verbose {
        log(LogTag::System, "BALANCE_RESULT", &format!("Balance for {}: {} tokens", mint, balance));
    }

    Ok(balance)
}

/// Update a position to closed status
fn close_position(position: &mut Position, current_price: f64, verbose: bool) {
    if verbose {
        log(
            LogTag::System,
            "CLOSING",
            &format!(
                "Closing position {} ({}) with current price: {:.10}",
                position.symbol,
                position.mint,
                current_price
            )
        );
    }

    position.exit_price = Some(current_price);
    position.exit_time = Some(Utc::now());
    position.effective_exit_price = Some(current_price);

    // Calculate estimated SOL received based on current price and token amount
    if let Some(token_amount) = position.token_amount {
        let sol_received = (token_amount as f64) * current_price;
        position.sol_received = Some(sol_received);

        if verbose {
            log(
                LogTag::System,
                "SOL_CALC",
                &format!(
                    "Estimated SOL received: {:.10} (tokens: {} * price: {:.10})",
                    sol_received,
                    token_amount,
                    current_price
                )
            );
        }
    }
}

/// Calculate P&L for a position
fn calculate_pnl(position: &Position) -> (f64, f64) {
    let entry_price = position.effective_entry_price.unwrap_or(position.entry_price);
    let entry_sol = position.entry_size_sol;

    if let Some(exit_price) = position.exit_price {
        if let Some(sol_received) = position.sol_received {
            // Use actual SOL received for closed positions
            let pnl_sol = sol_received - entry_sol;
            let pnl_percent = (pnl_sol / entry_sol) * 100.0;
            (pnl_sol, pnl_percent)
        } else {
            // Calculate based on price change
            let pnl_percent = ((exit_price - entry_price) / entry_price) * 100.0;
            let pnl_sol = (pnl_percent / 100.0) * entry_sol;
            (pnl_sol, pnl_percent)
        }
    } else {
        // Open position - no P&L calculation
        (0.0, 0.0)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger
    init_file_logging();

    // Parse command line arguments
    let args: Vec<String> = std::env::args().collect();
    let help_requested = args.contains(&"--help".to_string()) || args.contains(&"-h".to_string());

    if help_requested {
        print_help();
        return Ok(());
    }

    log(LogTag::System, "TOOL", "📊 Starting Positions Update Tool");

    let dry_run = args.contains(&"--dry-run".to_string());
    let force = args.contains(&"--force".to_string());
    let verbose = args.contains(&"--verbose".to_string());

    // Parse minimum balance threshold
    let min_balance = if let Some(min_pos) = args.iter().position(|x| x == "--min-balance") {
        if min_pos + 1 < args.len() { args[min_pos + 1].parse::<u64>().unwrap_or(100) } else { 100 }
    } else {
        100
    };

    if dry_run {
        log(LogTag::System, "MODE", "🔍 DRY RUN MODE - No changes will be saved to positions.json");
    } else if force {
        log(
            LogTag::System,
            "MODE",
            "💾 FORCE MODE - Position updates will be saved to positions.json"
        );
    } else {
        log(
            LogTag::System,
            "MODE",
            "📋 ANALYSIS MODE - Use --force to save changes or --dry-run for analysis"
        );
    }

    if verbose {
        log(LogTag::System, "MODE", "📝 VERBOSE MODE - Detailed logging enabled");
    }

    log(LogTag::System, "CONFIG", &format!("Minimum balance threshold: {} tokens", min_balance));

    // Get wallet address from configs
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr.clone(),
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("Failed to get wallet address: {}", e));
            return Err(e.into());
        }
    };

    log(LogTag::System, "INFO", &format!("Wallet: {}", wallet_address));

    // Initialize RPC client
    init_rpc_client()?;

    // Initialize price service for current price lookups
    if let Err(e) = initialize_price_service().await {
        log(LogTag::System, "ERROR", &format!("Failed to initialize price service: {}", e));
        return Err(e.into());
    }

    // Load current positions
    let mut positions = load_positions_from_file();
    log(
        LogTag::System,
        "LOADED",
        &format!("Loaded {} positions from positions.json", positions.len())
    );

    // Filter for open positions (no exit_price)
    let open_positions: Vec<&Position> = positions
        .iter()
        .filter(|p| p.exit_price.is_none())
        .collect();

    log(
        LogTag::System,
        "ANALYSIS",
        &format!("Found {} open positions to check", open_positions.len())
    );

    if open_positions.is_empty() {
        log(LogTag::System, "INFO", "No open positions found. Nothing to update.");
        return Ok(());
    }

    // Analyze each open position
    let mut positions_to_close = Vec::new();
    let mut analysis_results = Vec::new();

    println!("\n📊 Open Positions Analysis");
    println!(
        "╭─────────────┬──────────────────────────────────────────────┬─────────────────┬──────────────────┬─────────────────╮"
    );
    println!(
        "│ 🏷️ Symbol   │ 🔑 Mint                                      │ 💾 Token Balance │ 💰 Current Price │ 📊 Status        │"
    );
    println!(
        "├─────────────┼──────────────────────────────────────────────┼─────────────────┼──────────────────┼─────────────────┤"
    );

    for position in &open_positions {
        // Get current token balance
        let balance_result = get_token_balance_for_position(
            &wallet_address.to_string(),
            &position.mint,
            verbose
        ).await;

        let token_balance = match balance_result {
            Ok(balance) => balance,
            Err(e) => {
                log(
                    LogTag::System,
                    "ERROR",
                    &format!("Failed to get balance for {}: {}", position.mint, e)
                );
                continue;
            }
        };

        // Get current market price
        let current_price = match get_token_price_safe(&position.mint).await {
            Some(price) => price,
            None => {
                log(
                    LogTag::System,
                    "WARNING",
                    &format!("Failed to get price for {}", position.mint)
                );
                0.0
            }
        };

        // Determine status
        let status = if token_balance < min_balance {
            if token_balance == 0 {
                "🔴 SOLD (0 bal)".red()
            } else {
                format!("🟡 LOW ({} bal)", token_balance).yellow()
            }
        } else {
            "🟢 HOLDING".green()
        };

        // Display analysis row
        println!(
            "│ {:11} │ {:44} │ {:15} │ {:16.10} │ {:15} │",
            position.symbol,
            position.mint,
            token_balance,
            current_price,
            status
        );

        // Record for potential closure
        if token_balance < min_balance {
            positions_to_close.push((position.mint.clone(), current_price));
        }

        analysis_results.push((position.mint.clone(), token_balance, current_price));
    }

    println!(
        "╰─────────────┴──────────────────────────────────────────────┴─────────────────┴──────────────────┴─────────────────╯"
    );

    // Summary
    println!("\n📋 Analysis Summary:");
    println!("   🔍 Total open positions checked: {}", open_positions.len());
    println!("   🔴 Positions to close (balance < {}): {}", min_balance, positions_to_close.len());
    println!("   🟢 Positions still holding: {}", open_positions.len() - positions_to_close.len());

    if positions_to_close.is_empty() {
        log(
            LogTag::System,
            "INFO",
            "✅ All open positions have sufficient token balances. No updates needed."
        );
        return Ok(());
    }

    // Show positions that will be closed
    println!("\n🔴 Positions to be marked as CLOSED:");
    for (mint, price) in &positions_to_close {
        if let Some(pos) = positions.iter().find(|p| &p.mint == mint) {
            let (_, pnl_percent) = if price > &0.0 {
                let entry_price = pos.effective_entry_price.unwrap_or(pos.entry_price);
                let pnl_percent = ((price - entry_price) / entry_price) * 100.0;
                (0.0, pnl_percent)
            } else {
                (0.0, 0.0)
            };

            println!("   • {} ({}) - Est. P&L: {:.2}%", pos.symbol, &mint[..8], pnl_percent);
        }
    }

    // Execute position updates if not dry run
    if force {
        // Create backup before making changes
        if let Err(e) = backup_positions_file() {
            log(LogTag::System, "WARNING", &format!("Failed to create backup: {}", e));
        }

        let mut updated_count = 0;

        for (mint, current_price) in positions_to_close {
            if let Some(position) = positions.iter_mut().find(|p| p.mint == mint) {
                close_position(position, current_price, verbose);
                updated_count += 1;

                let (pnl_sol, pnl_percent) = calculate_pnl(position);
                log(
                    LogTag::System,
                    "CLOSED",
                    &format!(
                        "Closed {} - P&L: {:.6} SOL ({:.2}%)",
                        position.symbol,
                        pnl_sol,
                        pnl_percent
                    )
                );
            }
        }

        // Save updated positions
        save_positions_to_file(&positions);
        log(
            LogTag::System,
            "SAVED",
            &format!("Updated {} positions and saved to positions.json", updated_count)
        );

        println!("\n✅ Position updates completed successfully!");
        println!("   💾 Updated positions: {}", updated_count);
        println!("   📁 Backup created: positions_backup_*.json");
        println!("   📊 Updated file: positions.json");
    } else if dry_run {
        println!("\n🔍 DRY RUN - No changes made to positions.json");
        println!("   💡 Use --force to execute these position updates");
    } else {
        println!("\n📋 Analysis complete. No changes made.");
        println!("   💡 Use --force to update positions.json");
        println!("   💡 Use --dry-run for analysis without prompts");
    }

    log(LogTag::System, "TOOL", "📊 Positions Update Tool completed");
    Ok(())
}
