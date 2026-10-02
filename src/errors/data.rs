// Copyright (c) 2024-2026 ScreenerBot (screenerbot.io)
// SPDX-License-Identifier: BUSL-1.1

//! Data processing errors — parsing, transformation, and validation failures.

#[derive(Debug, Clone, thiserror::Error)]
pub enum DataError {
    #[error("failed to parse {data_type}: {error}")]
    ParseError { data_type: String, error: String },
    #[error("invalid value for field '{field}' ({value}): {reason}")]
    ValidationError {
        field: String,
        value: String,
        reason: String,
    },
    #[error("expected {expected}, received {received}")]
    InvalidFormat { expected: String, received: String },
    #[error("invalid amount '{amount}': {reason}")]
    InvalidAmount { amount: String, reason: String },
}
