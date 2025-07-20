// bin/test_find_specific_swaps.rs - Find and analyze specific token pair swaps
use screenerbot::transactions::*;
use screenerbot::global::read_configs;
use screenerbot::wallet::{ get_wallet_address, SOL_MINT };
use screenerbot::logger::{ log, LogTag };
use tokio::time::Instant;
use tabled::{ Tabled, Table, settings::{ Style, Modify, Alignment, object::Rows } };
use std::collections::HashMap;

/// Specific swap result display
#[derive(Debug, Clone, Tabled)]
pub struct SpecificSwapDisplay {
    #[tabled(rename = "📝 Signature")]
    signature: String,
    #[tabled(rename = "🏪 DEX")]
    dex_name: String,
    #[tabled(rename = "💰 Input Token")]
    input_token: String,
    #[tabled(rename = "🎯 Output Token")]
    output_token: String,
    #[tabled(rename = "📊 Input Amount")]
    input_amount: String,
    #[tabled(rename = "📈 Output Amount")]
    output_amount: String,
    #[tabled(rename = "💎 Type")]
    swap_type: String,
    #[tabled(rename = "⏰ Time")]
    block_time: String,
    #[tabled(rename = "✅ Success")]
    success: String,
}

/// DEX statistics display
#[derive(Debug, Clone, Tabled)]
pub struct DexStatsDisplay {
    #[tabled(rename = "🏪 DEX Name")]
    dex_name: String,
    #[tabled(rename = "🔄 Total Swaps")]
    total_swaps: String,
    #[tabled(rename = "✅ Successful")]
    successful_swaps: String,
    #[tabled(rename = "❌ Failed")]
    failed_swaps: String,
    #[tabled(rename = "📊 Success Rate")]
    success_rate: String,
    #[tabled(rename = "🪙 Unique Tokens")]
    unique_tokens: String,
}

/// Token pair analysis display
#[derive(Debug, Clone, Tabled)]
pub struct TokenPairDisplay {
    #[tabled(rename = "💰 Token A")]
    token_a: String,
    #[tabled(rename = "🎯 Token B")]
    token_b: String,
    #[tabled(rename = "🔄 Swap Count")]
    swap_count: String,
    #[tabled(rename = "🏪 Primary DEX")]
    primary_dex: String,
    #[tabled(rename = "📊 Total Volume")]
    total_volume: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    log(LogTag::System, "INFO", "🎯 Starting Specific Swap Detection Test");
    log(LogTag::System, "INFO", "========================================");

    // Initialize system
    let configs = read_configs("configs.json")?;
    let wallet_address = get_wallet_address().map_err(|e| {
        Box::new(std::io::Error::new(std::io::ErrorKind::Other, e)) as Box<dyn std::error::Error>
    })?;

    log(LogTag::System, "INFO", &format!("🎯 Analyzing wallet: {}", wallet_address));

    let db = TransactionDatabase::new()?;
    let fetcher = TransactionFetcher::new(configs.clone(), None)?;

    // Popular token mints for testing
    let bonk_mint = "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263"; // BONK
    let usdc_mint = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"; // USDC
    let pepe_mint = "BzKR5FyJ6Dctu8pFZQrrRZKqTNjWvNSbQN4v7YS5pump"; // PEPE (example)

    // Test 1: Load recent transactions
    log(LogTag::System, "INFO", "=== Test 1: Load Recent Transactions ===");
    let start_time = Instant::now();

    let signatures = fetcher.get_recent_signatures(&wallet_address, 100).await?;
    let transactions = fetcher.batch_fetch_transactions(&signatures, Some(50)).await?;

    log(
        LogTag::System,
        "SUCCESS",
        &format!(
            "📥 Loaded {} transactions in {:.2}s",
            transactions.len(),
            start_time.elapsed().as_secs_f64()
        )
    );

    // Test 2: Find SOL <-> BONK swaps
    log(LogTag::System, "INFO", "=== Test 2: SOL ↔ BONK Swaps ===");
    let sol_bonk_swaps = swap_detection::find_swaps_between_tokens(
        &transactions,
        SOL_MINT,
        bonk_mint
    );
    log(LogTag::System, "SUCCESS", &format!("🔍 Found {} SOL ↔ BONK swaps", sol_bonk_swaps.len()));

    if !sol_bonk_swaps.is_empty() {
        display_specific_swaps(&sol_bonk_swaps, "SOL ↔ BONK");
    }

    // Test 3: Find all BONK-related swaps
    log(LogTag::System, "INFO", "=== Test 3: All BONK-Related Swaps ===");
    let bonk_swaps = swap_detection::find_swaps_with_token(&transactions, bonk_mint);
    log(LogTag::System, "SUCCESS", &format!("🔍 Found {} BONK-related swaps", bonk_swaps.len()));

    if !bonk_swaps.is_empty() {
        display_specific_swaps(&bonk_swaps, "BONK-Related");
    }

    // Test 4: Find SOL <-> USDC swaps
    log(LogTag::System, "INFO", "=== Test 4: SOL ↔ USDC Swaps ===");
    let sol_usdc_swaps = swap_detection::find_swaps_between_tokens(
        &transactions,
        SOL_MINT,
        usdc_mint
    );
    log(LogTag::System, "SUCCESS", &format!("🔍 Found {} SOL ↔ USDC swaps", sol_usdc_swaps.len()));

    if !sol_usdc_swaps.is_empty() {
        display_specific_swaps(&sol_usdc_swaps, "SOL ↔ USDC");
    }

    // Test 5: DEX-specific analysis
    log(LogTag::System, "INFO", "=== Test 5: DEX-Specific Analysis ===");

