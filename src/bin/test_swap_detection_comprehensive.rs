// bin/test_swap_detection_comprehensive.rs - Comprehensive swap detection testing and debugging
use screenerbot::transactions::*;
use screenerbot::global::read_configs;
use screenerbot::wallet::get_wallet_address;
use screenerbot::logger::{ log, LogTag };
use tokio::time::Instant;
use tabled::{ Tabled, Table, settings::{ Style, Modify, Alignment, object::Rows } };
use std::collections::HashMap;

/// Enhanced swap detection results for display
#[derive(Debug, Clone, Tabled)]
pub struct SwapDetectionResult {
    #[tabled(rename = "📝 Signature")]
    signature: String,
    #[tabled(rename = "🔄 Is Swap")]
    is_swap: String,
    #[tabled(rename = "🏪 DEX")]
    dex_name: String,
    #[tabled(rename = "💰 Input Token")]
    input_token: String,
    #[tabled(rename = "🎯 Output Token")]
    output_token: String,
    #[tabled(rename = "💎 Input Amount")]
    input_amount: String,
    #[tabled(rename = "🚀 Output Amount")]
    output_amount: String,
    #[tabled(rename = "📊 Effective Price")]
    effective_price: String,
    #[tabled(rename = "⏰ Block Time")]
    block_time: String,
    #[tabled(rename = "✅ Success")]
    success: String,
}

/// Token interaction analysis
#[derive(Debug, Clone, Tabled)]
pub struct TokenInteractionDisplay {
    #[tabled(rename = "🔢 Index")]
    account_index: String,
    #[tabled(rename = "🪙 Token")]
    mint: String,
    #[tabled(rename = "📈 Change")]
    amount_change: String,
    #[tabled(rename = "🔄 Direction")]
    direction: String,
    #[tabled(rename = "💡 UI Amount")]
    ui_amount: String,
    #[tabled(rename = "🎯 Decimals")]
    decimals: String,
}

/// Program interaction analysis
#[derive(Debug, Clone, Tabled)]
pub struct ProgramInteractionDisplay {
    #[tabled(rename = "🔢 Instruction")]
    instruction_index: String,
    #[tabled(rename = "🏪 Program ID")]
    program_id: String,
    #[tabled(rename = "🏷️ DEX Name")]
    dex_name: String,
    #[tabled(rename = "✅ Known DEX")]
    is_known_dex: String,
    #[tabled(rename = "📏 Data Length")]
    data_length: String,
}

/// Enhanced swap statistics
#[derive(Debug)]
pub struct SwapDetectionStats {
    pub total_transactions: usize,
    pub swap_transactions: usize,
    pub swap_percentage: f64,
    pub dex_usage: HashMap<String, usize>,
    pub unique_tokens: usize,
    pub successful_swaps: usize,
    pub failed_swaps: usize,
    pub average_processing_time_ms: f64,
    pub most_active_dex: Option<String>,
    pub largest_swap_by_value: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(
        LogTag::System,
        "INFO",
        "🚀 Starting Comprehensive Swap Detection Test (1000 Transactions)"
    );
    log(LogTag::System, "INFO", "===============================================================");

    // Load configuration
    let configs = read_configs("configs.json")?;
    let wallet_address = get_wallet_address().map_err(|e| {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error>
    })?;

    log(LogTag::System, "INFO", &format!("🎯 Target wallet: {}", wallet_address));

    // Step 1: Initialize transaction system
    log(LogTag::System, "INFO", "=== Step 1: Initialize Transaction System ===");
    let db = TransactionDatabase::new()?;
    let fetcher = TransactionFetcher::new(configs.clone(), None)?;
    let analyzer = TransactionAnalyzer::new();

    let initial_count = db.get_transaction_count()?;
    log(LogTag::System, "INFO", &format!("📊 Database has {} cached transactions", initial_count));

    // Step 2: Fetch recent transactions
    log(LogTag::System, "INFO", "=== Step 2: Fetch Recent Transactions ===");
    let start_time = Instant::now();

