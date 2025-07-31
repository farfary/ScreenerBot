use screenerbot::positions::*;
use screenerbot::wallet::*;
use screenerbot::logger::*;
use screenerbot::global::*;
use screenerbot::rpc::*;
use screenerbot::utils::*;

use chrono::{ DateTime, Utc };
use serde::{ Serialize, Deserialize };
use std::collections::HashMap;
use reqwest;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionAnalysis {
    signature: String,
    position_mint: String,
    position_symbol: String,
    timestamp: DateTime<Utc>,

    // Transaction Details
    total_sol_received_lamports: u64,
    total_sol_received_sol: f64,

    // ATA Detection Results
    ata_detected: bool,
    ata_rent_detected_lamports: u64,
    ata_rent_detected_sol: f64,

    // Calculated Trading SOL
    trading_sol_calculated_lamports: u64,
    trading_sol_calculated_sol: f64,

    // Position Data
    token_amount_sold: u64,
    effective_exit_price_original: f64,
    effective_exit_price_corrected: f64,

    // Problem Analysis
    has_problem: bool,
    problem_type: String,
    suggested_fix: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FixResults {
    total_analyzed: usize,
    problems_found: usize,
    successful_fixes: usize,
    failed_fixes: usize,
    summary: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 ATA Transaction Analyzer & Fixer");
    println!("====================================");

    // Initialize systems
    let configs = read_configs("configs.json")?;
    let client = reqwest::Client::new();

    // Load positions
    let positions = load_positions_from_file();
    println!("📋 Loaded {} positions for analysis", positions.len());

    // Analyze all positions with exit transactions
    let mut analyses = Vec::new();
    let mut problems_found = 0;

    for position in &positions {
        if let Some(exit_signature) = &position.exit_transaction_signature {
            println!("\n🔍 Analyzing transaction: {}", exit_signature);

            match analyze_transaction_ata_issues(&client, exit_signature, position, &configs).await {
                Ok(analysis) => {
                    if analysis.has_problem {
                        problems_found += 1;
                        println!("❌ Problem found: {}", analysis.problem_type);
                    } else {
                        println!("✅ Transaction looks correct");
                    }
                    analyses.push(analysis);
                }
                Err(e) => {
                    println!("⚠️  Failed to analyze {}: {}", exit_signature, e);
                }
            }
        }
    }

    println!("\n📊 ANALYSIS SUMMARY");
    println!("==================");
    println!("Total positions analyzed: {}", analyses.len());
    println!("Problems found: {}", problems_found);

    // Group problems by type
    let mut problem_types = HashMap::new();
    for analysis in &analyses {
        if analysis.has_problem {
            *problem_types.entry(analysis.problem_type.clone()).or_insert(0) += 1;
        }
    }

    println!("\nProblem types:");
    for (problem_type, count) in &problem_types {
        println!("  {}: {} occurrences", problem_type, count);
    }

    // Save detailed analysis report
    let report_filename = format!(
        "ata_analysis_report_{}.json",
        Utc::now().format("%Y%m%d_%H%M%S")
    );

    let report_data = serde_json::to_string_pretty(&analyses)?;
    std::fs::write(&report_filename, report_data)?;
    println!("\n💾 Detailed report saved to: {}", report_filename);

    // Ask user if they want to apply fixes
    println!("\n🔧 REPAIR OPTIONS");
    println!("=================");
    println!("Found {} transactions with ATA-related issues", problems_found);

    if problems_found > 0 {
        println!("Do you want to apply automated fixes? (y/n)");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if input.trim().to_lowercase() == "y" {
            let fix_results = apply_ata_fixes(&analyses).await?;
            print_fix_results(&fix_results);
        }
    }

    // Generate recommendations
    print_recommendations(&analyses);

    Ok(())
}

async fn analyze_transaction_ata_issues(
    client: &reqwest::Client,
    transaction_signature: &str,
    position: &Position,
    configs: &Configs
) -> Result<TransactionAnalysis, Box<dyn std::error::Error>> {
    // Re-analyze the transaction with enhanced ATA detection
    let (effective_price, input_change, output_change, _, ata_detected, ata_rent, sol_from_trade) =
        calculate_effective_price_with_ata_detection(
            client,
            transaction_signature,
            &position.mint,
            SOL_MINT,
            &get_wallet_address()?,
            &configs.rpc_url,
            configs
        ).await?;

    // Determine problem type and severity
    let total_sol_lamports = output_change;
    let total_sol = lamports_to_sol(total_sol_lamports);
    let ata_rent_sol = lamports_to_sol(ata_rent);
    let trading_sol = lamports_to_sol(sol_from_trade);

    let mut has_problem = false;
    let mut problem_type = String::new();
    let mut suggested_fix = String::new();

    // Problem 1: Zero trading SOL (entire amount classified as ATA rent)
    if ata_detected && sol_from_trade == 0 && total_sol_lamports > 0 {
        has_problem = true;
        problem_type = "ZERO_TRADING_SOL".to_string();
        suggested_fix = format!(
            "ATA rent calculation too aggressive. Total SOL: {:.6}, ATA rent: {:.6}. Should use multi-method validation.",
            total_sol,
            ata_rent_sol
        );
    }

    // Problem 2: ATA rent larger than total SOL received
    if ata_detected && ata_rent > total_sol_lamports {
        has_problem = true;
        problem_type = "ATA_RENT_EXCEEDS_TOTAL".to_string();
        suggested_fix = format!(
            "ATA rent ({:.6} SOL) exceeds total SOL received ({:.6} SOL). Logic error in detection.",
            ata_rent_sol,
            total_sol
        );
    }

    // Problem 3: Suspicious small trading amounts (< 0.0001 SOL for 0.002 SOL trades)
    if ata_detected && trading_sol > 0.0 && trading_sol < 0.0001 && total_sol > 0.002 {
        has_problem = true;
        problem_type = "SUSPICIOUS_SMALL_TRADING_SOL".to_string();
        suggested_fix = format!(
            "Trading SOL ({:.6}) suspiciously small compared to total ({:.6}). May be over-estimating ATA rent.",
            trading_sol,
            total_sol
        );
    }

    // Problem 4: No ATA detection but amount suggests ATA closure
    if !ata_detected && total_sol > 0.002 && total_sol < 0.005 {
        // Check if this might be a missed ATA closure
        let ata_rent_standard = 0.00203928; // Standard ATA rent
        let potential_trading_sol = total_sol - ata_rent_standard;

        if potential_trading_sol > 0.0001 && potential_trading_sol < 0.002 {
            has_problem = true;
            problem_type = "MISSED_ATA_DETECTION".to_string();
            suggested_fix = format!(
                "Total SOL ({:.6}) suggests ATA closure not detected. Potential trading SOL: {:.6}",
                total_sol,
                potential_trading_sol
            );
        }
    }

    // Calculate corrected effective price
    let corrected_effective_price = if let Some(token_amount) = position.token_amount {
        if token_amount > 0 {
            let token_ui = (token_amount as f64) / 1_000_000.0; // Assume 6 decimals, adjust as needed
            if token_ui > 0.0 {
                trading_sol / token_ui
            } else {
                effective_price
            }
        } else {
            effective_price
        }
    } else {
        effective_price
    };

    Ok(TransactionAnalysis {
        signature: transaction_signature.to_string(),
        position_mint: position.mint.clone(),
        position_symbol: position.symbol.clone(),
        timestamp: position.exit_time.unwrap_or(Utc::now()),

        total_sol_received_lamports: total_sol_lamports,
        total_sol_received_sol: total_sol,

        ata_detected,
        ata_rent_detected_lamports: ata_rent,
        ata_rent_detected_sol: ata_rent_sol,

        trading_sol_calculated_lamports: sol_from_trade,
        trading_sol_calculated_sol: trading_sol,

        token_amount_sold: position.token_amount.unwrap_or(0),
        effective_exit_price_original: position.effective_exit_price.unwrap_or(0.0),
        effective_exit_price_corrected: corrected_effective_price,

        has_problem,
        problem_type,
        suggested_fix,
    })
}

async fn apply_ata_fixes(
    analyses: &[TransactionAnalysis]
) -> Result<FixResults, Box<dyn std::error::Error>> {
    println!("\n🔧 Applying ATA Fixes...");

    let mut positions = load_positions_from_file();
    let mut successful_fixes = 0;
    let mut failed_fixes = 0;
    let mut fix_summary = Vec::new();

    for analysis in analyses {
        if !analysis.has_problem {
            continue;
        }

        println!("🔧 Fixing {} ({})", analysis.position_symbol, analysis.problem_type);

        // Find the position to fix
        if let Some(position) = positions.iter_mut().find(|p| p.mint == analysis.position_mint) {
            match analysis.problem_type.as_str() {
                "ZERO_TRADING_SOL" => {
                    // Use a more conservative ATA rent estimate
                    let conservative_ata_rent = 0.00203928; // Standard ATA rent
                    let corrected_sol_received = (
                        analysis.total_sol_received_sol - conservative_ata_rent
                    ).max(0.0);

                    if corrected_sol_received > 0.0001 {
                        position.sol_received = Some(corrected_sol_received);

                        // Recalculate effective exit price
                        if let Some(token_amount) = position.token_amount {
                            if token_amount > 0 {
                                let token_ui = (token_amount as f64) / 1_000_000.0; // Assume 6 decimals
                                position.effective_exit_price = Some(
                                    corrected_sol_received / token_ui
                                );
                            }
                        }

                        successful_fixes += 1;
                        fix_summary.push(
                            format!(
                                "Fixed ZERO_TRADING_SOL for {}: SOL received corrected to {:.6}",
                                position.symbol,
                                corrected_sol_received
                            )
                        );
                    } else {
                        failed_fixes += 1;
                        fix_summary.push(
                            format!(
                                "Failed to fix {}: Corrected SOL amount too small ({:.6})",
                                position.symbol,
                                corrected_sol_received
                            )
                        );
                    }
                }

                "ATA_RENT_EXCEEDS_TOTAL" => {
                    // Use total SOL as trading SOL (no ATA rent deduction)
                    position.sol_received = Some(analysis.total_sol_received_sol);

                    if let Some(token_amount) = position.token_amount {
                        if token_amount > 0 {
                            let token_ui = (token_amount as f64) / 1_000_000.0;
                            position.effective_exit_price = Some(
                                analysis.total_sol_received_sol / token_ui
                            );
                        }
                    }

                    successful_fixes += 1;
                    fix_summary.push(
                        format!(
                            "Fixed ATA_RENT_EXCEEDS_TOTAL for {}: Used total SOL as trading SOL",
                            position.symbol
                        )
                    );
                }

                "SUSPICIOUS_SMALL_TRADING_SOL" => {
                    // Use 50% of total SOL as a conservative estimate
                    let conservative_trading_sol = analysis.total_sol_received_sol * 0.5;
                    position.sol_received = Some(conservative_trading_sol);

                    if let Some(token_amount) = position.token_amount {
                        if token_amount > 0 {
                            let token_ui = (token_amount as f64) / 1_000_000.0;
                            position.effective_exit_price = Some(
                                conservative_trading_sol / token_ui
                            );
                        }
                    }

                    successful_fixes += 1;
                    fix_summary.push(
                        format!(
                            "Fixed SUSPICIOUS_SMALL_TRADING_SOL for {}: Used conservative 50% estimate",
                            position.symbol
                        )
                    );
                }

                "MISSED_ATA_DETECTION" => {
                    // Subtract standard ATA rent from total
                    let ata_rent_standard = 0.00203928;
                    let corrected_trading_sol = (
                        analysis.total_sol_received_sol - ata_rent_standard
                    ).max(0.0);

                    if corrected_trading_sol > 0.0001 {
                        position.sol_received = Some(corrected_trading_sol);

                        if let Some(token_amount) = position.token_amount {
                            if token_amount > 0 {
                                let token_ui = (token_amount as f64) / 1_000_000.0;
                                position.effective_exit_price = Some(
                                    corrected_trading_sol / token_ui
                                );
                            }
                        }

                        successful_fixes += 1;
                        fix_summary.push(
                            format!(
                                "Fixed MISSED_ATA_DETECTION for {}: Deducted standard ATA rent",
                                position.symbol
                            )
                        );
                    } else {
                        failed_fixes += 1;
                    }
                }

                _ => {
                    failed_fixes += 1;
                    fix_summary.push(format!("Unknown problem type: {}", analysis.problem_type));
                }
            }
        } else {
            failed_fixes += 1;
            fix_summary.push(format!("Position not found: {}", analysis.position_symbol));
        }
    }

    // Save fixed positions
    if successful_fixes > 0 {
        save_positions_to_file(&positions);
        println!("💾 Saved {} fixed positions to positions.json", successful_fixes);
    }

    Ok(FixResults {
        total_analyzed: analyses.len(),
        problems_found: analyses
            .iter()
            .filter(|a| a.has_problem)
            .count(),
        successful_fixes,
        failed_fixes,
        summary: fix_summary,
    })
}

fn print_fix_results(results: &FixResults) {
    println!("\n📊 FIX RESULTS");
    println!("==============");
    println!("Total analyzed: {}", results.total_analyzed);
    println!("Problems found: {}", results.problems_found);
    println!("Successful fixes: {}", results.successful_fixes);
    println!("Failed fixes: {}", results.failed_fixes);

    println!("\nDetailed results:");
    for summary in &results.summary {
        println!("  {}", summary);
    }
}

fn print_recommendations(analyses: &[TransactionAnalysis]) {
    println!("\n💡 RECOMMENDATIONS");
    println!("==================");

    let problems_count = analyses
        .iter()
        .filter(|a| a.has_problem)
        .count();

    if problems_count == 0 {
        println!("✅ No ATA-related issues found in transactions");
        return;
    }

    println!("Based on analysis of {} problematic transactions:", problems_count);

    // Recommendation 1: Improve ATA detection
    println!("\n1. 🔧 IMPROVE ATA DETECTION LOGIC:");
    println!("   - Add multi-method validation for ATA rent detection");
    println!("   - Use conservative fallbacks when detection is uncertain");
    println!("   - Add bounds checking (ATA rent should not exceed total SOL)");

    // Recommendation 2: Add fallback mechanisms
    println!("\n2. 🛡️  ADD FALLBACK MECHANISMS:");
    println!("   - When ATA detection results in 0 trading SOL, use conservative estimates");
    println!("   - For 0.002 SOL trades, minimum trading SOL should be > 0.0001 SOL");
    println!("   - Add manual override capability for edge cases");

    // Recommendation 3: Enhanced validation
    println!("\n3. ✅ ENHANCED VALIDATION:");
    println!("   - Cross-validate ATA detection with transaction logs");
    println!("   - Add sanity checks based on trade size");
    println!("   - Implement progressive confidence scoring for ATA detection");

    // Recommendation 4: Better error handling
    println!("\n4. 🚨 BETTER ERROR HANDLING:");
    println!("   - Don't reject positions with zero trading SOL immediately");
    println!("   - Add recovery mechanisms for positions marked as failed");
    println!("   - Log detailed ATA detection reasoning for debugging");

    println!("\n📝 Next Steps:");
    println!("   1. Run this tool regularly to monitor ATA detection accuracy");
    println!("   2. Update wallet.rs with improved ATA detection logic");
    println!("   3. Add position recovery mechanisms for failed closes");
    println!("   4. Consider token-specific decimal handling improvements");
}
