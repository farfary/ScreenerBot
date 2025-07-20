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

    // Get recent transaction signatures
    let signatures = match fetcher.get_recent_signatures(&wallet_address, 20).await {
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

    // Analyze each transaction with both methods
    for (i, sig_info) in signatures.iter().enumerate().take(5) {
        println!(
            "{} Transaction {}/{}: {}",
            "🔸".bright_yellow(),
            i + 1,
            signatures.len().min(5),
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

        // Method 2: Enhanced analysis with token fetching
        let enhanced_analysis = analyzer.analyze_transaction_with_token_fetch(&transaction).await;

        // Compare results
        println!("   📋 Standard Analysis:");
        println!("      Is Swap: {}", if standard_analysis.is_swap {
            "✅ YES".bright_green()
        } else {
            "❌ NO".bright_red()
        });

        if !standard_analysis.token_transfers.is_empty() {
            println!("      Token Transfers: {}", standard_analysis.token_transfers.len());
            for transfer in &standard_analysis.token_transfers {
                let direction = if transfer.is_incoming { "→" } else { "←" };
                println!(
                    "         {} {} ({}...)",
                    direction,
                    transfer.ui_amount.unwrap_or(0.0),
                    &transfer.mint[..8]
                );
            }
        }

        println!("   🚀 Enhanced Analysis:");
        println!("      Is Swap: {}", if enhanced_analysis.is_swap {
            "✅ YES".bright_green()
        } else {
            "❌ NO".bright_red()
        });

        if !enhanced_analysis.token_transfers.is_empty() {
            println!("      Token Transfers: {}", enhanced_analysis.token_transfers.len());
            for transfer in &enhanced_analysis.token_transfers {
                let direction = if transfer.is_incoming { "→" } else { "←" };
                println!(
                    "         {} {} ({}...)",
                    direction,
                    transfer.ui_amount.unwrap_or(0.0),
                    &transfer.mint[..8]
                );
            }
        }

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

    println!("{}", "=".repeat(60));
    println!("{}", "🏁 Enhanced Swap Detection Test Complete!".bright_green().bold());
    println!("This test demonstrates the enhanced analyzer's ability to:");
    println!("• 🔍 Detect unknown tokens in transactions");
    println!("• 📥 Fetch token information from DexScreener");
    println!("• 💾 Cache tokens to the database");
    println!("• 🔄 Re-evaluate swap detection with new token data");
    println!("• ✨ Identify swaps that were previously missed");

    Ok(())
}
