//! DEX-specific correction functions for transaction amount analysis.
//
// This module contains methods that apply exchange-specific adjustments
// to calculated swap amounts based on instruction patterns and program IDs.

use super::core::TransactionProcessor;
use super::helpers::*;

impl TransactionProcessor {
    /// Apply DEX-specific corrections based on instruction analysis and program patterns
    pub(super) fn apply_dex_specific_corrections(
        &self,
        router: &str,
        calculated_amount: u64,
        tx_data: &crate::rpc::TransactionDetails,
        balance_analysis: &crate::transactions::analyzer::balance::BalanceAnalysis,
        direction: &crate::transactions::analyzer::classify::SwapDirection,
    ) -> Option<u64> {
        match router {
            "jupiter" => self.apply_jupiter_corrections(
                calculated_amount,
                tx_data,
                balance_analysis,
                direction,
            ),
            "pumpfun" => self.apply_pumpfun_corrections(
                calculated_amount,
                tx_data,
                balance_analysis,
                direction,
            ),
            "raydium" => {
                self.apply_raydium_corrections(calculated_amount, tx_data, balance_analysis)
            }
            _ => None,
        }
    }

    /// Apply Jupiter-specific corrections based on instruction analysis
    fn apply_jupiter_corrections(
        &self,
        calculated_amount: u64,
        tx_data: &crate::rpc::TransactionDetails,
        _balance_analysis: &crate::transactions::analyzer::balance::BalanceAnalysis,
        direction: &crate::transactions::analyzer::classify::SwapDirection,
    ) -> Option<u64> {
        // Direction-aware correction policy for Jupiter:
        // - Buy (SOL -> Token): prefer authoritative deposit amount from wallet to WSOL ATA(s).
        // If detected, set calculated amount to that deposit (instruction truth). Do not add fee legs.
        // - Sell (Token -> SOL): do not adjust; output is derived from WSOL credits/unwrapped SOL and should be net of fees.
        match direction {
            crate::transactions::analyzer::classify::SwapDirection::SolToToken => {
                // Find the authoritative deposit (largest non-tip system transfer from wallet)
                if let Some(deposit_raw) = find_largest_system_transfer_from_wallet_excluding_tips(
                    tx_data,
                    &self.wallet_pubkey.to_string(),
                ) {
                    if deposit_raw > 0 {
                        // Only replace when it differs meaningfully (>0.05%) to avoid churn
                        let rel = if calculated_amount > 0 {
                            (((calculated_amount as i128) - (deposit_raw as i128)).abs() as f64)
                                / (calculated_amount as f64)
                        } else {
                            1.0
                        };
                        if rel > 0.0005 {
                            return Some(deposit_raw);
                        }
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Apply Raydium-specific corrections
    fn apply_raydium_corrections(
        &self,
        calculated_amount: u64,
        tx_data: &crate::rpc::TransactionDetails,
        _balance_analysis: &crate::transactions::analyzer::balance::BalanceAnalysis,
    ) -> Option<u64> {
        let raydium_program_ids = [
            "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // Raydium AMM
            "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK", // Raydium CPMM
        ];

        // Check if this transaction involves Raydium by analyzing instructions
        let has_raydium =
            if let Some(instructions) = tx_data.transaction.message.get("instructions") {
                if let Some(instructions_array) = instructions.as_array() {
                    instructions_array.iter().any(|ix| {
                        if let Some(program_id) = ix.get("programId").and_then(|p| p.as_str()) {
                            raydium_program_ids.contains(&program_id)
                        } else {
                            false
                        }
                    })
                } else {
                    false
                }
            } else {
                false
            };

        if !has_raydium {
            return None;
        }

        // Raydium corrections would go here
        // For now, no specific patterns identified
        None
    }

    /// Apply Pumpfun-specific corrections (placeholder: keep micro-adjustments strictly <0.5%)
    fn apply_pumpfun_corrections(
        &self,
        calculated_amount: u64,
        tx_data: &crate::rpc::TransactionDetails,
        balance_analysis: &crate::transactions::analyzer::balance::BalanceAnalysis,
        direction: &crate::transactions::analyzer::classify::SwapDirection,
    ) -> Option<u64> {
        // Detect PumpFun by presence of legacy or AMM program IDs among outer instructions
        let pumpfun_programs = [
            crate::constants::PUMP_FUN_LEGACY_PROGRAM_ID,
            crate::constants::PUMP_FUN_AMM_PROGRAM_ID,
        ];

        let has_pumpfun = if let Some(ixs) = tx_data
            .transaction
            .message
            .get("instructions")
            .and_then(|v| v.as_array())
        {
            ixs.iter().any(|ix| {
                ix.get("programId")
                    .and_then(|v| v.as_str())
                    .map(|pid| pumpfun_programs.contains(&pid))
                    .unwrap_or_default()
            })
        } else {
            false
        };
        if !has_pumpfun {
            return None;
        }

        // Direction-aware selection of instruction-truth candidates
        match direction {
            crate::transactions::analyzer::classify::SwapDirection::TokenToSol => {
                // SELL: prefer exact WSOL inner credits to wallet ATAs
                let candidate_sell_ui =
                    sum_inner_wsol_transfers_ui_to_wallet(tx_data, &self.wallet_pubkey.to_string());
                let candidate_sell_raw = (candidate_sell_ui * 1_000_000_000.0)
                    .round()
                    .clamp(0.0, u64::MAX as f64) as u64;
                if candidate_sell_raw > 0 {
                    return Some(candidate_sell_raw);
                }
                // Fallback: keep original
                None
            }
            crate::transactions::analyzer::classify::SwapDirection::SolToToken => {
                // BUY: prefer largest non-tip system transfer from wallet (authoritative deposit)
                if let Some(deposit_raw) = find_largest_system_transfer_from_wallet_excluding_tips(
                    tx_data,
                    &self.wallet_pubkey.to_string(),
                ) {
                    if deposit_raw > 0 {
                        return Some(deposit_raw);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Check if adjustment pattern is likely for Pumpfun
    fn is_likely_pumpfun_pattern(
        &self,
        calculated_amount: u64,
        adjusted_amount: u64,
        tx_data: &crate::rpc::TransactionDetails,
        balance_analysis: &crate::transactions::analyzer::balance::BalanceAnalysis,
    ) -> bool {
        // Look for Pumpfun-specific instruction patterns
        let pumpfun_program_id = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

        // Count Pumpfun instructions by analyzing transaction structure
        let pumpfun_instruction_count =
            if let Some(instructions) = tx_data.transaction.message.get("instructions") {
                if let Some(instructions_array) = instructions.as_array() {
                    instructions_array
                        .iter()
                        .filter(|ix| {
                            if let Some(program_id) = ix.get("programId").and_then(|p| p.as_str()) {
                                program_id == pumpfun_program_id
                            } else {
                                false
                            }
                        })
                        .count()
                } else {
                    0
                }
            } else {
                0
            };

        // Check for intermediary account patterns in balance changes
        let has_intermediary_pattern =
            balance_analysis
                .sol_changes
                .iter()
                .any(|(account, change)| {
                    // Look for accounts that aren't the main wallet but have SOL changes
                    change.change.abs() > 0.0 && !change.change.is_nan()
                });

        // Pumpfun typically has 1-2 instructions and intermediary accounts
        pumpfun_instruction_count >= 1 && pumpfun_instruction_count <= 3 && has_intermediary_pattern
    }
}
