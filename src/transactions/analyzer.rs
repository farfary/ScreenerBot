// transactions/analyzer.rs - Transaction analysis and swap detection
use super::types::*;
use crate::logger::{ log, LogTag };
use std::collections::HashMap;

/// Transaction analyzer for categorization and swap detection
pub struct TransactionAnalyzer {
    dex_programs: HashMap<String, &'static str>,
}

impl Default for TransactionAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionAnalyzer {
    /// Create a new transaction analyzer
    pub fn new() -> Self {
        let mut dex_programs = HashMap::new();

        // Add DEX program IDs
        for (program_id, dex_name) in DEX_PROGRAM_IDS.iter() {
            dex_programs.insert(program_id.to_string(), dex_name);
        }

        Self { dex_programs }
    }

    /// Analyze a transaction to determine its type and extract swap information
    pub fn analyze_transaction(&self, transaction: &TransactionResult) -> TransactionAnalysis {
        let mut analysis = TransactionAnalysis {
            transaction_type: TransactionType::Unknown,
            is_swap: false,
            is_airdrop: false,
            is_transfer: false,
            swap_info: None,
            program_interactions: Vec::new(),
            token_transfers: Vec::new(),
            sol_balance_change: 0,
        };

        // Extract program interactions
        analysis.program_interactions = self.extract_program_interactions(transaction);

        // Check for DEX interactions
        let dex_interactions = self.find_dex_interactions(&analysis.program_interactions);

        // Extract token transfers
        analysis.token_transfers = self.extract_token_transfers(transaction);

        // Calculate SOL balance change
        analysis.sol_balance_change = self.calculate_sol_balance_change(transaction);

        // Determine transaction type based on analysis
        if
            !dex_interactions.is_empty() &&
            self.has_bidirectional_transfers(&analysis.token_transfers)
        {
            analysis.transaction_type = TransactionType::Swap;
            analysis.is_swap = true;
            analysis.swap_info = self.extract_swap_info(
                transaction,
                &dex_interactions,
                &analysis.token_transfers
            );
        } else if self.is_likely_airdrop(&analysis.token_transfers, analysis.sol_balance_change) {
            analysis.transaction_type = TransactionType::Airdrop;
            analysis.is_airdrop = true;
        } else if !analysis.token_transfers.is_empty() || analysis.sol_balance_change != 0 {
            analysis.transaction_type = TransactionType::Transfer;
            analysis.is_transfer = true;
        }

        analysis
    }

    /// Extract program interactions from transaction instructions
    fn extract_program_interactions(
        &self,
        transaction: &TransactionResult
    ) -> Vec<ProgramInteraction> {
        let mut interactions = Vec::new();

        if let Some(message) = &transaction.transaction.message {
            for (i, instruction) in message.instructions.iter().enumerate() {
                if
                    let Some(program_key) = message.account_keys.get(
                        instruction.program_id_index as usize
                    )
                {
                    let program_id = program_key.clone();
                    let dex_name = self.get_dex_name(&program_id);

                    interactions.push(ProgramInteraction {
                        instruction_index: i,
                        program_id: program_id.clone(),
                        dex_name: dex_name.map(|s| s.to_string()),
                        is_known_dex: dex_name.is_some(),
                        data_length: instruction.data.len(),
                    });
                }
            }
        }

        interactions
    }

    /// Find DEX-related program interactions
    fn find_dex_interactions(
        &self,
        interactions: &[ProgramInteraction]
    ) -> Vec<&ProgramInteraction> {
        interactions
            .iter()
            .filter(|interaction| interaction.is_known_dex)
            .collect()
    }

