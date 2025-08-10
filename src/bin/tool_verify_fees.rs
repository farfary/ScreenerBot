use screenerbot::logger::{log, LogTag};
use screenerbot::utils::load_positions_from_file;
use std::fs;

/// Simple tool to verify fee accuracy in positions.json vs actual transaction data
/// This tool provides a quick check for fee calculation accuracy

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 QUICK FEE VERIFICATION TOOL");
    println!("==============================\n");

    let positions = load_positions_from_file();
    let mut total_entry_fees = 0u64;
    let mut total_exit_fees = 0u64;
    let mut verified_positions = 0;
    let mut missing_exit_fees = 0;

    for position in &positions {
        if position.exit_time.is_some() {
            // Closed position
            verified_positions += 1;
            
            if let Some(entry_fee) = position.entry_fee_lamports {
                total_entry_fees += entry_fee;
            }
            
            if let Some(exit_fee) = position.exit_fee_lamports {
                total_exit_fees += exit_fee;
                println!("✅ {} - Entry: {} lamports, Exit: {} lamports", 
                        position.symbol, 
                        position.entry_fee_lamports.unwrap_or(0),
                        exit_fee);
            } else {
                missing_exit_fees += 1;
                println!("⚠️  {} - Entry: {} lamports, Exit: MISSING", 
                        position.symbol, 
                        position.entry_fee_lamports.unwrap_or(0));
            }
        }
    }

    println!("\n📊 QUICK SUMMARY");
    println!("================");
    println!("Total Positions: {}", positions.len());
    println!("Closed Positions: {}", verified_positions);
    println!("Missing Exit Fees: {}", missing_exit_fees);
    println!("Total Entry Fees: {} lamports ({:.9} SOL)", 
             total_entry_fees, 
             total_entry_fees as f64 / 1_000_000_000.0);
    println!("Total Exit Fees: {} lamports ({:.9} SOL)", 
             total_exit_fees, 
             total_exit_fees as f64 / 1_000_000_000.0);
    println!("Total All Fees: {} lamports ({:.9} SOL)", 
             total_entry_fees + total_exit_fees, 
             (total_entry_fees + total_exit_fees) as f64 / 1_000_000_000.0);

    if missing_exit_fees == 0 {
        println!("\n🎉 All fee calculations appear accurate!");
    } else {
        println!("\n⚠️  {} positions need exit fee backfill. Run: cargo run --bin tool_fix_exit_fees", missing_exit_fees);
    }

    Ok(())
}
