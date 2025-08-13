/// Fix effective_exit_price values in positions.json
/// 
/// This utility corrects the decimal conversion bug where effective_exit_price
/// was calculated using raw token amounts instead of UI amounts, resulting in
/// extremely small values that displayed as zero.

use screenerbot::positions::Position;
use screenerbot::tokens::get_token_decimals_sync;
use screenerbot::logger::{log, LogTag};
use serde_json;
use std::fs;
use std::path::Path;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "fix_exit_prices")]
#[command(about = "Fix effective_exit_price values in positions.json file")]
struct Args {
    /// Path to positions.json file (default: data/positions.json)
    #[arg(short, long, default_value = "data/positions.json")]
    positions_file: String,
    
    /// Create backup before modifying (default: true)
    #[arg(long, default_value = "true")]
    backup: bool,
    
    /// Dry run - show what would be fixed without making changes
    #[arg(long)]
    dry_run: bool,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    
    // Initialize logging
    screenerbot::logger::init_file_logging();
    
    println!("🔧 Position Exit Price Fixer");
    println!("═══════════════════════════════");
    
    // Check if positions file exists
    if !Path::new(&args.positions_file).exists() {
        eprintln!("❌ Error: Positions file not found: {}", args.positions_file);
        return Ok(());
    }
    
    // Read positions file
    println!("📖 Reading positions from: {}", args.positions_file);
    let content = fs::read_to_string(&args.positions_file)?;
    let mut positions: Vec<Position> = serde_json::from_str(&content)?;
    
    println!("📊 Found {} total positions", positions.len());
    
    // Create backup if requested
    if args.backup && !args.dry_run {
        let backup_file = format!("{}.backup_{}", args.positions_file, chrono::Utc::now().format("%Y%m%d_%H%M%S"));
        fs::copy(&args.positions_file, &backup_file)?;
        println!("💾 Created backup: {}", backup_file);
    }
    
    // Analyze positions that need fixing
    let mut needs_fixing = Vec::new();
    let mut already_correct = Vec::new();
    let mut missing_data = Vec::new();
    
    for (index, position) in positions.iter().enumerate() {
        // Only check closed positions with exit data
        if position.exit_price.is_some() && position.sol_received.is_some() && position.token_amount.is_some() {
            let exit_price = position.exit_price.unwrap();
            let effective_exit_price = position.effective_exit_price.unwrap_or(0.0);
            let sol_received = position.sol_received.unwrap();
            let token_amount_raw = position.token_amount.unwrap();
            
            // Check if effective_exit_price looks wrong (too small compared to exit_price)
            // We consider it wrong if it's more than 1000x smaller than exit_price
            let ratio = if effective_exit_price > 0.0 {
                exit_price / effective_exit_price
            } else {
                f64::INFINITY
            };
            
            if ratio > 1000.0 || effective_exit_price == 0.0 {
                needs_fixing.push((index, position.clone()));
            } else {
                already_correct.push((index, position.symbol.clone()));
            }
        } else {
            missing_data.push((index, position.symbol.clone()));
        }
    }
    
    println!("\n📋 Analysis Results:");
    println!("   🔴 Need fixing: {} positions", needs_fixing.len());
    println!("   ✅ Already correct: {} positions", already_correct.len());
    println!("   ⚠️  Missing data: {} positions", missing_data.len());
    
    if needs_fixing.is_empty() {
        println!("\n🎉 All positions already have correct exit prices!");
        return Ok(());
    }
    
    println!("\n🔧 Positions that need fixing:");
    println!("┌──────────────┬────────────────┬──────────────────┬──────────────────┬─────────────────┐");
    println!("│ Symbol       │ Current        │ Raw Token Amount │ SOL Received     │ Expected Fix    │");
    println!("├──────────────┼────────────────┼──────────────────┼──────────────────┼─────────────────┤");
    
    let mut fixed_count = 0;
    let mut decimals_missing = 0;
    
    for (index, position) in &needs_fixing {
        let current_effective = position.effective_exit_price.unwrap_or(0.0);
        let sol_received = position.sol_received.unwrap();
        let token_amount_raw = position.token_amount.unwrap() as f64;
        
        // Get token decimals
        let decimals_opt = get_token_decimals_sync(&position.mint);
        
        match decimals_opt {
            Some(decimals) => {
                // Calculate corrected effective exit price
                let token_amount_ui = token_amount_raw / (10_f64).powi(decimals as i32);
                let corrected_price = sol_received / token_amount_ui;
                
                println!("│ {:12} │ {:14.2e} │ {:16.0} │ {:16.9} │ {:15.2e} │",
                    position.symbol,
                    current_effective,
                    token_amount_raw,
                    sol_received,
                    corrected_price
                );
                
                // Apply fix if not dry run
                if !args.dry_run {
                    positions[*index].effective_exit_price = Some(corrected_price);
                    
                    if args.verbose {
                        log(
                            LogTag::System,
                            "FIX_APPLIED",
                            &format!(
                                "Fixed {}: {:.2e} → {:.2e} (decimals: {})",
                                position.symbol,
                                current_effective,
                                corrected_price,
                                decimals
                            )
                        );
                    }
                }
                
                fixed_count += 1;
            }
            None => {
                println!("│ {:12} │ {:14.2e} │ {:16.0} │ {:16.9} │ ❌ No decimals  │",
                    position.symbol,
                    current_effective,
                    token_amount_raw,
                    sol_received
                );
                decimals_missing += 1;
            }
        }
    }
    
    println!("└──────────────┴────────────────┴──────────────────┴──────────────────┴─────────────────┘");
    
    if args.dry_run {
        println!("\n🔍 DRY RUN - No changes made");
        println!("   Would fix: {} positions", fixed_count);
        if decimals_missing > 0 {
            println!("   Cannot fix (missing decimals): {} positions", decimals_missing);
        }
        println!("\nRun without --dry-run to apply fixes");
    } else {
        // Save updated positions
        if fixed_count > 0 {
            let updated_json = serde_json::to_string_pretty(&positions)?;
            fs::write(&args.positions_file, updated_json)?;
            
            println!("\n✅ Successfully fixed {} positions", fixed_count);
            println!("💾 Updated file: {}", args.positions_file);
            
            if decimals_missing > 0 {
                println!("⚠️  {} positions could not be fixed (missing decimals)", decimals_missing);
            }
        } else {
            println!("\n⚠️  No positions could be fixed (all missing decimals)");
        }
    }
    
    // Show some examples of the fixes
    if args.verbose && fixed_count > 0 && !args.dry_run {
        println!("\n📝 Fix Summary:");
        let sample_count = std::cmp::min(3, fixed_count);
        println!("   Showing first {} fixed positions:", sample_count);
        
        for (i, (index, original_position)) in needs_fixing.iter().take(sample_count).enumerate() {
            let fixed_position = &positions[*index];
            let original_effective = original_position.effective_exit_price.unwrap_or(0.0);
            let fixed_effective = fixed_position.effective_exit_price.unwrap_or(0.0);
            
            println!("   {}. {}: {:.2e} → {:.2e} ({:.1}x improvement)",
                i + 1,
                fixed_position.symbol,
                original_effective,
                fixed_effective,
                if original_effective > 0.0 { fixed_effective / original_effective } else { f64::INFINITY }
            );
        }
    }
    
    println!("\n🎯 Fix complete! Exit prices should now display correctly.");
    
    Ok(())
}
