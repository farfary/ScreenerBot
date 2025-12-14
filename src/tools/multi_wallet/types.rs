//! Multi-Wallet Trading Types
//!
//! Configuration and result types for multi-wallet operations.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::tools::types::DelayConfig;

// =============================================================================
// CONFIGURATION TYPES
// =============================================================================

/// Configuration for multi-buy operation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiBuyConfig {
    /// Token mint address to buy
    pub token_mint: String,
    /// Number of wallets to use for buying
    pub wallet_count: usize,
    /// Maximum total SOL to spend across all wallets (None = unlimited)
    pub total_sol_limit: Option<f64>,
    /// Minimum SOL amount per buy
    pub min_amount_sol: f64,
    /// Maximum SOL amount per buy
    pub max_amount_sol: f64,
    /// SOL buffer to leave in each wallet for transaction fees (default 0.015)
    pub sol_buffer: f64,
    /// Delay between operations
    pub delay: DelayConfig,
    /// Number of concurrent buy operations
    pub concurrency: usize,
    /// Slippage tolerance in basis points
    pub slippage_bps: u64,
    /// Optional: specific router to use (jupiter, raydium, gmgn)
    pub router: Option<String>,
    /// Abort flag for cancellation (not serialized)
    #[serde(skip)]
    pub abort_flag: Option<Arc<AtomicBool>>,
}

impl Default for MultiBuyConfig {
    fn default() -> Self {
        Self {
            token_mint: String::new(),
            wallet_count: 5,
            total_sol_limit: None,
            min_amount_sol: 0.01,
            max_amount_sol: 0.01,
            sol_buffer: 0.015,
            delay: DelayConfig::Fixed { delay_ms: 1000 },
            concurrency: 1,
            slippage_bps: 500,
            router: None,
            abort_flag: None,
        }
    }
}

impl MultiBuyConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.token_mint.is_empty() {
            return Err("Token mint is required".to_string());
        }
        if self.wallet_count == 0 {
            return Err("Wallet count must be at least 1".to_string());
        }
        if self.min_amount_sol < 0.001 {
            return Err("Minimum amount must be at least 0.001 SOL".to_string());
        }
        if self.max_amount_sol < self.min_amount_sol {
            return Err("Maximum amount must be >= minimum amount".to_string());
        }
        if self.sol_buffer < 0.005 {
            return Err("SOL buffer must be at least 0.005 SOL".to_string());
        }
        if self.concurrency == 0 {
            return Err("Concurrency must be at least 1".to_string());
        }
        if self.slippage_bps > 5000 {
            return Err("Slippage cannot exceed 50% (5000 bps)".to_string());
        }
        Ok(())
    }
}

/// Configuration for multi-sell operation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MultiSellConfig {
    /// Token mint address to sell
    pub token_mint: String,
    /// Specific wallet IDs to sell from (None = all wallets with token balance)
    pub wallet_ids: Option<Vec<i64>>,
    /// Percentage of token balance to sell (1.0 - 100.0)
    pub sell_percentage: f64,
    /// Minimum SOL required in wallet for transaction fee
    pub min_sol_for_fee: f64,
    /// Automatically top-up wallets with insufficient SOL for fees
    pub auto_topup: bool,
    /// Delay between operations
    pub delay: DelayConfig,
    /// Number of concurrent sell operations
    pub concurrency: usize,
    /// Slippage tolerance in basis points
    pub slippage_bps: u64,
    /// Consolidate SOL back to main wallet after sell
    pub consolidate_after: bool,
    /// Close token ATAs after selling (reclaim rent)
    pub close_atas_after: bool,
    /// Optional: specific router to use (jupiter, raydium, gmgn)
    pub router: Option<String>,
    /// Abort flag for cancellation (not serialized)
    #[serde(skip)]
    pub abort_flag: Option<Arc<AtomicBool>>,
}

impl Default for MultiSellConfig {
    fn default() -> Self {
        Self {
            token_mint: String::new(),
            wallet_ids: None,
            sell_percentage: 100.0,
            min_sol_for_fee: 0.01,
            auto_topup: true,
            delay: DelayConfig::Fixed { delay_ms: 1000 },
            concurrency: 1,
            slippage_bps: 500,
            consolidate_after: true,
            close_atas_after: true,
            router: None,
            abort_flag: None,
        }
    }
}

impl MultiSellConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.token_mint.is_empty() {
            return Err("Token mint is required".to_string());
        }
        if self.sell_percentage <= 0.0 || self.sell_percentage > 100.0 {
            return Err("Sell percentage must be between 0 and 100".to_string());
        }
        if self.min_sol_for_fee < 0.005 {
            return Err("Minimum SOL for fee must be at least 0.005 SOL".to_string());
        }
        if self.concurrency == 0 {
            return Err("Concurrency must be at least 1".to_string());
        }
        if self.slippage_bps > 5000 {
            return Err("Slippage cannot exceed 50% (5000 bps)".to_string());
        }
        Ok(())
    }
}

/// Configuration for consolidation operation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ConsolidateConfig {
    /// Specific wallet IDs to consolidate (None = all sub-wallets)
    pub wallet_ids: Option<Vec<i64>>,
    /// Transfer SOL back to main wallet
    pub transfer_sol: bool,
    /// Token mints to transfer (None = no tokens)
    pub transfer_tokens: Option<Vec<String>>,
    /// Close empty ATAs to reclaim rent
    pub close_atas: bool,
    /// Include Token-2022 accounts
    pub include_token_2022: bool,
    /// Leave rent-exempt amount (~0.00089 SOL) in sub-wallets
    pub leave_rent_exempt: bool,
}

