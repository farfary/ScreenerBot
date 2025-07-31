use screenerbot::wallet::*;
use screenerbot::logger::*;
use screenerbot::global::*;
use screenerbot::utils::*;

use serde::{ Serialize, Deserialize };

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedAtaDetectionResult {
    pub ata_detected: bool,
    pub confidence_score: f64, // 0.0 to 1.0
    pub ata_rent_lamports: u64,
    pub trading_sol_lamports: u64,
    pub detection_methods: Vec<String>,
    pub fallback_applied: bool,
    pub reasoning: String,
}

#[derive(Debug, Clone)]
pub struct AtaDetectionMethod {
    pub name: String,
    pub detected: bool,
    pub confidence: f64,
    pub ata_amount: u64,
    pub reasoning: String,
}

/// Enhanced ATA detection with multiple validation methods and conservative fallbacks
pub fn enhanced_ata_detection(
    transaction: &TransactionDetails,
    wallet_address: &str,
    actual_output_change: u64,
    is_sell_transaction: bool,
    trade_size_sol: f64 // Expected trade size for validation
) -> EnhancedAtaDetectionResult {
    if !is_sell_transaction {
        return EnhancedAtaDetectionResult {
            ata_detected: false,
            confidence_score: 1.0,
            ata_rent_lamports: 0,
            trading_sol_lamports: actual_output_change,
            detection_methods: vec!["NOT_SELL_TRANSACTION".to_string()],
            fallback_applied: false,
            reasoning: "Not a sell transaction, no ATA detection needed".to_string(),
        };
    }

    const ATA_RENT_STANDARD: u64 = 2_039_280; // Standard ATA rent in lamports
    const ATA_RENT_TOLERANCE: u64 = 100_000; // Tolerance for variations

    let mut detection_methods = Vec::new();
    let mut total_confidence = 0.0;
    let mut detected_ata_amounts = Vec::new();

    // Method 1: Transaction log analysis (Highest confidence)
    let log_detection = detect_ata_in_transaction_logs(transaction);
    if log_detection.detected {
        detection_methods.push(log_detection.clone());
        total_confidence += log_detection.confidence * 0.4; // 40% weight
        detected_ata_amounts.push(log_detection.ata_amount);
    }

    // Method 2: Account balance change analysis (High confidence)
    let balance_detection = detect_ata_from_balance_changes(transaction);
    if balance_detection.detected {
        detection_methods.push(balance_detection.clone());
        total_confidence += balance_detection.confidence * 0.3; // 30% weight
        detected_ata_amounts.push(balance_detection.ata_amount);
    }

    // Method 3: Pattern analysis (Medium confidence)
    let pattern_detection = detect_ata_pattern_analysis(actual_output_change, trade_size_sol);
    if pattern_detection.detected {
        detection_methods.push(pattern_detection.clone());
        total_confidence += pattern_detection.confidence * 0.2; // 20% weight
        detected_ata_amounts.push(pattern_detection.ata_amount);
    }

    // Method 4: Statistical analysis (Lower confidence)
    let statistical_detection = detect_ata_statistical_analysis(actual_output_change);
    if statistical_detection.detected {
        detection_methods.push(statistical_detection.clone());
        total_confidence += statistical_detection.confidence * 0.1; // 10% weight
        detected_ata_amounts.push(statistical_detection.ata_amount);
    }

    // Determine final ATA detection result
    let ata_detected = total_confidence > 0.3; // Require 30% confidence threshold
    let consensus_ata_amount = calculate_consensus_ata_amount(&detected_ata_amounts);

    // Apply safety checks and fallbacks
    let (final_ata_amount, trading_sol, fallback_applied, final_reasoning) =
        apply_safety_checks_and_fallbacks(
            ata_detected,
            consensus_ata_amount,
            actual_output_change,
            trade_size_sol,
            total_confidence
        );

    let method_names: Vec<String> = detection_methods
        .iter()
        .map(|m| m.name.clone())
        .collect();

    EnhancedAtaDetectionResult {
        ata_detected,
        confidence_score: total_confidence.min(1.0),
        ata_rent_lamports: final_ata_amount,
        trading_sol_lamports: trading_sol,
        detection_methods: method_names,
        fallback_applied,
        reasoning: final_reasoning,
    }
}

