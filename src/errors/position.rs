//! Position lifecycle errors — entry, exit, and state transition failures.

#[derive(Debug, Clone)]
pub enum PositionError {
    PositionNotFound {
        token_mint: String,
        signature: String,
    },
    VerificationTimeout {
        signature: String,
        timeout_seconds: u64,
    },
    VerificationFailed {
        signature: String,
        reason: String,
    },
    PhantomPositionDetected {
        token_mint: String,
        signature: String,
    },
    Generic {
        message: String,
    },
    DatabaseError(String),
}

impl std::fmt::Display for PositionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionError::PositionNotFound {
                token_mint,
                signature,
            } => {
                write!(
                    f,
                    "Position not found for token {} with signature {}",
                    token_mint, signature
                )
            }
            PositionError::Generic { message } => write!(f, "{message}"),
            PositionError::DatabaseError(msg) => write!(f, "Database error: {msg}"),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl std::error::Error for PositionError {}
