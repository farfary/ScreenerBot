/// Solana Blockchain Error Classifications
/// This module provides structured error handling for Solana blockchain-specific errors
/// replacing the current string-based error approach throughout the codebase.

use chrono::{ DateTime, Utc };
use serde::{ Deserialize, Serialize };
use serde_json::Value;
use std::fmt;
use tokio::time::Duration;
use crate::utils::safe_truncate;

/// Classification of blockchain error handling strategy
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureType {
    /// Permanent failures - cleanup immediately (slippage, insufficient funds)
    Permanent,
    /// Temporary failures - retry later (network congestion, blockhash expired)
    Temporary,
    /// Uncertain failures - wait for standard confirmation timeout
    Uncertain,
}

/// Structured Solana transaction error details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolanaTransactionError {
    pub error_type: FailureType,
    pub instruction_index: Option<u8>,
    pub error_code: Option<u32>,
    pub error_name: String,
    pub description: String,
    pub raw_error: Value,
}

impl SolanaTransactionError {
    /// Get human-readable error type name
    pub fn error_type_name(&self) -> &'static str {
        match self.error_type {
            FailureType::Permanent => "PERMANENT",
            FailureType::Temporary => "TEMPORARY",
            FailureType::Uncertain => "UNCERTAIN",
        }
    }
}

/// Primary Solana blockchain error classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlockchainError {
    // Block & Slot Issues
    BlockNotFound {
        slot: u64,
        signature: Option<String>,
    },
    SlotBehind {
        current_slot: u64,
        expected_slot: u64,
        lag_seconds: u64,
    },
    BlockhashExpired {
        blockhash: String,
        age_seconds: u64,
        signature: Option<String>,
    },

    // Account Related
    AccountNotFound {
        pubkey: String,
        context: String,
        rpc_endpoint: Option<String>,
    },
    AccountDataInvalid {
        pubkey: String,
        expected_type: String,
        actual_data_size: Option<usize>,
    },
    InsufficientBalance {
        pubkey: String,
        required_lamports: u64,
        available_lamports: u64,
        operation: String,
    },

    // Transaction Specific
    TransactionNotFound {
        signature: String,
        commitment_level: String,
        searched_endpoints: Vec<String>,
        age_seconds: Option<u64>,
    },
    TransactionExpired {
        signature: String,
        submitted_at: DateTime<Utc>,
        blockhash_used: Option<String>,
    },
    TransactionDropped {
        signature: String,
        reason: String,
        fee_paid: Option<u64>,
        attempts: u32,
    },

    // Instruction & Program Errors
    InstructionError {
        signature: String,
        instruction_index: u8,
        error_code: u32,
        error_description: String,
        program_id: Option<String>,
    },
    ProgramError {
        signature: String,
        program_id: String,
        error_code: u32,
        instruction_data: Option<String>,
        logs: Vec<String>,
    },

    // Commitment & Confirmation
    CommitmentTooLow {
        signature: String,
        requested: CommitmentLevel,
        available: CommitmentLevel,
        estimated_wait_seconds: u64,
    },
    ConfirmationTimeout {
        signature: String,
        waited_seconds: u64,
        commitment_level: CommitmentLevel,
        last_known_slot: Option<u64>,
    },

    // Network Congestion
    NetworkCongested {
        current_tps: f64,
        average_tps: f64,
        estimated_delay_seconds: u64,
        fee_escalation_recommended: bool,
    },
    HighFees {
        signature: Option<String>,
        current_fee_lamports: u64,
        recommended_fee_lamports: u64,
        network_congestion_level: CongestionLevel,
    },

    // Validator Issues
    ValidatorBehind {
        validator_id: String,
        validator_slot: u64,
        network_slot: u64,
        lag_minutes: u64,
    },
    ValidatorUnresponsive {
        validator_id: String,
        last_response_seconds: u64,
        rpc_endpoint: String,
    },

    // Specific Error Codes (Common Solana Program Errors)
    InsufficientFunds {
        signature: String,
        required: u64,
        available: u64,
    },
    InvalidAccountData {
        signature: String,
        account: String,
        expected_owner: String,
        actual_owner: Option<String>,
    },
    AccountAlreadyInUse {
        signature: String,
        account: String,
        current_user: Option<String>,
    },
    InvalidInstruction {
        signature: String,
        instruction_index: u8,
        reason: String,
    },
}

