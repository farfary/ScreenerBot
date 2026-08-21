//! Solana RPC request/response types.
//!
//! These mirror the JSON shapes Solana's `jsonParsed` RPC encoding returns
//! (`getTransaction`, `getSignatureStatuses`, `getTokenAccountsByOwner`, …).

use serde::{Deserialize, Serialize};

// ============================================================================
// Transaction Types
// ============================================================================

/// Transaction details from RPC
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionDetails {
    pub slot: u64,
    pub transaction: TransactionData,
    pub meta: Option<TransactionMeta>,
    // The RPC sends `blockTime`, but cached `raw_transaction_data` written before this
    // rename existed round-tripped the field as `block_time`. Without the alias those
    // rows deserialize with no timestamp at all, which silently dates every position
    // derived from them to the moment they were re-read.
    #[serde(rename = "blockTime", alias = "block_time")]
    pub block_time: Option<i64>,
}

/// Transaction data structure
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionData {
    pub message: serde_json::Value,
    pub signatures: Vec<String>,
}

/// Transaction metadata with balance changes
#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionMeta {
    pub err: Option<serde_json::Value>,
    #[serde(rename = "preBalances")]
    pub pre_balances: Vec<u64>,
    #[serde(rename = "postBalances")]
    pub post_balances: Vec<u64>,
    #[serde(rename = "preTokenBalances")]
    pub pre_token_balances: Option<Vec<TokenBalance>>,
    #[serde(rename = "postTokenBalances")]
    pub post_token_balances: Option<Vec<TokenBalance>>,
    pub fee: u64,
    #[serde(rename = "computeUnitsConsumed")]
    pub compute_units_consumed: Option<u64>,
    #[serde(rename = "logMessages")]
    pub log_messages: Option<Vec<String>>,
    #[serde(rename = "innerInstructions")]
    pub inner_instructions: Option<Vec<serde_json::Value>>,
}

/// Token balance information in transaction metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenBalance {
    #[serde(rename = "accountIndex")]
    pub account_index: u32,
    pub mint: String,
    pub owner: Option<String>,
    #[serde(rename = "programId")]
    pub program_id: Option<String>,
    #[serde(rename = "uiTokenAmount")]
    pub ui_token_amount: UiTokenAmount,
}

/// Token amount with UI representation
#[derive(Debug, Serialize, Deserialize)]
pub struct UiTokenAmount {
    pub amount: String,
    pub decimals: u8,
    #[serde(rename = "uiAmount")]
    pub ui_amount: Option<f64>,
    #[serde(rename = "uiAmountString")]
    pub ui_amount_string: Option<String>,
}

// ============================================================================
// Account Types
// ============================================================================

/// Structure to hold token account information
#[derive(Debug, Clone)]
pub struct TokenAccountInfo {
    pub account: String,
    pub mint: String,
    pub balance: u64,
    pub decimals: u8,
    pub is_token_2022: bool,
    pub is_nft: bool,
    /// True when the mint's freeze authority has frozen this account, so the balance
    /// exists but cannot be transferred or sold. Surfaced on positions as
    /// `holding_state = "frozen"` so the user can archive an unsellable holding
    /// instead of watching a sell fail forever.
    pub is_frozen: bool,
}

// ============================================================================
// Pagination & Response Types
// ============================================================================

/// Response structure for getProgramAccountsV2 with pagination support
#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedAccountsResponse {
    /// The accounts returned in this page
    pub accounts: Vec<serde_json::Value>,
    /// Pagination key for next page (None if this is the last page)
    pub pagination_key: Option<String>,
}

/// Signature status response structure for getSignatureStatuses
#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureStatusResponse {
    pub result: SignatureStatusResult,
}

/// Result wrapper for signature status response
#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureStatusResult {
    pub value: Vec<Option<SignatureStatusData>>,
}

/// Individual signature status data
#[derive(Debug, Serialize, Deserialize)]
pub struct SignatureStatusData {
    #[serde(rename = "confirmationStatus")]
    pub confirmation_status: Option<String>,
    pub err: Option<serde_json::Value>,
}
