//! Filesystem / OS I/O error classifications.
//!
//! Keep errors `Clone` by storing messages as strings.

#[derive(Debug, Clone, thiserror::Error)]
pub enum IoError {
    #[error("not found: {message}")]
    NotFound { message: String },
    #[error("permission denied: {message}")]
    PermissionDenied { message: String },
    #[error("already exists: {message}")]
    AlreadyExists { message: String },
    #[error("invalid input: {message}")]
    InvalidInput { message: String },
    #[error("{message}")]
    Generic { message: String },
}

impl From<std::io::Error> for IoError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => IoError::NotFound {
                message: err.to_string(),
            },
            std::io::ErrorKind::PermissionDenied => IoError::PermissionDenied {
                message: err.to_string(),
            },
            std::io::ErrorKind::AlreadyExists => IoError::AlreadyExists {
                message: err.to_string(),
            },
            std::io::ErrorKind::InvalidInput => IoError::InvalidInput {
                message: err.to_string(),
            },
            _ => IoError::Generic {
                message: err.to_string(),
            },
        }
    }
}
