//! RPC client methods implementation
//!
//! These methods provide a standard RpcClient API backed by the RpcManager.

use super::RpcClient;
use crate::constants::{SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID};
use crate::rpc::stats::RpcStatsResponse;
use crate::rpc::types::{CircuitState, ProviderKind};
use crate::rpc::RpcError;
use base64::Engine;
use chrono::{DateTime, Utc};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentLevel,
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::VersionedTransaction,
};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, TransactionStatus};
use std::str::FromStr;
use std::time::Duration;

use crate::rpc::types::TokenAccountInfo;

/// Health information for a single RPC provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderHealthInfo {
    /// Provider identifier
    pub provider_id: String,
    /// Provider URL (masked for security)
    pub url_masked: String,
    /// Provider kind (Helius, QuickNode, etc.)
    pub kind: ProviderKind,
    /// Whether provider is currently healthy
    pub is_healthy: bool,
    /// Whether provider is enabled
    pub is_enabled: bool,
    /// Circuit breaker state
    pub circuit_state: CircuitState,
    /// Total calls made to this provider
    pub total_calls: u64,
    /// Total errors from this provider
    pub total_errors: u64,
    /// Success rate (0.0 - 100.0)
    pub success_rate: f64,
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Consecutive failures count
    pub consecutive_failures: u32,
    /// Consecutive successes count
    pub consecutive_successes: u32,
    /// Base rate limit (requests per second)
    pub base_rate_limit: u32,
    /// Last successful call time
    pub last_success: Option<DateTime<Utc>>,
    /// Last failed call time
    pub last_failure: Option<DateTime<Utc>>,
    /// Last error message
    pub last_error: Option<String>,
}

/// Information about a transaction signature from getSignaturesForAddress
#[derive(Debug, Clone)]
pub struct SignatureInfo {
    /// The transaction signature
    pub signature: Signature,
    /// The slot the transaction was confirmed in
    pub slot: u64,
    /// Error if the transaction failed, None if successful
    pub err: Option<String>,
    /// Optional memo attached to the transaction
    pub memo: Option<String>,
    /// Block time as Unix timestamp
    pub block_time: Option<i64>,
    /// Confirmation status (processed, confirmed, finalized)
    pub confirmation_status: Option<String>,
}

/// Token account balance information from getTokenLargestAccounts
#[derive(Debug, Clone)]
pub struct RpcTokenAccountBalance {
    /// The token account address
    pub address: Pubkey,
    /// The token balance amount as a string
    pub amount: String,
    /// The number of decimals for this token
    pub decimals: u8,
    /// The UI-friendly balance (amount / 10^decimals)
    pub ui_amount: Option<f64>,
    /// The UI amount as a string
    pub ui_amount_string: String,
}

/// Token supply information from getTokenSupply
#[derive(Debug, Clone)]
pub struct TokenSupply {
    /// Total supply as raw amount string
    pub amount: String,
    /// Number of decimals
    pub decimals: u8,
    /// UI-friendly amount
    pub ui_amount: Option<f64>,
    /// UI amount as string
    pub ui_amount_string: String,
}

/// Filter type for getProgramAccounts
#[derive(Debug, Clone)]
pub enum RpcFilterType {
    /// Filter by data size
    DataSize(u64),
    /// Filter by memcmp - offset and base58 encoded bytes
    Memcmp { offset: usize, bytes: String },
}

/// Trait providing all RPC client methods
pub trait RpcClientMethods {
    // Account methods
    fn get_account(
        &self,
        pubkey: &Pubkey,
    ) -> impl std::future::Future<Output = Result<Option<Account>, String>> + Send;

    fn get_account_with_commitment(
        &self,
        pubkey: &Pubkey,
        commitment: CommitmentLevel,
    ) -> impl std::future::Future<Output = Result<Option<Account>, String>> + Send;

    fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> impl std::future::Future<Output = Result<Vec<Option<Account>>, String>> + Send;

    // Balance methods
    fn get_sol_balance(
        &self,
        wallet: &str,
    ) -> impl std::future::Future<Output = Result<f64, String>> + Send;

    /// Get token balance for a specific token account address
    ///
    /// Returns the UI amount (with decimals applied) for the given token account.
    fn get_token_account_balance(
        &self,
        token_account: &str,
    ) -> impl std::future::Future<Output = Result<f64, String>> + Send;

    /// Get token balance for a wallet address and mint
    ///
    /// Finds the associated token account for the wallet+mint combination
    /// and returns the raw balance in smallest units (lamports-equivalent).
    /// Returns 0 if no token account exists.
    fn get_token_balance(
        &self,
        wallet_address: &str,
        mint: &str,
    ) -> impl std::future::Future<Output = Result<u64, String>> + Send;

    // Blockhash methods
    fn get_latest_blockhash(
        &self,
    ) -> impl std::future::Future<Output = Result<Hash, String>> + Send;

    fn get_latest_blockhash_with_commitment(
        &self,
        commitment: CommitmentLevel,
    ) -> impl std::future::Future<Output = Result<(Hash, u64), String>> + Send;

    // Block height
    fn get_block_height(&self) -> impl std::future::Future<Output = Result<u64, String>> + Send;

    // Transaction methods
    fn send_transaction(
        &self,
        transaction: &VersionedTransaction,
    ) -> impl std::future::Future<Output = Result<Signature, String>> + Send;

    fn get_transaction(
        &self,
        signature: &Signature,
    ) -> impl std::future::Future<Output = Result<Option<EncodedConfirmedTransactionWithStatusMeta>, String>>
           + Send;

    fn get_signature_statuses(
        &self,
        signatures: &[Signature],
    ) -> impl std::future::Future<Output = Result<Vec<Option<TransactionStatus>>, String>> + Send;

    // Token account methods
    fn get_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> impl std::future::Future<Output = Result<Vec<(Pubkey, Account)>, String>> + Send;

    // Slot
    fn get_slot(&self) -> impl std::future::Future<Output = Result<u64, String>> + Send;

    // Rent
    fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> impl std::future::Future<Output = Result<u64, String>> + Send;

    // Health
    fn get_health(&self) -> impl std::future::Future<Output = Result<(), String>> + Send;

    // URL access
    fn url(&self) -> impl std::future::Future<Output = String> + Send;

    // =========================================================================
    // Advanced Transaction Methods
    // =========================================================================

