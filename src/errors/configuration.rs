//! Configuration validation errors — invalid settings, missing fields, parse failures.

#[derive(Debug, Clone, thiserror::Error)]
pub enum ConfigurationError {
    #[error("invalid private key: {error}")]
    InvalidPrivateKey { error: String },
    #[error("{message}")]
    Generic { message: String },
}
