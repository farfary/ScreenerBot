//! Typed failures of the direct pool-swap engine.
//!
//! Every consumer decides from the VARIANT, never from the message. The two
//! distinctions that matter most:
//!
//! * [`DirectSwapError::submitted`] — did anything reach the chain? A swap that
//!   failed before submission may be retried; one that timed out waiting for
//!   confirmation must NEVER be retried, because the transaction may still land.
//! * [`DirectSwapError::is_token_fault`] — is the token untradable here, or did
//!   OUR side fail? A router or RPC fault must never count against a mint.

use crate::chains::solana::solana_sdk::pubkey::Pubkey;
use std::fmt;

/// A failure inside the direct pool-swap engine.
#[derive(Debug, Clone)]
pub enum DirectSwapError {
    /// The pool program is not one the engine has a venue for.
    UnsupportedVenue { program: Pubkey },
    /// The pool account is missing, too short, or fails its layout check.
    PoolUndecodable { pool: Pubkey, detail: String },
    /// The pool exists but does not trade the requested mint pair.
    PairNotInPool {
        pool: Pubkey,
        input_mint: Pubkey,
        output_mint: Pubkey,
    },
    /// The pool is not currently accepting swaps (disabled, not yet open).
    PoolNotTradable { pool: Pubkey, detail: String },
    /// The pool cannot fill this size (reserves or in-range liquidity too thin).
    InsufficientLiquidity {
        pool: Pubkey,
        amount_in: u64,
        detail: String,
    },
    /// The request itself is invalid (zero amount, same mint both sides, ...).
    InvalidRequest { detail: String },
    /// A required on-chain account could not be read.
    AccountUnavailable { address: Pubkey, detail: String },
    /// Quote arithmetic could not produce a usable result.
    QuoteMath { detail: String },
    /// The transaction could not be assembled or signed.
    Build { detail: String },
    /// Preflight simulation rejected the transaction — nothing was submitted.
    SimulationRejected { detail: String, logs: Vec<String> },
    /// Submission failed before the transaction was accepted by a node.
    SubmitFailed { detail: String },
    /// Submitted, then the confirmation wait elapsed. The transaction MAY have
    /// landed. Never retry on this variant.
    ConfirmationTimeout { signature: String },
    /// Submitted and confirmed, but the chain reported the transaction failed.
    TransactionFailed { signature: String, detail: String },
    /// Confirmed on chain, but the wallet did not receive at least the minimum.
    /// Carries the signature so the position can still be reconciled.
    OutputNotReceived {
        signature: String,
        expected_minimum: u64,
        received: u64,
    },
    /// The wallet cannot cover the swap.
    InsufficientBalance {
        mint: Pubkey,
        required: u64,
        available: u64,
    },
}

impl DirectSwapError {
    /// Whether a transaction was handed to the network. `true` means a retry can
    /// double-spend; the caller must reconcile from chain state instead.
    pub fn submitted(&self) -> bool {
        matches!(
            self,
            DirectSwapError::ConfirmationTimeout { .. }
                | DirectSwapError::TransactionFailed { .. }
                | DirectSwapError::OutputNotReceived { .. }
        )
    }

    /// Whether this says something about the TOKEN rather than about our own
    /// side. Only these may feed blacklisting decisions.
    pub fn is_token_fault(&self) -> bool {
        matches!(
            self,
            DirectSwapError::PairNotInPool { .. }
                | DirectSwapError::PoolNotTradable { .. }
                | DirectSwapError::InsufficientLiquidity { .. }
        )
    }

    /// The on-chain signature, when one exists.
    pub fn signature(&self) -> Option<&str> {
        match self {
            DirectSwapError::ConfirmationTimeout { signature }
            | DirectSwapError::TransactionFailed { signature, .. }
            | DirectSwapError::OutputNotReceived { signature, .. } => Some(signature),
            _ => None,
        }
    }
}

impl fmt::Display for DirectSwapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DirectSwapError::UnsupportedVenue { program } => {
                write!(f, "no direct-swap venue for program {program}")
            }
            DirectSwapError::PoolUndecodable { pool, detail } => {
                write!(f, "pool {pool} could not be decoded: {detail}")
            }
            DirectSwapError::PairNotInPool {
                pool,
                input_mint,
                output_mint,
            } => write!(f, "pool {pool} does not trade {input_mint} -> {output_mint}"),
            DirectSwapError::PoolNotTradable { pool, detail } => {
                write!(f, "pool {pool} is not tradable: {detail}")
            }
            DirectSwapError::InsufficientLiquidity {
                pool,
                amount_in,
                detail,
            } => write!(
                f,
                "pool {pool} cannot fill {amount_in} raw units: {detail}"
            ),
            DirectSwapError::InvalidRequest { detail } => write!(f, "invalid swap request: {detail}"),
            DirectSwapError::AccountUnavailable { address, detail } => {
                write!(f, "account {address} unavailable: {detail}")
            }
            DirectSwapError::QuoteMath { detail } => write!(f, "quote math failed: {detail}"),
            DirectSwapError::Build { detail } => write!(f, "transaction build failed: {detail}"),
            DirectSwapError::SimulationRejected { detail, .. } => {
                write!(f, "simulation rejected the swap: {detail}")
            }
            DirectSwapError::SubmitFailed { detail } => write!(f, "swap submission failed: {detail}"),
            DirectSwapError::ConfirmationTimeout { signature } => write!(
                f,
                "swap {signature} was submitted but not confirmed in time; do not retry"
            ),
            DirectSwapError::TransactionFailed { signature, detail } => {
                write!(f, "swap {signature} failed on chain: {detail}")
            }
            DirectSwapError::OutputNotReceived {
                signature,
                expected_minimum,
                received,
            } => write!(
                f,
                "swap {signature} confirmed but delivered {received} raw units, below the {expected_minimum} minimum"
            ),
            DirectSwapError::InsufficientBalance {
                mint,
                required,
                available,
            } => write!(
                f,
                "wallet holds {available} of {mint}, needs {required}"
            ),
        }
    }
}

impl std::error::Error for DirectSwapError {}

/// Result alias for the direct pool-swap engine.
pub type DirectSwapResult<T> = Result<T, DirectSwapError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_confirmation_timeout_counts_as_submitted_so_it_is_never_retried() {
        let err = DirectSwapError::ConfirmationTimeout {
            signature: "sig".to_owned(),
        };
        assert!(err.submitted());
        assert_eq!(err.signature(), Some("sig"));
    }

    #[test]
    fn a_build_or_simulation_failure_never_counts_as_submitted() {
        assert!(!DirectSwapError::Build {
            detail: String::new()
        }
        .submitted());
        assert!(!DirectSwapError::SimulationRejected {
            detail: String::new(),
            logs: Vec::new(),
        }
        .submitted());
    }

    #[test]
    fn only_pool_side_failures_are_token_faults() {
        let pool = Pubkey::new_unique();
        assert!(DirectSwapError::PoolNotTradable {
            pool,
            detail: String::new()
        }
        .is_token_fault());
        assert!(!DirectSwapError::AccountUnavailable {
            address: pool,
            detail: String::new()
        }
        .is_token_fault());
        assert!(!DirectSwapError::SubmitFailed {
            detail: String::new()
        }
        .is_token_fault());
    }
}