    /// Sign a base64-encoded transaction and send it
    ///
    /// Decodes the base64 transaction, signs it with the provided keypair,
    /// and sends it to the network.
    fn sign_and_send_transaction(
        &self,
        transaction_base64: &str,
        keypair: &Keypair,
    ) -> impl std::future::Future<Output = Result<Signature, String>> + Send;

    /// Sign, send, and confirm a transaction
    ///
    /// Signs the transaction with the keypair, sends it, then polls for confirmation
    /// with the specified timeout and commitment level.
    fn sign_send_and_confirm_transaction(
        &self,
        transaction_base64: &str,
        keypair: &Keypair,
        commitment: CommitmentLevel,
        timeout: Duration,
    ) -> impl std::future::Future<Output = Result<Signature, String>> + Send;

    /// Send an already-serialized transaction (raw bytes as base64)
    fn send_raw_transaction(
        &self,
        transaction_base64: &str,
    ) -> impl std::future::Future<Output = Result<Signature, String>> + Send;

    /// Confirm a transaction with timeout
    ///
    /// Polls for transaction confirmation status until confirmed or timeout.
    fn confirm_transaction(
        &self,
        signature: &Signature,
        commitment: CommitmentLevel,
        timeout: Duration,
    ) -> impl std::future::Future<Output = Result<bool, String>> + Send;

    // =========================================================================
    // Token Account Utility Methods
    // =========================================================================

    /// Get all token accounts for a wallet (both SPL Token and Token-2022)
    fn get_all_token_accounts(
        &self,
        owner: &Pubkey,
    ) -> impl std::future::Future<Output = Result<Vec<TokenAccountInfo>, String>> + Send;

    /// Check if a mint is Token-2022 by checking account owner
    fn is_token_2022_mint(
        &self,
        mint: &Pubkey,
    ) -> impl std::future::Future<Output = Result<bool, String>> + Send;

    /// Get associated token address for a wallet and mint
    ///
    /// This is a pure calculation (no RPC call needed) using PDA derivation.
    /// For Token-2022 mints, use `get_associated_token_address_with_program`
    fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey
    where
        Self: Sized;

    /// Get associated token address with specific token program
    fn get_associated_token_address_with_program(
        wallet: &Pubkey,
        mint: &Pubkey,
        token_program_id: &Pubkey,
    ) -> Pubkey
    where
        Self: Sized;

    // =========================================================================
    // String-based Convenience Methods
    // =========================================================================

    /// Get all token accounts using string address (convenience wrapper)
    fn get_all_token_accounts_str(
        &self,
        owner: &str,
    ) -> impl std::future::Future<Output = Result<Vec<TokenAccountInfo>, String>> + Send;

    /// Check if a TOKEN ACCOUNT (not mint) is Token-2022 by checking owner program
    ///
    /// This is different from `is_token_2022_mint` - it checks the token account itself.
    fn is_token_account_token_2022(
        &self,
        token_account: &str,
    ) -> impl std::future::Future<Output = Result<bool, String>> + Send;

    /// Get associated token account address for wallet and mint (async, returns String)
    ///
    /// Finds the ATA and verifies it exists on-chain.
    /// Returns the account address as a String if found.
    fn get_associated_token_account(
        &self,
        wallet_address: &str,
        mint: &str,
    ) -> impl std::future::Future<Output = Result<String, String>> + Send;

    /// Send and confirm a signed Transaction (not VersionedTransaction)
    ///
    /// Serializes the transaction and sends it with confirmation polling.
    fn send_and_confirm_signed_transaction(
        &self,
        transaction: &solana_sdk::transaction::Transaction,
    ) -> impl std::future::Future<Output = Result<Signature, String>> + Send;

    // =========================================================================
    // Transaction History Methods
    // =========================================================================

    /// Get transaction signatures for an address
    ///
    /// Returns signatures in reverse chronological order (newest first).
    /// Use `before` for pagination to get older signatures.
    fn get_signatures_for_address(
        &self,
        address: &Pubkey,
        limit: Option<usize>,
        before: Option<&Signature>,
    ) -> impl std::future::Future<Output = Result<Vec<SignatureInfo>, String>> + Send;

    /// Batch get multiple transactions by signatures
    ///
    /// More efficient than calling get_transaction multiple times.
    /// Returns Vec with same order as input, None for transactions not found.
    fn get_transactions(
        &self,
        signatures: &[Signature],
    ) -> impl std::future::Future<Output = Result<Vec<Option<EncodedConfirmedTransactionWithStatusMeta>>, String>> + Send;

    // =========================================================================
    // Program Account Methods
    // =========================================================================

    /// Get all accounts owned by a program
    ///
    /// Warning: This can return large amounts of data. Use filters to narrow results.
    /// Consider using `get_program_accounts_with_config` for more options.
    fn get_program_accounts(
        &self,
        program_id: &Pubkey,
        filters: Option<Vec<RpcFilterType>>,
    ) -> impl std::future::Future<Output = Result<Vec<(Pubkey, Account)>, String>> + Send;

    /// Get program accounts with full configuration options
    ///
    /// Supports encoding, commitment level, data slice, and filters.
    fn get_program_accounts_with_config(
        &self,
        program_id: &Pubkey,
        filters: Option<Vec<RpcFilterType>>,
        encoding: Option<&str>,
        data_slice: Option<(usize, usize)>,
        commitment: Option<CommitmentLevel>,
    ) -> impl std::future::Future<Output = Result<Vec<(Pubkey, Account)>, String>> + Send;

    // =========================================================================
    // Token Supply Methods
    // =========================================================================

    /// Get total supply of a token mint
    fn get_token_supply(
        &self,
        mint: &Pubkey,
    ) -> impl std::future::Future<Output = Result<TokenSupply, String>> + Send;

    /// Get the largest token holders for a mint
    ///
    /// Returns up to 20 largest token accounts by balance.
    fn get_token_largest_accounts(
        &self,
        mint: &Pubkey,
    ) -> impl std::future::Future<Output = Result<Vec<RpcTokenAccountBalance>, String>> + Send;

    // =========================================================================
    // Statistics and Health Methods
    // =========================================================================

    /// Get RPC statistics
    ///
    /// Returns aggregated statistics about RPC calls, errors, and latency.
    fn get_stats(&self) -> impl std::future::Future<Output = RpcStatsResponse> + Send;

    /// Get health information for all providers
    ///
    /// Returns detailed health info for each configured RPC provider.
    fn get_provider_health(&self) -> impl std::future::Future<Output = Vec<ProviderHealthInfo>> + Send;

