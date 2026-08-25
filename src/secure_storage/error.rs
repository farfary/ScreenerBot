//! Errors produced by the secure storage module.

use std::time::Duration;

use crate::errors::{ErrorClass, IoError, Severity};

/// Everything that can go wrong while encrypting or decrypting local secrets.
#[derive(Debug, Clone, thiserror::Error)]
pub enum Error {
    /// The machine identity used to derive the local encryption key was unavailable.
    #[error("could not obtain machine identity: {detail}")]
    MachineIdentity { detail: String },

    /// Reading or creating Android's persistent machine identity failed.
    #[error(transparent)]
    DeviceIdentityIo(#[from] IoError),

    /// Initializing or using the authenticated cipher failed.
    #[error("cryptographic {operation} failed: {detail}")]
    CryptographicOperation {
        operation: &'static str,
        detail: String,
    },

    /// An encoded encrypted-data field could not be decoded.
    #[error("invalid {field} encoding: {detail}")]
    InvalidEncoding { field: &'static str, detail: String },

    /// The encoded nonce did not have AES-GCM's required size.
    #[error("invalid nonce length: expected 12 bytes, got {actual}")]
    InvalidNonceLength { actual: usize },

    /// The ciphertext could not be authenticated with this machine's key.
    #[error("decryption failed - wrong machine or corrupted data")]
    DecryptionFailed,

    /// Decrypted bytes were not valid UTF-8.
    #[error("decrypted data is not valid UTF-8: {detail}")]
    InvalidUtf8 { detail: String },
}

/// Result alias for the secure storage module.
pub type Result<T> = std::result::Result<T, Error>;

impl ErrorClass for Error {
    fn is_retryable(&self) -> bool {
        false
    }

    fn retry_after(&self) -> Option<Duration> {
        None
    }

    fn severity(&self) -> Severity {
        match self {
            Error::MachineIdentity { .. }
            | Error::DeviceIdentityIo(_)
            | Error::CryptographicOperation { .. } => Severity::Critical,
            Error::InvalidEncoding { .. }
            | Error::InvalidNonceLength { .. }
            | Error::DecryptionFailed
            | Error::InvalidUtf8 { .. } => Severity::Warning,
        }
    }

    fn http_status(&self) -> u16 {
        match self {
            Error::MachineIdentity { .. }
            | Error::DeviceIdentityIo(_)
            | Error::CryptographicOperation { .. } => 500,
            Error::InvalidEncoding { .. }
            | Error::InvalidNonceLength { .. }
            | Error::DecryptionFailed
            | Error::InvalidUtf8 { .. } => 400,
        }
    }
}