    /// Extract token transfer information from transaction
    fn extract_token_transfers(&self, transaction: &TransactionResult) -> Vec<TokenTransfer> {
        let mut transfers = Vec::new();

        // Parse pre and post token balances
        if
            let (Some(pre_balances), Some(post_balances)) = (
                &transaction.meta.pre_token_balances,
                &transaction.meta.post_token_balances,
            )
        {
            // Group balances by account and mint
            let mut balance_changes: HashMap<(String, String), (u64, u64)> = HashMap::new();

            // Collect pre-balances
            for pre_balance in pre_balances {
                let key = (pre_balance.account_index.to_string(), pre_balance.mint.clone());
                balance_changes.entry(key).or_insert((0, 0)).0 = pre_balance.ui_token_amount.amount
                    .parse()
                    .unwrap_or(0);
            }

            // Collect post-balances
            for post_balance in post_balances {
                let key = (post_balance.account_index.to_string(), post_balance.mint.clone());
                balance_changes.entry(key).or_insert((0, 0)).1 = post_balance.ui_token_amount.amount
                    .parse()
                    .unwrap_or(0);
            }

            // Calculate changes and create transfers
            for ((account_index, mint), (pre_amount, post_amount)) in balance_changes {
                if pre_amount != post_amount {
                    let amount_change = if post_amount > pre_amount {
                        (post_amount - pre_amount) as i64
                    } else {
                        -((pre_amount - post_amount) as i64)
                    };

                    // Get decimals from post_balance if available
                    let decimals = post_balances
                        .iter()
                        .find(|b| b.account_index.to_string() == account_index && b.mint == mint)
                        .map(|b| b.ui_token_amount.decimals)
                        .unwrap_or(9); // Default to 9 decimals

                    transfers.push(TokenTransfer {
                        mint: mint.clone(),
                        account_index: account_index.parse().unwrap_or(0),
                        amount_change,
                        decimals,
                        is_incoming: amount_change > 0,
                        ui_amount: (amount_change as f64) / (10_f64).powi(decimals as i32),
                    });
                }
            }
        }

        transfers
    }

    /// Calculate SOL balance change from transaction
    fn calculate_sol_balance_change(&self, transaction: &TransactionResult) -> i64 {
        if
            let (Some(pre_balances), Some(post_balances)) = (
                &transaction.meta.pre_balances,
                &transaction.meta.post_balances,
            )
        {
            // Assume first account is the wallet (fee payer)
            if
                let (Some(&pre_balance), Some(&post_balance)) = (
                    pre_balances.get(0),
                    post_balances.get(0),
                )
            {
                return (post_balance as i64) - (pre_balance as i64);
            }
        }
        0
    }

    /// Check if transaction has bidirectional token transfers (indication of swap)
    fn has_bidirectional_transfers(&self, transfers: &[TokenTransfer]) -> bool {
        let has_incoming = transfers.iter().any(|t| t.is_incoming);
        let has_outgoing = transfers.iter().any(|t| !t.is_incoming);
        has_incoming && has_outgoing && transfers.len() >= 2
    }

    /// Determine if transaction is likely an airdrop
    fn is_likely_airdrop(&self, transfers: &[TokenTransfer], sol_change: i64) -> bool {
        // Airdrop characteristics:
        // 1. Only incoming token transfers
        // 2. Small or negative SOL change (transaction fee)
        // 3. No outgoing token transfers
        let only_incoming_tokens = transfers.iter().all(|t| t.is_incoming) && !transfers.is_empty();
        let minimal_sol_cost = sol_change <= 0 && sol_change.abs() < 10_000_000; // Less than 0.01 SOL

        only_incoming_tokens && minimal_sol_cost
    }

