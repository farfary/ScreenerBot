// analyze_transaction.rs - Analyze a specific transaction signature
use screenerbot::transactions::*;
use screenerbot::global::{ read_configs, Configs };
use screenerbot::logger::{ log, LogTag };
use colored::Colorize;
use std::env;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <transaction_signature>", args[0]);
        eprintln!(
            "Example: {} 4j3eoxkAbfDrchhaYqFffZMTswbTTxQKBPWUSpFnXemybvaXuvbdQMCbG25R557y3Nzaf1bqxUwWGPhZcMjuonC4",
            args[0]
        );
        std::process::exit(1);
    }

    let transaction_signature = &args[1];

    println!("{}", "=".repeat(100).bright_cyan());
    println!(
        "{}",
        format!("🔍 ANALYZING TRANSACTION: {}", transaction_signature).bright_yellow().bold()
    );
    println!("{}", "=".repeat(100).bright_cyan());

    // Load configs
    let configs = read_configs("configs.json")?;

    // Initialize transaction fetcher
    let fetcher = TransactionFetcher::new(configs, None)?;
    let analyzer = TransactionAnalyzer::new();

    // Fetch the specific transaction
    log(LogTag::System, "INFO", &format!("Fetching transaction: {}", transaction_signature));

    let transaction_result =
        fetcher.get_transaction_details_with_fallback(transaction_signature).await;

    match transaction_result {
        Ok(Some(transaction_result)) => {
            println!("\n{}", "📊 TRANSACTION OVERVIEW".bright_green().bold());
            println!("{}", "-".repeat(50).green());

            let transaction = &transaction_result.transaction;
            let meta = transaction_result.meta.as_ref();

            // Basic transaction info
            println!("Signature: {}", transaction_signature.bright_white());
            println!("Slot: {}", transaction_result.slot.to_string().bright_white());
            if let Some(block_time) = transaction_result.block_time {
                let datetime = chrono::DateTime
                    ::from_timestamp(block_time as i64, 0)
                    .unwrap_or_default()
                    .format("%Y-%m-%d %H:%M:%S UTC");
                println!("Block Time: {}", datetime.to_string().bright_white());
            }

            if let Some(meta) = meta {
                println!("Status: {}", if meta.err.is_none() {
                    "✅ SUCCESS".bright_green()
                } else {
                    format!("❌ FAILED: {:?}", meta.err).bright_red()
                });
                println!(
                    "Fee: {} SOL",
                    ((meta.fee as f64) / 1_000_000_000.0).to_string().bright_white()
                );
            }

            // Instructions Analysis
            println!("\n{}", "🔧 INSTRUCTIONS".bright_blue().bold());
            println!("{}", "-".repeat(50).blue());

            for (i, instruction) in transaction.message.instructions.iter().enumerate() {
                let program_key = if let Some(program_id_index) = instruction.program_id_index {
                    if (program_id_index as usize) < transaction.message.account_keys.len() {
                        &transaction.message.account_keys[program_id_index as usize]
                    } else {
                        "Invalid Index"
                    }
                } else if let Some(ref program_id) = instruction.program_id {
                    program_id
                } else {
                    "Unknown"
                };
                let program_name = get_program_name(program_key);

                let accounts: Vec<String> = instruction.accounts
                    .iter()
                    .filter_map(|&idx| {
                        if (idx as usize) < transaction.message.account_keys.len() {
                            let account = &transaction.message.account_keys[idx as usize];
                            Some(format!("{}...{}", &account[..8], &account[account.len() - 4..]))
                        } else {
                            None
                        }
                    })
                    .collect();

                let data_preview = if instruction.data.len() > 32 {
                    format!(
                        "{}... ({} bytes)",
                        bs58::encode(&instruction.data[..16]).into_string(),
                        instruction.data.len()
                    )
                } else {
                    bs58::encode(&instruction.data).into_string()
                };

                println!(
                    "📝 {}Instruction {}: {} ({}...{})",
                    "  ".repeat(0),
                    i.to_string().bright_white(),
                    program_name.bright_yellow(),
                    &program_key[..8].dimmed(),
                    &program_key[program_key.len() - 4..].dimmed()
                );
                println!("   💾 Data: {}", data_preview.bright_white());
                if !accounts.is_empty() {
                    println!("   🔑 Accounts: {}", accounts.join(", ").dimmed());
                }
                println!();
            }

            // Balance Changes Analysis
            if let Some(meta) = meta {
                println!("\n{}", "⚖️ BALANCE CHANGES".bright_magenta().bold());
                println!("{}", "-".repeat(50).magenta());

                let mut has_changes = false;

                for (i, account_key) in transaction.message.account_keys.iter().enumerate() {
                    if i < meta.pre_balances.len() && i < meta.post_balances.len() {
                        let pre_balance = meta.pre_balances[i];
                        let post_balance = meta.post_balances[i];
                        let change = (post_balance as i64) - (pre_balance as i64);

                        if change != 0 {
                            has_changes = true;
                            let account_type = if
                                account_key == &transaction.message.account_keys[0]
                            {
                                "Fee Payer"
                            } else if get_program_name(account_key) != "Unknown" {
                                "Program"
                            } else {
                                "Account"
                            };

                            let change_display = if change > 0 {
                                format!("+{} SOL", (change as f64) / 1_000_000_000.0).bright_green()
                            } else {
                                format!("{} SOL", (change as f64) / 1_000_000_000.0).bright_red()
                            };

                            println!(
                                "📍 {}...{} ({})",
                                &account_key[..8].bright_white(),
                                &account_key[account_key.len() - 8..].bright_white(),
                                account_type.dimmed()
                            );
                            println!(
                                "   ⚖️ Pre:  {} SOL",
                                ((pre_balance as f64) / 1_000_000_000.0).to_string().dimmed()
                            );
                            println!(
                                "   📊 Post: {} SOL",
                                ((post_balance as f64) / 1_000_000_000.0).to_string().dimmed()
                            );
                            println!("   📈 Change: {}", change_display);
                            println!();
                        }
                    }
                }

                if !has_changes {
                    println!("No SOL balance changes detected.");
                }

                // Token Transfers Analysis
                println!("\n{}", "🪙 TOKEN TRANSFERS".bright_yellow().bold());
                println!("{}", "-".repeat(50).yellow());

                // Analyze transaction for token transfers
                let analysis = analyzer.analyze_transaction(&transaction_result);

                if !analysis.token_transfers.is_empty() {
                    for (i, transfer) in analysis.token_transfers.iter().enumerate() {
                        let amount_f64 = transfer.amount.parse::<f64>().unwrap_or(0.0);
                        let ui_amount = if transfer.decimals > 0 {
                            amount_f64 / (10_f64).powi(transfer.decimals as i32)
                        } else {
                            amount_f64
                        };

                        let from_display = if let Some(ref from) = transfer.from {
                            if from.len() >= 16 {
                                format!("{}...{}", &from[..8], &from[from.len() - 8..])
                            } else {
                                from.clone()
                            }
                        } else {
                            "N/A".to_string()
                        };

                        let to_display = if let Some(ref to) = transfer.to {
                            if to.len() >= 16 {
                                format!("{}...{}", &to[..8], &to[to.len() - 8..])
                            } else {
                                to.clone()
                            }
                        } else {
                            "N/A".to_string()
                        };

                        let mint_display = if transfer.mint.len() >= 16 {
                            format!(
                                "{}...{}",
                                &transfer.mint[..8],
                                &transfer.mint[transfer.mint.len() - 8..]
                            )
                        } else {
                            transfer.mint.clone()
                        };

                        println!(
                            "🪙 Transfer {}: {}",
                            (i + 1).to_string().bright_white(),
                            mint_display.bright_yellow()
                        );
                        println!("   📤 From: {}", from_display.dimmed());
                        println!("   📥 To: {}", to_display.dimmed());
                        println!(
                            "   💰 Amount: {} (Decimals: {})",
                            format!("{:.6}", ui_amount).bright_white(),
                            transfer.decimals.to_string().dimmed()
                        );
                        println!();
                    }
                } else {
                    println!("No token transfers detected.");
                }

                // Swap Analysis
                println!("\n{}", "🔄 SWAP ANALYSIS".bright_cyan().bold());
                println!("{}", "-".repeat(50).cyan());

                // Enhanced swap detection
                let is_swap_detected =
                    analysis.is_swap || detect_manual_swap_patterns(&transaction_result, &analysis);

                if is_swap_detected {
                    println!("✅ {}", "SWAP DETECTED".bright_green().bold());

                    // Find the actual DEX program (not compute budget or system programs)
                    let mut dex_program_found = false;

                    // First check analyzer's swap info
                    if let Some(ref swap_info) = analysis.swap_info {
                        if !is_system_program(&swap_info.program_id) {
                            println!("DEX: {}", swap_info.dex_name.bright_white());
                            println!("Program ID: {}", swap_info.program_id.bright_white());
                            dex_program_found = true;
                        }
                    }

                    // Then check program interactions for non-system programs
                    if !dex_program_found {
                        for program_interaction in &analysis.program_interactions {
                            if !is_system_program(&program_interaction.program_id) {
                                if let Some(ref dex_name) = program_interaction.dex_name {
                                    println!("DEX: {}", dex_name.bright_white());
                                } else {
                                    let program_name = get_program_name(
                                        &program_interaction.program_id
                                    );
                                    println!("DEX: {}", program_name.bright_white());
                                }
                                println!(
                                    "Program ID: {}",
                                    program_interaction.program_id.bright_white()
                                );
                                dex_program_found = true;
                                break;
                            }
                        }
                    }

                    // Manual detection - find the likely DEX program
                    if !dex_program_found {
                        for instruction in &transaction.message.instructions {
                            let program_key = if
                                let Some(program_id_index) = instruction.program_id_index
                            {
                                if
                                    (program_id_index as usize) <
                                    transaction.message.account_keys.len()
                                {
                                    &transaction.message.account_keys[program_id_index as usize]
                                } else {
                                    continue;
                                }
                            } else if let Some(ref program_id) = instruction.program_id {
                                program_id
                            } else {
                                continue;
                            };

                            if
                                is_likely_dex_program(program_key) &&
                                !is_system_program(program_key)
                            {
                                let program_name = get_program_name(program_key);
                                println!("DEX: {} (Manual Detection)", program_name.bright_white());
                                println!("Program ID: {}", program_key.bright_white());
                                dex_program_found = true;
                                break;
                            }
                        }
                    }

                    // Enhanced swap details analysis with direction detection
                    if analysis.token_transfers.len() >= 1 {
                        println!("\n📊 Swap Details:");

                        // Group by different tokens
                        let mut token_groups: std::collections::HashMap<
                            String,
                            Vec<&TokenTransfer>
                        > = std::collections::HashMap::new();
                        for transfer in &analysis.token_transfers {
                            token_groups
                                .entry(transfer.mint.clone())
                                .or_insert_with(Vec::new)
                                .push(transfer);
                        }

                        // Determine swap direction based on tokens and SOL balance changes
                        let has_wsol = token_groups.contains_key(
                            "So11111111111111111111111111111112"
                        );
                        let sol_balance_change = analysis.sol_balance_change;

                        if has_wsol && sol_balance_change != 0 {
                            // This is a token <-> SOL/WSOL swap
                            let wsol_transfers =
                                &token_groups["So11111111111111111111111111111112"];
                            let other_tokens: Vec<_> = token_groups
                                .iter()
                                .filter(|(mint, _)| *mint != "So11111111111111111111111112")
                                .collect();

                            if !other_tokens.is_empty() {
                                let (other_mint, other_transfers) = other_tokens[0];

                                // Calculate total amounts
                                let wsol_total: f64 = wsol_transfers
                                    .iter()
                                    .map(|t| {
                                        let amount_f64 = t.amount.parse::<f64>().unwrap_or(0.0);
                                        if t.decimals > 0 {
                                            amount_f64 / (10_f64).powi(t.decimals as i32)
                                        } else {
                                            amount_f64
                                        }
                                    })
                                    .sum();

                                let token_total: f64 = other_transfers
                                    .iter()
                                    .map(|t| {
                                        let amount_f64 = t.amount.parse::<f64>().unwrap_or(0.0);
                                        if t.decimals > 0 {
                                            amount_f64 / (10_f64).powi(t.decimals as i32)
                                        } else {
                                            amount_f64
                                        }
                                    })
                                    .sum();

                                let other_token_display = if other_mint.len() >= 16 {
                                    format!(
                                        "{}...{}",
                                        &other_mint[..8],
                                        &other_mint[other_mint.len() - 8..]
                                    )
                                } else {
                                    other_mint.clone()
                                };

                                // Determine direction based on SOL balance change
                                if sol_balance_change > 0 {
                                    // User gained SOL, so this is Token -> SOL swap
                                    println!(
                                        "� {}",
                                        "SWAP TYPE: TOKEN → SOL/WSOL".bright_green().bold()
                                    );
                                    println!(
                                        "�🔴 Input Token: {} ({:.6} tokens)",
                                        other_token_display.bright_red(),
                                        token_total
                                    );
                                    println!(
                                        "🟢 Output: SOL/WSOL ({} SOL + {} WSOL)",
                                        format!(
                                            "{:.6}",
                                            ((sol_balance_change as f64) / 1_000_000_000.0).abs()
                                        ).bright_green(),
                                        format!("{:.6}", wsol_total).bright_green()
                                    );
                                } else {
                                    // User lost SOL, so this is SOL -> Token swap
                                    println!(
                                        "🔄 {}",
                                        "SWAP TYPE: SOL/WSOL → TOKEN".bright_green().bold()
                                    );
                                    println!(
                                        "🔴 Input: SOL/WSOL ({} SOL + {} WSOL)",
                                        format!(
                                            "{:.6}",
                                            ((sol_balance_change as f64) / 1_000_000_000.0).abs()
                                        ).bright_red(),
                                        format!("{:.6}", wsol_total).bright_red()
                                    );
                                    println!(
                                        "🟢 Output Token: {} ({:.6} tokens)",
                                        other_token_display.bright_green(),
                                        token_total
                                    );
                                }

                                // Calculate exchange rate
                                if wsol_total > 0.0 && token_total > 0.0 {
                                    let sol_equivalent =
                                        wsol_total +
                                        ((sol_balance_change as f64) / 1_000_000_000.0).abs();
                                    if sol_equivalent > 0.0 {
                                        println!(
                                            "💱 Exchange Rate: 1 SOL ≈ {:.2} tokens",
                                            token_total / sol_equivalent
                                        );
                                        println!(
                                            "💱 Token Price: ~{:.8} SOL per token",
                                            sol_equivalent / token_total
                                        );
                                    }
                                }
                            }
                        } else if token_groups.len() >= 2 {
                            // Regular token-to-token swap
                            println!("🔄 {}", "SWAP TYPE: TOKEN → TOKEN".bright_green().bold());

                            for (i, (token_mint, transfers)) in token_groups.iter().enumerate() {
                                let total_amount: f64 = transfers
                                    .iter()
                                    .map(|t| {
                                        let amount_f64 = t.amount.parse::<f64>().unwrap_or(0.0);
                                        if t.decimals > 0 {
                                            amount_f64 / (10_f64).powi(t.decimals as i32)
                                        } else {
                                            amount_f64
                                        }
                                    })
                                    .sum();

                                let token_display = if token_mint.len() >= 16 {
                                    format!(
                                        "{}...{}",
                                        &token_mint[..8],
                                        &token_mint[token_mint.len() - 8..]
                                    )
                                } else {
                                    token_mint.clone()
                                };

                                if i == 0 {
                                    println!(
                                        "🔴 Input Token: {} ({:.6} tokens)",
                                        token_display.bright_red(),
                                        total_amount
                                    );
                                } else {
                                    println!(
                                        "🟢 Output Token: {} ({:.6} tokens)",
                                        token_display.bright_green(),
                                        total_amount
                                    );
                                }
                            }
                        } else {
                            // Single token with SOL changes - likely liquidity operation or complex swap
                            println!("🔄 {}", "COMPLEX TRANSACTION".bright_yellow().bold());
                            if sol_balance_change != 0 {
                                println!(
                                    "💰 SOL Balance Change: {:.6} SOL",
                                    (sol_balance_change as f64) / 1_000_000_000.0
                                );
                            }

                            for (token_mint, transfers) in &token_groups {
                                let total_amount: f64 = transfers
                                    .iter()
                                    .map(|t| {
                                        let amount_f64 = t.amount.parse::<f64>().unwrap_or(0.0);
                                        if t.decimals > 0 {
                                            amount_f64 / (10_f64).powi(t.decimals as i32)
                                        } else {
                                            amount_f64
                                        }
                                    })
                                    .sum();

                                let token_display = if
                                    token_mint == "So11111111111111111111111111111112"
                                {
                                    "SOL (Wrapped)".to_string()
                                } else if token_mint.len() >= 16 {
                                    format!(
                                        "{}...{}",
                                        &token_mint[..8],
                                        &token_mint[token_mint.len() - 8..]
                                    )
                                } else {
                                    token_mint.clone()
                                };

                                println!(
                                    "🪙 Token Activity: {} ({:.6} tokens)",
                                    token_display.bright_white(),
                                    total_amount
                                );
                            }
                        }
                    } else {
                        println!("No detailed token transfer data available for swap analysis.");
                    }
                } else {
                    println!("❌ {}", "NO SWAP DETECTED".bright_red());
                }

                // Log Analysis
                if let Some(ref log_messages) = meta.log_messages {
                    println!("\n{}", "📋 LOG MESSAGES".bright_white().bold());
                    println!("{}", "-".repeat(50).white());

                    for (i, log_msg) in log_messages.iter().take(10).enumerate() {
                        println!("[{}] {}", i, log_msg.dimmed());
                    }

                    if log_messages.len() > 10 {
                        println!("... and {} more log messages", log_messages.len() - 10);
                    }
                }
            }

            println!("\n{}", "=".repeat(100).bright_cyan());
            println!("{}", "✅ ANALYSIS COMPLETE".bright_green().bold());
            println!("{}", "=".repeat(100).bright_cyan());
        }
        Ok(None) => {
            println!("❌ Transaction not found: {}", transaction_signature);
        }
        Err(e) => {
            println!("❌ Error fetching transaction: {}", e);
        }
    }

    Ok(())
}

