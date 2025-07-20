use screenerbot::wallet::get_wallet_address;
use screenerbot::transactions::{
    get_transactions_with_cache_and_fallback,
    analyze_transactions,
    detect_swaps_in_transaction,
    detect_token_balance_changes,
    format_timestamp,
    TransactionAnalysis,
    SwapTransaction,
    SwapType,
    TokenChange,
    SignatureInfo,
};
use screenerbot::global::read_configs;
use screenerbot::logger::{ log_info, log_warning, log_error, LogTag };
use tabled::{ Tabled, Table, Style, Modify, Alignment, object::Rows };
use std::error::Error;

const TRANSACTION_LIMIT: u32 = 50;
const TARGET_TOKEN: &str = "VFpeBr3p3VMTZf6vkh9R9h8w2PZkHmBr99y3tVpjFhV";

#[derive(Tabled)]
struct TransactionSummary {
    #[tabled(rename = "#")]
    index: String,
    #[tabled(rename = "⏰ Time")]
    time: String,
    #[tabled(rename = "✅ Status")]
    status: String,
    #[tabled(rename = "🔄 Swaps")]
    swaps: String,
    #[tabled(rename = "💸 Fee (SOL)")]
    fee: String,
    #[tabled(rename = "🔗 Signature")]
    signature: String,
}

#[derive(Tabled)]
struct TokenChangeSummary {
    #[tabled(rename = "🪙 Token Mint")]
    mint: String,
    #[tabled(rename = "📊 Change Type")]
    change_type: String,
    #[tabled(rename = "💰 Amount")]
    amount: String,
    #[tabled(rename = "💵 Est. Price")]
    estimated_price: String,
    #[tabled(rename = "🔗 Signature")]
    signature: String,
}

#[derive(Tabled)]
struct SwapSummary {
    #[tabled(rename = "#")]
    index: String,
    #[tabled(rename = "⏰ Time")]
    time: String,
    #[tabled(rename = "🔄 Type")]
    swap_type: String,
    #[tabled(rename = "🔵 Input Token")]
    input_token: String,
    #[tabled(rename = "🟢 Output Token")]
    output_token: String,
    #[tabled(rename = "💱 Rate")]
    exchange_rate: String,
    #[tabled(rename = "🏪 DEX")]
    dex: String,
    #[tabled(rename = "🔗 Signature")]
    signature: String,
}

/// Estimate token price based on SOL changes in the same transaction
fn estimate_token_price(
    analyses: &[(&SignatureInfo, TransactionAnalysis)],
    token_mint: &str,
    token_amount: f64
) -> String {
    // Find the transaction that contains this token change
    for (_, analysis) in analyses {
        let has_token_change = analysis.token_changes
            .iter()
            .any(|change| change.mint == token_mint);
        if !has_token_change {
            continue;
        }

        // Look for SOL balance changes in the same transaction
        let sol_mint = "11111111111111111111111111111112"; // Native SOL program
        if
            let Some(sol_change) = analysis.token_changes
                .iter()
                .find(|change| change.mint == sol_mint)
        {
            if sol_change.change != 0.0 && token_amount != 0.0 {
                let price_per_token = sol_change.change.abs() / token_amount.abs();
                return format!("{:.8} SOL", price_per_token);
            }
        }
    }
    "Unknown".to_string()
}

