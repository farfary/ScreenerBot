use std::error::Error;
use screenerbot::global::read_configs;
use screenerbot::wallet::get_wallet_address;
use screenerbot::transactions::{
    get_recent_signatures_with_fallback,
    get_transactions_with_cache_and_fallback,
    analyze_transaction,
    detect_swaps_in_transaction,
    get_dex_name,
    is_known_dex,
    get_all_dex_program_ids,
    format_timestamp,
    SignatureInfo,
};
use screenerbot::logger::{ log, LogTag };

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🏪 DEX Recognition System Test");
    println!("==============================");

    // Test 1: Show all known DEX program IDs
    println!("\n📋 Known DEX Program IDs:");
    println!("{}", "=".repeat(80));

    let dex_program_ids = get_all_dex_program_ids();
    for (i, program_id) in dex_program_ids.iter().enumerate() {
        let dex_name = get_dex_name(program_id).unwrap_or("Unknown");
        println!("   {}. {} - {}", i + 1, dex_name, program_id);
    }

    // Test 2: Test DEX recognition functions
    println!("\n🔍 Testing DEX Recognition Functions:");
    println!("{}", "=".repeat(80));

    // Test with known DEX
    let jupiter_program = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";
    println!("   Jupiter Program: {}", jupiter_program);
    println!("   Is Known DEX: {}", is_known_dex(jupiter_program));
    println!("   DEX Name: {}", get_dex_name(jupiter_program).unwrap_or("Unknown"));

    // Test with unknown program
    let unknown_program = "11111111111111111111111111111111";
    println!("\n   Unknown Program: {}", unknown_program);
    println!("   Is Known DEX: {}", is_known_dex(unknown_program));
    println!("   DEX Name: {}", get_dex_name(unknown_program).unwrap_or("Unknown"));

    // Test 3: Real transaction analysis with caching
    println!("\n🔍 Testing Real Transaction Analysis with DEX Recognition:");
    println!("{}", "=".repeat(80));

    let configs = read_configs("configs.json")?;
    let wallet_address = get_wallet_address()?;
    let client = reqwest::Client::new();

    println!("   📍 Wallet: {}", wallet_address);

    // Get recent signatures
    log(LogTag::System, "INFO", "Fetching recent signatures...");
    let signatures = get_recent_signatures_with_fallback(
        &client,
        &wallet_address,
        &configs,
        20 // Get 20 recent signatures
    ).await?;

    if signatures.is_empty() {
        println!("   ⚠️  No signatures found");
        return Ok(());
    }

    println!("   📊 Found {} signatures", signatures.len());

    // Get transactions with caching
    log(LogTag::System, "INFO", "Fetching transaction details with caching...");
    let transactions = get_transactions_with_cache_and_fallback(
        &client,
        &signatures[..std::cmp::min(10, signatures.len())], // Analyze first 10
        &configs,
        None
    ).await;

    println!("   📊 Analyzed {} transactions", transactions.len());

    let mut swap_count = 0;
    let mut dex_usage = std::collections::HashMap::new();

    // Analyze each transaction for swaps and DEX usage
    for (sig_info, transaction) in &transactions {
        let analysis = analyze_transaction(transaction, &wallet_address, None);

        if analysis.contains_swaps {
            let swaps = detect_swaps_in_transaction(transaction, &wallet_address);

            for swap in swaps {
                swap_count += 1;
                let dex_name = swap.dex_name.unwrap_or_else(|| {
                    format!(
                        "Unknown ({}...{})",
                        &swap.program_id[..8],
                        &swap.program_id[swap.program_id.len() - 8..]
                    )
                });

                *dex_usage.entry(dex_name.clone()).or_insert(0) += 1;

                println!("   🔄 Swap found:");
                println!("      📅 Time: {}", format_timestamp(sig_info.block_time));
                println!("      🏪 DEX: {}", dex_name);
                println!(
                    "      🔗 Signature: {}...{}",
                    &sig_info.signature[..8],
                    &sig_info.signature[sig_info.signature.len() - 8..]
                );
                println!("      📊 Type: {:?}", swap.swap_type);
                println!();
            }
        }
    }

    // Display summary
    println!("📈 DEX Usage Summary:");
    println!("{}", "=".repeat(60));
    println!("   🔄 Total swaps found: {}", swap_count);

    if !dex_usage.is_empty() {
        println!("   🏪 DEX breakdown:");
        for (dex, count) in dex_usage {
            println!("      {} - {} swaps", dex, count);
        }
    } else {
        println!("   ℹ️  No swaps found in recent transactions");
    }

    println!("\n✅ DEX recognition system test completed!");
    Ok(())
}