    // =========================================================================
    // Convenience Methods
    // =========================================================================

    /// Sign a base64-encoded transaction with the main wallet and send it
    ///
    /// This is a convenience method that loads the main wallet keypair from
    /// config and calls sign_and_send_transaction. Useful when the caller
    /// doesn't need to manage keypairs directly.
    fn sign_and_send_with_main_wallet(
        &self,
        transaction_base64: &str,
    ) -> impl std::future::Future<Output = Result<Signature, String>> + Send;

    /// Sign, send, and confirm a transaction with the main wallet
    ///
    /// Convenience method combining sign_and_send_with_main_wallet with confirmation polling.
    fn sign_send_and_confirm_with_main_wallet(
        &self,
        transaction_base64: &str,
        commitment: CommitmentLevel,
        timeout: Duration,
    ) -> impl std::future::Future<Output = Result<Signature, String>> + Send;

    // =========================================================================
    // Convenience Aliases
    // =========================================================================

    /// Get wallet signatures (alias for get_signatures_for_address)
    ///
    /// Convenience alias for code using this method name.
    fn get_wallet_signatures_main_rpc(
        &self,
        wallet_pubkey: &Pubkey,
        limit: usize,
        before: Option<&str>,
    ) -> impl std::future::Future<Output = Result<Vec<SignatureInfo>, String>> + Send;

    /// Get transaction details (returns TransactionDetails type)
    ///
    /// Convenience alias. Uses jsonParsed encoding for proper decoding.
    fn get_transaction_details(
        &self,
        signature: &str,
    ) -> impl std::future::Future<Output = Result<crate::rpc::types::TransactionDetails, String>>
           + Send;

    /// Sign, send and confirm transaction with main wallet (simple API)
    ///
    /// Convenience method that uses default commitment and timeout.
    /// For more control, use sign_send_and_confirm_with_main_wallet.
    fn sign_send_and_confirm_transaction_simple(
        &self,
        transaction_base64: &str,
    ) -> impl std::future::Future<Output = Result<Signature, String>> + Send;

    /// Sign, send and confirm with explicit keypair
    fn sign_send_and_confirm_with_keypair(
        &self,
        transaction_base64: &str,
        keypair: &Keypair,
    ) -> impl std::future::Future<Output = Result<Signature, String>> + Send;
}

impl RpcClientMethods for RpcClient {
    async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<Account>, String> {
        self.get_account_with_commitment(pubkey, CommitmentLevel::Confirmed)
            .await
    }

    async fn get_account_with_commitment(
        &self,
        pubkey: &Pubkey,
        commitment: CommitmentLevel,
    ) -> Result<Option<Account>, String> {
        let params = serde_json::json!([
            pubkey.to_string(),
            {
                "encoding": "base64",
                "commitment": commitment_to_string(commitment)
            }
        ]);

        let result = self
            .manager
            .execute_raw("getAccountInfo", params)
            .await
            .map_err(|e| e.to_string())?;

        // Parse the response
        let value = result.get("value");
        if value.is_none() || value == Some(&serde_json::Value::Null) {
            return Ok(None);
        }

        let value = value.unwrap();
        parse_account_from_json(value)
    }

    async fn get_multiple_accounts(&self, pubkeys: &[Pubkey]) -> Result<Vec<Option<Account>>, String> {
        if pubkeys.is_empty() {
            return Ok(Vec::new());
        }

        // Batch in chunks of 100 (Solana limit)
        let mut all_accounts = Vec::with_capacity(pubkeys.len());

        for chunk in pubkeys.chunks(100) {
            let keys: Vec<String> = chunk.iter().map(|p| p.to_string()).collect();
            let params = serde_json::json!([
                keys,
                {
                    "encoding": "base64",
                    "commitment": "confirmed"
                }
            ]);

            let result = self
                .manager
                .execute_raw("getMultipleAccounts", params)
                .await
                .map_err(|e| e.to_string())?;

            let values = result
                .get("value")
                .and_then(|v| v.as_array())
                .ok_or("Invalid response: missing value array")?;

            for value in values {
                if value.is_null() {
                    all_accounts.push(None);
                } else {
                    all_accounts.push(parse_account_from_json(value)?);
                }
            }
        }

        Ok(all_accounts)
    }

    async fn get_sol_balance(&self, wallet: &str) -> Result<f64, String> {
        let params = serde_json::json!([wallet]);

        let result = self
            .manager
            .execute_raw("getBalance", params)
            .await
            .map_err(|e| e.to_string())?;

        let lamports = result
            .get("value")
            .and_then(|v| v.as_u64())
            .ok_or("Invalid balance response")?;

        Ok(lamports as f64 / 1_000_000_000.0)
    }