    let dex_names = ["Jupiter", "Raydium V4", "Serum DEX V3", "Orca", "Phoenix"];
    let mut dex_stats = Vec::new();

    for dex_name in &dex_names {
        let stats = swap_detection::get_dex_swap_stats(&transactions, dex_name);
        if stats.total_swaps > 0 {
            dex_stats.push(DexStatsDisplay {
                dex_name: stats.dex_name.clone(),
                total_swaps: stats.total_swaps.to_string(),
                successful_swaps: stats.successful_swaps.to_string(),
                failed_swaps: stats.failed_swaps.to_string(),
                success_rate: format!("{:.1}%", if stats.total_swaps > 0 {
                    ((stats.successful_swaps as f64) / (stats.total_swaps as f64)) * 100.0
                } else {
                    0.0
                }),
                unique_tokens: stats.unique_tokens.len().to_string(),
            });
        }
    }

    if !dex_stats.is_empty() {
        let mut table = Table::new(dex_stats);
        table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        println!("\n🏪 DEX Statistics:");
        println!("{}", table);
    }

    // Test 6: Token pair analysis
    log(LogTag::System, "INFO", "=== Test 6: Token Pair Analysis ===");

    let token_pairs = [
        (SOL_MINT, bonk_mint, "SOL", "BONK"),
        (SOL_MINT, usdc_mint, "SOL", "USDC"),
        (bonk_mint, usdc_mint, "BONK", "USDC"),
    ];

    let mut pair_analysis = Vec::new();

    for (token_a, token_b, symbol_a, symbol_b) in &token_pairs {
        let swaps = swap_detection::find_swaps_between_tokens(&transactions, token_a, token_b);

        if !swaps.is_empty() {
            // Find primary DEX
            let mut dex_counts: HashMap<String, usize> = HashMap::new();
            let mut total_volume = 0.0;

            for swap in &swaps {
                if let Some(dex) = &swap.dex_name {
                    *dex_counts.entry(dex.clone()).or_insert(0) += 1;
                }
                // Add volume calculation if available
                total_volume += swap.input_token.amount_ui; // Simplified volume calculation
            }

            let primary_dex = dex_counts
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(dex, _)| dex.clone())
                .unwrap_or_else(|| "Unknown".to_string());

            pair_analysis.push(TokenPairDisplay {
                token_a: symbol_a.to_string(),
                token_b: symbol_b.to_string(),
                swap_count: swaps.len().to_string(),
                primary_dex,
                total_volume: format!("{:.6}", total_volume),
            });
        }
    }

    if !pair_analysis.is_empty() {
        let mut table = Table::new(pair_analysis);
        table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        println!("\n💱 Token Pair Analysis:");
        println!("{}", table);
    }

    // Test 7: Swap pattern analysis
    log(LogTag::System, "INFO", "=== Test 7: Swap Pattern Analysis ===");

    let analyzer = TransactionAnalyzer::new();
    let mut all_swaps = Vec::new();

    for (_, transaction) in &transactions {
        let detected_swaps = analyzer.detect_swaps_advanced(&transaction);
        all_swaps.extend(detected_swaps);
    }

    if !all_swaps.is_empty() {
        // Analyze swap patterns
        let mut swap_type_counts: HashMap<String, usize> = HashMap::new();
        let mut hourly_distribution: HashMap<u8, usize> = HashMap::new();

        for swap in &all_swaps {
            let swap_type_str = format!("{}", swap.swap_type);
            *swap_type_counts.entry(swap_type_str).or_insert(0) += 1;

            if let Some(block_time) = swap.block_time {
                let hour = ((block_time % 86400) / 3600) as u8; // Extract hour of day
                *hourly_distribution.entry(hour).or_insert(0) += 1;
            }
        }

        log(LogTag::System, "INFO", "📊 Swap Type Distribution:");
        for (swap_type, count) in &swap_type_counts {
            log(LogTag::System, "INFO", &format!("  {}: {} swaps", swap_type, count));
        }

        if !hourly_distribution.is_empty() {
            let most_active_hour = hourly_distribution
                .iter()
                .max_by_key(|(_, count)| *count)
                .map(|(hour, count)| (*hour, *count));

            if let Some((hour, count)) = most_active_hour {
                log(
                    LogTag::System,
                    "INFO",
                    &format!("  Most active hour: {}:00 ({} swaps)", hour, count)
                );
            }
        }

        log(
            LogTag::System,
            "SUCCESS",
            &format!("📈 Total unique swaps detected: {}", all_swaps.len())
        );
    }

    log(LogTag::System, "SUCCESS", "🎉 Specific swap detection test completed!");

    Ok(())
}

/// Display specific swap results in a formatted table
fn display_specific_swaps(swaps: &[SwapTransaction], title: &str) {
    let displays: Vec<SpecificSwapDisplay> = swaps
        .iter()
        .map(|swap| SpecificSwapDisplay {
            signature: format!("{}...", &swap.signature[..12]),
            dex_name: swap.dex_name.clone().unwrap_or_else(|| "Unknown".to_string()),
            input_token: format!("{}...", &swap.input_token.mint[..8]),
            output_token: format!("{}...", &swap.output_token.mint[..8]),
            input_amount: format!("{:.6}", swap.input_token.amount_ui),
            output_amount: format!("{:.6}", swap.output_token.amount_ui),
            swap_type: format!("{}", swap.swap_type),
            block_time: swap.block_time
                .map(|t| format_timestamp(Some(t)))
                .unwrap_or_else(|| "Unknown".to_string()),
            success: (if swap.is_success { "✅" } else { "❌" }).to_string(),
        })
        .collect();

    if !displays.is_empty() {
        let mut table = Table::new(displays);
        table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        println!("\n🔄 {} Swaps:", title);
        println!("{}", table);
    }
}
