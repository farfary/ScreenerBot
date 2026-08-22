//! Errors returned while constructing or parsing chain identities.

use std::fmt;

use crate::chains::ChainId;

/// A validation error for a chain-neutral identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainError {
    /// The supplied chain name is not supported by this build.
    UnsupportedChain { value: String },
    /// An identity value was empty after whitespace was removed.
    EmptyIdentifier { kind: &'static str },
    /// The identity belongs to a different chain than the conversion requested.
    WrongChain { expected: ChainId, actual: ChainId },
    /// The account address is not valid for the identity's chain.
    InvalidAccount { chain: ChainId, value: String },
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedChain { value } => write!(f, "unsupported chain: {value}"),
            Self::EmptyIdentifier { kind } => write!(f, "{kind} cannot be empty"),
            Self::WrongChain { expected, actual } => {
                write!(f, "expected {expected} account, got {actual}")
            }
            Self::InvalidAccount { chain, value } => {
                write!(f, "invalid {chain} account '{value}'")
            }
        }
    }
}

impl std::error::Error for ChainError {}
