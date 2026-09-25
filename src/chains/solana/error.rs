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
    /// A typed RPC failure whose retry policy depends on the provider error code.
    #[error("solana rpc {operation} failed: {source}")]
    RpcFailure {
        operation: &'static str,
        #[source]
        source: crate::rpc::RpcError,
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
    /// A built transaction failed its pre-send simulation. Nothing was
    /// submitted, so another router may safely try the same trade.
    #[error("{router} transaction failed simulation: {detail}")]
    SimulationRejected {
        router: &'static str,
        detail: String,
    },
    /// A built transaction would spend the wallet's lamports on something other
    /// than the trade, beyond what the caller allows -- typically rent for an
    /// account a venue keeps for itself, which only that venue can ever close.
    /// Nothing was submitted, so another router may safely try the same trade.
    ///
    /// `venue` is for people to read; `venue_program` is the address the retry
    /// path needs to ask the aggregator to route around. They differ whenever
    /// the aggregator knows a name for the program, which is most of the time.
    #[error("{router} would spend {extra_lamports} lamports beyond the trade, on an account owned by {venue}")]
    SwapCostRejected {
        router: &'static str,
        extra_lamports: u64,
        venue: String,
        venue_program: String,
    },
    /// A direct pool swap failed. Wrapped transparently so the engine's own
    /// typed cause survives the trip up to the caller: whether anything was
    /// submitted, and whether the failure says anything about the token, are
    /// both properties of that inner variant and must not be flattened into a
    /// message here.
    #[error(transparent)]
    DirectSwap(crate::chains::solana::swaps::direct::DirectSwapError),
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
            Error::RpcFailure { source, .. } => source.is_retryable(),
            Error::InvalidAddress { .. }
            | Error::InvalidKeypair { .. }
            | Error::KeypairUnavailable { .. }
            | Error::SecureStorage(_)
            | Error::AccountNotFound { .. }
            | Error::Decode { .. }
            | Error::InstructionBuild { .. }
            | Error::SimulationRejected { .. }
            | Error::SwapCostRejected { .. } => false,
            // A direct swap is never retried from here. The engine already
            // distinguishes "nothing was submitted" from "something may have
            // landed", and only the caller holding the position knows which of
            // those it is safe to act on.
            Error::DirectSwap(_) => false,
        }
    }

    fn retry_after(&self) -> Option<Duration> {
        match self {
            Error::Execution(e) => e.retry_after(),
            Error::Rpc { .. } => Some(Duration::from_millis(500)),
            Error::RpcFailure { source, .. } => source.retry_after(),
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
            Error::Rpc { .. } | Error::RpcFailure { .. } => Severity::Warning,
            Error::Decode { .. }
            | Error::InstructionBuild { .. }
            | Error::SimulationRejected { .. }
            | Error::SwapCostRejected { .. } => Severity::Error,
            // A swap that may have landed needs an operator's eyes on it.
            Error::DirectSwap(e) if e.submitted() => Severity::Critical,
            Error::DirectSwap(_) => Severity::Error,
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
            Error::Rpc { .. } | Error::RpcFailure { .. } => 503,
            Error::Decode { .. } | Error::InstructionBuild { .. } => 500,
            Error::SimulationRejected { .. } | Error::SwapCostRejected { .. } => 422,
            Error::DirectSwap(_) => 502,
        }
    }
}
