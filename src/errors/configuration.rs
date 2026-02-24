#[derive(Debug, Clone)]
pub enum ConfigurationError {
    InvalidConfig { field: String, reason: String },
    MissingConfig { field: String },
    InvalidPrivateKey { error: String },
    InvalidWalletAddress { address: String, error: String },
    InvalidUrl { url: String, error: String },
    FileNotFound { path: String },
    Generic { message: String },
}

impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigurationError::InvalidConfig { field, reason } => {
                write!(f, "Invalid config field '{field}': {reason}")
            }
            ConfigurationError::InvalidPrivateKey { error } => {
                write!(f, "Invalid private key: {error}")
            }
            ConfigurationError::Generic { message } => write!(f, "{message}"),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl std::error::Error for ConfigurationError {}