    async fn get_token_account_balance(&self, token_account: &str) -> Result<f64, String> {
        let params = serde_json::json!([token_account]);

        let result = self
            .manager
            .execute_raw("getTokenAccountBalance", params)
            .await
            .map_err(|e| e.to_string())?;

        let ui_amount = result
            .get("value")
            .and_then(|v| v.get("uiAmount"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);

        Ok(ui_amount)
    }

    async fn get_token_balance(&self, wallet_address: &str, mint: &str) -> Result<u64, String> {
        // Use getTokenAccountsByOwner to find token accounts for this wallet+mint
        let params = serde_json::json!([
            wallet_address,
            { "mint": mint },
            { "encoding": "jsonParsed", "commitment": "confirmed" }
        ]);

        let result = self
            .manager
            .execute_raw("getTokenAccountsByOwner", params)
            .await
            .map_err(|e| e.to_string())?;

        // Parse the response - look for token accounts and sum their balances
        let value = result.get("value").and_then(|v| v.as_array());
        
        if let Some(accounts) = value {
            if let Some(account) = accounts.first() {
                if let Some(amount_str) = account
                    .get("account")
                    .and_then(|a| a.get("data"))
                    .and_then(|d| d.get("parsed"))
                    .and_then(|p| p.get("info"))
                    .and_then(|i| i.get("tokenAmount"))
                    .and_then(|t| t.get("amount"))
                    .and_then(|a| a.as_str())
                {
                    return amount_str
                        .parse::<u64>()
                        .map_err(|e| format!("Failed to parse token amount: {}", e));
                }
            }
        }

        // No token account found - return 0
        Ok(0)
    }

    async fn get_latest_blockhash(&self) -> Result<Hash, String> {
        let (hash, _) = self
            .get_latest_blockhash_with_commitment(CommitmentLevel::Finalized)
            .await?;
        Ok(hash)
    }

    async fn get_latest_blockhash_with_commitment(
        &self,
        commitment: CommitmentLevel,
    ) -> Result<(Hash, u64), String> {
        let params = serde_json::json!([{
            "commitment": commitment_to_string(commitment)
        }]);

        let result = self
            .manager
            .execute_raw("getLatestBlockhash", params)
            .await
            .map_err(|e| e.to_string())?;

        let value = result.get("value").ok_or("Missing value")?;
        let blockhash = value
            .get("blockhash")
            .and_then(|v| v.as_str())
            .ok_or("Missing blockhash")?;
        let last_valid_block_height = value
            .get("lastValidBlockHeight")
            .and_then(|v| v.as_u64())
            .ok_or("Missing lastValidBlockHeight")?;

        let hash =
            Hash::from_str(blockhash).map_err(|e| format!("Invalid blockhash: {}", e))?;

        Ok((hash, last_valid_block_height))
    }

    async fn get_block_height(&self) -> Result<u64, String> {
        let params = serde_json::json!([]);

        let result = self
            .manager
            .execute_raw("getBlockHeight", params)
            .await
            .map_err(|e| e.to_string())?;

        result
            .as_u64()
            .ok_or_else(|| "Invalid block height response".to_string())
    }

    async fn send_transaction(&self, transaction: &VersionedTransaction) -> Result<Signature, String> {
        // Serialize transaction
        let tx_bytes = bincode::serialize(transaction)
            .map_err(|e| format!("Failed to serialize transaction: {}", e))?;
        let tx_base64 = base64::engine::general_purpose::STANDARD.encode(&tx_bytes);

        let params = serde_json::json!([
            tx_base64,
            {
                "encoding": "base64",
                "skipPreflight": false,
                "preflightCommitment": "confirmed",
                "maxRetries": 3
            }
        ]);

        let result = self
            .manager
            .execute_raw("sendTransaction", params)
            .await
            .map_err(|e| e.to_string())?;

        let sig_str = result.as_str().ok_or("Invalid signature response")?;

        Signature::from_str(sig_str).map_err(|e| format!("Invalid signature: {}", e))
    }

    async fn get_transaction(
        &self,
        signature: &Signature,
    ) -> Result<Option<EncodedConfirmedTransactionWithStatusMeta>, String> {
        let params = serde_json::json!([
            signature.to_string(),
            {
                "encoding": "jsonParsed",
                "commitment": "confirmed",
                "maxSupportedTransactionVersion": 0
            }
        ]);

        let result = self.manager.execute_raw("getTransaction", params).await;

        match result {
            Ok(value) => {
                if value.is_null() {
                    return Ok(None);
                }
                let tx: EncodedConfirmedTransactionWithStatusMeta =
                    serde_json::from_value(value)
                        .map_err(|e| format!("Failed to parse transaction: {}", e))?;
                Ok(Some(tx))
            }
            Err(RpcError::AccountNotFound { .. }) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }

    async fn get_signature_statuses(
        &self,
        signatures: &[Signature],
    ) -> Result<Vec<Option<TransactionStatus>>, String> {
        let sig_strings: Vec<String> = signatures.iter().map(|s| s.to_string()).collect();
        let params = serde_json::json!([sig_strings, { "searchTransactionHistory": true }]);

        let result = self
            .manager
            .execute_raw("getSignatureStatuses", params)
            .await
            .map_err(|e| e.to_string())?;

        let values = result
            .get("value")
            .and_then(|v| v.as_array())
            .ok_or("Invalid response")?;

        let mut statuses = Vec::with_capacity(values.len());
        for value in values {
            if value.is_null() {
                statuses.push(None);
            } else {
                let status: TransactionStatus = serde_json::from_value(value.clone())
                    .map_err(|e| format!("Failed to parse status: {}", e))?;
                statuses.push(Some(status));
            }
        }

        Ok(statuses)
    }

    async fn get_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<(Pubkey, Account)>, String> {
        let params = serde_json::json!([
            owner.to_string(),
            { "programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" },
            { "encoding": "base64" }
        ]);

        let result = self
            .manager
            .execute_raw("getTokenAccountsByOwner", params)
            .await
            .map_err(|e| e.to_string())?;

        let values = result
            .get("value")
            .and_then(|v| v.as_array())
            .ok_or("Invalid response")?;

        let mut accounts = Vec::with_capacity(values.len());
        for item in values {
            let pubkey_str = item
                .get("pubkey")
                .and_then(|v| v.as_str())
                .ok_or("Missing pubkey")?;
            let pubkey =
                Pubkey::from_str(pubkey_str).map_err(|e| format!("Invalid pubkey: {}", e))?;

            if let Some(account) =
                parse_account_from_json(item.get("account").unwrap_or(&serde_json::Value::Null))?
            {
                accounts.push((pubkey, account));
            }
        }

        Ok(accounts)
    }

    async fn get_slot(&self) -> Result<u64, String> {
        let params = serde_json::json!([]);

        let result = self
            .manager
            .execute_raw("getSlot", params)
            .await
            .map_err(|e| e.to_string())?;

        result
            .as_u64()
            .ok_or_else(|| "Invalid slot response".to_string())
    }

    async fn get_minimum_balance_for_rent_exemption(&self, data_len: usize) -> Result<u64, String> {
        let params = serde_json::json!([data_len]);

        let result = self
            .manager
            .execute_raw("getMinimumBalanceForRentExemption", params)
            .await
            .map_err(|e| e.to_string())?;

        result
            .as_u64()
            .ok_or_else(|| "Invalid rent response".to_string())
    }

    async fn get_health(&self) -> Result<(), String> {
        let params = serde_json::json!([]);

        self.manager
            .execute_raw("getHealth", params)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }

    async fn url(&self) -> String {
        self.manager.primary_url().await.unwrap_or_default()
    }

    // =========================================================================
    // Advanced Transaction Methods Implementation
    // =========================================================================

    async fn sign_and_send_transaction(
        &self,
        transaction_base64: &str,
        keypair: &Keypair,
    ) -> Result<Signature, String> {
        // Decode the base64 transaction
        let tx_bytes = base64::engine::general_purpose::STANDARD
            .decode(transaction_base64)
            .map_err(|e| format!("Failed to decode transaction: {}", e))?;

        // Deserialize the VersionedTransaction
        let mut transaction: VersionedTransaction = bincode::deserialize(&tx_bytes)
            .map_err(|e| format!("Failed to deserialize transaction: {}", e))?;

        // Sign the transaction (first signature index is the fee payer)
        let sig = keypair.sign_message(&transaction.message.serialize());
        if transaction.signatures.is_empty() {
            transaction.signatures.push(sig);
        } else {
            transaction.signatures[0] = sig;
        }

        // Serialize and send
        self.send_transaction(&transaction).await
    }

    async fn sign_send_and_confirm_transaction(
        &self,
        transaction_base64: &str,
        keypair: &Keypair,
        commitment: CommitmentLevel,
        timeout: Duration,
    ) -> Result<Signature, String> {
        // Sign and send the transaction
        let signature = self
            .sign_and_send_transaction(transaction_base64, keypair)
            .await?;

        // Confirm the transaction
        let confirmed = self
            .confirm_transaction(&signature, commitment, timeout)
            .await?;

        if confirmed {
            Ok(signature)
        } else {
            Err(format!(
                "Transaction {} not confirmed within timeout",
                signature
            ))
        }
    }

    async fn send_raw_transaction(&self, transaction_base64: &str) -> Result<Signature, String> {
        let params = serde_json::json!([
            transaction_base64,
            {
                "encoding": "base64",
                "skipPreflight": false,
                "preflightCommitment": "confirmed",
                "maxRetries": 3
            }
        ]);

        let result = self
            .manager
            .execute_raw("sendTransaction", params)
            .await
            .map_err(|e| e.to_string())?;

        let sig_str = result.as_str().ok_or("Invalid signature response")?;
        Signature::from_str(sig_str).map_err(|e| format!("Invalid signature: {}", e))
    }

    async fn confirm_transaction(
        &self,
        signature: &Signature,
        commitment: CommitmentLevel,
        timeout: Duration,
    ) -> Result<bool, String> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(500);
        let commitment_str = commitment_to_string(commitment);

        loop {
            // Check if we've exceeded the timeout
            if start.elapsed() >= timeout {
                return Ok(false);
            }

            // Query signature status
            let params = serde_json::json!([
                [signature.to_string()],
                { "searchTransactionHistory": false }
            ]);

            match self.manager.execute_raw("getSignatureStatuses", params).await {
                Ok(result) => {
                    if let Some(values) = result.get("value").and_then(|v| v.as_array()) {
                        if let Some(status) = values.first() {
                            if !status.is_null() {
                                // Check for error
                                if let Some(err) = status.get("err") {
                                    if !err.is_null() {
                                        return Err(format!(
                                            "Transaction failed: {}",
                                            serde_json::to_string(err).unwrap_or_default()
                                        ));
                                    }
                                }

                                // Check confirmation status
                                if let Some(conf_status) =
                                    status.get("confirmationStatus").and_then(|v| v.as_str())
                                {
                                    let is_confirmed = match commitment_str {
                                        "processed" => {
                                            conf_status == "processed"
                                                || conf_status == "confirmed"
                                                || conf_status == "finalized"
                                        }
                                        "confirmed" => {
                                            conf_status == "confirmed"
                                                || conf_status == "finalized"
                                        }
                                        "finalized" => conf_status == "finalized",
                                        _ => false,
                                    };

                                    if is_confirmed {
                                        return Ok(true);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // Transient error, continue polling
                }
            }

            // Wait before next poll
            tokio::time::sleep(poll_interval).await;
        }
    }

    // =========================================================================
    // Token Account Utility Methods Implementation
    // =========================================================================

    async fn get_all_token_accounts(
        &self,
        owner: &Pubkey,
    ) -> Result<Vec<TokenAccountInfo>, String> {
        let mut all_accounts = Vec::new();

        // Fetch SPL Token accounts
        let spl_params = serde_json::json!([
            owner.to_string(),
            { "programId": SPL_TOKEN_PROGRAM_ID },
            { "encoding": "jsonParsed" }
        ]);

        if let Ok(result) = self
            .manager
            .execute_raw("getTokenAccountsByOwner", spl_params)
            .await
        {
            if let Some(values) = result.get("value").and_then(|v| v.as_array()) {
                for item in values {
                    if let Some(info) = parse_token_account_info(item, false) {
                        all_accounts.push(info);
                    }
                }
            }
        }

        // Fetch Token-2022 accounts
        let token2022_params = serde_json::json!([
            owner.to_string(),
            { "programId": TOKEN_2022_PROGRAM_ID },
            { "encoding": "jsonParsed" }
        ]);

        if let Ok(result) = self
            .manager
            .execute_raw("getTokenAccountsByOwner", token2022_params)
            .await
        {
            if let Some(values) = result.get("value").and_then(|v| v.as_array()) {
                for item in values {
                    if let Some(info) = parse_token_account_info(item, true) {
                        all_accounts.push(info);
                    }
                }
            }
        }

        Ok(all_accounts)
    }

    async fn is_token_2022_mint(&self, mint: &Pubkey) -> Result<bool, String> {
        let params = serde_json::json!([
            mint.to_string(),
            { "encoding": "jsonParsed" }
        ]);

        let result = self
            .manager
            .execute_raw("getAccountInfo", params)
            .await
            .map_err(|e| e.to_string())?;

        let value = result.get("value");
        if value.is_none() || value == Some(&serde_json::Value::Null) {
            return Err(format!("Mint account not found: {}", mint));
        }

        let value = value.unwrap();
        if let Some(owner) = value.get("owner").and_then(|v| v.as_str()) {
            Ok(owner == TOKEN_2022_PROGRAM_ID)
        } else {
            Err("Missing owner field in account info".to_string())
        }
    }

    fn get_associated_token_address(wallet: &Pubkey, mint: &Pubkey) -> Pubkey {
        let token_program_id =
            Pubkey::from_str(SPL_TOKEN_PROGRAM_ID).expect("Invalid SPL Token program ID");
        Self::get_associated_token_address_with_program(wallet, mint, &token_program_id)
    }

    fn get_associated_token_address_with_program(
        wallet: &Pubkey,
        mint: &Pubkey,
        token_program_id: &Pubkey,
    ) -> Pubkey {
        let associated_token_program_id =
            Pubkey::from_str(crate::constants::ASSOCIATED_TOKEN_PROGRAM_ID)
                .expect("Invalid Associated Token program ID");

        // PDA derivation: [wallet, token_program, mint]
        let seeds = &[
            wallet.as_ref(),
            token_program_id.as_ref(),
            mint.as_ref(),
        ];

        let (address, _bump) =
            Pubkey::find_program_address(seeds, &associated_token_program_id);
        address
    }

    // =========================================================================
    // String-based Convenience Methods Implementation
    // =========================================================================

    async fn get_all_token_accounts_str(
        &self,
        owner: &str,
    ) -> Result<Vec<TokenAccountInfo>, String> {
        let owner_pubkey = Pubkey::from_str(owner)
            .map_err(|e| format!("Invalid owner address '{}': {}", owner, e))?;
        self.get_all_token_accounts(&owner_pubkey).await
    }

    async fn is_token_account_token_2022(&self, token_account: &str) -> Result<bool, String> {
        let account_pubkey = Pubkey::from_str(token_account)
            .map_err(|e| format!("Invalid token account address '{}': {}", token_account, e))?;

        let params = serde_json::json!([
            account_pubkey.to_string(),
            { "encoding": "jsonParsed" }
        ]);

        let result = self
            .manager
            .execute_raw("getAccountInfo", params)
            .await
            .map_err(|e| e.to_string())?;

        let value = result.get("value");
        if value.is_none() || value == Some(&serde_json::Value::Null) {
            return Err(format!("Token account not found: {}", token_account));
        }

        let value = value.unwrap();
        if let Some(owner) = value.get("owner").and_then(|v| v.as_str()) {
            Ok(owner == TOKEN_2022_PROGRAM_ID)
        } else {
            Err("Missing owner field in account info".to_string())
        }
    }

    async fn get_associated_token_account(
        &self,
        wallet_address: &str,
        mint: &str,
    ) -> Result<String, String> {
        let wallet_pubkey = Pubkey::from_str(wallet_address)
            .map_err(|e| format!("Invalid wallet address '{}': {}", wallet_address, e))?;
        let mint_pubkey = Pubkey::from_str(mint)
            .map_err(|e| format!("Invalid mint address '{}': {}", mint, e))?;

        // First try standard SPL Token ATA
        let spl_ata = Self::get_associated_token_address(&wallet_pubkey, &mint_pubkey);

        // Check if ATA exists
        if let Ok(Some(_)) = self.get_account(&spl_ata).await {
            return Ok(spl_ata.to_string());
        }

        // Try Token-2022 ATA
        let token_2022_program_id = Pubkey::from_str(TOKEN_2022_PROGRAM_ID)
            .map_err(|e| format!("Invalid Token-2022 program ID: {}", e))?;
        let token_2022_ata = Self::get_associated_token_address_with_program(
            &wallet_pubkey,
            &mint_pubkey,
            &token_2022_program_id,
        );

        if let Ok(Some(_)) = self.get_account(&token_2022_ata).await {
            return Ok(token_2022_ata.to_string());
        }

        // Return the SPL ATA address even if it doesn't exist (for creation)
        Ok(spl_ata.to_string())
    }

    async fn send_and_confirm_signed_transaction(
        &self,
        transaction: &solana_sdk::transaction::Transaction,
    ) -> Result<Signature, String> {
        use bincode;

        // Serialize the transaction
        let serialized = bincode::serialize(transaction)
            .map_err(|e| format!("Failed to serialize transaction: {}", e))?;

        // Encode to base64
        let transaction_base64 = base64::engine::general_purpose::STANDARD.encode(&serialized);

        // Send the transaction
        let params = serde_json::json!([
            transaction_base64,
            {
                "encoding": "base64",
                "skipPreflight": false,
                "preflightCommitment": "confirmed",
                "maxRetries": 3
            }
        ]);

        let result = self
            .manager
            .execute_raw("sendTransaction", params)
            .await
            .map_err(|e| e.to_string())?;

        let signature_str = result
            .as_str()
            .ok_or("Invalid sendTransaction response: expected signature string")?;

        let signature = Signature::from_str(signature_str)
            .map_err(|e| format!("Invalid signature in response: {}", e))?;

        // Poll for confirmation with timeout
        let timeout = Duration::from_secs(60);
        let confirmed = self
            .confirm_transaction(&signature, CommitmentLevel::Confirmed, timeout)
            .await?;

        if confirmed {
            Ok(signature)
        } else {
            Err(format!(
                "Transaction {} not confirmed within timeout",
                signature
            ))
        }
    }

    // =========================================================================
    // Transaction History Methods Implementation
    // =========================================================================

    async fn get_signatures_for_address(
        &self,
        address: &Pubkey,
        limit: Option<usize>,
        before: Option<&Signature>,
    ) -> Result<Vec<SignatureInfo>, String> {
        let mut config = serde_json::Map::new();
        
        if let Some(limit_val) = limit {
            config.insert("limit".to_string(), serde_json::Value::Number(limit_val.into()));
        }
        
        if let Some(before_sig) = before {
            config.insert("before".to_string(), serde_json::Value::String(before_sig.to_string()));
        }
        
        config.insert("commitment".to_string(), serde_json::Value::String("confirmed".to_string()));
        
        let params = serde_json::json!([
            address.to_string(),
            serde_json::Value::Object(config)
        ]);

        let result = self
            .manager
            .execute_raw("getSignaturesForAddress", params)
            .await
            .map_err(|e| e.to_string())?;

        let signatures_array = result
            .as_array()
            .ok_or("Invalid response: expected array")?;

        let mut signatures = Vec::with_capacity(signatures_array.len());
        
        for item in signatures_array {
            let sig_str = item
                .get("signature")
                .and_then(|v| v.as_str())
                .ok_or("Missing signature field")?;
            
            let signature = Signature::from_str(sig_str)
                .map_err(|e| format!("Invalid signature: {}", e))?;
            
            let slot = item
                .get("slot")
                .and_then(|v| v.as_u64())
                .ok_or("Missing slot field")?;
            
            let err = item
                .get("err")
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else {
                        Some(serde_json::to_string(v).unwrap_or_default())
                    }
                });
            
            let memo = item
                .get("memo")
                .and_then(|v| v.as_str())
                .map(String::from);
            
            let block_time = item
                .get("blockTime")
                .and_then(|v| v.as_i64());
            
            let confirmation_status = item
                .get("confirmationStatus")
                .and_then(|v| v.as_str())
                .map(String::from);

            signatures.push(SignatureInfo {
                signature,
                slot,
                err,
                memo,
                block_time,
                confirmation_status,
            });
        }

        Ok(signatures)
    }

    async fn get_transactions(
        &self,
        signatures: &[Signature],
    ) -> Result<Vec<Option<EncodedConfirmedTransactionWithStatusMeta>>, String> {
        if signatures.is_empty() {
            return Ok(Vec::new());
        }

        // Process in chunks to avoid overwhelming RPC
        let mut all_transactions = Vec::with_capacity(signatures.len());
        
        for chunk in signatures.chunks(20) {
            // Fetch in parallel within chunk
            let mut futures = Vec::with_capacity(chunk.len());
            
            for sig in chunk {
                futures.push(self.get_transaction(sig));
            }
            
            // Execute all futures in the chunk concurrently
            let results = futures::future::join_all(futures).await;
            
            for result in results {
                match result {
                    Ok(tx) => all_transactions.push(tx),
                    Err(_) => all_transactions.push(None),
                }
            }
        }

        Ok(all_transactions)
    }

    // =========================================================================
    // Program Account Methods Implementation
    // =========================================================================

    async fn get_program_accounts(
        &self,
        program_id: &Pubkey,
        filters: Option<Vec<RpcFilterType>>,
    ) -> Result<Vec<(Pubkey, Account)>, String> {
        self.get_program_accounts_with_config(
            program_id,
            filters,
            Some("base64"),
            None,
            Some(CommitmentLevel::Confirmed),
        ).await
    }

    async fn get_program_accounts_with_config(
        &self,
        program_id: &Pubkey,
        filters: Option<Vec<RpcFilterType>>,
        encoding: Option<&str>,
        data_slice: Option<(usize, usize)>,
        commitment: Option<CommitmentLevel>,
    ) -> Result<Vec<(Pubkey, Account)>, String> {
        let mut config = serde_json::Map::new();
        
        config.insert(
            "encoding".to_string(),
            serde_json::Value::String(encoding.unwrap_or("base64").to_string()),
        );
        
        if let Some(commitment_level) = commitment {
            config.insert(
                "commitment".to_string(),
                serde_json::Value::String(commitment_to_string(commitment_level).to_string()),
            );
        }
        
        if let Some((offset, length)) = data_slice {
            config.insert(
                "dataSlice".to_string(),
                serde_json::json!({
                    "offset": offset,
                    "length": length
                }),
            );
        }
        
        if let Some(filter_list) = filters {
            let filters_json: Vec<serde_json::Value> = filter_list
                .into_iter()
                .map(|f| match f {
                    RpcFilterType::DataSize(size) => serde_json::json!({ "dataSize": size }),
                    RpcFilterType::Memcmp { offset, bytes } => serde_json::json!({
                        "memcmp": {
                            "offset": offset,
                            "bytes": bytes
                        }
                    }),
                })
                .collect();
            
            config.insert("filters".to_string(), serde_json::Value::Array(filters_json));
        }
        
        let params = serde_json::json!([
            program_id.to_string(),
            serde_json::Value::Object(config)
        ]);

        let result = self
            .manager
            .execute_raw("getProgramAccounts", params)
            .await
            .map_err(|e| e.to_string())?;

        let accounts_array = result
            .as_array()
            .ok_or("Invalid response: expected array")?;

        let mut accounts = Vec::with_capacity(accounts_array.len());
        
        for item in accounts_array {
            let pubkey_str = item
                .get("pubkey")
                .and_then(|v| v.as_str())
                .ok_or("Missing pubkey field")?;
            
            let pubkey = Pubkey::from_str(pubkey_str)
                .map_err(|e| format!("Invalid pubkey: {}", e))?;
            
            let account_data = item
                .get("account")
                .ok_or("Missing account field")?;
            
            if let Some(account) = parse_account_from_json(account_data)? {
                accounts.push((pubkey, account));
            }
        }

        Ok(accounts)
    }

    // =========================================================================
    // Token Supply Methods Implementation
    // =========================================================================

    async fn get_token_supply(&self, mint: &Pubkey) -> Result<TokenSupply, String> {
        let params = serde_json::json!([
            mint.to_string(),
            { "commitment": "confirmed" }
        ]);

        let result = self
            .manager
            .execute_raw("getTokenSupply", params)
            .await
            .map_err(|e| e.to_string())?;

        let value = result
            .get("value")
            .ok_or("Missing value field")?;

        let amount = value
            .get("amount")
            .and_then(|v| v.as_str())
            .ok_or("Missing amount field")?
            .to_string();

        let decimals = value
            .get("decimals")
            .and_then(|v| v.as_u64())
            .ok_or("Missing decimals field")? as u8;

        let ui_amount = value
            .get("uiAmount")
            .and_then(|v| v.as_f64());

        let ui_amount_string = value
            .get("uiAmountString")
            .and_then(|v| v.as_str())
            .unwrap_or("0")
            .to_string();

        Ok(TokenSupply {
            amount,
            decimals,
            ui_amount,
            ui_amount_string,
        })
    }

    async fn get_token_largest_accounts(
        &self,
        mint: &Pubkey,
    ) -> Result<Vec<RpcTokenAccountBalance>, String> {
        let params = serde_json::json!([
            mint.to_string(),
            { "commitment": "confirmed" }
        ]);

        let result = self
            .manager
            .execute_raw("getTokenLargestAccounts", params)
            .await
            .map_err(|e| e.to_string())?;

        let values = result
            .get("value")
            .and_then(|v| v.as_array())
            .ok_or("Missing value array")?;

        let mut accounts = Vec::with_capacity(values.len());
        
        for item in values {
            let address_str = item
                .get("address")
                .and_then(|v| v.as_str())
                .ok_or("Missing address field")?;
            
            let address = Pubkey::from_str(address_str)
                .map_err(|e| format!("Invalid address: {}", e))?;

            let amount = item
                .get("amount")
                .and_then(|v| v.as_str())
                .ok_or("Missing amount field")?
                .to_string();

            let decimals = item
                .get("decimals")
                .and_then(|v| v.as_u64())
                .ok_or("Missing decimals field")? as u8;

            let ui_amount = item
                .get("uiAmount")
                .and_then(|v| v.as_f64());

            let ui_amount_string = item
                .get("uiAmountString")
                .and_then(|v| v.as_str())
                .unwrap_or("0")
                .to_string();

            accounts.push(RpcTokenAccountBalance {
                address,
                amount,
                decimals,
                ui_amount,
                ui_amount_string,
            });
        }

        Ok(accounts)
    }

    // =========================================================================
    // Statistics and Health Methods Implementation
    // =========================================================================

    async fn get_stats(&self) -> RpcStatsResponse {
        self.manager.get_stats().await
    }

    async fn get_provider_health(&self) -> Vec<ProviderHealthInfo> {
        // Delegate to the RpcClient method
        RpcClient::get_provider_health(self).await
    }

    // =========================================================================
    // Convenience Methods Implementation
    // =========================================================================

    async fn sign_and_send_with_main_wallet(
        &self,
        transaction_base64: &str,
    ) -> Result<Signature, String> {
        // Load main wallet keypair from config
        let keypair = crate::config::get_wallet_keypair()
            .map_err(|e| format!("Failed to load wallet keypair: {}", e))?;

        // Delegate to sign_and_send_transaction
        self.sign_and_send_transaction(transaction_base64, &keypair)
            .await
    }

    async fn sign_send_and_confirm_with_main_wallet(
        &self,
        transaction_base64: &str,
        commitment: CommitmentLevel,
        timeout: Duration,
    ) -> Result<Signature, String> {
        // Load main wallet keypair from config
        let keypair = crate::config::get_wallet_keypair()
            .map_err(|e| format!("Failed to load wallet keypair: {}", e))?;

        // Delegate to sign_send_and_confirm_transaction
        self.sign_send_and_confirm_transaction(transaction_base64, &keypair, commitment, timeout)
            .await
    }

    // =========================================================================
    // Convenience Implementations
    // =========================================================================

    async fn get_wallet_signatures_main_rpc(
        &self,
        wallet_pubkey: &Pubkey,
        limit: usize,
        before: Option<&str>,
    ) -> Result<Vec<SignatureInfo>, String> {
        let before_sig = match before {
            Some(sig_str) => Some(
                Signature::from_str(sig_str)
                    .map_err(|e| format!("Invalid before signature: {}", e))?,
            ),
            None => None,
        };
        self.get_signatures_for_address(wallet_pubkey, Some(limit), before_sig.as_ref())
            .await
    }

    async fn get_transaction_details(
        &self,
        signature: &str,
    ) -> Result<crate::rpc::types::TransactionDetails, String> {
        // Use jsonParsed encoding for proper decoding (required for v0 transactions with LUTs)
        let params = serde_json::json!([
            signature,
            {
                "encoding": "jsonParsed",
                "maxSupportedTransactionVersion": 0
            }
        ]);

        let result = self
            .manager
            .execute_raw("getTransaction", params)
            .await
            .map_err(|e| e.to_string())?;

        if result.is_null() {
            return Err(format!("Transaction not found: {}", signature));
        }

        serde_json::from_value(result)
            .map_err(|e| format!("Failed to parse transaction details: {}", e))
    }

    async fn sign_send_and_confirm_transaction_simple(
        &self,
        transaction_base64: &str,
    ) -> Result<Signature, String> {
        // Use default commitment and timeout
        self.sign_send_and_confirm_with_main_wallet(
            transaction_base64,
            CommitmentLevel::Confirmed,
            Duration::from_secs(60),
        )
        .await
    }

    async fn sign_send_and_confirm_with_keypair(
        &self,
        transaction_base64: &str,
        keypair: &Keypair,
    ) -> Result<Signature, String> {
        // Use default commitment and timeout
        self.sign_send_and_confirm_transaction(
            transaction_base64,
            keypair,
            CommitmentLevel::Confirmed,
            Duration::from_secs(60),
        )
        .await
    }
}

// Helper functions

fn commitment_to_string(commitment: CommitmentLevel) -> &'static str {
    match commitment {
        CommitmentLevel::Finalized => "finalized",
        CommitmentLevel::Confirmed => "confirmed",
        CommitmentLevel::Processed => "processed",
    }
}

fn parse_account_from_json(value: &serde_json::Value) -> Result<Option<Account>, String> {
    if value.is_null() {
        return Ok(None);
    }

    let data = value.get("data").ok_or("Missing data field")?;

    let data_bytes = if let Some(arr) = data.as_array() {
        // [data_base64, encoding]
        let encoded = arr
            .first()
            .and_then(|v| v.as_str())
            .ok_or("Invalid data")?;
        let encoding = arr.get(1).and_then(|v| v.as_str()).unwrap_or("base64");

        if encoding == "base64" {
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|e| format!("Failed to decode base64: {}", e))?
        } else {
            return Err(format!("Unsupported encoding: {}", encoding));
        }
    } else if let Some(s) = data.as_str() {
        // Direct base64 string
        base64::engine::general_purpose::STANDARD
            .decode(s)
            .map_err(|e| format!("Failed to decode base64: {}", e))?
    } else {
        return Err("Invalid data format".to_string());
    };

    let lamports = value
        .get("lamports")
        .and_then(|v| v.as_u64())
        .ok_or("Missing lamports")?;

    let owner_str = value
        .get("owner")
        .and_then(|v| v.as_str())
        .ok_or("Missing owner")?;
    let owner =
        Pubkey::from_str(owner_str).map_err(|e| format!("Invalid owner pubkey: {}", e))?;

    let executable = value
        .get("executable")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let rent_epoch = value
        .get("rentEpoch")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    Ok(Some(Account {
        lamports,
        data: data_bytes,
        owner,
        executable,
        rent_epoch,
    }))
}

/// Parse token account info from jsonParsed response
fn parse_token_account_info(item: &serde_json::Value, is_token_2022: bool) -> Option<TokenAccountInfo> {
    let pubkey_str = item.get("pubkey")?.as_str()?;
    let account = item.get("account")?;
    let data = account.get("data")?;
    let parsed = data.get("parsed")?;
    let info = parsed.get("info")?;

    let mint_str = info.get("mint")?.as_str()?;
    let token_amount = info.get("tokenAmount")?;
    let amount_str = token_amount.get("amount")?.as_str()?;
    let decimals = token_amount.get("decimals")?.as_u64()? as u8;
    let balance = amount_str.parse::<u64>().ok()?;

    // NFT detection: decimals=0 and balance=1 typically indicates an NFT
    let is_nft = decimals == 0 && balance == 1;

    Some(TokenAccountInfo {
        account: pubkey_str.to_string(),
        mint: mint_str.to_string(),
        balance,
        decimals,
        is_token_2022,
        is_nft,
    })
}