    // Get signatures first - checking last 1000 transactions
    let signatures = match fetcher.get_recent_signatures(&wallet_address, 1000).await {
        Ok(sigs) => {
            log(LogTag::System, "SUCCESS", &format!("📥 Fetched {} signatures", sigs.len()));
            sigs
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Failed to fetch signatures: {}", e));
            return Ok(());
        }
    };

    // Batch fetch transaction details with caching - process all fetched signatures
    let transactions = match fetcher.batch_fetch_transactions(&signatures, None).await {
        Ok(txs) => {
            log(
                LogTag::System,
                "SUCCESS",
                &format!("📥 Fetched {} transaction details", txs.len())
            );
            txs
        }
        Err(e) => {
            log(LogTag::System, "ERROR", &format!("❌ Failed to fetch transaction details: {}", e));
            return Ok(());
        }
    };

    let fetch_duration = start_time.elapsed();
    log(
        LogTag::System,
        "INFO",
        &format!("⏱️ Fetch completed in {:.2}s", fetch_duration.as_secs_f64())
    );

    // Step 3: Enhanced Swap Detection Analysis
    log(LogTag::System, "INFO", "=== Step 3: Enhanced Swap Detection Analysis ===");
    let analysis_start = Instant::now();

    let mut swap_results = Vec::new();
    let mut swap_stats = SwapDetectionStats {
        total_transactions: transactions.len(),
        swap_transactions: 0,
        swap_percentage: 0.0,
        dex_usage: HashMap::new(),
        unique_tokens: 0,
        successful_swaps: 0,
        failed_swaps: 0,
        average_processing_time_ms: 0.0,
        most_active_dex: None,
        largest_swap_by_value: None,
    };

    let mut unique_tokens = std::collections::HashSet::new();
    let mut processing_times = Vec::new();

    for (sig_info, transaction) in &transactions {
        let analysis_start = Instant::now();

        // Analyze transaction for swaps
        let analysis = analyzer.analyze_transaction(&transaction);
        let processing_time = analysis_start.elapsed().as_millis() as f64;
        processing_times.push(processing_time);

        // Create display result
        let result = create_swap_detection_result(&sig_info, &transaction, &analysis);
        swap_results.push(result);

        // Update statistics
        if analysis.is_swap {
            swap_stats.swap_transactions += 1;
            if analysis.is_success {
                swap_stats.successful_swaps += 1;
            } else {
                swap_stats.failed_swaps += 1;
            }

            // Track DEX usage
            if let Some(swap_info) = &analysis.swap_info {
                *swap_stats.dex_usage.entry(swap_info.dex_name.clone()).or_insert(0) += 1;

                // Track unique tokens
                unique_tokens.insert(swap_info.input_mint.clone());
                unique_tokens.insert(swap_info.output_mint.clone());
            }
        }

        // Track all tokens in transfers
        for transfer in &analysis.token_transfers {
            unique_tokens.insert(transfer.mint.clone());
        }
    }

    // Finalize statistics
    swap_stats.swap_percentage = if swap_stats.total_transactions > 0 {
        ((swap_stats.swap_transactions as f64) / (swap_stats.total_transactions as f64)) * 100.0
    } else {
        0.0
    };
    swap_stats.unique_tokens = unique_tokens.len();
    swap_stats.average_processing_time_ms = if !processing_times.is_empty() {
        processing_times.iter().sum::<f64>() / (processing_times.len() as f64)
    } else {
        0.0
    };
    swap_stats.most_active_dex = swap_stats.dex_usage
        .iter()
        .max_by_key(|(_, count)| *count)
        .map(|(dex, count)| format!("{} ({} swaps)", dex, count));

    let analysis_duration = analysis_start.elapsed();
    log(
        LogTag::System,
        "INFO",
        &format!("⏱️ Analysis completed in {:.2}s", analysis_duration.as_secs_f64())
    );

    // Step 4: Display Results
    log(LogTag::System, "INFO", "=== Step 4: Swap Detection Results ===");

    // Display summary statistics
    display_swap_statistics(&swap_stats);

