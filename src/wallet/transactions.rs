use crate::core::{
    BotResult,
    BotError,
    WalletTransaction,
    TransactionType,
    TransactionStatus,
    ParsedTransactionData,
    RpcManager,
};
use solana_sdk::pubkey::Pubkey;
use solana_transaction_status::{ ConfirmedSignatureInfo, UiTransactionEncoding };
use chrono::{ DateTime, Utc };
use std::collections::HashMap;

/// Manages wallet transaction queries and parsing
pub struct TransactionManager<'a> {
    rpc: &'a RpcManager,
}

impl<'a> TransactionManager<'a> {
    pub fn new(rpc: &'a RpcManager) -> Self {
        Self { rpc }
    }

    /// Get recent transactions for a wallet
    pub async fn get_recent_transactions(
        &self,
        wallet: &Pubkey,
        limit: usize
    ) -> BotResult<Vec<WalletTransaction>> {
        log::debug!("📜 Fetching {} recent transactions for wallet: {}", limit, wallet);

        // Get signature list
        let signatures = self.rpc.get_signatures_for_address(wallet, limit).await?;

        let mut transactions = Vec::new();

        for sig_info in signatures {
            match self.parse_transaction(&sig_info).await {
                Ok(Some(tx)) => transactions.push(tx),
                Ok(None) => {
                    continue;
                } // Skip unparseable transactions
                Err(e) => {
                    log::warn!("Failed to parse transaction {}: {}", sig_info.signature, e);
                    continue;
                }
            }
        }

        log::info!("📊 Parsed {} transactions", transactions.len());
        Ok(transactions)
    }

    /// Parse a single transaction
    async fn parse_transaction(
        &self,
        sig_info: &ConfirmedSignatureInfo
    ) -> BotResult<Option<WalletTransaction>> {
        // Get detailed transaction data
        let tx_response = tokio::task
            ::spawn_blocking({
                let rpc_client = &self.rpc.client;
                let signature = sig_info.signature.clone();
                move || {
                    rpc_client.get_transaction_with_config(
                        &signature.parse().unwrap(),
                        solana_client::rpc_config::RpcTransactionConfig {
                            encoding: Some(UiTransactionEncoding::JsonParsed),
                            commitment: Some(
                                solana_sdk::commitment_config::CommitmentConfig::confirmed()
                            ),
                            max_supported_transaction_version: Some(0),
                        }
                    )
                }
            }).await
            .map_err(|e| BotError::Rpc(format!("Task failed: {}", e)))?
            .map_err(|e| BotError::Rpc(format!("Failed to get transaction details: {}", e)))?;

        // Parse transaction status
        let status = if sig_info.err.is_some() {
            TransactionStatus::Failed
        } else {
            TransactionStatus::Success
        };

        // Parse transaction type and data
        let (tx_type, tokens_involved, sol_change, token_changes, parsed_data) =
            self.analyze_transaction(&tx_response).await?;

        Ok(
            Some(WalletTransaction {
                signature: sig_info.signature.clone(),
                block_time: sig_info.block_time,
                slot: sig_info.slot,
                transaction_type: tx_type,
                tokens_involved,
                sol_change,
                token_changes,
                fees: tx_response.transaction.meta
                    .as_ref()
                    .map(|meta| meta.fee)
                    .unwrap_or(0),
                status,
                parsed_data,
            })
        )
    }

