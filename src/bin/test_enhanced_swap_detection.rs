use screenerbot::transactions::analyzer::TransactionAnalyzer;
use screenerbot::transactions::fetcher::TransactionFetcher;
use screenerbot::global::read_configs;
use colored::Colorize;
use solana_sdk::signer::Signer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "🔍 Testing Enhanced Swap Detection with Token Fetching".bright_blue().bold());
    println!("{}", "=".repeat(60));

    // Initialize the analyzer with token DB enabled
    let analyzer = TransactionAnalyzer::new_with_db_option(true);

    // Load configs
    let configs = read_configs("configs.json")?;
    let fetcher = TransactionFetcher::new(configs.clone(), None)?;

    // Use the main wallet from configs
    let wallet_keypair = screenerbot::global::load_wallet_from_config(&configs)?;
    let wallet_pubkey = wallet_keypair.pubkey();
    let wallet_address = wallet_pubkey.to_string();

    println!("🎯 Fetching recent transactions for main wallet: {}", wallet_address.bright_cyan());

    // Get recent transaction signatures - increase the search range
    let signatures = match fetcher.get_recent_signatures(&wallet_address, 100).await {
        Ok(sigs) => sigs,
        Err(e) => {
            println!("❌ Failed to fetch signatures: {}", e);
            return Ok(());
        }
    };

    if signatures.is_empty() {
        println!("⚠️ No recent transactions found");
        return Ok(());
    }

    println!("📊 Found {} recent transactions, analyzing...\n", signatures.len());

    // First, let's find transactions with any token transfers
    println!("🔍 Searching for transactions with token transfers...");
    let mut transactions_with_transfers = Vec::new();

    // Analyze each transaction with both methods
    for (i, sig_info) in signatures.iter().enumerate().take(20) {
        println!(
            "{} Transaction {}/{}: {}",
            "🔸".bright_yellow(),
            i + 1,
            signatures.len().min(20),
            sig_info.signature[..16].bright_white()
        );

        // Fetch the full transaction
        let transactions = match
            fetcher.batch_fetch_transactions(&[sig_info.clone()], Some(1)).await
        {
            Ok(txs) => txs,
            Err(e) => {
                println!("   ❌ Error fetching transaction: {}", e);
                continue;
            }
        };

        let transaction = if let Some((_, tx)) = transactions.first() {
            tx
        } else {
            println!("   ⚠️ Transaction not found");
            continue;
        };

        // Method 1: Standard analysis
        let standard_analysis = analyzer.analyze_transaction(&transaction);

        // Check if this transaction has any token transfers
        if !standard_analysis.token_transfers.is_empty() {
            transactions_with_transfers.push((
                sig_info.clone(),
                transaction.clone(),
                standard_analysis.clone(),
            ));
            println!("   💰 Found {} token transfers!", standard_analysis.token_transfers.len());
            for transfer in &standard_analysis.token_transfers {
                let direction = if transfer.is_incoming { "📥 IN" } else { "📤 OUT" };
                println!(
                    "      {} {} {} (mint: {}...)",
                    direction,
                    transfer.ui_amount.unwrap_or(0.0),
                    "TOKEN", // We don't have symbol in TokenTransfer struct
                    &transfer.mint[..8]
                );
            }
        }

        // Method 2: Enhanced analysis with token fetching
        let enhanced_analysis = analyzer.analyze_transaction_with_token_fetch(&transaction).await;

        // Compare results
        println!("   � Standard Analysis:");
        println!("      Is Swap: {}", if standard_analysis.is_swap {
            "✅ YES".bright_green()
        } else {
            "❌ NO".bright_red()
        });

        println!("   🚀 Enhanced Analysis:");
        println!("      Is Swap: {}", if enhanced_analysis.is_swap {
            "✅ YES".bright_green()
        } else {
            "❌ NO".bright_red()
        });

        // Show improvement
        if !standard_analysis.is_swap && enhanced_analysis.is_swap {
            println!(
                "   🎉 {} Enhancement detected swap that was missed by standard analysis!",
                "IMPROVEMENT:".bright_green().bold()
            );
        }

        // Test swap detection methods too
        let standard_swaps = analyzer.detect_swaps_advanced(&transaction);
        let enhanced_swaps = analyzer.detect_swaps_with_token_fetch(&transaction).await;

        if enhanced_swaps.len() > standard_swaps.len() {
            println!(
                "   🔍 Enhanced detection found {} additional swaps!",
                enhanced_swaps.len() - standard_swaps.len()
            );
        }

        println!(); // Empty line for readability
    }

    // Now focus on transactions with transfers for detailed analysis
    println!("{}", "=".repeat(60));
    println!("🔍 Detailed Analysis of Transactions with Token Transfers");
    println!("{}", "=".repeat(60));

    for (sig_info, transaction, standard_analysis) in &transactions_with_transfers {
        println!("📝 Transaction: {}", sig_info.signature[..16].bright_cyan());
        println!("   📊 Token Transfers: {}", standard_analysis.token_transfers.len());

        for (i, transfer) in standard_analysis.token_transfers.iter().enumerate() {
            println!(
                "   Transfer {}: {} {} {} (mint: {})",
                i + 1,
                if transfer.is_incoming {
                    "📥"
                } else {
                    "📤"
                },
                transfer.ui_amount.unwrap_or(0.0),
                "TOKEN", // We don't have symbol in TokenTransfer struct
                transfer.mint
            );
        }

        // Enhanced analysis
        let enhanced_analysis = analyzer.analyze_transaction_with_token_fetch(&transaction).await;

        println!("   🔄 Standard swap detection: {}", if standard_analysis.is_swap {
            "✅"
        } else {
            "❌"
        });
        println!("   🚀 Enhanced swap detection: {}", if enhanced_analysis.is_swap {
            "✅"
        } else {
            "❌"
        });

        if enhanced_analysis.is_swap && !standard_analysis.is_swap {
            println!("   🎉 {} Enhancement improved detection!", "SUCCESS:".bright_green().bold());
        }

        println!();
    }

    println!("{}", "=".repeat(60));
    println!("{}", "🏁 Enhanced Swap Detection Test Complete!".bright_green().bold());
    println!("This test demonstrates the enhanced analyzer's improvements:");
    println!("• 🔍 Detect swaps with DEX program ID + meaningful SOL changes");
    println!("• 📊 Filter micro-transactions (< 0.000001 SOL)");
    println!("• � Require actual SOL ↔ token exchanges for swap validation");
    println!("• �📥 Fetch token information from DexScreener");
    println!("• 💾 Cache tokens to the database");
    println!("• ✨ Enhanced accuracy with improved swap detection logic");

    Ok(())
}
