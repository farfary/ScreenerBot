// transactions/analyzer.rs - Transaction analysis and swap detection
use super::types::*;
use crate::logger::{ log, LogTag };
use crate::discovery::get_single_token_info;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Notify;
use colored::Colorize;

/// Transaction analyzer for categorization and swap detection
pub struct TransactionAnalyzer {
    dex_programs: HashMap<String, &'static str>,
    use_token_db: bool,
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
            dex_programs.insert(program_id.to_string(), *dex_name);
        }

        Self {
            dex_programs,
            use_token_db: true, // Enable token database usage by default
        }
    }

    /// Create analyzer with token database usage control
    pub fn new_with_db_option(use_token_db: bool) -> Self {
        let mut analyzer = Self::new();
        analyzer.use_token_db = use_token_db;
        analyzer
    }

    /// Enrich token transfers with database information
    fn enrich_token_transfers(&self, transfers: &mut Vec<TokenTransfer>) {
        if !self.use_token_db {
            return;
        }

        for transfer in transfers.iter_mut() {
            if let Some(token) = crate::global::get_token_from_db(&transfer.mint) {
                // Update transfer with database information if available
                transfer.decimals = token.decimals;

                // Log enrichment for debugging
                log(
                    LogTag::System,
                    "ENRICH",
                    &format!("Enriched transfer for {} ({})", token.symbol, token.mint)
                        .dimmed()
                        .to_string()
                );
            }
        }
    }

    /// Fetch and cache unknown tokens encountered in transfers
    async fn fetch_and_cache_unknown_tokens(&self, transfers: &[TokenTransfer]) -> Vec<String> {
        if !self.use_token_db {
            return Vec::new();
        }

        let mut unknown_mints = Vec::new();
        let mut newly_cached_mints = Vec::new();

        // Identify unknown tokens
        for transfer in transfers {
            if crate::global::get_token_from_db(&transfer.mint).is_none() {
                unknown_mints.push(transfer.mint.clone());
            }
        }

        if unknown_mints.is_empty() {
            return newly_cached_mints;
        }

        log(
            LogTag::System,
            "FETCH",
            &format!(
                "Found {} unknown tokens in transaction, fetching info...",
                unknown_mints.len()
            )
                .bright_yellow()
                .to_string()
        );

        // Create shutdown signal for API calls
        let shutdown = Arc::new(Notify::new());

        // Fetch information for each unknown token
        for mint in unknown_mints {
            match get_single_token_info(&mint, shutdown.clone()).await {
                Ok(Some(token)) => {
                    log(
                        LogTag::System,
                        "CACHE",
                        &format!(
                            "Successfully fetched and cached token: {} ({})",
                            token.symbol,
                            token.mint
                        )
                            .bright_green()
                            .to_string()
                    );
                    newly_cached_mints.push(mint);
                }
                Ok(None) => {
                    log(
                        LogTag::System,
                        "WARN",
                        &format!("Token not found on DexScreener: {}", mint)
                            .bright_yellow()
                            .to_string()
                    );
                }
                Err(e) => {
                    log(
                        LogTag::System,
                        "ERROR",
                        &format!("Failed to fetch token {}: {}", mint, e).bright_red().to_string()
                    );
                }
            }

            // Small delay to respect rate limits
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }

        newly_cached_mints
    }

    /// Get token symbol from database for better logging
    fn get_token_symbol(&self, mint: &str) -> String {
        if self.use_token_db {
            if let Some(token) = crate::global::get_token_from_db(mint) {
                return token.symbol;
            }
        }
        format!("{}...{}", &mint[..4], &mint[mint.len() - 4..])
    }

    /// Analyze a transaction to determine its type and extract swap information
    pub fn analyze_transaction(&self, transaction: &TransactionResult) -> TransactionAnalysis {
        let mut analysis = TransactionAnalysis {
            signature: transaction.transaction.signatures.first().cloned().unwrap_or_default(),
            block_time: transaction.block_time,
            slot: transaction.slot,
            is_success: transaction.meta.as_ref().map_or(false, |m| m.err.is_none()),
            fee_sol: transaction.meta.as_ref().map_or(0.0, |m| (m.fee as f64) / 1_000_000_000.0),
            transaction_type: TransactionType::Unknown,
            is_swap: false,
            is_airdrop: false,
            is_transfer: false,
            swap_info: None,
            token_transfers: Vec::new(),
            sol_balance_change: 0,
            contains_swaps: false,
            swaps: Vec::new(),
            token_changes: Vec::new(),
            involves_target_token: false,
            program_interactions: Vec::new(),
        };

        // Extract program interactions
        analysis.program_interactions = self.extract_program_interactions(transaction);

        // Check for DEX interactions
        let dex_interactions = self.find_dex_interactions(&analysis.program_interactions);

        // Extract token transfers
        analysis.token_transfers = self.extract_token_transfers(transaction);

        // Enrich token transfers with database information
        self.enrich_token_transfers(&mut analysis.token_transfers);

        // Calculate SOL balance change
        analysis.sol_balance_change = self.calculate_sol_balance_change(transaction);

        // Improved swap detection logic
        if
            self.is_valid_swap(
                &dex_interactions,
                &analysis.token_transfers,
                analysis.sol_balance_change
            )
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

    /// Enhanced analyze transaction that fetches unknown tokens and re-evaluates swap detection
    pub async fn analyze_transaction_with_token_fetch(
        &self,
        transaction: &TransactionResult
    ) -> TransactionAnalysis {
        // First, do the basic analysis
        let mut analysis = self.analyze_transaction(transaction);

        // If this doesn't look like a swap but has DEX interactions and token transfers,
        // try fetching unknown tokens and re-evaluate
        if
            !analysis.is_swap &&
            !analysis.program_interactions.is_empty() &&
            !analysis.token_transfers.is_empty()
        {
            let dex_interactions = self.find_dex_interactions(&analysis.program_interactions);

            if !dex_interactions.is_empty() {
                // Fetch and cache unknown tokens
                let newly_cached = self.fetch_and_cache_unknown_tokens(
                    &analysis.token_transfers
                ).await;

                if !newly_cached.is_empty() {
                    log(
                        LogTag::System,
                        "REEVAL",
                        &format!(
                            "Re-evaluating transaction after caching {} new tokens",
                            newly_cached.len()
                        )
                            .bright_cyan()
                            .to_string()
                    );

                    // Re-extract token transfers to get updated information
                    analysis.token_transfers = self.extract_token_transfers(transaction);

                    // Re-enrich with the newly cached token information
                    self.enrich_token_transfers(&mut analysis.token_transfers);

                    // Re-evaluate swap detection with the new token information
                    if
                        self.is_valid_swap(
                            &dex_interactions,
                            &analysis.token_transfers,
                            analysis.sol_balance_change
                        )
                    {
                        analysis.transaction_type = TransactionType::Swap;
                        analysis.is_swap = true;
                        analysis.swap_info = self.extract_swap_info(
                            transaction,
                            &dex_interactions,
                            &analysis.token_transfers
                        );

                        log(
                            LogTag::System,
                            "SWAP",
                            &format!(
                                "✅ Detected swap after token fetch for transaction: {}",
                                analysis.signature[..8].to_string()
                            )
                                .bright_green()
                                .to_string()
                        );
                    }
                }
            }
        }

        analysis
    }

    /// Extract program interactions from transaction instructions
    fn extract_program_interactions(
        &self,
        transaction: &TransactionResult
    ) -> Vec<ProgramInteraction> {
        let mut interactions = Vec::new();

        let message = &transaction.transaction.message;
        for (i, instruction) in message.instructions.iter().enumerate() {
            if let Some(program_id_index) = instruction.program_id_index {
                if let Some(program_key) = message.account_keys.get(program_id_index as usize) {
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
    fn find_dex_interactions<'a>(
        &self,
        interactions: &'a [ProgramInteraction]
    ) -> Vec<&'a ProgramInteraction> {
        interactions
            .iter()
            .filter(|interaction| interaction.is_known_dex)
            .collect()
    }

    /// Extract token transfer information from transaction
    fn extract_token_transfers(&self, transaction: &TransactionResult) -> Vec<TokenTransfer> {
        let mut transfers = Vec::new();

        // Parse pre and post token balances
        if let Some(meta) = &transaction.meta {
            if
                let (Some(pre_balances), Some(post_balances)) = (
                    &meta.pre_token_balances,
                    &meta.post_token_balances,
                )
            {
                // Group balances by account and mint
                let mut balance_changes: HashMap<(String, String), (u64, u64)> = HashMap::new();

                // Collect pre-balances
                for pre_balance in pre_balances {
                    let key = (pre_balance.account_index.to_string(), pre_balance.mint.clone());
                    balance_changes.entry(key).or_insert((0, 0)).0 =
                        pre_balance.ui_token_amount.amount.parse().unwrap_or(0);
                }

                // Collect post-balances
                for post_balance in post_balances {
                    let key = (post_balance.account_index.to_string(), post_balance.mint.clone());
                    balance_changes.entry(key).or_insert((0, 0)).1 =
                        post_balance.ui_token_amount.amount.parse().unwrap_or(0);
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
                            .find(
                                |b| b.account_index.to_string() == account_index && b.mint == mint
                            )
                            .map(|b| b.ui_token_amount.decimals)
                            .unwrap_or(9); // Default to 9 decimals

                        transfers.push(TokenTransfer {
                            mint: mint.clone(),
                            from: None,
                            to: None,
                            amount: amount_change.abs().to_string(),
                            account_index: account_index.parse().unwrap_or(0),
                            amount_change: amount_change as f64,
                            decimals,
                            is_incoming: amount_change > 0,
                            ui_amount: Some(
                                (amount_change as f64) / (10_f64).powi(decimals as i32)
                            ),
                        });
                    }
                }
            }
        }

        transfers
    }

    /// Calculate SOL balance change from transaction
    fn calculate_sol_balance_change(&self, transaction: &TransactionResult) -> i64 {
        if let Some(meta) = &transaction.meta {
            let (pre_balances, post_balances) = (&meta.pre_balances, &meta.post_balances);
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

    /// Enhanced swap validation - checks for DEX interaction, meaningful SOL change, and token exchange
    fn is_valid_swap(
        &self,
        dex_interactions: &[&ProgramInteraction],
        transfers: &[TokenTransfer],
        sol_change: i64
    ) -> bool {
        // Minimum SOL threshold (0.000001 SOL = 1,000 lamports)
        const MIN_SWAP_SOL_THRESHOLD: i64 = 1_000;

        // Must have DEX program interaction
        if dex_interactions.is_empty() {
            return false;
        }

        // Must have meaningful SOL balance change (excluding just fees)
        let abs_sol_change = sol_change.abs();
        if abs_sol_change < MIN_SWAP_SOL_THRESHOLD {
            return false;
        }

        // Must have bidirectional token transfers
        if !self.has_bidirectional_transfers(transfers) {
            return false;
        }

        // Check for actual SOL ↔ token exchange patterns
        self.has_sol_token_exchange(transfers, sol_change)
    }

    /// Check if transaction involves SOL ↔ token exchange (essential for swaps)
    fn has_sol_token_exchange(&self, transfers: &[TokenTransfer], sol_change: i64) -> bool {
        const WSOL_MINT: &str = "So11111111111111111111111111111111111111112";

        // Check for wrapped SOL (WSOL) transfers
        let has_wsol_transfer = transfers.iter().any(|t| t.mint == WSOL_MINT);

        // Check for significant SOL balance change (indicating SOL side of swap)
        let significant_sol_change = sol_change.abs() >= 1_000; // > 0.000001 SOL

        // Valid swap patterns:
        // 1. WSOL transfers + other token transfers (wrapped SOL swaps)
        // 2. Significant SOL balance change + token transfers (native SOL swaps)
        // 3. Both WSOL and significant SOL change (complex swaps)

        if has_wsol_transfer {
            // WSOL transfer pattern - should have other non-WSOL tokens
            let non_wsol_tokens = transfers.iter().any(|t| t.mint != WSOL_MINT);
            non_wsol_tokens
        } else if significant_sol_change {
            // Native SOL pattern - should have token transfers
            !transfers.is_empty()
        } else {
            // No clear SOL involvement - likely not a SOL-token swap
            false
        }
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
            let largest_outgoing = outgoing
                .iter()
                .max_by(|a, b|
                    a.amount_change
                        .abs()
                        .partial_cmp(&b.amount_change.abs())
                        .unwrap_or(std::cmp::Ordering::Equal)
                )?;
            let largest_incoming = incoming
                .iter()
                .max_by(|a, b|
                    a.amount_change
                        .partial_cmp(&b.amount_change)
                        .unwrap_or(std::cmp::Ordering::Equal)
                )?;
            (
                largest_outgoing.mint.clone(),
                largest_incoming.mint.clone(),
                largest_outgoing.amount_change.abs(),
                largest_incoming.amount_change,
            )
        };

        Some(SwapInfo {
            dex_name: primary_dex.clone(),
            program_id: "unknown".to_string(), // TODO: extract from dex_interactions
            input_mint: input_token.clone(),
            output_mint: output_token.clone(),
            input_amount: input_amount.to_string(),
            output_amount: output_amount.to_string(),
            input_decimals: outgoing.first()?.decimals,
            output_decimals: incoming.first()?.decimals,
            swap_type: SwapType::SwapAtoB, // TODO: determine actual swap type
            input_token: input_token.clone(),
            output_token: output_token.clone(),
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
            success_rate: 0.0, // Will be calculated later
            dex_usage: HashMap::new(),
        };

        for (sig_info, transaction) in transactions {
            let analysis = self.analyze_transaction(transaction);

            match analysis.transaction_type {
                TransactionType::Swap => {
                    if let Some(_swap_info) = analysis.swap_info {
                        categorization.swaps.push(analysis.signature.clone());
                    }
                }
                TransactionType::Airdrop => {
                    categorization.airdrops.push(analysis.signature.clone());
                }
                TransactionType::Transfer => {
                    categorization.transfers.push(analysis.signature.clone());
                }
                | TransactionType::StakeUnstake
                | TransactionType::ProgramDeploy
                | TransactionType::AccountCreation
                | TransactionType::Unknown => {
                    categorization.unknown.push(analysis.signature.clone());
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
            total_processed: categorization.total_transactions,
            successful: categorization.total_transactions, // Assume all are successful for now
            failed: 0,
            swaps_detected: categorization.swaps.len(),
            average_processing_time_ms: 0.0, // TODO: implement timing
        }
    }

    /// Enhanced swap detection with multiple criteria
    pub fn is_swap_transaction(&self, transaction: &TransactionResult) -> bool {
        let analysis = self.analyze_transaction(transaction);
        analysis.is_swap
    }

    /// Advanced swap detection using multiple heuristics
    pub fn detect_swaps_advanced(&self, transaction: &TransactionResult) -> Vec<SwapTransaction> {
        let mut detected_swaps = Vec::new();

        // Method 1: Traditional DEX + bidirectional token analysis
        if self.is_swap_transaction(transaction) {
            if let Some(swap_info) = self.analyze_transaction(transaction).swap_info {
                detected_swaps.push(self.create_swap_transaction(transaction, &swap_info));
            }
        }

        // Method 2: Log message analysis for additional patterns
        let log_based_swaps = self.detect_swaps_from_logs(transaction);
        detected_swaps.extend(log_based_swaps);

        // Method 3: Inner instruction analysis for complex swaps
        let inner_instruction_swaps = self.detect_swaps_from_inner_instructions(transaction);
        detected_swaps.extend(inner_instruction_swaps);

        // Remove duplicates based on signature
        detected_swaps.sort_by(|a, b| a.signature.cmp(&b.signature));
        detected_swaps.dedup_by(|a, b| a.signature == b.signature);

        detected_swaps
    }

    /// Enhanced swap detection that fetches unknown tokens and re-evaluates
    pub async fn detect_swaps_with_token_fetch(
        &self,
        transaction: &TransactionResult
    ) -> Vec<SwapTransaction> {
        let mut detected_swaps = Vec::new();

        // Method 1: Enhanced analysis with token fetching
        let analysis = self.analyze_transaction_with_token_fetch(transaction).await;
        if analysis.is_swap {
            if let Some(swap_info) = analysis.swap_info {
                detected_swaps.push(self.create_swap_transaction(transaction, &swap_info));
            }
        }

        // Method 2: Log message analysis for additional patterns
        let log_based_swaps = self.detect_swaps_from_logs(transaction);
        detected_swaps.extend(log_based_swaps);

        // Method 3: Inner instruction analysis for complex swaps
        let inner_instruction_swaps = self.detect_swaps_from_inner_instructions(transaction);
        detected_swaps.extend(inner_instruction_swaps);

        // Remove duplicates based on signature
        detected_swaps.sort_by(|a, b| a.signature.cmp(&b.signature));
        detected_swaps.dedup_by(|a, b| a.signature == b.signature);

        detected_swaps
    }

    /// Detect swaps by analyzing transaction log messages
    fn detect_swaps_from_logs(&self, transaction: &TransactionResult) -> Vec<SwapTransaction> {
        let mut swaps = Vec::new();

        if let Some(meta) = &transaction.meta {
            if let Some(log_messages) = &meta.log_messages {
                // Look for common swap log patterns
                let swap_patterns = [
                    "Program log: Instruction: Swap",
                    "Program log: SwapEvent",
                    "Swap completed",
                    "Token swap:",
                    "swapped",
                ];

                for log in log_messages {
                    if swap_patterns.iter().any(|pattern| log.contains(pattern)) {
                        // Extract swap information from logs if possible
                        if let Some(swap) = self.parse_swap_from_log(transaction, log) {
                            swaps.push(swap);
                        }
                    }
                }
            }
        }

        swaps
    }

    /// Detect swaps from inner instructions (for complex routing)
    fn detect_swaps_from_inner_instructions(
        &self,
        transaction: &TransactionResult
    ) -> Vec<SwapTransaction> {
        let mut swaps = Vec::new();

        if let Some(meta) = &transaction.meta {
            if let Some(inner_instructions) = &meta.inner_instructions {
                for inner_group in inner_instructions {
                    // Analyze each inner instruction group for swap patterns
                    let dex_instructions: Vec<_> = inner_group.instructions
                        .iter()
                        .filter(|inst| {
                            if let Some(program_id_index) = inst.program_id_index {
                                if
                                    let Some(program_key) =
                                        transaction.transaction.message.account_keys.get(
                                            program_id_index as usize
                                        )
                                {
                                    return self.is_known_dex(program_key);
                                }
                            }
                            false
                        })
                        .collect();

                    if !dex_instructions.is_empty() {
                        // This inner instruction group contains DEX operations
                        if
                            let Some(swap) = self.analyze_inner_instruction_swap(
                                transaction,
                                &inner_group
                            )
                        {
                            swaps.push(swap);
                        }
                    }
                }
            }
        }

        swaps
    }

    /// Parse swap information from a log message
    fn parse_swap_from_log(
        &self,
        transaction: &TransactionResult,
        log_message: &str
    ) -> Option<SwapTransaction> {
        // Simple log parsing - can be enhanced with more sophisticated regex patterns
        if log_message.contains("Swap") {
            let signature = transaction.transaction.signatures.first()?.clone();

            // Create a basic swap transaction from log analysis
            Some(SwapTransaction {
                signature,
                block_time: transaction.block_time,
                slot: transaction.slot,
                is_success: transaction.meta.as_ref().map_or(false, |m| m.err.is_none()),
                fee_sol: transaction.meta
                    .as_ref()
                    .map_or(0.0, |m| (m.fee as f64) / 1_000_000_000.0),
                swap_type: SwapType::Unknown,
                input_token: SwapTokenInfo {
                    mint: "Unknown".to_string(),
                    symbol: None,
                    amount_raw: "0".to_string(),
                    amount_ui: 0.0,
                    decimals: 9,
                },
                output_token: SwapTokenInfo {
                    mint: "Unknown".to_string(),
                    symbol: None,
                    amount_raw: "0".to_string(),
                    amount_ui: 0.0,
                    decimals: 9,
                },
                program_id: "Unknown".to_string(),
                dex_name: Some("Log-detected".to_string()),
                log_messages: vec![log_message.to_string()],
            })
        } else {
            None
        }
    }

    /// Analyze inner instructions for swap patterns
    fn analyze_inner_instruction_swap(
        &self,
        transaction: &TransactionResult,
        inner_group: &InnerInstruction
    ) -> Option<SwapTransaction> {
        let signature = transaction.transaction.signatures.first()?.clone();

        // Find DEX program in inner instructions
        let dex_program = inner_group.instructions.iter().find_map(|inst| {
            if let Some(program_id_index) = inst.program_id_index {
                if
                    let Some(program_key) = transaction.transaction.message.account_keys.get(
                        program_id_index as usize
                    )
                {
                    if self.is_known_dex(program_key) {
                        return Some(program_key.clone());
                    }
                }
            }
            None
        })?;

        let dex_name = self.get_dex_name(&dex_program);

        Some(SwapTransaction {
            signature,
            block_time: transaction.block_time,
            slot: transaction.slot,
            is_success: transaction.meta.as_ref().map_or(false, |m| m.err.is_none()),
            fee_sol: transaction.meta.as_ref().map_or(0.0, |m| (m.fee as f64) / 1_000_000_000.0),
            swap_type: SwapType::Unknown,
            input_token: SwapTokenInfo {
                mint: "Unknown".to_string(),
                symbol: None,
                amount_raw: "0".to_string(),
                amount_ui: 0.0,
                decimals: 9,
            },
            output_token: SwapTokenInfo {
                mint: "Unknown".to_string(),
                symbol: None,
                amount_raw: "0".to_string(),
                amount_ui: 0.0,
                decimals: 9,
            },
            program_id: dex_program,
            dex_name: dex_name.map(|s| s.to_string()),
            log_messages: Vec::new(),
        })
    }

    /// Create a SwapTransaction from SwapInfo
    fn create_swap_transaction(
        &self,
        transaction: &TransactionResult,
        swap_info: &SwapInfo
    ) -> SwapTransaction {
        let signature = transaction.transaction.signatures.first().cloned().unwrap_or_default();

        SwapTransaction {
            signature,
            block_time: transaction.block_time,
            slot: transaction.slot,
            is_success: transaction.meta.as_ref().map_or(false, |m| m.err.is_none()),
            fee_sol: transaction.meta.as_ref().map_or(0.0, |m| (m.fee as f64) / 1_000_000_000.0),
            swap_type: swap_info.swap_type.clone(),
            input_token: SwapTokenInfo {
                mint: swap_info.input_mint.clone(),
                symbol: None,
                amount_raw: swap_info.input_amount.clone(),
                amount_ui: swap_info.input_amount.parse().unwrap_or(0.0) /
                (10_f64).powi(swap_info.input_decimals as i32),
                decimals: swap_info.input_decimals,
            },
            output_token: SwapTokenInfo {
                mint: swap_info.output_mint.clone(),
                symbol: None,
                amount_raw: swap_info.output_amount.clone(),
                amount_ui: swap_info.output_amount.parse().unwrap_or(0.0) /
                (10_f64).powi(swap_info.output_decimals as i32),
                decimals: swap_info.output_decimals,
            },
            program_id: swap_info.program_id.clone(),
            dex_name: Some(swap_info.dex_name.clone()),
            log_messages: Vec::new(),
        }
    }

    /// Comprehensive swap analysis with confidence scoring
    pub fn analyze_swap_confidence(
        &self,
        transaction: &TransactionResult
    ) -> (bool, f64, Vec<String>) {
        let mut confidence_score = 0.0;
        let mut reasons = Vec::new();

        // Factor 1: Known DEX program interaction (+30 points)
        let program_interactions = self.extract_program_interactions(transaction);
        let dex_interactions = self.find_dex_interactions(&program_interactions);
        if !dex_interactions.is_empty() {
            confidence_score += 30.0;
            reasons.push(
                format!(
                    "Known DEX program detected: {}",
                    dex_interactions[0].dex_name.as_deref().unwrap_or("Unknown")
                )
            );
        }

        // Factor 2: Bidirectional token transfers (+25 points)
        let token_transfers = self.extract_token_transfers(transaction);
        if self.has_bidirectional_transfers(&token_transfers) {
            confidence_score += 25.0;
            reasons.push("Bidirectional token transfers detected".to_string());
        }

        // Factor 3: Log message analysis (+15 points)
        if let Some(meta) = &transaction.meta {
            if let Some(logs) = &meta.log_messages {
                let swap_log_count = logs
                    .iter()
                    .filter(|log| log.to_lowercase().contains("swap"))
                    .count();
                if swap_log_count > 0 {
                    confidence_score += 15.0;
                    reasons.push(format!("Swap-related log messages found ({})", swap_log_count));
                }
            }
        }

        // Factor 4: Token balance changes consistency (+20 points)
        if token_transfers.len() >= 2 {
            let incoming_count = token_transfers
                .iter()
                .filter(|t| t.is_incoming)
                .count();
            let outgoing_count = token_transfers
                .iter()
                .filter(|t| !t.is_incoming)
                .count();
            if incoming_count >= 1 && outgoing_count >= 1 {
                confidence_score += 20.0;
                reasons.push(
                    format!("Balanced token flow: {} in, {} out", incoming_count, outgoing_count)
                );
            }
        }

        // Factor 5: Transaction success (+10 points)
        let is_success = transaction.meta.as_ref().map_or(false, |m| m.err.is_none());
        if is_success {
            confidence_score += 10.0;
            reasons.push("Transaction executed successfully".to_string());
        }

        let is_swap = confidence_score >= 50.0; // 50% confidence threshold
        (is_swap, confidence_score, reasons)
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
                        transfer.ui_amount.map_or("N/A".to_string(), |amount| amount.to_string()),
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