impl Default for ConsolidateConfig {
    fn default() -> Self {
        Self {
            wallet_ids: None,
            transfer_sol: true,
            transfer_tokens: None,
            close_atas: true,
            include_token_2022: true,
            leave_rent_exempt: false,
        }
    }
}

impl ConsolidateConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<(), String> {
        // At least one operation should be enabled
        if !self.transfer_sol && self.transfer_tokens.is_none() && !self.close_atas {
            return Err("At least one consolidation operation must be enabled".to_string());
        }
        Ok(())
    }
}

// =============================================================================
// STATUS AND RESULT TYPES
// =============================================================================

/// Session status for multi-wallet operations
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// Session created but not started
    Pending,
    /// Funding wallets before execution
    Funding,
    /// Executing buy/sell operations
    Executing,
    /// Consolidating funds after execution
    Consolidating,
    /// Successfully completed
    Completed,
    /// Failed with error
    Failed,
    /// Aborted by user
    Aborted,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Pending => write!(f, "pending"),
            SessionStatus::Funding => write!(f, "funding"),
            SessionStatus::Executing => write!(f, "executing"),
            SessionStatus::Consolidating => write!(f, "consolidating"),
            SessionStatus::Completed => write!(f, "completed"),
            SessionStatus::Failed => write!(f, "failed"),
            SessionStatus::Aborted => write!(f, "aborted"),
        }
    }
}

/// Wallet plan for buy/sell operations
#[derive(Clone, Debug, Serialize)]
pub struct WalletPlan {
    /// Database ID of the wallet
    pub wallet_id: i64,
    /// Solana address (base58)
    pub wallet_address: String,
    /// User-friendly wallet name
    pub wallet_name: String,
    /// Current SOL balance
    pub sol_balance: f64,
    /// Current token balance (if applicable)
    pub token_balance: Option<f64>,
    /// Planned SOL amount for this operation
    pub planned_amount_sol: f64,
    /// Whether wallet needs funding before operation
    pub needs_funding: bool,
    /// Amount of SOL to fund (if needs_funding)
    pub funding_amount: f64,
}

/// Result of a single wallet operation
#[derive(Clone, Debug, Serialize)]
pub struct WalletOpResult {
    /// Database ID of the wallet
    pub wallet_id: i64,
    /// Solana address (base58)
    pub wallet_address: String,
    /// Whether operation succeeded
    pub success: bool,
    /// Transaction signature (if successful)
    pub signature: Option<String>,
    /// SOL amount involved (spent or received)
    pub amount_sol: Option<f64>,
    /// Token amount involved (bought or sold)
    pub token_amount: Option<f64>,
    /// Error message (if failed)
    pub error: Option<String>,
}

impl WalletOpResult {
    /// Create a successful result
    pub fn success(
        wallet_id: i64,
        wallet_address: String,
        signature: String,
        amount_sol: f64,
        token_amount: Option<f64>,
    ) -> Self {
        Self {
            wallet_id,
            wallet_address,
            success: true,
            signature: Some(signature),
            amount_sol: Some(amount_sol),
            token_amount,
            error: None,
        }
    }

    /// Create a failed result
    pub fn failure(wallet_id: i64, wallet_address: String, error: String) -> Self {
        Self {
            wallet_id,
            wallet_address,
            success: false,
            signature: None,
            amount_sol: None,
            token_amount: None,
            error: Some(error),
        }
    }
}

/// Overall session result for multi-wallet operations
#[derive(Clone, Debug, Serialize)]
pub struct SessionResult {
    /// Unique session identifier
    pub session_id: String,
    /// Whether overall operation succeeded
    pub success: bool,
    /// Total number of wallets involved
    pub total_wallets: usize,
    /// Number of successful operations
    pub successful_ops: usize,
    /// Number of failed operations
    pub failed_ops: usize,
    /// Total SOL spent (for buys) or fees (for sells)
    pub total_sol_spent: f64,
    /// Total SOL recovered (for sells or consolidation)
    pub total_sol_recovered: f64,
    /// Individual operation results
    pub operations: Vec<WalletOpResult>,
    /// Error message (if overall failure)
    pub error: Option<String>,
}

impl SessionResult {
    /// Create a new session result
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            success: true,
            total_wallets: 0,
            successful_ops: 0,
            failed_ops: 0,
            total_sol_spent: 0.0,
            total_sol_recovered: 0.0,
            operations: Vec::new(),
            error: None,
        }
    }

    /// Add an operation result
    pub fn add_operation(&mut self, result: WalletOpResult) {
        if result.success {
            self.successful_ops += 1;
            if let Some(sol) = result.amount_sol {
                self.total_sol_spent += sol;
            }
        } else {
            self.failed_ops += 1;
        }
        self.operations.push(result);
    }

    /// Finalize the result
    pub fn finalize(&mut self) {
        self.total_wallets = self.operations.len();
        // Consider success if more than half succeeded
        self.success = self.successful_ops > self.failed_ops || self.failed_ops == 0;
    }
}