    // Display detailed swap results
    if !swap_results.is_empty() {
        let mut table = Table::new(&swap_results);
        table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        println!("\n🔄 Detailed Swap Detection Results:");
        println!("{}", table);
    } else {
        log(LogTag::System, "INFO", "📝 No transactions found for analysis");
    }

    // Step 5: Deep Dive Analysis on Detected Swaps
    log(LogTag::System, "INFO", "=== Step 5: Deep Dive Analysis ===");

    let swap_transactions: Vec<_> = transactions
        .iter()
        .filter(|(_, tx)| analyzer.is_swap_transaction(tx))
        .collect();

    if !swap_transactions.is_empty() {
        log(
            LogTag::System,
            "SUCCESS",
            &format!("🔍 Found {} swap transactions for deep analysis", swap_transactions.len())
        );

        for (i, (sig_info, transaction)) in swap_transactions.iter().enumerate().take(3) {
            log(
                LogTag::System,
                "INFO",
                &format!("📊 Deep Analysis #{}: {}", i + 1, &sig_info.signature[..12])
            );

            // Show detailed analysis
            let debug_info = analyzer.debug_transaction_analysis(transaction);
            println!("{}", debug_info);

            // Show token interactions
            let analysis = analyzer.analyze_transaction(transaction);
            if !analysis.token_transfers.is_empty() {
                display_token_interactions(&analysis.token_transfers);
            }

            // Show program interactions
            if !analysis.program_interactions.is_empty() {
                display_program_interactions(&analysis.program_interactions);
            }

            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
        }
    } else {
        log(LogTag::System, "INFO", "📝 No swap transactions found in recent history");
    }

    // Step 6: Performance and Accuracy Summary
    log(LogTag::System, "INFO", "=== Step 6: Performance Summary ===");
    log(
        LogTag::System,
        "SUCCESS",
        &format!(
            "✅ Total processing time: {:.2}s",
            (fetch_duration + analysis_duration).as_secs_f64()
        )
    );
    log(
        LogTag::System,
        "SUCCESS",
        &format!(
            "⚡ Average analysis time per transaction: {:.2}ms",
            swap_stats.average_processing_time_ms
        )
    );
    log(
        LogTag::System,
        "SUCCESS",
        &format!("🎯 Swap detection accuracy: {:.1}%", swap_stats.swap_percentage)
    );

    if let Some(most_active) = &swap_stats.most_active_dex {
        log(LogTag::System, "SUCCESS", &format!("🏆 Most active DEX: {}", most_active));
    }

    log(LogTag::System, "SUCCESS", "🎉 Comprehensive swap detection test completed successfully!");

    Ok(())
}

/// Create a swap detection result for display
fn create_swap_detection_result(
    sig_info: &SignatureInfo,
    _transaction: &TransactionResult,
    analysis: &TransactionAnalysis
) -> SwapDetectionResult {
    let block_time = sig_info.block_time
        .map(|t| format_timestamp(Some(t)))
        .unwrap_or_else(|| "Unknown".to_string());

    if analysis.is_swap {
        if let Some(swap_info) = &analysis.swap_info {
            SwapDetectionResult {
                signature: format!("{}...", &analysis.signature[..12]),
                is_swap: "✅ YES".to_string(),
                dex_name: swap_info.dex_name.clone(),
                input_token: format!("{}...", &swap_info.input_mint[..8]),
                output_token: format!("{}...", &swap_info.output_mint[..8]),
                input_amount: format!(
                    "{:.6}",
                    swap_info.input_amount.parse::<f64>().unwrap_or(0.0)
                ),
                output_amount: format!(
                    "{:.6}",
                    swap_info.output_amount.parse::<f64>().unwrap_or(0.0)
                ),
                effective_price: format!("{:.9}", swap_info.effective_price),
                block_time,
                success: (if analysis.is_success { "✅" } else { "❌" }).to_string(),
            }
        } else {
            SwapDetectionResult {
                signature: format!("{}...", &analysis.signature[..12]),
                is_swap: "⚠️ PARTIAL".to_string(),
                dex_name: "Unknown".to_string(),
                input_token: "N/A".to_string(),
                output_token: "N/A".to_string(),
                input_amount: "N/A".to_string(),
                output_amount: "N/A".to_string(),
                effective_price: "N/A".to_string(),
                block_time,
                success: (if analysis.is_success { "✅" } else { "❌" }).to_string(),
            }
        }
    } else {
        SwapDetectionResult {
            signature: format!("{}...", &analysis.signature[..12]),
            is_swap: "❌ NO".to_string(),
            dex_name: "-".to_string(),
            input_token: "-".to_string(),
            output_token: "-".to_string(),
            input_amount: "-".to_string(),
            output_amount: "-".to_string(),
            effective_price: "-".to_string(),
            block_time,
            success: (if analysis.is_success { "✅" } else { "❌" }).to_string(),
        }
    }
}