/// Commitment levels for transaction verification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CommitmentLevel {
    Processed,
    Confirmed,
    Finalized,
}

/// Network congestion levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CongestionLevel {
    Low, // < 1000 TPS
    Medium, // 1000-2000 TPS
    High, // 2000-3000 TPS
    Extreme, // > 3000 TPS
}

/// Error severity classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ErrorSeverity {
    Low, // Temporary, auto-recoverable
    Medium, // May need retry with different strategy
    High, // Requires attention, affects functionality
    Critical, // System failure, immediate action needed
}

/// Error recovery strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategy {
    Retry {
        delay_seconds: u64,
        max_attempts: u32,
        exponential_backoff: bool,
    },
    RefreshAndRetry {
        refresh_blockhash: bool,
        refresh_account_data: bool,
        delay_seconds: u64,
    },
    EscalateFees {
        increase_percentage: f64,
        max_fee_lamports: u64,
    },
    SwitchRpcProvider {
        preferred_commitment: CommitmentLevel,
    },
    WaitForConfirmation {
        timeout_seconds: u64,
        poll_interval_seconds: u64,
    },
    AbortOperation {
        reason: String,
        cleanup_required: bool,
    },
    NoRetry,
}

impl BlockchainError {
    /// Get the severity level of this error
    pub fn get_severity(&self) -> ErrorSeverity {
        match self {
            BlockchainError::AccountNotFound { .. } => ErrorSeverity::Low,
            BlockchainError::TransactionNotFound { age_seconds, .. } => {
                match age_seconds {
                    Some(age) if *age > 300 => ErrorSeverity::Medium, // > 5 minutes
                    Some(age) if *age > 60 => ErrorSeverity::Low, // > 1 minute
                    _ => ErrorSeverity::Low, // Recent
                }
            }
            BlockchainError::BlockhashExpired { age_seconds, .. } => {
                if *age_seconds > 300 { ErrorSeverity::Medium } else { ErrorSeverity::Low }
            }
            BlockchainError::NetworkCongested { current_tps, .. } => {
                if *current_tps < 500.0 {
                    ErrorSeverity::Critical
                } else if *current_tps < 1000.0 {
                    ErrorSeverity::High
                } else {
                    ErrorSeverity::Medium
                }
            }
            BlockchainError::ValidatorUnresponsive { last_response_seconds, .. } => {
                if *last_response_seconds > 300 {
                    ErrorSeverity::High
                } else {
                    ErrorSeverity::Medium
                }
            }
            BlockchainError::InstructionError { error_code, .. } => {
                match error_code {
                    0x1 => ErrorSeverity::Medium, // InsufficientFunds
                    0x6 => ErrorSeverity::Low, // InvalidAccountData (may be temporary)
                    _ => ErrorSeverity::Medium,
                }
            }
            BlockchainError::ConfirmationTimeout { waited_seconds, .. } => {
                if *waited_seconds > 300 { ErrorSeverity::High } else { ErrorSeverity::Medium }
            }
            _ => ErrorSeverity::Medium,
        }
    }

