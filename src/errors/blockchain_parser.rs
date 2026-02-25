//! Parsing and classification functions for Solana blockchain errors.
//!
//! Converts raw RPC/transaction error JSON into structured
//! [`SolanaTransactionError`] and [`BlockchainError`] types defined in
//! [`super::blockchain`].

use serde_json::Value;

use super::blockchain::{BlockchainError, FailureType, SolanaTransactionError};

/// Parse structured Solana transaction error from meta.err JSON
pub fn parse_structured_solana_error(
    error_value: &Value,
    signature: Option<&str>,
) -> SolanaTransactionError {
    match error_value {
        // InstructionError format: {"InstructionError": [index, error_detail]}
        Value::Object(obj) if obj.contains_key("InstructionError") => {
            let instruction_error = &obj["InstructionError"];
            if let Some(array) = instruction_error.as_array() {
                if array.len() >= 2 {
                    let instruction_index = array[0].as_u64().unwrap_or_default() as u8;
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
        error_name: "UnknownError".to_owned(),
        description: format!("Unknown error structure: {error_value}"),
        raw_error: error_value.clone(),
    }
}

/// Parse instruction-level errors (Custom codes and built-in errors)
fn parse_instruction_error(
    instruction_index: u8,
    error_detail: &Value,
    raw_error: &Value,
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
        "BlockhashNotFound" => (
            FailureType::Temporary,
            "Transaction blockhash has expired".to_owned(),
        ),
        "AlreadyProcessed" => (
            FailureType::Permanent,
            "Transaction has already been processed".to_owned(),
        ),
        "AccountInUse" => (
            FailureType::Temporary,
            "Account is being used by another transaction".to_owned(),
        ),
        "InsufficientFundsForFee" => (
            FailureType::Permanent,
            "Insufficient SOL to pay transaction fee".to_owned(),
        ),
        "SignatureFailure" => (
            FailureType::Permanent,
            "Transaction signature verification failed".to_owned(),
        ),
        "UnsupportedVersion" => (
            FailureType::Permanent,
            "Transaction version is not supported".to_owned(),
        ),
        "InvalidAccountIndex" => (
            FailureType::Permanent,
            "Transaction contains invalid account reference".to_owned(),
        ),
        "InvalidProgramForExecution" => (
            FailureType::Permanent,
            "Program cannot be used for execution".to_owned(),
        ),
        "SanitizeFailure" => (
            FailureType::Permanent,
            "Transaction failed sanitization checks".to_owned(),
        ),
        "WouldExceedMaxBlockCostLimit" => (
            FailureType::Temporary,
            "Transaction would exceed block cost limit".to_owned(),
        ),
        _ => (
            FailureType::Uncertain,
            format!("Unknown transaction error: {error_string}"),
        ),
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
        6001 => (
            FailureType::Permanent,
            "SlippageExceeded".to_owned(),
            "Price slippage tolerance exceeded".to_owned(),
        ),
        6002 => (
            FailureType::Permanent,
            "InsufficientLiquidity".to_owned(),
            "Insufficient liquidity in pool".to_owned(),
        ),
        6003 => (
            FailureType::Permanent,
            "InvalidTokenAccount".to_owned(),
            "Invalid token account provided".to_owned(),
        ),
        6004 => (
            FailureType::Permanent,
            "InvalidPoolState".to_owned(),
            "AMM pool is in invalid state".to_owned(),
        ),
        6005 => (
            FailureType::Permanent,
            "InvalidCalculation".to_owned(),
            "Swap calculation failed".to_owned(),
        ),
        6006 => (
            FailureType::Temporary,
            "PoolSuspended".to_owned(),
            "Trading pool is temporarily suspended".to_owned(),
        ),
        6007 => (
            FailureType::Permanent,
            "InvalidTokenMint".to_owned(),
            "Invalid token mint provided".to_owned(),
        ),
        6008 => (
            FailureType::Permanent,
            "InvalidSwapDirection".to_owned(),
            "Invalid swap direction".to_owned(),
        ),
        6009 => (
            FailureType::Temporary,
            "RouteNotFound".to_owned(),
            "No valid route found for swap".to_owned(),
        ),
        6010 => (
            FailureType::Permanent,
            "PriceImpactTooHigh".to_owned(),
            "Price impact exceeds maximum allowed".to_owned(),
        ),

        // Orca DEX specific errors
        34 => (
            FailureType::Permanent,
            "OrcaSlippageExceeded".to_owned(),
            "Orca slippage tolerance exceeded".to_owned(),
        ),
        35 => (
            FailureType::Permanent,
            "OrcaInvalidSwap".to_owned(),
            "Orca invalid swap parameters".to_owned(),
        ),

        // Raydium DEX specific errors
        6000 => (
            FailureType::Permanent,
            "RaydiumInvalidInput".to_owned(),
            "Raydium invalid input parameters".to_owned(),
        ),
        6011 => (
            FailureType::Permanent,
            "RaydiumInsufficientFunds".to_owned(),
            "Raydium insufficient funds for swap".to_owned(),
        ),

        // SPL Token errors
        0 => (
            FailureType::Permanent,
            "TokenInsufficientFunds".to_owned(),
            "Insufficient token balance".to_owned(),
        ),
        1 => (
            FailureType::Permanent,
            "TokenInvalidInstruction".to_owned(),
            "Invalid token instruction".to_owned(),
        ),
        3 => (
            FailureType::Permanent,
            "TokenOwnerMismatch".to_owned(),
            "Token account owner mismatch".to_owned(),
        ),
        5 => (
            FailureType::Permanent,
            "TokenInvalidAmount".to_owned(),
            "Invalid token amount".to_owned(),
        ),
        17 => (
            FailureType::Permanent,
            "TokenAccountFrozen".to_owned(),
            "Token account is frozen".to_owned(),
        ),

        // Generic program errors
        _ => (
            FailureType::Uncertain,
            format!("CustomError{code}"),
            format!("Custom program error code: {code}"),
        ),
    }
}

/// Classify built-in instruction errors
fn classify_builtin_error(error_name: &str) -> (FailureType, String) {
    match error_name {
        "GenericError" => (
            FailureType::Uncertain,
            "Generic instruction error".to_owned(),
        ),
        "InsufficientFunds" => (
            FailureType::Permanent,
            "Insufficient lamports for operation".to_owned(),
        ),
        "IncorrectProgramId" => (
            FailureType::Permanent,
            "Incorrect program ID provided".to_owned(),
        ),
        "InvalidAccountData" => (FailureType::Permanent, "Account data is invalid".to_owned()),
        "InvalidInstructionData" => (
            FailureType::Permanent,
            "Instruction data is invalid".to_owned(),
        ),
        "ReadonlyLamportChange" => (
            FailureType::Permanent,
            "Attempted to change lamports in readonly account".to_owned(),
        ),
        "ReadonlyDataModified" => (
            FailureType::Permanent,
            "Attempted to modify readonly account data".to_owned(),
        ),
        "DuplicateAccountIndex" => (
            FailureType::Permanent,
            "Duplicate account index in instruction".to_owned(),
        ),
        "ExecutableModified" => (
            FailureType::Permanent,
            "Attempted to modify executable account".to_owned(),
        ),
        "RentEpochModified" => (
            FailureType::Permanent,
            "Attempted to modify rent epoch".to_owned(),
        ),
        "NotEnoughAccountKeys" => (
            FailureType::Permanent,
            "Not enough account keys provided".to_owned(),
        ),
        "AccountDataSizeChanged" => (
            FailureType::Permanent,
            "Account data size unexpectedly changed".to_owned(),
        ),
        "AccountNotExecutable" => (
            FailureType::Permanent,
            "Account is not executable".to_owned(),
        ),
        "AccountBorrowFailed" => (
            FailureType::Temporary,
            "Failed to borrow account".to_owned(),
        ),
        "AccountBorrowOutstanding" => (
            FailureType::Temporary,
            "Account has outstanding borrow".to_owned(),
        ),
        "DuplicateAccountOutOfSync" => (
            FailureType::Permanent,
            "Duplicate account is out of sync".to_owned(),
        ),
        _ => (
            FailureType::Uncertain,
            format!("Unknown built-in error: {error_name}"),
        ),
    }
}

/// Helper to create unknown instruction error
fn create_unknown_instruction_error(
    instruction_index: u8,
    raw_error: &Value,
) -> SolanaTransactionError {
    SolanaTransactionError {
        error_type: FailureType::Uncertain,
        instruction_index: Some(instruction_index),
        error_code: None,
        error_name: "UnknownInstructionError".to_owned(),
        description: format!("Unknown instruction error at index {instruction_index}"),
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
    context: &str,
) -> BlockchainError {
    let error_lower = error_message.to_lowercase();
    let sig = signature.map(|s| s.to_string());

    // Blockhash errors
    if error_lower.contains("blockhash")
        && (error_lower.contains("not found") || error_lower.contains("expired"))
    {
        return BlockchainError::BlockhashExpired {
            blockhash: extract_blockhash(error_message).unwrap_or_else(|| "unknown".to_owned()),
            age_seconds: 150, // Solana blockhashes expire after ~2.5 minutes
            signature: sig,
        };
    }

    // Account not found
    if error_lower.contains("account") && error_lower.contains("not found") {
        return BlockchainError::AccountNotFound {
            pubkey: extract_pubkey(error_message).unwrap_or_else(|| "unknown".to_owned()),
            context: context.to_string(),
            rpc_endpoint: None,
        };
    }

    // Transaction not found
    if error_lower.contains("transaction") && error_lower.contains("not found") {
        return BlockchainError::TransactionNotFound {
            signature: sig.unwrap_or_else(|| "unknown".to_owned()),
            commitment_level: "confirmed".to_owned(),
            searched_endpoints: vec![],
            age_seconds: None,
        };
    }

    // Instruction errors with specific codes
    if error_lower.contains("instructionerror") || error_lower.contains("instruction error") {
        if let Some(code) = extract_error_code(error_message) {
            return BlockchainError::InstructionError {
                signature: sig.unwrap_or_else(|| "unknown".to_owned()),
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
            signature: sig.unwrap_or_else(|| "unknown".to_owned()),
            required: 0, // Will be extracted if available
            available: 0,
        };
    }

    // Default fallback for unmatched errors
    BlockchainError::TransactionDropped {
        signature: sig.unwrap_or_else(|| "unknown".to_owned()),
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
        0x0 => "GenericError".to_owned(),
        0x1 => "InsufficientFunds".to_owned(),
        0x2 => "IncorrectProgramId".to_owned(),
        0x3 => "InvalidAccountData".to_owned(),
        0x4 => "InvalidInstructionData".to_owned(),
        0x5 => "ReadonlyLamportChange".to_owned(),
        0x6 => "ReadonlyDataModified".to_owned(),
        0x7 => "DuplicateAccountIndex".to_owned(),
        0x8 => "ExecutableModified".to_owned(),
        0x9 => "RentEpochModified".to_owned(),
        0xa => "NotEnoughAccountKeys".to_owned(),
        0xb => "AccountDataSizeChanged".to_owned(),
        0xc => "AccountNotExecutable".to_owned(),
        0xd => "AccountBorrowFailed".to_owned(),
        0xe => "AccountBorrowOutstanding".to_owned(),
        0xf => "DuplicateAccountOutOfSync".to_owned(),

        // DEX Trading Errors
        6001 => "SlippageExceeded".to_owned(),
        6002 => "InsufficientLiquidity".to_owned(),
        6003 => "InvalidTokenAccount".to_owned(),
        6004 => "InvalidPoolState".to_owned(),
        6005 => "InvalidCalculation".to_owned(),
        6006 => "PoolSuspended".to_owned(),
        6007 => "InvalidTokenMint".to_owned(),
        6008 => "InvalidSwapDirection".to_owned(),
        6009 => "RouteNotFound".to_owned(),
        6010 => "PriceImpactTooHigh".to_owned(),

        // Orca DEX specific
        34 => "OrcaSlippageExceeded".to_owned(),
        35 => "OrcaInvalidSwap".to_owned(),

        // Raydium DEX specific
        6000 => "RaydiumInvalidInput".to_owned(),
        6011 => "RaydiumInsufficientFunds".to_owned(),

        // SPL Token Program errors
        0 => "TokenInsufficientFunds".to_owned(),
        1 => "TokenInvalidInstruction".to_owned(),
        3 => "TokenOwnerMismatch".to_owned(),
        5 => "TokenInvalidAmount".to_owned(),
        17 => "TokenAccountFrozen".to_owned(),

        _ => format!("UnknownError({code})"),
    }
}

/// Check if an error message indicates RPC indexing delay
/// These are temporary delays where the transaction exists on-chain but RPC nodes
/// haven't indexed it yet. Should be retried with longer delays, not treated as permanent failures.
pub fn is_rpc_indexing_delay(error_msg: &str) -> bool {
    let msg = error_msg.to_lowercase();
    msg.contains("not yet indexed")
        || msg.contains("transaction not found")
        || msg.contains("not available yet")
        || msg.contains("still being indexed")
        || msg.contains("indexing in progress")
}

/// Check if an error is transient and should be retried
pub fn is_transient_rpc_error(error_msg: &str) -> bool {
    let msg = error_msg.to_lowercase();
    msg.contains("rpc error")
        || msg.contains("network error")
        || msg.contains("connection")
        || msg.contains("timeout")
        || msg.contains("rate limit")
        || msg.contains("503")
        || msg.contains("502")
        || msg.contains("429")
}