fn format_swap_type(swap_type: &SwapType) -> String {
    match swap_type {
        SwapType::Buy => "🟢 BUY".to_string(),
        SwapType::Sell => "🔴 SELL".to_string(),
        SwapType::SwapAtoB => "🔄 SWAP".to_string(),
        SwapType::SwapBtoA => "🔄 SWAP".to_string(),
        SwapType::Unknown => "❓ UNKNOWN".to_string(),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("🔍 Enhanced Transaction Analysis with DEX Recognition");
    println!("🎯 Target Token: {}", TARGET_TOKEN);
    println!("{}", "=".repeat(100));

    // Load configuration
    let configs = match read_configs("configs.json") {
        Ok(cfg) => cfg,
        Err(e) => {
            log_error(&LogTag::System, &format!("Failed to load configuration: {}", e));
            return Err(Box::new(e));
        }
    };

    // Get wallet address
    let wallet_address = get_wallet_address(&configs).await?;
    println!("📍 Wallet Address: {}", wallet_address);

    // Get transactions using cached function
    log_info(
        &LogTag::System,
        &format!("Fetching last {} transactions with caching...", TRANSACTION_LIMIT)
    );
    let transactions = get_transactions_with_cache_and_fallback(
        &wallet_address,
        TRANSACTION_LIMIT,
        &configs
    ).await?;

    log_info(&LogTag::System, &format!("Retrieved {} transactions", transactions.len()));

    // Analyze transactions
    let mut analyses = Vec::new();
    let mut all_token_changes = Vec::new();
    let mut all_swaps = Vec::new();

    for transaction in &transactions {
        let sig_info = SignatureInfo {
            signature: transaction.transaction.signatures
                .first()
                .unwrap_or(&"unknown".to_string())
                .clone(),
            block_time: transaction.block_time,
            slot: transaction.slot,
        };

        // Analyze transaction comprehensively
        let analysis = analyze_transactions(&[transaction.clone()], &wallet_address);

        if analysis.is_empty() {
            continue;
        }

        let analysis = analysis.into_iter().next().unwrap();

        // Collect token changes for the target token
        let target_changes: Vec<_> = analysis.token_changes
            .iter()
            .filter(|change| change.mint == TARGET_TOKEN && change.change.abs() > 0.001)
            .cloned()
            .collect();

        for change in target_changes {
            all_token_changes.push((sig_info.clone(), change));
        }

        // Collect swaps
        for swap in &analysis.swaps {
            all_swaps.push((sig_info.clone(), swap.clone()));
        }

        analyses.push((sig_info, analysis));
    }

    // Display transaction summary table
    let significant_transactions: Vec<_> = analyses
        .iter()
        .filter(|(_, analysis)| {
            !analysis.token_changes.is_empty() || !analysis.swaps.is_empty()
        })
        .collect();

    if !significant_transactions.is_empty() {
        println!("\n📊 Transaction Summary ({} with activity):", significant_transactions.len());
        println!("{}", "=".repeat(100));

        let summary_data: Vec<TransactionSummary> = significant_transactions
            .iter()
            .enumerate()
            .map(|(i, (sig_info, analysis))| {
                TransactionSummary {
                    index: format!("{}", i + 1),
                    time: format_timestamp(analysis.block_time),
                    status: (
                        if analysis.is_success {
                            "✅ Success"
                        } else {
                            "❌ Failed"
                        }
                    ).to_string(),
                    swaps: if analysis.contains_swaps {
                        format!("🔄 {} swaps", analysis.swaps.len())
                    } else {
                        "➖ None".to_string()
                    },
                    fee: format!("{:.6}", analysis.fee_sol),
                    signature: format!(
                        "{}...{}",
                        &analysis.signature[..8],
                        &analysis.signature[analysis.signature.len() - 8..]
                    ),
                }
            })
            .collect();

        let mut table = Table::new(summary_data);
        table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        println!("{}", table);
    } else {
        println!("\n📊 No significant transaction activity found in last {} transactions", TRANSACTION_LIMIT);
    }

    // Display significant token changes with price estimates
    if !all_token_changes.is_empty() {
        println!("\n💰 Token Balance Changes ({} significant changes):", all_token_changes.len());
        println!("{}", "=".repeat(120));

        let token_changes_data: Vec<TokenChangeSummary> = all_token_changes
            .iter()
            .map(|(sig_info, change)| {
                let change_type = if change.change > 0.0 {
                    "📈 BUY/RECEIVE"
                } else {
                    "📉 SELL/SEND"
                };
                let amount = format!("{:.6}", change.change.abs());

                // Estimate price based on SOL changes (if available)
                let estimated_price = estimate_token_price(&analyses, &change.mint, change.change);

                TokenChangeSummary {
                    mint: change.mint.clone(),
                    change_type: change_type.to_string(),
                    amount,
                    estimated_price,
                    signature: format!(
                        "{}...{}",
                        &sig_info.signature[..8],
                        &sig_info.signature[sig_info.signature.len() - 8..]
                    ),
                }
            })
            .collect();

        let mut table = Table::new(token_changes_data);
        table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        println!("{}", table);
    }

    // Display swap analysis with DEX information
    if !all_swaps.is_empty() {
        println!("\n🔄 Detected Swaps with DEX Recognition ({} total):", all_swaps.len());
        println!("{}", "=".repeat(140));

        let swap_data: Vec<SwapSummary> = all_swaps
            .iter()
            .enumerate()
            .map(|(i, (sig_info, swap))| {
                let exchange_rate = if swap.output_token.amount_ui > 0.0 {
                    format!("{:.6}", swap.input_token.amount_ui / swap.output_token.amount_ui)
                } else {
                    "N/A".to_string()
                };

                SwapSummary {
                    index: format!("{}", i + 1),
                    time: format_timestamp(sig_info.block_time),
                    swap_type: format_swap_type(&swap.swap_type),
                    input_token: format!(
                        "{}...{}",
                        &swap.input_token.mint[..8],
                        &swap.input_token.mint[swap.input_token.mint.len() - 8..]
                    ),
                    output_token: format!(
                        "{}...{}",
                        &swap.output_token.mint[..8],
                        &swap.output_token.mint[swap.output_token.mint.len() - 8..]
                    ),
                    exchange_rate,
                    dex: swap.dex_name
                        .clone()
                        .unwrap_or_else(|| {
                            format!(
                                "{}...{}",
                                &swap.program_id[..8],
                                &swap.program_id[swap.program_id.len() - 8..]
                            )
                        }),
                    signature: format!(
                        "{}...{}",
                        &sig_info.signature[..8],
                        &sig_info.signature[sig_info.signature.len() - 8..]
                    ),
                }
            })
            .collect();

        let mut table = Table::new(swap_data);
        table.with(Style::rounded()).with(Modify::new(Rows::new(1..)).with(Alignment::center()));
        println!("{}", table);
    }

    // Display summary statistics
    println!("\n📈 Summary Statistics:");
    println!("{}", "=".repeat(60));
    println!("   📊 Total transactions analyzed: {}", transactions.len());
    println!(
        "   ✅ Successful transactions: {}",
        analyses
            .iter()
            .filter(|(_, a)| a.is_success)
            .count()
    );
    println!(
        "   ❌ Failed transactions: {}",
        analyses
            .iter()
            .filter(|(_, a)| !a.is_success)
            .count()
    );
    println!("   🔄 Transactions with swaps: {}", all_swaps.len());
    println!("   💰 Significant token changes: {}", all_token_changes.len());
    println!(
        "   💸 Total fees paid: {:.6} SOL",
        analyses
            .iter()
            .map(|(_, a)| a.fee_sol)
            .sum::<f64>()
    );

    // Display DEX statistics if we have swaps
    if !all_swaps.is_empty() {
        println!("\n🏪 DEX Usage Statistics:");
        println!("{}", "=".repeat(60));
        let mut dex_counts = std::collections::HashMap::new();
        for (_, swap) in &all_swaps {
            let dex_name = swap.dex_name
                .clone()
                .unwrap_or_else(|| {
                    format!(
                        "Unknown ({}...{})",
                        &swap.program_id[..8],
                        &swap.program_id[swap.program_id.len() - 8..]
                    )
                });
            *dex_counts.entry(dex_name).or_insert(0) += 1;
        }

        for (dex, count) in dex_counts {
            println!("   🏪 {}: {} swaps", dex, count);
        }
    }

    println!("\n✅ Enhanced transaction analysis with DEX recognition completed!");

    Ok(())
}