/// Get human-readable program name
fn get_program_name(program_id: &str) -> &'static str {
    match program_id {
        "11111111111111111111111111111111" => "System Program",
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => "SPL Token",
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" => "Token Extensions",
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" => "Associated Token",
        "ComputeBudget111111111111111111111111111111" => "Compute Budget",
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => "Jupiter V6",
        "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => "Jupiter V4",
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => "Raydium AMM",
        "5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h" => "Raydium CLMM",
        "27haf8L6oxUeXrHrgEgsexjSY5hbVUWEmvv9Nyxg8vQv" => "Raydium CPMM",
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => "Raydium CAMM",
        "PhoeNiX7VDBAENnyBzGGqndP2rW4G9fqmKNaVphh1zQ" => "Phoenix",
        "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX" => "Serum DEX",
        "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP" => "Orca Whirlpool",
        "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1" => "Orca V1",
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => "Orca V2",
        "PumpFun6NEdKZz5uXgKSBzw1b4cYEyTCHYHmZ7Xr6e" => "PumpFun",
        "pAMMBay6eYBQJFijNn2JYdVdef4BUVyqZ4sFA5fXEA" => "Unknown DEX/AMM", // The program from this transaction
        _ => "Unknown",
    }
}

/// Enhanced swap detection that looks for swap patterns beyond the basic analyzer
fn detect_manual_swap_patterns(
    transaction_result: &TransactionResult,
    analysis: &TransactionAnalysis
) -> bool {
    // Check if we have multiple token transfers (strong swap indicator)
    if analysis.token_transfers.len() >= 2 {
        // Check for different tokens being transferred
        let mut unique_mints = std::collections::HashSet::new();
        for transfer in &analysis.token_transfers {
            unique_mints.insert(&transfer.mint);
        }

        // If we have 2+ different tokens, likely a swap
        if unique_mints.len() >= 2 {
            return true;
        }
    }

    // Check for SOL balance changes combined with token transfers
    if !analysis.token_transfers.is_empty() && analysis.sol_balance_change != 0 {
        return true;
    }

    // Check for unknown programs that might be DEXes
    for instruction in &transaction_result.transaction.message.instructions {
        let program_key = if let Some(program_id_index) = instruction.program_id_index {
            if
                (program_id_index as usize) <
                transaction_result.transaction.message.account_keys.len()
            {
                &transaction_result.transaction.message.account_keys[program_id_index as usize]
            } else {
                continue;
            }
        } else if let Some(ref program_id) = instruction.program_id {
            program_id
        } else {
            continue;
        };

        // Check for known DEX patterns or unknown programs with complex interactions
        if is_likely_dex_program(program_key) {
            return true;
        }
    }

    false
}

/// Check if a program ID is likely a DEX based on patterns and known programs
fn is_likely_dex_program(program_id: &str) -> bool {
    match program_id {
        // Known DEX programs
        | "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4"
        | "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB"
        | "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8"
        | "5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h"
        | "27haf8L6oxUeXrHrgEgsexjSY5hbVUWEmvv9Nyxg8vQv"
        | "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK"
        | "PhoeNiX7VDBAENnyBzGGqndP2rW4G9fqmKNaVphh1zQ"
        | "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX"
        | "9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP"
        | "DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1"
        | "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc"
        | "PumpFun6NEdKZz5uXgKSBzw1b4cYEyTCHYmZ7Xr6e"
        | "pAMMBay6eYBQJFijNn2JYdVdef4BUVyqZ4sFA5fXEA" => true, // The program from this transaction
        _ => false,
    }
}

/// Check if a program is a system/infrastructure program (not a DEX)
fn is_system_program(program_id: &str) -> bool {
    match program_id {
        | "11111111111111111111111111111111" // System Program
        | "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" // SPL Token
        | "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" // Token Extensions
        | "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" // Associated Token
        | "ComputeBudget111111111111111111111111111111" => true, // Compute Budget
        _ => false,
    }
}
