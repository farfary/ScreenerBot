/// Filesystem / OS I/O error classifications.
///
/// Keep errors `Clone` by storing messages as strings.

#[derive(Debug, Clone)]
pub enum IoError {
    NotFound { message: String },
    PermissionDenied { message: String },
    AlreadyExists { message: String },
    InvalidInput { message: String },
    Generic { message: String },
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IoError::NotFound { message } => write!(f, "Not found: {message}"),
            IoError::PermissionDenied { message } => write!(f, "Permission denied: {message}"),
            IoError::AlreadyExists { message } => write!(f, "Already exists: {message}"),
            IoError::InvalidInput { message } => write!(f, "Invalid input: {message}"),
            IoError::Generic { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for IoError {}

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

