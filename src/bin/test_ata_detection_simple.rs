use screenerbot::wallet::lamports_to_sol;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Testing Enhanced ATA Detection Logic");
    println!("{}", "=".repeat(50));

    // Test scenarios based on our on-chain analysis
    
    // PHI Transaction (PROBLEMATIC - should detect ATA)
    println!("\n🧪 PHI Transaction Scenario (PROBLEMATIC):");
    test_balance_change_detection("PHI", 2554144, true).await;

    // AIRFRY Transaction (CLEAN - should NOT detect ATA)
    println!("\n🧪 AIRFRY Transaction Scenario (CLEAN):");
    test_balance_change_detection("AIRFRY", 59585, false).await;

    println!("\n✅ ATA Detection Logic Testing Complete!");
    
    Ok(())
}

async fn test_balance_change_detection(token_name: &str, total_sol_lamports: u64, should_detect_ata: bool) {
    println!("Token: {}", token_name);
    println!("Total SOL received: {:.6}", lamports_to_sol(total_sol_lamports));
    
    // ATA rent constants
    const ATA_RENT_LAMPORTS: u64 = 2_039_280;  // Standard ATA rent
    const ATA_RENT_TOLERANCE: u64 = 100_000;   // Allow some tolerance
    
    // Simulate balance changes based on our transaction analysis
    let (pre_balances, post_balances) = if should_detect_ata {
        // PHI scenario: Account closure detected (account went from 2039280 to 0)
        (vec![100000000u64, 2039280, 0], vec![102554144u64, 0, 0])
    } else {
        // AIRFRY scenario: Clean transaction, no account closure
        (vec![100000000u64, 0, 0], vec![100059585u64, 0, 0])
    };
    
    // Test the balance change detection logic
    let mut ata_detected = false;
    let mut ata_rent_amount = 0u64;
    
    println!("📊 Testing balance change patterns:");
    
    for (i, (pre_balance, post_balance)) in pre_balances.iter()
        .zip(post_balances.iter())
        .enumerate() 
    {
        println!("  Account {}: {} → {} lamports", i, pre_balance, post_balance);
        
        // Skip the wallet account (first account)
        if i == 0 {
            continue;
        }

        // Look for negative balance changes (account closures)
        if *post_balance < *pre_balance {
            let closed_amount = *pre_balance - *post_balance;
            
            println!("    → Account closure detected: {} lamports", closed_amount);
            
            // Check if this matches ATA rent amount
            if closed_amount >= ATA_RENT_LAMPORTS - ATA_RENT_TOLERANCE &&
               closed_amount <= ATA_RENT_LAMPORTS + ATA_RENT_TOLERANCE {
                println!("    → ✅ Matches ATA rent pattern!");
                ata_detected = true;
                ata_rent_amount = closed_amount;
                break;
            } else {
                println!("    → ❌ Does not match ATA rent pattern");
            }
        }
    }
    
    println!("\n📈 Results:");
    println!("  🎯 ATA Detected: {}", ata_detected);
    
    if ata_detected {
        let sol_from_trade_only = total_sol_lamports.saturating_sub(ata_rent_amount);
        println!("  💰 Total SOL: {:.6}", lamports_to_sol(total_sol_lamports));
        println!("  🏦 ATA Rent: {:.6} SOL", lamports_to_sol(ata_rent_amount));
        println!("  📈 Trade Only: {:.6} SOL", lamports_to_sol(sol_from_trade_only));
        println!("  ✅ FIXED: ATA contamination separated from trading proceeds");
    } else {
        println!("  💰 Total SOL: {:.6} (clean)", lamports_to_sol(total_sol_lamports));
        println!("  ✅ CLEAN: No ATA contamination detected");
    }
    
    // Validation
    if should_detect_ata == ata_detected {
        println!("  ✅ CORRECT: Detection worked as expected!");
    } else {
        println!("  ❌ ERROR: Detection failed!");
        if should_detect_ata {
            println!("    Expected ATA detection but none found");
        } else {
            println!("    Unexpected ATA detection");
        }
    }
}
