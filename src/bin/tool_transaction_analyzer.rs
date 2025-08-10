use screenerbot::{
    positions::get_closed_positions,
};
use serde_json::Value;
use std::fs;
use std::collections::HashMap;
use std::path::Path;

/// Comprehensive transaction analyzer for closed positions
/// Extracts all SOL and token values from transaction JSON files
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 TRANSACTION ANALYSIS TOOL FOR CLOSED POSITIONS");
    println!("═══════════════════════════════════════════════════════════════");
    
    // Get all closed positions
    let closed_positions = get_closed_positions();
    
    if closed_positions.is_empty() {
        println!("❌ No closed positions found");
        return Ok(());
    }
    
    println!("📊 Found {} closed positions to analyze", closed_positions.len());
    println!("");
    
    let mut position_summaries = Vec::new();
    let mut total_entry_fees_lamports = 0u64;
    let mut total_exit_fees_lamports = 0u64;
    let mut total_sol_invested = 0.0f64;
    let mut total_sol_received = 0.0f64;
    
    for (index, position) in closed_positions.iter().enumerate() {
        println!("🔍 Position {}/{}: {} ({})", 
                 index + 1, 
                 closed_positions.len(), 
                 position.symbol, 
                 position.mint);
        
        let mut position_summary = PositionSummary {
            symbol: position.symbol.clone(),
            mint: position.mint.clone(),
            entry_signature: position.entry_transaction_signature.clone(),
            exit_signature: position.exit_transaction_signature.clone(),
            entry_analysis: None,
            exit_analysis: None,
            stored_entry_fee: position.entry_fee_lamports,
            stored_exit_fee: position.exit_fee_lamports,
            stored_sol_received: position.sol_received,
            stored_token_amount: position.token_amount,
            stored_entry_size: position.entry_size_sol,
        };
        
        // Analyze entry transaction
        if let Some(ref entry_sig) = position.entry_transaction_signature {
            match analyze_transaction_from_file(entry_sig, &position.mint).await {
                Ok(analysis) => {
                    println!("  📈 Entry Transaction Analysis:");
                    print_transaction_analysis(&analysis, "    ");
                    position_summary.entry_analysis = Some(analysis);
                }
                Err(e) => {
                    println!("  ❌ Entry transaction analysis failed: {}", e);
                }
            }
        }
        
        // Analyze exit transaction
        if let Some(ref exit_sig) = position.exit_transaction_signature {
            match analyze_transaction_from_file(exit_sig, &position.mint).await {
                Ok(analysis) => {
                    println!("  🚪 Exit Transaction Analysis:");
                    print_transaction_analysis(&analysis, "    ");
                    position_summary.exit_analysis = Some(analysis);
                }
                Err(e) => {
                    println!("  ❌ Exit transaction analysis failed: {}", e);
                }
            }
        }
        
        // Compare stored vs extracted values
        print_comparison(&position_summary);
        
        // Accumulate totals
        if let Some(entry_fee) = position.entry_fee_lamports {
            total_entry_fees_lamports += entry_fee;
        }
        if let Some(exit_fee) = position.exit_fee_lamports {
            total_exit_fees_lamports += exit_fee;
        }
        total_sol_invested += position.entry_size_sol;
        if let Some(sol_received) = position.sol_received {
            total_sol_received += sol_received;
        }
        
        position_summaries.push(position_summary);
        println!("");
    }
    
    // Print overall summary
    print_overall_summary(&position_summaries, total_entry_fees_lamports, total_exit_fees_lamports, total_sol_invested, total_sol_received);
    
    Ok(())
}

#[derive(Debug)]
struct PositionSummary {
    symbol: String,
    mint: String,
    entry_signature: Option<String>,
    exit_signature: Option<String>,
    entry_analysis: Option<TransactionAnalysis>,
    exit_analysis: Option<TransactionAnalysis>,
    stored_entry_fee: Option<u64>,
    stored_exit_fee: Option<u64>,
    stored_sol_received: Option<f64>,
    stored_token_amount: Option<u64>,
    stored_entry_size: f64,
}

#[derive(Debug)]
struct TransactionAnalysis {
    signature: String,
    fee_lamports: Option<u64>,
    sol_balance_changes: Vec<SolBalanceChange>,
    token_balance_changes: Vec<TokenBalanceChange>,
    compute_units_consumed: Option<u64>,
    total_sol_in: f64,
    total_sol_out: f64,
    target_token_in: u64,
    target_token_out: u64,
}

#[derive(Debug)]
struct SolBalanceChange {
    account: String,
    pre_balance: u64,
    post_balance: u64,
    change_lamports: i64,
    change_sol: f64,
}

