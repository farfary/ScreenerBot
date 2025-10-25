/// Swap types and common structures
///
/// This module defines the core types used across the swap system.
use crate::constants::{SOL_MINT, SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID};
use solana_sdk::{pubkey::Pubkey, signature::Signature, transaction::Transaction};
use std::fmt;

/// Direction of the swap operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapDirection {
    /// Buying tokens with SOL
    Buy,
    /// Selling tokens for SOL
    Sell,
}

/// Swap request parameters
#[derive(Debug, Clone)]
pub struct SwapRequest {
    /// Pool address to swap in
    pub pool_address: Pubkey,
    /// Token mint address (non-SOL token)
    pub token_mint: Pubkey,
    /// Amount for the swap (SOL for buy, tokens for sell)
    pub amount: f64,
    /// Swap direction (buy or sell)
    pub direction: SwapDirection,
    /// Slippage tolerance in basis points (100 = 1%)
    pub slippage_bps: u16,
    /// Whether this is a dry run (no actual transaction)
    pub dry_run: bool,
}

/// Calculated swap parameters
#[derive(Debug, Clone)]
pub struct SwapParams {
    /// Input amount in UI units
    pub input_amount: f64,
    /// Expected output amount in UI units
    pub expected_output: f64,
    /// Minimum output amount (after slippage) in UI units
    pub minimum_output: f64,
    /// Input amount in raw units (smallest denomination)
    pub input_amount_raw: u64,
    /// Minimum output amount in raw units (smallest denomination)
    pub minimum_output_raw: u64,
}

/// Swap execution result
#[derive(Debug, Clone)]
pub struct SwapResult {
    /// Transaction signature (if executed)
    pub signature: Option<Signature>,
    /// Calculated swap parameters
    pub params: SwapParams,
    /// Transaction object (for dry runs or failed executions)
    pub transaction: Option<Transaction>,
    /// Execution success status
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// Swap-related errors
#[derive(Debug, Clone)]
pub enum SwapError {
    /// Invalid input parameters
    InvalidInput(String),
    /// Pool not found or invalid
    InvalidPool(String),
    /// Insufficient balance
    InsufficientBalance(String),
    /// Calculation error
    CalculationError(String),
    /// Transaction building error
    TransactionError(String),
    /// Execution error
    ExecutionError(String),
    /// RPC error
    RpcError(String),
    /// Decoder error
    DecoderError(String),
}

impl fmt::Display for SwapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SwapError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            SwapError::InvalidPool(msg) => write!(f, "Invalid pool: {}", msg),
            SwapError::InsufficientBalance(msg) => write!(f, "Insufficient balance: {}", msg),
            SwapError::CalculationError(msg) => write!(f, "Calculation error: {}", msg),
            SwapError::TransactionError(msg) => write!(f, "Transaction error: {}", msg),
            SwapError::ExecutionError(msg) => write!(f, "Execution error: {}", msg),
            SwapError::RpcError(msg) => write!(f, "RPC error: {}", msg),
            SwapError::DecoderError(msg) => write!(f, "Decoder error: {}", msg),
        }
    }
}

impl std::error::Error for SwapError {}

impl From<solana_sdk::program_error::ProgramError> for SwapError {
    fn from(error: solana_sdk::program_error::ProgramError) -> Self {
        SwapError::TransactionError(format!("Program error: {:?}", error))
    }
}