    /// Analyze transaction to extract trading information
    async fn analyze_transaction(
        &self,
        tx_response: &solana_transaction_status::EncodedConfirmedTransactionWithStatusMeta
    ) -> BotResult<
        (TransactionType, Vec<Pubkey>, i64, HashMap<Pubkey, i64>, Option<ParsedTransactionData>)
    > {
        let mut tx_type = TransactionType::Unknown;
        let mut tokens_involved = Vec::new();
        let mut sol_change = 0i64;
        let mut token_changes = HashMap::new();
        let mut parsed_data = None;

        // Analyze meta for balance changes
        if let Some(meta) = &tx_response.transaction.meta {
            // Check pre/post balances for SOL changes
            if
                let (Some(pre_balances), Some(post_balances)) = (
                    &meta.pre_balances,
                    &meta.post_balances,
                )
            {
                if !pre_balances.is_empty() && !post_balances.is_empty() {
                    sol_change = (post_balances[0] as i64) - (pre_balances[0] as i64);
                }
            }

            // Analyze token balance changes
            if
                let (Some(pre_token_balances), Some(post_token_balances)) = (
                    &meta.pre_token_balances,
                    &meta.post_token_balances,
                )
            {
                // Create maps for easier comparison
                let mut pre_balances_map: HashMap<String, u64> = HashMap::new();
                let mut post_balances_map: HashMap<String, u64> = HashMap::new();

                for balance in pre_token_balances {
                    if let Some(mint) = &balance.mint {
                        pre_balances_map.insert(
                            mint.clone(),
                            balance.ui_token_amount.amount.parse().unwrap_or(0)
                        );
                    }
                }

                for balance in post_token_balances {
                    if let Some(mint) = &balance.mint {
                        post_balances_map.insert(
                            mint.clone(),
                            balance.ui_token_amount.amount.parse().unwrap_or(0)
                        );

                        // Add to tokens involved
                        if let Ok(pubkey) = mint.parse::<Pubkey>() {
                            if !tokens_involved.contains(&pubkey) {
                                tokens_involved.push(pubkey);
                            }
                        }
                    }
                }

                // Calculate changes
                for (mint, post_amount) in &post_balances_map {
                    let pre_amount = pre_balances_map.get(mint).unwrap_or(&0);
                    let change = (*post_amount as i64) - (*pre_amount as i64);

                    if change != 0 {
                        if let Ok(pubkey) = mint.parse::<Pubkey>() {
                            token_changes.insert(pubkey, change);
                        }
                    }
                }

                // Check for tokens that were completely sold
                for (mint, pre_amount) in &pre_balances_map {
                    if !post_balances_map.contains_key(mint) && *pre_amount > 0 {
                        if let Ok(pubkey) = mint.parse::<Pubkey>() {
                            token_changes.insert(pubkey, -(*pre_amount as i64));
                            if !tokens_involved.contains(&pubkey) {
                                tokens_involved.push(pubkey);
                            }
                        }
                    }
                }
            }
        }

        // Determine transaction type based on changes
        if !token_changes.is_empty() {
            let positive_changes = token_changes
                .values()
                .filter(|&&v| v > 0)
                .count();
            let negative_changes = token_changes
                .values()
                .filter(|&&v| v < 0)
                .count();

            if positive_changes > 0 && sol_change < 0 {
                tx_type = TransactionType::Buy;
            } else if negative_changes > 0 && sol_change > 0 {
                tx_type = TransactionType::Sell;
            } else if positive_changes > 0 && negative_changes > 0 {
                tx_type = TransactionType::Swap;
            }
        } else if sol_change != 0 {
            tx_type = TransactionType::Transfer;
        }

        // Try to create parsed data for trades
        if matches!(tx_type, TransactionType::Buy | TransactionType::Sell | TransactionType::Swap) {
            parsed_data = self.create_parsed_trade_data(&token_changes, sol_change).await;
        }

        Ok((tx_type, tokens_involved, sol_change, token_changes, parsed_data))
    }

    /// Create parsed trade data from changes
    async fn create_parsed_trade_data(
        &self,
        token_changes: &HashMap<Pubkey, i64>,
        sol_change: i64
    ) -> Option<ParsedTransactionData> {
        let mut input_token = None;
        let mut output_token = None;
        let mut input_amount = None;
        let mut output_amount = None;

        // Find input and output tokens
        for (mint, &change) in token_changes {
            if change > 0 {
                output_token = Some(*mint);
                output_amount = Some(change as u64);
            } else if change < 0 {
                input_token = Some(*mint);
                input_amount = Some(-change as u64);
            }
        }

        // If SOL was involved, include it
        if sol_change > 0 && output_token.is_none() {
            output_token = Some(Pubkey::from_str(crate::core::WSOL_MINT).ok()?);
            output_amount = Some(sol_change as u64);
        } else if sol_change < 0 && input_token.is_none() {
            input_token = Some(Pubkey::from_str(crate::core::WSOL_MINT).ok()?);
            input_amount = Some(-sol_change as u64);
        }

        // Calculate price per token if we have both amounts
        let price_per_token = if
            let (Some(input_amt), Some(output_amt)) = (input_amount, output_amount)
        {
            if output_amt > 0 { Some((input_amt as f64) / (output_amt as f64)) } else { None }
        } else {
            None
        };

        Some(ParsedTransactionData {
            input_token,
            output_token,
            input_amount,
            output_amount,
            price_per_token,
            pool_address: None, // Would need more analysis to determine
            dex: None, // Would need program analysis to determine
        })
    }

    /// Get transactions involving a specific token
    pub async fn get_token_transactions(
        &self,
        wallet: &Pubkey,
        token_mint: &Pubkey
    ) -> BotResult<Vec<WalletTransaction>> {
        let all_transactions = self.get_recent_transactions(wallet, 200).await?;

        let token_transactions: Vec<WalletTransaction> = all_transactions
            .into_iter()
            .filter(|tx| tx.tokens_involved.contains(token_mint))
            .collect();

        Ok(token_transactions)
    }
}

use std::str::FromStr;
