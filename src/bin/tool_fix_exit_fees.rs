use screenerbot::logger::{log, LogTag};
use screenerbot::positions::Position;
use screenerbot::transactions_tools::analyze_post_swap_transaction;
use screenerbot::utils::{get_wallet_address, load_positions_from_file, save_positions_to_file};
use std::fs;
use tokio;

/// Tool to backfill missing exit fees for closed positions
/// This tool examines all closed positions and extracts actual exit transaction fees
/// from blockchain data to replace the incorrect 0 values currently stored

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔧 EXIT FEE BACKFILL TOOL");
    println!("========================");
    println!("This tool will fix missing exit fees for closed positions\n");

    // Load current positions
    let mut positions = load_positions_from_file();
    let wallet_address = get_wallet_address().unwrap_or_default();
    
    if wallet_address.is_empty() {
        println!("❌ ERROR: Could not get wallet address");
        return Err("Wallet address not available".into());
    }

    println!("📋 Loaded {} positions", positions.len());
    println!("🏦 Using wallet: {}\n", wallet_address);

    // Filter closed positions that need exit fee extraction
    let mut positions_to_fix = Vec::new();
    let mut total_missing_fees = 0u64;

    for (index, position) in positions.iter().enumerate() {
        if position.exit_time.is_some() && 
           position.exit_transaction_signature.is_some() &&
           (position.exit_fee_lamports.is_none() || position.exit_fee_lamports == Some(0)) {
            positions_to_fix.push(index);
            
            // Try to get the actual fee from transaction data to show potential recovery
            if let Some(exit_sig) = &position.exit_transaction_signature {
                if let Ok(content) = fs::read_to_string(format!("data/transactions/{}.json", exit_sig)) {
                    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(meta_fee) = json.get("transaction_data")
                            .and_then(|td| td.get("meta"))
                            .and_then(|meta| meta.get("fee"))
                            .and_then(|fee| fee.as_u64()) {
                            total_missing_fees += meta_fee;
                        }
                    }
                }
            }
        }
    }

    if positions_to_fix.is_empty() {
        println!("✅ No positions need exit fee backfill");
        return Ok(());
    }

    println!("🔍 Found {} positions needing exit fee backfill", positions_to_fix.len());
    println!("💰 Estimated missing fees: {} lamports ({:.9} SOL)\n", 
             total_missing_fees, 
             total_missing_fees as f64 / 1_000_000_000.0);

    // Ask for confirmation
    println!("Do you want to proceed with backfilling exit fees? (y/N)");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    if !input.trim().to_lowercase().starts_with('y') {
        println!("❌ Operation cancelled");
        return Ok(());
    }

    println!("\n🔧 Starting exit fee backfill process...\n");

    let mut fixed_count = 0;
    let mut failed_count = 0;
    let mut total_fees_recovered = 0u64;

    for &index in &positions_to_fix {
        let position = &mut positions[index];
        
        if let Some(exit_signature) = &position.exit_transaction_signature {
            println!("🔍 Processing {} ({})", position.symbol, &exit_signature[..12]);
            
            match analyze_post_swap_transaction(
                exit_signature,
                &wallet_address,
                &position.mint,
                "So11111111111111111111111111111111111111112", // SOL mint
                "sell"
            ).await {
                Ok(analysis) => {
                    if let Some(transaction_fee) = analysis.transaction_fee {
                        let old_fee = position.exit_fee_lamports.unwrap_or(0);
                        position.exit_fee_lamports = Some(transaction_fee);
                        
                        println!("   ✅ Fee extracted: {} lamports ({:.9} SOL)", 
                                transaction_fee, 
                                analysis.fees_paid);
                        
                        if transaction_fee > old_fee {
                            total_fees_recovered += transaction_fee - old_fee;
                        }
                        
                        fixed_count += 1;
                    } else {
                        println!("   ⚠️  No transaction fee found in analysis");
                        failed_count += 1;
                    }
                }
                Err(e) => {
                    println!("   ❌ Failed to analyze transaction: {}", e);
                    failed_count += 1;
                }
            }
        }
    }

    println!("\n📊 BACKFILL SUMMARY");
    println!("===================");
    println!("Positions processed: {}", positions_to_fix.len());
    println!("Successfully fixed: {}", fixed_count);
    println!("Failed: {}", failed_count);
    println!("Total fees recovered: {} lamports ({:.9} SOL)", 
             total_fees_recovered, 
             total_fees_recovered as f64 / 1_000_000_000.0);

    if fixed_count > 0 {
        // Save updated positions
        save_positions_to_file(&positions);
        println!("\n✅ Updated positions saved successfully");
        
        log(LogTag::System, "FEE_BACKFILL", &format!(
            "Exit fee backfill completed: {} positions fixed, {} lamports recovered",
            fixed_count, total_fees_recovered
        ));
    } else {
        println!("\n⚠️  No positions were updated");
    }

    Ok(())
}
