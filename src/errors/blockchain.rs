//! Solana blockchain error classifications.
//!
//! Structured error handling for Solana blockchain-specific errors, replacing
//! string-based error matching throughout the codebase.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

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
    Low,     // < 1000 TPS
    Medium,  // 1000-2000 TPS
    High,    // 2000-3000 TPS
    Extreme, // > 3000 TPS
}

/// Error severity classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, PartialOrd)]
pub enum ErrorSeverity {
    Low,      // Temporary, auto-recoverable
    Medium,   // May need retry with different strategy
    High,     // Requires attention, affects functionality
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
                    Some(age) if *age > 60 => ErrorSeverity::Low,     // > 1 minute
                    _ => ErrorSeverity::Low,                          // Recent
                }
            }
            BlockchainError::BlockhashExpired { age_seconds, .. } => {
                if *age_seconds > 300 {
                    ErrorSeverity::Medium
                } else {
                    ErrorSeverity::Low
                }
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
            BlockchainError::ValidatorUnresponsive {
                last_response_seconds,
                ..
            } => {
                if *last_response_seconds > 300 {
                    ErrorSeverity::High
                } else {
                    ErrorSeverity::Medium
                }
            }
            BlockchainError::InstructionError { error_code, .. } => {
                match error_code {
                    0x1 => ErrorSeverity::Medium, // InsufficientFunds
                    0x6 => ErrorSeverity::Low,    // InvalidAccountData (may be temporary)
                    _ => ErrorSeverity::Medium,
                }
            }
            BlockchainError::ConfirmationTimeout { waited_seconds, .. } => {
                if *waited_seconds > 300 {
                    ErrorSeverity::High
                } else {
                    ErrorSeverity::Medium
                }
            }
            _ => ErrorSeverity::Medium,
        }
    }

    /// Get the recommended recovery strategy
    pub fn get_recovery_strategy(&self) -> RecoveryStrategy {
        match self {
            BlockchainError::BlockhashExpired { .. } => RecoveryStrategy::RefreshAndRetry {
                refresh_blockhash: true,
                refresh_account_data: false,
                delay_seconds: 1,
            },
            BlockchainError::TransactionNotFound { age_seconds, .. } => match age_seconds {
                Some(age) if *age > 300 => RecoveryStrategy::NoRetry,
                _ => RecoveryStrategy::WaitForConfirmation {
                    timeout_seconds: 120,
                    poll_interval_seconds: 10,
                },
            },
            BlockchainError::NetworkCongested { .. } => RecoveryStrategy::EscalateFees {
                increase_percentage: 50.0,
                max_fee_lamports: 100_000,
            },
            BlockchainError::ValidatorUnresponsive { .. } => RecoveryStrategy::SwitchRpcProvider {
                preferred_commitment: CommitmentLevel::Confirmed,
            },
            BlockchainError::AccountNotFound { .. } => RecoveryStrategy::Retry {
                delay_seconds: 2,
                max_attempts: 3,
                exponential_backoff: false,
            },
            BlockchainError::InstructionError { error_code, .. } => {
                match error_code {
                    0x1 => RecoveryStrategy::NoRetry, // InsufficientFunds - don't retry
                    _ => RecoveryStrategy::Retry {
                        delay_seconds: 5,
                        max_attempts: 2,
                        exponential_backoff: false,
                    },
                }
            }
            _ => RecoveryStrategy::Retry {
                delay_seconds: 3,
                max_attempts: 3,
                exponential_backoff: true,
            },
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
            BlockchainError::TransactionNotFound {
                signature,
                age_seconds,
                ..
            } => match age_seconds {
                Some(age) if *age > 300 => format!(
                    "Transaction {} not found after {} minutes - likely failed",
                    signature,
                    age / 60
                ),
                Some(age) => format!("Transaction {signature} still processing ({age}s)"),
                None => format!("Transaction {signature} not yet indexed"),
            },
            BlockchainError::BlockhashExpired {
                signature,
                age_seconds,
                ..
            } => {
                format!(
                    "Transaction {} failed: blockhash expired ({}s old)",
                    signature.as_deref().unwrap_or("unknown"),
                    age_seconds
                )
            }
            BlockchainError::NetworkCongested {
                current_tps,
                estimated_delay_seconds,
                ..
            } => {
                format!(
                    "Network congested ({:.0} TPS), estimated delay: {}s",
                    current_tps, estimated_delay_seconds
                )
            }
            BlockchainError::InsufficientFunds {
                signature,
                required,
                available,
            } => {
                format!(
                    "Transaction {} failed: insufficient funds (need {} lamports, have {})",
                    signature, required, available
                )
            }
            BlockchainError::AccountNotFound {
                pubkey, context, ..
            } => {
                format!("Account {pubkey} not found ({context})")
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