fn detect_ata_in_transaction_logs(transaction: &TransactionDetails) -> AtaDetectionMethod {
    let mut detected = false;
    let mut reasoning = String::new();

    if let Some(meta) = &transaction.meta {
        if let Some(log_messages) = &meta.log_messages {
            for log in log_messages {
                // Check for SPL Token close account instructions
                if
                    log.contains("Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA") ||
                    log.contains("Program TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb")
                {
                    if
                        log.contains("Instruction: CloseAccount") ||
                        log.contains("close account") ||
                        log.contains("Close Account")
                    {
                        detected = true;
                        reasoning = "Found SPL Token CloseAccount instruction in logs".to_string();
                        break;
                    }
                }

                // Check for account closing patterns
                if
                    log.contains("closed") &&
                    log.contains("account") &&
                    (log.contains("token") || log.contains("Token"))
                {
                    detected = true;
                    reasoning = "Found account closure pattern in logs".to_string();
                    break;
                }
            }
        }
    }

    if !detected {
        reasoning = "No ATA closure indicators found in transaction logs".to_string();
    }

    AtaDetectionMethod {
        name: "TRANSACTION_LOGS".to_string(),
        detected,
        confidence: if detected {
            0.9
        } else {
            0.0
        },
        ata_amount: if detected {
            2_039_280
        } else {
            0
        },
        reasoning,
    }
}

fn detect_ata_from_balance_changes(transaction: &TransactionDetails) -> AtaDetectionMethod {
    let mut detected = false;
    let mut ata_amount = 0u64;
    let mut reasoning = String::new();

    if let Some(meta) = &transaction.meta {
        for (i, (pre_balance, post_balance)) in meta.pre_balances
            .iter()
            .zip(meta.post_balances.iter())
            .enumerate() {
            // Skip the main wallet account (usually first)
            if i == 0 {
                continue;
            }

            // Look for accounts that went from positive to zero (closed)
            if *pre_balance > 0 && *post_balance == 0 {
                let closed_amount = *pre_balance;

                // Check if amount matches ATA rent pattern
                if closed_amount >= 2_000_000 && closed_amount <= 2_100_000 {
                    detected = true;
                    ata_amount = closed_amount;
                    reasoning = format!(
                        "Account {} closed with {} lamports (matches ATA rent)",
                        i,
                        closed_amount
                    );
                    break;
                }
            }

            // Also check for significant balance decreases
            if *post_balance < *pre_balance {
                let decrease = *pre_balance - *post_balance;
                if decrease >= 2_000_000 && decrease <= 2_100_000 {
                    detected = true;
                    ata_amount = decrease;
                    reasoning = format!(
                        "Account {} decreased by {} lamports (matches ATA rent)",
                        i,
                        decrease
                    );
                    break;
                }
            }
        }
    }

    if !detected {
        reasoning = "No ATA-sized balance changes detected".to_string();
    }

    AtaDetectionMethod {
        name: "BALANCE_CHANGES".to_string(),
        detected,
        confidence: if detected {
            0.8
        } else {
            0.0
        },
        ata_amount,
        reasoning,
    }
}

