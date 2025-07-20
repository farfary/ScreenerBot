use std::error::Error;
use screenerbot::global::read_configs;
use screenerbot::transactions::{
    TransactionCache,
    analyze_transaction,
    detect_swaps_in_transaction,
    get_dex_name,
    is_known_dex,
    get_all_dex_program_ids,
    format_timestamp,
};
use screenerbot::logger::{ log, LogTag };

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🔍 Analyzing Cached Transactions for DEX Activity");
    println!("================================================");

    // Test 1: Show all known DEX program IDs
    println!("\n📋 Known DEX Program IDs:");
    println!("{}", "=".repeat(80));

    let dex_program_ids = get_all_dex_program_ids();
    for (i, program_id) in dex_program_ids.iter().enumerate() {
        let dex_name = get_dex_name(program_id).unwrap_or("Unknown");
        println!("   {}. {} - {}", i + 1, dex_name, program_id);
    }

    // Test 2: Load cached transactions and analyze for swaps
    println!("\n🔍 Analyzing Cached Transactions:");
    println!("{}", "=".repeat(80));

    let cache = TransactionCache::load();
    let (total_cached, _) = cache.stats();
    println!("   📊 Total cached transactions: {}", total_cached);

    if total_cached == 0 {
        println!("   ⚠️  No cached transactions found. Run a transaction fetch first.");
        return Ok(());
    }

    let wallet_address = "B2DtMPbpQWvHYTP1izFTYvKBvbzVc2SWvFPCYRTWws59"; // From our config

    let mut total_swaps = 0;
    let mut dex_usage = std::collections::HashMap::new();
    let mut program_usage = std::collections::HashMap::new();

    // Analyze each cached transaction
    for (signature, transaction) in cache.transactions.iter() {
        let analysis = analyze_transaction(transaction, wallet_address, None);

        if analysis.contains_swaps {
            let swaps = detect_swaps_in_transaction(transaction, wallet_address);

            for swap in swaps {
                total_swaps += 1;

                // Track program usage
                *program_usage.entry(swap.program_id.clone()).or_insert(0) += 1;

                // Track DEX usage
                let dex_name = swap.dex_name
                    .clone()
                    .unwrap_or_else(|| {
                        format!(
                            "Unknown ({}...{})",
                            &swap.program_id[..8],
                            &swap.program_id[swap.program_id.len() - 8..]
                        )
                    });
                *dex_usage.entry(dex_name.clone()).or_insert(0) += 1;

                println!("   🔄 Swap found:");
                if let Some(block_time) = transaction.block_time {
                    println!("      📅 Time: {}", format_timestamp(Some(block_time)));
                }
                println!("      🏪 DEX: {}", dex_name);
                println!(
                    "      🔗 Signature: {}...{}",
                    &signature[..8],
                    &signature[signature.len() - 8..]
                );
                println!("      📊 Type: {:?}", swap.swap_type);
                println!("      🔧 Program: {}", swap.program_id);

                // Show token info
                println!(
                    "      🔵 Input: {} ({})",
                    swap.input_token.amount_ui,
                    &swap.input_token.mint[..8]
                );
                println!(
                    "      🟢 Output: {} ({})",
                    swap.output_token.amount_ui,
                    &swap.output_token.mint[..8]
                );
                println!();
            }
        }

        // Also check what programs are used in each transaction
        for instruction in &transaction.transaction.message.instructions {
            if let Some(program_idx) = instruction.program_id_index {
                if
                    let Some(program_id) = transaction.transaction.message.account_keys.get(
                        program_idx as usize
                    )
                {
                    if is_known_dex(program_id) {
                        println!(
                            "   🏪 Found known DEX program: {} ({})",
                            get_dex_name(program_id).unwrap_or("Unknown"),
                            program_id
                        );
                    }
                }
            }
        }
    }

    // Display summary
    println!("📈 Analysis Summary:");
    println!("{}", "=".repeat(60));
    println!("   📊 Total transactions analyzed: {}", total_cached);
    println!("   🔄 Total swaps found: {}", total_swaps);

    if !dex_usage.is_empty() {
        println!("   🏪 DEX usage breakdown:");
        for (dex, count) in &dex_usage {
            println!("      {} - {} swaps", dex, count);
        }
    }

    if !program_usage.is_empty() {
        println!("   🔧 Program usage (all):");
        for (program, count) in &program_usage {
            let dex_name = get_dex_name(program).unwrap_or("Unknown DEX");
            println!("      {} ({}) - {} times", dex_name, &program[..8], count);
        }
    }

    // Test 3: Check for specific DEX programs in instruction data
    println!("\n🔍 Searching for DEX Programs in All Instructions:");
    println!("{}", "=".repeat(80));

    let mut instruction_dex_count = std::collections::HashMap::new();

    for (signature, transaction) in cache.transactions.iter() {
        for instruction in &transaction.transaction.message.instructions {
            if let Some(program_idx) = instruction.program_id_index {
                if
                    let Some(program_id) = transaction.transaction.message.account_keys.get(
                        program_idx as usize
                    )
                {
                    if is_known_dex(program_id) {
                        let dex_name = get_dex_name(program_id).unwrap_or("Unknown");
                        *instruction_dex_count.entry(dex_name.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    if !instruction_dex_count.is_empty() {
        println!("   🏪 DEX programs found in instructions:");
        for (dex, count) in instruction_dex_count {
            println!("      {} - {} instructions", dex, count);
        }
    } else {
        println!("   ℹ️  No known DEX programs found in cached transactions");
        println!("   💡 This could mean:");
        println!("      - The wallet doesn't trade much");
        println!("      - Transactions are mostly transfers/other activities");
        println!("      - DEX programs used are not in our known list");
    }

    println!("\n✅ Cached transaction DEX analysis completed!");
    Ok(())
}
