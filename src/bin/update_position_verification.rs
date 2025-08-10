/// Position Verification Update Tool
/// 
/// This tool manually updates position verification status by running transaction
/// verification on unverified positions and updating their data with verified results.
///
/// Usage:
///   cargo run --bin update_position_verification -- --mint <MINT>
///   cargo run --bin update_position_verification -- --all
///   cargo run --bin update_position_verification -- --signature <SIG>

use clap::Parser;
use screenerbot::{
    logger::{init_file_logging, log, LogTag},
    positions::{get_open_positions, get_closed_positions, SAVED_POSITIONS},
    transactions_manager::{initialize_transactions_manager},
    transactions_tools::{analyze_post_swap_transaction_simple},
    utils::{get_wallet_address, save_positions_to_file},
};
use serde::{Deserialize, Serialize};
use colored::Colorize;

#[derive(Parser)]
#[command(about = "Update position verification status")]
pub struct Args {
    /// Specific token mint to verify
    #[arg(short, long)]
    pub mint: Option<String>,
    
    /// Specific transaction signature to verify and update
    #[arg(short, long)]
    pub signature: Option<String>,
    
    /// Verify all unverified positions
    #[arg(short, long)]
    pub all: bool,
    
    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
    
    /// Verify exit transaction instead of entry transaction
    #[arg(short, long)]
    pub exit: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationUpdate {
    pub mint: String,
    pub symbol: String,
    pub signature: String,
    pub was_verified: bool,
    pub now_verified: bool,
    pub verification_type: String, // "entry" or "exit"
    pub effective_entry_price: Option<f64>,
    pub effective_exit_price: Option<f64>,
    pub token_amount: Option<u64>,
    pub sol_amount: Option<f64>,
    pub entry_fee: Option<f64>,
    pub exit_fee: Option<f64>,
    pub error: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize logging system
    init_file_logging();
    
    // Enable debug transactions for this session
    {
        use screenerbot::global::CMD_ARGS;
        if let Ok(mut cmd_args) = CMD_ARGS.lock() {
            cmd_args.push("--debug-transactions".to_string());
        }
    }
    
    log(LogTag::Transactions, "INFO", "🔄 STARTING POSITION VERIFICATION UPDATE");
    
    // Initialize global transaction manager
    initialize_transactions_manager().await?;
    
    if args.all {
        update_all_positions(&args).await?;
    } else if let Some(mint) = &args.mint {
        update_position_by_mint(mint, &args).await?;
    } else if let Some(signature) = &args.signature {
        update_position_by_signature(signature, &args).await?;
    } else {
        println!("❌ Please specify --mint, --signature, or --all");
        return Ok(());
    }
    
    Ok(())
}

async fn update_all_positions(args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{}", "🔄 UPDATING ALL UNVERIFIED POSITIONS".bright_cyan().bold());
    
    let open_positions = get_open_positions();
    let closed_positions = get_closed_positions();
    
    // Combine all positions
    let mut all_positions = open_positions;
    all_positions.extend(closed_positions);
    
    let mut updates = Vec::new();
    let mut updated_count = 0;
    let mut already_verified_count = 0;
    let mut failed_count = 0;
    
    for position in &all_positions {
        let mut needs_entry_verification = false;
        let mut needs_exit_verification = false;
        
        // Check entry verification status
        if !position.transaction_entry_verified || 
           position.effective_entry_price.is_none() ||
           position.token_amount.is_none() {
            needs_entry_verification = true;
        }
        
        // Check exit verification status for closed positions
        if position.exit_transaction_signature.is_some() {
            if !position.transaction_exit_verified || position.effective_exit_price.is_none() {
                needs_exit_verification = true;
            }
        }
        
        if needs_entry_verification && position.entry_transaction_signature.is_some() {
            println!("\n{} Verifying ENTRY: {} ({})", 
                "🔍".bright_yellow(), 
                position.symbol.bright_white(), 
                position.mint.bright_blue()
            );
            
            match update_position_verification(&position.mint, &position.entry_transaction_signature.as_ref().unwrap(), args, false).await {
                Ok(update) => {
                    if update.now_verified {
                        updated_count += 1;
                        println!("✅ Entry verified and updated: {}", position.symbol.bright_green());
                    } else {
                        failed_count += 1;
                        println!("❌ Entry verification failed: {}", position.symbol.bright_red());
                    }
                    updates.push(update);
                }
                Err(e) => {
                    failed_count += 1;
                    println!("❌ Error verifying entry for {}: {}", position.symbol.bright_red(), e);
                }
            }
        }
        
        if needs_exit_verification && position.exit_transaction_signature.is_some() {
            println!("\n{} Verifying EXIT: {} ({})", 
                "🔍".bright_yellow(), 
                position.symbol.bright_white(), 
                position.mint.bright_blue()
            );
            
            match update_position_verification(&position.mint, &position.exit_transaction_signature.as_ref().unwrap(), args, true).await {
                Ok(update) => {
                    if update.now_verified {
                        updated_count += 1;
                        println!("✅ Exit verified and updated: {}", position.symbol.bright_green());
                    } else {
                        failed_count += 1;
                        println!("❌ Exit verification failed: {}", position.symbol.bright_red());
                    }
                    updates.push(update);
                }
                Err(e) => {
                    failed_count += 1;
                    println!("❌ Error verifying exit for {}: {}", position.symbol.bright_red(), e);
                }
            }
        }
        
        if !needs_entry_verification && !needs_exit_verification {
            already_verified_count += 1;
            if args.verbose {
                println!("✅ Already fully verified: {}", position.symbol.bright_green());
            }
        }
    }
    
    println!("\n{}", "📈 UPDATE SUMMARY".bright_green().bold());
    println!("✅ Updated positions: {}", updated_count);
    println!("✅ Already verified: {}", already_verified_count);
    println!("❌ Failed to verify: {}", failed_count);
    println!("📊 Total positions: {}", all_positions.len());
    
    if args.verbose && !updates.is_empty() {
        println!("\n{}", "📋 DETAILED UPDATES".bright_blue().bold());
        for update in updates {
            display_update(&update);
        }
    }
    
    Ok(())
}

async fn update_position_by_mint(mint: &str, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let verification_type = if args.exit { "EXIT" } else { "ENTRY" };
    println!("\n{} {}", 
        format!("🔍 UPDATING {} VERIFICATION FOR MINT:", verification_type).bright_cyan().bold(), 
        mint.bright_blue()
    );
    
    // Find position with this mint
    let positions = {
        let open_positions = get_open_positions();
        let closed_positions = get_closed_positions();
        let mut all_positions = open_positions;
        all_positions.extend(closed_positions);
        all_positions
    };
    
    let position = positions.iter().find(|p| p.mint == mint);
    
    if let Some(position) = position {
        let signature = if args.exit {
            &position.exit_transaction_signature
        } else {
            &position.entry_transaction_signature
        };
        
        if let Some(signature) = signature {
            let update = update_position_verification(mint, signature, args, args.exit).await?;
            display_update(&update);
            
            if update.now_verified {
                println!("✅ Position {} verification successful!", verification_type.to_lowercase());
            } else {
                println!("❌ Position {} verification failed", verification_type.to_lowercase());
            }
        } else {
            println!("❌ Position has no {} transaction signature", verification_type.to_lowercase());
        }
    } else {
        println!("❌ Position not found for mint: {}", mint);
    }
    
    Ok(())
}

async fn update_position_by_signature(signature: &str, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    let verification_type = if args.exit { "EXIT" } else { "ENTRY" };
    println!("\n{} {}", 
        format!("🔍 UPDATING {} VERIFICATION FOR SIGNATURE:", verification_type).bright_cyan().bold(), 
        signature.bright_blue()
    );
    
    // Find position with this signature
    let positions = {
        let open_positions = get_open_positions();
        let closed_positions = get_closed_positions();
        let mut all_positions = open_positions;
        all_positions.extend(closed_positions);
        all_positions
    };
    
    let position = if args.exit {
        positions.iter().find(|p| 
            p.exit_transaction_signature.as_ref().map(|s| s.as_str()) == Some(signature)
        )
    } else {
        positions.iter().find(|p| 
            p.entry_transaction_signature.as_ref().map(|s| s.as_str()) == Some(signature)
        )
    };
    
    if let Some(position) = position {
        let update = update_position_verification(&position.mint, signature, args, args.exit).await?;
        display_update(&update);
        
        if update.now_verified {
            println!("✅ Position {} verification successful!", verification_type.to_lowercase());
        } else {
            println!("❌ Position {} verification failed", verification_type.to_lowercase());
        }
    } else {
        println!("❌ Position not found for {} signature: {}", verification_type.to_lowercase(), signature);
    }
    
    Ok(())
}

async fn update_position_verification(mint: &str, signature: &str, args: &Args, is_exit: bool) -> Result<VerificationUpdate, Box<dyn std::error::Error>> {
    let wallet_address = get_wallet_address().unwrap_or_default();
    let verification_type = if is_exit { "exit" } else { "entry" };
    
    // Get current position state
    let (was_verified, symbol) = {
        let positions = {
            let open_positions = get_open_positions();
            let closed_positions = get_closed_positions();
            let mut all_positions = open_positions;
            all_positions.extend(closed_positions);
            all_positions
        };
        
        let position = positions.iter().find(|p| p.mint == mint);
        if let Some(pos) = position {
            let verified = if is_exit {
                pos.transaction_exit_verified && pos.effective_exit_price.is_some()
            } else {
                pos.transaction_entry_verified && 
                pos.effective_entry_price.is_some() &&
                pos.token_amount.is_some()
            };
            (verified, pos.symbol.clone())
        } else {
            return Err("Position not found".into());
        }
    };
    
    if args.verbose {
        println!("  📊 Analyzing {} transaction: {}", verification_type, &signature[..16]);
    }
    
    // Try to verify the transaction
    match analyze_post_swap_transaction_simple(signature, &wallet_address).await {
        Ok(analysis) => {
            if args.verbose {
                println!("  ✅ Transaction analysis successful");
                println!("    • Effective price: {:.12} SOL", analysis.effective_price);
                println!("    • Token amount: {}", analysis.token_amount);
                println!("    • SOL amount: {:.9}", analysis.sol_amount);
                println!("    • Fees: {:.9} SOL", analysis.fees_paid);
            }
            
            // Update the position in memory
            let updated = {
                if let Ok(mut positions) = SAVED_POSITIONS.lock() {
                    if let Some(position) = positions.iter_mut().find(|p| p.mint == mint) {
                        if is_exit {
                            // Update exit verification fields
                            position.transaction_exit_verified = true;
                            position.effective_exit_price = Some(analysis.effective_price);
                            position.exit_fee_lamports = Some((analysis.fees_paid * 1_000_000_000.0) as u64);
                            // For exits, we might want to update sol_received as well
                            position.sol_received = Some(analysis.sol_amount);
                        } else {
                            // Update entry verification fields
                            position.transaction_entry_verified = true;
                            position.effective_entry_price = Some(analysis.effective_price);
                            position.token_amount = Some(analysis.token_amount as u64);
                            position.total_size_sol = analysis.sol_amount; // Update actual SOL spent
                            position.entry_fee_lamports = Some((analysis.fees_paid * 1_000_000_000.0) as u64);
                        }
                        
                        if args.verbose {
                            println!("  💾 Updated {} verification in memory", verification_type);
                        }
                        
                        // Save to file
                        save_positions_to_file(&positions);
                        
                        if args.verbose {
                            println!("  💾 Saved positions to file");
                        }
                        
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            };
            
            if updated {
                Ok(VerificationUpdate {
                    mint: mint.to_string(),
                    symbol,
                    signature: signature.to_string(),
                    was_verified,
                    now_verified: true,
                    verification_type: verification_type.to_string(),
                    effective_entry_price: if is_exit { None } else { Some(analysis.effective_price) },
                    effective_exit_price: if is_exit { Some(analysis.effective_price) } else { None },
                    token_amount: if is_exit { None } else { Some(analysis.token_amount as u64) },
                    sol_amount: Some(analysis.sol_amount),
                    entry_fee: if is_exit { None } else { Some(analysis.fees_paid) },
                    exit_fee: if is_exit { Some(analysis.fees_paid) } else { None },
                    error: None,
                })
            } else {
                Err("Failed to update position in memory".into())
            }
        }
        Err(e) => {
            Ok(VerificationUpdate {
                mint: mint.to_string(),
                symbol,
                signature: signature.to_string(),
                was_verified,
                now_verified: false,
                verification_type: verification_type.to_string(),
                effective_entry_price: None,
                effective_exit_price: None,
                token_amount: None,
                sol_amount: None,
                entry_fee: None,
                exit_fee: None,
                error: Some(e),
            })
        }
    }
}

fn display_update(update: &VerificationUpdate) {
    println!("\n{} {} ({})", "📋 UPDATE RESULT:".bright_blue(), update.symbol.bright_white(), update.verification_type.to_uppercase().bright_cyan());
    println!("  🏷️  Mint: {}", update.mint.bright_blue());
    println!("  📡 Signature: {}", &update.signature[..16].bright_yellow());
    println!("  📊 Was verified: {}", if update.was_verified { "✅".to_string() } else { "❌".to_string() });
    println!("  📊 Now verified: {}", if update.now_verified { "✅".to_string() } else { "❌".to_string() });
    
    if let Some(price) = update.effective_entry_price {
        println!("  💰 Effective entry price: {:.12} SOL", price);
    }
    
    if let Some(price) = update.effective_exit_price {
        println!("  💰 Effective exit price: {:.12} SOL", price);
    }
    
    if let Some(amount) = update.token_amount {
        println!("  🪙 Token amount: {}", amount);
    }
    
    if let Some(sol) = update.sol_amount {
        if update.verification_type == "exit" {
            println!("  💎 SOL received: {:.9}", sol);
        } else {
            println!("  💎 SOL spent: {:.9}", sol);
        }
    }
    
    if let Some(fee) = update.entry_fee {
        println!("  💸 Entry fee: {:.9} SOL", fee);
    }
    
    if let Some(fee) = update.exit_fee {
        println!("  💸 Exit fee: {:.9} SOL", fee);
    }
    
    if let Some(error) = &update.error {
        println!("  ❌ Error: {}", error.bright_red());
    }
}