fn detect_ata_pattern_analysis(
    actual_output_change: u64,
    trade_size_sol: f64
) -> AtaDetectionMethod {
    let total_sol = lamports_to_sol(actual_output_change);
    let ata_rent_sol = lamports_to_sol(2_039_280);

    // Pattern 1: Total SOL is suspiciously close to trade size + ATA rent
    let expected_total = trade_size_sol + ata_rent_sol;
    let difference = (total_sol - expected_total).abs();

    if difference < 0.0005 {
        // Within 0.5 milliSOL
        return AtaDetectionMethod {
            name: "PATTERN_TRADE_PLUS_ATA".to_string(),
            detected: true,
            confidence: 0.7,
            ata_amount: 2_039_280,
            reasoning: format!(
                "Total SOL ({:.6}) matches trade size + ATA rent pattern",
                total_sol
            ),
        };
    }

    // Pattern 2: Total SOL is suspiciously close to just ATA rent (zero trade value)
    if (total_sol - ata_rent_sol).abs() < 0.0005 {
        return AtaDetectionMethod {
            name: "PATTERN_ONLY_ATA".to_string(),
            detected: true,
            confidence: 0.5, // Lower confidence since this might be a failed trade
            ata_amount: 2_039_280,
            reasoning: format!("Total SOL ({:.6}) approximately equals ATA rent only", total_sol),
        };
    }

    // Pattern 3: Total SOL significantly exceeds expected trade size
    if total_sol > trade_size_sol * 2.0 && total_sol > 0.003 {
        return AtaDetectionMethod {
            name: "PATTERN_EXCESS_SOL".to_string(),
            detected: true,
            confidence: 0.6,
            ata_amount: 2_039_280,
            reasoning: format!(
                "Total SOL ({:.6}) significantly exceeds trade size ({:.6}), likely includes ATA",
                total_sol,
                trade_size_sol
            ),
        };
    }

    AtaDetectionMethod {
        name: "PATTERN_ANALYSIS".to_string(),
        detected: false,
        confidence: 0.0,
        ata_amount: 0,
        reasoning: "No suspicious patterns detected".to_string(),
    }
}

fn detect_ata_statistical_analysis(actual_output_change: u64) -> AtaDetectionMethod {
    let total_sol = lamports_to_sol(actual_output_change);

    // Statistical check: Is this amount likely to include ATA rent?
    // Based on common patterns observed in trading

    // Check if amount has the "signature" of ATA inclusion
    let ata_rent_sol = lamports_to_sol(2_039_280);
    let remainder = total_sol % ata_rent_sol;

    // If remainder is very small or very close to ATA rent, might include ATA
    let likelihood = if remainder < 0.0002 || remainder > ata_rent_sol - 0.0002 {
        0.4 // 40% confidence
    } else if total_sol > 0.002 && total_sol < 0.006 {
        // In the range where ATA might be included
        0.3 // 30% confidence
    } else {
        0.0
    };

    AtaDetectionMethod {
        name: "STATISTICAL_ANALYSIS".to_string(),
        detected: likelihood > 0.25,
        confidence: likelihood,
        ata_amount: if likelihood > 0.25 {
            2_039_280
        } else {
            0
        },
        reasoning: format!("Statistical likelihood of ATA inclusion: {:.1}%", likelihood * 100.0),
    }
}

fn calculate_consensus_ata_amount(detected_amounts: &[u64]) -> u64 {
    if detected_amounts.is_empty() {
        return 0;
    }

    // Use median of detected amounts for robustness
    let mut amounts = detected_amounts.to_vec();
    amounts.sort();

    let mid = amounts.len() / 2;
    if amounts.len() % 2 == 0 && amounts.len() > 1 {
        (amounts[mid - 1] + amounts[mid]) / 2
    } else {
        amounts[mid]
    }
}