    /// Get the recommended recovery strategy
    pub fn get_recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            BlockchainError::BlockhashExpired { .. } => {
                RecoveryStrategy::RefreshAndRetry {
                    refresh_blockhash: true,
                    refresh_account_data: false,
                    delay_seconds: 1,
                }
            }
            BlockchainError::TransactionNotFound { age_seconds, .. } => {
                match age_seconds {
                    Some(age) if *age > 300 => RecoveryStrategy::NoRetry,
                    _ =>
                        RecoveryStrategy::WaitForConfirmation {
                            timeout_seconds: 120,
                            poll_interval_seconds: 10,
                        },
                }
            }
            BlockchainError::NetworkCongested { .. } => {
                RecoveryStrategy::EscalateFees {
                    increase_percentage: 50.0,
                    max_fee_lamports: 100_000,
                }
            }
            BlockchainError::ValidatorUnresponsive { .. } => {
                RecoveryStrategy::SwitchRpcProvider {
                    preferred_commitment: CommitmentLevel::Confirmed,
                }
            }
            BlockchainError::AccountNotFound { .. } => {
                RecoveryStrategy::Retry {
                    delay_seconds: 2,
                    max_attempts: 3,
                    exponential_backoff: false,
                }
            }
            BlockchainError::InstructionError { error_code, .. } => {
                match error_code {
                    0x1 => RecoveryStrategy::NoRetry, // InsufficientFunds - don't retry
                    _ =>
                        RecoveryStrategy::Retry {
                            delay_seconds: 5,
                            max_attempts: 2,
                            exponential_backoff: false,
                        },
                }
            }
            _ =>
                RecoveryStrategy::Retry {
                    delay_seconds: 3,
                    max_attempts: 3,
                    exponential_backoff: true,
                },
        }
    }

    /// Estimate recovery time for this error
    pub fn estimated_recovery_time(&self) -> Option<Duration> {
        match self {
            BlockchainError::BlockhashExpired { .. } => Some(Duration::from_secs(30)),
            BlockchainError::NetworkCongested { estimated_delay_seconds, .. } => {
                Some(Duration::from_secs(*estimated_delay_seconds))
            }
            BlockchainError::CommitmentTooLow { estimated_wait_seconds, .. } => {
                Some(Duration::from_secs(*estimated_wait_seconds))
            }
            BlockchainError::TransactionDropped { .. } => Some(Duration::from_secs(60)),
            BlockchainError::ValidatorBehind { lag_minutes, .. } => {
                Some(Duration::from_secs(*lag_minutes * 60))
            }
            _ => None,
        }
    }

    /// Check if this error should trigger a retry
    pub fn is_retryable(&self) -> bool {
        !matches!(
            self.get_recovery_strategy(),
            RecoveryStrategy::NoRetry | RecoveryStrategy::AbortOperation { .. }
        )
    }

    /// Get user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            BlockchainError::TransactionNotFound { signature, age_seconds, .. } => {
                match age_seconds {
                    Some(age) if *age > 300 =>
                        format!(
                            "Transaction {} not found after {} minutes - likely failed",
                            signature,
                            age / 60
                        ),
                    Some(age) => format!("Transaction {} still processing ({}s)", signature, age),
                    None => format!("Transaction {} not yet indexed", signature),
                }
            }
            BlockchainError::BlockhashExpired { signature, age_seconds, .. } => {
                format!(
                    "Transaction {} failed: blockhash expired ({}s old)",
                    signature
                        .as_ref()
                        .map(|s| safe_truncate(s, 8))
                        .unwrap_or("unknown"),
                    age_seconds
                )
            }
            BlockchainError::NetworkCongested { current_tps, estimated_delay_seconds, .. } => {
                format!(
                    "Network congested ({:.0} TPS), estimated delay: {}s",
                    current_tps,
                    estimated_delay_seconds
                )
            }
            BlockchainError::InsufficientFunds { signature, required, available } => {
                format!(
                    "Transaction {} failed: insufficient funds (need {} lamports, have {})",
                    signature,
                    required,
                    available
                )
            }
            BlockchainError::AccountNotFound { pubkey, context, .. } => {
                format!("Account {} not found ({})", safe_truncate(pubkey, 8), context)
            }
            _ => format!("{:?}", self), // Fallback to debug format
        }
    }
}

impl fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.user_message())
    }
}

impl std::error::Error for BlockchainError {}

