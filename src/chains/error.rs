//! Errors returned while constructing or parsing chain identities.

use std::fmt;

/// A validation error for a chain-neutral identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainError {
    /// The supplied chain name is not supported by this build.
    UnsupportedChain { value: String },
    /// An identity value was empty after whitespace was removed.
    EmptyIdentifier { kind: &'static str },
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedChain { value } => write!(f, "unsupported chain: {value}"),
            Self::EmptyIdentifier { kind } => write!(f, "{kind} cannot be empty"),
        }
    }
}

impl std::error::Error for ChainError {}
