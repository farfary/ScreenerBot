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
    wallet_transactions::{initialize_wallet_transaction_manager},
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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerificationUpdate {
    pub mint: String,
    pub symbol: String,
    pub signature: String,
    pub was_verified: bool,
    pub now_verified: bool,
    pub effective_entry_price: Option<f64>,
    pub token_amount: Option<u64>,
    pub sol_amount: Option<f64>,
    pub entry_fee: Option<f64>,
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
    initialize_wallet_transaction_manager().await?;
    
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
        let is_verified = position.transaction_entry_verified && 
                         position.effective_entry_price.is_some() &&
                         position.token_amount.is_some();
        
        if !is_verified {
            if let Some(signature) = &position.entry_transaction_signature {
                println!("\n{} Verifying: {} ({})", 
                    "🔍".bright_yellow(), 
                    position.symbol.bright_white(), 
                    position.mint.bright_blue()
                );
                
                match update_position_verification(&position.mint, signature, args).await {
                    Ok(update) => {
                        if update.now_verified {
                            updated_count += 1;
                            println!("✅ Verified and updated: {}", position.symbol.bright_green());
                        } else {
                            failed_count += 1;
                            println!("❌ Verification failed: {}", position.symbol.bright_red());
                        }
                        updates.push(update);
                    }
                    Err(e) => {
                        failed_count += 1;
                        println!("❌ Error verifying {}: {}", position.symbol.bright_red(), e);
                    }
                }
            } else {
                println!("⚠️  No signature for position: {}", position.symbol.bright_yellow());
            }
        } else {
            already_verified_count += 1;
            if args.verbose {
                println!("✅ Already verified: {}", position.symbol.bright_green());
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
    println!("\n{} {}", "🔍 UPDATING POSITION FOR MINT:".bright_cyan().bold(), mint.bright_blue());
    
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
        if let Some(signature) = &position.entry_transaction_signature {
            let update = update_position_verification(mint, signature, args).await?;
            display_update(&update);
            
            if update.now_verified {
                println!("✅ Position successfully verified and updated!");
            } else {
                println!("❌ Position verification failed");
            }
        } else {
            println!("❌ Position has no entry transaction signature");
        }
    } else {
        println!("❌ Position not found for mint: {}", mint);
    }
    
    Ok(())
}

async fn update_position_by_signature(signature: &str, args: &Args) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{} {}", "🔍 UPDATING POSITION FOR SIGNATURE:".bright_cyan().bold(), signature.bright_blue());
    
    // Find position with this signature
    let positions = {
        let open_positions = get_open_positions();
        let closed_positions = get_closed_positions();
        let mut all_positions = open_positions;
        all_positions.extend(closed_positions);
        all_positions
    };
    
    let position = positions.iter().find(|p| 
        p.entry_transaction_signature.as_ref().map(|s| s.as_str()) == Some(signature)
    );
    
    if let Some(position) = position {
        let update = update_position_verification(&position.mint, signature, args).await?;
        display_update(&update);
        
        if update.now_verified {
            println!("✅ Position successfully verified and updated!");
        } else {
            println!("❌ Position verification failed");
        }
    } else {
        println!("❌ Position not found for signature: {}", signature);
    }
    
    Ok(())
}

async fn update_position_verification(mint: &str, signature: &str, args: &Args) -> Result<VerificationUpdate, Box<dyn std::error::Error>> {
    let wallet_address = get_wallet_address().unwrap_or_default();
    
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
            let verified = pos.transaction_entry_verified && 
                          pos.effective_entry_price.is_some() &&
                          pos.token_amount.is_some();
            (verified, pos.symbol.clone())
        } else {
            return Err("Position not found".into());
        }
    };
    
    if args.verbose {
        println!("  📊 Analyzing transaction: {}", &signature[..16]);
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
                        position.transaction_entry_verified = true;
                        position.effective_entry_price = Some(analysis.effective_price);
                        position.token_amount = Some(analysis.token_amount as u64);
                        position.total_size_sol = analysis.sol_amount; // Update actual SOL spent
                        position.entry_fee_lamports = Some((analysis.fees_paid * 1_000_000_000.0) as u64); // Convert SOL to lamports
                        
                        if args.verbose {
                            println!("  💾 Updated position in memory");
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
                    effective_entry_price: Some(analysis.effective_price),
                    token_amount: Some(analysis.token_amount as u64),
                    sol_amount: Some(analysis.sol_amount),
                    entry_fee: Some(analysis.fees_paid),
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
                effective_entry_price: None,
                token_amount: None,
                sol_amount: None,
                entry_fee: None,
                error: Some(e),
            })
        }
    }
}

fn display_update(update: &VerificationUpdate) {
    println!("\n{} {}", "📋 UPDATE RESULT:".bright_blue(), update.symbol.bright_white());
    println!("  🏷️  Mint: {}", update.mint.bright_blue());
    println!("  📡 Signature: {}", &update.signature[..16].bright_yellow());
    println!("  📊 Was verified: {}", if update.was_verified { "✅".to_string() } else { "❌".to_string() });
    println!("  📊 Now verified: {}", if update.now_verified { "✅".to_string() } else { "❌".to_string() });
    
    if let Some(price) = update.effective_entry_price {
        println!("  💰 Effective price: {:.12} SOL", price);
    }
    
    if let Some(amount) = update.token_amount {
        println!("  🪙 Token amount: {}", amount);
    }
    
    if let Some(sol) = update.sol_amount {
        println!("  💎 SOL spent: {:.9}", sol);
    }
    
    if let Some(fee) = update.entry_fee {
        println!("  💸 Entry fee: {:.9} SOL", fee);
    }
    
    if let Some(error) = &update.error {
        println!("  ❌ Error: {}", error.bright_red());
    }
}