/// Parse structured Solana transaction error from meta.err JSON
pub fn parse_structured_solana_error(
    error_value: &Value,
    signature: Option<&str>
) -> SolanaTransactionError {
    match error_value {
        // InstructionError format: {"InstructionError": [index, error_detail]}
        Value::Object(obj) if obj.contains_key("InstructionError") => {
            let instruction_error = &obj["InstructionError"];
            if let Some(array) = instruction_error.as_array() {
                if array.len() >= 2 {
                    let instruction_index = array[0].as_u64().unwrap_or(0) as u8;
                    let error_detail = &array[1];

                    return parse_instruction_error(instruction_index, error_detail, error_value);
                }
            }
        }
        // Transaction-level string errors
        Value::String(s) => {
            return parse_transaction_level_error(s, error_value);
        }
        _ => {}
    }

    // Fallback for unknown error structure
    SolanaTransactionError {
        error_type: FailureType::Uncertain,
        instruction_index: None,
        error_code: None,
        error_name: "UnknownError".to_string(),
        description: format!("Unknown error structure: {}", error_value),
        raw_error: error_value.clone(),
    }
}

/// Parse instruction-level errors (Custom codes and built-in errors)
fn parse_instruction_error(
    instruction_index: u8,
    error_detail: &Value,
    raw_error: &Value
) -> SolanaTransactionError {
    match error_detail {
        // Custom program errors: {"Custom": 6001}
        Value::Object(obj) if obj.contains_key("Custom") => {
            if let Some(code) = obj.get("Custom").and_then(|v| v.as_u64()) {
                let code = code as u32;
                let (failure_type, error_name, description) = classify_custom_error(code);

                SolanaTransactionError {
                    error_type: failure_type,
                    instruction_index: Some(instruction_index),
                    error_code: Some(code),
                    error_name,
                    description,
                    raw_error: raw_error.clone(),
                }
            } else {
                create_unknown_instruction_error(instruction_index, raw_error)
            }
        }
        // Built-in instruction errors: "InsufficientFunds"
        Value::String(s) => {
            let (failure_type, description) = classify_builtin_error(s);

            SolanaTransactionError {
                error_type: failure_type,
                instruction_index: Some(instruction_index),
                error_code: None,
                error_name: s.clone(),
                description,
                raw_error: raw_error.clone(),
            }
        }
        _ => create_unknown_instruction_error(instruction_index, raw_error),
    }
}

/// Parse transaction-level errors (string errors like "BlockhashNotFound")
fn parse_transaction_level_error(error_string: &str, raw_error: &Value) -> SolanaTransactionError {
    let (failure_type, description) = match error_string {
        "BlockhashNotFound" =>
            (FailureType::Temporary, "Transaction blockhash has expired".to_string()),
        "AlreadyProcessed" =>
            (FailureType::Permanent, "Transaction has already been processed".to_string()),
        "AccountInUse" =>
            (FailureType::Temporary, "Account is being used by another transaction".to_string()),
        "InsufficientFundsForFee" =>
            (FailureType::Permanent, "Insufficient SOL to pay transaction fee".to_string()),
        "SignatureFailure" =>
            (FailureType::Permanent, "Transaction signature verification failed".to_string()),
        "UnsupportedVersion" =>
            (FailureType::Permanent, "Transaction version is not supported".to_string()),
        "InvalidAccountIndex" =>
            (FailureType::Permanent, "Transaction contains invalid account reference".to_string()),
        "InvalidProgramForExecution" =>
            (FailureType::Permanent, "Program cannot be used for execution".to_string()),
        "SanitizeFailure" =>
            (FailureType::Permanent, "Transaction failed sanitization checks".to_string()),
        "WouldExceedMaxBlockCostLimit" =>
            (FailureType::Temporary, "Transaction would exceed block cost limit".to_string()),
        _ => (FailureType::Uncertain, format!("Unknown transaction error: {}", error_string)),
    };

    SolanaTransactionError {
        error_type: failure_type,
        instruction_index: None,
        error_code: None,
        error_name: error_string.to_string(),
        description,
        raw_error: raw_error.clone(),
    }
}