    /// Extract detailed swap information from transaction
    fn extract_swap_info(
        &self,
        transaction: &TransactionResult,
        dex_interactions: &[&ProgramInteraction],
        transfers: &[TokenTransfer]
    ) -> Option<SwapInfo> {
        if transfers.len() < 2 {
            return None;
        }

        // Find incoming and outgoing transfers
        let incoming: Vec<_> = transfers
            .iter()
            .filter(|t| t.is_incoming)
            .collect();
        let outgoing: Vec<_> = transfers
            .iter()
            .filter(|t| !t.is_incoming)
            .collect();

        if incoming.is_empty() || outgoing.is_empty() {
            return None;
        }

        // Get primary DEX used
        let primary_dex = dex_interactions
            .first()
            .and_then(|interaction| interaction.dex_name.clone())
            .unwrap_or_else(|| "Unknown".to_string());

        // Determine swap direction and tokens
        let (input_token, output_token, input_amount, output_amount) = if
            incoming.len() == 1 &&
            outgoing.len() == 1
        {
            let input = &outgoing[0];
            let output = &incoming[0];
            (
                input.mint.clone(),
                output.mint.clone(),
                input.amount_change.abs(),
                output.amount_change,
            )
        } else {
            // Handle complex swaps - use largest transfers
            let largest_outgoing = outgoing.iter().max_by_key(|t| t.amount_change.abs())?;
            let largest_incoming = incoming.iter().max_by_key(|t| t.amount_change)?;
            (
                largest_outgoing.mint.clone(),
                largest_incoming.mint.clone(),
                largest_outgoing.amount_change.abs(),
                largest_incoming.amount_change,
            )
        };

        Some(SwapInfo {
            dex_name: primary_dex,
            input_token,
            output_token,
            input_amount: input_amount as u64,
            output_amount: output_amount as u64,
            input_decimals: outgoing.first()?.decimals,
            output_decimals: incoming.first()?.decimals,
            effective_price: self.calculate_effective_price(
                input_amount as u64,
                output_amount as u64,
                outgoing.first()?.decimals,
                incoming.first()?.decimals
            ),
        })
    }

    /// Calculate effective price from swap amounts
    fn calculate_effective_price(
        &self,
        input_amount: u64,
        output_amount: u64,
        input_decimals: u8,
        output_decimals: u8
    ) -> f64 {
        if output_amount == 0 {
            return 0.0;
        }

        let input_ui = (input_amount as f64) / (10_f64).powi(input_decimals as i32);
        let output_ui = (output_amount as f64) / (10_f64).powi(output_decimals as i32);

        if output_ui > 0.0 {
            input_ui / output_ui
        } else {
            0.0
        }
    }

