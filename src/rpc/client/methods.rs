//! RPC client method types and trait definition
//!
//! Type definitions and trait signatures for RPC client methods.
//! The trait implementation for `RpcClient` is in `methods_impl.rs`.

use crate::rpc::stats::RpcStatsResponse;
use crate::rpc::types::{CircuitState, ProviderKind, TokenAccountInfo};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    account::Account,
    commitment_config::CommitmentLevel,
    hash::Hash,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    transaction::VersionedTransaction,
};
use solana_transaction_status::{EncodedConfirmedTransactionWithStatusMeta, TransactionStatus};
use std::time::Duration;

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
    ) -> impl std::future::Future<Output = crate::Result<Option<Account>>> + Send;

    fn get_account_with_commitment(
        &self,
        pubkey: &Pubkey,
        commitment: CommitmentLevel,
    ) -> impl std::future::Future<Output = crate::Result<Option<Account>>> + Send;

    fn get_multiple_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> impl std::future::Future<Output = crate::Result<Vec<Option<Account>>>> + Send;

    // Balance methods
    fn get_sol_balance(
        &self,
        wallet: &str,
    ) -> impl std::future::Future<Output = crate::Result<f64>> + Send;

    /// Get token balance for a specific token account address
    ///
    /// Returns the UI amount (with decimals applied) for the given token account.
    fn get_token_account_balance(
        &self,
        token_account: &str,
    ) -> impl std::future::Future<Output = crate::Result<f64>> + Send;

    /// Get token balance for a wallet address and mint
    ///
    /// Finds the associated token account for the wallet+mint combination
    /// and returns the raw balance in smallest units (lamports-equivalent).
    /// Returns 0 if no token account exists.
    fn get_token_balance(
        &self,
        wallet_address: &str,
        mint: &str,
    ) -> impl std::future::Future<Output = crate::Result<u64>> + Send;

    // Blockhash methods
    fn get_latest_blockhash(&self)
        -> impl std::future::Future<Output = crate::Result<Hash>> + Send;

    fn get_latest_blockhash_with_commitment(
        &self,
        commitment: CommitmentLevel,
    ) -> impl std::future::Future<Output = crate::Result<(Hash, u64)>> + Send;

    // Block height
    fn get_block_height(&self) -> impl std::future::Future<Output = crate::Result<u64>> + Send;

    // Transaction methods
    fn send_transaction(
        &self,
        transaction: &VersionedTransaction,
    ) -> impl std::future::Future<Output = crate::Result<Signature>> + Send;

    fn get_transaction(
        &self,
        signature: &Signature,
    ) -> impl std::future::Future<
        Output = crate::Result<Option<EncodedConfirmedTransactionWithStatusMeta>>,
    > + Send;

    fn get_signature_statuses(
        &self,
        signatures: &[Signature],
    ) -> impl std::future::Future<Output = crate::Result<Vec<Option<TransactionStatus>>>> + Send;

    // Token account methods
    fn get_token_accounts_by_owner(
        &self,
        owner: &Pubkey,
    ) -> impl std::future::Future<Output = crate::Result<Vec<(Pubkey, Account)>>> + Send;

    // Slot
    fn get_slot(&self) -> impl std::future::Future<Output = crate::Result<u64>> + Send;

    // Rent
    fn get_minimum_balance_for_rent_exemption(
        &self,
        data_len: usize,
    ) -> impl std::future::Future<Output = crate::Result<u64>> + Send;

    // Health
    fn get_health(&self) -> impl std::future::Future<Output = crate::Result<()>> + Send;

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
    ) -> impl std::future::Future<Output = crate::Result<Signature>> + Send;

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
    ) -> impl std::future::Future<Output = crate::Result<Signature>> + Send;

    /// Send an already-serialized transaction (raw bytes as base64)
    fn send_raw_transaction(
        &self,
        transaction_base64: &str,
    ) -> impl std::future::Future<Output = crate::Result<Signature>> + Send;

    /// Confirm a transaction with timeout
    ///
    /// Polls for transaction confirmation status until confirmed or timeout.
    fn confirm_transaction(
        &self,
        signature: &Signature,
        commitment: CommitmentLevel,
        timeout: Duration,
    ) -> impl std::future::Future<Output = crate::Result<bool>> + Send;

    // =========================================================================
    // Token Account Utility Methods
    // =========================================================================

    /// Get all token accounts for a wallet (both SPL Token and Token-2022)
    fn get_all_token_accounts(
        &self,
        owner: &Pubkey,
    ) -> impl std::future::Future<Output = crate::Result<Vec<TokenAccountInfo>>> + Send;

    /// Check if a mint is Token-2022 by checking account owner
    fn is_token_2022_mint(
        &self,
        mint: &Pubkey,
    ) -> impl std::future::Future<Output = crate::Result<bool>> + Send;

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
    ) -> impl std::future::Future<Output = crate::Result<Vec<TokenAccountInfo>>> + Send;

    /// Check if a TOKEN ACCOUNT (not mint) is Token-2022 by checking owner program
    ///
    /// This is different from `is_token_2022_mint` - it checks the token account itself.
    fn is_token_account_token_2022(
        &self,
        token_account: &str,
    ) -> impl std::future::Future<Output = crate::Result<bool>> + Send;

    /// Get associated token account address for wallet and mint (async, returns String)
    ///
    /// Finds the ATA and verifies it exists on-chain.
    /// Returns the account address as a String if found.
    fn get_associated_token_account(
        &self,
        wallet_address: &str,
        mint: &str,
    ) -> impl std::future::Future<Output = crate::Result<String>> + Send;

    /// Send and confirm a signed Transaction (not VersionedTransaction)
    ///
    /// Serializes the transaction and sends it with confirmation polling.
    fn send_and_confirm_signed_transaction(
        &self,
        transaction: &solana_sdk::transaction::Transaction,
    ) -> impl std::future::Future<Output = crate::Result<Signature>> + Send;

    // =========================================================================
    // Transaction History Methods
    // =========================================================================

    /// Get transaction signatures for an address
    ///
    /// Returns signatures in reverse chronological order (newest first).
    /// Use `before` for pagination to get older signatures. Use `until` to stop the
    /// search at (and exclude) a known signature -- the gap-fill / cursor-resume case,
    /// where paging all the way back to `before=None` would re-walk history already
    /// on file.
    fn get_signatures_for_address(
        &self,
        address: &Pubkey,
        limit: Option<usize>,
        before: Option<&Signature>,
        until: Option<&Signature>,
    ) -> impl std::future::Future<Output = crate::Result<Vec<SignatureInfo>>> + Send;

    /// Batch get multiple transactions by signatures
    ///
    /// More efficient than calling get_transaction multiple times.
    /// Returns Vec with same order as input, None for transactions not found.
    fn get_transactions(
        &self,
        signatures: &[Signature],
    ) -> impl std::future::Future<
        Output = crate::Result<Vec<Option<EncodedConfirmedTransactionWithStatusMeta>>>,
    > + Send;

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
    ) -> impl std::future::Future<Output = crate::Result<Vec<(Pubkey, Account)>>> + Send;

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
    ) -> impl std::future::Future<Output = crate::Result<Vec<(Pubkey, Account)>>> + Send;

    // =========================================================================
    // Token Supply Methods
    // =========================================================================

    /// Get total supply of a token mint
    fn get_token_supply(
        &self,
        mint: &Pubkey,
    ) -> impl std::future::Future<Output = crate::Result<TokenSupply>> + Send;

    /// Get the largest token holders for a mint
    ///
    /// Returns up to 20 largest token accounts by balance.
    fn get_token_largest_accounts(
        &self,
        mint: &Pubkey,
    ) -> impl std::future::Future<Output = crate::Result<Vec<RpcTokenAccountBalance>>> + Send;

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
    fn get_provider_health(
        &self,
    ) -> impl std::future::Future<Output = Vec<ProviderHealthInfo>> + Send;

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
    ) -> impl std::future::Future<Output = crate::Result<Signature>> + Send;

    /// Sign, send, and confirm a transaction with the main wallet
    ///
    /// Convenience method combining sign_and_send_with_main_wallet with confirmation polling.
    fn sign_send_and_confirm_with_main_wallet(
        &self,
        transaction_base64: &str,
        commitment: CommitmentLevel,
        timeout: Duration,
    ) -> impl std::future::Future<Output = crate::Result<Signature>> + Send;

    // =========================================================================
    // Convenience Aliases
    // =========================================================================

    /// Get wallet signatures (alias for get_signatures_for_address)
    ///
    /// Convenience alias for code using this method name. `until` is the base58
    /// signature to stop paging at (exclusive) -- see `get_signatures_for_address`.
    fn get_wallet_signatures_main_rpc(
        &self,
        wallet_pubkey: &Pubkey,
        limit: usize,
        before: Option<&str>,
        until: Option<&str>,
    ) -> impl std::future::Future<Output = crate::Result<Vec<SignatureInfo>>> + Send;

    /// Get transaction details (returns TransactionDetails type)
    ///
    /// Convenience alias. Uses jsonParsed encoding for proper decoding.
    fn get_transaction_details(
        &self,
        signature: &str,
    ) -> impl std::future::Future<Output = crate::Result<crate::rpc::types::TransactionDetails>> + Send;

    /// Sign, send and confirm transaction with main wallet (simple API)
    ///
    /// Convenience method that uses default commitment and timeout.
    /// For more control, use sign_send_and_confirm_with_main_wallet.
    fn sign_send_and_confirm_transaction_simple(
        &self,
        transaction_base64: &str,
    ) -> impl std::future::Future<Output = crate::Result<Signature>> + Send;

    /// Sign, send and confirm with explicit keypair
    fn sign_send_and_confirm_with_keypair(
        &self,
        transaction_base64: &str,
        keypair: &Keypair,
    ) -> impl std::future::Future<Output = crate::Result<Signature>> + Send;
}