/// Classify custom program error codes
fn classify_custom_error(code: u32) -> (FailureType, String, String) {
    match code {
        // DEX Trading Errors (Permanent)
        6001 =>
            (
                FailureType::Permanent,
                "SlippageExceeded".to_string(),
                "Price slippage tolerance exceeded".to_string(),
            ),
        6002 =>
            (
                FailureType::Permanent,
                "InsufficientLiquidity".to_string(),
                "Insufficient liquidity in pool".to_string(),
            ),
        6003 =>
            (
                FailureType::Permanent,
                "InvalidTokenAccount".to_string(),
                "Invalid token account provided".to_string(),
            ),
        6004 =>
            (
                FailureType::Permanent,
                "InvalidPoolState".to_string(),
                "AMM pool is in invalid state".to_string(),
            ),
        6005 =>
            (
                FailureType::Permanent,
                "InvalidCalculation".to_string(),
                "Swap calculation failed".to_string(),
            ),
        6006 =>
            (
                FailureType::Temporary,
                "PoolSuspended".to_string(),
                "Trading pool is temporarily suspended".to_string(),
            ),
        6007 =>
            (
                FailureType::Permanent,
                "InvalidTokenMint".to_string(),
                "Invalid token mint provided".to_string(),
            ),
        6008 =>
            (
                FailureType::Permanent,
                "InvalidSwapDirection".to_string(),
                "Invalid swap direction".to_string(),
            ),
        6009 =>
            (
                FailureType::Temporary,
                "RouteNotFound".to_string(),
                "No valid route found for swap".to_string(),
            ),
        6010 =>
            (
                FailureType::Permanent,
                "PriceImpactTooHigh".to_string(),
                "Price impact exceeds maximum allowed".to_string(),
            ),

        // Orca DEX specific errors
        34 =>
            (
                FailureType::Permanent,
                "OrcaSlippageExceeded".to_string(),
                "Orca slippage tolerance exceeded".to_string(),
            ),
        35 =>
            (
                FailureType::Permanent,
                "OrcaInvalidSwap".to_string(),
                "Orca invalid swap parameters".to_string(),
            ),

        // Raydium DEX specific errors
        6000 =>
            (
                FailureType::Permanent,
                "RaydiumInvalidInput".to_string(),
                "Raydium invalid input parameters".to_string(),
            ),
        6011 =>
            (
                FailureType::Permanent,
                "RaydiumInsufficientFunds".to_string(),
                "Raydium insufficient funds for swap".to_string(),
            ),

        // SPL Token errors
        0 =>
            (
                FailureType::Permanent,
                "TokenInsufficientFunds".to_string(),
                "Insufficient token balance".to_string(),
            ),
        1 =>
            (
                FailureType::Permanent,
                "TokenInvalidInstruction".to_string(),
                "Invalid token instruction".to_string(),
            ),
        3 =>
            (
                FailureType::Permanent,
                "TokenOwnerMismatch".to_string(),
                "Token account owner mismatch".to_string(),
            ),
        5 =>
            (
                FailureType::Permanent,
                "TokenInvalidAmount".to_string(),
                "Invalid token amount".to_string(),
            ),
        17 =>
            (
                FailureType::Permanent,
                "TokenAccountFrozen".to_string(),
                "Token account is frozen".to_string(),
            ),

        // Generic program errors
        _ =>
            (
                FailureType::Uncertain,
                format!("CustomError{}", code),
                format!("Custom program error code: {}", code),
            ),
    }
}