    /// Get DEX name from program ID
    pub fn get_dex_name(&self, program_id: &str) -> Option<&'static str> {
        self.dex_programs.get(program_id).copied()
    }

    /// Check if program ID is a known DEX
    pub fn is_known_dex(&self, program_id: &str) -> bool {
        self.dex_programs.contains_key(program_id)
    }

    /// Categorize multiple transactions
    pub fn categorize_transactions(
        &self,
        transactions: &[(SignatureInfo, TransactionResult)]
    ) -> TransactionCategorization {
        let mut categorization = TransactionCategorization {
            total_transactions: transactions.len(),
            swaps: Vec::new(),
            airdrops: Vec::new(),
            transfers: Vec::new(),
            unknown: Vec::new(),
            dex_usage: HashMap::new(),
        };

        for (sig_info, transaction) in transactions {
            let analysis = self.analyze_transaction(transaction);

            match analysis.transaction_type {
                TransactionType::Swap => {
                    if let Some(swap_info) = analysis.swap_info {
                        *categorization.dex_usage
                            .entry(swap_info.dex_name.clone())
                            .or_insert(0) += 1;
                        categorization.swaps.push((sig_info.clone(), swap_info));
                    }
                }
                TransactionType::Airdrop => {
                    categorization.airdrops.push(sig_info.clone());
                }
                TransactionType::Transfer => {
                    categorization.transfers.push(sig_info.clone());
                }
                TransactionType::Unknown => {
                    categorization.unknown.push(sig_info.clone());
                }
            }
        }

        categorization
    }

    /// Get transaction statistics
    pub fn get_transaction_stats(
        &self,
        transactions: &[(SignatureInfo, TransactionResult)]
    ) -> TransactionStats {
        let categorization = self.categorize_transactions(transactions);

        TransactionStats {
            total: categorization.total_transactions,
            swaps: categorization.swaps.len(),
            airdrops: categorization.airdrops.len(),
            transfers: categorization.transfers.len(),
            unknown: categorization.unknown.len(),
            swap_percentage: if categorization.total_transactions > 0 {
                ((categorization.swaps.len() as f64) / (categorization.total_transactions as f64)) *
                    100.0
            } else {
                0.0
            },
            most_used_dex: categorization.dex_usage
                .into_iter()
                .max_by_key(|(_, count)| *count)
                .map(|(dex, count)| format!("{} ({} swaps)", dex, count)),
        }
    }

    /// Enhanced swap detection with multiple criteria
    pub fn is_swap_transaction(&self, transaction: &TransactionResult) -> bool {
        let analysis = self.analyze_transaction(transaction);
        analysis.is_swap
    }

    /// Get detailed swap analysis for debugging
    pub fn debug_transaction_analysis(&self, transaction: &TransactionResult) -> String {
        let analysis = self.analyze_transaction(transaction);

        let mut debug_info = vec![
            format!("Transaction Type: {:?}", analysis.transaction_type),
            format!("Is Swap: {}", analysis.is_swap),
            format!("Is Airdrop: {}", analysis.is_airdrop),
            format!("Is Transfer: {}", analysis.is_transfer),
            format!("SOL Balance Change: {} lamports", analysis.sol_balance_change)
        ];

        if !analysis.program_interactions.is_empty() {
            debug_info.push("Program Interactions:".to_string());
            for interaction in &analysis.program_interactions {
                debug_info.push(
                    format!(
                        "  - {} ({})",
                        interaction.program_id,
                        interaction.dex_name.as_deref().unwrap_or("Unknown")
                    )
                );
            }
        }

        if !analysis.token_transfers.is_empty() {
            debug_info.push("Token Transfers:".to_string());
            for transfer in &analysis.token_transfers {
                debug_info.push(
                    format!(
                        "  - {}: {} {} ({})",
                        transfer.mint,
                        if transfer.is_incoming {
                            "+"
                        } else {
                            "-"
                        },
                        transfer.ui_amount,
                        if transfer.is_incoming {
                            "incoming"
                        } else {
                            "outgoing"
                        }
                    )
                );
            }
        }

        if let Some(swap_info) = &analysis.swap_info {
            debug_info.push(format!("Swap Info:"));
            debug_info.push(format!("  - DEX: {}", swap_info.dex_name));
            debug_info.push(format!("  - {} -> {}", swap_info.input_token, swap_info.output_token));
            debug_info.push(format!("  - Effective Price: {}", swap_info.effective_price));
        }

        debug_info.join("\n")
    }
}

/// Legacy compatibility functions
pub fn is_swap_transaction(transaction: &TransactionResult) -> bool {
    let analyzer = TransactionAnalyzer::new();
    analyzer.is_swap_transaction(transaction)
}

pub fn get_dex_name(program_id: &str) -> Option<&'static str> {
    let analyzer = TransactionAnalyzer::new();
    analyzer.get_dex_name(program_id)
}

pub fn is_known_dex(program_id: &str) -> bool {
    let analyzer = TransactionAnalyzer::new();
    analyzer.is_known_dex(program_id)
}

pub fn find_swap_program(
    message: &solana_transaction_status::UiMessage,
    program_interactions: &[ProgramInteraction]
) -> String {
    // Find first known DEX program
    for interaction in program_interactions {
        if interaction.is_known_dex {
            return interaction.program_id.clone();
        }
    }

    // Fallback to first program if no DEX found
    if let solana_transaction_status::UiMessage::Raw(raw_message) = message {
        if
            let Some(first_program) = raw_message.account_keys.get(
                raw_message.instructions
                    .get(0)
                    .map(|i| i.program_id_index as usize)
                    .unwrap_or(0)
            )
        {
            return first_program.clone();
        }
    }

    "Unknown".to_string()
}
