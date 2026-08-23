//! Network errors — HTTP request failures, timeouts, and connection issues.

#[derive(Debug, Clone, thiserror::Error)]
pub enum NetworkError {
    #[error("{message}")]
    Generic { message: String },
}