/// Classify built-in instruction errors
fn classify_builtin_error(error_name: &str) -> (FailureType, String) {
    match error_name {
        "GenericError" => (FailureType::Uncertain, "Generic instruction error".to_string()),
        "InsufficientFunds" =>
            (FailureType::Permanent, "Insufficient lamports for operation".to_string()),
        "IncorrectProgramId" =>
            (FailureType::Permanent, "Incorrect program ID provided".to_string()),
        "InvalidAccountData" => (FailureType::Permanent, "Account data is invalid".to_string()),
        "InvalidInstructionData" =>
            (FailureType::Permanent, "Instruction data is invalid".to_string()),
        "ReadonlyLamportChange" =>
            (
                FailureType::Permanent,
                "Attempted to change lamports in readonly account".to_string(),
            ),
        "ReadonlyDataModified" =>
            (FailureType::Permanent, "Attempted to modify readonly account data".to_string()),
        "DuplicateAccountIndex" =>
            (FailureType::Permanent, "Duplicate account index in instruction".to_string()),
        "ExecutableModified" =>
            (FailureType::Permanent, "Attempted to modify executable account".to_string()),
        "RentEpochModified" =>
            (FailureType::Permanent, "Attempted to modify rent epoch".to_string()),
        "NotEnoughAccountKeys" =>
            (FailureType::Permanent, "Not enough account keys provided".to_string()),
        "AccountDataSizeChanged" =>
            (FailureType::Permanent, "Account data size unexpectedly changed".to_string()),
        "AccountNotExecutable" => (FailureType::Permanent, "Account is not executable".to_string()),
        "AccountBorrowFailed" => (FailureType::Temporary, "Failed to borrow account".to_string()),
        "AccountBorrowOutstanding" =>
            (FailureType::Temporary, "Account has outstanding borrow".to_string()),
        "DuplicateAccountOutOfSync" =>
            (FailureType::Permanent, "Duplicate account is out of sync".to_string()),
        _ => (FailureType::Uncertain, format!("Unknown built-in error: {}", error_name)),
    }
}

/// Helper to create unknown instruction error
fn create_unknown_instruction_error(
    instruction_index: u8,
    raw_error: &Value
) -> SolanaTransactionError {
    SolanaTransactionError {
        error_type: FailureType::Uncertain,
        instruction_index: Some(instruction_index),
        error_code: None,
        error_name: "UnknownInstructionError".to_string(),
        description: format!("Unknown instruction error at index {}", instruction_index),
        raw_error: raw_error.clone(),
    }
}

/// Determine if error represents permanent failure requiring immediate cleanup
pub fn is_permanent_failure(error: &SolanaTransactionError) -> bool {
    error.error_type == FailureType::Permanent
}

/// Determine if error is temporary and should be retried
pub fn is_temporary_failure(error: &SolanaTransactionError) -> bool {
    error.error_type == FailureType::Temporary
}

/// Parse Solana RPC error response into structured BlockchainError
pub fn parse_solana_error(
    error_message: &str,
    signature: Option<&str>,
    context: &str
) -> BlockchainError {
    let error_lower = error_message.to_lowercase();
    let sig = signature.map(|s| s.to_string());

    // Blockhash errors
    if
        error_lower.contains("blockhash") &&
        (error_lower.contains("not found") || error_lower.contains("expired"))
    {
        return BlockchainError::BlockhashExpired {
            blockhash: extract_blockhash(error_message).unwrap_or_else(|| "unknown".to_string()),
            age_seconds: 150, // Solana blockhashes expire after ~2.5 minutes
            signature: sig,
        };
    }

    // Account not found
    if error_lower.contains("account") && error_lower.contains("not found") {
        return BlockchainError::AccountNotFound {
            pubkey: extract_pubkey(error_message).unwrap_or_else(|| "unknown".to_string()),
            context: context.to_string(),
            rpc_endpoint: None,
        };
    }

    // Transaction not found
    if error_lower.contains("transaction") && error_lower.contains("not found") {
        return BlockchainError::TransactionNotFound {
            signature: sig.unwrap_or_else(|| "unknown".to_string()),
            commitment_level: "confirmed".to_string(),
            searched_endpoints: vec![],
            age_seconds: None,
        };
    }

    // Instruction errors with specific codes
    if error_lower.contains("instructionerror") || error_lower.contains("instruction error") {
        if let Some(code) = extract_error_code(error_message) {
            return BlockchainError::InstructionError {
                signature: sig.unwrap_or_else(|| "unknown".to_string()),
                instruction_index: 0,
                error_code: code,
                error_description: map_instruction_error_code(code),
                program_id: None,
            };
        }
    }

    // Network congestion indicators
    if error_lower.contains("timeout") || error_lower.contains("slow") {
        return BlockchainError::NetworkCongested {
            current_tps: 0.0, // Will be filled by caller if available
            average_tps: 1500.0,
            estimated_delay_seconds: 60,
            fee_escalation_recommended: true,
        };
    }

    // Insufficient funds
    if error_lower.contains("insufficient") && error_lower.contains("fund") {
        return BlockchainError::InsufficientFunds {
            signature: sig.unwrap_or_else(|| "unknown".to_string()),
            required: 0, // Will be extracted if available
            available: 0,
        };
    }

    // Default fallback for unmatched errors
    BlockchainError::TransactionDropped {
        signature: sig.unwrap_or_else(|| "unknown".to_string()),
        reason: error_message.to_string(),
        fee_paid: None,
        attempts: 1,
    }
}