fn apply_safety_checks_and_fallbacks(
    ata_detected: bool,
    ata_amount: u64,
    total_sol_lamports: u64,
    trade_size_sol: f64,
    confidence: f64
) -> (u64, u64, bool, String) {
    let total_sol = lamports_to_sol(total_sol_lamports);
    let ata_sol = lamports_to_sol(ata_amount);
    let mut fallback_applied = false;
    let mut reasoning = String::new();

    // Safety Check 1: ATA rent cannot exceed total SOL
    if ata_detected && ata_amount >= total_sol_lamports {
        fallback_applied = true;
        reasoning = format!(
            "FALLBACK: ATA rent ({:.6} SOL) exceeds total SOL ({:.6} SOL). Using conservative estimate.",
            ata_sol,
            total_sol
        );

        // Use 50% of total as ATA rent estimate (conservative)
        let conservative_ata = total_sol_lamports / 2;
        let trading_sol = total_sol_lamports - conservative_ata;
        return (conservative_ata, trading_sol, fallback_applied, reasoning);
    }

    // Safety Check 2: For small trades (0.002 SOL), ensure minimum trading SOL
    if ata_detected && trade_size_sol <= 0.003 {
        let trading_sol_lamports = total_sol_lamports.saturating_sub(ata_amount);
        let trading_sol = lamports_to_sol(trading_sol_lamports);

        // Minimum trading SOL should be at least 10% of trade size
        let min_trading_sol = trade_size_sol * 0.1;

        if trading_sol < min_trading_sol {
            fallback_applied = true;
            reasoning = format!(
                "FALLBACK: Trading SOL ({:.6}) below minimum ({:.6}) for trade size ({:.6}). Using conservative estimate.",
                trading_sol,
                min_trading_sol,
                trade_size_sol
            );

            // Reduce ATA rent estimate to ensure minimum trading SOL
            let required_trading_lamports = sol_to_lamports(min_trading_sol);
            let adjusted_ata = total_sol_lamports.saturating_sub(required_trading_lamports);
            return (adjusted_ata, required_trading_lamports, fallback_applied, reasoning);
        }
    }

    // Safety Check 3: Low confidence detection - use conservative approach
    if ata_detected && confidence < 0.5 {
        fallback_applied = true;
        reasoning = format!(
            "FALLBACK: Low confidence ({:.1}%) in ATA detection. Using conservative estimate.",
            confidence * 100.0
        );

        // Use 70% of standard ATA rent (conservative)
        let conservative_ata = ((2_039_280 as f64) * 0.7) as u64;
        let trading_sol = total_sol_lamports.saturating_sub(conservative_ata);
        return (conservative_ata, trading_sol, fallback_applied, reasoning);
    }

    // Normal case: use detected values
    if ata_detected {
        let trading_sol = total_sol_lamports.saturating_sub(ata_amount);
        reasoning = format!(
            "ATA detected with {:.1}% confidence. ATA: {:.6} SOL, Trading: {:.6} SOL",
            confidence * 100.0,
            ata_sol,
            lamports_to_sol(trading_sol)
        );
        (ata_amount, trading_sol, fallback_applied, reasoning)
    } else {
        reasoning = "No ATA detected. Using total SOL as trading SOL.".to_string();
        (0, total_sol_lamports, fallback_applied, reasoning)
    }
}

/// Convert SOL to lamports
pub fn sol_to_lamports(sol: f64) -> u64 {
    (sol * 1_000_000_000.0) as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ata_detection_patterns() {
        // Test case 1: Clear ATA inclusion (0.002 trade + 0.002 ATA ≈ 0.004 total)
        let result = detect_ata_pattern_analysis(sol_to_lamports(0.004), 0.002);
        assert!(result.detected);
        assert!(result.confidence > 0.6);

        // Test case 2: No ATA (small trade amount)
        let result = detect_ata_pattern_analysis(sol_to_lamports(0.0015), 0.002);
        assert!(!result.detected);

        // Test case 3: Only ATA rent (failed trade)
        let result = detect_ata_pattern_analysis(sol_to_lamports(0.00203928), 0.002);
        assert!(result.detected);
        assert_eq!(result.name, "PATTERN_ONLY_ATA");
    }

    #[test]
    fn test_safety_fallbacks() {
        // Test case 1: ATA exceeds total (impossible)
        let (ata, trading, fallback, _) = apply_safety_checks_and_fallbacks(
            true,
            3_000_000,
            2_000_000,
            0.002,
            0.8
        );
        assert!(fallback);
        assert!(trading > 0);

        // Test case 2: Trading SOL too small
        let (ata, trading, fallback, _) = apply_safety_checks_and_fallbacks(
            true,
            2_039_280,
            2_100_000,
            0.002,
            0.8
        );
        assert!(trading >= sol_to_lamports(0.0002)); // At least 10% of 0.002
    }
}