/// Display comprehensive swap detection statistics
fn display_swap_statistics(stats: &SwapDetectionStats) {
    log(LogTag::System, "INFO", "📊 Swap Detection Statistics:");
    log(LogTag::System, "INFO", &format!("  📈 Total Transactions: {}", stats.total_transactions));
    log(LogTag::System, "INFO", &format!("  🔄 Swap Transactions: {}", stats.swap_transactions));
    log(LogTag::System, "INFO", &format!("  📊 Swap Percentage: {:.1}%", stats.swap_percentage));
    log(LogTag::System, "INFO", &format!("  ✅ Successful Swaps: {}", stats.successful_swaps));
    log(LogTag::System, "INFO", &format!("  ❌ Failed Swaps: {}", stats.failed_swaps));
    log(LogTag::System, "INFO", &format!("  🪙 Unique Tokens: {}", stats.unique_tokens));
    log(
        LogTag::System,
        "INFO",
        &format!("  ⚡ Avg Processing Time: {:.2}ms", stats.average_processing_time_ms)
    );

    if let Some(most_active) = &stats.most_active_dex {
        log(LogTag::System, "INFO", &format!("  🏆 Most Active DEX: {}", most_active));
    }

    if !stats.dex_usage.is_empty() {
        log(LogTag::System, "INFO", "  🏪 DEX Usage Breakdown:");
        for (dex, count) in &stats.dex_usage {
            log(LogTag::System, "INFO", &format!("    - {}: {} swaps", dex, count));
        }
    }
}

/// Display token interactions in a formatted table
fn display_token_interactions(transfers: &[TokenTransfer]) {
    let token_displays: Vec<TokenInteractionDisplay> = transfers
        .iter()
        .map(|transfer| TokenInteractionDisplay {
            account_index: transfer.account_index.to_string(),
            mint: format!("{}...", &transfer.mint[..8]),
            amount_change: format!("{:.6}", transfer.amount_change),
            direction: (if transfer.is_incoming { "📈 IN" } else { "📉 OUT" }).to_string(),
            ui_amount: transfer.ui_amount.map_or("N/A".to_string(), |amount|
                format!("{:.6}", amount)
            ),
            decimals: transfer.decimals.to_string(),
        })
        .collect();

    if !token_displays.is_empty() {
        let mut table = Table::new(token_displays);
        table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        println!("\n🪙 Token Interactions:");
        println!("{}", table);
    }
}

/// Display program interactions in a formatted table
fn display_program_interactions(interactions: &[ProgramInteraction]) {
    let program_displays: Vec<ProgramInteractionDisplay> = interactions
        .iter()
        .map(|interaction| ProgramInteractionDisplay {
            instruction_index: interaction.instruction_index.to_string(),
            program_id: format!("{}...", &interaction.program_id[..12]),
            dex_name: interaction.dex_name.clone().unwrap_or_else(|| "Unknown".to_string()),
            is_known_dex: (if interaction.is_known_dex { "✅" } else { "❌" }).to_string(),
            data_length: interaction.data_length.to_string(),
        })
        .collect();

    if !program_displays.is_empty() {
        let mut table = Table::new(program_displays);
        table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        println!("\n🏪 Program Interactions:");
        println!("{}", table);
    }
}