#[derive(Debug)]
struct TokenBalanceChange {
    mint: String,
    account: String,
    owner: String,
    pre_amount: Option<u64>,
    post_amount: u64,
    change: i64,
    decimals: u8,
    change_ui_amount: f64,
}

async fn analyze_transaction_from_file(signature: &str, target_mint: &str) -> Result<TransactionAnalysis, String> {
    let transaction_file = format!("data/transactions/{}.json", signature);
    
    if !Path::new(&transaction_file).exists() {
        return Err(format!("Transaction file not found: {}", transaction_file));
    }
    
    let file_content = fs::read_to_string(&transaction_file)
        .map_err(|e| format!("Failed to read transaction file: {}", e))?;
    
    let json: Value = serde_json::from_str(&file_content)
        .map_err(|e| format!("Failed to parse transaction JSON: {}", e))?;
    
    // Extract transaction data
    let transaction_data = json.get("transaction_data")
        .ok_or("Missing transaction_data")?;
    
    let transaction = transaction_data.get("transaction")
        .ok_or("Missing transaction")?;
    
    let meta = transaction_data.get("meta")
        .ok_or("Missing meta")?;
    
    // Extract fee
    let fee_lamports = meta.get("fee")
        .and_then(|f| f.as_u64());
    
    // Extract compute units
    let compute_units_consumed = meta.get("computeUnitsConsumed")
        .and_then(|c| c.as_u64());
    
    // Extract SOL balance changes
    let empty_vec = vec![];
    let pre_balances = meta.get("preBalances")
        .and_then(|b| b.as_array())
        .unwrap_or(&empty_vec);
    
    let post_balances = meta.get("postBalances")
        .and_then(|b| b.as_array())
        .unwrap_or(&empty_vec);
    
    let account_keys = transaction.get("message")
        .and_then(|m| m.get("accountKeys"))
        .and_then(|k| k.as_array())
        .unwrap_or(&empty_vec);
    
    let mut sol_balance_changes = Vec::new();
    let mut total_sol_in = 0.0;
    let mut total_sol_out = 0.0;
    
    for (i, account_key) in account_keys.iter().enumerate() {
        if let (Some(pre_bal), Some(post_bal)) = (pre_balances.get(i), post_balances.get(i)) {
            if let (Some(pre), Some(post)) = (pre_bal.as_u64(), post_bal.as_u64()) {
                let change = post as i64 - pre as i64;
                let change_sol = change as f64 / 1_000_000_000.0;
                
                if change != 0 {
                    let account = account_key.get("pubkey")
                        .and_then(|p| p.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    
                    sol_balance_changes.push(SolBalanceChange {
                        account: account.clone(),
                        pre_balance: pre,
                        post_balance: post,
                        change_lamports: change,
                        change_sol,
                    });
                    
                    if change > 0 {
                        total_sol_in += change_sol;
                    } else {
                        total_sol_out += -change_sol;
                    }
                }
            }
        }
    }
    
    // Extract token balance changes
    let mut token_balance_changes = Vec::new();
    let mut target_token_in = 0u64;
    let mut target_token_out = 0u64;
    
    if let Some(pre_token_balances) = meta.get("preTokenBalances").and_then(|b| b.as_array()) {
        if let Some(post_token_balances) = meta.get("postTokenBalances").and_then(|b| b.as_array()) {
            // Create a map of post balances by account index
            let mut post_balance_map: HashMap<usize, &Value> = HashMap::new();
            for post_balance in post_token_balances {
                if let Some(account_index) = post_balance.get("accountIndex").and_then(|i| i.as_u64()) {
                    post_balance_map.insert(account_index as usize, post_balance);
                }
            }
            
            // Compare pre and post balances
            for pre_balance in pre_token_balances {
                if let Some(account_index) = pre_balance.get("accountIndex").and_then(|i| i.as_u64()) {
                    let account_index = account_index as usize;
                    
                    if let Some(post_balance) = post_balance_map.get(&account_index) {
                        extract_token_balance_change(pre_balance, post_balance, target_mint, &mut token_balance_changes, &mut target_token_in, &mut target_token_out);
                    }
                }
            }
            
            // Check for new token accounts (only in post balances)
            for post_balance in post_token_balances {
                if let Some(account_index) = post_balance.get("accountIndex").and_then(|i| i.as_u64()) {
                    let account_index = account_index as usize;
                    
                    // Check if this account wasn't in pre balances
                    let was_in_pre = pre_token_balances.iter().any(|pre| {
                        pre.get("accountIndex").and_then(|i| i.as_u64()).map(|i| i as usize) == Some(account_index)
                    });
                    
                    if !was_in_pre {
                        extract_token_balance_change(&Value::Null, post_balance, target_mint, &mut token_balance_changes, &mut target_token_in, &mut target_token_out);
                    }
                }
            }
        }
    }
    
    Ok(TransactionAnalysis {
        signature: signature.to_string(),
        fee_lamports,
        sol_balance_changes,
        token_balance_changes,
        compute_units_consumed,
        total_sol_in,
        total_sol_out,
        target_token_in,
        target_token_out,
    })
}

fn extract_token_balance_change(
    pre_balance: &Value,
    post_balance: &Value,
    target_mint: &str,
    token_balance_changes: &mut Vec<TokenBalanceChange>,
    target_token_in: &mut u64,
    target_token_out: &mut u64,
) {
    let mint = post_balance.get("mint")
        .and_then(|m| m.as_str())
        .unwrap_or("");
    
    let account = post_balance.get("accountIndex")
        .and_then(|i| i.as_u64())
        .map(|i| format!("account_{}", i))
        .unwrap_or_else(|| "unknown".to_string());
    
    let owner = post_balance.get("owner")
        .and_then(|o| o.as_str())
        .unwrap_or("unknown")
        .to_string();
    
    let pre_amount = if pre_balance.is_null() {
        None
    } else {
        pre_balance.get("uiTokenAmount")
            .and_then(|ui| ui.get("amount"))
            .and_then(|a| a.as_str())
            .and_then(|s| s.parse::<u64>().ok())
    };
    
    let post_amount = post_balance.get("uiTokenAmount")
        .and_then(|ui| ui.get("amount"))
        .and_then(|a| a.as_str())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    
    let decimals = post_balance.get("uiTokenAmount")
        .and_then(|ui| ui.get("decimals"))
        .and_then(|d| d.as_u64())
        .unwrap_or(0) as u8;
    
    let change = post_amount as i64 - pre_amount.unwrap_or(0) as i64;
    let change_ui_amount = change as f64 / 10_f64.powi(decimals as i32);
    
    if change != 0 {
        token_balance_changes.push(TokenBalanceChange {
            mint: mint.to_string(),
            account,
            owner,
            pre_amount,
            post_amount,
            change,
            decimals,
            change_ui_amount,
        });
        
        // Track target token changes
        if mint == target_mint {
            if change > 0 {
                *target_token_in += change as u64;
            } else {
                *target_token_out += (-change) as u64;
            }
        }
    }
}

fn print_transaction_analysis(analysis: &TransactionAnalysis, indent: &str) {
    println!("{}📋 Signature: {}...{}", indent, &analysis.signature[..8], &analysis.signature[analysis.signature.len()-8..]);
    
    if let Some(fee) = analysis.fee_lamports {
        println!("{}💳 Transaction Fee: {} lamports ({:.9} SOL)", indent, fee, fee as f64 / 1_000_000_000.0);
    }
    
    if let Some(compute_units) = analysis.compute_units_consumed {
        println!("{}🖥️ Compute Units: {}", indent, compute_units);
    }
    
    println!("{}💰 SOL Flow: In: {:.9} SOL, Out: {:.9} SOL", indent, analysis.total_sol_in, analysis.total_sol_out);
    println!("{}🪙 Target Token Flow: In: {}, Out: {}", indent, analysis.target_token_in, analysis.target_token_out);
    
    if !analysis.sol_balance_changes.is_empty() {
        println!("{}🔄 SOL Balance Changes:", indent);
        for change in &analysis.sol_balance_changes {
            println!("{}  {} → {:+.9} SOL ({}...{})",
                     indent,
                     if change.change_sol > 0.0 { "📈" } else { "📉" },
                     change.change_sol,
                     &change.account[..8],
                     &change.account[change.account.len().saturating_sub(8)..]);
        }
    }
    
    if !analysis.token_balance_changes.is_empty() {
        println!("{}🎫 Token Balance Changes:", indent);
        for change in &analysis.token_balance_changes {
            println!("{}  {} → {:+.6} tokens ({}...{}, owner: {}...{})",
                     indent,
                     if change.change > 0 { "📈" } else { "📉" },
                     change.change_ui_amount,
                     &change.mint[..8],
                     &change.mint[change.mint.len().saturating_sub(8)..],
                     &change.owner[..8],
                     &change.owner[change.owner.len().saturating_sub(8)..]);
        }
    }
}

fn print_comparison(summary: &PositionSummary) {
    println!("  📊 STORED vs EXTRACTED COMPARISON:");
    
    // Entry fee comparison
    if let (Some(stored_entry_fee), Some(ref entry_analysis)) = (&summary.stored_entry_fee, &summary.entry_analysis) {
        if let Some(extracted_fee) = entry_analysis.fee_lamports {
            let match_icon = if *stored_entry_fee == extracted_fee { "✅" } else { "❌" };
            println!("    💳 Entry Fee: {} Stored: {} lamports | Extracted: {} lamports",
                     match_icon, stored_entry_fee, extracted_fee);
        }
    }
    
    // Exit fee comparison
    if let (Some(stored_exit_fee), Some(ref exit_analysis)) = (&summary.stored_exit_fee, &summary.exit_analysis) {
        if let Some(extracted_fee) = exit_analysis.fee_lamports {
            let match_icon = if *stored_exit_fee == extracted_fee { "✅" } else { "❌" };
            println!("    💳 Exit Fee: {} Stored: {} lamports | Extracted: {} lamports",
                     match_icon, stored_exit_fee, extracted_fee);
        }
    }
    
    // Token amount comparison
    if let (Some(stored_token_amount), Some(ref entry_analysis)) = (&summary.stored_token_amount, &summary.entry_analysis) {
        let extracted_tokens = entry_analysis.target_token_in;
        if extracted_tokens > 0 {
            let match_icon = if *stored_token_amount == extracted_tokens { "✅" } else { "❌" };
            println!("    🪙 Tokens Received: {} Stored: {} | Extracted: {}",
                     match_icon, stored_token_amount, extracted_tokens);
        }
    }
    
    // SOL received comparison (from exit transaction)
    if let (Some(stored_sol_received), Some(ref exit_analysis)) = (&summary.stored_sol_received, &summary.exit_analysis) {
        let extracted_sol_received = exit_analysis.total_sol_in;
        if extracted_sol_received > 0.0 {
            let diff = (stored_sol_received - extracted_sol_received).abs();
            let match_icon = if diff < 0.000001 { "✅" } else { "❌" };
            println!("    💰 SOL Received: {} Stored: {:.6} SOL | Extracted: {:.6} SOL (diff: {:.9})",
                     match_icon, stored_sol_received, extracted_sol_received, diff);
        }
    }
}

fn print_overall_summary(
    summaries: &[PositionSummary],
    total_entry_fees: u64,
    total_exit_fees: u64,
    total_sol_invested: f64,
    total_sol_received: f64,
) {
    println!("📈 OVERALL SUMMARY");
    println!("═══════════════════════════════════════════════════════════════");
    
    println!("🏢 Portfolio Overview:");
    println!("  📊 Total Positions Analyzed: {}", summaries.len());
    println!("  💰 Total SOL Invested: {:.6} SOL", total_sol_invested);
    println!("  💵 Total SOL Received: {:.6} SOL", total_sol_received);
    println!("  📈 Net SOL Change: {:+.6} SOL", total_sol_received - total_sol_invested);
    
    println!("\n💳 Fee Analysis:");
    println!("  📥 Total Entry Fees: {} lamports ({:.6} SOL)", total_entry_fees, total_entry_fees as f64 / 1_000_000_000.0);
    println!("  📤 Total Exit Fees: {} lamports ({:.6} SOL)", total_exit_fees, total_exit_fees as f64 / 1_000_000_000.0);
    println!("  🧮 Total Fees Paid: {} lamports ({:.6} SOL)", 
             total_entry_fees + total_exit_fees, 
             (total_entry_fees + total_exit_fees) as f64 / 1_000_000_000.0);
    
    // Data quality analysis
    let mut fee_matches = 0;
    let mut fee_mismatches = 0;
    let mut missing_data = 0;
    
    for summary in summaries {
        if let (Some(stored_entry), Some(ref entry_analysis)) = (&summary.stored_entry_fee, &summary.entry_analysis) {
            if let Some(extracted_entry) = entry_analysis.fee_lamports {
                if *stored_entry == extracted_entry {
                    fee_matches += 1;
                } else {
                    fee_mismatches += 1;
                }
            } else {
                missing_data += 1;
            }
        } else {
            missing_data += 1;
        }
        
        if let (Some(stored_exit), Some(ref exit_analysis)) = (&summary.stored_exit_fee, &summary.exit_analysis) {
            if let Some(extracted_exit) = exit_analysis.fee_lamports {
                if *stored_exit == extracted_exit {
                    fee_matches += 1;
                } else {
                    fee_mismatches += 1;
                }
            } else {
                missing_data += 1;
            }
        } else {
            missing_data += 1;
        }
    }
    
    println!("\n🔍 Data Quality Assessment:");
    println!("  ✅ Fee Data Matches: {}", fee_matches);
    println!("  ❌ Fee Data Mismatches: {}", fee_mismatches);
    println!("  ⚠️ Missing Data Points: {}", missing_data);
    
    let total_checks = fee_matches + fee_mismatches;
    if total_checks > 0 {
        let accuracy = (fee_matches as f64 / total_checks as f64) * 100.0;
        println!("  📊 Data Accuracy: {:.1}%", accuracy);
    }
}