/// Helper functions for error parsing
fn extract_blockhash(error_msg: &str) -> Option<String> {
    // Try to extract blockhash from error message
    None // Implement based on actual error formats
}

fn extract_pubkey(error_msg: &str) -> Option<String> {
    // Try to extract pubkey from error message
    None // Implement based on actual error formats
}

fn extract_error_code(error_msg: &str) -> Option<u32> {
    // Try to extract numeric error code from message
    None // Implement based on actual error formats
}

fn map_instruction_error_code(code: u32) -> String {
    match code {
        // Built-in Solana instruction errors
        0x0 => "GenericError".to_string(),
        0x1 => "InsufficientFunds".to_string(),
        0x2 => "IncorrectProgramId".to_string(),
        0x3 => "InvalidAccountData".to_string(),
        0x4 => "InvalidInstructionData".to_string(),
        0x5 => "ReadonlyLamportChange".to_string(),
        0x6 => "ReadonlyDataModified".to_string(),
        0x7 => "DuplicateAccountIndex".to_string(),
        0x8 => "ExecutableModified".to_string(),
        0x9 => "RentEpochModified".to_string(),
        0xa => "NotEnoughAccountKeys".to_string(),
        0xb => "AccountDataSizeChanged".to_string(),
        0xc => "AccountNotExecutable".to_string(),
        0xd => "AccountBorrowFailed".to_string(),
        0xe => "AccountBorrowOutstanding".to_string(),
        0xf => "DuplicateAccountOutOfSync".to_string(),

        // DEX Trading Errors
        6001 => "SlippageExceeded".to_string(),
        6002 => "InsufficientLiquidity".to_string(),
        6003 => "InvalidTokenAccount".to_string(),
        6004 => "InvalidPoolState".to_string(),
        6005 => "InvalidCalculation".to_string(),
        6006 => "PoolSuspended".to_string(),
        6007 => "InvalidTokenMint".to_string(),
        6008 => "InvalidSwapDirection".to_string(),
        6009 => "RouteNotFound".to_string(),
        6010 => "PriceImpactTooHigh".to_string(),

        // Orca DEX specific
        34 => "OrcaSlippageExceeded".to_string(),
        35 => "OrcaInvalidSwap".to_string(),

        // Raydium DEX specific
        6000 => "RaydiumInvalidInput".to_string(),
        6011 => "RaydiumInsufficientFunds".to_string(),

        // SPL Token Program errors
        0 => "TokenInsufficientFunds".to_string(),
        1 => "TokenInvalidInstruction".to_string(),
        3 => "TokenOwnerMismatch".to_string(),
        5 => "TokenInvalidAmount".to_string(),
        17 => "TokenAccountFrozen".to_string(),

        _ => format!("UnknownError({})", code),
    }
}
