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
const TARGET_TOKEN: &str = "VFpeBr3p3VMTZf6vkh9R9h8w2PZkHmBr99y3tVpjFhV"; // VFpeBr3p3VMTZf6vkh9R9h8w2PZkHmBr99y3tVpjFhV

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
read_configs;
use screenerbot::transactions::{
    get_recent_signatures_with_fallback,
    get_transactions_with_cache_and_fallback,
    analyze_transaction,
    format_timestamp,
    SignatureInfo,
    TransactionAnalysis,
    SwapType,
};
use std::error::Error;
use tabled::{ Table, Tabled, settings::{ Style, Modify, Alignment, object::Rows } };

/// Specific token to check
const TARGET_TOKEN: &str = "E3kRwpjjt75R5KrXDHPkgZg4uskGR5BQnyc5wCRrbonk";

/// Number of transactions to analyze
const TRANSACTION_LIMIT: usize = 50;

/// Minimum significant token amount to avoid spam (0.001 tokens)
const MIN_SIGNIFICANT_AMOUNT: f64 = 0.001;

#[derive(Tabled)]
struct TransactionSummary {
    #[tabled(rename = "🔸 #")]
    index: String,
    #[tabled(rename = "📅 Time")]
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
    #[tabled(rename = "📊 Type")]
    change_type: String,
    #[tabled(rename = "📈 Amount")]
    amount: String,
    #[tabled(rename = "💰 Estimated Price")]
    estimated_price: String,
    #[tabled(rename = "🔗 Signature")]
    signature: String,
}

#[derive(Tabled)]
struct SwapSummary {
    #[tabled(rename = "🔸 #")]
    index: String,
    #[tabled(rename = "📅 Time")]
    time: String,
    #[tabled(rename = "🔄 Type")]
    swap_type: String,
    #[tabled(rename = "🔵 Input Token")]
    input_token: String,
    #[tabled(rename = "🟢 Output Token")]
    output_token: String,
    #[tabled(rename = "💱 Rate")]
    exchange_rate: String,
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
    println!("🔍 Advanced Transaction Analysis for Token: {}", TARGET_TOKEN);
    println!("{}", "=".repeat(100));

    // Load configuration
    let configs = match read_configs("configs.json") {
        Ok(cfg) => cfg,
        Err(e) => {
            println!("❌ Failed to read configs: {}", e);
            return Ok(());
        }
    };

    // Get wallet address
    let wallet_address = match get_wallet_address() {
        Ok(addr) => addr,
        Err(e) => {
            println!("❌ Failed to get wallet address: {}", e);
            return Ok(());
        }
    };

    println!("👛 Wallet Address: {}", wallet_address);

    let client = reqwest::Client::new();

    // Step 1: Get recent transaction signatures
    println!("🔄 Fetching last {} transaction signatures...", TRANSACTION_LIMIT);
    let signatures = match
        get_recent_signatures_with_fallback(
            &client,
            &wallet_address,
            &configs,
            TRANSACTION_LIMIT
        ).await
    {
        Ok(sigs) => {
            println!("   ✅ Found {} recent signatures", sigs.len());
            sigs
        }
        Err(e) => {
            println!("❌ Failed to get recent signatures: {}", e);
            return Ok(());
        }
    };

    if signatures.is_empty() {
        println!("   ❌ No transactions found!");
        return Ok(());
    }

    // Step 2: Fetch and analyze transaction details (with caching)
    println!("🔍 Fetching and analyzing transaction details with caching...");
    let transactions = get_transactions_with_cache_and_fallback(
        &client,
        &signatures,
        &configs,
        Some(TRANSACTION_LIMIT)
    ).await;

    if transactions.is_empty() {
        println!("   ❌ Failed to fetch any transaction details!");
        return Ok(());
    }

    println!("   ✅ Successfully analyzed {} transactions", transactions.len());

    // Analyze all transactions
    let mut analyses = Vec::new();
    let mut all_token_changes = Vec::new();
    let mut all_swaps = Vec::new();

    for (sig_info, transaction) in &transactions {
        let analysis = analyze_transaction(&transaction, &wallet_address, None); // Check all tokens

        // Collect significant token changes (filter out dust)
        for change in &analysis.token_changes {
            let amount_abs = change.change.abs();
            if amount_abs >= MIN_SIGNIFICANT_AMOUNT {
                all_token_changes.push((sig_info, change.clone()));
            }
        }

        // Collect all swaps
        for swap in &analysis.swaps {
            all_swaps.push((sig_info, swap.clone()));
        }

        analyses.push((sig_info, analysis));
    }

    // Display transaction summary table (only show transactions with significant activity)
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
                    mint: change.mint.clone(), // Full mint address
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

    // Display swap analysis
    if !all_swaps.is_empty() {
        println!("\n🔄 Detected Swaps ({} total):", all_swaps.len());
        println!("{}", "=".repeat(120));

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
                    input_token: swap.input_token.mint.clone(), // Full mint address
                    output_token: swap.output_token.mint.clone(), // Full mint address
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

    println!("\n✅ Advanced transaction analysis completed!");

    Ok(())
}
