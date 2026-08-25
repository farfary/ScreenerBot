//! Errors produced by the Solana chain adapter.

use std::time::Duration;

use crate::chains::ExecutionFailure;
use crate::errors::{ErrorClass, Severity};

/// Everything that can go wrong talking to Solana: RPC transport, keys,
/// address parsing, instruction/account decoding.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// An on-chain execution attempt failed in a chain-neutral way.
    #[error(transparent)]
    Execution(#[from] ExecutionFailure),

    /// A supplied address is not a valid Solana address of the expected kind.
    #[error("'{value}' is not a valid Solana {kind}")]
    InvalidAddress { kind: &'static str, value: String },
    /// The raw keypair bytes could not be turned into a usable keypair.
    #[error("the keypair bytes are not usable: {detail}")]
    InvalidKeypair { detail: String },
    /// The wallet keypair could not be loaded (config, decrypt, or storage failure).
    #[error("the wallet keypair could not be loaded: {detail}")]
    KeypairUnavailable { detail: String },
    /// Decrypting the wallet's stored private-key material failed.
    #[error(transparent)]
    SecureStorage(#[from] crate::secure_storage::Error),
    /// A Solana RPC call failed.
    #[error("solana rpc {operation} failed: {detail}")]
    Rpc {
        operation: &'static str,
        detail: String,
    },
    /// The requested account does not exist on chain.
    #[error("account {address} does not exist")]
    AccountNotFound { address: String },
    /// A payload (swap data, quote, instruction, response) could not be decoded.
    #[error("could not decode {payload}: {detail}")]
    Decode {
        payload: &'static str,
        detail: String,
    },
    /// An instruction could not be constructed.
    #[error("could not build the {instruction} instruction: {detail}")]
    InstructionBuild {
        instruction: &'static str,
        detail: String,
    },
}

/// Result alias for the Solana chain adapter.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Maps this Solana-native cause onto the chain-neutral classification,
    /// when one applies. Returns `None` for causes that have no
    /// chain-neutral equivalent (address/decode/build failures, etc).
    pub fn classify(&self) -> Option<ExecutionFailure> {
        match self {
            Error::Execution(e) => Some(e.clone()),
            _ => None,
        }
    }
}

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        match self {
            Error::Execution(e) => e.is_retryable(),
            Error::Rpc { .. } => true,
            Error::InvalidAddress { .. }
            | Error::InvalidKeypair { .. }
            | Error::KeypairUnavailable { .. }
            | Error::SecureStorage(_)
            | Error::AccountNotFound { .. }
            | Error::Decode { .. }
            | Error::InstructionBuild { .. } => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Execution(e) => e.retry_after(),
            Error::Rpc { .. } => Some(Duration::from_millis(500)),
            _ => None,
        }
    }

    fn severity(&self) -> Severity {
        match self {
            Error::Execution(e) => e.severity(),
            Error::InvalidAddress { .. } | Error::AccountNotFound { .. } => Severity::Warning,
            Error::InvalidKeypair { .. }
            | Error::KeypairUnavailable { .. }
            | Error::SecureStorage(_) => Severity::Critical,
            Error::Rpc { .. } => Severity::Warning,
            Error::Decode { .. } | Error::InstructionBuild { .. } => Severity::Error,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::Execution(e) => e.http_status(),
            Error::InvalidAddress { .. } => 400,
            Error::InvalidKeypair { .. }
            | Error::KeypairUnavailable { .. }
            | Error::SecureStorage(_) => 500,
            Error::AccountNotFound { .. } => 404,
            Error::Rpc { .. } => 503,
            Error::Decode { .. } | Error::InstructionBuild { .. } => 500,
        }
    }
}
